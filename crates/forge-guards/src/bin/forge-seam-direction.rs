use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use forge_guards::seams::inspect_seam_directions;

const USAGE: &str = "Usage: forge-seam-direction [--root <path>]\n\
\n\
Inspects the real Cargo normal and build dependency graph for every reviewed\n\
ForgeOS subsystem. Unknown ForgeOS workspace packages and undeclared direct or\n\
transitive subsystem reachability fail closed.\n";

fn main() -> ExitCode {
    match parse_options(env::args_os().skip(1)) {
        Ok(ParseResult::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(ParseResult::Run(options)) => run(&options),
        Err(message) => {
            eprintln!("forge-seam-direction: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run(options: &Options) -> ExitCode {
    let report = match inspect_seam_directions(&options.root) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("FORGE_SEAM_DIRECTION_ERROR {error}");
            return ExitCode::from(2);
        }
    };

    for package in report.workspace_packages() {
        let status = if report
            .violations()
            .iter()
            .any(|violation| violation.root() == "workspace" && violation.target() == package)
        {
            "FORBIDDEN"
        } else {
            "ALLOWED"
        };
        println!("FORGE_SEAM_DIRECTION_PACKAGE status={status} package={package}");
    }

    for relation in report.relations() {
        let status = if report.violations().iter().any(|violation| {
            violation.root() == relation.root() && violation.target() == relation.target()
        }) {
            "FORBIDDEN"
        } else {
            "ALLOWED"
        };
        println!(
            "FORGE_SEAM_DIRECTION_ROUTE status={status} root={} target={}",
            relation.root(),
            relation.target()
        );
    }

    let status = if report.is_legal() { "PASS" } else { "FAIL" };
    println!(
        "FORGE_SEAM_DIRECTION_SUMMARY status={status} packages={} routes={} forbidden={} policy=exact-reviewed-subsystem-reachability-v1",
        report.workspace_packages().len(),
        report.relations().len(),
        report.violations().len()
    );

    if report.is_legal() {
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
