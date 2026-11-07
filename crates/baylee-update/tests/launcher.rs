//! Exercise the actual packaged entry executable, with a small test runtime.
//! Helpers run as separate OS processes; no in-process mutex can pass these.
mod support;

use baylee_update::apply::{self, Install, Staged};
use baylee_update::launch::{self, Activation};
use baylee_update::plan::{NEW, Os};
use baylee_update::relaunch;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn os() -> Os {
    if cfg!(target_os = "macos") {
        Os::MacOs
    } else if cfg!(target_os = "windows") {
        Os::Windows
    } else {
        Os::Linux
    }
}

fn runtime(base: &Path) -> PathBuf {
    if os() == Os::MacOs {
        base.join("Baylee.app/Contents/MacOS/baylee-runtime")
    } else {
        base.join(launch::runtime_name(os()))
    }
}

/// Where a release tree keeps its launcher.
fn launcher(base: &Path) -> PathBuf {
    if os() == Os::MacOs {
        runtime(base).with_file_name(relaunch::MAC_LAUNCHER)
    } else {
        base.join(os().program())
    }
}

struct Fixture {
    root: PathBuf,
    install: Install,
    entry: PathBuf,
    result: PathBuf,
}

impl Fixture {
    /// # Panics
    /// Inside a fixture. A runtime here is this test program, so a launcher that loses
    /// the arguments starts the whole suite, whose fixtures would start runtimes of
    /// their own without end: that is a fork bomb, not a red test.
    fn new(tag: &str) -> Self {
        assert!(
            std::env::var_os("BAYLEE_TEST_RESULT").is_none(),
            "a fixture inside a fixture: a launcher lost the test's arguments"
        );
        let root = support::scratch(tag);
        let base = root.join("package");
        let entry = support::exe(os(), &base);
        fs::create_dir_all(entry.parent().unwrap()).unwrap();
        fs::copy(env!("CARGO_BIN_EXE_baylee-launch"), &entry).unwrap();
        fs::copy(std::env::current_exe().unwrap(), runtime(&base)).unwrap();
        fs::write(runtime(&base).with_file_name("version.txt"), "original").unwrap();
        let install = Install::around(&entry, os()).unwrap();
        let install = launch::in_state_root(&install, &root.join("state")).unwrap();
        let result = root.join("result.json");
        Self {
            root,
            install,
            entry,
            result,
        }
    }

    fn stage(&self, version: &str) {
        let new = self.install.stage().join(NEW);
        let exe = runtime(&new);
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        fs::write(exe.with_file_name("version.txt"), version).unwrap();
        apply::mark_staged(
            &self.install,
            &Staged {
                version: version.into(),
                tag: format!("v{version}"),
                page: String::new(),
                asset: String::new(),
                size: 0,
            },
        )
        .unwrap();
    }

    /// A release as an archive carries it: with its own launcher, this same program.
    fn stage_whole(&self, version: &str) {
        self.stage(version);
        let new = self.install.stage().join(NEW);
        fs::copy(env!("CARGO_BIN_EXE_baylee-launch"), launcher(&new)).unwrap();
    }

    /// A release whose launcher starts and dies: this test program, which finds no
    /// session and panics.
    fn stage_with_a_dying_launcher(&self, version: &str) {
        self.stage(version);
        let new = self.install.stage().join(NEW);
        fs::copy(std::env::current_exe().unwrap(), launcher(&new)).unwrap();
    }

    /// The attempts the first stage has written down.
    fn attempts(&self) -> Option<u64> {
        let path = self.install.stage().join("launch-attempts.json");
        let bytes = fs::read(path).ok()?;
        let attempts: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        attempts["count"].as_u64()
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.entry);
        cmd.args(["--ignored", "--exact", "runtime_helper", "--nocapture"])
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("LOCALAPPDATA", self.root.join("state"))
            .env("BAYLEE_TEST_RESULT", &self.result)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        cmd
    }

    fn result(&self) -> serde_json::Value {
        serde_json::from_slice(&fs::read(&self.result).unwrap()).unwrap()
    }

    /// Starts a client once the last one has left, which is how an orphan is waited
    /// for: nobody here is its parent.
    fn starts_when_free(&self) {
        let until = Instant::now() + Duration::from_secs(20);
        while !self.command().status().unwrap().success() {
            assert!(Instant::now() < until, "the orphan never left");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn interrupted(&self, phase: &str) {
        let ready = self.root.join("prepared");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "activation_helper", "--nocapture"])
            .env(
                "BAYLEE_TEST_INSTALL",
                serde_json::to_string(&self.install).unwrap(),
            )
            .env("BAYLEE_TEST_PHASE", phase)
            .env("BAYLEE_TEST_READY", &ready)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        wait_file(&ready);
        child.kill().unwrap();
        child.wait().unwrap();
    }
}

