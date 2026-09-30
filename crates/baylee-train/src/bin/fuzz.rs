//! Plays the engine against itself with answers picked at random among the
//! ones each question offers, and reports every broken invariant.
//!
//! ```text
//! cargo run --profile selfplay -p baylee-train --bin fuzz -- \
//!     --games 5000 --out ~/baylee-data/fuzz/f001
//! ```
//!
//! Each game deals two decks generated from the working pool
//! ([`baylee_train::deckgen`], every shape) and takes both chairs over
//! (`Session::take_over`). A question is answered at random `--chaos` of the
//! time, and by the house the rest, so games still reach the boards real
//! play reaches. A random answer picks uniformly, pick by pick, among what
//! [`baylee_train::policy::options`] says the question offers. What is
//! checked, each broken one a finding:
//!
//! - `panic`: the engine panicked.
//! - `refused`: the engine refused an answer made only of offered options
//!   (the question offered what it does not take).
//! - `stated-<fault>`: a refused random answer the question's own check
//!   (`Pending::answer_fault`) faults: refused as stated, counted apart.
//! - `refusal-changed-state`: a refused answer changed the game anyway
//!   (`Session::snapshot_hash` before and after).
//! - `house-refused`: the house's own answer was refused, or it had none.
//! - `no-options`: a question offered nothing to pick.
//! - `unassembled`: random picks this crate could not make an answer of (a
//!   bug here, not in the engine).
//! - `replay`: the game's record does not replay to the same hashes.
//! - `answer-cap`, `time-cap`: the game did not end (a loop, or an engine
//!   that got slow).
//!
//! A game's decks and every random pick follow from its number and the
//! run's `--name`, so `--only <n>` with the same arguments plays a finding's
//! game again, move for move. Writes `run.json`, `games.jsonl`,
//! `findings.jsonl`, and the record of every game with a finding under
//! `records/`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write as _};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, bail};
use baylee_cards::formats::Shape;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_train::deckgen::{self, Archetype, Rng, shape_name};
use baylee_train::features::{PENDING_KINDS, question};
use baylee_train::housedeck::HouseDeck;
use baylee_train::policy::{self, Picked, Unscored};
use baylee_train::selfplay::table;
use baylee_train::working::{Working, repo_root};
use clap::Parser;
use serde_json::{Value, json};

#[derive(Parser, Debug)]
#[command(about = "Plays the engine with random answers and reports broken invariants")]
struct Args {
    /// Games to play.
    #[arg(long, default_value_t = 1000)]
    games: u64,
    /// The first game's seed; game `i` is dealt with `first_seed + i`.
    #[arg(long, default_value_t = 1)]
    first_seed: u64,
    /// Threads; 0 = one per core.
    #[arg(long, default_value_t = 0)]
    threads: usize,
    /// How much of a chair's answering is random, the house answering the
    /// rest.
    #[arg(long, default_value_t = 0.5)]
    chaos: f64,
    /// Question kinds (`features::PENDING_KINDS`) always left to the house:
    /// one with an open finding would otherwise bury every other.
    #[arg(long, value_delimiter = ',')]
    house_kinds: Vec<String>,
    /// Decks generated per shape.
    #[arg(long, default_value_t = 40)]
    decks_per_shape: usize,
    /// Shapes played.
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "constructed,highlander,commander"
    )]
    shapes: Vec<String>,
    /// Seed of the deck generator.
    #[arg(long, default_value_t = 1)]
    deck_seed: u64,
    /// Answers a game may take before it is stopped.
    #[arg(long, default_value_t = 20_000)]
    max_answers: u64,
    /// Seconds a game may take before it is stopped.
    #[arg(long, default_value_t = 60)]
    max_secs: u64,
    /// Play only these game numbers instead of `0..games`.
    #[arg(long, value_delimiter = ',')]
    only: Vec<u64>,
    /// The run's name, which the random picks are seeded from; defaults to
    /// the name of `--out`.
    #[arg(long)]
    name: Option<String>,
    /// Where the run is written.
    #[arg(long)]
    out: PathBuf,
}

/// `SplitMix64`'s finaliser: one number from another, spread.
const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a of a name: the same on every machine.
fn name_hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// One game's findings and what it did.
struct Game {
    line: Value,
    findings: Vec<Value>,
    record: Vec<u8>,
}

/// What a finding says about the question it was found at.
fn at_question(session: &Session, pending: &Pending, hand: usize) -> Value {
    json!({
        "decision": session.decision_seq(),
        "turn": session.state().turn.number,
        "kind": PENDING_KINDS[usize::try_from(question(pending, hand).0).unwrap_or(0)],
        "pending": format!("{pending:?}").chars().take(800).collect::<String>(),
    })
}

