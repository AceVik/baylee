//! `scripts/server/baylee-deploy stage` with every command it would change a
//! server with stood in for (`git`, `cargo`, `npm`, `sudo`, `systemctl`,
//! `curl`, `flock`, `logger`): it builds the feedback service with the rest,
//! installs and restarts it exactly when its unit is installed, and builds
//! and installs its web UI before that restart when npm is there
//! (`docs/feedback.md` §"Running it"); and it installs the legal pages
//! (`web/legal/`) where Caddy serves them (`scripts/server/legal.caddy`).

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

/// The legal pages a tree carries (`web/legal/`), which `stage` installs.
const LEGAL_PAGES: [&str; 3] = ["datenschutz.html", "impressum.html", "privacy.html"];

/// Runs `stage` in a scratch tree; the deploy root and every command it
/// ran, one per line.
fn stage(feedback_installed: bool, npm: Npm) -> (PathBuf, String) {
    stage_in(feedback_installed, npm, true)
}

/// [`stage`], in a tree that carries the legal pages or not.
fn stage_in(feedback_installed: bool, npm: Npm, legal_pages: bool) -> (PathBuf, String) {
    // Tests run at once, some with the same arguments: each run its own tree.
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let scratch = std::env::temp_dir().join(format!(
        "baylee-deploy-test-{}-{}-{feedback_installed}-{npm:?}-{legal_pages}",
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
    if legal_pages {
        std::fs::create_dir_all(root.join("src/web/legal")).unwrap();
        for page in LEGAL_PAGES {
            std::fs::write(root.join("src/web/legal").join(page), b"<!doctype html>").unwrap();
        }
    }
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

/// The privacy statement and imprint are installed at every stage, whether
/// or not the feedback service is on this machine, where Caddy serves them;
/// a tree from before them installs none and leaves what is there.
#[test]
fn stage_installs_the_legal_pages_where_caddy_serves_them() {
    for feedback_installed in [true, false] {
        let (root, ran) = stage(feedback_installed, Npm::Absent);
        let legal = root.join("web/legal");
        let legal = legal.display();
        assert!(
            ran.lines().any(|l| l == format!("sudo mkdir -p {legal}")),
            "{ran}"
        );
        let install = ran
            .lines()
            .find(|l| l.starts_with("sudo install -m644 ") && l.ends_with(&format!(" {legal}/")))
            .unwrap_or_else(|| panic!("the legal pages were not installed:\n{ran}"));
        for page in LEGAL_PAGES {
            assert!(
                install.contains(&format!("web/legal/{page}")),
                "{page} is missing from `{install}`"
            );
        }
    }
    let (root, ran) = stage_in(true, Npm::Absent, false);
    let legal = root.join("web/legal");
    assert!(
        !ran.contains(&format!("{}", legal.display())),
        "a tree without pages installed some:\n{ran}"
    );
}

/// Caddy's snippet serves each legal path from a page this repository
/// ships, and every page is served at a path: a renamed page or a dropped
/// rewrite would be a 404 on the server that no build notices. It also
/// holds no `log` directive, because the pages say the proxy keeps no
/// access log.
#[test]
fn caddy_serves_every_legal_page_and_logs_nothing() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let snippet = std::fs::read_to_string(repo.join("scripts/server/legal.caddy")).unwrap();
    let directives: Vec<&str> = snippet
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let mut served: Vec<(&str, &str)> = directives
        .iter()
        .filter_map(|l| l.strip_prefix("rewrite "))
        .filter_map(|l| l.split_once(' '))
        .collect();
    served.sort_unstable();
    assert_eq!(
        served,
        [
            ("/datenschutz", "/datenschutz.html"),
            ("/impressum", "/impressum.html"),
            ("/privacy", "/privacy.html"),
        ]
    );
    let matcher = directives
        .iter()
        .find(|l| l.starts_with("@baylee_legal path "))
        .expect("the matcher");
    for (path, page) in served {
        assert!(matcher.split(' ').any(|p| p == path), "{path} not matched");
        let file = repo.join("web/legal").join(page.trim_start_matches('/'));
        assert!(file.is_file(), "{} is not shipped", file.display());
    }
    let mut shipped: Vec<String> = std::fs::read_dir(repo.join("web/legal"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    shipped.sort_unstable();
    assert_eq!(shipped, LEGAL_PAGES, "a page no path serves");
    assert!(
        !directives
            .iter()
            .any(|l| l.split_whitespace().next() == Some("log")),
        "an access log in the snippet"
    );
}
