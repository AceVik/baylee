//! The server half of a deploy, installed as `/usr/local/sbin/baylee-deploy`
//! from the tree it deploys, so a release also updates how the next one goes:
//!
//! ```text
//! baylee-deploy stage <rev> [--force]   build <rev>, hold admission, drain
//! baylee-deploy finish [--force]        swap the gateway once no game is open
//! baylee-deploy watch                   stage the newest release tag, finish
//! baylee-deploy status                  what is deployed, staged and running
//! ```
//!
//! A deploy never ends a game unless told to. It goes in two halves:
//!
//! 1. Stage: build and retain the release binaries; hold gateway admission
//!    (`BAYLEE_ADMISSION_HOLD`) so no new game starts. The owner agent stays
//!    alive: stopping or disconnecting it ends the games it owns.
//! 2. Finish: verify `/health` reports admission held, then wait for no local
//!    games and no waiting rooms. Only then stop and replace the owner,
//!    replace/restart the gateway and start the new agent. Until then the
//!    owner stays alive and `watch` asks again every minute.
//!
//! "This machine's" games are those of the agent on the gateway's unix
//! socket (`games.local_running`, `docs/protocol.md` §"On the same machine:
//! the unix socket"). Restarting the gateway still drops the link of a game
//! an agent elsewhere runs; the owner chose not to wait for those. A gateway
//! too old to count local games is waited on for all its running ones.
//! `--force` skips the wait and ends whatever is open. A waiting room nobody
//! starts would hold a deploy forever, so rooms alone stop blocking after
//! `BAYLEE_DEPLOY_ROOM_GRACE` minutes (60); running games always block.
//!
//! The feedback service (`docs/feedback.md`) holds no game, so stage replaces
//! it straight away, with its web UI when npm is there to build it. The
//! browser client (#327) is built by stage when trunk and the wasm target
//! are installed, and installed by finish beside the gateway it talks to;
//! without the toolchain, or when the build fails, the installed one stays.
//!
//! Deploy hooks (`docs/deploy-hooks.md`): when `/etc/baylee/deploy-hooks.d`
//! holds a file named like a hook, the two halves run as a transaction with
//! three hook phases through the root-owned dispatcher
//! `/usr/local/lib/baylee/run-deploy-hooks` (`sudo -n`, nothing but the phase
//! and the full commit):
//!
//! - `prepare`: the target is resolved to its full commit; nothing is
//!   installed, stopped or staged before this passes. Exit 75 ("not ready")
//!   keeps the old system and tries the same target on the next run.
//! - (stage) the admission hold goes up; the owner agent stays alive.
//! - (drain) every running game, on any agent, has to end; only then stop and
//!   replace the owner. `--force` skips this wait and ends its games.
//! - `before-switch`: then the seat bridge, the gateway and the
//!   browser client are installed and the gateway restarts; `/info` has to
//!   name the target commit, clean.
//! - `after-switch`: then the hold comes down, the agent starts and the seat
//!   agents restart, and the target is recorded as deployed.
//!
//! A failed hook stops the deploy where it is, `--force` or not; the next
//! `baylee-deploy finish` by hand runs prepare again and continues. Where it
//! is stands in `$state/transaction`, so a timer run after a crash goes on.
use crate::{
    gateway,
    sh::{self, Run, Stop, cmd, log, must, ok, out, out_quiet, quiet},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

mod maintenance;

const TAG: &str = "baylee-deploy";
const REPO: &str = "https://github.com/AceVik/baylee.git";

fn say(msg: &str) {
    log(TAG, msg);
}

/// Where everything is. `BAYLEE_DEPLOY_ROOT` and `BAYLEE_DEPLOY_ETC` move the
/// tree for the deploy's own tests; a server leaves them unset.
struct Paths {
    etc: PathBuf,
    src: PathBuf,
    bin: PathBuf,
    state: PathBuf,
    /// Where the feedback web UI goes (`FEEDBACK_WEB_DIR`).
    web: PathBuf,
    /// The browser client, Caddy's `root`.
    play: PathBuf,
    hooks_dir: PathBuf,
    hooks_bin: PathBuf,
    hooks_sudoers: PathBuf,
    txn: PathBuf,
    /// Whose the signers file and the keyring must be (root).
    trusted_uid: u32,
    /// How often the new gateway's `/info` is asked for (once a second).
    info_tries: u32,
    room_grace_minutes: u64,
    cargo: String,
    npm: String,
    trunk: String,
    rustup: String,
    /// Build-time settings for the clients built here (`/etc/baylee/deploy.env`,
    /// e.g. `BAYLEE_FEEDBACK_PUBLIC_URL`; no secrets).
    build_env: Vec<(String, String)>,
}

impl Paths {
    fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        let home = PathBuf::from(var("HOME").unwrap_or_else(|| "/root".into()));
        let tool = |name: &str, default: &str| {
            var(name).unwrap_or_else(|| home.join(default).display().to_string())
        };
        let testing = var("BAYLEE_DEPLOY_ROOT").is_some();
        let root = PathBuf::from(var("BAYLEE_DEPLOY_ROOT").unwrap_or_else(|| "/opt/baylee".into()));
        let etc = PathBuf::from(var("BAYLEE_DEPLOY_ETC").unwrap_or_else(|| "/etc/baylee".into()));
        let state = root.join("state");
        let (mut hooks_bin, mut hooks_sudoers) = (
            PathBuf::from("/usr/local/lib/baylee/run-deploy-hooks"),
            PathBuf::from("/etc/sudoers.d/baylee-deploy-hooks"),
        );
        let (mut trusted_uid, mut info_tries) = (0, 20);
        if testing {
            // Only for the deploy tests, which run in a scratch tree as
            // whoever runs them; the dispatcher's own checks have no switch.
            hooks_bin = var("BAYLEE_DEPLOY_HOOKS_BIN").map_or(hooks_bin, PathBuf::from);
            hooks_sudoers = root.join("sudoers.d/baylee-deploy-hooks");
            trusted_uid = var("BAYLEE_DEPLOY_TRUSTED_UID")
                .and_then(|v| v.parse().ok())
                .unwrap_or(trusted_uid);
            info_tries = var("BAYLEE_DEPLOY_INFO_TRIES")
                .and_then(|v| v.parse().ok())
                .unwrap_or(info_tries);
        }
        Self {
            src: root.join("src"),
            bin: root.join("bin"),
            web: root.join("web/feedback"),
            play: root.join("web/play"),
            txn: state.join("transaction"),
            hooks_dir: etc.join("deploy-hooks.d"),
            state,
            etc,
            hooks_bin,
            hooks_sudoers,
            trusted_uid,
            info_tries,
            room_grace_minutes: var("BAYLEE_DEPLOY_ROOM_GRACE")
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            cargo: tool("CARGO", ".cargo/bin/cargo"),
            npm: var("NPM").unwrap_or_else(|| "npm".into()),
            trunk: tool("TRUNK", ".cargo/bin/trunk"),
            rustup: tool("RUSTUP", ".cargo/bin/rustup"),
            build_env: crate::settings::read(Path::new("/etc/baylee/deploy.env")),
        }
    }

    fn gateway_env(&self) -> PathBuf {
        self.etc.join("gateway.env")
    }

    fn staged(&self, name: &str) -> PathBuf {
        self.state.join("staged").join(name)
    }
}

/// Where a deploy under hooks stands (`$state/transaction`).
#[derive(Default, Clone)]
struct Txn {
    target: String,
    phase: String,
    failed: String,
    tag: String,
}

struct Deploy {
    p: Paths,
    /// The release tag being staged by `watch`, recorded in the transaction.
    staging_tag: Option<String>,
}

