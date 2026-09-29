//! Plays the trained AI against the house profiles and says how often it wins.
//!
//! ```text
//! cargo run --profile selfplay -p baylee-train --features onnx --bin arena -- \
//!     --model ~/baylee-data/models/policy-v1/policy.onnx --games 1000 --out ~/baylee-data/arena/a001
//! ```
//!
//! Each game is hosted by a `Session` as a table is: the net's chair is taken
//! over (`Session::take_over`) and answered through `Session::act`, so its
//! answers are recorded as a seat's and the house opponent keeps everything it
//! has at a real table, scouting included. The net plays each seat half the
//! time. A question the net does not answer yet (`Arrange`, a wide number)
//! and an answer the engine refuses are handed to the house and counted.
//!
//! Writes `arena.json` (win rates with 95 % Wilson intervals, per opponent
//! profile) and the games' records, like a self-play run. A v2 export
//! (`policy.onnx`) and a v3 one (`net.onnx`) both play; the export's JSON says
//! which encoder it reads.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};
use baylee_core::ids::PlayerId;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::Pending;
use baylee_gamehost::Session;
use baylee_train::features3::{Table, deck_list};
use baylee_train::housedeck::HouseDeck;
use baylee_train::netplay::{Answer, NetPlayer};
use baylee_train::netplay3::NetPlayer3;
use baylee_train::selfplay::table;
use baylee_train::working::{Working, repo_root};
use clap::Parser;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::json;

#[derive(Parser, Debug)]
#[command(about = "Plays the trained AI against the house profiles")]
struct Args {
    /// The exported policy (`policy.onnx`).
    #[arg(long)]
    model: PathBuf,
    /// Rows the model was exported for.
    #[arg(long, default_value_t = 192)]
    entities: usize,
    /// The house profile the net is told to play as.
    #[arg(long, default_value = "expert")]
    as_profile: String,
    /// House profiles it plays against; games cycle through them.
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "novice,casual,steady,sharp,expert"
    )]
    against: Vec<String>,
    /// House decks; each game plays the two in both seat orders.
    #[arg(long, value_delimiter = ',', default_value = "allytifact,victory")]
    decks: Vec<String>,
    /// Cut each deck to its working cards, as the training games were.
    #[arg(long, default_value_t = true)]
    working_only: bool,
    /// Games.
    #[arg(long, default_value_t = 1000)]
    games: u64,
    /// The first game's seed.
    #[arg(long, default_value_t = 7_000_001)]
    first_seed: u64,
    /// Threads; 0 = one per core.
    #[arg(long, default_value_t = 0)]
    threads: usize,
    /// Seconds a game may take.
    #[arg(long, default_value_t = 60)]
    max_secs: u64,
    /// Where results go; must not exist.
    #[arg(long)]
    out: PathBuf,
}

const PROFILES: [&str; 5] = ["novice", "casual", "steady", "sharp", "expert"];

/// A net of either encoding.
enum Net {
    V2(NetPlayer),
    V3(NetPlayer3),
}

impl Net {
    fn answer(
        &mut self,
        view: &baylee_view::PlayerView,
        pending: &Pending,
        table: &Table<'_>,
    ) -> anyhow::Result<Option<Answer>> {
        match self {
            Self::V2(net) => net.answer(view, pending),
            Self::V3(net) => net.answer(view, pending, table),
        }
    }
}

fn profile(name: &str) -> anyhow::Result<AIProfile> {
    AIProfile::named(name).with_context(|| format!("unknown profile {name}"))
}

/// A game's result from the net's side.
struct Played {
    i: u64,
    against: usize,
    net_seat: u8,
    outcome: &'static str,
    turn: u32,
    net_answers: u64,
    house_fallbacks: u64,
    refused: u64,
    net_ms: f64,
    record: Vec<u8>,
    /// The net's refused answers: what was asked, what it answered, why the
    /// engine refused, and whether the answer is made of options the question
    /// enumerated (it must be; `false` is a trainer bug, `true` an engine one).
    refusals: Vec<serde_json::Value>,
}

