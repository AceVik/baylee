//! The replay-exact audit: about one recorded game in `every`, chosen by its
//! seed and deal, replayed by the converting build's `convert4`.
//!
//! `convert4`'s first step for a game is `record::replay_exact`: every input
//! applied again and the engine's snapshot hash compared after each. It
//! quarantines a game whose replay diverges, is refused or (from the
//! activator boundary on) was written before that boundary, and names why
//! in `rejected-games.json`. The audit gives it a run of only the sampled
//! games, one thread, and an output that is thrown away afterwards; it
//! reads the reasons and nothing else. A replay on the build that wrote the
//! record must reproduce it: a divergence there is a determinism bug. On
//! another build it is a rules change, unless it is the boundary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context as _, bail};
use serde::Serialize;
use serde_json::Value;

use super::{Class, Entry, Kind};

/// What the audit did.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Replay {
    /// `ran`, or why not (`no records`, `no converting build`, …).
    pub status: String,
    /// The `convert4` that replayed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converter: Option<PathBuf>,
    /// Its commit (its runtime's `BUILD.txt`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converter_commit: Option<String>,
    /// The commit that wrote the records (`run.json`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writer_commit: Option<String>,
    /// One finished game in this many is sampled.
    pub every: u64,
    /// Finished games in the records.
    pub finished: u64,
    /// Those of them sampled.
    pub sampled: u64,
    /// Sampled games that replayed exactly.
    pub exact: u64,
    /// The sampled games' deals.
    pub deals: Vec<u64>,
}

impl Replay {
    /// No audit, and why.
    #[must_use]
    pub fn not_run(why: &str) -> Self {
        Self {
            status: why.to_owned(),
            ..Self::default()
        }
    }
}

/// Whether the game at `deal` with `seed` is in the one-in-`every` sample:
/// `SplitMix64` of both, so the sample is the same whoever asks and spreads
/// over tables and seats alike.
#[must_use]
pub fn sampled(seed: u64, deal: u64, every: u64) -> bool {
    if every == 0 {
        return false;
    }
    let mut z = seed ^ deal.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (z ^ (z >> 31)).is_multiple_of(every)
}

/// A sampled game: its deal, seed and focal seat.
pub(super) type Place = (u64, u64, Option<u64>);

/// What `convert4` said of the sampled games (`places`, in the order it was
/// given them; its `game_id` counts from 1): its rejections, and the panics
/// in its stderr, the game in flight then by its worker's journal. A
/// divergence or refused input on the build that wrote the record is a
/// bug; on another build a rules change. A rejection that is not the
/// replay's (an encoding the converter lacks) is left out.
pub(super) fn findings(
    rejected: &[Value],
    stderr: &str,
    in_flight: Option<u64>,
    places: &[Place],
    same_build: Option<bool>,
) -> Vec<Entry> {
    let place = |game_id: u64, entry: Entry| {
        let at = usize::try_from(game_id)
            .ok()
            .and_then(|id| id.checked_sub(1));
        match at.and_then(|at| places.get(at)) {
            Some((deal, seed, seat)) => entry.at(Some(*deal), Some(*seed), None).seat(*seat),
            None => entry,
        }
    };
    let replay_class = if same_build == Some(false) {
        Class::Expected
    } else {
        Class::New
    };
    let mut entries = Vec::new();
    for game in rejected {
        let id = game["game_id"].as_u64().unwrap_or(0);
        let reason = game["reason"].as_str().unwrap_or_default();
        let entry = if reason.starts_with("record written before activator boundary") {
            Entry::new(Kind::Boundary, Class::Known, reason)
        } else if reason.starts_with("replay audit: Diverged") {
            let what = if same_build == Some(false) {
                format!("{reason} (on another build than the one that wrote it)")
            } else {
                reason.to_owned()
            };
            Entry::new(Kind::ReplayDivergence, replay_class, what)
        } else if reason.starts_with("replay audit:") || reason.starts_with("record has no") {
            Entry::new(Kind::ReplayRefusal, replay_class, reason)
        } else {
            continue;
        };
        entries.push(place(id, entry));
    }
    for line in stderr.lines().filter(|l| l.contains("panicked")) {
        let entry = Entry::new(Kind::ReplayPanic, Class::New, line.trim().to_owned());
        entries.push(match in_flight {
            Some(id) => place(id, entry),
            None => entry,
        });
    }
    entries
}

/// The commit a runtime directory was built from (`BUILD.txt`'s `commit=`).
fn runtime_commit(runtime: &Path) -> Option<String> {
    fs::read_to_string(runtime.join("BUILD.txt"))
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("commit=").map(str::to_owned))
}

