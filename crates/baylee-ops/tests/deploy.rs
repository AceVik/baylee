//! `baylee-deploy stage` with every command it would change a
//! server with stood in for (`git`, `cargo`, `npm`, `sudo`, `systemctl`,
//! `curl`, `flock`, `logger`): it builds the feedback service with the rest,
//! installs and restarts it exactly when its unit is installed, and builds
//! and installs its web UI before that restart when npm is there
//! (`docs/feedback.md` §"Running it").

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

#[path = "support/native_command.rs"]
mod native_command;
#[path = "support/profile_stubs.rs"]
mod profile_stubs;

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

/// Whether `text` carries something shaped like a console token (64 hex
/// digits in a row), the check that a refused setup printed none.
fn shows_a_token(text: &str) -> bool {
    text.as_bytes()
        .windows(64)
        .any(|w| w.iter().all(u8::is_ascii_hexdigit))
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

/// What trunk does on the server under test (the browser client, #327).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Trunk {
    /// Not installed.
    Absent,
    /// Builds `index.html` into its `--dist`.
    Builds,
    /// Installed, but the build fails.
    Fails,
}

/// The server a test deploys to.
#[derive(Clone, Copy, Debug)]
struct Server {
    feedback_installed: bool,
    npm: Npm,
    trunk: Trunk,
    /// Whether rustup lists the `wasm32-unknown-unknown` target.
    wasm_target: bool,
    /// A game running here, so `finish` waits; otherwise it swaps.
    game_running: bool,
    /// The deploy keeps games (`BAYLEE_DEPLOY_KEEP_GAMES=1`, an agent unit
    /// that reads its list of engines), and what `/health` says of them.
    keep: Option<Keeping>,
    /// A Tower here (`deploy/tower.rs`): its unit is installed, and how it fares.
    tower: Option<TowerCase>,
}

/// How the Tower authority fares in a deploy (`deploy/tower.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TowerCase {
    /// Its documents are in place and the new build answers `ready`.
    Ready,
    /// Neither the new build nor the previous one answers `ready`.
    Unhealthy,
    /// The schema owner's documents are missing.
    NoDocuments,
}

/// What `/health` says of the running games to a deploy that keeps them.
#[derive(Clone, Copy, Debug)]
struct Keeping {
    /// How many of the one running game a restart brings back.
    kept: u8,
    /// How many have a seat bridge.
    bridged: u8,
    /// Whether `deploy.env` names the Tower cohort's pinned engine.
    tower_pin: bool,
}

/// The pinned Tower engine a test names (`BAYLEE_DEPLOY_TOWER_ENGINE`).
const TOWER_PIN: &str = "/opt/baylee/engines/tower-0123456789/baylee-engine-tower";

/// The release deployed before: the build the running game plays on.
const DEPLOYED: &str = "fedcba9876543210fedcba9876543210fedcba98";

impl Server {
    /// The server the feedback tests were written against: no browser
    /// toolchain, and a game running, so the gateway is never swapped.
    fn with_feedback(feedback_installed: bool, npm: Npm) -> Self {
        Self {
            feedback_installed,
            npm,
            trunk: Trunk::Absent,
            wasm_target: false,
            game_running: true,
            keep: None,
            tower: None,
        }
    }

    /// One that can build the browser client.
    fn with_trunk(trunk: Trunk, game_running: bool) -> Self {
        Self {
            feedback_installed: false,
            npm: Npm::Absent,
            trunk,
            wasm_target: true,
            game_running,
            keep: None,
            tower: None,
        }
    }
}

/// Runs `stage` in a scratch tree; the deploy root and every command it
/// ran, one per line.
fn stage(feedback_installed: bool, npm: Npm) -> (PathBuf, String) {
    let (root, ran, _) = stage_on(Server::with_feedback(feedback_installed, npm));
    (root, ran)
}

/// The stand-ins that say what the machine is: whether the feedback and
/// Tower units are installed (asked with `systemctl cat`) and whether a game
/// runs (with one running `finish` waits and the gateway is left alone).
fn machine_stubs(stubs: &Path, feedback_installed: bool, tower: bool) {
    stub(
        stubs,
        "systemctl",
        &format!(
            r#"[ "$1 $2" = "show baylee-agent" ] && echo "${{AGENT_ENV_FILES:-}}" && exit 0
[ "$1 $2" = "cat baylee-tower" ] && exit {}
[ "$1 $2" = "cat baylee-feedback" ] && exit {}; exit 0"#,
            u8::from(!tower),
            u8::from(!feedback_installed)
        ),
    );
    native_command::install(stubs, &["curl"]);
}

