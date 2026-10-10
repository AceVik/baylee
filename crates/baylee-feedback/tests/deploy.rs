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

/// What the stand-in `openssl rand -hex 32` makes.
const CONSOLE_TOKEN: &str = "c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00";

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
}

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
        }
    }
}

/// Runs `stage` in a scratch tree; the deploy root and every command it
/// ran, one per line.
fn stage(feedback_installed: bool, npm: Npm) -> (PathBuf, String) {
    let (root, ran, _) = stage_on(Server::with_feedback(feedback_installed, npm));
    (root, ran)
}

/// The stand-ins that say what the machine is: whether the feedback unit is
/// installed (asked with `systemctl cat`) and whether a game runs (with one
/// running `finish` waits and the gateway is left alone).
fn machine_stubs(stubs: &Path, feedback_installed: bool, game_running: bool) {
    stub(
        stubs,
        "systemctl",
        &format!(
            r#"[ "$1 $2" = "cat baylee-feedback" ] && exit {}; exit 0"#,
            u8::from(!feedback_installed)
        ),
    );
    stub(
        stubs,
        "curl",
        &format!(
            r#"echo '{{"games":{{"running":{0},"local_running":{0},"waiting":0}}}}'"#,
            u8::from(game_running)
        ),
    );
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

/// [`stage_in`] where sudo refuses to run `refused` (a sudoers rule that
/// does not allow it).
fn stage_refusing(server: Server, etc: Option<&Path>, refused: &str) -> (PathBuf, String, String) {
    // Tests run at once, some with the same arguments: each run its own tree.
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let Server {
        feedback_installed,
        npm,
        trunk,
        wasm_target,
        game_running,
    } = server;
    let scratch = std::env::temp_dir().join(format!(
        "baylee-deploy-test-{}-{}-{feedback_installed}-{npm:?}-{trunk:?}",
        std::process::id(),
        RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    let (root, stubs) = (scratch.join("opt"), scratch.join("stubs"));
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
    std::fs::create_dir_all(&stubs).unwrap();
    for binary in ["baylee-gateway", "baylee-catalog", "run-deploy-hooks"] {
        std::fs::write(root.join("src/target/release").join(binary), b"").unwrap();
    }
    std::fs::write(
        root.join("src/scripts/server/baylee-deploy-hooks.sudoers"),
        b"",
    )
    .unwrap();
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
    sudo_stub(&stubs, etc.is_some());
    stub(&stubs, "logger", "exit 0");
    stub(&stubs, "openssl", &format!("echo {CONSOLE_TOKEN}"));
    stub(&stubs, "flock", "exit 0");
    machine_stubs(&stubs, feedback_installed, game_running);
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
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/server/baylee-deploy");
    let mut command = std::process::Command::new("bash");
    let no_etc = scratch.join("no-etc");
    command.env("BAYLEE_DEPLOY_ETC", etc.unwrap_or(&no_etc));
    command.env("SUDO_REFUSES", refused);
    let out = command
        .arg(&script)
        .args(["stage", "0123456789abcdef0123"])
        .env("BAYLEE_DEPLOY_ROOT", &root)
        .env("CARGO", stubs.join("cargo"))
        .env("NPM", &npm_path)
        .env("TRUNK", &trunk_path)
        .env("RUSTUP", stubs.join("rustup"))
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
    (root, ran, String::from_utf8_lossy(&out.stdout).into_owned())
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
    std::fs::write(&gw, "BAYLEE_REGISTRATION=invite\n").unwrap();
    std::fs::write(&fb, "FEEDBACK_DATABASE_URL=postgres://f@127.0.0.1/f\n").unwrap();
    let server = Server::with_feedback(true, Npm::Absent);

    let (_, ran, said) = stage_in(server, Some(&etc));
    let gateway = std::fs::read_to_string(&gw).unwrap();
    let feedback = std::fs::read_to_string(&fb).unwrap();
    assert_eq!(
        gateway,
        format!("BAYLEE_REGISTRATION=invite\nBAYLEE_ADMIN_TOKEN={CONSOLE_TOKEN}\n")
    );
    assert_eq!(
        feedback,
        format!(
            "FEEDBACK_DATABASE_URL=postgres://f@127.0.0.1/f\n\
             FEEDBACK_GATEWAY_ADMIN_TOKEN={CONSOLE_TOKEN}\n\
             FEEDBACK_GATEWAY_ADMIN_URL=http://127.0.0.1:28767\n"
        )
    );
    for file in [&gw, &fb] {
        let mode = std::fs::metadata(file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "{}", file.display());
    }
    assert!(!ran.contains(CONSOLE_TOKEN), "on a command line: {ran}");
    assert!(!said.contains(CONSOLE_TOKEN), "printed: {said}");
    assert!(said.contains("admin console token made"), "{said}");
    // Before the service restarts, so it starts with its half.
    let wrote = ran.find("sudo tee -a").expect("written");
    let restart = ran
        .find("sudo systemctl restart baylee-feedback")
        .expect("restarted");
    assert!(wrote < restart, "{ran}");

    // Again: nothing changes, nothing is made.
    let (_, ran, said) = stage_in(server, Some(&etc));
    assert_eq!(std::fs::read_to_string(&gw).unwrap(), gateway);
    assert_eq!(std::fs::read_to_string(&fb).unwrap(), feedback);
    assert!(!ran.contains("openssl"), "{ran}");
    assert!(!said.contains("token made"), "{said}");

    // A gateway that has one already lends it to the service.
    let chosen = "one-the-operator-chose-0123456789abcdef";
    std::fs::write(&gw, format!("BAYLEE_ADMIN_TOKEN={chosen}\n")).unwrap();
    std::fs::write(&fb, "FEEDBACK_DATABASE_URL=x\n").unwrap();
    let (_, ran, _) = stage_in(server, Some(&etc));
    assert!(!ran.contains("openssl"), "{ran}");
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
        std::fs::write(etc.join("gateway.env"), "BAYLEE_REGISTRATION=invite\n").unwrap();
        std::fs::write(etc.join("feedback.env"), "FEEDBACK_DATABASE_URL=x\n").unwrap();
        let server = Server::with_feedback(true, Npm::Absent);
        let (_, ran, said) = stage_refusing(server, Some(&etc), refused);
        assert!(said.contains("token was not set up"), "{refused}: {said}");
        assert!(
            ran.contains("sudo systemctl restart baylee-feedback"),
            "{refused}: {ran}"
        );
        assert!(!said.contains(CONSOLE_TOKEN), "{refused}: {said}");
        let _ = std::fs::remove_dir_all(&etc);
    }
}