/// 95 % Wilson interval for `k` of `n`. Game counts stay far below 2^52.
#[allow(clippy::cast_precision_loss)]
fn wilson(k: u64, n: u64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let (k, n) = (k as f64, n as f64);
    let z = 1.96_f64;
    let p = k / n;
    let denom = 1.0 + z * z / n;
    let centre = (p + z * z / (2.0 * n)) / denom;
    let half = z * ((p * (1.0 - p) + z * z / (4.0 * n)) / n).sqrt() / denom;
    (centre - half, centre + half)
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)] // one game loop and its tallies; counts stay far below 2^52
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.out.exists() {
        bail!("{} exists", args.out.display());
    }
    fs::create_dir_all(&args.out)?;
    let working = Working::scan(&repo_root())?;
    let mut decks: Vec<HouseDeck> = args
        .decks
        .iter()
        .map(|k| HouseDeck::named(k))
        .collect::<anyhow::Result<_>>()?;
    if args.working_only {
        for d in &mut decks {
            *d = d.working_only(&working)?.0;
        }
    }
    let against: Vec<AIProfile> = args
        .against
        .iter()
        .map(|p| profile(p))
        .collect::<anyhow::Result<_>>()?;
    let as_profile = PROFILES
        .iter()
        .position(|p| *p == args.as_profile)
        .context("--as-profile names a house profile")? as i64;
    let meta: serde_json::Value =
        serde_json::from_slice(&fs::read(args.model.with_extension("onnx.json"))?)
            .context("the export's .onnx.json")?;
    let glob_width = meta["inputs"]["glob"][1].as_u64().context("glob width")? as usize;
    // The interface a model is exported against: a net of any size drops
    // in, a net of another encoding or table layout is refused.
    let encoder = meta["encoder_version"]
        .as_u64()
        .context("encoder version")?;
    let known = [
        u64::from(baylee_train::features::ENCODER_VERSION),
        u64::from(baylee_train::features3::VERSION),
    ];
    if !known.contains(&encoder) {
        bail!("the model reads encoder v{encoder}; this build writes {known:?}");
    }
    let outputs: Vec<&str> = meta["outputs"]
        .as_array()
        .context("outputs")?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    if outputs != baylee_train::netplay::TABLES {
        bail!(
            "the model's score tables are {outputs:?}; this build reads {:?}",
            baylee_train::netplay::TABLES
        );
    }
    let exported_for = meta["entities"].as_u64().context("entities")? as usize;
    if exported_for != args.entities {
        bail!(
            "the model was exported for {exported_for} rows, not --entities {}",
            args.entities
        );
    }
    let threads = match args.threads {
        0 => std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        n => n,
    };
    let run = args
        .out
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("arena")
        .to_owned();
    eprintln!(
        "[arena] {} games on {threads} threads · net as {} vs {:?} · model {}",
        args.games,
        args.as_profile,
        args.against,
        args.model.display()
    );

    let decks = Arc::new(decks);
    let against = Arc::new(against);
    let next = Arc::new(AtomicU64::new(0));
    let (tx, rx) = mpsc::channel::<anyhow::Result<Played>>();
    for _ in 0..threads {
        let (decks, against, next, tx, model, run) = (
            decks.clone(),
            against.clone(),
            next.clone(),
            tx.clone(),
            args.model.clone(),
            run.clone(),
        );
        let (games, first_seed, entities, wall) = (
            args.games,
            args.first_seed,
            args.entities,
            Duration::from_secs(args.max_secs),
        );
        std::thread::spawn(move || {
            let loaded = if encoder == u64::from(baylee_train::features3::VERSION) {
                NetPlayer3::load(&model, entities, as_profile).map(Net::V3)
            } else {
                NetPlayer::load(&model, entities, glob_width, as_profile).map(Net::V2)
            };
            let mut net = match loaded {
                Ok(net) => net,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= games {
                    break;
                }
                // A game that panics the engine is a finding, not the end of this
                // worker: it is reported with its seed and decks, and the next
                // game is played.
                let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || -> anyhow::Result<Played> {
                        let seed = first_seed + i;
                        let opp = (i / 2) as usize % against.len();
                        let net_seat = (i % 2) as u8;
                        let deck_order = ((i / 2) / against.len() as u64) % 2;
                        let (a, b) = if deck_order == 0 {
                            (&decks[0], &decks[1])
                        } else {
                            (&decks[1], &decks[0])
                        };
                        let mut profiles = [against[opp]; 2];
                        profiles[usize::from(net_seat)] =
                            AIProfile::named("expert").unwrap_or(against[opp]);
                        let preset = table(seed, a, b, profiles);
                        let mut session = Session::new_recorded(&preset, baylee_build::short())
                            .context("the preset builds")?;
                        session.describe(format!("{run}-{i:07}"), vec!["0".into(), "1".into()]);
                        let me = PlayerId::new(net_seat);
                        // What a v3 net reads beyond the view: the sides and
                        // its own deck list.
                        let teams: Vec<Option<u8>> = preset.seats.iter().map(|s| s.team).collect();
                        let deck = deck_list(&preset.seats[usize::from(net_seat)]);
                        let seat_table = Table {
                            teams: &teams,
                            deck: &deck,
                        };
                        session.take_over(me);
                        let started = Instant::now();
                        let (mut net_answers, mut fallbacks, mut refused, mut net_time) =
                            (0, 0, 0, Duration::ZERO);
                        let mut refusals = Vec::new();
                        let outcome = loop {
                            if let Pending::GameOver(result) = session.pending() {
                                let winners = session.winning_seats(*result);
                                break match winners.as_slice() {
                                    [w] if *w == me => "win",
                                    [_] => "loss",
                                    _ => "draw",
                                };
                            }
                            if started.elapsed() > wall {
                                break "time_cap";
                            }
                            session.pump_at_most(32);
                            if let Some((pending, view)) = session.view_for(me) {
                                let t = Instant::now();
                                let answer = net.answer(&view, &pending, &seat_table)?;
                                net_time += t.elapsed();
                                let action = if let Some(a) = answer {
                                    net_answers += 1;
                                    a.action
                                } else {
                                    fallbacks += 1;
                                    session
                                        .house_action(me)
                                        .context("a question has an answer")?
                                };
                                let hand: Vec<baylee_core::ids::ObjectId> =
                                    view.hand.iter().map(|h| h.id).collect();
                                let enumerated =
                                    baylee_train::policy::steps(&pending, &hand, &action).is_ok();
                                if let Err(why) = session.act(me, action.clone()) {
                                    refused += 1;
                                    refusals.push(json!({
                                    "game": i,
                                    "decision": session.decision_seq(),
                                    "kind": baylee_train::features::PENDING_KINDS
                                        [baylee_train::features::question(&pending, view.hand.len()).0 as usize],
                                    "pending": format!("{pending:?}").chars().take(600).collect::<String>(),
                                    "answer": format!("{action:?}"),
                                    "refusal": why,
                                    "enumerated": enumerated,
                                }));
                                    let fallback = session
                                        .house_action(me)
                                        .context("a question has an answer")?;
                                    if session.act(me, fallback).is_err() {
                                        break "stuck";
                                    }
                                }
                            }
                        };
                        Ok(Played {
                            i,
                            against: opp,
                            net_seat,
                            outcome,
                            turn: session.state().turn.number,
                            net_answers,
                            house_fallbacks: fallbacks,
                            refused,
                            net_ms: net_time.as_secs_f64() * 1e3,
                            record: session.take_record(),
                            refusals,
                        })
                    },
                ));
                let result = played.unwrap_or_else(|panic| {
                    let seed = first_seed + i;
                    let deck_order = ((i / 2) / against.len() as u64) % 2;
                    let message = panic
                        .downcast_ref::<&str>()
                        .map(ToString::to_string)
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_default();
                    eprintln!(
                        "[arena] game {i} (seed {seed}, deck order {deck_order}, net seat {}) panicked: {message}",
                        i % 2
                    );
                    Ok(Played {
                        i,
                        against: (i / 2) as usize % against.len(),
                        net_seat: (i % 2) as u8,
                        outcome: "panicked",
                        turn: 0,
                        net_answers: 0,
                        house_fallbacks: 0,
                        refused: 0,
                        net_ms: 0.0,
                        record: Vec::new(),
                        refusals: vec![json!({"game": i, "seed": seed, "deck_order": deck_order, "panic": message})],
                    })
                });
                if tx.send(result).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);

    let mut records = GzEncoder::new(
        File::create(args.out.join("records.jsonl.gz"))?,
        Compression::new(6),
    );
    let mut games_log = File::create(args.out.join("games.jsonl"))?;
    let mut refusals_log = File::create(args.out.join("refusals.jsonl"))?;
    // Refusals by question kind, and whether the refused answer was made of
    // enumerated options (true: the engine refused what it offered).
    let mut refusal_kinds: BTreeMap<String, [u64; 2]> = BTreeMap::new();
    // Per opponent: wins, losses, draws, other.
    let mut tally: BTreeMap<usize, [u64; 4]> = BTreeMap::new();
    let (mut answers, mut fallbacks, mut refused, mut net_ms) = (0_u64, 0_u64, 0_u64, 0.0_f64);
    let started = Instant::now();
    let mut done = 0_u64;
    let mut last = Instant::now();
    for result in rx {
        let p = result?;
        done += 1;
        let t = tally.entry(p.against).or_default();
        t[match p.outcome {
            "win" => 0,
            "loss" => 1,
            "draw" => 2,
            _ => 3,
        }] += 1;
        answers += p.net_answers;
        fallbacks += p.house_fallbacks;
        refused += p.refused;
        net_ms += p.net_ms;
        records.write_all(&p.record)?;
        for r in &p.refusals {
            writeln!(refusals_log, "{r}")?;
            let kind = r["kind"].as_str().unwrap_or("?").to_owned();
            let enumerated = r["enumerated"].as_bool().unwrap_or(false);
            refusal_kinds.entry(kind).or_default()[usize::from(enumerated)] += 1;
        }
        writeln!(
            games_log,
            "{}",
            json!({"i": p.i, "against": args.against[p.against], "net_seat": p.net_seat, "outcome": p.outcome,
                   "turn": p.turn, "net_answers": p.net_answers, "house_fallbacks": p.house_fallbacks,
                   "refused": p.refused, "net_ms": p.net_ms})
        )?;
        if last.elapsed() >= Duration::from_secs(5) || done == args.games {
            last = Instant::now();
            let line: Vec<String> = tally
                .iter()
                .map(|(o, t)| {
                    let decided = t[0] + t[1];
                    format!("{} {}/{}", args.against[*o], t[0], decided)
                })
                .collect();
            eprintln!(
                "[arena] {done}/{} games · net wins vs {} · {:.2} ms/answer · fallbacks {fallbacks} · refused {refused}",
                args.games,
                line.join(", "),
                net_ms / answers.max(1) as f64
            );
        }
    }
    records.finish()?;
    let results: BTreeMap<String, serde_json::Value> = tally
        .iter()
        .map(|(o, t)| {
            let decided = t[0] + t[1];
            let (lo, hi) = wilson(t[0], decided);
            (
                args.against[*o].clone(),
                json!({"wins": t[0], "losses": t[1], "draws": t[2], "stopped": t[3],
                       "win_rate": t[0] as f64 / decided.max(1) as f64, "ci95": [lo, hi]}),
            )
        })
        .collect();
    let report = json!({
        "model": args.model, "as_profile": args.as_profile, "games": done, "results": results,
        "net_answers": answers, "house_fallbacks": fallbacks, "refused": refused,
        "refused_by_kind": refusal_kinds.iter().map(|(k, [not_enumerated, enumerated])| (k.clone(), json!({"enumerated": enumerated, "not_enumerated": not_enumerated}))).collect::<BTreeMap<_, _>>(),
        "ms_per_answer": net_ms / answers.max(1) as f64, "seconds": started.elapsed().as_secs_f64(),
        "build": baylee_build::short(), "working_hash": working.hash(),
    });
    fs::write(
        args.out.join("arena.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
