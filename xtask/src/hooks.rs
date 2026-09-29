//! L4's firing part and L5, read from the engine's test hooks
//! (`docs/verification-hooks.md`, which is normative for everything read
//! here).
//!
//! - **Firing** (`--ability-log <dir>`): a run of the engine's tests with
//!   `BAYLEE_ABILITY_LOG=<dir>` writes, per test, every `(card, index, kind)`
//!   that fired, and `pool-inventory.json`, every card's ability entries. A
//!   card passes when each entry a door logs fired in some test. An entry no
//!   door logs (`kind: null`: ward, toxic, echo, suspend, prepared) passes
//!   where the mechanics report ([`crate::mechanics`]) shows the engine's
//!   rule tests run that variant, and blocks otherwise.
//! - **Mutation** (`--mutate`): for each card at L4, each ability entry but
//!   the CR 305.6 land mana is replaced by nothing (`BAYLEE_MUTATE`) and the
//!   tests that fired the card are run again; the card is L5 when every such
//!   mutant is killed.
//! - **Leaving** (`--leave-log <dir>`): the leave probe's `leave.jsonl` has a
//!   verdict per card and way off the battlefield (destroy, exile, bounce,
//!   …). A card leaves clean when it has lines and every one is `ok`;
//!   `skipped` is not known, so not clean.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};
use baylee_core::ids::CardIndex;

use crate::mechanics::{Analysis, Status};

/// `AbilityRef::SPELL`: a spell ability's index in the log.
const SPELL: u32 = u32::MAX;

/// How long one mutant's tests may run; a hang is counted as a kill (the
/// mutant changed what the tests see) and said so.
const MUTANT_WALL: Duration = Duration::from_secs(300);

/// A test that fires more cards than this is a sweep over the pool (every
/// offered ability, every land's mana): it runs for tens of seconds, so a
/// mutant meets the sweeps only once the card's own tests let it survive.
const SWEEP: usize = 50;

/// One ability entry of a card, as the inventory lists it.
#[derive(Clone, Debug)]
pub struct Entry {
    /// What the recorder logs: the position, or [`SPELL`].
    pub index: u32,
    /// The position in the list.
    pub position: u32,
    /// The `AbilityDef` variant.
    pub variant: String,
    /// The kind the recorder logs it as; `None` where no door logs it.
    pub kind: Option<String>,
    /// The mana ability CR 305.6 gives a land for its basic land types.
    pub intrinsic: bool,
    /// The faces whose list holds it.
    pub faces: Vec<u8>,
}

/// One card of the inventory.
#[derive(Clone, Debug)]
pub struct Card {
    /// A Room, which numbers its abilities by its unlocked doors.
    pub room: bool,
    /// Its entries.
    pub entries: Vec<Entry>,
}

impl Card {
    /// A Room's left half's length: a right-half entry fired with both
    /// doors open is logged this far past its position.
    fn left_len(&self) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.faces.contains(&0) && e.index != SPELL)
            .map(|e| e.position + 1)
            .max()
            .unwrap_or(0)
    }
}

/// What a run of the engine's tests with the firing recorder wrote.
pub struct Firing {
    /// The inventory, by card.
    pub inventory: BTreeMap<CardIndex, Card>,
    /// Per `(card, index, kind)` that fired, the tests that fired it.
    pub fired: BTreeMap<(CardIndex, u32, String), BTreeSet<String>>,
    /// Per card, every test that fired one of its abilities.
    pub tests_of: BTreeMap<CardIndex, BTreeSet<String>>,
    /// Per test, the cards it fired.
    pub cards_of: BTreeMap<String, BTreeSet<CardIndex>>,
    /// Lines naming no inventory entry, even after a Room's mapping.
    pub outside: BTreeSet<(CardIndex, u32, String)>,
    /// Test files read.
    pub files: usize,
}

fn u32_of(v: &serde_json::Value) -> Option<u32> {
    v.as_u64().and_then(|n| u32::try_from(n).ok())
}

