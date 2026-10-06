//! A load bench for the gateway, not a test: ignored, and run by hand.
//!
//! ```text
//! DATABASE_URL=… cargo test --release -p baylee-gateway --test load -- --ignored --nocapture
//! ```
//!
//! Always `--release`: the gateway under test is the binary this profile
//! builds, and a debug gateway measures the debug build. Local only, as every
//! test here: the gateway it starts binds a port of its own and keeps its
//! accounts in a schema of its own (`common::spawn_gateway_with`).
//!
//! The engine behind every game is a stand-in that answers in-band, not the
//! real one: the gateway forwards seat frames without decoding them, so the
//! routing can be loaded with bytes that carry their own send time, and what
//! is measured is the gateway rather than the rules. Four phases, each with
//! the gateway's own CPU time (`ps`) beside its wall time:
//!
//! - **lobby churn**: `LOAD_LOBBY` lobby feeds open while one player opens
//!   and leaves `LOAD_CHURN` rooms — every change is a listing per feed;
//! - **memory**: the gateway's resident set idle, per lobby feed, per game
//!   (two seat sockets and an engine link);
//! - **round trip**: every seat of `LOAD_GAMES` games sends `LOAD_ECHOES`
//!   frames at `LOAD_ECHO_HZ` that the engine echoes, seat → engine → seat;
//! - **fan-out**: every engine sends `LOAD_BLAST` frames of `LOAD_BYTES`
//!   bytes to each of its seats at once, as an engine does after an action.

#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

mod common;

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use baylee_protocol::v1::{self, Envelope};
use common::{Gateway, Socket, http, json_field};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use tokio_tungstenite::tungstenite::Message;

/// What every payload of this bench starts with: four bytes no encoded
/// `Envelope` can (a group end as the first tag), so a `SeatReady` the
/// harness sends is never taken for one.
const MAGIC: &[u8; 4] = b"LDB\x01";
const ECHO: u8 = b'E';
const BLAST: u8 = b'B';
const DATA: u8 = b'D';