/// `baylee-deploy <command>`; the exit status.
pub fn main(args: &[String]) -> i32 {
    let mut deploy = Deploy {
        p: Paths::from_env(),
        staging_tag: None,
    };
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match words.as_slice() {
        ["stage", rev] => deploy.lock().and_then(|_held| deploy.stage(rev, false)),
        ["stage", rev, "--force"] => deploy.lock().and_then(|_held| deploy.stage(rev, true)),
        ["finish"] => deploy
            .lock()
            .and_then(|_held| deploy.finish(false, From::Hand)),
        ["finish", "--force"] => deploy
            .lock()
            .and_then(|_held| deploy.finish(true, From::Hand)),
        ["watch"] => deploy.lock().and_then(|_held| deploy.watch()),
        [] | ["status"] => {
            deploy.status();
            Ok(())
        }
        _ => {
            eprintln!(
                "usage: baylee-deploy stage <rev> [--force] | finish [--force] | watch | status"
            );
            Err(Stop(2))
        }
    };
    match result {
        Ok(()) => 0,
        Err(Stop(code)) => code,
    }
}

/// Who asked for a finish: by hand it runs a stopped transaction's prepare
/// again; the timer only reports it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum From {
    Hand,
    Watch,
}

impl Deploy {
    /// Serialises the timer against a hand-started deploy. `status` only
    /// reads and takes no lock: it has to answer while a deploy is building,
    /// which is exactly when someone asks.
    fn lock(&self) -> Run<fs::File> {
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.p.state.join("lock"))
            .map_err(|_| Stop(1))?;
        if file.try_lock().is_err() {
            say("another deploy is running");
            return Err(Stop(0));
        }
        Ok(file)
    }

    /// A command run in the source tree with the build settings.
    fn in_src<S: AsRef<std::ffi::OsStr>>(&self, words: &[S]) -> Command {
        let mut command = cmd(words);
        command
            .current_dir(&self.p.src)
            .env("PWD", &self.p.src)
            .envs(self.p.build_env.iter().cloned());
        command
    }

    fn sudo<S: AsRef<std::ffi::OsStr>>(&self, words: &[S]) -> Command {
        let mut command = self.in_src(&["sudo"]);
        command.args(words);
        command
    }

    fn installed(unit: &str) -> bool {
        quiet(Command::new("systemctl").args(["cat", unit]))
    }

    /// Running games here and waiting rooms, or `None` when the gateway does
    /// not answer, which counts as open. Local games are counted apart only
    /// when the gateway has a unix socket at all: without one this machine's
    /// agent came in over TCP and every local count is 0.
    fn open_games(&self) -> Option<(u64, u64)> {
        // With sudo: the file holds the admin console's token and is 0600.
        let socket = quiet(
            Command::new("sudo")
                .args(["grep", "-qs", "^BAYLEE_UNIX_SOCKET=."])
                .arg(self.p.gateway_env()),
        );
        let field = if socket { "local_running" } else { "running" };
        gateway::fetch("health", 5).and_then(|body| gateway::open_games(&body, field))
    }

    /// Prepares authoritative catalog facts before stopping anything, as
    /// root, the importer reading the gateway's settings itself (secrets.env,
    /// then gateway.env), so neither the URL nor the importer's diagnostics
    /// reach a command line or the log. The importer skips empty and current
    /// catalogs and serialises upgrades. A failure is deliberate: never
    /// switch to rules that need absent facts.
    fn catalog_prepare(&self, importer: &Path) -> Run {
        say("checking the catalog upgrade");
        let status = Command::new("sudo")
            .args(["-n", "timeout", "--kill-after=5s", "15m"])
            .arg(importer)
            .arg("--settings")
            .arg(self.p.etc.join("secrets.env"))
            .arg("--settings")
            .arg(self.p.gateway_env())
            .arg("upgrade")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let code = status.ok().and_then(|s| s.code()).unwrap_or(1);
        if code != 0 {
            say(&format!(
                "catalog upgrade failed (status {code}); deployment stopped"
            ));
            return Err(Stop(code));
        }
        say("catalog upgrade ready");
        Ok(())
    }

    fn stage(&mut self, rev: &str, force: bool) -> Run {
        std::env::set_current_dir(&self.p.src).map_err(|_| Stop(1))?;
        if !quiet(&mut self.in_src(&["git", "cat-file", "-e", &format!("{rev}^{{commit}}")])) {
            must(&mut self.in_src(&["git", "fetch", "-q", "--tags", REPO]))?;
        }
        // A tag named by hand is verified as `watch` verifies one; a commit
        // id is the operator's own choice.
        if quiet(&mut self.in_src(&[
            "git",
            "show-ref",
            "--verify",
            "-q",
            &format!("refs/tags/{rev}"),
        ])) && !self.verify_tag(rev)
        {
            return Err(Stop(1));
        }
        if self.hooks_configured()
            && let Some(txn) = self.txn_load()
            && matches!(
                txn.phase.as_str(),
                "staged" | "draining" | "switching" | "switched"
            )
        {
            // Past the point where anything changed: only this target may go
            // on, and it goes on where it stopped.
            let full = out(&mut self.in_src(&["git", "rev-parse", &format!("{rev}^{{commit}}")]))
                .unwrap_or_default();
            if full.trim() != txn.target {
                say(&format!(
                    "a deploy of {} is under way ({}); it has to finish first",
                    short(&txn.target),
                    txn.phase
                ));
                return Err(Stop(1));
            }
            return self.finish(force, From::Hand);
        }
        must(&mut self.in_src(&["git", "checkout", "-q", "-f", "--detach", rev]))?;
        must(&mut self.in_src(&[
            "git",
            "clean",
            "-q",
            "-fd",
            "-e",
            "target",
            "-e",
            "web/feedback/node_modules",
        ]))?;
        let mut build = self.in_src(&[self.p.cargo.as_str(), "build", "--release", "--locked"]);
        for package in [
            "baylee-gateway",
            "baylee-agent",
            "baylee-engine-server",
            "baylee-catalog",
            "baylee-feedback",
            "baylee-seat",
            "baylee-seathost",
            "baylee-deploy-hooks",
            "baylee-ops",
        ] {
            build.args(["-p", package]);
        }
        run_build(&mut build)?;
        let rev = out(&mut self.in_src(&["git", "rev-parse", "HEAD"]))
            .unwrap_or_default()
            .trim()
            .to_owned();
        if self.hooks_configured() {
            return self.stage_with_hooks(&rev, force);
        }

        self.catalog_prepare(&self.p.src.join("target/release/baylee-catalog"))?;
        self.fresh_stage(&rev)?;
        // Before the agent stops: the wasm build is the long one, and games
        // can still start while it runs.
        self.play_build();
        // Seat agents (`docs/llm-seat.md` §"A hosted seat"): the bridge and
        // the agent are installed as the engine is, unlinked and written
        // anew, so a bridge playing keeps its old file; they restart in finish.
        if Path::new("/etc/systemd/system/baylee-seathost@.service").is_file() {
            must(&mut self.sudo(&[
                "install",
                "-m755",
                "target/release/baylee-seat",
                "target/release/baylee-seathost",
                &dir(&self.p.bin),
            ]))?;
            say(&format!("seat agent and bridge {} installed", short(&rev)));
        }
        self.feedback(&rev)?;
        self.finish(force, From::Hand)
    }

    /// The staged files `finish` installs: the gateway and catalog, the
    /// dispatcher and its sudoers rule (installed once the deploy went
    /// through), and the seat bridge and agent.
    fn fresh_stage(&self, rev: &str) -> Run {
        let staged = self.p.state.join("staged");
        let _ = fs::remove_dir_all(&staged);
        fs::create_dir_all(&staged).map_err(|_| Stop(1))?;
        let release = self.p.src.join("target/release");
        let mut copy = vec![
            "baylee-gateway",
            "baylee-catalog",
            "run-deploy-hooks",
            "baylee-agent",
            "baylee-engine-server",
            "baylee-deploy",
            "baylee-invite",
            "baylee-reboot",
        ];
        if release.join("baylee-seat").is_file() {
            copy.extend(["baylee-seat", "baylee-seathost"]);
        }
        for name in copy {
            fs::copy(release.join(name), staged.join(name)).map_err(|_| Stop(1))?;
        }
        fs::copy(
            self.p
                .src
                .join("scripts/server/baylee-deploy-hooks.sudoers"),
            staged.join("baylee-deploy-hooks.sudoers"),
        )
        .map_err(|_| Stop(1))?;
        fs::write(staged.join("rev"), format!("{rev}\n")).map_err(|_| Stop(1))?;
        fs::write(staged.join("since"), format!("{}\n", sh::now())).map_err(|_| Stop(1))?;
        Ok(())
    }

    /// The feedback service and its UI, straight away: it holds no game.
    fn feedback(&self, rev: &str) -> Run {
        if Self::installed("baylee-feedback") {
            must(&mut self.sudo(&[
                "install",
                "-m755",
                "target/release/baylee-feedback",
                &dir(&self.p.bin),
            ]))?;
            self.feedback_web();
            self.console_token();
            must(&mut self.sudo(&["systemctl", "restart", "baylee-feedback"]))?;
            say(&format!("feedback service {} installed", short(rev)));
        }
        Ok(())
    }

    // ------------------------------------------------------- deploy hooks

    /// Whether this server has deploy hooks: the directory holds a name the
    /// dispatcher would run (or cannot be listed, or is a link, which the
    /// dispatcher refuses). Names it ignores do not count.
    fn hooks_configured(&self) -> bool {
        let dir = &self.p.hooks_dir;
        let Ok(meta) = fs::symlink_metadata(dir) else {
            return false;
        };
        if meta.file_type().is_symlink() {
            return true;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return true;
        };
        entries
            .filter_map(Result::ok)
            .any(|e| hook_name(&e.file_name().to_string_lossy()))
    }

    fn txn_load(&self) -> Option<Txn> {
        let text = fs::read_to_string(&self.p.txn).ok()?;
        let mut txn = Txn::default();
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "target" => value.clone_into(&mut txn.target),
                    "phase" => value.clone_into(&mut txn.phase),
                    "failed" => value.clone_into(&mut txn.failed),
                    "tag" => value.clone_into(&mut txn.tag),
                    _ => {}
                }
            }
        }
        Some(txn)
    }

    /// Written beside and renamed over, so a crash leaves the old or the new.
    fn txn_save(&self, txn: &Txn) -> Run {
        let next = self.p.txn.with_extension("new");
        let text = format!(
            "target={}\nphase={}\nfailed={}\ntag={}\n",
            txn.target, txn.phase, txn.failed, txn.tag
        );
        fs::write(&next, text)
            .and_then(|()| fs::rename(&next, &self.p.txn))
            .map_err(|_| Stop(1))
    }

    /// Records where the deploy stopped, says why, and stops. Nothing goes on
    /// until `baylee-deploy finish` (or `stage`) is run by hand, or, while
    /// nothing has changed yet, a newer release replaces the target.
    fn txn_fail(&self, txn: &mut Txn, failed: &str, why: &str) -> Stop {
        failed.clone_into(&mut txn.failed);
        let _ = self.txn_save(txn);
        say(why);
        Stop(1)
    }

    /// One phase's hooks for the full commit; the dispatcher's own status
    /// (75: not ready, 0: passed). What it prints is neutral: the phase, a
    /// hook's place and how it ended.
    fn run_hooks(&self, phase: &str, rev: &str) -> i32 {
        if !sh::runnable(&self.p.hooks_bin.display().to_string()) {
            say(&format!(
                "deploy hooks are set up in {}, but {} is not installed (docs/deploy-hooks.md §\"Installing\")",
                self.p.hooks_dir.display(),
                self.p.hooks_bin.display()
            ));
            return 1;
        }
        let code = Command::new("sudo")
            .arg("-n")
            .arg(&self.p.hooks_bin)
            .args([phase, rev])
            .status()
            .ok()
            .and_then(|s| s.code())
            .unwrap_or(1);
        match code {
            0 => say(&format!("deploy hooks: {phase} for {} passed", short(rev))),
            75 => say(&format!(
                "deploy hooks: {phase} for {} is not ready; tried again on the next run",
                short(rev)
            )),
            _ => say(&format!(
                "deploy hooks: {phase} for {} failed (status {code})",
                short(rev)
            )),
        }
        code
    }

    /// The admission hold's path, from the gateway's settings.
    fn hold_path(&self) -> String {
        sh::sudo_setting(&self.p.gateway_env(), "BAYLEE_ADMISSION_HOLD").unwrap_or_default()
    }

    fn place_hold(&self, hold: &str) -> Run {
        // Made only when missing: an existing directory (the gateway's own
        // state, say) keeps its owner and mode.
        let parent = Path::new(hold)
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        if !Path::new(&parent).is_dir() {
            must(&mut self.sudo(&[
                "install", "-d", "-m755", "-o", "root", "-g", "root", &parent,
            ]))?;
        }
        must(&mut self.sudo(&[
            "install",
            "-m644",
            "-o",
            "root",
            "-g",
            "root",
            "/dev/null",
            hold,
        ]))
    }

    fn lift_hold(&self, hold: &str) -> Run {
        must(&mut self.sudo(&["rm", "-f", "--", hold]))
    }

    /// Running anywhere, waiting, admission; `None` when the gateway does
    /// not answer.
    fn health() -> Option<(u64, u64, String)> {
        gateway::fetch("health", 5).and_then(|body| gateway::health(&body))
    }

    /// Whether the gateway's `/info` names exactly this commit, built clean.
    fn info_is(rev: &str) -> bool {
        gateway::fetch("info", 2)
            .and_then(|body| gateway::info(&body))
            .is_some_and(|(commit, built)| commit == rev && built == "clean")
    }

    /// Stage under deploy hooks: read-only checks, prepare, and only then
    /// anything that changes the server.
    fn stage_with_hooks(&mut self, rev: &str, force: bool) -> Run {
        let mut txn = self.txn_load().unwrap_or_default();
        if txn.target != rev {
            txn.tag = self.staging_tag.clone().unwrap_or_default();
        }
        txn = Txn {
            target: rev.to_owned(),
            phase: "prepare".into(),
            failed: String::new(),
            tag: txn.tag,
        };
        self.txn_save(&txn)?;
        let hold = self.hold_path();
        if hold.is_empty() {
            return Err(self.txn_fail(&mut txn, "admission", &format!(
                "deploy hooks need BAYLEE_ADMISSION_HOLD in {}: without it a game could start on another agent while the hooks run (docs/deploy-hooks.md)",
                self.p.gateway_env().display()
            )));
        }
        match Self::health() {
            Some((_, _, admission)) if admission == "open" || admission == "held" => {}
            None => {
                say(&format!(
                    "waiting for {}: the gateway does not answer /health",
                    short(rev)
                ));
                return Ok(());
            }
            Some((_, _, admission)) => {
                return Err(self.txn_fail(&mut txn, "admission", &format!(
                    "the running gateway does not honour BAYLEE_ADMISSION_HOLD (/health admission: {admission}); deploy once without hooks, then add them (docs/deploy-hooks.md)"
                )));
            }
        }
        match self.run_hooks("prepare", rev) {
            0 => {}
            75 => return Ok(()),
            _ => {
                return Err(self.txn_fail(
                    &mut txn,
                    "prepare",
                    &format!(
                        "the deploy of {} stopped before it changed anything",
                        short(rev)
                    ),
                ));
            }
        }
        if self
            .catalog_prepare(&self.p.src.join("target/release/baylee-catalog"))
            .is_err()
        {
            return Err(self.txn_fail(
                &mut txn,
                "prepare",
                &format!(
                    "the deploy of {} stopped at catalog preparation",
                    short(rev)
                ),
            ));
        }
        txn.phase = "prepared".into();
        self.txn_save(&txn)?;

        self.fresh_stage(rev)?;
        self.play_build();
        self.place_hold(&hold)?;
        say(&format!(
            "admission held: no new game starts on any agent until {} is deployed",
            short(rev)
        ));
        self.feedback(rev)?;
        txn.phase = "staged".into();
        self.txn_save(&txn)?;
        self.finish(force, From::Hand)
    }

    /// Finish under deploy hooks: drain every game, before-switch, the
    /// gateway, its `/info`, after-switch, and only then games again. One
    /// function on purpose: it is the documented sequence, step by step.
    #[allow(clippy::too_many_lines)]
    fn finish_with_hooks(&mut self, force: bool, from: From) -> Run {
        let rev = read_trim(&self.p.staged("rev"));
        let since: u64 = read_trim(&self.p.staged("since")).parse().unwrap_or(0);
        let mut need_prepare = false;
        let mut txn = match self.txn_load() {
            Some(txn) if txn.target == rev => txn,
            _ => {
                // Staged before hooks were set up: it has not been prepared.
                let txn = Txn {
                    target: rev.clone(),
                    phase: "staged".into(),
                    ..Txn::default()
                };
                need_prepare = true;
                self.txn_save(&txn)?;
                txn
            }
        };
        match txn.phase.as_str() {
            "prepare" | "prepared" => {
                say(&format!(
                    "the stage of {} did not complete; staging it again",
                    short(&rev)
                ));
                return self.stage(&rev, force);
            }
            "switching" | "switched" => need_prepare = true,
            _ => {}
        }
        if !txn.failed.is_empty() {
            if from == From::Watch {
                say(&format!(
                    "the deploy of {} stopped at {}; run 'baylee-deploy finish' once that is fixed",
                    short(&rev),
                    txn.failed
                ));
                return Ok(());
            }
            need_prepare = true;
        }
        let hold = self.hold_path();
        if hold.is_empty() {
            return Err(self.txn_fail(
                &mut txn,
                "admission",
                &format!(
                    "deploy hooks need BAYLEE_ADMISSION_HOLD in {} (docs/deploy-hooks.md)",
                    self.p.gateway_env().display()
                ),
            ));
        }
        if need_prepare {
            match self.run_hooks("prepare", &rev) {
                0 => {}
                75 => return Ok(()),
                _ => {
                    return Err(self.txn_fail(
                        &mut txn,
                        "prepare",
                        &format!("the deploy of {} stopped at prepare", short(&rev)),
                    ));
                }
            }
            if self
                .catalog_prepare(&self.p.staged("baylee-catalog"))
                .is_err()
            {
                return Err(self.txn_fail(
                    &mut txn,
                    "prepare",
                    &format!(
                        "the deploy of {} stopped at catalog preparation",
                        short(&rev)
                    ),
                ));
            }
            txn.failed.clear();
            txn.phase = "staged".into();
            self.txn_save(&txn)?;
        }

        self.place_hold(&hold)?;
        if txn.phase == "staged" {
            txn.phase = "draining".into();
            self.txn_save(&txn)?;
        }
        let health = Self::health();
        let (running, waiting) = health
            .as_ref()
            .map_or((None, None), |h| (Some(h.0), Some(h.1)));
        if !force {
            match &health {
                Some((_, _, admission)) if admission == "held" => {}
                None => {
                    say(&format!(
                        "waiting for {}: the gateway does not answer /health",
                        short(&rev)
                    ));
                    return Ok(());
                }
                Some((_, _, admission)) => {
                    return Err(self.txn_fail(&mut txn, "drain", &format!(
                        "the gateway reports admission '{admission}' with the hold in place; BAYLEE_ADMISSION_HOLD in {} is not what it runs with",
                        self.p.gateway_env().display()
                    )));
                }
            }
            // Every game, on any agent: a hook may stop what a remote
            // engine's game or a local seat's bridge needs.
            if running != Some(0) {
                say(&format!(
                    "waiting for {}: {} game(s) running, {} waiting",
                    short(&rev),
                    shown(running),
                    shown(waiting)
                ));
                return Ok(());
            }
            if waiting != Some(0)
                && sh::now().saturating_sub(since) < self.p.room_grace_minutes * 60
            {
                say(&format!(
                    "waiting for {}: {} room(s) waiting",
                    short(&rev),
                    shown(waiting)
                ));
                return Ok(());
            }
        }

        self.install_agent(&rev)?;
        match self.run_hooks("before-switch", &rev) {
            0 => {}
            75 => return Ok(()),
            _ => {
                return Err(self.txn_fail(
                    &mut txn,
                    "before-switch",
                    &format!(
                        "the deploy of {} stopped at before-switch: the gateway was not replaced",
                        short(&rev)
                    ),
                ));
            }
        }
        txn.phase = "switching".into();
        self.txn_save(&txn)?;
        // The bridge a seat agent starts has to speak the new gateway's
        // protocol; no game starts (the hold) until the seat agents restart.
        if Path::new("/etc/systemd/system/baylee-seathost@.service").is_file()
            && self.p.staged("baylee-seat").is_file()
        {
            self.install_staged(&["baylee-seat", "baylee-seathost"])?;
            say(&format!("seat agent and bridge {} installed", short(&rev)));
        }
        self.install_staged(&["baylee-gateway", "baylee-catalog"])?;
        self.play_install()?;
        must(&mut self.sudo(&["systemctl", "restart", "baylee-gateway"]))?;
        let up = (0..self.p.info_tries).any(|_| {
            if Self::info_is(&rev) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            false
        });
        if !up {
            return Err(self.txn_fail(&mut txn, "switch", &format!(
                "the new gateway's /info does not name {} built clean; the agent and the seat agents stay off",
                short(&rev)
            )));
        }
        txn.phase = "switched".into();
        self.txn_save(&txn)?;

        match self.run_hooks("after-switch", &rev) {
            0 => {}
            75 => return Ok(()),
            _ => {
                return Err(self.txn_fail(&mut txn, "after-switch", &format!(
                    "the deploy of {} stopped at after-switch: no game starts, the agent and the seat agents stay off",
                    short(&rev)
                )));
            }
        }
        self.lift_hold(&hold)?;
        must(&mut self.sudo(&["systemctl", "start", "baylee-agent"]))?;
        let _ = self
            .sudo(&["systemctl", "try-restart", "baylee-seathost@*"])
            .stderr(std::process::Stdio::null())
            .status();
        self.install_dispatcher()?;
        self.deployed(&rev)?;
        let _ = fs::remove_file(&self.p.txn);
        say(&format!(
            "deployed {} with deploy hooks (games running: {}, waiting: {}{})",
            short(&rev),
            shown(running),
            shown(waiting),
            if force { ", forced" } else { "" }
        ));
        print_info();
        Ok(())
    }

    fn install_staged(&self, names: &[&str]) -> Run {
        let mut install = self.sudo(&["install", "-m755"]);
        for name in names {
            install.arg(self.p.staged(name));
        }
        install.arg(dir(&self.p.bin));
        must(&mut install)
    }

    /// Records `rev` as deployed and drops the stage.
    fn deployed(&self, rev: &str) -> Run {
        fs::write(self.p.state.join("deployed"), format!("{rev}\n")).map_err(|_| Stop(1))?;
        let _ = fs::remove_dir_all(self.p.state.join("staged"));
        Ok(())
    }

    /// The dispatcher and its sudoers rule, from a stage that went through,
    /// so the next deploy runs the hooks with this release's dispatcher. The
    /// rule is checked by visudo first and left out where it does not
    /// validate (a sudo before 1.9.10 knows no argument patterns): the
    /// dispatcher checks its own arguments either way.
    fn install_dispatcher(&self) -> Run {
        if !self.p.staged("run-deploy-hooks").is_file() {
            return Ok(());
        }
        let parent = |p: &Path| {
            p.parent()
                .map(|d| d.display().to_string())
                .unwrap_or_default()
        };
        must(&mut self.sudo(&[
            "install",
            "-d",
            "-m755",
            "-o",
            "root",
            "-g",
            "root",
            &parent(&self.p.hooks_bin),
        ]))?;
        let mut install = self.sudo(&["install", "-m755", "-o", "root", "-g", "root"]);
        must(
            install
                .arg(self.p.staged("run-deploy-hooks"))
                .arg(&self.p.hooks_bin),
        )?;
        let rule = self.p.staged("baylee-deploy-hooks.sudoers");
        let Ok(template) = fs::read_to_string(&rule) else {
            return Ok(());
        };
        let user = out_quiet(&mut cmd(&["id", "-un"]))
            .unwrap_or_default()
            .trim()
            .to_owned();
        if !sudoers_user(&user) {
            say(
                "the deploy user's name is not one a sudoers rule takes; the deploy hooks' rule is left out",
            );
            return Ok(());
        }
        let filled =
            std::env::temp_dir().join(format!("baylee-deploy-sudoers-{}", std::process::id()));
        fs::write(&filled, template.replace("@DEPLOY_USER@", &user)).map_err(|_| Stop(1))?;
        let valid = quiet(Command::new("sudo").args(["visudo", "-cqf"]).arg(&filled));
        let result = if valid {
            must(&mut self.sudo(&[
                "install",
                "-d",
                "-m750",
                "-o",
                "root",
                "-g",
                "root",
                &parent(&self.p.hooks_sudoers),
            ]))
            .and_then(|()| {
                must(
                    self.sudo(&["install", "-m440", "-o", "root", "-g", "root"])
                        .arg(&filled)
                        .arg(&self.p.hooks_sudoers),
                )
            })
        } else {
            say(
                "the deploy hooks' sudoers rule does not validate here; left out (docs/deploy-hooks.md)",
            );
            Ok(())
        };
        let _ = fs::remove_file(&filled);
        result
    }

    /// A transaction left from when this server had hooks: its hold comes
    /// down with the deploy that went through without them.
    fn forget_transaction(&self) -> Run {
        if !self.p.txn.is_file() {
            return Ok(());
        }
        let hold = self.hold_path();
        if !hold.is_empty() {
            self.lift_hold(&hold)?;
        }
        let _ = fs::remove_file(&self.p.txn);
        Ok(())
    }

    /// Whether a release tag is signed by a key the operator allows: an SSH
    /// allowed-signers file (`$etc/release-signers`) or an `OpenPGP` keyring
    /// (`$etc/release-keyring.gpg`), each root's alone, in a directory that
    /// is. Neither: a warning, and the tag is taken as before. `git` runs with
    /// an empty `GnuPG` home (or one holding only that keyring), only that
    /// signers file and none of the deploy user's git settings, so no key or
    /// signing program of the deploy user's own counts.
    fn verify_tag(&self, tag: &str) -> bool {
        let signers = self.p.etc.join("release-signers");
        let keyring = self.p.etc.join("release-keyring.gpg");
        let exists = |p: &Path| fs::symlink_metadata(p).is_ok();
        if !exists(&signers) && !exists(&keyring) {
            say(&format!(
                "release tag {tag} is not verified: no {} or {} (docs/deploy-hooks.md §\"Signed release tags\")",
                signers.display(),
                keyring.display()
            ));
            return true;
        }
        for file in [&self.p.etc, &signers, &keyring] {
            if exists(file) && !root_alone(file, self.p.trusted_uid) {
                say(&format!(
                    "refusing release tag {tag}: {} is a link, or not root's alone",
                    file.display()
                ));
                return false;
            }
        }
        let Ok(home) = tempfile_dir() else {
            return false;
        };
        let mut verified = true;
        if exists(&keyring)
            && !quiet(
                Command::new("gpg")
                    .arg("--homedir")
                    .arg(&home)
                    .args(["--batch", "--quiet", "--import"])
                    .arg(&keyring),
            )
        {
            verified = false;
        }
        if verified {
            let signers = if exists(&signers) {
                signers.clone()
            } else {
                PathBuf::from("/dev/null")
            };
            // The deploy user's own git settings (a signing program, a
            // format) play no part: only these do.
            verified = quiet(
                Command::new("git")
                    .env("GNUPGHOME", &home)
                    .env("GIT_CONFIG_GLOBAL", "/dev/null")
                    .env("GIT_CONFIG_NOSYSTEM", "1")
                    .arg("-C")
                    .arg(&self.p.src)
                    .args([
                        "-c",
                        "gpg.program=gpg",
                        "-c",
                        "gpg.openpgp.program=gpg",
                        "-c",
                        "gpg.ssh.program=ssh-keygen",
                    ])
                    .arg("-c")
                    .arg(format!("gpg.ssh.allowedSignersFile={}", signers.display()))
                    .args(["-c", "gpg.minTrustLevel=undefined", "verify-tag", tag]),
            );
        }
        let _ = quiet(
            Command::new("gpgconf")
                .arg("--homedir")
                .arg(&home)
                .args(["--kill", "all"]),
        );
        let _ = fs::remove_dir_all(&home);
        if !verified {
            say(&format!(
                "refusing release tag {tag}: its signature does not verify against {}/release-signers or {}/release-keyring.gpg",
                self.p.etc.display(),
                self.p.etc.display()
            ));
            return false;
        }
        say(&format!("release tag {tag}: signature verified"));
        true
    }

    /// The admin console's shared token (`docs/feedback.md` §"The admin
    /// console"): the gateway's `BAYLEE_ADMIN_TOKEN` and the feedback
    /// service's `FEEDBACK_GATEWAY_ADMIN_TOKEN` are one secret. Made once
    /// where neither file has it, copied across where one has it, never
    /// replaced, and never printed: it reaches the files on `sudo tee -a`'s
    /// stdin, so no command line shows it. Both files end 0600.
    ///
    /// Never fatal: it runs after the agent has stopped, and a stage that
    /// died here would leave the server starting no game. Whatever fails is
    /// logged and the deploy goes on without the console.
    fn console_token(&self) {
        if self.console_token_now().is_none() {
            say(&format!(
                "the admin console's token was not set up; set BAYLEE_ADMIN_TOKEN in {} and FEEDBACK_GATEWAY_ADMIN_URL/_TOKEN in {}/feedback.env by hand (docs/feedback.md)",
                self.p.gateway_env().display(),
                self.p.etc.display()
            ));
        }
    }

    fn console_token_now(&self) -> Option<()> {
        let (gw, fb) = (self.p.gateway_env(), self.p.etc.join("feedback.env"));
        let file_ok = |f: &Path| quiet(Command::new("sudo").args(["test", "-f"]).arg(f));
        if !file_ok(&gw) || !file_ok(&fb) {
            say(&format!(
                "no {} or {}: the admin console's token is not set up",
                gw.display(),
                fb.display()
            ));
            return Some(());
        }
        let read = |f: &Path, key: &str| -> Option<String> {
            let text = out_quiet(
                Command::new("sudo")
                    .args(["sed", "-n", &format!("s/^{key}=//p")])
                    .arg(f),
            )?;
            Some(text.lines().last().unwrap_or_default().to_owned())
        };
        let ours = read(&gw, "BAYLEE_ADMIN_TOKEN")?;
        let theirs = read(&fb, "FEEDBACK_GATEWAY_ADMIN_TOKEN")?;
        if !ours.is_empty() && !theirs.is_empty() && ours != theirs {
            say(&format!(
                "BAYLEE_ADMIN_TOKEN in {} and FEEDBACK_GATEWAY_ADMIN_TOKEN in {} differ; left as they are",
                gw.display(),
                fb.display()
            ));
        }
        let mut token = if ours.is_empty() { theirs } else { ours };
        if token.is_empty() {
            token = random_hex(32)?;
            say(&format!(
                "admin console token made for {} and {}",
                gw.display(),
                fb.display()
            ));
        }
        let has = |f: &Path, key: &str| {
            quiet(
                Command::new("sudo")
                    .args(["grep", "-q", &format!("^{key}=.")])
                    .arg(f),
            )
        };
        let append = |f: &Path, line: &str| {
            sh::feed(
                Command::new("sudo").args(["tee", "-a"]).arg(f),
                line.as_bytes(),
            )
        };
        if !has(&gw, "BAYLEE_ADMIN_TOKEN") && !append(&gw, &format!("BAYLEE_ADMIN_TOKEN={token}\n"))
        {
            return None;
        }
        if !has(&fb, "FEEDBACK_GATEWAY_ADMIN_TOKEN")
            && !append(&fb, &format!("FEEDBACK_GATEWAY_ADMIN_TOKEN={token}\n"))
        {
            return None;
        }
        if !has(&fb, "FEEDBACK_GATEWAY_ADMIN_URL")
            && !append(&fb, "FEEDBACK_GATEWAY_ADMIN_URL=http://127.0.0.1:28767\n")
        {
            return None;
        }
        token.clear();
        ok(Command::new("sudo")
            .args(["chmod", "600"])
            .arg(&gw)
            .arg(&fb))
        .then_some(())
    }

    /// The browser client, built into the stage (#327). Skipped, with a line
    /// in the log, where trunk or the wasm target is missing; a failed build
    /// is logged and leaves nothing staged, so finish keeps the installed
    /// client. `--public-url /play/` because Caddy serves it there.
    fn play_build(&self) {
        if !sh::runnable(&self.p.trunk) {
            say("no trunk here: the browser client is not built, the one installed stays");
            return;
        }
        let targets = out_quiet(&mut cmd(&[
            self.p.rustup.as_str(),
            "target",
            "list",
            "--installed",
        ]))
        .unwrap_or_default();
        if !targets.lines().any(|t| t == "wasm32-unknown-unknown") {
            say(
                "no wasm32-unknown-unknown target here: the browser client is not built, the one installed stays",
            );
            return;
        }
        let dist = self.p.staged("play");
        // trunk runs cargo from PATH, which a systemd unit's PATH may not have.
        let cargo_dir = Path::new(&self.p.cargo)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let path = std::env::join_paths(std::iter::once(cargo_dir).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )))
        .unwrap_or_default();
        let mut build = self.in_src(&[
            self.p.trunk.as_str(),
            "build",
            "index.html",
            "--release",
            "--locked",
            "--public-url",
            "/play/",
            "--dist",
        ]);
        build
            .arg(&dist)
            .current_dir(self.p.src.join("crates/baylee-client"))
            .env("PWD", self.p.src.join("crates/baylee-client"))
            .env("PATH", path);
        if run_build(&mut build).is_err() || !dist.join("index.html").is_file() {
            let _ = fs::remove_dir_all(&dist);
            say("the browser client did not build; the one installed stays");
            return;
        }
        let version = out_quiet(&mut cmd(&[self.p.trunk.as_str(), "--version"]))
            .map_or_else(|| "trunk".into(), |v| v.trim().to_owned());
        say(&format!("browser client built ({version})"));
    }

    /// Installs the staged browser client next to the gateway, as the
    /// feedback UI is installed: a copy beside the old one, then one rename.
    fn play_install(&self) -> Run {
        if !self.p.staged("play/index.html").is_file() {
            say("no browser client staged; the one installed stays");
            return Ok(());
        }
        self.swap_in(&self.p.staged("play"), &self.p.play)?;
        say(&format!(
            "browser client installed to {}",
            self.p.play.display()
        ));
        Ok(())
    }

    fn swap_in(&self, from: &Path, to: &Path) -> Run {
        let next = format!("{}.new", to.display());
        let parent = to
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        must(&mut self.sudo(&["mkdir", "-p", &parent]))?;
        must(&mut self.sudo(&["rm", "-rf", &next]))?;
        must(self.sudo(&["cp", "-R"]).arg(from).arg(&next))?;
        must(self.sudo(&["rm", "-rf"]).arg(to))?;
        must(self.sudo(&["mv", &next]).arg(to))
    }

    /// The feedback service's web UI (`web/feedback`, #311): built here when
    /// npm (and so node) is installed, and put where the unit's
    /// `FEEDBACK_WEB_DIR` points, before the service restarts. Without npm,
    /// or when the build fails, the UI installed last stays.
    fn feedback_web(&self) {
        if !sh::runnable(&self.p.npm) {
            say("no npm here: the feedback web UI is not built, the one installed stays");
            return;
        }
        let ui = self.p.src.join("web/feedback");
        let built =
            quiet(cmd(&[self.p.npm.as_str(), "ci", "--no-audit", "--no-fund"]).current_dir(&ui))
                && quiet(cmd(&[self.p.npm.as_str(), "run", "build"]).current_dir(&ui));
        if !built || !ui.join("dist/index.html").is_file() {
            say("the feedback web UI did not build; the one installed stays");
            return;
        }
        if self
            .swap_in(Path::new("web/feedback/dist"), &self.p.web)
            .is_ok()
        {
            say(&format!(
                "feedback web UI installed to {}",
                self.p.web.display()
            ));
        }
    }

    fn finish(&mut self, force: bool, from: From) -> Run {
        if !self.p.staged("rev").is_file() {
            say("nothing staged");
            return Ok(());
        }
        if self.hooks_configured() {
            return self.finish_with_hooks(force, from);
        }
        let rev = read_trim(&self.p.staged("rev"));
        let hold = self.hold_path();
        if !hold.is_empty() {
            self.place_hold(&hold)?;
        }
        if !force && !Self::admission_held(&hold) {
            say(
                "waiting: configure BAYLEE_ADMISSION_HOLD and verify /health admission=held; the owner agent stays running",
            );
            return Ok(());
        }
        let games = self.open_games();
        let since: u64 = read_trim(&self.p.staged("since")).parse().unwrap_or(0);
        let (running, waiting) = (games.map(|g| g.0), games.map(|g| g.1));
        if !force {
            if running != Some(0) {
                say(&format!(
                    "waiting for {}: {} game(s) running here, {} waiting",
                    short(&rev),
                    shown(running),
                    shown(waiting)
                ));
                return Ok(());
            }
            if waiting != Some(0)
                && sh::now().saturating_sub(since) < self.p.room_grace_minutes * 60
            {
                say(&format!(
                    "waiting for {}: {} room(s) waiting",
                    short(&rev),
                    shown(waiting)
                ));
                return Ok(());
            }
        }
        self.install_agent(&rev)?;
        self.install_staged(&["baylee-gateway", "baylee-catalog"])?;
        self.play_install()?;
        must(&mut self.sudo(&["systemctl", "restart", "baylee-gateway"]))?;
        for _ in 0..20 {
            if gateway::fetch("info", 2).is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        must(&mut self.sudo(&["systemctl", "start", "baylee-agent"]))?;
        // Under the same wait as the gateway (no local game running): a seat
        // agent's restart lets go of every bridge it runs.
        let _ = self
            .sudo(&["systemctl", "try-restart", "baylee-seathost@*"])
            .stderr(std::process::Stdio::null())
            .status();
        self.install_dispatcher()?;
        self.forget_transaction()?;
        if !hold.is_empty() {
            self.lift_hold(&hold)?;
        }
        self.deployed(&rev)?;
        say(&format!(
            "deployed {} (games running: {}, waiting: {}{})",
            short(&rev),
            shown(running),
            shown(waiting),
            if force { ", forced" } else { "" }
        ));
        print_info();
        Ok(())
    }

    /// The newest release tag.
    fn newest_tag() -> Option<String> {
        let refs =
            out_quiet(Command::new("git").args(["ls-remote", "--tags", "--refs", REPO, "v*"]))?;
        let mut tags: Vec<String> = refs
            .lines()
            .filter_map(|l| {
                l.split_whitespace()
                    .nth(1)?
                    .strip_prefix("refs/tags/")
                    .map(str::to_owned)
            })
            .collect();
        tags.sort_by(|a, b| version_cmp(a, b));
        tags.pop()
    }

    fn last_release(&self) -> String {
        read_trim(&self.p.state.join("last-release"))
    }

    /// A new release tag, verified and remembered; `None` when there is none
    /// to stage (no tag, the one seen before, a refused one, or one deployed).
    fn new_release(&self) -> Result<Option<(String, String)>, Stop> {
        let Some(tag) = Self::newest_tag().filter(|t| *t != self.last_release()) else {
            return Ok(None);
        };
        let _ = quiet(
            Command::new("git")
                .current_dir(&self.p.src)
                .args(["fetch", "-q", "--tags", REPO]),
        );
        // A refused tag is not tried again every minute: `stage <tag>` by
        // hand, once the signers are right.
        let remember = || {
            fs::write(self.p.state.join("last-release"), format!("{tag}\n")).map_err(|_| Stop(1))
        };
        if !self.verify_tag(&tag) {
            remember()?;
            return Ok(None);
        }
        let rev = out(Command::new("git")
            .arg("-C")
            .arg(&self.p.src)
            .args(["rev-parse", &format!("{tag}^{{commit}}")]))
        .unwrap_or_default()
        .trim()
        .to_owned();
        remember()?;
        if rev.is_empty() || rev == read_trim(&self.p.state.join("deployed")) {
            return Ok(None);
        }
        say(&format!("release {tag} found"));
        Ok(Some((tag, rev)))
    }

    fn watch(&mut self) -> Run {
        if self.hooks_configured() {
            return self.watch_with_hooks();
        }
        if let Some((_, rev)) = self.new_release()? {
            return self.stage(&rev, false);
        }
        self.finish(false, From::Hand)
    }

    /// `watch` under deploy hooks. A transaction that has changed something
    /// goes on and nothing else starts; a newer release replaces only a
    /// target that has not got past prepare; one that was not ready is tried
    /// again.
    fn watch_with_hooks(&mut self) -> Run {
        if let Some(txn) = self.txn_load() {
            match txn.phase.as_str() {
                "staged" | "draining" | "switching" | "switched" => {
                    return self.finish(false, From::Watch);
                }
                "prepared" => {
                    say(&format!(
                        "the stage of {} did not complete; staging it again",
                        short(&txn.target)
                    ));
                    return self.stage(&txn.target, false);
                }
                _ => {}
            }
        }
        if let Some((tag, rev)) = self.new_release()? {
            self.txn_save(&Txn {
                target: rev.clone(),
                phase: "prepare".into(),
                failed: String::new(),
                tag: tag.clone(),
            })?;
            self.staging_tag = Some(tag);
            return self.stage(&rev, false);
        }
        if let Some(txn) = self.txn_load().filter(|t| t.phase == "prepare") {
            if !txn.failed.is_empty() {
                let name = if txn.tag.is_empty() {
                    &txn.target
                } else {
                    &txn.tag
                };
                say(&format!(
                    "the deploy of {} stopped at {} before it changed anything; 'baylee-deploy stage {name}' tries again",
                    short(&txn.target),
                    txn.failed
                ));
                return Ok(());
            }
            return self.stage(&txn.target, false);
        }
        self.finish(false, From::Watch)
    }

    fn status(&self) {
        let or_dash = |p: PathBuf| {
            Some(read_trim(&p))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "-".into())
        };
        println!("deployed: {}", or_dash(self.p.state.join("deployed")));
        println!("staged:   {}", or_dash(self.p.staged("rev")));
        println!("release:  {}", or_dash(self.p.state.join("last-release")));
        if self.hooks_configured() {
            match self.txn_load() {
                Some(txn) => println!(
                    "hooks:    {} at {}{}",
                    short(&txn.target),
                    txn.phase,
                    if txn.failed.is_empty() {
                        String::new()
                    } else {
                        format!(", stopped at {}", txn.failed)
                    }
                ),
                None => println!("hooks:    set up, nothing under way"),
            }
        }
        let games = self.open_games();
        println!(
            "games:    {} {} (running here, waiting)",
            shown(games.map(|g| g.0)),
            shown(games.map(|g| g.1))
        );
        let mut units = vec!["baylee-gateway", "baylee-agent"];
        if Self::installed("baylee-feedback") {
            units.push("baylee-feedback");
        }
        println!("services: {}", units.join(" "));
        let active = Command::new("systemctl")
            .arg("is-active")
            .args(&units)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        println!(
            "          {}",
            active.split_whitespace().collect::<Vec<_>>().join(" ")
        );
    }
}

