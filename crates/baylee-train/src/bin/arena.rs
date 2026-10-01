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
//! has at a real table, scouting included. A question the net does not answer
//! yet (`Arrange`, a wide number) and an answer the engine refuses are handed
//! to the house and counted.
//!
//! **Duplicate deals.** Magic's decks and draws move a result more than play
//! does, so games come in deals, as in duplicate bridge. A deal fixes the
//! seed (the shuffles), the opponent and which deck sits in which seat. The
//! net plays it from both seats, and the house at `--as-profile` plays it
//! from the same two seats as the baseline. Every game of a deal carries one
//! game id, so the house's noise (`Session::describe`) is the same in all of
//! them, and a result that differs from the baseline is the net's doing.
//! Deck strength, draw luck and the seat cancel within a deal.
//!
//! Writes `arena.json`: win rates with 95 % Wilson intervals per opponent
//! profile, and under `duplicate` the net against the house's baseline per
//! matchup and overall. Also writes the games' records, like a self-play run. A v2 export
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
    /// Deals: each is played by the net from both seats and by the house from
    /// both seats (once, when the net plays as the opponent's own profile).
    #[arg(long, default_value_t = 500)]
    deals: u64,
    /// The first deal's seed.
    #[arg(long, default_value_t = 7_000_001)]
    first_seed: u64,
    /// Threads; 0 = one per core.
    #[arg(long, default_value_t = 0)]
    threads: usize,
    /// Decisions a game may take, every seat's questions together
    /// (`Session::decision_seq`), before it is stopped. The cap that shapes
    /// the data: it ends a game at the same move on every machine. Finished
    /// games take about 900 (at most ~3,400 in a 1,500-game arena), and a
    /// game past 6,000 is a stall, not play.
    #[arg(long, default_value_t = 6000)]
    max_decisions: u64,
    /// Seconds a game may take: only a guard against an engine that stops
    /// deciding. A game it stops ends at a machine-dependent point, so it is
    /// reported as `time_cap` apart from `decision_cap`.
    #[arg(long, default_value_t = 600)]
    max_secs: u64,
    /// The name each game's id is made of (`<name>-<i>`), from which
    /// `Session::describe` seeds the house's play. The same name plays the
    /// same house on the same seeds, so two nets' arenas pair game for
    /// game; a name taken from `--out` would not.
    #[arg(long, default_value = "arena")]
    name: String,
    /// Run the net on its fixed-batch exports (`export_onnx3.py --batch`)
    /// through one batch server shared by every game: on the GPU with the
    /// `onnx-cuda` feature. Give it more `--threads` than cores, since a game
    /// waits while its decision is in a batch.
    #[arg(long)]
    gpu_batch: bool,
    /// How long the batch server waits for a batch to fill, in microseconds.
    #[arg(long, default_value_t = 2000)]
    batch_wait_us: u64,
    /// A language model plays the watched seat instead of the net, as the
    /// seat bridge names one (`openai:<model>`): the house plays the seat
    /// and the model answers the questions of `--llm-kinds`. Measured against
    /// the house's baseline on the same deals, like the net.
    #[cfg(feature = "llm")]
    #[arg(long)]
    llm: Option<String>,
    /// Where the model is served (an OpenAI-compatible server on this
    /// machine needs no key; LM Studio's default port).
    #[cfg(feature = "llm")]
    #[arg(long, default_value = "http://127.0.0.1:1234/v1")]
    llm_base: String,
    /// The question kinds the model answers (`features::PENDING_KINDS`
    /// names, e.g. `choose_blockers`); none: every question with more than
    /// one answer.
    #[cfg(feature = "llm")]
    #[arg(long, value_delimiter = ',')]
    llm_kinds: Vec<String>,
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

/// Who plays a deal's watched seat: the net, or the house at the net's
/// profile (the baseline).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Net(u8),
    /// The house with a language model answering the questions of chosen
    /// kinds (`--llm`, `baylee_train::llmchair`).
    #[cfg_attr(not(feature = "llm"), allow(dead_code))]
    Llm(u8),
    House(u8),
}