/// The stand-ins for the browser client's toolchain (#327): trunk writes the
/// page into `--dist` and says where it ran (a failing build writes one too
/// before it fails, so only its exit status can tell), and rustup lists the
/// wasm target or not.
fn browser_stubs(stubs: &Path, trunk: Trunk, wasm_target: bool) {
    stub(
        stubs,
        "trunk",
        &format!(
            r#"[ "$1" = --version ] && {{ echo "trunk 0.21.14"; exit 0; }}
echo "trunk ran in $PWD" >> "$DEPLOY_CALLS"
while [ $# -gt 0 ]; do [ "$1" = --dist ] && dist=$2; shift; done
mkdir -p "$dist" && echo '<script src="/play/baylee-client-0123456789abcdef.js">' > "$dist/index.html"
exit {}"#,
            u8::from(trunk == Trunk::Fails)
        ),
    );
    stub(
        stubs,
        "rustup",
        &format!(
            r#"[ "$1 $2 $3" = "target list --installed" ] && echo aarch64-unknown-linux-gnu {}; exit 0"#,
            if wasm_target {
                "&& echo wasm32-unknown-unknown"
            } else {
                ""
            }
        ),
    );
}

/// sudo, which does nothing, or with `reads` runs the commands that read and
/// append the settings (and stands in for the rest).
fn sudo_stub(stubs: &Path, reads: bool) {
    stub(
        stubs,
        "sudo",
        if reads {
            r#"[ "$1" = "${SUDO_REFUSES:-}" ] && exit 1
case "$1" in test|sed|grep|tee|chmod) exec "$@" ;; esac; exit 0"#
        } else {
            "exit 0"
        },
    );
}

/// The same on any server; also what `stage` said.
fn stage_on(server: Server) -> (PathBuf, String, String) {
    stage_in(server, None)
}

/// [`stage_on`] with the services' settings in `etc` (`BAYLEE_DEPLOY_ETC`),
/// where sudo really reads and appends: it runs `test`, `sed`, `grep`, `tee`
/// and `chmod` and stands in for everything else, as on any other run.
fn stage_in(server: Server, etc: Option<&Path>) -> (PathBuf, String, String) {
    stage_refusing(server, etc, "")
}

fn stage_files(root: &Path) {
    for dir in [
        "src/target/release",
        "src/web/feedback",
        "src/crates/baylee-client",
        "src/scripts/server",
        "bin",
        "state",
    ] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    for binary in [
        "baylee-gateway",
        "baylee-catalog",
        "run-deploy-hooks",
        "baylee-agent",
        "baylee-engine-server",
        "baylee-deploy",
        "baylee-invite",
        "baylee-reboot",
        "baylee-tower-service",
        "baylee-tower-provision",
    ] {
        std::fs::write(root.join("src/target/release").join(binary), b"").unwrap();
    }
    std::fs::write(
        root.join("src/scripts/server/baylee-deploy-hooks.sudoers"),
        b"",
    )
    .unwrap();
}