/// Prints the new gateway's `/info` after a deploy, as a last word.
fn print_info() {
    if let Some(info) = gateway::fetch("info", 5) {
        println!("{}", info.trim_end());
    }
}

/// Runs a build and fails as the build fails. A build that worked shows its
/// last line; one that failed shows why ([`build_report`]).
fn run_build(command: &mut Command) -> Run {
    let output = command
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|_| Stop(1))?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    for line in build_report(&text, output.status.success()) {
        println!("{line}");
    }
    if output.status.success() {
        Ok(())
    } else {
        Err(Stop(output.status.code().unwrap_or(1)))
    }
}

/// Lines of a failed build's first error block shown, at most.
const ERROR_LINES: usize = 40;

/// What a build's output shows: its last line when it worked. When it
/// failed, the last line alone is cargo's "build failed, waiting for other
/// jobs" (22.10.2026, three deploys), so it is the first `error` block (up to
/// [`ERROR_LINES`]) and the last line, and for the causes a server meets
/// after a release (`docs/releasing.md` §"What the server builds with") one
/// sentence on what to do.
fn build_report(text: &str, success: bool) -> Vec<String> {
    let last = text.lines().rfind(|l| !l.trim().is_empty());
    if success {
        return last.map(str::to_owned).into_iter().collect();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut shown: Vec<String> = match lines.iter().position(|l| l.starts_with("error")) {
        Some(start) => lines[start..]
            .iter()
            .take(ERROR_LINES)
            .map(|l| (*l).to_owned())
            .collect(),
        None => lines[lines.len().saturating_sub(ERROR_LINES)..]
            .iter()
            .map(|l| (*l).to_owned())
            .collect(),
    };
    if let Some(last) = last
        && shown.last().map(String::as_str) != Some(last)
    {
        shown.push(last.to_owned());
    }
    if text.contains("is not supported by the following packages") {
        shown.push(format!(
            "{TAG}: this machine's Rust is older than the workspace's rust-version: `rustup update stable`"
        ));
    }
    if text.contains("The system library") {
        shown.push(format!(
            "{TAG}: a system library for the build is missing (docs/releasing.md §\"What the server builds with\")"
        ));
    }
    shown
}

/// A directory argument with its trailing slash (`install … "$bin/"`).
fn dir(path: &Path) -> String {
    format!("{}/", path.display())
}

fn read_trim(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

fn short(rev: &str) -> &str {
    rev.get(..10).unwrap_or(rev)
}

fn shown(n: Option<u64>) -> String {
    n.map_or_else(|| "?".into(), |n| n.to_string())
}

/// A name the dispatcher would run: `[A-Za-z0-9][A-Za-z0-9_-]*`.
fn hook_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// A name a sudoers rule takes: `[a-z_][a-z0-9_-]*`.
fn sudoers_user(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Owned by the trusted user, not a link, not group- or other-writable.
#[cfg(unix)]
fn root_alone(path: &Path, uid: u32) -> bool {
    use std::os::unix::fs::MetadataExt;
    fs::symlink_metadata(path)
        .is_ok_and(|m| !m.file_type().is_symlink() && m.uid() == uid && m.mode() & 0o022 == 0)
}

#[cfg(not(unix))]
fn root_alone(_path: &Path, _uid: u32) -> bool {
    false
}

fn tempfile_dir() -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "baylee-deploy-gnupg-{}-{}",
        std::process::id(),
        random_hex(6).unwrap_or_default()
    ));
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(&dir)?;
    Ok(dir)
}

