//! A load test of the house's search at hosted tables: so many engine
//! processes at once, each started as an agent starts one (the same slots
//! directory, the operator's `BAYLEE_FORESIGHT_*` settings), each playing one
//! table of a searching level against a seat this test plays over the engine
//! link, as a gateway would forward a player's socket. It reports what the
//! engines cost in CPU and memory and how long the slowest answers took.
//!
//! Ignored: it measures the machine it runs on, and a whole round of games
//! takes minutes. Run it on the machine to be measured, with a release build:
//!
//! ```text
//! BAYLEE_LOAD_TABLES=8 BAYLEE_LOAD_ROUNDS=2 BAYLEE_FORESIGHT_SEARCHES=4 \
//!   cargo test --release -p baylee-engine-server --test foresight_load -- --ignored --nocapture
//! ```
//!
//! - `BAYLEE_LOAD_TABLES` (default 4): tables at once.
//! - `BAYLEE_LOAD_ROUNDS` (default 1): rounds of them, one after another.
//! - `BAYLEE_LOAD_LEVEL` (default `expert`): the house's level, by its name.
//! - `BAYLEE_LOAD_SEAT` (default `steady`): the level the seat this test
//!   plays answers at, from its own view, as a seat bridge's house mind does.
//! - `BAYLEE_LOAD_SECS` (default 900): the longest one table may take.
//! - `BAYLEE_FORESIGHT*`: handed to every engine as an agent would.
//!
//! It prints one JSON line (`load: {…}`) and a summary. Every engine it starts
//! it stops on the way out, however the test ends.

// A report of measurements: counts and times go into floats for it.
#![allow(clippy::cast_precision_loss)]

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

use std::collections::BTreeMap;
use std::io::BufRead as _;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use tokio_tungstenite::tungstenite::Message;

/// The seat this test plays: the second chair, as a person's.
const SEAT: u32 = 1;

/// The decks the tables are dealt, in pairs: the house's own.
const PAIRS: [(&str, &str); 4] = [
    ("allytifact", "victory"),
    ("maik", "schwarzrand"),
    ("weltenbaum", "victory"),
    ("schwarzrand", "allytifact"),
];

/// A number from the environment, or `default`.
fn setting(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

/// An engine process, stopped and reaped when dropped, whatever happened.
struct Engine {
    child: std::process::Child,
    /// Every line it wrote, read on a thread of its own.
    lines: Arc<Mutex<Vec<String>>>,
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Engine {
    /// Starts an engine for `game` dialling `url`, as an agent does: the
    /// game's token in the environment, the slots directory, and the
    /// operator's foresight settings it inherits.
    fn start(url: &str, game: &str, token: &str, slots: &std::path::Path) -> Self {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-engine-server"));
        command
            .args(["--attach", url, "--game", game])
            .env("BAYLEE_ENGINE_TOKEN", token)
            .env("BAYLEE_FORESIGHT_SLOTS", slots)
            .env("NO_COLOR", "1")
            .env(
                "RUST_LOG",
                "baylee_engine_server=info,baylee_gamehost=debug,warn",
            )
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = command.spawn().expect("the engine starts");
        let lines = Arc::new(Mutex::new(Vec::new()));
        for out in [
            child
                .stdout
                .take()
                .map(|o| Box::new(o) as Box<dyn std::io::Read + Send>),
            child
                .stderr
                .take()
                .map(|e| Box::new(e) as Box<dyn std::io::Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let lines = Arc::clone(&lines);
            std::thread::spawn(move || {
                for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                    lines.lock().expect("lines").push(line);
                }
            });
        }
        Self { child, lines }
    }
}

/// The value of `key=` in a log line, without its quotes and any colour.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let at = line.find(&format!(" {key}="))? + key.len() + 2;
    let rest = &line[at..];
    let end = rest.find(' ').unwrap_or(rest.len());
    Some(rest[..end].trim_matches('"'))
}

/// One searched answer as the engine logged it.
#[derive(Clone, Copy, Debug, Default)]
struct Searched {
    micros: u64,
    work: u64,
    walled: bool,
}

/// What one table did and cost.
#[derive(Clone, Debug, Default)]
struct Table {
    game: String,
    pairing: String,
    finished: bool,
    turn: u32,
    house_won: Option<bool>,
    /// The seat's questions, and the longest it waited for one after its
    /// last answer: the house's whole turn, searches and all.
    questions: u64,
    longest_wait_ms: u64,
    /// The engine's CPU time and peak memory, read from `/proc`.
    cpu_ms: u64,
    peak_kib: u64,
    searches: Vec<Searched>,
    /// The engine's own count at the end (`house searches`).
    summary: BTreeMap<String, String>,
}

/// `/proc/<pid>/stat`'s user and system time, in milliseconds, and
/// `/proc/<pid>/status`'s `VmHWM` in KiB; zero where it cannot be read.
fn usage(pid: u32) -> (u64, u64) {
    let ticks = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            // The command name may hold spaces: count from its closing paren.
            let rest = &stat[stat.rfind(')')? + 2..];
            let fields: Vec<&str> = rest.split(' ').collect();
            let utime: u64 = fields.get(11)?.parse().ok()?;
            let stime: u64 = fields.get(12)?.parse().ok()?;
            Some(utime + stime)
        })
        .unwrap_or(0);
    let peak = std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find_map(|line| line.strip_prefix("VmHWM:"))
                .and_then(|kib| kib.trim().trim_end_matches("kB").trim().parse().ok())
        })
        .unwrap_or(0);
    // `CLK_TCK` is 100 on every Linux this runs on.
    (ticks * 10, peak)
}