/// Each fixture holds copies of two programs, so a suite that kept them would leave
/// gigabytes in the temporary folder. A test that fails keeps its folder to be read.
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn wait_file(path: &Path) {
    let until = Instant::now() + Duration::from_secs(20);
    while !path.exists() {
        assert!(
            Instant::now() < until,
            "timed out waiting for {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// What a runtime can say of how it was started: whether its launcher was a
/// generation's, and whether the package's record reached it, which it must not.
fn started() -> (bool, bool) {
    let session = std::env::var("BAYLEE_LAUNCH_SESSION").unwrap();
    let session: serde_json::Value = serde_json::from_str(&session).unwrap();
    (
        session["handed_on"].as_bool().unwrap_or(false),
        std::env::var_os("BAYLEE_LAUNCH_PACKAGE").is_some(),
    )
}

#[test]
#[ignore = "subprocess helper"]
fn runtime_helper() {
    let Some(result) = std::env::var_os("BAYLEE_TEST_RESULT") else {
        return;
    };
    if let Some(record) = std::env::var_os("BAYLEE_TEST_RECORD") {
        fs::write(record, std::env::var("BAYLEE_LAUNCH_PACKAGE").unwrap()).unwrap();
        return;
    }
    if let Some(ready) = std::env::var_os("BAYLEE_TEST_BEFORE_JOIN") {
        fs::write(ready, "ready").unwrap();
        wait_file(Path::new(&std::env::var_os("BAYLEE_TEST_JOIN_GO").unwrap()));
        assert!(
            launch::join().is_err(),
            "late orphan must reject its superseded token"
        );
        fs::write(
            std::env::var_os("BAYLEE_TEST_STALE_DONE").unwrap(),
            "refused",
        )
        .unwrap();
        return;
    }
    let (install, lease) = launch::join().unwrap().expect("launcher lease");
    let exe = std::env::current_exe().unwrap();
    if let Some(second) = std::env::var_os("BAYLEE_TEST_RELAUNCH") {
        relaunched(
            &install,
            &lease,
            &exe,
            Path::new(&result),
            Path::new(&second),
        );
        return;
    }
    let (handed_on, told) = started();
    let value = serde_json::json!({
        "version": fs::read_to_string(exe.with_file_name("version.txt")).unwrap(),
        "writable": lease.writable(), "exe": exe,
        "handed_on": handed_on, "told": told,
        "args": std::env::args().skip(1).collect::<Vec<_>>(),
    });
    fs::write(result, value.to_string()).unwrap();
    if let Some(stop) = std::env::var_os("BAYLEE_TEST_STOP") {
        wait_file(Path::new(&stop));
    }
}

/// "Restart now", as the client does it: the first run installs the staged
/// update as it ends and leaves the helper behind with a handoff; the run
/// the helper starts writes what it is and what it was handed.
fn relaunched(
    install: &Install,
    lease: &launch::ClientLease,
    exe: &Path,
    result: &Path,
    second: &Path,
) {
    let version = fs::read_to_string(exe.with_file_name("version.txt")).unwrap();
    if result.exists() {
        let handoff = relaunch::handoff_from_stdin(Duration::from_secs(10)).unwrap_or_default();
        let (handed_on, told) = started();
        let value = serde_json::json!({
            "version": version,
            "handoff": String::from_utf8(handoff).unwrap(),
            "handed_on": handed_on, "told": told,
        });
        let tmp = second.with_extension("tmp");
        fs::write(&tmp, value.to_string()).unwrap();
        fs::rename(tmp, second).unwrap();
        return;
    }
    fs::write(
        result,
        serde_json::json!({ "version": version }).to_string(),
    )
    .unwrap();
    let again = relaunch::Again::launcher(
        install,
        lease.original(),
        ["--ignored", "--exact", "runtime_helper", "--nocapture"]
            .map(std::ffi::OsString::from)
            .to_vec(),
    );
    let mut helper = Command::new(exe);
    helper.args([
        "--ignored",
        "--exact",
        "--nocapture",
        "relaunch_helper",
        "--",
    ]);
    let handoff = relaunch::leave_behind(helper, &again, b"handed over").unwrap();
    launch::activate(install, "original").unwrap();
    // Held to the end of the process, as the client holds it.
    std::mem::forget(handoff);
}

#[test]
#[ignore = "subprocess helper"]
fn relaunch_helper() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let Some(again) = relaunch::asked(&args) else {
        return;
    };
    relaunch::helper(std::io::stdin(), &again, relaunch::WAIT).unwrap();
}

/// The whole of "Restart now" through the packaged launcher: the helper
/// waits for the old runtime and its launcher, the launcher it starts
/// selects the update the old runtime installed on its way out, and the new
/// runtime finds the handoff on its stdin.
#[test]
fn restart_now_starts_the_installed_update_with_the_handoff() {
    let fixture = Fixture::new("relaunch");
    fixture.stage("1.0.0-beta.6");
    let second = fixture.root.join("second.json");
    let status = fixture
        .command()
        .env("BAYLEE_TEST_RELAUNCH", &second)
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(fixture.result()["version"], "original");
    wait_file(&second);
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&second).unwrap()).unwrap();
    assert_eq!(
        value["version"], "1.0.0-beta.6",
        "the update, started again"
    );
    assert_eq!(value["handoff"], "handed over");
}