fn knob(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Nanoseconds since the bench began; one clock for every task in it.
fn now_ns() -> u64 {
    static BASE: OnceLock<Instant> = OnceLock::new();
    BASE.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

/// A payload of `size` bytes (at least the header) saying `kind` and when
/// it left.
fn stamped(kind: u8, size: usize, extra: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(size.max(13 + extra.len()));
    out.extend_from_slice(MAGIC);
    out.push(kind);
    out.extend_from_slice(&now_ns().to_le_bytes());
    out.extend_from_slice(extra);
    out.resize(size.max(out.len()), 0);
    out
}

/// `(kind, sent_ns, rest)` of one of this bench's payloads.
fn read_stamp(data: &[u8]) -> Option<(u8, u64, &[u8])> {
    if data.len() < 13 || &data[..4] != MAGIC {
        return None;
    }
    let sent = u64::from_le_bytes(data[5..13].try_into().ok()?);
    Some((data[4], sent, &data[13..]))
}

// ------------------------------------------------------------ the process

fn ps(pid: u32, field: &str) -> String {
    let out = std::process::Command::new("ps")
        .args(["-o", &format!("{field}="), "-p", &pid.to_string()])
        .output()
        .expect("ps");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Resident set, KiB.
fn rss_kib(pid: u32) -> u64 {
    ps(pid, "rss").parse().unwrap_or(0)
}

/// User + system CPU seconds so far (`[[h:]m:]s[.cc]`).
fn cpu_secs(pid: u32) -> f64 {
    ps(pid, "time").split(':').fold(0.0, |acc, part| {
        acc * 60.0 + part.parse::<f64>().unwrap_or(0.0)
    })
}

struct Sample {
    wall: Instant,
    cpu: f64,
}

impl Sample {
    fn take(pid: u32) -> Self {
        Self {
            wall: Instant::now(),
            cpu: cpu_secs(pid),
        }
    }

    /// `(wall seconds, gateway CPU seconds)` since this sample.
    fn since(&self, pid: u32) -> (f64, f64) {
        (self.wall.elapsed().as_secs_f64(), cpu_secs(pid) - self.cpu)
    }
}

fn percentile(sorted: &[u64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let at = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[at] as f64 / 1000.0
}

fn latency_line(name: &str, mut ns: Vec<u64>) {
    ns.sort_unstable();
    println!(
        "  {name}: n={} p50={:.0}µs p90={:.0}µs p99={:.0}µs max={:.0}µs",
        ns.len(),
        percentile(&ns, 0.5),
        percentile(&ns, 0.9),
        percentile(&ns, 0.99),
        percentile(&ns, 1.0),
    );
}

// ------------------------------------------------------------- accounts

/// [`http`] from a different address each time: the gateway trusts
/// loopback as a proxy here, so the limiter keys each guest on its own.
fn http_from(
    port: u16,
    from: usize,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: &str,
) -> (u16, String) {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect http");
    let auth = token.map_or(String::new(), |t| format!("Authorization: Bearer {t}\r\n"));
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Forwarded-For: 10.{}.{}.{}\r\n\
         Content-Type: application/json\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        (from >> 16) & 0xff,
        (from >> 8) & 0xff,
        from & 0xff,
        body.len()
    );
    stream.write_all(request.as_bytes()).expect("write http");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read http");
    let status = raw
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status");
    (
        status,
        raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string(),
    )
}

/// A guest's session and a deck of its own.
fn player(port: u16, i: usize) -> (String, String) {
    let (status, body) = http_from(
        port,
        i,
        "POST",
        "/auth/guest",
        None,
        &format!("{{\"display_name\":\"load{i}\"}}"),
    );
    assert_eq!(status, 200, "guest {i}: {body}");
    let token = json_field(&body, "token").to_string();
    let deck = r#"{"name":"d","cards":["40 Island","20 Forest"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(&token), deck);
    assert_eq!(status, 200, "deck {i}: {body}");
    (token, json_field(&body, "deck_id").to_string())
}

// --------------------------------------------------- the stand-in engine

/// An agent whose engines answer in-band: an echo goes back to its seat, a
/// blast becomes that many frames to every seat of the game.
async fn stand_in_agent(gateway: &Gateway) -> tokio::task::JoinHandle<()> {
    let url = format!("ws://127.0.0.1:{}/agent/ws", gateway.port);
    let mut ws = common::dial(&url).await.expect("agent socket");
    common::send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
                token: gateway.agent_token.clone(),
                name: "load-agent".to_string(),
                capacity: 0,
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
            })),
        },
    )
    .await;
    assert!(matches!(
        common::next_msg(&mut ws).await,
        Some(v1::envelope::Msg::AgentWelcome(_))
    ));
    tokio::spawn(async move {
        while let Some(msg) = common::next_msg(&mut ws).await {
            if let v1::envelope::Msg::StartEngine(start) = msg {
                tokio::spawn(stand_in_engine(start));
            }
        }
    })
}

fn seat_frame(seat: u32, envelope: Vec<u8>) -> Message {
    let outer = Envelope {
        msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
            seat,
            envelope,
        })),
    };
    Message::Binary(outer.encode_to_vec().into())
}

