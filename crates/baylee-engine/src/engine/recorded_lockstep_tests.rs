//! Recorded games played twice in lockstep: by the engine as it is and by
//! the engine as it was before three fixes from Carbon's ladder games
//! (11.11.2026), switched back on by [`as_before`]. The untap step now
//! brings the statics of what phased in along before it untaps
//! (`UNTAP_BEFORE_PHASED_STATICS`), an order between damage effects that
//! cannot change the event is no longer asked (`damage::ASK_EVERY_ORDER`),
//! and a decided game asks nothing more (`ASK_IN_A_DECIDED_GAME`).
//!
//! Both engines take every recorded answer. After the opening and after each
//! answer their hashes are compared, and the first answer they part on is
//! reported: one refused it, or the hashes differ. A record the two never
//! part on replays on the engine as it is exactly as it did on the engine as
//! it was. Beside that, the engine as it was is held to the record's own
//! hashes, so the report says which parted records replayed exactly before.
//!
//! `BAYLEE_LOCKSTEP_ONLY=<an earlier report>` plays only the records that
//! report lists as parted.
//!
//! Ignored: it reads runs from disk, every `games.jsonl` with a `records/`
//! beside it under each root, and writes a parted record per line to a file
//! that must not exist yet.
//!
//! ```text
//! BAYLEE_LOCKSTEP_ROOTS=<dir>:<dir> BAYLEE_LOCKSTEP_OUT=<new file> \
//!   [BAYLEE_LOCKSTEP_THREADS=4] cargo test --profile ci-release -p baylee-engine \
//!   --lib -- --ignored --exact engine::recorded_lockstep_tests::recorded_games_in_lockstep
//! ```
//!
//! Measured 12.11.2026 at about 80 games a second on six threads of a busy
//! machine.

use super::testkit::RegistryLookup;
use super::*;
use std::fs::File;
use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The record version this reads (`baylee_gamehost::record::RECORD_VERSION`).
const RECORD_VERSION: u64 = 1;

/// Runs `f` as the engine was before the three fixes.
fn as_before<T>(f: impl FnOnce() -> T) -> T {
    let set = |on: bool| {
        super::progress::UNTAP_BEFORE_PHASED_STATICS.with(|s| s.set(on));
        super::progress::ASK_IN_A_DECIDED_GAME.with(|s| s.set(on));
        crate::damage::ASK_EVERY_ORDER.with(|s| s.set(on));
    };
    set(true);
    let out = f();
    set(false);
    out
}

/// One recorded game: where its record is.
struct Game {
    run: PathBuf,
    line: usize,
    shard: String,
    offset: u64,
    len: u64,
}

/// What the lockstep found in one record.
#[derive(Debug)]
enum Found {
    /// Never parted; `exact` when the engine as it was matched every
    /// recorded hash, `inputs` the answers both took.
    Alike { exact: bool, inputs: usize },
    /// Both refused input `n`: the record no longer replays on either
    /// engine from there (it was written by an older build). They had not
    /// parted before it.
    BothRefused { n: u64, exact_before: bool },
    /// The two parted at input `n` (`None`: before any input): the question
    /// the engine as it was had asked before it (`asked`), and what each
    /// asks after it. `exact_record`: the engine as it was, going on alone,
    /// took the whole record with every recorded hash, so a replay held to
    /// those hashes passes before the change and fails after it.
    Parted {
        n: Option<u64>,
        why: &'static str,
        exact_before: bool,
        exact_record: bool,
        turn: u32,
        step: String,
        asked: String,
        then: String,
        now: String,
    },
    /// The record could not be read or its preset not built.
    Unreadable(String),
}

impl Found {
    /// The report's line for a record, beside where it is.
    fn json(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            Self::Alike { exact, inputs } => {
                json!({"alike": {"exact": exact, "inputs": inputs}})
            }
            Self::BothRefused { n, exact_before } => {
                json!({"both_refused": {"n": n, "exact_before": exact_before}})
            }
            Self::Parted {
                n,
                why,
                exact_before,
                exact_record,
                turn,
                step,
                asked,
                then,
                now,
            } => json!({"parted": {
                "n": n, "why": why, "exact_before": exact_before,
                "exact_record": exact_record, "turn": turn, "step": step,
                "asked": asked, "then": then, "now": now,
            }}),
            Self::Unreadable(why) => json!({ "unreadable": why }),
        }
    }

    /// A parting, with whether the engine as it was took the whole record
    /// exactly.
    fn exact_record(mut self, value: bool) -> Self {
        if let Self::Parted { exact_record, .. } = &mut self {
            *exact_record = value;
        }
        self
    }
}

