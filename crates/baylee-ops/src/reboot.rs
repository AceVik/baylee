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
//! of waiting, the window drains like a deploy does: the agent is stopped, so
//! no new game starts, and the reboot follows once the last game is over. If
//! the window closes first, the agent is started again and the next night
//! tries again. Running games are never ended.
use crate::{gateway, settings, sh};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

const TAG: &str = "baylee-reboot";

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

    let undrain = || {
        if drained.exists() {
            let _ = std::fs::remove_file(&drained);
            sh::ok(Command::new("systemctl").args(["start", "baylee-agent"]));
            sh::log(
                TAG,
                "window closed with games still running; agent started again",
            );
        }
    };
    if !required.exists() {
        undrain();
        return 0;
    }
    let (first, last) = window.split_once('-').map_or((3, 7), |(a, b)| {
        (a.parse().unwrap_or(3), b.parse().unwrap_or(7))
    });
    let hour = local_hour();
    if hour < first || hour > last {
        undrain();
        return 0;
    }
    // A deploy in progress owns the agent and the gateway; wait for it.
    let Ok(_held) = hold(&deploy_lock) else {
        sh::log(TAG, "a deploy is running; not now");
        return 0;
    };
    let field = if settings::read(&etc.join("gateway.env"))
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
    if running == Some(0) && (waiting == Some(0) || drained.exists()) {
        let packages = std::fs::read_to_string(run.join("reboot-required.pkgs")).map_or_else(
            |_| "updates".to_owned(),
            |t| t.lines().collect::<Vec<_>>().join(" ") + " ",
        );
        sh::log(TAG, &format!("rebooting for {packages}"));
        let _ = std::fs::remove_file(&drained);
        sh::ok(Command::new("systemctl").arg("reboot"));
        return 0;
    }
    let age_days = age_secs(&required) / 86_400;
    if age_days >= drain_days && !drained.exists() && running.is_some() {
        sh::ok(Command::new("systemctl").args(["stop", "baylee-agent"]));
        let _ = std::fs::File::create(&drained);
        sh::log(
            TAG,
            &format!(
                "reboot pending for {age_days} days: agent stopped, rebooting once {} game(s) end",
                shown(running)
            ),
        );
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