fn random_hex(bytes: usize) -> Option<String> {
    let mut buf = vec![0_u8; bytes];
    getrandom::fill(&mut buf).ok()?;
    Some(
        buf.iter()
            .fold(String::with_capacity(bytes * 2), |mut s, b| {
                let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{b:02x}"));
                s
            }),
    )
}

/// `sort -V` order: digit runs compare as numbers, everything else as text.
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn parts(s: &str) -> Vec<Result<u64, &str>> {
        let mut out = Vec::new();
        let mut rest = s;
        while !rest.is_empty() {
            let digits = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            if digits > 0 {
                out.push(Ok(rest[..digits].parse().unwrap_or(u64::MAX)));
                rest = &rest[digits..];
            } else {
                let text = rest
                    .find(|c: char| c.is_ascii_digit())
                    .unwrap_or(rest.len());
                out.push(Err(&rest[..text]));
                rest = &rest[text..];
            }
        }
        out
    }
    let (pa, pb) = (parts(a), parts(b));
    for (x, y) in pa.iter().zip(&pb) {
        let ordering = match (x, y) {
            (Ok(x), Ok(y)) => x.cmp(y),
            (Err(x), Err(y)) => x.cmp(y),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        };
        if ordering.is_ne() {
            return ordering;
        }
    }
    pa.len().cmp(&pb.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_tags_sort_as_versions() {
        let mut tags = [
            "v0.1.0-beta.10",
            "v0.1.0-beta.7b",
            "v0.1.0-beta.9",
            "v0.1.0-beta.7",
        ];
        tags.sort_by(|a, b| version_cmp(a, b));
        assert_eq!(tags.last(), Some(&"v0.1.0-beta.10"));
        assert_eq!(
            version_cmp("v0.1.0-beta.7", "v0.1.0-beta.7b"),
            std::cmp::Ordering::Less
        );
    }

    /// Every package CI installs for a Linux build is either the client's
    /// alone or one the server builds with, and then `docs/releasing.md`
    /// §"What the server builds with" names it: a new native dependency of a
    /// server crate reaches the server's list, not only CI's.
    #[test]
    fn the_server_s_build_packages_are_written_down() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ci = fs::read_to_string(root.join(".github/actions/linux-deps/action.yml")).unwrap();
        let doc = fs::read_to_string(root.join("docs/releasing.md")).unwrap();
        let section = doc
            .split("### What the server builds with")
            .nth(1)
            .and_then(|rest| rest.split("\n## ").next())
            .expect("the section");
        let packages = ci
            .lines()
            .find_map(|l| l.trim().strip_prefix("pkgs=("))
            .and_then(|l| l.strip_suffix(')'))
            .expect("linux-deps' package list");
        let client_only = ["libasound2-dev", "libudev-dev", "libwayland-dev"];
        let mut seen = 0;
        for package in packages.split_whitespace() {
            seen += 1;
            assert!(
                client_only.contains(&package) || section.contains(package),
                "{package} is installed for CI's Linux builds but neither the client's alone nor in docs/releasing.md §\"What the server builds with\""
            );
        }
        assert!(seen >= 4, "read {seen} packages from linux-deps");
    }

    /// What 22.10.2026's three failed deploys printed, abridged: cargo's
    /// last line said nothing, the cause sat in its first error.
    const MISSING_GLIB: &str = "   Compiling glib-sys v0.22.9
