//! Self-play for reinforcement learning: a learner net plays a league.
//!
//! ```text
//! cargo run --profile selfplay -p baylee-train --features onnx --bin league -- \
//!     --learner ~/baylee-data/models/v3-b/net.onnx \
//!     --league house:expert,house:sharp,net:~/baylee-data/models/policy-v1/policy.onnx,self \
//!     --games 20000 --out ~/baylee-data/runs/l001
//! ```
//!
//! The learner (a v3 export) takes one seat and samples its answers at
//! `--temperature`; the other seat is drawn from the league for each game:
//! a house profile (`house:<profile>`), a frozen net (`net:<export>`, v2 or
//! v3), or the learner itself (`self`, sampling too). Every chair a net plays
//! is taken over (`Session::take_over`) and answered through `Session::act`;
//! an answer the engine refuses is handed to the house and counted.
//!
//! The run is written as `selfplay` writes one (`run.json`, `games.jsonl`,
//! `records/`), with the learner's seat named `learner` among the
//! profiles, so `convert3` turns it into a dataset and a trainer finds the
//! learner's decisions by that name. A game's decks, opponent, seat and
//! every sampled answer follow from its number and `--name`.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, bail};
use baylee_cards::formats::Shape;
use baylee_core::ids::PlayerId;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::Pending;
use baylee_gamehost::Session;
use baylee_train::batchnet::BatchNet;
use baylee_train::deckgen::{self, Archetype, Rng};
use baylee_train::features3::{Table, deck_list};
use baylee_train::housedeck::HouseDeck;
use baylee_train::netplay::{Answer, NetPlayer};
use baylee_train::netplay3::NetPlayer3;
use baylee_train::selfplay::table;
use baylee_train::working::{Working, repo_root};
use clap::Parser;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value, json};

#[derive(Parser, Debug)]
#[command(about = "Self-play for reinforcement learning: a learner net plays a league")]
struct Args {
    /// The learner: a v3 export (`net.onnx`).
    #[arg(long)]
    learner: PathBuf,
    /// Rows the exports take at most.
    #[arg(long, default_value_t = 192)]
    entities: usize,
    /// The learner's sampling temperature (0 = its best answer).
    #[arg(long, default_value_t = 1.0)]
    temperature: f32,
    /// The league, comma-separated: `house:<profile>`, `net:<export>`, `self`.
    #[arg(long, value_delimiter = ',')]
    league: Vec<String>,
    /// House decks (`data/decks/<key>.txt`).
    #[arg(long, value_delimiter = ',', default_value = "allytifact,victory")]
    decks: Vec<String>,
    /// Decks to generate from the working pool besides the house decks
    /// (constructed, 60 cards).
    #[arg(long, default_value_t = 0)]
    generated: usize,
    /// Seed of the deck generator.
    #[arg(long, default_value_t = 1)]
    deck_seed: u64,
    /// Games.
    #[arg(long, default_value_t = 1000)]
    games: u64,
    /// The first game's seed.
    #[arg(long, default_value_t = 11_000_001)]
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
    /// Games per record shard.
    #[arg(long, default_value_t = 1000)]
    shard_games: u64,
    /// Play only these game numbers (comma-separated) instead of
    /// `0..games`: with the same `--name` and arguments, each is the game
    /// that number was in that run, move for move.
    #[arg(long, value_delimiter = ',')]
    only: Vec<u64>,
    /// The run's name; defaults to the name of `--out`.
    #[arg(long)]
    name: Option<String>,
    /// Run every v3 net on its fixed-batch exports (`export_onnx3.py
    /// --batch`) through one batch server per net, shared by every game: on
    /// the GPU with the `onnx-cuda` feature. v2 nets stay on the CPU. Give
    /// it more `--threads` than cores, since a game waits while its decision
    /// is in a batch.
    #[arg(long)]
    gpu_batch: bool,
    /// How long a batch server waits for a batch to fill, in microseconds.
    #[arg(long, default_value_t = 2000)]
    batch_wait_us: u64,
    /// Where the run is written; must not exist yet.
    #[arg(long)]
    out: PathBuf,
}

/// One chair of the league.
#[derive(Clone, Debug)]
enum Opponent {
    House(&'static str, AIProfile),
    Net(PathBuf),
    SelfPlay,
}

impl Opponent {
    fn parse(spec: &str) -> anyhow::Result<Self> {
        if spec == "self" {
            return Ok(Self::SelfPlay);
        }
        if let Some(p) = spec.strip_prefix("house:") {
            let (name, profile) = AIProfile::NAMED
                .iter()
                .find(|(n, _)| *n == p)
                .copied()
                .with_context(|| format!("unknown house profile {p}"))?;
            return Ok(Self::House(name, profile));
        }
        if let Some(path) = spec.strip_prefix("net:") {
            let path = expand(path);
            if !path.exists() {
                bail!("no export at {}", path.display());
            }
            return Ok(Self::Net(path));
        }
        bail!("a league chair is house:<profile>, net:<export> or self, not {spec}")
    }

