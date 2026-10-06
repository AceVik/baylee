//! Exercise the actual packaged entry executable, with a small test runtime.
//! Helpers run as separate OS processes; no in-process mutex can pass these.
mod support;

use baylee_update::apply::{self, Install, Staged};
use baylee_update::launch::{self, Activation};
use baylee_update::plan::{NEW, Os};
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

struct Fixture {
    root: PathBuf,
    install: Install,
    entry: PathBuf,
    result: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
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

#[test]
#[ignore = "subprocess helper"]
fn runtime_helper() {
    let Some(result) = std::env::var_os("BAYLEE_TEST_RESULT") else {
        return;
    };
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
    let (_, lease) = launch::join().unwrap().expect("launcher lease");
    let exe = std::env::current_exe().unwrap();
    let value = serde_json::json!({
        "version": fs::read_to_string(exe.with_file_name("version.txt")).unwrap(),
        "writable": lease.writable(), "exe": exe,
    });
    fs::write(result, value.to_string()).unwrap();
    if let Some(stop) = std::env::var_os("BAYLEE_TEST_STOP") {
        wait_file(Path::new(&stop));
    }
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
