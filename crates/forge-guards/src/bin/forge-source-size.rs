use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use forge_guards::source_size::{ModuleStatus, scan_authored_rust};

const USAGE: &str = "Usage: forge-source-size [--root <path>] [--deny-warnings]\n\
\n\
Scans authored Rust source modules below the repository root.\n\
PASS: 0-1000 physical lines\n\
WARN: 1001-1200 physical lines\n\
FAIL: 1201+ physical lines\n\
\n\
--deny-warnings preserves WARN classification but returns a failing exit code.\n";

fn main() -> ExitCode {
    match parse_options(env::args_os().skip(1)) {
        Ok(ParseResult::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(ParseResult::Run(options)) => run(&options),
        Err(message) => {
            eprintln!("forge-source-size: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run(options: &Options) -> ExitCode {
    let report = match scan_authored_rust(&options.root) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("FORGE_SOURCE_SIZE_ERROR {error}");
            return ExitCode::from(2);
        }
    };

    for module in report.modules() {
        println!(
            "FORGE_SOURCE_SIZE_MODULE status={} lines={} path={}",
            module.status().as_str(),
            module.physical_lines(),
            normalized_path(module.relative_path())
        );
    }

    let status = report.overall_status();
    println!(
        "FORGE_SOURCE_SIZE_SUMMARY status={} modules={} pass={} warn={} fail={} warnings_denied={}",
        status.as_str(),
        report.modules().len(),
        report.pass_count(),
        report.warning_count(),
        report.failure_count(),
        options.deny_warnings
    );

    match status {
        ModuleStatus::Pass => ExitCode::SUCCESS,
        ModuleStatus::Warn if !options.deny_warnings => ExitCode::SUCCESS,
        ModuleStatus::Warn | ModuleStatus::Fail => ExitCode::FAILURE,
    }
}

struct Options {
    root: PathBuf,
    deny_warnings: bool,
}

enum ParseResult {
    Help,
    Run(Options),
}

fn parse_options(
    arguments: impl Iterator<Item = std::ffi::OsString>,
) -> Result<ParseResult, String> {
    let mut arguments = arguments;
    let mut root = PathBuf::from(".");
    let mut root_seen = false;
    let mut deny_warnings = false;

    while let Some(argument) = arguments.next() {
        if argument == "--help" || argument == "-h" {
            return Ok(ParseResult::Help);
        }

        if argument == "--root" {
            if root_seen {
                return Err("--root may be supplied only once".to_owned());
            }
            root = arguments
                .next()
                .ok_or_else(|| "--root requires a path".to_owned())?
                .into();
            root_seen = true;
            continue;
        }

        if argument == "--deny-warnings" {
            if deny_warnings {
                return Err("--deny-warnings may be supplied only once".to_owned());
            }
            deny_warnings = true;
            continue;
        }

        return Err(format!("unknown argument: {}", argument.to_string_lossy()));
    }

    Ok(ParseResult::Run(Options {
        root,
        deny_warnings,
    }))
}

fn normalized_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