#[test]
#[ignore = "subprocess helper"]
fn activation_helper() {
    let Ok(value) = std::env::var("BAYLEE_TEST_INSTALL") else {
        return;
    };
    let install: Install = serde_json::from_str(&value).unwrap();
    let transaction = Activation::begin(&install, "original").unwrap();
    let phase = std::env::var("BAYLEE_TEST_PHASE").unwrap();
    if phase != "intent" {
        transaction.place().unwrap();
    }
    if phase == "committed" {
        transaction.commit().unwrap();
    }
    fs::write(std::env::var_os("BAYLEE_TEST_READY").unwrap(), "ready").unwrap();
    std::thread::sleep(Duration::from_secs(30));
}

#[test]
fn normal_launch_path_recovers_after_process_death_at_each_activation_boundary() {
    for phase in ["intent", "placed", "committed"] {
        let fixture = Fixture::new(phase);
        fixture.stage("1.0.0-beta.3");
        fixture.interrupted(phase);
        assert!(fixture.entry.is_file(), "normal entry must never disappear");
        assert!(fixture.command().status().unwrap().success());
        assert_eq!(fixture.result()["version"], "1.0.0-beta.3");
        assert!(!fixture.install.stage().join("activation.json").exists());
    }
}

#[test]
fn missing_pending_payload_keeps_previous_client_launchable_and_retains_intent() {
    let fixture = Fixture::new("lost-payload");
    fixture.stage("1.0.0-beta.3");
    fixture.interrupted("intent");
    fs::remove_dir_all(fixture.install.stage().join(NEW)).unwrap();
    assert!(fixture.command().status().unwrap().success());
    assert_eq!(fixture.result()["version"], "original");
    assert!(fixture.install.stage().join("activation.json").exists());
}

#[test]
fn competing_process_cannot_activate_while_the_first_owns_staging() {
    let fixture = Fixture::new("activation-lock");
    fixture.stage("1.0.0-beta.3");
    let ready = fixture.root.join("ready");
    let mut first = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "activation_helper", "--nocapture"])
        .env(
            "BAYLEE_TEST_INSTALL",
            serde_json::to_string(&fixture.install).unwrap(),
        )
        .env("BAYLEE_TEST_PHASE", "intent")
        .env("BAYLEE_TEST_READY", &ready)
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    wait_file(&ready);
    assert_eq!(
        Activation::begin(&fixture.install, "original")
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(apply::recover(&fixture.install), apply::Recovery::Deferred);
    let asset = baylee_update::release::Asset {
        name: "baylee-client-2.0.0-x86_64-unknown-linux-gnu.tar.gz".into(),
        size: 1,
        browser_download_url: "http://127.0.0.1:9/never".into(),
    };
    let offer = baylee_update::release::Offer {
        version: "2.0.0".parse().unwrap(),
        tag: "v2.0.0".into(),
        page: String::new(),
        files: Some(baylee_update::release::Files {
            archive: asset.clone(),
            signature: Some(asset),
            checksum: None,
        }),
    };
    let mut checker = baylee_update::check::Checker::new("http://127.0.0.1:9/never", "1.0.0");
    let result = checker.stage(&fixture.install, &offer, &[]);
    assert!(
        matches!(result, Err(baylee_update::check::Manual::Download(ref why)) if why.contains("another process"))
    );
    assert!(
        fixture.install.stage().join(NEW).exists(),
        "competing stager must not clear the claimed payload"
    );
    first.kill().unwrap();
    first.wait().unwrap();
    assert!(fixture.command().status().unwrap().success());
}

