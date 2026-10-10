//! `run-deploy-hooks PHASE COMMIT`: runs the server's deploy hooks for one
//! phase (`docs/deploy-hooks.md`). Installed root-owned `0755` at
//! `/usr/local/lib/baylee/run-deploy-hooks` by `baylee-deploy`, and called
//! by it through `sudo -n`. Reads no environment and takes no option.

use std::process::ExitCode;

use baylee_deploy_hooks::{args, exit};

fn main() -> ExitCode {
    let given: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let (phase, commit) = match args::parse(&given) {
        Ok(parsed) => parsed,
        Err(why) => {
            eprintln!("run-deploy-hooks: {why}\n{}", args::USAGE);
            return ExitCode::from(exit::USAGE);
        }
    };
    run(phase, &commit)
}

#[cfg(target_os = "linux")]
fn run(phase: args::Phase, commit: &str) -> ExitCode {
    let outcome =
        baylee_deploy_hooks::run::run(&baylee_deploy_hooks::Layout::production(), phase, commit);
    println!("run-deploy-hooks: {phase}: {outcome}");
    ExitCode::from(outcome.code())
}

#[cfg(not(target_os = "linux"))]
fn run(phase: args::Phase, _commit: &str) -> ExitCode {
    eprintln!("run-deploy-hooks: {phase}: deploy hooks run on Linux only");
    ExitCode::from(exit::INTERNAL)
}