error: failed to run custom build command for `glib-sys v0.22.9`

Caused by:
  pkg-config exited with status code 1
  The system library `glib-2.0` required by crate `glib-sys` was not found.
  HINT: you may need to install a package such as glib-2.0, glib-2.0-dev or glib-2.0-devel.
warning: build failed, waiting for other jobs to finish...
";
    const OLD_RUST: &str = "error: rustc 1.98.1 is not supported by the following packages:
  baylee-ops@0.1.0-beta.8a requires rustc 1.99

";

    #[test]
    fn a_build_that_worked_shows_its_last_line() {
        let shown = build_report("   Compiling a\n    Finished `release` profile\n\n", true);
        assert_eq!(shown, ["    Finished `release` profile"]);
    }

    #[test]
    fn a_failed_build_shows_its_first_error_and_what_to_do() {
        let shown = build_report(MISSING_GLIB, false);
        assert_eq!(
            shown[0],
            "error: failed to run custom build command for `glib-sys v0.22.9`"
        );
        assert!(
            shown
                .iter()
                .any(|l| l.contains("`glib-2.0` required by crate `glib-sys`"))
        );
        assert_eq!(
            shown
                .iter()
                .filter(|l| l.starts_with("warning: build failed"))
                .count(),
            1,
            "the last line once: {shown:?}"
        );
        assert!(
            shown
                .last()
                .unwrap()
                .contains("a system library for the build is missing")
        );

        let shown = build_report(OLD_RUST, false);
        assert!(shown[0].contains("rustc 1.98.1 is not supported"));
        assert!(shown.last().unwrap().contains("`rustup update stable`"));
    }

    #[test]
    fn a_failed_build_shows_a_bounded_tail_when_nothing_says_error() {
        let text = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let shown = build_report(&text, false);
        assert_eq!(shown.len(), ERROR_LINES);
        assert_eq!(shown.first().unwrap(), "line 60");
        assert_eq!(shown.last().unwrap(), "line 99");

        let text = std::iter::once("error: x".to_owned())
            .chain((0..100).map(|i| format!("  line {i}")))
            .collect::<Vec<_>>()
            .join("\n");
        let shown = build_report(&text, false);
        assert_eq!(
            shown.len(),
            ERROR_LINES + 1,
            "the block, bounded, and the last line"
        );
        assert_eq!(shown.last().unwrap(), "  line 99");
    }

    #[test]
    fn hook_and_user_names_follow_the_dispatcher_and_sudoers() {
        for good in ["10-backup", "a", "x_y"] {
            assert!(hook_name(good), "{good}");
        }
        for bad in ["", ".hidden", "-x", "a.b", "a b"] {
            assert!(!hook_name(bad), "{bad}");
        }
        assert!(sudoers_user("viktor") && sudoers_user("_svc-1"));
        assert!(!sudoers_user("Viktor") && !sudoers_user("1x") && !sudoers_user(""));
    }
}
