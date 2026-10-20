//! `baylee-invite`: see [`baylee_ops::invite`].
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = baylee_ops::invite::main(&args);
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