impl Role {
    const fn seat(self) -> u8 {
        match self {
            Self::Net(s) | Self::Llm(s) | Self::House(s) => s,
        }
    }
}

/// Which deck of `decks` sits in seat 0 and seat 1 in `deal`, and the
/// opponent's profile: every opponent meets both deck orders.
fn deal_setup(deal: u64, opponents: usize) -> (usize, [usize; 2]) {
    let opp = usize::try_from(deal).unwrap_or(0) % opponents;
    let order = (deal / opponents as u64) % 2;
    (opp, if order == 0 { [0, 1] } else { [1, 0] })
}

/// A game's result from the watched seat's side.
struct Played {
    i: u64,
    deal: u64,
    role: Role,
    against: usize,
    net_seat: u8,
    outcome: &'static str,
    turn: u32,
    net_answers: u64,
    house_fallbacks: u64,
    /// Questions the house answered because the net was asked again in a
    /// game state it had been asked in: a loop of its own answers.
    stalls: u64,
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
    let run = args.name.clone();
    let as_house = profile(&args.as_profile)?;
    #[cfg(feature = "llm")]
    let chair = args
        .llm
        .as_ref()
        .map(|spec| baylee_train::llmchair::LlmChair::new(spec, &args.llm_base, &args.llm_kinds))
        .transpose()?
        .map(Arc::new);
    #[cfg(feature = "llm")]
    let player: fn(u8) -> Role = if chair.is_some() {
        Role::Llm
    } else {
        Role::Net
    };
    #[cfg(not(feature = "llm"))]
    let player: fn(u8) -> Role = Role::Net;
    // Each deal: the net from both seats, then the house's baseline from
    // both. Against its own profile the house's two baseline games are one
    // game, so it is played once.
    let tasks: Vec<(u64, Role)> = (0..args.deals)
        .flat_map(|deal| {
            let mirror = against[deal_setup(deal, against.len()).0] == as_house;
            [player(0), player(1), Role::House(0)]
                .into_iter()
                .chain((!mirror).then_some(Role::House(1)))
                .map(move |role| (deal, role))
        })
        .collect();
    let total = tasks.len() as u64;
    let tasks = Arc::new(tasks);
    eprintln!(
        "[arena] {} deals, {total} games on {threads} threads · net as {} vs {:?} · model {}",
        args.deals,
        args.as_profile,
        args.against,
        args.model.display()
    );