    fn name(&self) -> String {
        match self {
            Self::House(n, _) => (*n).to_owned(),
            Self::Net(p) => format!("net:{}", p.display()),
            Self::SelfPlay => "self".to_owned(),
        }
    }
}

fn expand(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => std::env::var_os("HOME")
            .map_or_else(|| PathBuf::from(path), |h| PathBuf::from(h).join(rest)),
        None => PathBuf::from(path),
    }
}

/// A net in a chair: v2 or v3, from its export's JSON.
enum Player {
    V2(NetPlayer),
    V3(Box<NetPlayer3>),
}

impl Player {
    fn load(
        model: &Path,
        entities: usize,
        temperature: f32,
        seed: u64,
        servers: &BTreeMap<PathBuf, BatchNet>,
    ) -> anyhow::Result<Self> {
        let meta: Value = serde_json::from_slice(&fs::read(model.with_extension("onnx.json"))?)
            .with_context(|| format!("{}.json", model.display()))?;
        if meta["encoder_version"] == json!(3) {
            let mut net = NetPlayer3::load(model, entities, 4)?;
            if let Some(server) = servers.get(model) {
                net = net.with_server(server.clone());
            }
            net.temperature = temperature;
            net.seed(seed);
            Ok(Self::V3(Box::new(net)))
        } else {
            let glob = meta["inputs"]["glob"][1].as_u64().context("glob width")? as usize;
            Ok(Self::V2(NetPlayer::load(model, entities, glob, 4)?))
        }
    }

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

    fn reseed(&mut self, seed: u64) {
        if let Self::V3(net) = self {
            net.seed(seed);
        }
    }
}

/// `SplitMix64`'s finaliser.
const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn name_hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A shard of records, one gzip member per game.
struct Shard {
    path: PathBuf,
    file: BufWriter<File>,
    written: u64,
    games: u64,
}

impl Shard {
    fn open(dir: &Path, worker: usize, number: u64) -> anyhow::Result<Self> {
        let path = dir.join(format!("w{worker:02}-{number:04}.jsonl.gz"));
        Ok(Self {
            file: BufWriter::new(File::create(&path)?),
            path,
            written: 0,
            games: 0,
        })
    }

    fn append(&mut self, record: &[u8]) -> anyhow::Result<(u64, u64)> {
        let mut gz = GzEncoder::new(Vec::with_capacity(record.len() / 6), Compression::new(6));
        gz.write_all(record)?;
        let bytes = gz.finish()?;
        let at = self.written;
        self.file.write_all(&bytes)?;
        self.file.flush()?;
        self.written += bytes.len() as u64;
        self.games += 1;
        Ok((at, bytes.len() as u64))
    }
}

