//! Exercise the actual native research-cache CLI without tensor or model work.
#![cfg(target_os = "linux")]
use baylee_train_ops as ops;
#[path = "../src/research_cache/tests/support.rs"]
mod support;
use serde_json::Value;
use std::process::Command;

#[test]
fn explicit_native_cli_builds_verifies_and_random_reads_without_ordinary_cache() {
    let fixture = support::Fixture::new(false);
    assert!(fixture.source.is_dir());
    let exe = env!("CARGO_BIN_EXE_trajectory_cache4");
    let missing = Command::new(exe)
        .args(["research-build", "--data-dir"])
        .arg(&fixture.converted)
        .arg("--out")
        .arg(&fixture.cache)
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(!fixture.cache.exists());
    let built = Command::new(exe)
        .arg("research-build")
        .arg("--data-dir")
        .arg(&fixture.converted)
        .arg("--out")
        .arg(&fixture.cache)
        .arg("--expected-manifest-sha256")
        .arg(&fixture.expected)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let result: Value = serde_json::from_slice(&built.stdout).unwrap();
    assert_eq!(result["frames"], 10);
    assert_eq!(result["admitted"], false);
    for action in ["research-verify", "research-read"] {
        let mut command = Command::new(exe);
        command
            .arg(action)
            .arg("--cache-dir")
            .arg(&fixture.cache)
            .arg("--expected-manifest-sha256")
            .arg(&fixture.expected);
        if action == "research-read" {
            command.args(["--row", "1"]);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if action == "research-read" {
            let mut expected = fixture.raw[1].clone();
            expected.push(b'\n');
            assert_eq!(output.stdout, expected);
        }
    }
}