/// The preset a gateway builds for one table: the house at `level` in the
/// first chair, by its name, and a person's chair.
fn preset(seed: u64, pairing: (&str, &str), level: &str) -> GamePreset {
    let deck = |name: &str| {
        let path = format!("{}/../../data/decks/{name}.txt", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        baylee_cards::decks::house_deck(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
    };
    let (a, b) = (deck(pairing.0), deck(pairing.1));
    let mut preset = baylee_cards::decks::preset_for_all(seed, &[&a, &b]);
    preset.seats[0].controller =
        SeatController::Ai(AIProfile::named(level).expect("a level by its name"));
    preset.seats[1].controller = SeatController::Open;
    preset.house_rules.decision_timeout_secs = 0;
    preset
}

fn frame(seat: u32, inner: &Envelope) -> Message {
    let outer = Envelope {
        msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
            seat,
            envelope: inner.encode_to_vec().into(),
        })),
    };
    Message::Binary(outer.encode_to_vec().into())
}

/// Plays one table on an engine's socket, as a gateway forwarding a
/// player's: the game set up, the seat attached and ready, and every question
/// it is asked answered by `mind` from the seat's view.
async fn serve<S>(
    mut ws: tokio_tungstenite::WebSocketStream<S>,
    game: String,
    preset: GamePreset,
    mind: baylee_ai::HeuristicAgent,
    pid: u32,
) -> Table
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut table = Table {
        game: game.clone(),
        ..Table::default()
    };
    let setup = Envelope {
        msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
            game_id: game.clone(),
            preset_json: serde_json::to_vec(&preset).expect("a preset"),
            seat_names: vec!["House".into(), "Load".into()],
            seat_guests: Vec::new(),
        })),
    };
    let attached = Envelope {
        msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
            seat: SEAT,
            resync: false,
        })),
    };
    let ready = Envelope {
        msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
    };
    for message in [
        Message::Binary(setup.encode_to_vec().into()),
        Message::Binary(attached.encode_to_vec().into()),
        frame(SEAT, &ready),
    ] {
        if ws.send(message).await.is_err() {
            return table;
        }
    }
    let mut reader = baylee_view::ViewReader::default();
    let mut asked_since = Instant::now();
    while let Some(Ok(message)) = ws.next().await {
        let Message::Binary(bytes) = message else {
            continue;
        };
        let Ok(envelope) = Envelope::decode(bytes) else {
            continue;
        };
        match envelope.msg {
            Some(v1::envelope::Msg::GameEnded(ended)) => {
                table.finished = true;
                table.turn = ended.turn;
                table.house_won = Some(ended.winners.contains(&0));
                (table.cpu_ms, table.peak_kib) = usage(pid);
                break;
            }
            Some(v1::envelope::Msg::SeatFrame(seat)) if seat.seat == SEAT => {
                let Ok(inner) = Envelope::decode(&seat.envelope[..]) else {
                    continue;
                };
                match inner.msg {
                    Some(v1::envelope::Msg::StateDelta(delta)) => {
                        let _ = reader.read(&delta.view_frame);
                    }
                    Some(v1::envelope::Msg::ChoiceRequest(request)) => {
                        let waited = asked_since.elapsed();
                        table.longest_wait_ms = table
                            .longest_wait_ms
                            .max(u64::try_from(waited.as_millis()).unwrap_or(u64::MAX));
                        table.questions += 1;
                        let Ok(pending) = baylee_core::wire::decode::<Pending>(&request.pending)
                        else {
                            continue;
                        };
                        if matches!(pending, Pending::GameOver(_)) {
                            continue;
                        }
                        let action = match reader.read(&[]) {
                            Ok(Some(view)) => mind.act(view, &pending),
                            _ => timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority),
                        };
                        let act = Envelope {
                            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                                game_id: game.clone(),
                                seat_token: String::new(),
                                action: baylee_core::wire::encode(&action),
                            })),
                        };
                        asked_since = Instant::now();
                        if ws.send(frame(SEAT, &act)).await.is_err() {
                            break;
                        }
                        (table.cpu_ms, table.peak_kib) = usage(pid);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    table
}

