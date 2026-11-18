//! Daily and Weekly tasks for the stack's Central (`docs/challenges.md`).
//!
//! A fresh Central has no task, and a task is certified for one engine build,
//! so every start publishes its own with the `challenge` publisher built from
//! the same commit as the engines. Two passes, so there is something to play
//! soon: today's Daily and this week's Weekly first, then the days ahead and
//! the Weekly expedition, whose certification takes long. Neither holds the
//! stack's start back, and a failed pass is reported, not fatal: the game
//! runs without tasks.
#![forbid(unsafe_code)]

use super::{
    central,
    services::{ManagedChild, staged_exe},
};
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// One run of `challenge stock`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pass {
    /// Today's Daily and this week's Weekly.
    Today,
    /// The days ahead, and the Weekly Tower expedition.
    Ahead,
}

impl Pass {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Today => "challenge-stock-today",
            Self::Ahead => "challenge-stock-ahead",
        }
    }

    /// What follows this pass when `days` are to be kept ahead.
    pub(super) const fn next(self, days: u8) -> Option<Self> {
        match self {
            Self::Today if days > 1 => Some(Self::Ahead),
            Self::Today | Self::Ahead => None,
        }
    }

    /// `challenge stock`'s arguments after the endpoint and the state directory.
    pub(super) fn arguments(self, days: u8) -> [String; 4] {
        let (activities, days) = match self {
            Self::Today => ("calendar", 1),
            Self::Ahead => ("all", days),
        };
        [
            "--activities".into(),
            activities.into(),
            "--days".into(),
            days.to_string(),
        ]
    }
}

/// The publisher's passes, run one after the other beside the stack.
pub(super) struct Stock {
    bin_dir: PathBuf,
    logs_dir: PathBuf,
    /// Staged proofs, kept across starts: a pass after a restart on the same
    /// commit republishes what it certified before.
    proofs: PathBuf,
    bridge_hex: String,
    days: u8,
    running: Option<(Pass, ManagedChild)>,
}

impl Stock {
    /// Starts the first pass. `days` of zero publishes nothing.
    pub(super) fn begin(
        bin_dir: &Path,
        logs_dir: &Path,
        state_dir: &Path,
        bridge_hex: &str,
        days: u8,
    ) -> Result<Self> {
        let mut stock = Self {
            bin_dir: bin_dir.to_path_buf(),
            logs_dir: logs_dir.to_path_buf(),
            proofs: state_dir.join("challenge-stock"),
            bridge_hex: bridge_hex.to_owned(),
            days,
            running: None,
        };
        // `status` reads each pass's last line: an earlier start's must not
        // stand in for a pass this one has not reached.
        for pass in [Pass::Today, Pass::Ahead] {
            for stream in ["out", "err"] {
                let _ = fs::remove_file(logs_dir.join(format!("{}.{stream}.log", pass.name())));
            }
        }
        if days > 0 {
            stock.spawn(Pass::Today)?;
        }
        Ok(stock)
    }

    fn spawn(&mut self, pass: Pass) -> Result<()> {
        let child = ManagedChild::spawn(
            pass.name(),
            Command::new(staged_exe(&self.bin_dir, "challenge"))
                .arg("stock")
                .args(["--central-dev", &central::origin(central::NATIVE_PORT)])
                .arg("--state-dir")
                .arg(&self.proofs)
                .args(pass.arguments(self.days))
                // The publisher takes Central's bridge capability from its
                // environment only (`docs/challenges.md`).
                .env("BAYLEE_CHALLENGE_BRIDGE_TOKEN", &self.bridge_hex),
            &self.logs_dir,
        )?;
        println!(
            "Tasks: {} started; its progress is in {}",
            pass.name(),
            self.logs_dir
                .join(format!("{}.out.log", pass.name()))
                .display()
        );
        self.running = Some((pass, child));
        Ok(())
    }

    /// Called from the supervisor's loop: reports a finished pass and starts
    /// the next one.
    pub(super) fn poll(&mut self) -> Result<()> {
        let Some((pass, child)) = &mut self.running else {
            return Ok(());
        };
        let Some(status) = child.exited()? else {
            return Ok(());
        };
        let pass = *pass;
        self.running = None;
        if status.success() {
            println!(
                "Tasks: {} done: {}",
                pass.name(),
                last_line(&self.logs_dir, pass).unwrap_or_else(|| "no summary".into())
            );
            if let Some(next) = pass.next(self.days) {
                self.spawn(next)?;
            }
        } else {
            println!(
                "Tasks: {} failed with {status}; see {}",
                pass.name(),
                self.logs_dir
                    .join(format!("{}.err.log", pass.name()))
                    .display()
            );
        }
        Ok(())
    }
}

/// The last thing a pass said, for `status` and the supervisor's report.
pub(super) fn last_line(logs_dir: &Path, pass: Pass) -> Option<String> {
    ["out", "err"].into_iter().find_map(|stream| {
        let text =
            fs::read_to_string(logs_dir.join(format!("{}.{stream}.log", pass.name()))).ok()?;
        text.lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_owned())
    })
}