/// The names of the objects `text` (an answer's `Debug` form) mentions, as
/// the seat's view names them: a finding has to say which card it is about.
fn named(view: &baylee_view::PlayerView, text: &str) -> Vec<String> {
    let mut names: BTreeMap<String, &str> = BTreeMap::new();
    for h in &view.hand {
        names.insert(format!("{:?}", h.id), &h.name);
    }
    let zones = [
        &view.battlefield,
        &view.stack,
        &view.looking_at,
        &view.library_tops,
    ];
    let piles = view
        .graveyards
        .iter()
        .chain(&view.exile)
        .chain(&view.command);
    for o in zones.into_iter().chain(piles).flatten() {
        names.insert(format!("{:?}", o.id), &o.name);
    }
    let mut out = Vec::new();
    for (at, _) in text.match_indices("ObjectId(") {
        let Some(end) = text[at..].find(')') else {
            continue;
        };
        let id = &text[at..=at + end];
        let name = names.get(id).copied().unwrap_or("?");
        let entry = format!("{id} {name}");
        if !out.contains(&entry) {
            out.push(entry);
        }
    }
    out
}

/// A random answer to `pending`: uniform picks among the offered options
/// until the answer is finished. `Ok(None)` where this crate does not answer
/// the question kind (the house does).
fn random_answer(
    pending: &Pending,
    hand: &[ObjectId],
    rng: &mut Rng,
) -> Result<Option<PlayerAction>, &'static str> {
    let mut picked = Picked::default();
    let mut picks = Vec::new();
    loop {
        let mut options = match policy::options(pending, hand, &picked) {
            Ok(o) => o,
            Err(Unscored::Unsupported | Unscored::Over) => return Ok(None),
        };
        // "Done" only where the question's own check passes the answer,
        // unless nothing else is left: then it is refused as stated.
        if options.len() > 1 && !policy::done_allowed(pending, &picked) {
            options.retain(|c| *c != policy::Choice::Fixed(policy::fixed::DONE));
        }
        if options.is_empty() {
            return Err("no-options");
        }
        let choice = options[rng.below(options.len())];
        picks.push(choice);
        picked.add(choice);
        if policy::finished(pending, &picked, choice) {
            break;
        }
    }
    policy::assemble(pending, &picks)
        .map(Some)
        .map_err(|_| "unassembled")
}