async fn stand_in_engine(start: v1::StartEngine) {
    let Some(mut ws) = common::dial(&start.gateway_url).await else {
        return;
    };
    common::send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::EngineHello(v1::EngineHello {
                game_id: start.game_id.clone(),
                token: start.engine_token.clone(),
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
            })),
        },
    )
    .await;
    while let Some(msg) = common::next_msg(&mut ws).await {
        let v1::envelope::Msg::SeatFrame(frame) = msg else {
            continue;
        };
        let Some((kind, _, rest)) = read_stamp(&frame.envelope) else {
            continue;
        };
        match kind {
            ECHO => {
                let back = frame.envelope.clone();
                if ws.send(seat_frame(frame.seat, back)).await.is_err() {
                    return;
                }
            }
            BLAST if rest.len() >= 9 => {
                let count = u32::from_le_bytes(rest[0..4].try_into().unwrap());
                let size = u32::from_le_bytes(rest[4..8].try_into().unwrap()) as usize;
                let seats = u32::from(rest[8]);
                for _ in 0..count {
                    for seat in 0..seats {
                        let _ = ws.feed(seat_frame(seat, stamped(DATA, size, &[]))).await;
                    }
                }
                if ws.flush().await.is_err() {
                    return;
                }
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------------ the phases

/// A game of two guests, started, with both seat sockets open.
async fn open_game(port: u16, a: &(String, String), b: &(String, String)) -> [Socket; 2] {
    let create = format!("{{\"deck_id\":\"{}\",\"mode\":\"open\"}}", a.1);
    let (status, body) = http(port, "POST", "/lobby/games", Some(&a.0), &create);
    assert_eq!(status, 200, "create: {body}");
    let game = json_field(&body, "game_id").to_string();
    let seat_a = json_field(&body, "seat_token").to_string();
    let join = format!("{{\"deck_id\":\"{}\"}}", b.1);
    let path = format!("/lobby/games/{game}/join");
    let (status, body) = http(port, "POST", &path, Some(&b.0), &join);
    assert_eq!(status, 200, "join: {body}");
    let seat_b = json_field(&body, "seat_token").to_string();
    for token in [&a.0, &b.0] {
        let path = format!("/lobby/games/{game}/ready");
        let (status, body) = http(port, "POST", &path, Some(token), "{}");
        assert_eq!(status, 200, "ready: {body}");
    }
    let path = format!("/lobby/games/{game}/start");
    let (status, body) = http(port, "POST", &path, Some(&a.0), "");
    assert_eq!(status, 200, "start: {body}");
    [
        common::dial_seat(port, &game, &seat_a).await,
        common::dial_seat(port, &game, &seat_b).await,
    ]
}

/// The next payload of this bench on a socket, within `budget`.
async fn next_stamp(ws: &mut Socket, budget: Duration) -> Option<(u8, u64)> {
    let deadline = tokio::time::Instant::now() + budget;
    loop {
        let frame = tokio::time::timeout_at(deadline, ws.next())
            .await
            .ok()??
            .ok()?;
        if let Message::Binary(data) = frame
            && let Some((kind, sent, _)) = read_stamp(&data)
        {
            return Some((kind, sent));
        }
    }
}

/// Sends `count` echoes at `every`, and the latency of each that came back.
async fn echo_seat(mut ws: Socket, count: usize, every: Duration) -> (Socket, Vec<u64>) {
    let mut seen = Vec::with_capacity(count);
    let mut tick = tokio::time::interval(every);
    let mut sent = 0;
    while seen.len() < count {
        tokio::select! {
            _ = tick.tick(), if sent < count => {
                sent += 1;
                if ws.send(Message::Binary(stamped(ECHO, 64, &[]).into())).await.is_err() {
                    break;
                }
            }
            got = next_stamp(&mut ws, Duration::from_secs(5)) => match got {
                Some((ECHO, at)) => seen.push(now_ns() - at),
                Some(_) => {}
                None => break,
            }
        }
    }
    (ws, seen)
}

/// Receives `count` data frames, the latency of each.
async fn drain_seat(mut ws: Socket, count: usize) -> (Socket, Vec<u64>) {
    let mut seen = Vec::with_capacity(count);
    while seen.len() < count {
        match next_stamp(&mut ws, Duration::from_secs(5)).await {
            Some((DATA, at)) => seen.push(now_ns() - at),
            Some(_) => {}
            None => break,
        }
    }
    (ws, seen)
}

/// Opens and leaves `count` rooms as `churner`: two lobby changes each,
/// and the latency of every request.
fn churn_rooms(port: u16, churner: &(String, String), count: usize) -> Vec<u64> {
    let mut ops = Vec::with_capacity(2 * count);
    for _ in 0..count {
        let at = Instant::now();
        let body = format!("{{\"deck_id\":\"{}\",\"mode\":\"open\"}}", churner.1);
        let (status, body) = http(port, "POST", "/lobby/games", Some(&churner.0), &body);
        assert_eq!(status, 200, "create: {body}");
        ops.push(at.elapsed().as_nanos() as u64);
        let game = json_field(&body, "game_id").to_string();
        let at = Instant::now();
        let path = format!("/lobby/games/{game}/leave");
        let (status, body) = http(port, "POST", &path, Some(&churner.0), "");
        assert_eq!(status, 204, "leave: {body}");
        ops.push(at.elapsed().as_nanos() as u64);
    }
    ops
}

/// Every seat echoes at once; the seats back, and every round trip.
async fn round_trip(
    pid: u32,
    seats: Vec<Socket>,
    echoes: usize,
    every: Duration,
    name: &str,
) -> (Vec<Socket>, Vec<u64>) {
    let sample = Sample::take(pid);
    let runs: Vec<_> = seats
        .into_iter()
        .map(|ws| tokio::spawn(echo_seat(ws, echoes, every)))
        .collect();
    let mut seats = Vec::with_capacity(runs.len());
    let mut rtt = Vec::new();
    for run in runs {
        let (ws, seen) = run.await.expect("echo seat");
        seats.push(ws);
        rtt.extend(seen);
    }
    let (wall, cpu) = sample.since(pid);
    println!(
        "{name}: {} echoes over {} seats in {wall:.2}s, gateway CPU {cpu:.3}s = {:.1}µs per forwarded frame",
        rtt.len(),
        seats.len(),
        cpu * 1e6 / (rtt.len() * 2).max(1) as f64
    );
    (seats, rtt)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "a load bench, run by hand in --release"]
#[allow(clippy::too_many_lines)] // four phases, read top to bottom
async fn gateway_under_load() {
    let games = knob("LOAD_GAMES", 50);
    let lobby_feeds = knob("LOAD_LOBBY", 200);
    let churn = knob("LOAD_CHURN", 50);
    let echoes = knob("LOAD_ECHOES", 200);
    let echo_hz = knob("LOAD_ECHO_HZ", 100);
    let blast = knob("LOAD_BLAST", 100);
    let bytes = knob("LOAD_BYTES", 16 * 1024);

    let gateway = common::spawn_gateway_with(
        "load",
        &[
            ("BAYLEE_TRUSTED_PROXIES", "127.0.0.1".to_string()),
            ("BAYLEE_DB_POOL", "8".to_string()),
            ("BAYLEE_GUEST_CAP", "0".to_string()),
        ],
    );
    let port = gateway.port;
    let pid = gateway.pid();
    let _agent = stand_in_agent(&gateway).await;
    println!(
        "LOAD games={games} lobby_feeds={lobby_feeds} churn={churn} echoes={echoes}@{echo_hz}Hz blast={blast}x{bytes}B"
    );
    let rss_idle = rss_kib(pid);
    println!("memory: idle {rss_idle} KiB");

    // Accounts: the feeds' and the games' players, and one who churns.
    let started = Instant::now();
    let players: Vec<_> = (0..=lobby_feeds.max(2 * games))
        .map(|i| player(port, i))
        .collect();
    println!(
        "accounts: {} guests with a deck each in {:.2}s",
        players.len(),
        started.elapsed().as_secs_f64()
    );

    // ---- lobby churn
    let mut feeds = Vec::with_capacity(lobby_feeds);
    for (token, _) in players.iter().take(lobby_feeds) {
        let url = common::lobby_url(port, token, "");
        feeds.push(common::dial(&url).await.expect("lobby feed"));
    }
    let rss_feeds = rss_kib(pid);
    println!(
        "memory: {lobby_feeds} lobby feeds +{} KiB ({:.1} KiB each)",
        rss_feeds.saturating_sub(rss_idle),
        rss_feeds.saturating_sub(rss_idle) as f64 / lobby_feeds.max(1) as f64
    );
    let counted = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let readers: Vec<_> = feeds
        .into_iter()
        .map(|mut ws| {
            let counted = std::sync::Arc::clone(&counted);
            tokio::spawn(async move {
                while let Ok(Some(Ok(_))) =
                    tokio::time::timeout(Duration::from_secs(60), ws.next()).await
                {
                    counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            })
        })
        .collect();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let opening = counted.load(std::sync::atomic::Ordering::Relaxed);
    let churner = players.last().expect("a churner").clone();
    let sample = Sample::take(pid);
    let churned = {
        let churner = churner.clone();
        tokio::task::spawn_blocking(move || churn_rooms(port, &churner, churn))
            .await
            .expect("churn")
    };
    let (http_wall, _) = sample.since(pid);
    // Drained when nothing has arrived for half a second.
    let mut last = usize::MAX;
    loop {
        let now = counted.load(std::sync::atomic::Ordering::Relaxed);
        if now == last {
            break;
        }
        last = now;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let (wall, cpu) = sample.since(pid);
    let pushed = last - opening;
    println!(
        "lobby churn: {} changes, {lobby_feeds} feeds: requests {:.2}s, drained {:.2}s (-0.5s idle), \
         gateway CPU {:.3}s = {:.2}ms/change, {pushed} listings pushed ({:.1}/feed, {:.0}µs CPU each)",
        2 * churn,
        http_wall,
        wall - 0.5,
        cpu,
        cpu * 1000.0 / (2 * churn) as f64,
        pushed as f64 / lobby_feeds.max(1) as f64,
        cpu * 1e6 / pushed.max(1) as f64,
    );
    latency_line("create/leave request", churned);

    // ---- games
    let rss_before_games = rss_kib(pid);
    let started = Instant::now();
    let mut seats = Vec::with_capacity(2 * games);
    for g in 0..games {
        let [a, b] = open_game(port, &players[2 * g], &players[2 * g + 1]).await;
        seats.push(a);
        seats.push(b);
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    let rss_games = rss_kib(pid);
    println!(
        "games: {games} opened (create, join, 2x ready, start, 2 seat sockets) in {:.2}s; \
         memory +{} KiB ({:.1} KiB per game)",
        started.elapsed().as_secs_f64(),
        rss_games.saturating_sub(rss_before_games),
        rss_games.saturating_sub(rss_before_games) as f64 / games.max(1) as f64
    );

    // ---- round trip, quiet and beside lobby churn
    let every = Duration::from_secs_f64(1.0 / echo_hz.max(1) as f64);
    let (seats, rtt) = round_trip(pid, seats, echoes, every, "round trip").await;
    latency_line("seat → engine → seat", rtt);
    let churning = {
        let churner = churner.clone();
        tokio::task::spawn_blocking(move || churn_rooms(port, &churner, churn))
    };
    let (mut seats, rtt) = round_trip(pid, seats, echoes, every, "round trip beside churn").await;
    latency_line("seat → engine → seat", rtt);
    latency_line("create/leave request", churning.await.expect("churn"));

    // ---- fan-out
    let sample = Sample::take(pid);
    let mut order = Vec::with_capacity(blast * 2);
    order.extend_from_slice(&u32::try_from(blast).unwrap().to_le_bytes());
    order.extend_from_slice(&u32::try_from(bytes).unwrap().to_le_bytes());
    order.push(2);
    for pair in seats.chunks_mut(2) {
        pair[0]
            .send(Message::Binary(stamped(BLAST, 0, &order).into()))
            .await
            .expect("blast order");
    }
    let runs: Vec<_> = seats
        .into_iter()
        .map(|ws| tokio::spawn(drain_seat(ws, blast)))
        .collect();
    let mut delivered = Vec::new();
    let mut seats = Vec::with_capacity(runs.len());
    for run in runs {
        let (ws, seen) = run.await.expect("drain seat");
        seats.push(ws);
        delivered.extend(seen);
    }
    let (wall, cpu) = sample.since(pid);
    let expected = blast * seats.len();
    println!(
        "fan-out: {}/{} frames of {bytes} B delivered in {wall:.2}s = {:.0} frames/s, {:.1} MB/s; \
         gateway CPU {cpu:.3}s = {:.1}µs per frame",
        delivered.len(),
        expected,
        delivered.len() as f64 / wall,
        (delivered.len() * bytes) as f64 / wall / 1e6,
        cpu * 1e6 / delivered.len().max(1) as f64
    );
    latency_line("engine → seat", delivered);
    println!("memory: peak-ish {} KiB", rss_kib(pid));
    drop(seats);
    for reader in readers {
        reader.abort();
    }
}