/// Whether the engine as it was takes the rest of the record's inputs with
/// every recorded hash.
fn exact_to_the_end(
    then: &mut Engine<RegistryLookup>,
    lines: impl Iterator<Item = serde_json::Result<serde_json::Value>>,
) -> bool {
    for line in lines {
        let Ok(line) = line else {
            return false;
        };
        if line["kind"] != "input" {
            continue;
        }
        let (Some(seat), Ok(action)) = (
            line["seat"].as_u64(),
            serde_json::from_value::<PlayerAction>(line["action"].clone()),
        ) else {
            return false;
        };
        let seat = PlayerId::new(u8::try_from(seat).expect("a seat"));
        if as_before(|| then.apply(seat, action)).is_err()
            || line["hash"].as_str() != Some(hex(then.snapshot_hash()).as_str())
        {
            return false;
        }
    }
    true
}

fn kind(pending: &Pending) -> String {
    let debug = format!("{pending:?}");
    debug
        .split([' ', '(', '{'])
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn record_of(game: &Game) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(game.run.join("records").join(&game.shard))?;
    file.seek(SeekFrom::Start(game.offset))?;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(file.take(game.len)).read_to_end(&mut out)?;
    Ok(out)
}

fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

#[allow(clippy::too_many_lines)] // One record, read and played twice, in order.
fn lockstep(record: &[u8]) -> Found {
    // A trailing line without its newline is a record cut off mid-write.
    let mut lines = record
        .split_inclusive(|b| *b == b'\n')
        .filter(|line| line.ends_with(b"\n"))
        .map(serde_json::from_slice::<serde_json::Value>);
    let Some(Ok(header)) = lines.next() else {
        return Found::Unreadable("no header".into());
    };
    if header["kind"] != "header" || header["record"].as_u64() != Some(RECORD_VERSION) {
        return Found::Unreadable(format!("header {}", header["record"]));
    }
    let preset: baylee_core::preset::GamePreset =
        match serde_json::from_value(header["preset"].clone()) {
            Ok(preset) => preset,
            Err(e) => return Found::Unreadable(format!("preset: {e}")),
        };
    let (Ok(mut now), Ok(mut then)) = (
        Engine::new(&preset, RegistryLookup),
        as_before(|| Engine::new(&preset, RegistryLookup)),
    ) else {
        return Found::Unreadable("unbuildable".into());
    };
    // The answers both took, so a parted record's question before the answer
    // they parted on can be asked again of the engine as it was.
    let mut taken: Vec<(PlayerId, PlayerAction)> = Vec::new();
    let parted = |n,
                  why,
                  exact_before,
                  taken: &[(PlayerId, PlayerAction)],
                  now: &Engine<RegistryLookup>,
                  then: &Engine<RegistryLookup>| {
        let asked = as_before(|| {
            let mut again = Engine::new(&preset, RegistryLookup).expect("built once");
            for (seat, action) in taken {
                again.apply(*seat, action.clone()).expect("taken once");
            }
            kind(again.pending())
        });
        Found::Parted {
            n,
            why,
            exact_before,
            exact_record: false,
            turn: now.state().turn.number,
            step: format!("{:?}", now.state().turn.step),
            asked,
            then: kind(then.pending()),
            now: kind(now.pending()),
        }
    };
    let mut exact = header["hash"].as_str() == Some(hex(then.snapshot_hash()).as_str());
    if now.snapshot_hash() != then.snapshot_hash() {
        let found = parted(None, "opening hash", exact, &taken, &now, &then);
        return found.exact_record(exact && exact_to_the_end(&mut then, lines));
    }
    let mut inputs = 0;
    while let Some(line) = lines.next() {
        let Ok(line) = line else {
            return Found::Unreadable(format!("line after input {inputs}"));
        };
        if line["kind"] != "input" {
            continue;
        }
        let (Some(n), Some(seat)) = (line["n"].as_u64(), line["seat"].as_u64()) else {
            return Found::Unreadable("input without n or seat".into());
        };
        let action: PlayerAction = match serde_json::from_value(line["action"].clone()) {
            Ok(action) => action,
            Err(e) => return Found::Unreadable(format!("action at {n}: {e}")),
        };
        let seat = PlayerId::new(u8::try_from(seat).expect("a seat"));
        let taken_now = now.apply(seat, action.clone()).is_ok();
        let taken_then = as_before(|| then.apply(seat, action.clone())).is_ok();
        let why = match (taken_now, taken_then) {
            (false, false) => {
                return Found::BothRefused {
                    n,
                    exact_before: exact,
                };
            }
            (true, false) => Some("refused as it was"),
            (false, true) => Some("refused as it is"),
            (true, true) => None,
        };
        if let Some(why) = why {
            let found = parted(Some(n), why, exact, &taken, &now, &then);
            // Refused as it was: the engine as it was cannot go on either.
            let went_on = taken_then
                && line["hash"].as_str() == Some(hex(then.snapshot_hash()).as_str())
                && exact_to_the_end(&mut then, &mut lines);
            return found.exact_record(exact && went_on);
        }
        exact &= line["hash"].as_str() == Some(hex(then.snapshot_hash()).as_str());
        if now.snapshot_hash() != then.snapshot_hash() {
            let found = parted(Some(n), "hash", exact, &taken, &now, &then);
            return found.exact_record(exact && exact_to_the_end(&mut then, &mut lines));
        }
        taken.push((seat, action));
        inputs += 1;
    }
    Found::Alike { exact, inputs }
}

