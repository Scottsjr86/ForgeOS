//! Strict text-patch structure parser for PATCH-100.

use crate::patches::PatchApplyError;
use forge_protocol::patches::{PatchFileAction, PatchFileRecord};
use forge_protocol::paths::RepositoryRelativePath;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedPatchFile {
    action: PatchFileAction,
    path: RepositoryRelativePath,
}

fn path_bytes(path: &RepositoryRelativePath) -> &[u8] {
    path.as_path().as_os_str().as_encoded_bytes()
}

fn parse_patch(bytes: &[u8]) -> Result<Vec<ParsedPatchFile>, PatchApplyError> {
    if bytes.contains(&0) {
        return Err(PatchApplyError::HiddenBinaryPatch);
    }
    if bytes.contains(&b'\r') {
        return Err(PatchApplyError::MalformedPatch(
            "carriage returns are not canonical patch bytes".to_owned(),
        ));
    }

    let lines: Vec<&[u8]> = bytes.split(|byte| *byte == b'\n').collect();
    if lines
        .iter()
        .any(|line| *line == b"GIT binary patch" || line.starts_with(b"Binary files "))
    {
        return Err(PatchApplyError::HiddenBinaryPatch);
    }
    let mut files = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if lines[index].is_empty() {
            index += 1;
            continue;
        }
        if !lines[index].starts_with(b"diff --git ") {
            return Err(PatchApplyError::MalformedPatch(format!(
                "expected diff --git header at line {}",
                index + 1
            )));
        }
        let (diff_old, diff_new) = parse_diff_header(lines[index])?;
        index += 1;
        let mut old_header = None;
        let mut new_header = None;
        let mut saw_hunk = false;
        let mut saw_new_file_mode = false;
        let mut saw_deleted_file_mode = false;

        while index < lines.len() && !lines[index].starts_with(b"diff --git ") {
            let line = lines[index];
            if line.starts_with(b"@@ ") {
                saw_hunk = true;
                index = consume_hunk(&lines, index)?;
                continue;
            }
            if saw_hunk {
                if line.is_empty() {
                    index += 1;
                    continue;
                }
                return Err(PatchApplyError::MalformedPatch(format!(
                    "unexpected data after completed hunk at line {}",
                    index + 1
                )));
            }
            if line.starts_with(b"--- ") {
                if old_header
                    .replace(parse_file_header(line, b"--- ")?)
                    .is_some()
                {
                    return Err(PatchApplyError::MalformedPatch(
                        "duplicate --- header".to_owned(),
                    ));
                }
            } else if line.starts_with(b"+++ ") {
                if new_header
                    .replace(parse_file_header(line, b"+++ ")?)
                    .is_some()
                {
                    return Err(PatchApplyError::MalformedPatch(
                        "duplicate +++ header".to_owned(),
                    ));
                }
            } else if line.starts_with(b"new file mode ") {
                if line != b"new file mode 100644" || saw_new_file_mode {
                    return Err(PatchApplyError::MalformedPatch(
                        "only one regular 100644 added-file mode is accepted".to_owned(),
                    ));
                }
                saw_new_file_mode = true;
            } else if line.starts_with(b"deleted file mode ") {
                if line != b"deleted file mode 100644" || saw_deleted_file_mode {
                    return Err(PatchApplyError::MalformedPatch(
                        "only one regular 100644 deleted-file mode is accepted".to_owned(),
                    ));
                }
                saw_deleted_file_mode = true;
            } else if forbidden_metadata(line) {
                return Err(PatchApplyError::MalformedPatch(format!(
                    "unsupported rename, copy, or mode metadata at line {}",
                    index + 1
                )));
            }
            index += 1;
        }
        let old_header = old_header.ok_or_else(|| {
            PatchApplyError::MalformedPatch("diff section is missing --- header".to_owned())
        })?;
        let new_header = new_header.ok_or_else(|| {
            PatchApplyError::MalformedPatch("diff section is missing +++ header".to_owned())
        })?;
        if !saw_hunk {
            return Err(PatchApplyError::MalformedPatch(
                "diff section contains no hunk".to_owned(),
            ));
        }
        let parsed = classify_section(diff_old, diff_new, old_header, new_header)?;
        match parsed.action {
            PatchFileAction::Add if !saw_new_file_mode || saw_deleted_file_mode => {
                return Err(PatchApplyError::MalformedPatch(
                    "added files require only new file mode 100644".to_owned(),
                ));
            }
            PatchFileAction::Delete if !saw_deleted_file_mode || saw_new_file_mode => {
                return Err(PatchApplyError::MalformedPatch(
                    "deleted files require only deleted file mode 100644".to_owned(),
                ));
            }
            PatchFileAction::Modify if saw_new_file_mode || saw_deleted_file_mode => {
                return Err(PatchApplyError::MalformedPatch(
                    "modified files may not carry add/delete mode metadata".to_owned(),
                ));
            }
            _ => {}
        }
        files.push(parsed);
    }
    if files.is_empty() {
        return Err(PatchApplyError::MalformedPatch(
            "patch contains no file sections".to_owned(),
        ));
    }
    let mut seen = BTreeSet::new();
    for file in &files {
        if !seen.insert(path_bytes(&file.path).to_vec()) {
            return Err(PatchApplyError::MalformedPatch(format!(
                "patch repeats path {}",
                file.path.as_path().display()
            )));
        }
    }
    files.sort_by(|left, right| path_bytes(&left.path).cmp(path_bytes(&right.path)));
    Ok(files)
}

