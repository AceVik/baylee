//! `scripts/server/baylee-deploy stage` with every command it would change a
//! server with stood in for (`git`, `cargo`, `npm`, `sudo`, `systemctl`,
//! `curl`, `flock`, `logger`): it builds the feedback service with the rest,
//! installs and restarts it exactly when its unit is installed, and builds
//! and installs its web UI before that restart when npm is there
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

/// What npm does on the server under test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Npm {
    /// Not installed.
    Absent,
    /// Installs and builds `dist/`.
    Builds,
    /// Installed, but the build fails.
    Fails,
}

/// Runs `stage` in a scratch tree; the deploy root and every command it
/// ran, one per line.
fn stage(feedback_installed: bool, npm: Npm) -> (PathBuf, String) {
    // Tests run at once, some with the same arguments: each run its own tree.
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let scratch = std::env::temp_dir().join(format!(
        "baylee-deploy-test-{}-{}-{feedback_installed}-{npm:?}",
        std::process::id(),
        RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    let (root, stubs) = (scratch.join("opt"), scratch.join("stubs"));
    for dir in [
        "src/target/release",
        "src/data",
        "src/web/feedback",
        "bin",
        "state",
    ] {
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
    stub(
        &stubs,
        "npm",
        match npm {
            Npm::Fails => r#"[ "$1" = run ] && exit 1; exit 0"#,
            _ => {
                r#"[ "$1 $2" = "run build" ] && mkdir -p dist && echo page > dist/index.html; exit 0"#
            }
        },
    );
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

    let npm_path = if npm == Npm::Absent {
        scratch.join("no-such-npm")
    } else {
        stubs.join("npm")
    };
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/server/baylee-deploy");
    let out = std::process::Command::new("bash")
        .arg(&script)
        .args(["stage", "0123456789abcdef0123"])
        .env("BAYLEE_DEPLOY_ROOT", &root)
        .env("CARGO", stubs.join("cargo"))
        .env("NPM", &npm_path)
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
    (root, ran)
}

#[test]
fn stage_builds_the_feedback_service_and_installs_it_only_where_its_unit_is() {
    let bin = "/bin/";
    for installed in [true, false] {
        let (_, ran) = stage(installed, Npm::Builds);
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
        // A service that is not here gets no web UI built either.
        assert_eq!(ran.contains("npm "), installed, "{ran}");
    }
}

#[test]
fn stage_builds_the_web_ui_and_installs_it_before_the_service_restarts() {
    let (root, ran) = stage(true, Npm::Builds);
    let lines: Vec<&str> = ran.lines().collect();
    let at = |wanted: &str| {
        lines
            .iter()
            .position(|l| *l == wanted)
            .unwrap_or_else(|| panic!("no `{wanted}` in:\n{ran}"))
    };
    let web = root.join("web/feedback");
    let web = web.display();
    let ci = at("npm ci --no-audit --no-fund");
    let build = at("npm run build");
    let copy = at(&format!("sudo cp -R web/feedback/dist {web}.new"));
    let swap = at(&format!("sudo mv {web}.new {web}"));
    let restart = at("sudo systemctl restart baylee-feedback");
    assert!(
        ci < build && build < copy && copy < swap && swap < restart,
        "{ran}"
    );
    // A fresh checkout keeps the installed packages between deploys.
    let clean = lines
        .iter()
        .find(|l| l.starts_with("git clean"))
        .expect("a clean");
    assert!(clean.contains("-e web/feedback/node_modules"), "{clean}");
}

#[test]
fn without_npm_or_with_a_failed_build_the_service_is_still_deployed() {
    for npm in [Npm::Absent, Npm::Fails] {
        let (_, ran) = stage(true, npm);
        assert!(
            !ran.contains("sudo cp -R web/feedback/dist"),
            "{npm:?}: a UI was installed:\n{ran}"
        );
        assert!(
            ran.lines()
                .any(|l| l == "sudo systemctl restart baylee-feedback"),
            "{npm:?}: the service was not restarted:\n{ran}"
        );
        assert_eq!(ran.contains("npm run build"), npm == Npm::Fails, "{ran}");
    }
}

/// `stage` installs the closed-beta key command (#317) beside itself, so a
/// release that changes it reaches the owner's terminal.
#[test]
fn stage_installs_the_invite_command() {
    let (_, ran) = stage(false, Npm::Absent);
    assert!(
        ran.lines().any(|l| l
            == "sudo install -m755 scripts/server/baylee-invite /usr/local/sbin/baylee-invite"),
        "{ran}"
    );
}

/// `baylee-invite` reads the gateway's two settings files and hands its
/// arguments to `baylee-gateway invite` with them; with no database named
/// in either it says so and runs nothing.
#[test]
fn the_invite_command_runs_the_gateway_with_its_settings() {
    let scratch = std::env::temp_dir().join(format!("baylee-invite-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let etc = scratch.join("etc");
    std::fs::create_dir_all(&etc).unwrap();
    let gateway = scratch.join("baylee-gateway");
    std::fs::write(
        &gateway,
        "#!/usr/bin/env bash\necho \"args: $*\"\necho \"db: $DATABASE_URL\"\n\
         echo \"reg: $BAYLEE_REGISTRATION\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&gateway, std::fs::Permissions::from_mode(0o755)).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/server/baylee-invite");
    let run = || {
        std::process::Command::new("bash")
            .arg(&script)
            .args(["create", "--note", "Max und Moritz"])
            .env("BAYLEE_INVITE_ETC", &etc)
            .env("BAYLEE_INVITE_BIN", &gateway)
            .env_remove("DATABASE_URL")
            .env_remove("BAYLEE_REGISTRATION")
            .output()
            .expect("bash runs")
    };

    let out = run();
    assert_eq!(out.status.code(), Some(1), "no database named");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("DATABASE_URL"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty(), "the gateway was not run");

    std::fs::write(
        etc.join("secrets.env"),
        "DATABASE_URL=postgres://baylee:secret@127.0.0.1/baylee\n",
    )
    .unwrap();
    std::fs::write(etc.join("gateway.env"), "BAYLEE_REGISTRATION=invite\n").unwrap();
    let out = run();
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{said}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        said,
        "args: invite create --note Max und Moritz\n\
         db: postgres://baylee:secret@127.0.0.1/baylee\n\
         reg: invite\n"
    );
    let _ = std::fs::remove_dir_all(&scratch);
}