/// The `q`-quantile of sorted `values`, nearest rank.
fn quantile(values: &[u64], q: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }
    #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
    let at = ((values.len() - 1) as f64 * q).round() as usize;
    values[at.min(values.len() - 1)]
}

/// What a run is asked to do, from the environment.
struct Run {
    tables: usize,
    rounds: u64,
    level: String,
    seat: AIProfile,
    seat_level: String,
    limit: Duration,
}

impl Run {
    fn from_env() -> Self {
        let seat_level = std::env::var("BAYLEE_LOAD_SEAT").unwrap_or_else(|_| "steady".into());
        Self {
            tables: usize::try_from(setting("BAYLEE_LOAD_TABLES", 4))
                .unwrap_or(4)
                .max(1),
            rounds: setting("BAYLEE_LOAD_ROUNDS", 1).max(1),
            level: std::env::var("BAYLEE_LOAD_LEVEL").unwrap_or_else(|_| "expert".into()),
            seat: AIProfile::named(&seat_level).expect("the seat's level by its name"),
            seat_level,
            limit: Duration::from_secs(setting("BAYLEE_LOAD_SECS", 900)),
        }
    }
}

/// Plays `run`'s rounds: in each, its tables at once, one engine each, every
/// engine dialling this test's loopback listener as it would a gateway's.
/// The tables, with what each engine logged.
async fn play(run: &Run, slots: &std::path::Path) -> Vec<(Table, Vec<String>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a loopback port");
    let url = format!(
        "ws://{}/engine/ws",
        listener.local_addr().expect("an address")
    );
    let mut played = Vec::new();
    for round in 0..run.rounds {
        let mut waiting = BTreeMap::new();
        let mut engines = BTreeMap::new();
        for at in 0..run.tables as u64 {
            let number = round * run.tables as u64 + at;
            let game = format!("load-{number}");
            let token = format!("token-{number}");
            let pairing = PAIRS[usize::try_from(number).unwrap_or(0) % PAIRS.len()];
            let preset = preset(1_000 + number, pairing, &run.level);
            engines.insert(game.clone(), Engine::start(&url, &game, &token, slots));
            waiting.insert(
                game,
                (token, preset, format!("{} v {}", pairing.0, pairing.1)),
            );
        }
        let mut serving = tokio::task::JoinSet::new();
        while !waiting.is_empty() {
            let accepted = tokio::time::timeout(Duration::from_secs(60), listener.accept()).await;
            let Ok(Ok((stream, _))) = accepted else {
                panic!(
                    "an engine never dialled: {:?}",
                    waiting.keys().collect::<Vec<_>>()
                );
            };
            let mut ws = tokio_tungstenite::accept_async(stream)
                .await
                .expect("a websocket");
            let Some(Ok(Message::Binary(hello))) = ws.next().await else {
                panic!("no hello");
            };
            let Some(v1::envelope::Msg::EngineHello(hello)) =
                Envelope::decode(hello).expect("an envelope").msg
            else {
                panic!("the first frame is no hello");
            };
            let (token, preset, pairing) = waiting
                .remove(&hello.game_id)
                .expect("an engine this test started");
            assert_eq!(hello.token, token, "the engine proves its game");
            let mind = baylee_ai::HeuristicAgent::new(run.seat)
                .with_seed(baylee_ai::policy_seed(&hello.game_id, 1));
            let limit = run.limit;
            serving.spawn(async move {
                let game = hello.game_id;
                let mut table =
                    tokio::time::timeout(limit, serve(ws, game.clone(), preset, mind, hello.pid))
                        .await
                        .unwrap_or_else(|_| Table {
                            game,
                            ..Table::default()
                        });
                table.pairing = pairing;
                table
            });
        }
        let mut tables = Vec::new();
        while let Some(table) = serving.join_next().await {
            tables.push(table.expect("a table task"));
        }
        // Each engine writes its last lines and exits after its game; then
        // it is stopped whatever it is doing, and what it wrote is read.
        tokio::time::sleep(Duration::from_millis(500)).await;
        for table in tables {
            let lines = engines.remove(&table.game).map_or_else(Vec::new, |engine| {
                let lines = Arc::clone(&engine.lines);
                drop(engine);
                lines.lock().expect("lines").clone()
            });
            played.push((table, lines));
        }
    }
    played
}