impl Firing {
    /// Reads a recorder's directory.
    ///
    /// # Errors
    /// When the directory or its inventory cannot be read.
    pub fn read(dir: &Path) -> anyhow::Result<Self> {
        let inventory_path = dir.join("pool-inventory.json");
        let text = fs::read_to_string(&inventory_path).with_context(|| {
            format!(
                "reading {} (a run that names `engine::verification_tests::pool_inventory` writes it)",
                inventory_path.display()
            )
        })?;
        let rows: serde_json::Value = serde_json::from_str(&text)?;
        let mut inventory = BTreeMap::new();
        for row in rows.as_array().context("the inventory is a JSON array")? {
            let card = CardIndex::new(u32_of(&row["card"]).context("an inventory row's card")?);
            let entries = row["abilities"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|a| {
                    Ok(Entry {
                        index: u32_of(&a["index"]).context("an entry's index")?,
                        position: u32_of(&a["position"]).context("an entry's position")?,
                        variant: a["variant"].as_str().unwrap_or("?").to_owned(),
                        kind: a["kind"].as_str().map(str::to_owned),
                        intrinsic: a["intrinsic"].as_bool().unwrap_or(false),
                        faces: a["faces"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|f| f.as_u64().and_then(|f| u8::try_from(f).ok()))
                            .collect(),
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            inventory.insert(
                card,
                Card {
                    room: row["room"].as_bool().unwrap_or(false),
                    entries,
                },
            );
        }

        let mut firing = Self {
            inventory,
            fired: BTreeMap::new(),
            tests_of: BTreeMap::new(),
            cards_of: BTreeMap::new(),
            outside: BTreeSet::new(),
            files: 0,
        };
        let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
            .collect();
        paths.sort();
        for path in paths {
            firing.files += 1;
            for line in fs::read_to_string(&path)?.lines().filter(|l| !l.is_empty()) {
                let v: serde_json::Value = serde_json::from_str(line)
                    .with_context(|| format!("a line of {}", path.display()))?;
                let (Some(test), Some(card), Some(index), Some(kind)) = (
                    v["test"].as_str(),
                    u32_of(&v["card"]),
                    u32_of(&v["index"]),
                    v["kind"].as_str(),
                ) else {
                    bail!(
                        "a line of {} is not {{test, card, index, kind}}: {line}",
                        path.display()
                    );
                };
                firing.credit(test, CardIndex::new(card), index, kind);
            }
        }
        Ok(firing)
    }

    fn credit(&mut self, test: &str, card: CardIndex, index: u32, kind: &str) {
        let known = |c: &Card, i: u32| {
            c.entries
                .iter()
                .any(|e| e.index == i && e.kind.as_deref() == Some(kind))
        };
        let entry = match self.inventory.get(&card) {
            Some(c) if known(c, index) => Some(index),
            // A Room's right half with both doors open is logged past the
            // left half.
            Some(c) if c.room && index >= c.left_len() && known(c, index - c.left_len()) => {
                Some(index - c.left_len())
            }
            _ => None,
        };
        self.tests_of
            .entry(card)
            .or_default()
            .insert(test.to_owned());
        self.cards_of
            .entry(test.to_owned())
            .or_default()
            .insert(card);
        match entry {
            Some(i) => {
                self.fired
                    .entry((card, i, kind.to_owned()))
                    .or_default()
                    .insert(test.to_owned());
            }
            None => {
                self.outside.insert((card, index, kind.to_owned()));
            }
        }
    }

    /// Why `card`'s firing evidence falls short of L4, if it does.
    pub fn gap(&self, card: CardIndex, mechanics: Option<&Analysis>) -> Option<String> {
        let Some(c) = self.inventory.get(&card) else {
            return Some("not in the inventory".into());
        };
        let mut unfired = Vec::new();
        let mut unlogged = Vec::new();
        for e in &c.entries {
            if let Some(kind) = &e.kind {
                if !self.fired.contains_key(&(card, e.index, kind.clone())) {
                    unfired.push(format!("{} {}", e.position, e.variant));
                }
            } else {
                let ran = mechanics
                    .and_then(|m| m.status_of("AbilityDef", &e.variant))
                    .is_some_and(|s| matches!(s, Status::Tested | Status::Partly));
                if !ran {
                    unlogged.push(e.variant.clone());
                }
            }
        }
        if !unfired.is_empty() {
            return Some(format!("abilities no test fired: {}", unfired.join(", ")));
        }
        if !unlogged.is_empty() {
            return Some(format!(
                "no door logs {} and the rule tests do not run it{}",
                unlogged.join(", "),
                if mechanics.is_none() {
                    " (no --coverage)"
                } else {
                    ""
                }
            ));
        }
        None
    }
}

/// What the leave probe wrote (`BAYLEE_LEAVE_LOG`): per card, each route
/// and its verdict.
pub struct Leave {
    /// Per card: (route, verdict, detail).
    pub routes: BTreeMap<CardIndex, Vec<(String, String, String)>>,
}

impl Leave {
    /// Reads `<dir>/leave.jsonl`.
    ///
    /// # Errors
    /// When the file cannot be read or a line is not a probe line.
    pub fn read(dir: &Path) -> anyhow::Result<Self> {
        let path = dir.join("leave.jsonl");
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let mut routes: BTreeMap<CardIndex, Vec<(String, String, String)>> = BTreeMap::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let v: serde_json::Value = serde_json::from_str(line)
                .with_context(|| format!("a line of {}", path.display()))?;
            let (Some(card), Some(route), Some(verdict)) = (
                u32_of(&v["card"]),
                v["route"].as_str(),
                v["verdict"].as_str(),
            ) else {
                bail!(
                    "a line of {} is not {{card, route, verdict, detail}}: {line}",
                    path.display()
                );
            };
            routes.entry(CardIndex::new(card)).or_default().push((
                route.to_owned(),
                verdict.to_owned(),
                v["detail"].as_str().unwrap_or("").to_owned(),
            ));
        }
        Ok(Self { routes })
    }

    /// Why `card` is not known to leave the battlefield clean, if it is not.
    /// A card that can never be a permanent (an instant, a sorcery) has
    /// nothing to leave, and the probe rightly does not try it.
    pub fn gap(&self, card: CardIndex, permanent: bool) -> Option<String> {
        let Some(routes) = self.routes.get(&card) else {
            return permanent.then(|| "the leave probe did not try it".into());
        };
        let bad: Vec<String> = routes
            .iter()
            .filter(|(_, verdict, _)| verdict != "ok")
            .map(|(route, verdict, detail)| {
                if detail.is_empty() {
                    format!("{route} {verdict}")
                } else {
                    format!("{route} {verdict} ({detail})")
                }
            })
            .collect();
        (!bad.is_empty()).then(|| format!("does not leave clean: {}", bad.join(", ")))
    }
}

/// What a mutant's tests did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// A test failed.
    Killed,
    /// The tests did not finish in [`MUTANT_WALL`]: counted as killed.
    Hung,
    /// Every test passed.
    Survived,
    /// No test ran (a filter that matched nothing).
    NoTests,
    /// Not a mutant (exit 3), or the run failed otherwise: why.
    Invalid(String),
}

impl Verdict {
    /// Whether the mutant was killed.
    pub fn killed(&self) -> bool {
        matches!(self, Self::Killed | Self::Hung)
    }

    /// Its name in the report.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Killed => "killed",
            Self::Hung => "hung",
            Self::Survived => "survived",
            Self::NoTests => "no_tests",
            Self::Invalid(_) => "invalid",
        }
    }
}