/// [`stage_in`] where sudo refuses to run `refused` (a sudoers rule that
/// does not allow it).
fn stage_refusing(server: Server, etc: Option<&Path>, refused: &str) -> (PathBuf, String, String) {
    let (out, root, ran) = stage_run(server, etc, refused, false);
    assert_stage_success(&out);
    (root, ran, String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Runs `stage` and answers how it ended and what it ran.
fn stage_run(
    server: Server,
    etc: Option<&Path>,
    refused: &str,
    profile_fails: bool,
) -> (std::process::Output, PathBuf, String) {
    // Tests run at once, some with the same arguments: each run its own tree.
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let Server {
        feedback_installed,
        npm,
        trunk,
        wasm_target,
        game_running,
        keep,
        tower,
    } = server;
    let scratch = std::env::temp_dir().join(format!(
        "baylee-deploy-test-{}-{}-{feedback_installed}-{npm:?}-{trunk:?}",
        std::process::id(),
        RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    let (root, stubs) = (scratch.join("opt"), scratch.join("stubs"));
    stage_files(&root);
    if keep.is_some() {
        std::fs::write(root.join("state/deployed"), format!("{DEPLOYED}\n")).unwrap();
    }
    // The Tower's root-only documents (`deploy/tower.rs`), which only sudo's
    // `test` reads here.
    if tower.is_some_and(|case| case != TowerCase::NoDocuments) {
        let documents = root.join("tower/etc");
        std::fs::create_dir_all(&documents).unwrap();
        for name in ["service.json", "migrate.json", "grants.json"] {
            std::fs::write(documents.join(name), b"{}").unwrap();
        }
    }
    std::fs::create_dir_all(&stubs).unwrap();
    let calls: PathBuf = scratch.join("calls");

    stub(
        &stubs,
        "git",
        r#"[ "$1" = rev-parse ] && echo 0123456789abcdef0123; exit 0"#,
    );
    stub(&stubs, "cargo", profile_stubs::CARGO);
    profile_stubs::install(&stubs, stub);
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
    sudo_stub(&stubs, true);
    stub(&stubs, "logger", "exit 0");
    stub(&stubs, "flock", "exit 0");
    machine_stubs(&stubs, feedback_installed, tower.is_some());
    browser_stubs(&stubs, trunk, wasm_target);

    let npm_path = if npm == Npm::Absent {
        scratch.join("no-such-npm")
    } else {
        stubs.join("npm")
    };
    let trunk_path = if trunk == Trunk::Absent {
        scratch.join("no-such-trunk")
    } else {
        stubs.join("trunk")
    };

    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-deploy"));
    let no_etc = scratch.join("no-etc");
    std::fs::create_dir_all(&no_etc).unwrap();
    std::fs::write(
        no_etc.join("gateway.env"),
        format!("BAYLEE_ADMISSION_HOLD={}\n", scratch.join("hold").display()),
    )
    .unwrap();
    command.env("BAYLEE_DEPLOY_ETC", etc.unwrap_or(&no_etc));
    command.env("SUDO_REFUSES", refused);
    let health = health_and_keeping(
        &mut command,
        game_running,
        keep,
        &etc.unwrap_or(&no_etc).join("agent-engines.env"),
    );
    if profile_fails {
        command.env("PROFILE_FAILS", "1");
    }
    if let Some(case) = tower {
        command.env(
            "BAYLEE_DEPLOY_TOWER_HEALTH",
            "http://127.0.0.1:9/tower-health",
        );
        command.env("BAYLEE_DEPLOY_INFO_TRIES", "2");
        command.env(
            "OPS_FAKE_TOWER",
            if case == TowerCase::Unhealthy {
                "starting"
            } else {
                "ready"
            },
        );
    }
    let out = command
        .args(["stage", "0123456789abcdef0123"])
        .env("BAYLEE_DEPLOY_ROOT", &root)
        .env("CARGO", stubs.join("cargo"))
        .env("RUSTC", stubs.join("rustc"))
        .env("NPM", &npm_path)
        .env("TRUNK", &trunk_path)
        .env("RUSTUP", stubs.join("rustup"))
        .env("DEPLOY_CALLS", &calls)
        .env("OPS_FAKE_CALLS", &calls)
        .env("OPS_FAKE_HEALTH", health.to_string())
        .env(
            "PATH",
            format!(
                "{}:{}",
                stubs.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .expect("baylee-deploy runs");
    let mut ran = std::fs::read_to_string(&calls).unwrap_or_default();
    // What the agent is told to install, written before sudo puts it in place.
    if let Ok(list) = std::fs::read_to_string(root.join("state/agent-engines.env")) {
        ran.push_str(&list);
    }
    let _ = std::fs::remove_dir_all(&scratch);
    (out, root, ran)
}

/// What `/health` answers, and with `keep`, the setting and an agent unit
/// that reads `agent_env` (`deploy/keep.rs`).
fn health_and_keeping(
    command: &mut std::process::Command,
    game_running: bool,
    keep: Option<Keeping>,
    agent_env: &Path,
) -> serde_json::Value {
    let mut health = serde_json::json!({
        "admission": "held", "games": {
            "running": u8::from(game_running), "local_running": u8::from(game_running), "waiting": 0,
        }
    });
    if let Some(Keeping {
        kept,
        bridged,
        tower_pin,
    }) = keep
    {
        command.env("BAYLEE_DEPLOY_KEEP_GAMES", "1");
        if tower_pin {
            command.env("BAYLEE_DEPLOY_TOWER_ENGINE", TOWER_PIN);
        }
        command.env(
            "AGENT_ENV_FILES",
            format!("{} (ignore_errors=yes)", agent_env.display()),
        );
        health["games"]["kept"] = kept.into();
        health["games"]["bridged"] = bridged.into();
        health["games"]["engine_revisions"] = serde_json::json!([DEPLOYED]);
    }
    health
}

fn assert_stage_success(out: &std::process::Output) {
    assert!(
        out.status.success(),
        "stage failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The lines of `ran` that install the browser client into `play`.
fn play_installs(ran: &str, root: &Path) -> Vec<String> {
    let play = root.join("web/play");
    let play = play.display().to_string();
    ran.lines()
        .filter(|l| l.starts_with("sudo ") && l.contains(&play))
        .map(str::to_string)
        .collect()
}

/// With trunk and the wasm target, stage builds the browser client from the
/// client's directory, `--release` under `/play/`, into the stage; with a
/// game running nothing is installed yet, because the client goes live with
/// the gateway it talks to.
#[test]
fn stage_builds_the_browser_client_into_the_stage() {
    let (root, ran, _) = stage_on(Server::with_trunk(Trunk::Builds, true));
    let staged = root.join("state/staged/play");
    let build = format!(
        "trunk build index.html --release --locked --public-url /play/ --dist {}",
        staged.display()
    );
    assert!(ran.lines().any(|l| l == build), "no `{build}` in:\n{ran}");
    let client = root.join("src/crates/baylee-client");
    assert!(
        ran.lines()
            .any(|l| l == format!("trunk ran in {}", client.display())),
        "{ran}"
    );
    assert_eq!(play_installs(&ran, &root), Vec::<String>::new(), "{ran}");
}

/// When finish swaps the gateway it installs the staged client first, as a
/// copy beside the old one and one rename, before the gateway restarts.
#[test]
fn finish_installs_the_browser_client_with_the_gateway() {
    let (root, ran, _) = stage_on(Server::with_trunk(Trunk::Builds, false));
    let lines: Vec<&str> = ran.lines().collect();
    let at = |wanted: &str| {
        lines
            .iter()
            .position(|l| *l == wanted)
            .unwrap_or_else(|| panic!("no `{wanted}` in:\n{ran}"))
    };
    let play = root.join("web/play");
    let play = play.display();
    let staged = root.join("state/staged/play");
    let build = at(&format!(
        "trunk build index.html --release --locked --public-url /play/ --dist {}",
        staged.display()
    ));
    let copy = at(&format!("sudo cp -R {} {play}.new", staged.display()));
    let swap = at(&format!("sudo mv {play}.new {play}"));
    let restart = at("sudo systemctl restart baylee-gateway");
    // The long wasm build runs while the agent still starts games.
    let stop = at("sudo systemctl stop baylee-agent");
    assert!(
        build < stop && build < copy && copy < swap && swap < restart,
        "{ran}"
    );
}

/// Without trunk, without the wasm target, or with a build that fails, the
/// backend is deployed all the same and the installed client is left alone.
#[test]
fn without_the_browser_toolchain_or_with_a_failed_build_the_backend_still_deploys() {
    let cases = [
        (Server::with_trunk(Trunk::Absent, false), "no trunk here"),
        (
            Server {
                wasm_target: false,
                ..Server::with_trunk(Trunk::Builds, false)
            },
            "no wasm32-unknown-unknown target here",
        ),
        (
            Server::with_trunk(Trunk::Fails, false),
            "the browser client did not build",
        ),
    ];
    for (server, said) in cases {
        let (root, ran, out) = stage_on(server);
        assert!(out.contains(said), "{server:?}: said\n{out}");
        assert_eq!(
            play_installs(&ran, &root),
            Vec::<String>::new(),
            "{server:?}: a client was installed:\n{ran}"
        );
        assert!(
            ran.lines()
                .any(|l| l == "sudo systemctl restart baylee-gateway"),
            "{server:?}: the gateway was not deployed:\n{ran}"
        );
        assert!(
            out.contains("deployed 0123456789"),
            "{server:?}: no deploy:\n{out}"
        );
        assert_eq!(
            ran.contains("trunk build"),
            server.trunk != Trunk::Absent && server.wasm_target,
            "{server:?}:\n{ran}"
        );
    }
}

/// The engine server is built from a profile of the commit being deployed, as
/// a step of its own after the plain build, and needs the toolchain's tool.
#[test]
fn stage_profiles_the_engine_server_from_this_commit() {
    let (_, ran, said) = stage_on(Server::with_feedback(false, Npm::Builds));
    assert!(ran.contains("rustup component add llvm-tools"), "{ran}");
    let builds: Vec<&str> = ran
        .lines()
        .filter(|l| l.starts_with("cargo build"))
        .collect();
    assert!(
        builds.len() >= 10,
        "base build plus three isolated training, shipping and proof builds:\n{ran}"
    );
    assert!(
        said.contains("all three engine variants") && !said.contains("without a profile"),
        "every engine variant, Tower too, ships from its profile:\n{said}"
    );
    assert!(builds[0].contains("-p baylee-gateway"), "{ran}");
    assert!(
        builds
            .iter()
            .any(|b| b.contains("--bin baylee-engine-server")),
        "{ran}"
    );
    assert!(ran.contains("llvm-profdata merge"), "{ran}");
    for artifact in baylee_pgo::EngineArtifact::ALL {
        let args = artifact.cargo_args().join(" ");
        assert_eq!(
            builds.iter().filter(|build| build.contains(&args)).count(),
            3,
            "{} trains, ships and proves with identical features: {ran}",
            artifact.name()
        );
    }
}

/// A release never goes out unprofiled by accident: a profile step that fails
/// stops the deploy before anything is staged or installed.
#[test]
fn a_failed_profile_stops_the_deploy_before_anything_is_staged() {
    let server = Server {
        feedback_installed: false,
        npm: Npm::Builds,
        trunk: Trunk::Absent,
        wasm_target: false,
        game_running: false,
        keep: None,
        tower: None,
    };
    let (out, _, ran) = stage_run(server, None, "", true);
    assert!(!out.status.success(), "stage went on:\n{ran}");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("the profile-guided build stopped"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        !ran.lines().any(|l| l.starts_with("sudo install")),
        "something was installed:\n{ran}"
    );
    assert!(
        !ran.contains("systemctl restart"),
        "something was restarted:\n{ran}"
    );
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
        assert!(!ran.contains("sudo systemctl stop baylee-agent"));
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

/// `stage` installs `baylee-invite` beside itself. It only says the closed
/// beta's keys are gone now, and is installed so that it replaces the
/// wrapper an earlier release put there (`baylee_ops::invite`).
#[test]
fn finish_installs_the_staged_invite_command() {
    let (root, ran, _) = stage_on(Server::with_trunk(Trunk::Absent, false));
    let expected = format!(
        "sudo install -m755 {} /usr/local/sbin/baylee-invite",
        root.join("state/staged/baylee-invite").display()
    );
    assert!(ran.lines().any(|l| l == expected), "{ran}");
}

/// `baylee-invite` made the closed beta's keys; they are gone with the
/// beta (10.11.2026). Asked anything, it says so and runs nothing, not
/// even the gateway, with a database named and a gateway in place.
#[test]
fn the_invite_command_says_the_keys_are_gone_and_runs_nothing() {
    let scratch = std::env::temp_dir().join(format!("baylee-invite-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let etc = scratch.join("etc");
    std::fs::create_dir_all(&etc).unwrap();
    let ran = scratch.join("ran");
    let gateway = scratch.join("baylee-gateway");
    std::fs::write(
        &gateway,
        format!("#!/usr/bin/env bash\ntouch {}\n", ran.display()),
    )
    .unwrap();
    std::fs::set_permissions(&gateway, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(
        etc.join("secrets.env"),
        "DATABASE_URL=postgres://baylee:secret@127.0.0.1/baylee\n",
    )
    .unwrap();
    std::fs::write(etc.join("gateway.env"), "BAYLEE_REGISTRATION=open\n").unwrap();
    for args in [
        &[][..],
        &["list"][..],
        &["create", "--note", "Max und Moritz"][..],
    ] {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-invite"))
            .args(args)
            .env("BAYLEE_INVITE_ETC", &etc)
            .env("BAYLEE_INVITE_BIN", &gateway)
            .output()
            .expect("baylee-invite runs");
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {said}");
        assert!(said.contains("the closed beta's keys are gone"), "{said}");
        assert!(said.contains("BAYLEE_REGISTRATION=open"), "{said}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(!ran.exists(), "{args:?}: the gateway was run");
    }
    let _ = std::fs::remove_dir_all(&scratch);
}

/// The admin console's token (`docs/feedback.md` §"The admin console"):
/// stage makes it once, writes the same one to the gateway's and the feedback
/// service's settings, both 0600, with the console's address, and never
/// says it; a second stage changes nothing, and a token one file already
/// has is copied to the other rather than replaced.
#[test]
fn stage_sets_up_one_console_token_for_both_services_and_never_prints_it() {
    use std::os::unix::fs::PermissionsExt as _;
    let etc = std::env::temp_dir().join(format!("baylee-deploy-etc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&etc);
    std::fs::create_dir_all(&etc).unwrap();
    let (gw, fb) = (etc.join("gateway.env"), etc.join("feedback.env"));
    std::fs::write(&gw, "BAYLEE_REGISTRATION=open\n").unwrap();
    std::fs::write(&fb, "FEEDBACK_DATABASE_URL=postgres://f@127.0.0.1/f\n").unwrap();
    let server = Server::with_feedback(true, Npm::Absent);

    let (_, ran, said) = stage_in(server, Some(&etc));
    let gateway = std::fs::read_to_string(&gw).unwrap();
    let feedback = std::fs::read_to_string(&fb).unwrap();
    // Made from the system's randomness: 32 bytes as hex.
    let token = gateway
        .lines()
        .find_map(|l| l.strip_prefix("BAYLEE_ADMIN_TOKEN="))
        .expect("a token was written")
        .to_owned();
    assert!(
        token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()),
        "{token}"
    );
    assert_eq!(
        gateway,
        format!("BAYLEE_REGISTRATION=open\nBAYLEE_ADMIN_TOKEN={token}\n")
    );
    assert_eq!(
        feedback,
        format!(
            "FEEDBACK_DATABASE_URL=postgres://f@127.0.0.1/f\n\
             FEEDBACK_GATEWAY_ADMIN_TOKEN={token}\n\
             FEEDBACK_GATEWAY_ADMIN_URL=http://127.0.0.1:28767\n"
        )
    );
    for file in [&gw, &fb] {
        let mode = std::fs::metadata(file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "{}", file.display());
    }
    assert!(!ran.contains(&token), "on a command line: {ran}");
    assert!(!said.contains(&token), "printed: {said}");
    assert!(said.contains("admin console token made"), "{said}");
    // Before the service restarts, so it starts with its half.
    let wrote = ran.find("sudo tee -a").expect("written");
    let restart = ran
        .find("sudo systemctl restart baylee-feedback")
        .expect("restarted");
    assert!(wrote < restart, "{ran}");

    // Again: nothing changes, nothing is made.
    let (_, _, said) = stage_in(server, Some(&etc));
    assert_eq!(std::fs::read_to_string(&gw).unwrap(), gateway);
    assert_eq!(std::fs::read_to_string(&fb).unwrap(), feedback);
    assert!(!said.contains("token made"), "{said}");

    // A gateway that has one already lends it to the service.
    let chosen = "one-the-operator-chose-0123456789abcdef";
    std::fs::write(&gw, format!("BAYLEE_ADMIN_TOKEN={chosen}\n")).unwrap();
    std::fs::write(&fb, "FEEDBACK_DATABASE_URL=x\n").unwrap();
    let _ = stage_in(server, Some(&etc));
    assert!(
        std::fs::read_to_string(&fb)
            .unwrap()
            .contains(&format!("FEEDBACK_GATEWAY_ADMIN_TOKEN={chosen}\n"))
    );
    assert_eq!(
        std::fs::read_to_string(&gw).unwrap(),
        format!("BAYLEE_ADMIN_TOKEN={chosen}\n"),
        "never replaced"
    );
    let _ = std::fs::remove_dir_all(&etc);
}

/// A server whose sudo does not allow a command the token needs still
/// deploys: the stage says so and restarts the service without a console
/// (`stage_refusing` asserts the stage itself succeeded).
#[test]
fn a_token_that_cannot_be_set_up_does_not_stop_the_deploy() {
    for refused in ["sed", "tee", "chmod"] {
        let etc = std::env::temp_dir().join(format!(
            "baylee-deploy-etc-{refused}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&etc);
        std::fs::create_dir_all(&etc).unwrap();
        std::fs::write(etc.join("gateway.env"), "BAYLEE_REGISTRATION=open\n").unwrap();
        std::fs::write(etc.join("feedback.env"), "FEEDBACK_DATABASE_URL=x\n").unwrap();
        let server = Server::with_feedback(true, Npm::Absent);
        let (_, ran, said) = stage_refusing(server, Some(&etc), refused);
        assert!(said.contains("token was not set up"), "{refused}: {said}");
        assert!(
            ran.contains("sudo systemctl restart baylee-feedback"),
            "{refused}: {ran}"
        );
        assert!(!shows_a_token(&said), "{refused}: {said}");
        let _ = std::fs::remove_dir_all(&etc);
    }
}

/// A deploy that keeps games does not wait for a running game every restart
/// brings back: it keeps the engines that game plays on, stops the gateway
/// first (which stores the game) and only then the agent, and names the kept
/// engines in the agent's list before the agent starts again.
#[test]
fn a_deploy_that_keeps_games_stops_the_gateway_first_and_keeps_their_engines() {
    let server = Server {
        keep: Some(Keeping {
            kept: 1,
            bridged: 0,
            tower_pin: false,
        }),
        ..Server::with_trunk(Trunk::Absent, true)
    };
    let (root, ran, said) = stage_on(server);
    let lines: Vec<&str> = ran.lines().collect();
    let at = |wanted: &dyn Fn(&str) -> bool, what: &str| {
        lines
            .iter()
            .position(|l| wanted(l))
            .unwrap_or_else(|| panic!("no {what} in:\n{ran}\n{said}"))
    };
    let kept = root.join("engines").join(DEPLOYED);
    let keep = at(
        &|l| l == format!("sudo install -d -m755 {}", kept.display()),
        "copy of the deployed engines",
    );
    let gateway = at(
        &|l| l == "sudo systemctl stop baylee-gateway",
        "gateway stop",
    );
    let agent = at(&|l| l == "sudo systemctl stop baylee-agent", "agent stop");
    let list = at(
        &|l| l.starts_with("sudo install -m644 ") && l.ends_with("/agent-engines.env"),
        "the agent's list",
    );
    let restart = at(
        &|l| l == "sudo systemctl restart baylee-gateway",
        "gateway start",
    );
    let start = at(&|l| l == "sudo systemctl start baylee-agent", "agent start");
    assert!(
        keep < gateway && gateway < agent && agent < list && list < restart && restart < start,
        "{ran}"
    );
    let bin = root.join("bin");
    assert!(
        ran.contains(&format!(
            "BAYLEE_ENGINE_BINS={b}/baylee-engine-standard-loops:{b}/baylee-engine-tower:\
             {k}/baylee-engine-server:{k}/baylee-engine-standard-loops\n",
            b = bin.display(),
            k = kept.display()
        )),
        "{ran}"
    );
}

/// It waits as before, and says why, for a running game a restart would not
/// bring back, and for one with a seat bridge, which the release replaces.
#[test]
fn a_deploy_that_keeps_games_waits_for_one_it_cannot_keep() {
    for (keeping, why) in [
        (
            Keeping {
                kept: 0,
                bridged: 0,
                tower_pin: false,
            },
            "1 of 1 running game(s) would not be kept",
        ),
        (
            Keeping {
                kept: 1,
                bridged: 1,
                tower_pin: false,
            },
            "1 game(s) with a seat bridge",
        ),
    ] {
        let server = Server {
            keep: Some(keeping),
            ..Server::with_trunk(Trunk::Absent, true)
        };
        let (_, ran, said) = stage_on(server);
        assert!(said.contains(why), "{why}:\n{said}");
        assert!(!ran.contains("systemctl stop baylee-gateway"), "{ran}");
        assert!(!ran.contains("systemctl restart baylee-gateway"), "{ran}");
    }
}

/// The Tower cohort's pinned engine stands in for the deployed Tower engine in
/// a keeping deploy's list, as the agent's `tower-engine.conf` names it while
/// no deploy keeps games (`docs/tower-operations.md`): the list replaces that
/// drop-in, and without the pin the cohort's runs would find no host.
#[test]
fn a_keeping_deploy_names_the_tower_cohorts_pinned_engine() {
    let server = Server {
        keep: Some(Keeping {
            kept: 1,
            bridged: 0,
            tower_pin: true,
        }),
        ..Server::with_trunk(Trunk::Absent, true)
    };
    let (root, ran, _) = stage_on(server);
    let bin = root.join("bin");
    assert!(
        ran.contains(&format!(
            "BAYLEE_ENGINE_BINS={}/baylee-engine-standard-loops:{TOWER_PIN}:",
            bin.display()
        )),
        "{ran}"
    );
    let list = ran
        .lines()
        .find(|line| line.starts_with("BAYLEE_ENGINE_BINS="))
        .expect("the agent's list");
    assert!(
        !list.contains(&format!("{}/baylee-engine-tower", bin.display())),
        "the deployed Tower engine is not named beside the pin: {list}"
    );
}

/// A server that runs the Tower authority (`deploy/tower.rs`), with no game
/// running, so `finish` swaps.
fn tower_server(case: TowerCase) -> Server {
    Server {
        tower: Some(case),
        ..Server::with_trunk(Trunk::Absent, false)
    }
}

/// The Tower is built with the release and switched right after the gateway
/// it serves and before the agent starts again: the new build checks the
/// service's configuration beside the running one, then the service stops,
/// the schema owner's documents migrate and renew the grants, the new build
/// takes the names and starts.
#[test]
fn a_tower_here_is_built_and_switched_right_after_the_gateway() {
    let (_, ran, said) = stage_on(tower_server(TowerCase::Ready));
    let lines: Vec<&str> = ran.lines().collect();
    let at = |wanted: &str| {
        lines
            .iter()
            .position(|line| line.contains(wanted))
            .unwrap_or_else(|| panic!("never ran {wanted}:\n{ran}"))
    };
    assert!(
        lines.iter().any(|line| line.starts_with("cargo build")
            && line.contains("-p baylee-tower-service -p baylee-tower-provision")),
        "{ran}"
    );
    let order = [
        at("systemctl restart baylee-gateway"),
        at("baylee-tower-service.next --check"),
        at("systemctl stop baylee-tower"),
        at("baylee-tower-service.next --migrate"),
        at("baylee-tower-service.next --admin"),
        at("mv -f"),
        at("systemctl start baylee-tower"),
        at("systemctl start baylee-agent"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{ran}");
    assert!(
        ran.contains("StandardInput=file:") && ran.contains("User=baylee-tower"),
        "the documents are read by systemd-run as the service's user: {ran}"
    );
    assert!(
        said.contains("Tower service") && said.contains("installed"),
        "{said}"
    );
}

#[test]
fn without_a_tower_unit_nothing_of_it_is_built_or_touched() {
    let (_, ran, _) = stage_on(Server::with_trunk(Trunk::Absent, false));
    assert!(!ran.contains("baylee-tower"), "{ran}");
}

/// A Tower that cannot switch says why and leaves the deploy to go on: the
/// gateway and the games do not wait for it.
#[test]
fn a_tower_without_its_documents_or_that_never_answers_does_not_stop_the_deploy() {
    for (case, why) in [
        (TowerCase::NoDocuments, "is missing"),
        (TowerCase::Unhealthy, "did not answer ready"),
    ] {
        let (_, ran, said) = stage_on(tower_server(case));
        assert!(
            said.contains("the Tower service was not switched") && said.contains(why),
            "{case:?}: {said}"
        );
        assert!(
            ran.contains("systemctl start baylee-agent"),
            "{case:?}: {ran}"
        );
        match case {
            TowerCase::NoDocuments => {
                assert!(!ran.contains("systemctl stop baylee-tower"), "{ran}");
            }
            TowerCase::Unhealthy => {
                assert!(ran.contains("baylee-tower-service.prev"), "{ran}");
                assert!(ran.contains("systemctl restart baylee-tower"), "{ran}");
                assert!(said.contains("does not answer either"), "{said}");
            }
            TowerCase::Ready => unreachable!(),
        }
    }
}