/// Whether two commits name the same one (one may be abbreviated).
fn same_commit(a: &str, b: &str) -> bool {
    let n = a.len().min(b.len()).min(40);
    n >= 7 && a[..n] == b[..n]
}

/// Audits the records in `records` (a run directory: `run.json`,
/// `games.jsonl`, `records/`) with `convert4`, in `scratch`, which it
/// creates and removes. Returns what it did and the entries it found.
///
/// # Errors
/// Unreadable run files or a `convert4` that cannot be started.
#[allow(clippy::too_many_lines)] // one audit, read top to bottom
pub fn audit(
    records: &Path,
    convert4: &Path,
    every: u64,
    scratch: &Path,
) -> anyhow::Result<(Replay, Vec<Entry>)> {
    let mut replay = Replay {
        status: "ran".to_owned(),
        converter: Some(convert4.to_path_buf()),
        every,
        ..Replay::default()
    };
    let run_json: Value = serde_json::from_slice(
        &fs::read(records.join("run.json")).with_context(|| format!("{}", records.display()))?,
    )?;
    replay.writer_commit = run_json["evaluator_revision"]
        .as_str()
        .or_else(|| run_json["revision"].as_str())
        .map(str::to_owned);
    replay.converter_commit = convert4.parent().and_then(runtime_commit);
    let same_build = match (&replay.writer_commit, &replay.converter_commit) {
        (Some(w), Some(c)) => Some(same_commit(w, c)),
        _ => None,
    };
    let games = fs::read_to_string(records.join("games.jsonl"))?;
    let mut chosen = Vec::new();
    for line in games.lines().filter(|l| !l.trim().is_empty()) {
        let game: Value = serde_json::from_str(line)?;
        // convert4 takes finished games only.
        if !matches!(
            game["outcome"]["kind"].as_str(),
            Some("won" | "team_won" | "draw")
        ) {
            continue;
        }
        replay.finished += 1;
        let (Some(seed), Some(deal)) = (game["seed"].as_u64(), game["i"].as_u64()) else {
            continue;
        };
        if sampled(seed, deal, every) {
            chosen.push((deal, seed, game["focal_seat"].as_u64(), line.to_owned()));
        }
    }
    replay.sampled = chosen.len() as u64;
    replay.deals = chosen.iter().map(|(deal, ..)| *deal).collect();
    if chosen.is_empty() {
        "ran: no game in the sample".clone_into(&mut replay.status);
        return Ok((replay, Vec::new()));
    }
    if scratch.exists() {
        bail!("{} exists", scratch.display());
    }
    let run = scratch.join("run");
    fs::create_dir_all(&run)?;
    fs::copy(records.join("run.json"), run.join("run.json"))?;
    let mut index = String::new();
    for (.., line) in &chosen {
        index.push_str(line);
        index.push('\n');
    }
    fs::write(run.join("games.jsonl"), index)?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(records.join("records").canonicalize()?, run.join("records"))?;
    #[cfg(not(unix))]
    bail!("the replay audit links the records into its run, which needs Unix");
    let out = scratch.join("out");
    let encoder = run_json
        .pointer("/model_identity/encoder_version")
        .and_then(Value::as_u64)
        .unwrap_or(7);
    let status = Command::new(convert4)
        .args(["--runs"])
        .arg(&run)
        .arg("--out")
        .arg(&out)
        .args(["--threads", "1", "--val-every", "0", "--encoder-version"])
        .arg(encoder.to_string())
        .stdout(Stdio::null())
        .stderr(fs::File::create(scratch.join("convert4.stderr.log"))?)
        .status()
        .with_context(|| format!("starting {}", convert4.display()))?;
    let stderr = fs::read_to_string(scratch.join("convert4.stderr.log")).unwrap_or_default();
    let rejected: Vec<Value> = fs::read(out.join("rejected-games.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let in_flight = fs::read(out.join("workers").join("w00.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(|journal| journal["status"] == "replaying")
        .and_then(|journal| journal["game_id"].as_u64());
    let places: Vec<Place> = chosen
        .iter()
        .map(|(deal, seed, seat, _)| (*deal, *seed, *seat))
        .collect();
    let entries = findings(&rejected, &stderr, in_flight, &places, same_build);
    replay.exact = fs::read(out.join("meta.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|meta| meta.pointer("/replay_checks/exact").and_then(Value::as_u64))
        .unwrap_or(0);
    if !status.success() && rejected.is_empty() && entries.is_empty() {
        let tail: Vec<&str> = stderr.lines().rev().take(3).collect();
        replay.status = format!(
            "convert4 failed ({status}): {}",
            tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
        );
    }
    fs::remove_dir_all(scratch).with_context(|| format!("removing {}", scratch.display()))?;
    Ok((replay, entries))
}