fn consume_hunk(lines: &[&[u8]], header_index: usize) -> Result<usize, PatchApplyError> {
    let (mut old_remaining, mut new_remaining) = parse_hunk_counts(lines[header_index])?;
    let mut index = header_index + 1;
    while old_remaining != 0 || new_remaining != 0 {
        let line = lines.get(index).ok_or_else(|| {
            PatchApplyError::MalformedPatch("hunk ended before its declared line counts".to_owned())
        })?;
        match line.first().copied() {
            Some(b' ') => {
                consume_count(&mut old_remaining, "old", index)?;
                consume_count(&mut new_remaining, "new", index)?;
            }
            Some(b'-') => consume_count(&mut old_remaining, "old", index)?,
            Some(b'+') => consume_count(&mut new_remaining, "new", index)?,
            Some(b'\\') if *line == b"\\ No newline at end of file" => {}
            _ => {
                return Err(PatchApplyError::MalformedPatch(format!(
                    "invalid hunk body at line {}",
                    index + 1
                )));
            }
        }
        index += 1;
    }
    while lines.get(index).copied() == Some(b"\\ No newline at end of file".as_slice()) {
        index += 1;
    }
    Ok(index)
}

fn consume_count(
    remaining: &mut usize,
    side: &'static str,
    line_index: usize,
) -> Result<(), PatchApplyError> {
    if *remaining == 0 {
        return Err(PatchApplyError::MalformedPatch(format!(
            "hunk contains too many {side} lines at line {}",
            line_index + 1
        )));
    }
    *remaining -= 1;
    Ok(())
}

fn parse_hunk_counts(line: &[u8]) -> Result<(usize, usize), PatchApplyError> {
    let fields: Vec<&[u8]> = line.split(|byte| *byte == b' ').collect();
    if fields.len() < 4 || fields[0] != b"@@" || fields[3] != b"@@" {
        return Err(PatchApplyError::MalformedPatch(
            "hunk header must use canonical unified-diff ranges".to_owned(),
        ));
    }
    Ok((
        parse_hunk_range(fields[1], b'-')?,
        parse_hunk_range(fields[2], b'+')?,
    ))
}

fn parse_hunk_range(value: &[u8], prefix: u8) -> Result<usize, PatchApplyError> {
    let value = value.strip_prefix(&[prefix]).ok_or_else(|| {
        PatchApplyError::MalformedPatch("hunk range has the wrong side prefix".to_owned())
    })?;
    let mut parts = value.split(|byte| *byte == b',');
    let start = parts.next().unwrap_or_default();
    let count = parts.next();
    if start.is_empty() || parts.next().is_some() {
        return Err(PatchApplyError::MalformedPatch(
            "hunk range is malformed".to_owned(),
        ));
    }
    parse_decimal(start, "hunk start")?;
    match count {
        Some(count) => parse_decimal(count, "hunk count"),
        None => Ok(1),
    }
}

fn parse_decimal(value: &[u8], field: &'static str) -> Result<usize, PatchApplyError> {
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return Err(PatchApplyError::MalformedPatch(format!(
            "{field} must be an unsigned decimal"
        )));
    }
    std::str::from_utf8(value)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or_else(|| PatchApplyError::MalformedPatch(format!("{field} is out of range")))
}

fn parse_diff_header(line: &[u8]) -> Result<(Vec<u8>, Vec<u8>), PatchApplyError> {
    let fields: Vec<&[u8]> = line.split(|byte| *byte == b' ').collect();
    if fields.len() != 4 || fields[0] != b"diff" || fields[1] != b"--git" {
        return Err(PatchApplyError::MalformedPatch(
            "diff header must contain two unquoted paths".to_owned(),
        ));
    }
    let old = strip_prefix_path(fields[2], b"a/")?;
    let new = strip_prefix_path(fields[3], b"b/")?;
    Ok((old.to_vec(), new.to_vec()))
}