/// Every run directory under `root`, in path order: a `games.jsonl` with a
/// `records/` beside it. A symbolic link is not followed.
fn runs(root: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    entries.sort();
    if root.join("games.jsonl").is_file() && root.join("records").is_dir() {
        found.push(root.to_path_buf());
    }
    for path in entries {
        if path.is_dir() && !path.is_symlink() {
            runs(&path, found);
        }
    }
}

fn games_of(run: &Path) -> Vec<Game> {
    let Ok(index) = File::open(run.join("games.jsonl")) else {
        return Vec::new();
    };
    BufReader::new(index)
        .lines()
        .map_while(Result::ok)
        .enumerate()
        .filter_map(|(line, text)| {
            let game: serde_json::Value = serde_json::from_str(&text).ok()?;
            Some(Game {
                run: run.to_path_buf(),
                line,
                shard: game["shard"].as_str()?.to_owned(),
                offset: game["offset"].as_u64()?,
                len: game["len"].as_u64()?,
            })
        })
        .collect()
}

#[test]
#[ignore = "reads recorded runs from BAYLEE_LOCKSTEP_ROOTS"]
fn recorded_games_in_lockstep() {
    let roots = std::env::var("BAYLEE_LOCKSTEP_ROOTS").expect("BAYLEE_LOCKSTEP_ROOTS");
    let out = PathBuf::from(std::env::var("BAYLEE_LOCKSTEP_OUT").expect("BAYLEE_LOCKSTEP_OUT"));
    let threads = std::env::var("BAYLEE_LOCKSTEP_THREADS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(4_usize);
    let mut found = Vec::new();
    for root in roots.split(':').filter(|r| !r.is_empty()) {
        runs(Path::new(root), &mut found);
    }
    let mut games: Vec<Game> = found.iter().flat_map(|run| games_of(run)).collect();
    // `BAYLEE_LOCKSTEP_ONLY`: a report this wrote; only the records it
    // lists as parted are played again.
    if let Ok(only) = std::env::var("BAYLEE_LOCKSTEP_ONLY") {
        let report = std::fs::read_to_string(&only).expect("BAYLEE_LOCKSTEP_ONLY");
        let listed: std::collections::BTreeSet<(PathBuf, usize)> = report
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|line| line["found"].get("parted").is_some())
            .filter_map(|line| {
                let at = usize::try_from(line["line"].as_u64()?).ok()?;
                Some((PathBuf::from(line["run"].as_str()?), at))
            })
            .collect();
        games.retain(|game| listed.contains(&(game.run.clone(), game.line)));
    }
    eprintln!("{} runs, {} games", found.len(), games.len());
    let mut file = File::create_new(&out).expect("BAYLEE_LOCKSTEP_OUT must not exist");
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, Found)>> = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some(game) = games.get(at) else {
                        return;
                    };
                    let found = match record_of(game) {
                        Ok(record) => lockstep(&record),
                        Err(e) => Found::Unreadable(e.to_string()),
                    };
                    if at.is_multiple_of(1000) {
                        eprintln!("{at} of {}", games.len());
                    }
                    results.lock().expect("no panic holds it").push((at, found));
                }
            });
        }
    });
    let mut results = results.into_inner().expect("no panic held it");
    results.sort_by_key(|(at, _)| *at);
    // Per outcome: games, and of those the ones the engine as it was
    // replayed with every recorded hash up to where the outcome was found.
    let mut tally = std::collections::BTreeMap::<&str, (usize, usize)>::new();
    let mut inputs_alike = 0;
    for (at, found) in &results {
        let game = &games[*at];
        let (what, exact) = match found {
            Found::Alike { exact, inputs } => {
                inputs_alike += inputs;
                ("alike", *exact)
            }
            Found::BothRefused { exact_before, .. } => ("both_refused", *exact_before),
            Found::Unreadable(_) => ("unreadable", false),
            Found::Parted { exact_before, .. } => ("parted", *exact_before),
        };
        let entry = tally.entry(what).or_default();
        entry.0 += 1;
        entry.1 += usize::from(exact);
        if what != "alike" {
            writeln!(
                file,
                "{}",
                serde_json::json!({
                    "run": game.run,
                    "line": game.line,
                    "found": found.json(),
                })
            )
            .expect("written");
        }
    }
    let summary = serde_json::json!({
        "runs": found.len(),
        "games": results.len(),
        "inputs_alike": inputs_alike,
        "games_and_exact_before": tally,
    });
    writeln!(file, "{summary}").expect("written");
    eprintln!("{summary}");
}
