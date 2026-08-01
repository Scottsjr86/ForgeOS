use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use forge_guards::core_purity::inspect_core_dependencies;

const USAGE: &str = "Usage: forge-core-purity [--root <path>]\n\
\n\
Inspects the normal and build dependency graph reachable from forge-core.\n\
Only explicitly reviewed pure packages pass. Any unknown direct or transitive\n\
package fails closed.\n";

fn main() -> ExitCode {
    match parse_options(env::args_os().skip(1)) {
        Ok(ParseResult::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(ParseResult::Run(options)) => run(&options),
        Err(message) => {
            eprintln!("forge-core-purity: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run(options: &Options) -> ExitCode {
    let report = match inspect_core_dependencies(&options.root) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("FORGE_CORE_PURITY_ERROR {error}");
            return ExitCode::from(2);
        }
    };

    for package in report.packages() {
        let status = if report
            .violations()
            .iter()
            .any(|violation| violation.package() == package)
        {
            "FORBIDDEN"
        } else {
            "ALLOWED"
        };
        println!("FORGE_CORE_PURITY_PACKAGE status={status} package={package}");
    }

    let status = if report.is_pure() { "PASS" } else { "FAIL" };
    println!(
        "FORGE_CORE_PURITY_SUMMARY status={status} packages={} allowed={} forbidden={} policy=exact-reviewed-production-graph-v1",
        report.packages().len(),
        report.allowed_count(),
        report.violations().len()
    );

    if report.is_pure() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

struct Options {
    root: PathBuf,
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

        return Err(format!("unknown argument: {}", argument.to_string_lossy()));
    }

    Ok(ParseResult::Run(Options { root }))
}