/// Plays game `i` with both chairs taken over; every broken invariant is a
/// finding.
#[allow(clippy::too_many_lines)] // one game loop and its checks
fn play(
    i: u64,
    args: &Args,
    run: &str,
    decks: &[(Shape, HouseDeck)],
    by_shape: &[Vec<usize>],
) -> Game {
    let seed = args.first_seed + i;
    let mut rng = Rng::new(mix(seed ^ name_hash(run)));
    let class = &by_shape[usize::try_from(i).unwrap_or(0) % by_shape.len()];
    let a = class[rng.below(class.len())];
    let b = class[rng.below(class.len())];
    let (deck_a, deck_b) = (&decks[a].1, &decks[b].1);
    let mut line = json!({
        "game": i,
        "seed": seed,
        "shape": shape_name(decks[a].0),
        "decks": [deck_a.key, deck_b.key],
    });
    let mut findings: Vec<Value> = Vec::new();
    let mut finding = |kind: &str, what: Value| {
        let mut f = json!({"game": i, "seed": seed, "finding": kind});
        if let (Some(f), Value::Object(what)) = (f.as_object_mut(), what) {
            f.extend(what);
        }
        findings.push(f);
    };
    let preset = table(seed, deck_a, deck_b, [AIProfile::default(); 2]);
    let started = Instant::now();
    let wall = Duration::from_secs(args.max_secs);
    let (mut random, mut house, mut refused) = (0_u64, 0_u64, 0_u64);
    let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(mut session) = Session::new_recorded(&preset, baylee_build::short()) else {
            return (json!("unbuildable"), Vec::new());
        };
        session.describe(format!("{run}-{i:07}"), vec!["0".into(), "1".into()]);
        let seats: Vec<PlayerId> = (0..2).map(PlayerId::new).collect();
        for s in &seats {
            session.take_over(*s);
        }
        let mut idle = 0;
        let outcome = loop {
            if let Pending::GameOver(result) = session.pending() {
                let winners = session.winning_seats(*result);
                break match winners.as_slice() {
                    [w] => json!({"won": w.get()}),
                    _ => json!("draw"),
                };
            }
            if session.decision_seq() >= args.max_answers {
                finding("answer-cap", json!({"decision": session.decision_seq()}));
                break json!("answer_cap");
            }
            if started.elapsed() >= wall {
                finding("time-cap", json!({"decision": session.decision_seq()}));
                break json!("time_cap");
            }
            session.pump_at_most(32);
            let mut asked = false;
            for me in &seats {
                let Some((pending, view)) = session.view_for(*me) else {
                    continue;
                };
                asked = true;
                let hand: Vec<ObjectId> = view.hand.iter().map(|h| h.id).collect();
                let mut action = None;
                let kind =
                    PENDING_KINDS[usize::try_from(question(&pending, hand.len()).0).unwrap_or(0)];
                if rng.unit() < args.chaos && !args.house_kinds.iter().any(|k| k == kind) {
                    match random_answer(&pending, &hand, &mut rng) {
                        Ok(a) => action = a,
                        Err(kind) => finding(kind, at_question(&session, &pending, hand.len())),
                    }
                }
                let random_pick = action.is_some();
                let Some(action) = action.or_else(|| session.house_action(*me)) else {
                    finding("house-refused", json!({"why": "no answer"}));
                    return (json!("stuck"), session.take_record());
                };
                if random_pick {
                    random += 1;
                } else {
                    house += 1;
                }
                let before = session.snapshot_hash();
                if let Err(why) = session.act(*me, action.clone()) {
                    refused += 1;
                    let after = session.snapshot_hash();
                    let mut what = at_question(&session, &pending, hand.len());
                    what["answer"] = json!(format!("{action:?}"));
                    what["refusal"] = json!(why);
                    what["random"] = json!(random_pick);
                    what["objects"] = json!(named(&view, &format!("{action:?}")));
                    if after != before {
                        finding("refusal-changed-state", what.clone());
                    }
                    if random_pick {
                        // An answer the question's own check faults is one
                        // the question said it refuses: counted apart. Only
                        // a refusal it passed is an engine defect.
                        let kind = pending
                            .answer_fault(&action)
                            .map_or_else(|| "refused".to_owned(), |f| format!("stated-{f:?}"));
                        finding(&kind, what);
                        let fallback = session.house_action(*me);
                        match fallback.map(|a| session.act(*me, a)) {
                            Some(Ok(_)) => {}
                            Some(Err(why)) => {
                                finding("house-refused", json!({"why": why}));
                                return (json!("stuck"), session.take_record());
                            }
                            None => {
                                finding("house-refused", json!({"why": "no answer"}));
                                return (json!("stuck"), session.take_record());
                            }
                        }
                    } else {
                        finding("house-refused", what);
                        return (json!("stuck"), session.take_record());
                    }
                }
            }
            if asked {
                idle = 0;
            } else {
                idle += 1;
                if idle > 3 {
                    finding(
                        "house-refused",
                        json!({"why": "no seat is asked and the game is not over"}),
                    );
                    break json!("stuck");
                }
            }
        };
        (outcome, session.take_record())
    }));
    let (outcome, record) = match played {
        Ok(played) => played,
        Err(panic) => {
            let message = panic
                .downcast_ref::<&str>()
                .map(ToString::to_string)
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            finding("panic", json!({"message": message}));
            (json!("panicked"), Vec::new())
        }
    };
    if !record.is_empty()
        && let Err(e) = baylee_gamehost::record::replay(&record)
    {
        finding("replay", json!({"error": format!("{e:?}")}));
    }
    line["outcome"] = outcome;
    line["random"] = json!(random);
    line["house"] = json!(house);
    line["refused"] = json!(refused);
    line["ms"] = json!(started.elapsed().as_millis());
    line["findings"] = json!(findings.len());
    Game {
        line,
        findings,
        record,
    }
}