/// What one game left.
struct Played {
    line: Value,
    record: Vec<u8>,
    learner_won: Option<f64>,
    opponent: usize,
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one game loop
fn play_one(
    i: u64,
    run: &str,
    args: &Args,
    decks: &[HouseDeck],
    league: &[Opponent],
    learner: &mut Player,
    nets: &mut BTreeMap<usize, Player>,
    selfish: &mut Player,
) -> anyhow::Result<Played> {
    let seed = args.first_seed + i;
    let mut rng = Rng::new(mix(seed ^ name_hash(run)));
    let opponent = rng.below(league.len());
    let a = rng.below(decks.len());
    let mut b = rng.below(decks.len());
    while b == a && decks.len() > 1 {
        b = rng.below(decks.len());
    }
    let learner_seat = (i % 2) as u8;
    let other = 1 - learner_seat;
    let mut profiles = [AIProfile::named("expert").unwrap_or_default(); 2];
    if let Opponent::House(_, p) = &league[opponent] {
        profiles[usize::from(other)] = *p;
    }
    let preset = table(seed, &decks[a], &decks[b], profiles);
    let mut session =
        Session::new_recorded(&preset, baylee_build::short()).context("the preset builds")?;
    session.describe(format!("{run}-{i:07}"), vec!["0".into(), "1".into()]);
    let me = PlayerId::new(learner_seat);
    let them = PlayerId::new(other);
    session.take_over(me);
    let opponent_is_net = !matches!(league[opponent], Opponent::House(..));
    if opponent_is_net {
        session.take_over(them);
    }
    learner.reseed(mix(seed ^ 0x1ea7));
    selfish.reseed(mix(seed ^ 0x5e1f));
    let teams: Vec<Option<u8>> = preset.seats.iter().map(|s| s.team).collect();
    let deck_me = deck_list(&preset.seats[usize::from(learner_seat)]);
    let deck_them = deck_list(&preset.seats[usize::from(other)]);
    let started = Instant::now();
    let wall = Duration::from_secs(args.max_secs);
    let (mut refused, mut fallbacks, mut stalls) = (0_u64, 0_u64, 0_u64);
    // Per seat, how often it has been asked in each game state (only looked
    // up).
    let mut asked_in: [BTreeMap<u64, u32>; 2] = Default::default();
    let outcome = loop {
        if let Pending::GameOver(result) = session.pending() {
            let reason = format!("{:?}", result.reason);
            let winners = session.winning_seats(*result);
            break match winners.as_slice() {
                [] => json!({"kind": "draw", "reason": reason}),
                [w] => json!({"kind": "won", "seat": w.get(), "reason": reason}),
                ws => {
                    json!({"kind": "team_won", "seats": ws.iter().map(|w| w.get()).collect::<Vec<_>>(), "reason": reason})
                }
            };
        }
        if session.decision_seq() >= args.max_decisions {
            break json!({"kind": "decision_cap"});
        }
        if started.elapsed() > wall {
            break json!({"kind": "time_cap"});
        }
        session.pump_at_most(32);
        let mut asked = false;
        for seat in [me, them] {
            if seat == them && !opponent_is_net {
                continue;
            }
            let Some((pending, view)) = session.view_for(seat) else {
                continue;
            };
            asked = true;
            let (player, deck): (&mut Player, &[_]) = if seat == me {
                (&mut *learner, &deck_me)
            } else {
                match &league[opponent] {
                    Opponent::SelfPlay => (&mut *selfish, &deck_them),
                    _ => (
                        nets.get_mut(&opponent)
                            .context("the opponent's net is loaded")?,
                        &deck_them,
                    ),
                }
            };
            let seat_table = Table {
                teams: &teams,
                deck,
            };
            // Asked too often in one game state, a net has gone round a loop
            // of its own answers: the house answers there
            // (`netplay3::STALL_VISITS`).
            let visits = asked_in[usize::from(seat.get())]
                .entry(session.snapshot_hash())
                .or_default();
            *visits += 1;
            let again = *visits > baylee_train::netplay3::STALL_VISITS;
            stalls += u64::from(again);
            let answered = if again {
                None
            } else {
                player.answer(&view, &pending, &seat_table)?
            };
            let action = if let Some(a) = answered {
                a.action
            } else {
                fallbacks += 1;
                session
                    .house_action(seat)
                    .context("a question has an answer")?
            };
            if session.act(seat, action).is_err() {
                refused += 1;
                let fallback = session
                    .house_action(seat)
                    .context("a question has an answer")?;
                if session.act(seat, fallback).is_err() {
                    return Ok(Played {
                        line: json!({"i": i, "outcome": {"kind": "stuck"}}),
                        record: session.take_record(),
                        learner_won: None,
                        opponent,
                    });
                }
            }
        }
        if !asked && session.view_for(me).is_none() && session.view_for(them).is_none() {
            // Nothing asked of a net: the house chairs move on the next pump.
        }
    };
    let learner_won = match outcome["kind"].as_str() {
        Some("won") => Some(f64::from(u8::from(outcome["seat"] == json!(learner_seat)))),
        Some("draw") => Some(0.5),
        _ => None,
    };
    let mut names = vec![String::new(); 2];
    names[usize::from(learner_seat)] = "learner".into();
    names[usize::from(other)] = league[opponent].name();
    Ok(Played {
        line: json!({
            "game": format!("{run}-{i:07}"),
            "i": i,
            "seed": seed,
            "decks": [decks[a].key, decks[b].key],
            "profiles": names,
            "learner_seat": learner_seat,
            "opponent": league[opponent].name(),
            "outcome": outcome,
            "answers": session.decision_seq(),
            "turn": session.state().turn.number,
            "ms": started.elapsed().as_millis() as u64,
            "refused": refused,
            "house_fallbacks": fallbacks,
            "stalls": stalls,
        }),
        record: session.take_record(),
        learner_won,
        opponent,
    })
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)] // setup, workers and the tallies
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.out.exists() {
        bail!("{} exists", args.out.display());
    }
    let run = args.name.clone().unwrap_or_else(|| {
        args.out
            .file_name()
            .map_or_else(|| "league".into(), |n| n.to_string_lossy().into_owned())
    });
    let league: Vec<Opponent> = args
        .league
        .iter()
        .map(|s| Opponent::parse(s))
        .collect::<anyhow::Result<_>>()?;
    if league.is_empty() {
        bail!("--league names no chair");
    }
    let working = Working::scan(&repo_root())?;
    let mut decks: Vec<HouseDeck> = Vec::new();
    for key in &args.decks {
        decks.push(HouseDeck::named(key)?.working_only(&working)?.0);
    }
    if args.generated > 0 {
        let pool = deckgen::Pool::new(&working, &std::collections::BTreeSet::new());
        let mut attempt = 0_u64;
        let mut made = 0;
        while made < args.generated && attempt < args.generated as u64 * 4 {
            let archetype = Archetype::ALL[made % Archetype::ALL.len()];
            let seed = args.deck_seed.wrapping_mul(1_000_003).wrapping_add(attempt);
            attempt += 1;
            let Ok(g) = deckgen::generate(&pool, Shape::Constructed, archetype, seed) else {
                continue;
            };
            decks.push(HouseDeck {
                key: format!("g{made:04}"),
                deck: g.loaded(),
            });
            made += 1;
        }
    }
    if decks.len() < 2 {
        bail!("a league needs two decks or more");
    }
    let learner_meta: Value =
        serde_json::from_slice(&fs::read(args.learner.with_extension("onnx.json"))?)
            .context("the learner's net.onnx.json")?;
    if learner_meta["encoder_version"] != json!(3) {
        bail!("the learner must be a v3 export");
    }
    fs::create_dir_all(args.out.join("records"))?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    fs::write(
        args.out.join("run.json"),
        serde_json::to_vec_pretty(&json!({
            "run": run,
            "kind": "league",
            "record_version": baylee_gamehost::record::RECORD_VERSION,
            "build": baylee_build::short(),
            "commit": baylee_build::COMMIT,
            "dirty": baylee_build::DIRTY,
            "working": {"hash": working.hash(), "works": working.cards.len()},
            "training_data": true,
            "learner": args.learner,
            "learner_export": learner_meta,
            "temperature": args.temperature,
            "league": league.iter().map(Opponent::name).collect::<Vec<_>>(),
            "decks": decks.iter().map(|d| d.key.clone()).collect::<Vec<_>>(),
            "games": args.games,
            "first_seed": args.first_seed,
            "caps": {"decisions": args.max_decisions, "secs": args.max_secs},
            "started": stamp,
        }))?,
    )?;
    let threads = if args.threads == 0 {
        std::thread::available_parallelism().map_or(4, usize::from)
    } else {
        args.threads
    };
    eprintln!(
        "[league] {run}: {} games on {threads} threads · learner {} at temperature {} · league {:?}",
        args.games,
        args.learner.display(),
        args.temperature,
        league.iter().map(Opponent::name).collect::<Vec<_>>()
    );
    // One batch server per v3 net, the learner's shared by its self-play.
    let mut servers: BTreeMap<PathBuf, BatchNet> = BTreeMap::new();
    if args.gpu_batch {
        let wait = Duration::from_micros(args.batch_wait_us);
        let nets = league.iter().filter_map(|o| match o {
            Opponent::Net(path) => Some(path.clone()),
            _ => None,
        });
        for model in std::iter::once(args.learner.clone()).chain(nets) {
            let meta: Value =
                serde_json::from_slice(&fs::read(model.with_extension("onnx.json"))?)?;
            if meta["encoder_version"] == json!(3) && !servers.contains_key(&model) {
                let server = BatchNet::spawn(&model, wait)
                    .with_context(|| format!("the batch server for {}", model.display()))?;
                servers.insert(model, server);
            }
        }
    }
    let next = AtomicU64::new(0);
    let done = AtomicU64::new(0);
    let index = Mutex::new(BufWriter::new(File::create(args.out.join("games.jsonl"))?));
    let tally: Mutex<Vec<(f64, u64)>> = Mutex::new(vec![(0.0, 0); league.len()]);
    let failure: Mutex<Option<anyhow::Error>> = Mutex::new(None);
    let started = Instant::now();
    std::thread::scope(|scope| {
        for worker in 0..threads {
            let (args, decks, league, run, servers) = (&args, &decks, &league, &run, &servers);
            let (next, done, index, tally, failure) = (&next, &done, &index, &tally, &failure);
            scope.spawn(move || {
                let work = || -> anyhow::Result<()> {
                    let mut learner =
                        Player::load(&args.learner, args.entities, args.temperature, 0, servers)?;
                    let mut selfish =
                        Player::load(&args.learner, args.entities, args.temperature, 1, servers)?;
                    let mut nets = BTreeMap::new();
                    for (k, o) in league.iter().enumerate() {
                        if let Opponent::Net(path) = o {
                            nets.insert(k, Player::load(path, args.entities, 0.0, 0, servers)?);
                        }
                    }
                    let records = args.out.join("records");
                    let mut shard: Option<Shard> = None;
                    let mut shards = 0;
                    loop {
                        let taken = next.fetch_add(1, Ordering::Relaxed);
                        let i = if args.only.is_empty() {
                            taken
                        } else {
                            match usize::try_from(taken).ok().and_then(|n| args.only.get(n)) {
                                Some(&i) => i,
                                None => return Ok(()),
                            }
                        };
                        if i >= args.games && args.only.is_empty() {
                            return Ok(());
                        }
                        // A game that panics the engine is a finding, not
                        // the end of the run: it is reported with its seed,
                        // written as `panicked` (convert3 reads only
                        // finished games), and the next one is played.
                        let played = match std::panic::catch_unwind(
                            std::panic::AssertUnwindSafe(|| {
                                play_one(
                                    i,
                                    run,
                                    args,
                                    decks,
                                    league,
                                    &mut learner,
                                    &mut nets,
                                    &mut selfish,
                                )
                            }),
                        ) {
                            Ok(played) => played?,
                            Err(panic) => {
                                let message = panic
                                    .downcast_ref::<&str>()
                                    .map(ToString::to_string)
                                    .or_else(|| panic.downcast_ref::<String>().cloned())
                                    .unwrap_or_default();
                                let seed = args.first_seed + i;
                                eprintln!("[league] game {i} (seed {seed}) panicked: {message}");
                                Played {
                                    line: json!({"i": i, "seed": seed,
                                                 "outcome": {"kind": "panicked", "message": message}}),
                                    record: Vec::new(),
                                    learner_won: None,
                                    opponent: 0,
                                }
                            }
                        };
                        if shard.as_ref().is_none_or(|s| s.games >= args.shard_games) {
                            shard = Some(Shard::open(&records, worker, shards)?);
                            shards += 1;
                        }
                        let s = shard.as_mut().expect("opened above");
                        let (offset, len) = s.append(&played.record)?;
                        let mut line = played.line;
                        line["shard"] = json!(s.path.file_name().and_then(|n| n.to_str()));
                        line["offset"] = json!(offset);
                        line["len"] = json!(len);
                        writeln!(index.lock().expect("index"), "{line}")?;
                        if let Some(w) = played.learner_won {
                            let mut t = tally.lock().expect("tally");
                            t[played.opponent].0 += w;
                            t[played.opponent].1 += 1;
                        }
                        let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                        if d.is_multiple_of(200) {
                            let t = tally.lock().expect("tally").clone();
                            let rates: Vec<String> = t
                                .iter()
                                .zip(league)
                                .map(|((w, n), o)| {
                                    format!("{} {:.3} ({n})", o.name(), w / (*n).max(1) as f64)
                                })
                                .collect();
                            eprintln!(
                                "[league] {d}/{} games · {:.1} games/s · learner scores {}",
                                args.games,
                                d as f64 / started.elapsed().as_secs_f64(),
                                rates.join(", ")
                            );
                        }
                    }
                };
                if let Err(e) = work() {
                    failure.lock().expect("failure").get_or_insert(e);
                }
            });
        }
    });
    index.lock().expect("index").flush()?;
    let batches: Vec<Value> = servers
        .iter()
        .map(|(model, s)| {
            s.shutdown();
            let (batches, decisions, size) = s.stats();
            json!({"model": model, "size": size, "batches": batches, "decisions": decisions,
                   "fill": decisions as f64 / (batches.max(1) * size as u64) as f64})
        })
        .collect();
    if let Some(e) = failure.into_inner().expect("failure") {
        return Err(e);
    }
    let t = tally.into_inner().expect("tally");
    let summary: Vec<Value> = t
        .iter()
        .zip(&league)
        .map(|((w, n), o)| json!({"opponent": o.name(), "games": n, "learner_score": w / (*n).max(1) as f64}))
        .collect();
    fs::write(
        args.out.join("summary.json"),
        serde_json::to_vec_pretty(
            &json!({"games": args.games, "seconds": started.elapsed().as_secs_f64(), "vs": summary,
                    "batch": batches}),
        )?,
    )?;
    eprintln!(
        "[league] done in {:.0} s: {summary:?}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
