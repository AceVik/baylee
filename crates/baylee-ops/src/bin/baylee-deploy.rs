//! `baylee-deploy`: see [`baylee_ops::deploy`].
use std::process::ExitCode;

fn main() -> ExitCode {
    // `cargo` starts this in rustc's place during a profile-guided build.
    if let Some(code) = baylee_pgo::wrap_rustc() {
        return ExitCode::from(u8::try_from(code).unwrap_or(1));
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = baylee_ops::deploy::main(&args);
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