#[allow(clippy::too_many_lines)] // setup, the pool of workers, the summary
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let run = args.name.clone().unwrap_or_else(|| {
        args.out
            .file_name()
            .map_or_else(|| "fuzz".into(), |n| n.to_string_lossy().into_owned())
    });
    let root = repo_root();
    let working = Working::scan(&root).context("reading the engine's test code")?;
    let pool = deckgen::Pool::new(&working, &BTreeSet::new());
    let shapes: Vec<Shape> = args
        .shapes
        .iter()
        .map(|s| match s.as_str() {
            "constructed" => Ok(Shape::Constructed),
            "highlander" => Ok(Shape::Highlander),
            "commander" => Ok(Shape::Commander),
            other => Err(anyhow::anyhow!("unknown shape {other}")),
        })
        .collect::<anyhow::Result<_>>()?;
    let mut decks: Vec<(Shape, HouseDeck)> = Vec::new();
    let mut by_shape: Vec<Vec<usize>> = Vec::new();
    let mut deck_json = Vec::new();
    for shape in &shapes {
        let mut members = Vec::new();
        let mut attempt = 0_u64;
        while members.len() < args.decks_per_shape && attempt < args.decks_per_shape as u64 * 4 {
            let archetype = Archetype::ALL[members.len() % Archetype::ALL.len()];
            let seed = args
                .deck_seed
                .wrapping_mul(1_000_003)
                .wrapping_add(attempt)
                .wrapping_add(name_hash(shape_name(*shape)));
            attempt += 1;
            let Ok(g) = deckgen::generate(&pool, *shape, archetype, seed) else {
                continue;
            };
            let key = format!("{}{:03}", shape_name(*shape), members.len());
            deck_json.push(json!({
                "key": key, "name": g.name, "shape": shape_name(*shape),
                "archetype": archetype.name(), "text": g.text(),
            }));
            members.push(decks.len());
            decks.push((
                *shape,
                HouseDeck {
                    key,
                    deck: g.loaded(),
                },
            ));
        }
        if members.is_empty() {
            bail!("no {} deck could be built", shape_name(*shape));
        }
        by_shape.push(members);
    }

    fs::create_dir_all(args.out.join("records"))?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    fs::write(
        args.out.join("run.json"),
        serde_json::to_vec_pretty(&json!({
            "run": run,
            "build": baylee_build::short(),
            "started": stamp,
            "working_hash": working.hash(),
            "working_cards": working.tested.len(),
            "args": format!("{args:?}"),
            "decks": deck_json,
        }))?,
    )?;

    let list: Vec<u64> = if args.only.is_empty() {
        (0..args.games).collect()
    } else {
        args.only.clone()
    };
    let threads = if args.threads == 0 {
        std::thread::available_parallelism().map_or(4, usize::from)
    } else {
        args.threads
    };
    let next = AtomicU64::new(0);
    let games_out = Mutex::new(BufWriter::new(File::create(args.out.join("games.jsonl"))?));
    let findings_out = Mutex::new(BufWriter::new(File::create(
        args.out.join("findings.jsonl"),
    )?));
    let tally: Arc<Mutex<BTreeMap<String, u64>>> = Arc::default();
    let examples: Arc<Mutex<BTreeMap<String, Vec<u64>>>> = Arc::default();
    let done = AtomicU64::new(0);
    let started = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let slot = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&number) = list.get(usize::try_from(slot).unwrap_or(usize::MAX))
                    else {
                        break;
                    };
                    let game = play(number, &args, &run, &decks, &by_shape);
                    if !game.findings.is_empty() {
                        let _ = fs::write(
                            args.out.join(format!("records/{number:07}.jsonl")),
                            &game.record,
                        );
                        let mut counts = tally.lock().expect("tally");
                        let mut firsts = examples.lock().expect("examples");
                        let mut out = findings_out.lock().expect("findings");
                        for found in &game.findings {
                            let kind = found["finding"].as_str().unwrap_or("?").to_owned();
                            *counts.entry(kind.clone()).or_default() += 1;
                            let games = firsts.entry(kind).or_default();
                            if games.len() < 5 && !games.contains(&number) {
                                games.push(number);
                            }
                            let _ = writeln!(out, "{found}");
                        }
                    }
                    let _ = writeln!(games_out.lock().expect("games"), "{}", game.line);
                    let finished = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if finished.is_multiple_of(100) {
                        eprintln!(
                            "[fuzz] {finished}/{} games, {:.0} s, findings {:?}",
                            list.len(),
                            started.elapsed().as_secs_f64(),
                            tally.lock().expect("tally")
                        );
                    }
                }
            });
        }
    });
    games_out.lock().expect("games").flush()?;
    findings_out.lock().expect("findings").flush()?;
    let tally = tally.lock().expect("tally").clone();
    let examples = examples.lock().expect("examples").clone();
    println!(
        "[fuzz] {} games in {:.0} s; findings by kind (first games):",
        list.len(),
        started.elapsed().as_secs_f64()
    );
    for (kind, n) in &tally {
        println!("  {kind:24} {n:6}  {:?}", examples.get(kind));
    }
    let summary = json!({"games": list.len(), "findings": tally, "examples": examples,
        "secs": started.elapsed().as_secs_f64()});
    fs::write(
        args.out.join("summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    Ok(())
}