/// Reads an engine's log into its table: each searched answer (`baylee_gamehost`
/// at debug) and the engine's count at the end of the game.
fn read_log(table: &mut Table, lines: &[String]) {
    for line in lines {
        if line.contains(": house search ") {
            table.searches.push(Searched {
                micros: field(line, "us").and_then(|v| v.parse().ok()).unwrap_or(0),
                work: field(line, "work")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0),
                walled: field(line, "walled") == Some("true"),
            });
        }
        if line.contains(": house searches ") {
            for key in [
                "asked",
                "searched",
                "replaced",
                "refused",
                "busy",
                "walled",
                "slowest_ms",
            ] {
                if let Some(value) = field(line, key) {
                    table.summary.insert(key.into(), value.into());
                }
            }
        }
    }
}

/// The run's report: one JSON line and a sentence.
fn report(run: &Run, played: &[Table], wall: Duration) {
    let mut micros: Vec<u64> = played
        .iter()
        .flat_map(|table| table.searches.iter().map(|s| s.micros))
        .collect();
    micros.sort_unstable();
    let ms = |q: f64| quantile(&micros, q) as f64 / 1000.0;
    let count = |key: &str| -> u64 {
        played
            .iter()
            .filter_map(|table| table.summary.get(key)?.parse::<u64>().ok())
            .sum()
    };
    let cpu_s = played.iter().map(|table| table.cpu_ms).sum::<u64>() as f64 / 1000.0;
    let games = played.len().max(1) as f64;
    let finished = played.iter().filter(|table| table.finished).count();
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let busy = cpu_s / (wall.as_secs_f64().max(0.001) * cores as f64);
    let longest_wait = played.iter().map(|t| t.longest_wait_ms).max().unwrap_or(0);
    let report = serde_json::json!({
        "tables_at_once": run.tables,
        "rounds": run.rounds,
        "level": run.level,
        "seat": run.seat_level,
        "cores": cores,
        "searches_setting": std::env::var("BAYLEE_FORESIGHT_SEARCHES").ok(),
        "wall_ms_setting": std::env::var("BAYLEE_FORESIGHT_WALL_MS").ok(),
        "work_setting": std::env::var("BAYLEE_FORESIGHT_WORK").ok(),
        "switch": std::env::var("BAYLEE_FORESIGHT").ok(),
        "games": played.len(),
        "finished": finished,
        "house_won": played.iter().filter(|t| t.house_won == Some(true)).count(),
        "wall_s": wall.as_secs_f64(),
        "engine_cpu_s": cpu_s,
        "cpu_s_per_game": cpu_s / games,
        "share_of_the_cores_busy": busy,
        "peak_rss_mib_max": played.iter().map(|t| t.peak_kib).max().unwrap_or(0) / 1024,
        "searches": {
            "asked": count("asked"),
            "searched": count("searched"),
            "replaced": count("replaced"),
            "refused": count("refused"),
            "busy": count("busy"),
            "walled": count("walled"),
            "logged": micros.len(),
            "ms_p50": ms(0.5),
            "ms_p95": ms(0.95),
            "ms_p99": ms(0.99),
            "ms_max": ms(1.0),
            "most_work": played.iter().flat_map(|t| t.searches.iter().map(|s| s.work)).max().unwrap_or(0),
        },
        "seat_longest_wait_ms": longest_wait,
        "tables": played.iter().map(|t| serde_json::json!({
            "game": t.game,
            "decks": t.pairing,
            "finished": t.finished,
            "turn": t.turn,
            "house_won": t.house_won,
            "questions": t.questions,
            "longest_wait_ms": t.longest_wait_ms,
            "cpu_s": t.cpu_ms as f64 / 1000.0,
            "peak_rss_mib": t.peak_kib / 1024,
            "searched": t.searches.len(),
            "walled": t.searches.iter().filter(|s| s.walled).count(),
            "slowest_ms": t.searches.iter().map(|s| s.micros).max().unwrap_or(0) as f64 / 1000.0,
        })).collect::<Vec<_>>(),
    });
    println!("load: {report}");
    println!(
        "{} tables at once, {} games ({finished} finished) in {:.0} s: engines {cpu_s:.0} CPU-s \
         ({:.1} a game), {:.0} % of {cores} cores; searches {} (busy {}, walled {}), ms p50 {:.0} \
         p99 {:.0} max {:.0}; the seat's longest wait {longest_wait} ms",
        run.tables,
        played.len(),
        wall.as_secs_f64(),
        cpu_s / games,
        busy * 100.0,
        count("searched"),
        count("busy"),
        count("walled"),
        ms(0.5),
        ms(0.99),
        ms(1.0),
    );
}

#[test]
#[ignore = "a load test: run by hand on the machine it measures (see the module's comment)"]
fn hosted_tables_of_a_searching_level() {
    let run = Run::from_env();
    // The directory an agent makes for its engines: only its user enters it.
    let slots = {
        let mut builder = tempfile::Builder::new();
        builder.prefix("baylee-load-slots-");
        #[cfg(unix)]
        builder.permissions(
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o700),
        );
        builder.tempdir().expect("a slots directory")
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime");
    let started = Instant::now();
    let played = runtime.block_on(play(&run, slots.path()));
    let wall = started.elapsed();
    let played: Vec<Table> = played
        .into_iter()
        .map(|(mut table, lines)| {
            read_log(&mut table, &lines);
            table
        })
        .collect();
    report(&run, &played, wall);
    assert_eq!(
        played.len(),
        run.tables * usize::try_from(run.rounds).unwrap_or(1)
    );
    assert!(played.iter().any(|t| t.finished), "no table finished");
}
