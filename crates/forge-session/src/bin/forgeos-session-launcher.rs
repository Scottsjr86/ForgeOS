//! Display-manager entrypoint for a dedicated ForgeOS session.

use forge_session::bootstrap::{
    DEFAULT_COMPOSITION_ROOT, SessionEnvironment, SessionLaunchError, SessionLaunchRequest,
    launch_session,
};
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

const CLI_ERROR: i32 = 64;
const ENVIRONMENT_ERROR: i32 = 78;
const SPAWN_ERROR: i32 = 127;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let (composition_root, arguments) = match parse_arguments(env::args_os().skip(1)) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("forgeos-session-launcher: {message}");
            return CLI_ERROR;
        }
    };

    let environment = match SessionEnvironment::from_parent(env::vars_os()) {
        Ok(environment) => environment,
        Err(error) => {
            eprintln!("forgeos-session-launcher: {error}");
            return ENVIRONMENT_ERROR;
        }
    };
    let request = match SessionLaunchRequest::new(composition_root, arguments, environment) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("forgeos-session-launcher: {error}");
            return CLI_ERROR;
        }
    };

    match launch_session(&request) {
        Ok(outcome) => outcome.launcher_exit_code(),
        Err(error) => {
            eprintln!("forgeos-session-launcher: {error}");
            match error {
                SessionLaunchError::Spawn { .. } => SPAWN_ERROR,
                SessionLaunchError::RelativeCompositionRoot(_) => CLI_ERROR,
            }
        }
    }
}

fn parse_arguments<I>(arguments: I) -> Result<(PathBuf, Vec<OsString>), String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut arguments = arguments.into_iter();
    let mut composition_root = PathBuf::from(DEFAULT_COMPOSITION_ROOT);
    let mut root_overridden = false;
    let mut child_arguments = Vec::new();

    while let Some(argument) = arguments.next() {
        if argument.as_os_str() == OsStr::new("--") {
            child_arguments.extend(arguments);
            return Ok((composition_root, child_arguments));
        }
        if argument.as_os_str() == OsStr::new("--composition-root") {
            if root_overridden {
                return Err("--composition-root may be supplied only once".to_owned());
            }
            let value = arguments
                .next()
                .ok_or_else(|| "--composition-root requires a path".to_owned())?;
            composition_root = PathBuf::from(value);
            root_overridden = true;
            continue;
        }
        return Err(format!("unknown launcher argument {:?}", argument));
    }

    Ok((composition_root, child_arguments))
}
