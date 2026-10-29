//! Reboots the server for pending updates only while no game of this machine
//! runs. Installed as `/usr/local/sbin/baylee-reboot` and run by
//! `baylee-reboot.timer` every five minutes; unattended-upgrades itself never
//! reboots (`Automatic-Reboot "false"` in /etc/apt/apt.conf.d/52baylee).
//!
//! A reboot is due when `/run/reboot-required` exists. It happens only inside
//! the night window (`BAYLEE_REBOOT_WINDOW`, local hours, default `3-7`
//! meaning 03:00–07:59) and only when /health counts no game running here
//! and no room waiting. An unreachable gateway is not taken for an empty one.
//!
//! Games can go on through every night. After `BAYLEE_REBOOT_DRAIN_DAYS` (3)
//! of waiting, admission is held while the owner agent keeps its games alive.
//! The reboot follows only after a fresh held-admission health response counts
//! no local game. Closing the window removes only this command's own hold.
use crate::{gateway, settings, sh};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

const TAG: &str = "baylee-reboot";
const HOLD_OWNER: &str = "baylee-reboot admission hold\n";

fn var(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

/// One look at the machine; always exit 0 unless the process itself fails.
pub fn main(_args: &[String]) -> i32 {
    // Moved only by its test.
    let run = PathBuf::from(var("BAYLEE_REBOOT_RUN", "/run"));
    let etc = PathBuf::from(var("BAYLEE_REBOOT_ETC", "/etc/baylee"));
    let deploy_lock = PathBuf::from(var("BAYLEE_REBOOT_DEPLOY_LOCK", "/opt/baylee/state/lock"));
    let window = var("BAYLEE_REBOOT_WINDOW", "3-7");
    let drain_days: u64 = var("BAYLEE_REBOOT_DRAIN_DAYS", "3").parse().unwrap_or(3);
    let drained = run.join("baylee-reboot.drained");
    let required = run.join("reboot-required");
    // Serialise even releasing a hold with deployment operations.
    let Ok(_held) = hold(&deploy_lock) else {
        sh::log(TAG, "a deploy is running; not now");
        return 0;
    };
    let configured = settings::read(&etc.join("gateway.env"));
    let admission = configured
        .iter()
        .find(|(k, _)| k == "BAYLEE_ADMISSION_HOLD")
        .map(|(_, value)| PathBuf::from(value));

    if !required.exists() {
        undrain(admission.as_deref(), &drained);
        return 0;
    }
    let (first, last) = window.split_once('-').map_or((3, 7), |(a, b)| {
        (a.parse().unwrap_or(3), b.parse().unwrap_or(7))
    });
    let hour = local_hour();
    if hour < first || hour > last {
        undrain(admission.as_deref(), &drained);
        return 0;
    }
    let field = if configured
        .iter()
        .any(|(k, v)| k == "BAYLEE_UNIX_SOCKET" && !v.is_empty())
    {
        "local_running"
    } else {
        "running"
    };
    let games = gateway::fetch("health", 5).and_then(|body| gateway::open_games(&body, field));
    let shown = |n: Option<u64>| n.map_or_else(|| "?".to_owned(), |n| n.to_string());
    let (running, waiting) = (games.map(|g| g.0), games.map(|g| g.1));
    let age_days = age_secs(&required) / 86_400;
    if running.is_none() {
        sh::log(TAG, "gateway health unavailable; owner agent left running");
        return 0;
    }
    let quiet = running == Some(0) && waiting == Some(0);
    if !(drained.exists() || quiet) && age_days < drain_days {
        return 0;
    }
    let Some(admission) = admission.filter(|p| p.is_absolute()) else {
        sh::log(
            TAG,
            "reboot requires BAYLEE_ADMISSION_HOLD; owner agent left running",
        );
        return 0;
    };
    if !claim_hold(&admission) {
        sh::log(
            TAG,
            "admission hold belongs to another operation or cannot be created",
        );
        return 0;
    }
    let draining = drained.exists() || age_days >= drain_days;
    if draining {
        let _ = std::fs::write(&drained, "admission held; owner running\n");
    }
    // The initial count predates the hold. Only this fresh response can
    // authorise reboot, closing the last-game/new-game admission race.
    let health = gateway::fetch("health", 5);
    let held = health
        .as_deref()
        .and_then(gateway::health)
        .is_some_and(|(_, _, admission)| admission == "held");
    let games = health
        .as_deref()
        .and_then(|body| gateway::open_games(body, field));
    let (running, waiting) = (games.map(|g| g.0), games.map(|g| g.1));
    if !held {
        sh::log(
            TAG,
            "waiting for verified admission hold; owner agent left running",
        );
        return 0;
    }
    if !draining && (running != Some(0) || waiting != Some(0)) {
        undrain(Some(&admission), &drained);
        return 0;
    }
    if running == Some(0) && (waiting == Some(0) || draining) {
        let _ = std::fs::remove_file(&drained);
        reboot(&run);
        return 0;
    }
    sh::log(
        TAG,
        &format!(
            "reboot pending: {} game(s) running here, {} waiting",
            shown(running),
            shown(waiting)
        ),
    );
    0
}

fn reboot(run: &Path) {
    let packages = std::fs::read_to_string(run.join("reboot-required.pkgs")).map_or_else(
        |_| "updates".to_owned(),
        |t| t.lines().collect::<Vec<_>>().join(" ") + " ",
    );
    sh::log(TAG, &format!("rebooting for {packages}"));
    sh::ok(Command::new("systemctl").arg("reboot"));
}

fn undrain(admission: Option<&Path>, drained: &Path) {
    if let Some(path) = admission
        && std::fs::read_to_string(path).ok().as_deref() == Some(HOLD_OWNER)
    {
        let _ = std::fs::remove_file(path);
    }
    if drained.exists() {
        let _ = std::fs::remove_file(drained);
        sh::log(
            TAG,
            "reboot admission hold released; owner agent kept running",
        );
    }
}

/// An existing foreign hold is never adopted, overwritten or removed.
fn claim_hold(path: &Path) -> bool {
    use std::io::Write as _;
    if std::fs::read_to_string(path).ok().as_deref() == Some(HOLD_OWNER) {
        return true;
    }
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return false;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut file| {
            file.write_all(HOLD_OWNER.as_bytes())
                .and_then(|()| file.sync_all())
        })
        .is_ok()
}

/// The deploy's lock, taken without waiting; a missing lock file holds
/// nothing. `Err` when a deploy holds it.
fn hold(lock: &Path) -> Result<Option<std::fs::File>, ()> {
    if !lock.exists() {
        return Ok(None);
    }
    let file = std::fs::File::open(lock).map_err(|_| ())?;
    file.try_lock().map_err(|_| ())?;
    Ok(Some(file))
}

fn age_secs(path: &Path) -> u64 {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|m| m.elapsed().ok())
        .map_or(0, |d| d.as_secs())
}

/// The local hour, as `date +%H` gives it (the server's own time zone).
fn local_hour() -> u32 {
    sh::out_quiet(Command::new("date").arg("+%H"))
        .and_then(|h| h.trim().parse().ok())
        .unwrap_or(12)
}
