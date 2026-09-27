//! `scripts/server/baylee-deploy stage` with every command it would change a
//! server with stood in for (`git`, `cargo`, `sudo`, `systemctl`, `curl`,
//! `flock`, `logger`): it builds the feedback service with the rest, and
//! installs and restarts it exactly when its unit is installed
//! (`docs/feedback.md` §"Running it").

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

/// A stand-in command that appends its name and arguments to `calls` and
/// then runs `body`.
fn stub(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    std::fs::write(
        &path,
        format!("#!/usr/bin/env bash\necho \"{name} $*\" >> \"$DEPLOY_CALLS\"\n{body}\n"),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Runs `stage` in a scratch tree; every command it ran, one per line.
fn stage(feedback_installed: bool) -> String {
    let scratch = std::env::temp_dir().join(format!(
        "baylee-deploy-test-{}-{feedback_installed}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    let (root, stubs) = (scratch.join("opt"), scratch.join("stubs"));
    for dir in ["src/target/release", "src/data", "bin", "state"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::create_dir_all(&stubs).unwrap();
    for binary in ["baylee-gateway", "baylee-catalog"] {
        std::fs::write(root.join("src/target/release").join(binary), b"").unwrap();
    }
    std::fs::write(root.join("src/data/acceptance-decks.txt"), b"").unwrap();
    let calls: PathBuf = scratch.join("calls");

    stub(
        &stubs,
        "git",
        r#"[ "$1" = rev-parse ] && echo 0123456789abcdef0123; exit 0"#,
    );
    stub(&stubs, "cargo", "echo built");
    stub(&stubs, "sudo", "exit 0");
    stub(&stubs, "logger", "exit 0");
    stub(&stubs, "flock", "exit 0");
    // Whether a unit is installed is asked with `systemctl cat`.
    stub(
        &stubs,
        "systemctl",
        &format!(
            r#"[ "$1 $2" = "cat baylee-feedback" ] && exit {}; exit 0"#,
            u8::from(!feedback_installed)
        ),
    );
    // A game is running, so `finish` waits and the gateway is left alone.
    stub(
        &stubs,
        "curl",
        r#"echo '{"games":{"running":1,"local_running":1,"waiting":0}}'"#,
    );

    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/server/baylee-deploy");
    let out = std::process::Command::new("bash")
        .arg(&script)
        .args(["stage", "0123456789abcdef0123"])
        .env("BAYLEE_DEPLOY_ROOT", &root)
        .env("CARGO", stubs.join("cargo"))
        .env("DEPLOY_CALLS", &calls)
        .env(
            "PATH",
            format!(
                "{}:{}",
                stubs.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .expect("bash runs");
    assert!(
        out.status.success(),
        "stage failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let ran = std::fs::read_to_string(&calls).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&scratch);
    ran
}

#[test]
fn stage_builds_the_feedback_service_and_installs_it_only_where_its_unit_is() {
    let bin = "/bin/";
    for installed in [true, false] {
        let ran = stage(installed);
        let build = ran
            .lines()
            .find(|l| l.starts_with("cargo build"))
            .unwrap_or_else(|| panic!("no build in:\n{ran}"));
        assert!(build.contains("-p baylee-feedback"), "{build}");
        let install = ran.lines().any(|l| {
            l.starts_with("sudo install -m755 target/release/baylee-feedback ") && l.ends_with(bin)
        });
        let restart = ran
            .lines()
            .any(|l| l == "sudo systemctl restart baylee-feedback");
        assert_eq!(install, installed, "installed: {installed}\n{ran}");
        assert_eq!(restart, installed, "restarted: {installed}\n{ran}");
        // The gateway waits for the running game either way.
        assert!(
            !ran.contains("systemctl restart baylee-gateway"),
            "the gateway was swapped under a running game:\n{ran}"
        );
        assert!(ran.contains("sudo systemctl stop baylee-agent"));
    }
}
