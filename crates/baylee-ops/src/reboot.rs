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
//!
//! With `BAYLEE_DEPLOY_KEEP_GAMES=1` in `deploy.env`, running games do not
//! hold the reboot while every one is kept (`all_kept`): the gateway is
//! stopped first, so it stores them before the agent's engines go, and
//! resumes them after the reboot. Waiting rooms still hold it as before.
//!
//! A backup run (the separate `baylee-backup` service) holds
//! `/run/baylee-backup/run.lock` while it dumps, uploads, prunes or restores.
//! The reboot waits for it as it waits for a deploy, before it claims an
//! admission hold, and keeps the lock to itself until it has asked for the
//! reboot, so that no backup starts in between.
use crate::{gateway, settings, sh};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

const TAG: &str = "baylee-reboot";
const HOLD_OWNER: &str = "baylee-reboot admission hold\n";
/// The lock every run of `baylee-backup` holds; both repositories name it.
const BACKUP_LOCK: &str = "/run/baylee-backup/run.lock";

fn var(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

/// One look at the machine; always exit 0 unless the process itself fails.
pub fn main(_args: &[String]) -> i32 {
    // Moved only by its test.
    let run = PathBuf::from(var("BAYLEE_REBOOT_RUN", "/run"));
    let etc = PathBuf::from(var("BAYLEE_REBOOT_ETC", "/etc/baylee"));
    let deploy_lock = PathBuf::from(var("BAYLEE_REBOOT_DEPLOY_LOCK", "/opt/baylee/state/lock"));
    let backup_lock = PathBuf::from(var("BAYLEE_REBOOT_BACKUP_LOCK", BACKUP_LOCK));
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
    if !in_window(&window, local_hour()) {
        undrain(admission.as_deref(), &drained);
        return 0;
    }
    let field = counted_games(&configured);
    let games = gateway::fetch("health", 5).and_then(|body| gateway::open_games(&body, field));
    let shown = |n: Option<u64>| n.map_or_else(|| "?".to_owned(), |n| n.to_string());
    let (running, waiting) = (games.map(|g| g.0), games.map(|g| g.1));
    let age_days = age_secs(&required) / 86_400;
    if running.is_none() {
        sh::log(TAG, "gateway health unavailable; owner agent left running");
        return 0;
    }
    let kept = || all_kept(&etc, &deploy_lock);
    let quiet = (running == Some(0) || kept()) && waiting == Some(0);
    if !(drained.exists() || quiet) && age_days < drain_days {
        return 0;
    }
    // Before any hold is claimed: a backup can take an hour, and admission
    // must not stay closed for it. Held to the end, so none starts meanwhile.
    let Ok(_no_backup) = hold(&backup_lock) else {
        sh::log(TAG, "a backup is running; not now");
        return 0;
    };
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
    let keeping = running != Some(0) && kept();
    if !draining && ((running != Some(0) && !keeping) || waiting != Some(0)) {
        undrain(Some(&admission), &drained);
        return 0;
    }
    if (running == Some(0) || keeping) && (waiting == Some(0) || draining) {
        let _ = std::fs::remove_file(&drained);
        reboot(&run, keeping);
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

/// Reboots; `keeping` running games, stops the gateway first, before
/// systemd stops the agent whose engines hold them.
fn reboot(run: &Path, keeping: bool) {
    if keeping {
        sh::log(
            TAG,
            "stopping the gateway first: it keeps the running games",
        );
        sh::ok(Command::new("systemctl").args(["stop", "baylee-gateway"]));
    }
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

/// Another operation's lock (a deploy's, a backup's), taken without waiting;
/// a missing lock file holds nothing. `Err` when that operation holds it.
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

/// Whether every running game is kept across a restart and comes back after
/// the reboot: the setting is on, none has a seat bridge, and each plays on
/// the deployed build or on an older one the agent's list names
/// (`deploy/keep.rs`). A gateway that does not say is not taken for one that
/// keeps everything.
fn all_kept(etc: &Path, deploy_lock: &Path) -> bool {
    if settings::last(&[&etc.join("deploy.env")], "BAYLEE_DEPLOY_KEEP_GAMES").as_deref()
        != Some("1")
    {
        return false;
    }
    let Some(games) = gateway::fetch("health", 5).and_then(|body| gateway::kept(&body)) else {
        return false;
    };
    let deployed = deploy_lock
        .parent()
        .and_then(|state| std::fs::read_to_string(state.join("deployed")).ok())
        .unwrap_or_default();
    let listed = std::fs::read_to_string(etc.join("agent-engines.env")).unwrap_or_default();
    games.kept == games.running
        && games.bridged == 0
        && games.revisions.iter().all(|revision| {
            revision == deployed.trim() || listed.contains(&format!("/{revision}/"))
        })
}

/// The `/health` count that names this machine's games: the local one where
/// the gateway has its unix socket.
fn counted_games(configured: &[(String, String)]) -> &'static str {
    if configured
        .iter()
        .any(|(k, v)| k == "BAYLEE_UNIX_SOCKET" && !v.is_empty())
    {
        "local_running"
    } else {
        "running"
    }
}

/// Whether the hour lies in a window `first-last`, both hours included.
fn in_window(window: &str, hour: u32) -> bool {
    let (first, last) = window.split_once('-').map_or((3, 7), |(a, b)| {
        (a.parse().unwrap_or(3), b.parse().unwrap_or(7))
    });
    (first..=last).contains(&hour)
}

/// The local hour, as `date +%H` gives it (the server's own time zone).
fn local_hour() -> u32 {
    sh::out_quiet(Command::new("date").arg("+%H"))
        .and_then(|h| h.trim().parse().ok())
        .unwrap_or(12)
}

#[cfg(test)]
mod tests;
