//! The installed binary refuses bad arguments with its own exit code (64)
//! before it looks at any file, and runs nothing.

use std::process::Command;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn bad_arguments_exit_64_and_say_only_the_usage() {
    let upper = SHA.to_uppercase();
    let cases: [&[&str]; 8] = [
        &[],
        &["prepare"],
        &["prepare", SHA, "extra"],
        &["--help"],
        &["deploy", SHA],
        &["prepare", &SHA[..12]],
        &["prepare", &upper],
        &["prepare", "HEAD"],
    ];
    for args in cases {
        let out = Command::new(env!("CARGO_BIN_EXE_run-deploy-hooks"))
            .args(args)
            .env_clear()
            .output()
            .expect("the binary runs");
        assert_eq!(out.status.code(), Some(64), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(said.contains("usage: run-deploy-hooks"), "{args:?}: {said}");
    }
}