fn parse_file_header(line: &[u8], prefix: &[u8]) -> Result<HeaderPath, PatchApplyError> {
    let value = line
        .strip_prefix(prefix)
        .ok_or_else(|| PatchApplyError::MalformedPatch("invalid file header".to_owned()))?;
    if value == b"/dev/null" {
        return Ok(HeaderPath::Null);
    }
    if value.contains(&b'\t') || value.contains(&b' ') || value.starts_with(b"\"") {
        return Err(PatchApplyError::MalformedPatch(
            "quoted, timestamped, or whitespace-bearing patch paths are not accepted".to_owned(),
        ));
    }
    let path = if prefix == b"--- " {
        strip_prefix_path(value, b"a/")?
    } else {
        strip_prefix_path(value, b"b/")?
    };
    Ok(HeaderPath::Path(path.to_vec()))
}

fn strip_prefix_path<'a>(value: &'a [u8], prefix: &[u8]) -> Result<&'a [u8], PatchApplyError> {
    let path = value.strip_prefix(prefix).ok_or_else(|| {
        PatchApplyError::MalformedPatch(
            "patch path is missing canonical a/ or b/ prefix".to_owned(),
        )
    })?;
    if path.is_empty() || path.contains(&b'\t') || path.contains(&b' ') || path.starts_with(b"\"") {
        return Err(PatchApplyError::MalformedPatch(
            "patch paths must be nonempty unquoted bytes without whitespace".to_owned(),
        ));
    }
    Ok(path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HeaderPath {
    Null,
    Path(Vec<u8>),
}

fn classify_section(
    diff_old: Vec<u8>,
    diff_new: Vec<u8>,
    old: HeaderPath,
    new: HeaderPath,
) -> Result<ParsedPatchFile, PatchApplyError> {
    match (old, new) {
        (HeaderPath::Null, HeaderPath::Path(path)) => {
            if path != diff_new || diff_old != diff_new {
                return Err(PatchApplyError::MalformedPatch(
                    "added-file paths disagree across headers".to_owned(),
                ));
            }
            Ok(ParsedPatchFile {
                action: PatchFileAction::Add,
                path: relative_from_bytes(path)?,
            })
        }
        (HeaderPath::Path(path), HeaderPath::Null) => {
            if path != diff_old || diff_old != diff_new {
                return Err(PatchApplyError::MalformedPatch(
                    "deleted-file paths disagree across headers".to_owned(),
                ));
            }
            Ok(ParsedPatchFile {
                action: PatchFileAction::Delete,
                path: relative_from_bytes(path)?,
            })
        }
        (HeaderPath::Path(old), HeaderPath::Path(new)) => {
            if old != new || old != diff_old || new != diff_new {
                return Err(PatchApplyError::MalformedPatch(
                    "renames and path disagreement are not accepted by PATCH-100".to_owned(),
                ));
            }
            Ok(ParsedPatchFile {
                action: PatchFileAction::Modify,
                path: relative_from_bytes(old)?,
            })
        }
        (HeaderPath::Null, HeaderPath::Null) => Err(PatchApplyError::MalformedPatch(
            "both file headers may not be /dev/null".to_owned(),
        )),
    }
}

fn forbidden_metadata(line: &[u8]) -> bool {
    [
        b"rename from ".as_slice(),
        b"rename to ".as_slice(),
        b"copy from ".as_slice(),
        b"copy to ".as_slice(),
        b"similarity index ".as_slice(),
        b"dissimilarity index ".as_slice(),
        b"old mode ".as_slice(),
        b"new mode ".as_slice(),
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

#[cfg(unix)]
fn relative_from_bytes(bytes: Vec<u8>) -> Result<RepositoryRelativePath, PatchApplyError> {
    let path = PathBuf::from(std::ffi::OsString::from_vec(bytes));
    RepositoryRelativePath::new(path)
        .map_err(|error| PatchApplyError::MalformedPatch(error.to_string()))
}

#[cfg(not(unix))]
fn relative_from_bytes(_bytes: Vec<u8>) -> Result<RepositoryRelativePath, PatchApplyError> {
    Err(PatchApplyError::UnsupportedPlatform)
}

fn compare_file_table(
    declared: &[PatchFileRecord],
    parsed: &[ParsedPatchFile],
) -> Result<(), PatchApplyError> {
    if declared.len() != parsed.len() {
        return Err(PatchApplyError::FileTableMismatch {
            message: format!(
                "declared {} files but payload contains {}",
                declared.len(),
                parsed.len()
            ),
        });
    }
    for (declared, parsed) in declared.iter().zip(parsed) {
        if declared.action() != parsed.action
            || path_bytes(declared.path()) != path_bytes(&parsed.path)
        {
            return Err(PatchApplyError::FileTableMismatch {
                message: format!(
                    "declared {:?} {} but payload contains {:?} {}",
                    declared.action(),
                    declared.path().as_path().display(),
                    parsed.action,
                    parsed.path.as_path().display()
                ),
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_file_table(
    declared: &[PatchFileRecord],
    bytes: &[u8],
) -> Result<(), PatchApplyError> {
    let parsed = parse_patch(bytes)?;
    compare_file_table(declared, &parsed)
}
