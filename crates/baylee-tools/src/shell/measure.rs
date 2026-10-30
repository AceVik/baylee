//! Hover rebuilds, counted setup presses and real-window quit/Undo flushing.
use super::{
    common::{self, Report, value_array},
    devctl::{Devctl, Http, sleep},
};
use anyhow::{Context, Result, ensure};
use clap::Args;
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
/// The session `measure quit` reads the client's decks with: the one the
/// client signed in with, since a gateway signs nobody in with a password
/// any more. Never an argument, never printed.
#[derive(Args)]
pub(super) struct Session {
    /// Environment variable containing the client's session token (its value is never printed).
    #[arg(long = "session-env", conflicts_with_all = ["file", "stdin"])]
    pub env: Option<String>,
    /// File containing the session token. A final newline is removed.
    #[arg(long = "session-file", conflicts_with = "stdin")]
    pub file: Option<PathBuf>,
    /// Read the session token from standard input (not an interactive prompt).
    #[arg(long = "session-stdin")]
    pub stdin: bool,
}
impl Session {
    pub fn read(&self) -> Result<String> {
        read_secret(
            self.env.as_deref(),
            self.file.as_deref(),
            self.stdin,
            "BAYLEE_SHELL_SESSION",
            "session",
        )
    }
}
/// A secret (`what`: the flags' word) from the named environment variable, a
/// file or standard input, else the `fallback` variable; never from argv,
/// and never in an error. A final newline is removed.
fn read_secret(
    env: Option<&str>,
    file: Option<&Path>,
    stdin: bool,
    fallback: &str,
    what: &str,
) -> Result<String> {
    let mut secret = if let Some(name) = env {
        std::env::var(name)
            .map_err(|_| anyhow::anyhow!("{what} environment variable is unavailable"))?
    } else if let Some(path) = file {
        fs::read_to_string(path).map_err(|_| anyhow::anyhow!("cannot read {what} file"))?
    } else if stdin {
        let mut value = String::new();
        io::stdin()
            .take(65537)
            .read_to_string(&mut value)
            .map_err(|_| anyhow::anyhow!("cannot read {what} stdin"))?;
        value
    } else {
        std::env::var(fallback).map_err(|_| {
            anyhow::anyhow!("supply --{what}-env, --{what}-file or --{what}-stdin (or {fallback})")
        })?
    };
    if secret.ends_with('\n') {
        secret.pop();
        if secret.ends_with('\r') {
            secret.pop();
        }
    }
    ensure!(
        !secret.is_empty() && secret.len() <= 65536,
        "{what} is empty or too long"
    );
    Ok(secret)
}
fn rebuilds(d: &Devctl) -> Result<u64> {
    d.state()?["ui_rebuilds"]["total"]
        .as_u64()
        .context("missing rebuild total")
}
pub(super) fn hover(d: &Devctl) -> Result<()> {
    d.resize(1920, 1080)?;
    d.knobs("m", "en", false)?;
    let mut report = Report::default();
    for screen in ["decks", "play"] {
        common::go(d, screen, 0.7)?;
        d.settle(8.0)?;
        let controls: Vec<_> = d
            .controls()?
            .into_iter()
            .filter(|c| !c.press.starts_with("Header"))
            .take(30)
            .collect();
        ensure!(!controls.is_empty(), "no hover targets");
        let before = rebuilds(d)?;
        for c in &controls {
            d.pointer(c.at_x, c.at_y, false)?;
            d.pointer(c.at_x + 40.0, c.at_y + 30.0, false)?;
        }
        sleep(0.5);
        let after = rebuilds(d)?;
        report.expect(
            before == after,
            &format!(
                "hover {screen}: {} moves over {} controls, rebuilds {before} -> {after}",
                controls.len() * 2,
                controls.len()
            ),
        );
    }
    report.finish(None, false)
}
pub(super) fn presses(d: &Devctl) -> Result<()> {
    d.resize(1920, 1080)?;
    common::go(d, "play", 0.7)?;
    let mut count = 0;
    let mut tap = |name: &str| -> Result<()> {
        d.tap(name, 1.5)?;
        count += 1;
        println!("  {count}: {name}");
        Ok(())
    };
    tap("Play(CreateTable)")?;
    tap("Play(Template(Duel))")?;
    tap("Play(Open)")?;
    Devctl::until(15.0, || Ok(d.shell_state()?["table"] == "room"))?;
    d.settle(8.0)?;
    let ai = d
        .names()?
        .into_iter()
        .find(|n| n.starts_with("Shared(OpenMenu(SeatAi("))
        .context("missing AI seat menu")?;
    tap(&ai)?;
    let pick = d
        .names()?
        .into_iter()
        .find(|n| n.ends_with("\"steady\"))"))
        .context("missing steady AI choice")?;
    tap(&pick)?;
    d.settle(8.0)?;
    let start = d
        .names()?
        .into_iter()
        .find(|n| n.starts_with("Room(StartRoom("))
        .context("missing enabled Start")?;
    tap(&start)?;
    Devctl::until(30.0, || {
        let s = d.shell_state()?;
        Ok(
            !matches!(s["screen"].as_str(), Some("Room" | "Hub" | "Play"))
                && !d.state()?["view"].is_null(),
        )
    })?;
    println!("presses Play -> room -> Start: {count}; game view present");
    Ok(())
}
fn decks(http: &Http, token: &str) -> Result<BTreeSet<String>> {
    let body = http.call("/decks", None, Some(token), Duration::from_secs(60))?;
    let items = if body.is_array() {
        &body
    } else {
        &body["decks"]
    };
    value_array(items)?
        .iter()
        .map(|d| {
            d["id"]
                .as_str()
                .map(str::to_owned)
                .context("deck id missing")
        })
        .collect()
}
fn process_alive(pid: u32) -> Result<bool> {
    Ok(Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success())
}
fn close_window(pid: u32) -> Result<()> {
    ensure!(
        cfg!(target_os = "macos"),
        "quit-window measurement is macOS-only"
    );
    let script = format!(
        "tell application \"System Events\" to tell (first process whose unix id is {pid}) to click button 1 of front window"
    );
    let mut child = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("cannot launch window-close helper")?;
    let end = Instant::now() + Duration::from_secs(15);
    let result = (|| {
        loop {
            if let Some(status) = child.try_wait()? {
                ensure!(status.success(), "window-close helper failed");
                return Ok(());
            }
            if Instant::now() >= end {
                anyhow::bail!("window-close helper timed out");
            }
            sleep(0.1);
        }
    })();
    if result.is_err() {
        let _ = child.kill();
    }
    let waited = child.wait().context("cannot reap window-close helper");
    result?;
    waited?;
    Ok(())
}
pub(super) fn quit(d: &Devctl, gateway: &str, token: &str, pid: u32) -> Result<()> {
    ensure!(
        cfg!(target_os = "macos"),
        "quit-window measurement is macOS-only"
    );
    ensure!(pid > 1 && process_alive(pid)?, "client PID is not live");
    let http = Http::new(gateway)?;
    let before = decks(&http, token)?;
    common::go(d, "decks", 0.7)?;
    d.settle(8.0)?;
    d.tap("Shared(OpenMenu(Deck(0)))", 0.7)?;
    d.tap("Decks(Duplicate(0))", 2.5)?;
    d.settle(8.0)?;
    let copied = decks(&http, token)?;
    let copy: BTreeSet<_> = copied.difference(&before).cloned().collect();
    ensure!(
        copy.len() == 1 && copied.len() == before.len() + 1,
        "duplicate must create exactly one deck"
    );
    println!("decks before {}, with copy {}", before.len(), copied.len());
    d.key("ArrowRight", None, 0.0)?;
    let menu = d
        .names()?
        .into_iter()
        .find(|n| n.starts_with("Shared(OpenMenu(Deck("))
        .context("missing newest deck menu")?;
    d.tap(&menu, 0.7)?;
    let delete = d
        .names()?
        .into_iter()
        .find(|n| n.starts_with("Decks(Delete("))
        .context("missing Delete")?;
    d.tap(&delete, 0.7)?;
    let staged = decks(&http, token)?;
    ensure!(
        copy.is_subset(&staged) && d.has("Decks(Undo)")?,
        "copy must still exist while Undo stands"
    );
    close_window(pid)?;
    sleep(4.0);
    let alive = process_alive(pid)?;
    let after = decks(&http, token)?;
    let mut r = Report::default();
    r.expect(!alive, "client exited after real window close");
    r.expect(
        copy.is_disjoint(&after) && after == before,
        "only duplicated/deleted deck gone after close; original decks unchanged",
    );
    r.finish(None, false)
}