/// The engine's library test binary, built if it is not.
fn engine_test_binary(root: &Path) -> anyhow::Result<PathBuf> {
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args([
            "test",
            "-p",
            "baylee-engine",
            "--lib",
            "--no-run",
            "--message-format=json",
        ])
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .context("building the engine's test binary")?;
    if !output.status.success() {
        bail!("the engine's test binary did not build");
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v["reason"] == "compiler-artifact"
            && v["target"]["name"] == "baylee_engine"
            && v["profile"]["test"] == true
            && let Some(exe) = v["executable"].as_str()
        {
            return Ok(PathBuf::from(exe));
        }
    }
    bail!("cargo named no test binary for baylee-engine")
}

/// Runs one mutant: `card`'s ability `index` replaced by nothing, `tests`
/// run by `exe` from the engine's directory; its output kept in `out`.
fn run_mutant(
    exe: &Path,
    engine_dir: &Path,
    card: CardIndex,
    index: u32,
    tests: &BTreeSet<String>,
    out: &Path,
) -> anyhow::Result<Verdict> {
    let stdout = out.with_extension("out");
    let stderr = out.with_extension("err");
    let mut child = Command::new(exe)
        .current_dir(engine_dir)
        .env("BAYLEE_MUTATE", format!("{}:{index}", card.get()))
        .env_remove("BAYLEE_ABILITY_LOG")
        .arg("--exact")
        .args(tests)
        .arg("--test-threads=1")
        .stdout(File::create(&stdout)?)
        .stderr(File::create(&stderr)?)
        .spawn()
        .context("starting the engine's test binary")?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if started.elapsed() > MUTANT_WALL {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let Some(status) = status else {
        return Ok(Verdict::Hung);
    };
    Ok(match status.code() {
        Some(101) => Verdict::Killed,
        Some(0) => {
            let text = fs::read_to_string(&stdout).unwrap_or_default();
            let passed = text
                .lines()
                .filter_map(|l| l.strip_prefix("test result: ok. "))
                .find_map(|l| l.split_whitespace().next()?.parse::<u32>().ok())
                .unwrap_or(0);
            if passed > 0 {
                Verdict::Survived
            } else {
                Verdict::NoTests
            }
        }
        code => {
            let err = fs::read_to_string(&stderr).unwrap_or_default();
            let last = err
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("");
            Verdict::Invalid(format!("exit {code:?}: {last}"))
        }
    })
}

/// A mutant: the card, the ability, the card's own tests and the sweeps
/// that fired it.
type Job = (CardIndex, u32, BTreeSet<String>, BTreeSet<String>);

/// A mutant judged by the card's own tests first and by the sweeps (see
/// [`SWEEP`]) only where those let it survive.
fn judge(
    exe: &Path,
    engine_dir: &Path,
    (card, index): (CardIndex, u32),
    own: &BTreeSet<String>,
    sweeps: &BTreeSet<String>,
    out: &Path,
) -> anyhow::Result<Verdict> {
    let first = if own.is_empty() {
        Verdict::NoTests
    } else {
        run_mutant(exe, engine_dir, card, index, own, out)?
    };
    if first.killed() || sweeps.is_empty() || matches!(first, Verdict::Invalid(_)) {
        return Ok(first);
    }
    let swept = run_mutant(
        exe,
        engine_dir,
        card,
        index,
        sweeps,
        &out.with_file_name(format!(
            "{}-sweeps",
            out.file_name().and_then(|n| n.to_str()).unwrap_or("mutant")
        )),
    )?;
    Ok(match (first, swept) {
        (_, v) if v.killed() => v,
        (Verdict::Survived, _) | (_, Verdict::Survived) => Verdict::Survived,
        (_, v) => v,
    })
}

/// Every mutant of every card in `cards`: each distinct index of an entry
/// but the intrinsic land mana, run against the tests that fired the card,
/// on `threads` threads (0 = one per core).
///
/// # Errors
/// When the test binary cannot be built or a mutant not started.
pub fn mutate(
    root: &Path,
    firing: &Firing,
    cards: &BTreeSet<CardIndex>,
    threads: usize,
    work: &Path,
) -> anyhow::Result<BTreeMap<(CardIndex, u32), Verdict>> {
    let threads = if threads == 0 {
        std::thread::available_parallelism().map_or(4, usize::from)
    } else {
        threads
    };
    let exe = engine_test_binary(root)?;
    let engine_dir = root.join("crates/baylee-engine");
    fs::create_dir_all(work)?;
    let mut jobs: Vec<Job> = Vec::new();
    for card in cards {
        let (Some(c), Some(tests)) = (firing.inventory.get(card), firing.tests_of.get(card)) else {
            continue;
        };
        let (sweeps, own): (BTreeSet<String>, BTreeSet<String>) = tests
            .iter()
            .cloned()
            .partition(|t| firing.cards_of.get(t).is_some_and(|c| c.len() > SWEEP));
        let indices: BTreeSet<u32> = c
            .entries
            .iter()
            .filter(|e| !e.intrinsic)
            .map(|e| e.index)
            .collect();
        jobs.extend(
            indices
                .into_iter()
                .map(|i| (*card, i, own.clone(), sweeps.clone())),
        );
    }
    // The slowest first: a card named by many tests.
    jobs.sort_by_key(|(_, _, own, _)| std::cmp::Reverse(own.len()));
    println!(
        "L5: {} mutants of {} cards on {threads} threads",
        jobs.len(),
        cards.len()
    );
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let verdicts = Mutex::new(BTreeMap::new());
    let failure: Mutex<Option<anyhow::Error>> = Mutex::new(None);
    let started = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                loop {
                    let n = next.fetch_add(1, Ordering::Relaxed);
                    let Some((card, index, own, sweeps)) = jobs.get(n) else {
                        break;
                    };
                    let out = work.join(format!("{}-{index}", card.get()));
                    match judge(&exe, &engine_dir, (*card, *index), own, sweeps, &out) {
                        Ok(v) => {
                            verdicts
                                .lock()
                                .expect("verdicts")
                                .insert((*card, *index), v);
                        }
                        Err(e) => {
                            failure.lock().expect("failure").get_or_insert(e);
                            break;
                        }
                    }
                    let finished = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if finished.is_multiple_of(250) {
                        println!(
                            "L5: {finished}/{} mutants, {:.0} s",
                            jobs.len(),
                            started.elapsed().as_secs_f64()
                        );
                    }
                }
            });
        }
    });
    if let Some(e) = failure.into_inner().expect("failure") {
        return Err(e);
    }
    Ok(verdicts.into_inner().expect("verdicts"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: u32, position: u32, kind: Option<&str>, faces: &[u8]) -> Entry {
        Entry {
            index,
            position,
            variant: "Triggered".into(),
            kind: kind.map(str::to_owned),
            intrinsic: false,
            faces: faces.to_vec(),
        }
    }

    fn firing(inventory: BTreeMap<CardIndex, Card>) -> Firing {
        Firing {
            inventory,
            fired: BTreeMap::new(),
            tests_of: BTreeMap::new(),
            cards_of: BTreeMap::new(),
            outside: BTreeSet::new(),
            files: 0,
        }
    }

    #[test]
    fn a_card_passes_when_every_logged_entry_fired() {
        let card = CardIndex::new(7);
        let mut f = firing(BTreeMap::from([(
            card,
            Card {
                room: false,
                entries: vec![
                    entry(0, 0, Some("triggered"), &[0]),
                    entry(SPELL, 1, Some("spell"), &[0]),
                ],
            },
        )]));
        f.credit("t1", card, 0, "triggered");
        assert!(f.gap(card, None).is_some_and(|g| g.contains("1 Triggered")));
        f.credit("t2", card, SPELL, "spell");
        assert_eq!(f.gap(card, None), None);
        assert_eq!(f.tests_of[&card].len(), 2);
        // A line naming no entry is kept apart, not credited.
        f.credit("t3", card, 5, "static");
        assert!(f.outside.contains(&(card, 5, "static".into())));
    }

    #[test]
    fn an_entry_no_door_logs_needs_the_rule_tests() {
        let card = CardIndex::new(8);
        let f = firing(BTreeMap::from([(
            card,
            Card {
                room: false,
                entries: vec![entry(0, 0, None, &[0])],
            },
        )]));
        assert!(
            f.gap(card, None)
                .is_some_and(|g| g.contains("no --coverage"))
        );
    }

    #[test]
    fn a_card_leaves_clean_only_when_every_route_is_ok() {
        let dir = std::env::temp_dir().join(format!("leave-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let lines = [
            r#"{"card":1,"route":"destroy","verdict":"ok","detail":""}"#,
            r#"{"card":1,"route":"exile","verdict":"ok","detail":""}"#,
            r#"{"card":2,"route":"destroy","verdict":"ok","detail":""}"#,
            r#"{"card":2,"route":"bounce","verdict":"lingers","detail":"its anthem stays"}"#,
            r#"{"card":3,"route":"phase_out","verdict":"skipped","detail":"no way to try"}"#,
        ];
        fs::write(dir.join("leave.jsonl"), lines.join("\n")).unwrap();
        let leave = Leave::read(&dir).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(leave.gap(CardIndex::new(1), true), None);
        assert!(
            leave
                .gap(CardIndex::new(2), true)
                .is_some_and(|g| g.contains("bounce lingers (its anthem stays)"))
        );
        assert!(
            leave.gap(CardIndex::new(3), true).is_some(),
            "skipped is not known"
        );
        assert!(leave.gap(CardIndex::new(4), true).is_some(), "not probed");
        assert_eq!(
            leave.gap(CardIndex::new(4), false),
            None,
            "an instant has nothing to leave"
        );
    }

    #[test]
    fn a_rooms_right_half_with_both_doors_open_maps_back() {
        let card = CardIndex::new(9);
        let mut f = firing(BTreeMap::from([(
            card,
            Card {
                room: true,
                entries: vec![
                    entry(0, 0, Some("triggered"), &[0]),
                    entry(1, 1, Some("static"), &[0]),
                    entry(0, 0, Some("activated"), &[1]),
                ],
            },
        )]));
        // Left half: two entries; the right half's 0 is logged as 2.
        f.credit("t", card, 2, "activated");
        assert!(f.fired.contains_key(&(card, 0, "activated".into())));
        assert!(f.outside.is_empty());
    }
}