    let server = if args.gpu_batch {
        if encoder != u64::from(baylee_train::features3::VERSION) {
            bail!("--gpu-batch plays v3 exports only");
        }
        Some(baylee_train::batchnet::BatchNet::spawn(
            &args.model,
            Duration::from_micros(args.batch_wait_us),
        )?)
    } else {
        None
    };
    let decks = Arc::new(decks);
    let against = Arc::new(against);
    let next = Arc::new(AtomicU64::new(0));
    let (tx, rx) = mpsc::channel::<anyhow::Result<Played>>();
    for _ in 0..threads {
        let server = server.clone();
        #[cfg(feature = "llm")]
        let chair = chair.clone();
        let (decks, against, next, tx, model, run) = (
            decks.clone(),
            against.clone(),
            next.clone(),
            tx.clone(),
            args.model.clone(),
            run.clone(),
        );
        let (tasks, as_house, first_seed, entities, max_decisions, wall) = (
            tasks.clone(),
            as_house,
            args.first_seed,
            args.entities,
            args.max_decisions,
            Duration::from_secs(args.max_secs),
        );
        std::thread::spawn(move || {
            let loaded = if encoder == u64::from(baylee_train::features3::VERSION) {
                match server {
                    Some(server) => NetPlayer3::served(&model, entities, as_profile, server),
                    None => NetPlayer3::load(&model, entities, as_profile),
                }
                .map(Net::V3)
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
                let Some(&(deal, role)) = usize::try_from(i).ok().and_then(|i| tasks.get(i)) else {
                    break;
                };
                // A game that panics the engine is a finding, not the end of this
                // worker: it is reported with its seed and decks, and the next
                // game is played.
                let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || -> anyhow::Result<Played> {
                        let seed = first_seed + deal;
                        let (opp, [a, b]) = deal_setup(deal, against.len());
                        let net_seat = role.seat();
                        let mut profiles = [against[opp]; 2];
                        profiles[usize::from(net_seat)] = as_house;
                        let preset = table(seed, &decks[a], &decks[b], profiles);
                        let mut session = Session::new_recorded(&preset, baylee_build::short())
                            .context("the preset builds")?;
                        // One id for every game of the deal: the house's
                        // noise is seeded from it, seat by seat.
                        session.describe(format!("{run}-{deal:07}"), vec!["0".into(), "1".into()]);
                        let me = PlayerId::new(net_seat);
                        // What a v3 net reads beyond the view: the sides and
                        // its own deck list.
                        let teams: Vec<Option<u8>> = preset.seats.iter().map(|s| s.team).collect();
                        let deck = deck_list(&preset.seats[usize::from(net_seat)]);
                        let seat_table = Table {
                            teams: &teams,
                            deck: &deck,
                        };
                        let net_plays = matches!(role, Role::Net(_));
                        if net_plays {
                            session.take_over(me);
                        }
                        // The model's chair: the house plays the seat, and the
                        // questions the model takes are taken over one by one.
                        #[cfg(feature = "llm")]
                        let chair_plays = chair.as_ref().filter(|_| matches!(role, Role::Llm(_)));
                        #[cfg(feature = "llm")]
                        let context = baylee_train::llmchair::LlmChair::context(
                            format!("{run}-{deal:07}"),
                            &preset.seats[usize::from(net_seat)],
                            net_seat,
                            preset.format,
                        );
                        // A question the model could not answer, left to the
                        // house: by its decision number, so it is not asked
                        // again.
                        #[cfg(feature = "llm")]
                        let mut declined: Option<u64> = None;
                        let started = Instant::now();
                        let (mut net_answers, mut fallbacks, mut refused, mut net_time) =
                            (0, 0, 0, Duration::ZERO);
                        let mut refusals = Vec::new();
                        let (mut asked_in, mut stalls) = (BTreeMap::<u64, u32>::new(), 0_u64);
                        let outcome = loop {
                            if let Pending::GameOver(result) = session.pending() {
                                let winners = session.winning_seats(*result);
                                break match winners.as_slice() {
                                    [w] if *w == me => "win",
                                    [_] => "loss",
                                    _ => "draw",
                                };
                            }
                            if session.decision_seq() >= max_decisions {
                                break "decision_cap";
                            }
                            if started.elapsed() > wall {
                                break "time_cap";
                            }
                            #[cfg(feature = "llm")]
                            if let Some(chair) = chair_plays {
                                let seq = session.decision_seq();
                                let asked = session
                                    .view_for(me)
                                    .filter(|(p, v)| declined != Some(seq) && chair.takes(p, v));
                                if let Some((pending, view)) = asked {
                                    session.take_over(me);
                                    let answer = chair.answer(&context, seq, &view, &pending);
                                    let taken = match answer {
                                        Some(action) => {
                                            let ok = session.act(me, action).is_ok();
                                            if !ok {
                                                chair.engine_refused();
                                            }
                                            ok
                                        }
                                        None => false,
                                    };
                                    session.release(me);
                                    if taken {
                                        net_answers += 1;
                                    } else {
                                        fallbacks += 1;
                                        declined = Some(seq);
                                    }
                                } else {
                                    session.pump_at_most(1);
                                }
                                continue;
                            }
                            session.pump_at_most(32);
                            if !net_plays {
                                continue;
                            }
                            if let Some((pending, view)) = session.view_for(me) {
                                // Asked too often in one state, the net has gone
                                // round a loop of its own answers: the house
                                // answers there (`netplay3::STALL_VISITS`).
                                let visits = asked_in.entry(session.snapshot_hash()).or_default();
                                *visits += 1;
                                let again = *visits > baylee_train::netplay3::STALL_VISITS;
                                stalls += u64::from(again);
                                let t = Instant::now();
                                let answer = if again {
                                    None
                                } else {
                                    net.answer(&view, &pending, &seat_table)?
                                };
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
                            deal,
                            role,
                            against: opp,
                            net_seat,
                            outcome,
                            turn: session.state().turn.number,
                            net_answers,
                            house_fallbacks: fallbacks,
                            stalls,
                            refused,
                            net_ms: net_time.as_secs_f64() * 1e3,
                            record: session.take_record(),
                            refusals,
                        })
                    },
                ));
                let result = played.unwrap_or_else(|panic| {
                    let seed = first_seed + deal;
                    let (_, deck_order) = deal_setup(deal, against.len());
                    let message = panic
                        .downcast_ref::<&str>()
                        .map(ToString::to_string)
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_default();
                    eprintln!(
                        "[arena] game {i} (seed {seed}, decks {deck_order:?}, {role:?}) panicked: {message}",
                    );
                    Ok(Played {
                        i,
                        deal,
                        role,
                        against: deal_setup(deal, against.len()).0,
                        net_seat: role.seat(),
                        outcome: "panicked",
                        turn: 0,
                        net_answers: 0,
                        house_fallbacks: 0,
                        stalls: 0,
                        refused: 0,
                        net_ms: 0.0,
                        record: Vec::new(),
                        refusals: vec![json!({"game": i, "seed": seed, "decks": deck_order, "panic": message})],
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
    // Per opponent: the net's wins, losses, draws, other.
    let mut tally: BTreeMap<usize, [u64; 4]> = BTreeMap::new();
    // Per deal, [net, house] by seat: the watched seat's outcome.
    let mut deals: BTreeMap<u64, [[Option<&'static str>; 2]; 2]> = BTreeMap::new();
    let (mut answers, mut fallbacks, mut refused, mut net_ms) = (0_u64, 0_u64, 0_u64, 0.0_f64);
    let mut stalls = 0_u64;
    let started = Instant::now();
    let mut done = 0_u64;
    let mut last = Instant::now();
    for result in rx {
        let p = result?;
        done += 1;
        let (who, net) = match p.role {
            Role::Net(_) | Role::Llm(_) => (0, true),
            Role::House(_) => (1, false),
        };
        deals.entry(p.deal).or_default()[who][usize::from(p.net_seat)] = Some(p.outcome);
        if net {
            tally.entry(p.against).or_default()[match p.outcome {
                "win" => 0,
                "loss" => 1,
                "draw" => 2,
                _ => 3,
            }] += 1;
        }
        answers += p.net_answers;
        fallbacks += p.house_fallbacks;
        stalls += p.stalls;
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
            json!({"i": p.i, "deal": p.deal, "role": match p.role { Role::Net(_) => "net", Role::Llm(_) => "llm", Role::House(_) => "house" },
                   "deck": decks[deal_setup(p.deal, args.against.len()).1[usize::from(p.net_seat)]].key,
                   "against": args.against[p.against], "net_seat": p.net_seat, "outcome": p.outcome,
                   "turn": p.turn, "net_answers": p.net_answers, "house_fallbacks": p.house_fallbacks, "stalls": p.stalls,
                   "refused": p.refused, "net_ms": p.net_ms})
        )?;
        if last.elapsed() >= Duration::from_secs(5) || done == total {
            last = Instant::now();
            let line: Vec<String> = tally
                .iter()
                .map(|(o, t)| {
                    let decided = t[0] + t[1];
                    format!("{} {}/{}", args.against[*o], t[0], decided)
                })
                .collect();
            eprintln!(
                "[arena] {done}/{total} games · net wins vs {} · {:.2} ms/answer · fallbacks {fallbacks} · refused {refused}",
                line.join(", "),
                net_ms / answers.max(1) as f64
            );
        }
    }
    records.finish()?;
    if let Some(server) = &server {
        server.shutdown();
    }
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
    let duplicate = duplicate(&deals, &decks, &args.against);
    #[cfg(feature = "llm")]
    let llm = chair.as_ref().map(|c| c.report());
    #[cfg(not(feature = "llm"))]
    let llm: Option<serde_json::Value> = None;
    let report = json!({
        "model": args.model, "as_profile": args.as_profile, "name": run, "deals": args.deals,
        "games": done, "results": results, "duplicate": duplicate, "llm": llm,
        "caps": {"decisions": args.max_decisions, "secs": args.max_secs},
        "batch": server.as_ref().map(|s| {
            let (batches, decisions, size) = s.stats();
            json!({"size": size, "batches": batches, "decisions": decisions,
                   "fill": decisions as f64 / (batches.max(1) * size as u64) as f64})
        }),
        "net_answers": answers, "house_fallbacks": fallbacks, "stalls": stalls, "refused": refused,
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

/// The net against the house's baseline, deal by deal: in each seat of each
/// deal both played, the net scores 1, ½ or 0 and so does the house at the
/// net's profile, and the difference is the net's doing. A deal counts only
/// when all its games ended (no cap, no panic). Per matchup (the watched
/// seat's deck against the other deck, and the opponent's profile), and
/// overall with a 95 % interval over deals.
#[allow(clippy::cast_precision_loss)] // counts stay far below 2^52
fn duplicate(
    deals: &BTreeMap<u64, [[Option<&'static str>; 2]; 2]>,
    decks: &[HouseDeck],
    against: &[String],
) -> serde_json::Value {
    #[derive(Default)]
    struct Duel {
        games: u64,
        net: f64,
        house: f64,
        net_only: u64,
        house_only: u64,
    }
    let score = |o: Option<&str>| match o? {
        "win" => Some(1.0),
        "loss" => Some(0.0),
        "draw" => Some(0.5),
        _ => None,
    };
    // Against its own profile the house played one baseline game: the other
    // seat's side of it.
    let other_side = |o: &'static str| match o {
        "win" => "loss",
        "loss" => "win",
        o => o,
    };
    let mut by_matchup: BTreeMap<String, Duel> = BTreeMap::new();
    let mut per_deal: Vec<f64> = Vec::new();
    let mut incomplete = 0_u64;
    for (&deal, [net, house]) in deals {
        let house = [house[0], house[1].or_else(|| house[0].map(other_side))];
        let scored: Option<Vec<(f64, f64)>> = (0..2)
            .map(|s| Some((score(net[s])?, score(house[s])?)))
            .collect();
        let Some(scored) = scored else {
            incomplete += 1;
            continue;
        };
        let (opp, order) = deal_setup(deal, against.len());
        for (s, &(n, h)) in scored.iter().enumerate() {
            let key = format!(
                "{} vs {} ({})",
                decks[order[s]].key,
                decks[order[1 - s]].key,
                against[opp]
            );
            let m = by_matchup.entry(key).or_default();
            m.games += 1;
            m.net += n;
            m.house += h;
            m.net_only += u64::from(n > h);
            m.house_only += u64::from(h > n);
        }
        per_deal.push(scored.iter().map(|(n, h)| n - h).sum::<f64>() / 2.0);
    }
    let n = per_deal.len() as f64;
    let mean = per_deal.iter().sum::<f64>() / n.max(1.0);
    let var = per_deal.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    let half = 1.96 * (var / n.max(1.0)).sqrt();
    let rate = |x: f64, games: u64| x / games.max(1) as f64;
    let games: u64 = by_matchup.values().map(|m| m.games).sum();
    json!({
        "deals": per_deal.len(), "incomplete": incomplete,
        "net": rate(by_matchup.values().map(|m| m.net).sum(), games),
        "house": rate(by_matchup.values().map(|m| m.house).sum(), games),
        "delta": mean, "ci95": [mean - half, mean + half],
        "net_only": by_matchup.values().map(|m| m.net_only).sum::<u64>(),
        "house_only": by_matchup.values().map(|m| m.house_only).sum::<u64>(),
        "by_matchup": by_matchup.iter().map(|(k, m)| (k.clone(), json!({
            "games": m.games, "net": rate(m.net, m.games), "house": rate(m.house, m.games),
            "delta": rate(m.net - m.house, m.games), "net_only": m.net_only, "house_only": m.house_only,
        }))).collect::<BTreeMap<_, _>>(),
    })
}