fn start_held(fixture: &Fixture) -> (Child, PathBuf) {
    let _ = fs::remove_file(&fixture.result);
    let stop = fixture.root.join("stop");
    let child = fixture
        .command()
        .env("BAYLEE_TEST_STOP", &stop)
        .spawn()
        .unwrap();
    wait_file(&fixture.result);
    (child, stop)
}

#[test]
fn second_launcher_is_refused_even_after_first_launcher_dies() {
    let fixture = Fixture::new("orphan");
    let (mut first, stop) = start_held(&fixture);
    assert!(!fixture.command().status().unwrap().success());
    first.kill().unwrap();
    first.wait().unwrap();
    assert!(
        !fixture.command().status().unwrap().success(),
        "orphan runtime still holds its lease"
    );
    fs::write(stop, "exit").unwrap();
    fixture.starts_when_free();
}

#[test]
fn a_late_child_of_a_dead_launcher_cannot_join_a_new_session() {
    let fixture = Fixture::new("late-orphan");
    let ready = fixture.root.join("before-join");
    let go = fixture.root.join("join-go");
    let done = fixture.root.join("stale-done");
    let mut first = fixture
        .command()
        .env("BAYLEE_TEST_BEFORE_JOIN", &ready)
        .env("BAYLEE_TEST_JOIN_GO", &go)
        .env("BAYLEE_TEST_STALE_DONE", &done)
        .spawn()
        .unwrap();
    wait_file(&ready);
    first.kill().unwrap();
    first.wait().unwrap();
    let (mut second, stop) = start_held(&fixture);
    fs::write(go, "join").unwrap();
    wait_file(&done);
    fs::write(stop, "exit").unwrap();
    assert!(second.wait().unwrap().success());
}

#[test]
fn obsolete_payloads_are_pruned_only_after_all_runtimes_exit() {
    let fixture = Fixture::new("prune");
    for version in ["1.0.0", "1.1.0", "1.2.0"] {
        fixture.stage(version);
        launch::activate(&fixture.install, "original").unwrap();
    }
    let versions = fixture.install.stage().join("versions");
    assert_eq!(fs::read_dir(&versions).unwrap().count(), 3);
    assert!(fixture.command().status().unwrap().success());
    assert_eq!(fs::read_dir(&versions).unwrap().count(), 2);
    let (mut first, stop) = start_held(&fixture);
    fixture.stage("1.3.0");
    launch::activate(&fixture.install, "1.2.0").unwrap();
    assert!(!fixture.command().status().unwrap().success());
    assert_eq!(fs::read_dir(&versions).unwrap().count(), 3);
    fs::write(stop, "exit").unwrap();
    first.wait().unwrap();
    assert!(fixture.command().status().unwrap().success());
    assert_eq!(fs::read_dir(&versions).unwrap().count(), 2);
}

#[cfg(unix)]
#[test]
fn readonly_installation_launches_original_and_selected_payload_without_auto_install() {
    use std::os::unix::fs::PermissionsExt as _;
    let fixture = Fixture::new("readonly");
    fs::set_permissions(&fixture.install.base, fs::Permissions::from_mode(0o555)).unwrap();
    let probe = fs::write(fixture.install.base.join("probe"), "");
    assert!(
        probe.is_err(),
        "readonly regression must run as an unprivileged user"
    );
    assert!(fixture.command().status().unwrap().success());
    assert_eq!(fixture.result()["version"], "original");
    assert_eq!(fixture.result()["writable"], false);
    fixture.stage("1.0.0");
    launch::activate(&fixture.install, "original").unwrap();
    assert!(fixture.command().status().unwrap().success());
    assert_eq!(fixture.result()["version"], "1.0.0");
    assert_eq!(fixture.result()["writable"], false);
    fs::set_permissions(&fixture.install.base, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The packaged launcher's own version (this workspace's) is newer than
/// the installed update: the player installed a newer package by hand, so
/// the original starts, and the runtime accepts the session that says so.
#[test]
fn a_newer_package_starts_instead_of_an_older_installed_update() {
    let fixture = Fixture::new("hand-installed");
    fixture.stage("0.0.1");
    launch::activate(&fixture.install, "original").unwrap();
    assert!(fixture.command().status().unwrap().success());
    assert_eq!(fixture.result()["version"], "original");
}

/// A release that is installed is started by that release's own launcher, in the
/// package's one state, and the client joining says it went well.
#[test]
fn a_package_hands_each_start_to_the_generations_launcher() {
    let fixture = Fixture::new("handed-on");
    fixture.stage_whole("9.0.0");
    launch::activate(&fixture.install, "original").unwrap();
    assert!(fixture.command().status().unwrap().success());
    let result = fixture.result();
    assert_eq!(result["version"], "9.0.0");
    assert_eq!(result["handed_on"], true);
    assert_eq!(result["told"], false, "the record stops at the launcher");
    assert_eq!(fixture.attempts(), None);
    let states = fs::read_dir(fixture.root.join("state/baylee")).unwrap();
    assert_eq!(states.count(), 1, "the generation keys no state of its own");
}

/// A launcher that starts and dies at once: two starts are lost to it, and from the
/// third the package's own logic starts the client, for as long as that generation
/// stands. The client it starts says nothing for the launcher that failed. The next
/// release gets its own attempts.
#[test]
fn a_generations_launcher_that_dies_is_passed_over_after_two_starts() {
    let fixture = Fixture::new("dying");
    fixture.stage_with_a_dying_launcher("9.0.0");
    launch::activate(&fixture.install, "original").unwrap();
    for attempt in 1..=2 {
        assert!(!fixture.command().status().unwrap().success());
        assert!(!fixture.result.exists(), "no client came of it");
        assert_eq!(fixture.attempts(), Some(attempt));
    }
    for _ in 0..2 {
        assert!(fixture.command().status().unwrap().success());
        let result = fixture.result();
        assert_eq!(result["version"], "9.0.0");
        assert_eq!(result["handed_on"], false);
        assert_eq!(fixture.attempts(), Some(2));
        fs::remove_file(&fixture.result).unwrap();
    }
    fixture.stage_whole("9.1.0");
    launch::activate(&fixture.install, "original").unwrap();
    assert!(fixture.command().status().unwrap().success());
    assert_eq!(fixture.result()["version"], "9.1.0");
    assert_eq!(fixture.result()["handed_on"], true);
    assert_eq!(fixture.attempts(), None);
}

/// `launch::chosen` comes first: a package at least as new as the pointer starts no
/// generation's launcher, and this one would die if it did.
#[test]
fn a_package_as_new_as_the_generation_starts_no_generation() {
    for version in [env!("CARGO_PKG_VERSION"), "0.0.1"] {
        let fixture = Fixture::new("as-new");
        fixture.stage_with_a_dying_launcher(version);
        launch::activate(&fixture.install, "original").unwrap();
        assert!(fixture.command().status().unwrap().success());
        assert_eq!(fixture.result()["version"], "original");
        assert_eq!(fixture.result()["handed_on"], false);
        assert_eq!(fixture.attempts(), None);
    }
}

/// `current.json` is the commit, so an activation cut short leaves the first stage
/// on the generation before it, whole, and that generation's launcher finishes the
/// job. It runs from a generation the cleanup after it must keep.
#[test]
fn an_activation_cut_short_is_finished_by_the_previous_generations_launcher() {
    for phase in ["intent", "placed", "committed"] {
        let fixture = Fixture::new(phase);
        fixture.stage_whole("9.0.0");
        launch::activate(&fixture.install, "original").unwrap();
        fixture.stage_whole("9.1.0");
        fixture.interrupted(phase);
        assert!(fixture.command().status().unwrap().success());
        let result = fixture.result();
        assert_eq!(result["version"], "9.1.0", "{phase}");
        assert_eq!(result["handed_on"], true, "{phase}");
        assert!(!fixture.install.stage().join("activation.json").exists());
        let kept = fs::read_dir(fixture.install.stage().join("versions")).unwrap();
        assert_eq!(kept.count(), 2, "the selected one and its predecessor");
    }
}

/// A generation with no launcher in it (a tree from before the first stage, or a
/// damaged one) costs no attempt: the package's own logic starts at once.
#[test]
fn a_generation_without_its_launcher_is_left_to_the_package() {
    let fixture = Fixture::new("no-launcher");
    fixture.stage("9.0.0");
    launch::activate(&fixture.install, "original").unwrap();
    assert!(fixture.command().status().unwrap().success());
    assert_eq!(fixture.result()["version"], "9.0.0");
    assert_eq!(fixture.result()["handed_on"], false);
    assert_eq!(fixture.attempts(), None);
}

/// The second stage judges by the package's version, never its own. Both stages are
/// one build here, so the record says what a package of another version would.
#[test]
fn the_second_stage_judges_by_the_version_it_was_handed() {
    let fixture = Fixture::new("handed-version");
    fixture.stage_whole("9.0.0");
    launch::activate(&fixture.install, "original").unwrap();
    let record = serde_json::json!({ "install": fixture.install, "version": "99.0.0" });
    let status = fixture
        .command()
        .env("BAYLEE_LAUNCH_PACKAGE", record.to_string())
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(fixture.result()["version"], "original");
    assert_eq!(fixture.result()["handed_on"], true);
}

/// What the first stage writes, read as a generation's launcher reads it. A later
/// launcher has to understand it for as long as this package is installed, and no
/// start can show a wrong version: the first stage only hands on what is newer than
/// the package, so any older version the record named would choose the same.
#[test]
fn the_first_stage_tells_the_second_which_package_it_is() {
    let fixture = Fixture::new("record");
    fixture.stage_with_a_dying_launcher("9.0.0");
    launch::activate(&fixture.install, "original").unwrap();
    let record = fixture.root.join("record.json");
    let status = fixture
        .command()
        .env("BAYLEE_TEST_RECORD", &record)
        .status()
        .unwrap();
    assert!(status.success());
    let handed: serde_json::Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(handed["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        handed["install"],
        serde_json::to_value(&fixture.install).unwrap()
    );
}

/// The arguments cross both stages as they were given.
#[test]
fn the_arguments_reach_the_client_whole() {
    let fixture = Fixture::new("arguments");
    fixture.stage_whole("9.0.0");
    launch::activate(&fixture.install, "original").unwrap();
    let spelled = "baylee:join?name=eine Tür \"auf\"";
    assert!(fixture.command().arg(spelled).status().unwrap().success());
    let args = fixture.result()["args"].clone();
    assert_eq!(
        args,
        serde_json::json!([
            "--ignored",
            "--exact",
            "runtime_helper",
            "--nocapture",
            spelled
        ])
    );
    assert_eq!(fixture.result()["handed_on"], true);
}

/// "Restart now" through both stages: the handoff crosses the first stage on the
/// pipe, and the first stage the helper starts is one and not a second stage.
#[test]
fn restart_now_crosses_both_stages_with_the_handoff_whole() {
    let fixture = Fixture::new("relaunch-handed");
    fixture.stage_whole("9.0.0");
    launch::activate(&fixture.install, "original").unwrap();
    fixture.stage_whole("9.1.0");
    let second = fixture.root.join("second.json");
    let status = fixture
        .command()
        .env("BAYLEE_TEST_RELAUNCH", &second)
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(fixture.result()["version"], "9.0.0");
    wait_file(&second);
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&second).unwrap()).unwrap();
    assert_eq!(value["version"], "9.1.0", "the update, started again");
    assert_eq!(value["handoff"], "handed over");
    assert_eq!(value["handed_on"], true);
    assert_eq!(value["told"], false);
}

/// A second start finds a client running. That is the generation's launcher working,
/// so it costs no attempt, however often it happens.
#[test]
fn a_second_start_costs_the_generation_no_attempt() {
    let fixture = Fixture::new("second-start");
    fixture.stage_whole("9.0.0");
    launch::activate(&fixture.install, "original").unwrap();
    let (mut first, stop) = start_held(&fixture);
    for _ in 0..3 {
        assert!(!fixture.command().status().unwrap().success());
        assert_eq!(fixture.attempts(), None);
    }
    fs::write(stop, "exit").unwrap();
    assert!(first.wait().unwrap().success());
}
