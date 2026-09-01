//! baylee-engine-server — one process per game, websocket transport.
//!
//! v1 protocol: protobuf `Envelope` framing over binary websocket frames;
//! complex engine structures (`Pending`, `PlayerAction`) travel as
//! `serde_json` payloads inside `ChoiceRequest` / `PlayerActionMsg`.
//!
//! Two ways in, and the game logic behind both is the same:
//!
//! - **Attached** (`--attach ws://… --game <id> --token <tok>`): the real
//!   one. An agent started this process for one game; it dials the gateway,
//!   proves it was asked for, and plays that game until it ends. Everything
//!   it does goes through [`baylee_engine_server::EngineRunner`].
//! - **Listening** (no arguments): a dev harness on loopback with no
//!   authentication at all, kept because it is the shortest way to poke the
//!   engine over the wire by hand.

use baylee_core::preset::GamePreset;
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::{Session, preset};
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message;

/// Default port (dev).
const DEFAULT_PORT: u16 = 28765;

/// The one human seat this dev harness serves.
///
/// There is no authentication here: every action runs as seat 0, which is
/// exactly why [`DEFAULT_BIND`] is loopback.
const SEAT: baylee_core::ids::PlayerId = baylee_core::ids::PlayerId::new(0);

/// Seat names for the dev duel's roster.
fn seat_names() -> Vec<String> {
    vec!["You".to_string(), "House AI".to_string()]
}

/// Default bind address: loopback only. This is a dev harness with no
/// authentication — `Join` hands out full seat-0 views and every action
/// runs as seat 0 — so it must not be reachable from the network unless
/// the operator explicitly says so (`BAYLEE_BIND=0.0.0.0`).
const DEFAULT_BIND: &str = "127.0.0.1";

#[tokio::main]
async fn main() {
    tracing_subscriber_init();
    if let Some(attach) = Attach::discover() {
        let game_id = attach.game_id.clone();
        match attached::run(attach).await {
            Ok(()) => tracing::info!(game_id, "game finished"),
            Err(err) => {
                tracing::error!(game_id, %err, "engine link failed");
                std::process::exit(1);
            }
        }
        return;
    }
    listen().await;
}

/// Where an attached engine dials, and what it proves it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attach {
    /// The gateway's engine socket (`ws://…/engine/ws`).
    pub url: String,
    /// The game this process was started for.
    pub game_id: String,
    /// One game's worth of authority, issued by the gateway and handed over
    /// by the agent. It never leaves the machine the agent runs on.
    pub token: String,
}

impl Attach {
    /// The attachment this launch was given, if it was given one.
    ///
    /// Command line first (that is how the agent starts it), environment
    /// second (that is how a person starts it by hand).
    fn discover() -> Option<Self> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let flag = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        let env = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        let url = flag("--attach").or_else(|| env("BAYLEE_ATTACH_URL"))?;
        let game_id = flag("--game").or_else(|| env("BAYLEE_GAME"))?;
        let token = flag("--token").or_else(|| env("BAYLEE_ENGINE_TOKEN"))?;
        Some(Self {
            url,
            game_id,
            token,
        })
    }
}

/// The dev harness: listen on loopback and serve whoever connects.
async fn listen() {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let bind = std::env::var("BAYLEE_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_string());
    let listener = tokio::net::TcpListener::bind((bind.as_str(), port))
        .await
        .expect("bind websocket port");
    tracing::info!(port, "baylee-engine-server listening");
    let games: Games =
        std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    while let Ok((stream, peer)) = listener.accept().await {
        tracing::info!(%peer, "client connected");
        let games = games.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, games).await {
                tracing::warn!(%peer, %err, "connection closed with error");
            }
        });
    }
}

fn tracing_subscriber_init() {
    // Without a subscriber every `tracing::` call is a silent no-op.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

/// All live games on this server (v1: process-local; federation is a
/// gateway milestone).
type Games = std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, Session>>>;

/// Transport limits.
///
/// tungstenite defaults to a 64 MiB message, which is three orders of
/// magnitude more than any message this protocol has: the largest is a
/// `CreateGame` carrying eight seats of at most
/// [`baylee_core::preset::MAX_CARDS_PER_SEAT`] entries plus a print table,
/// well under 1 MiB. Decoding happens *before* the preset can be
/// validated, so the frame budget is the only thing standing between an
/// unauthenticated peer and an arbitrary allocation — it belongs here, not
/// in the message handler.
fn ws_config() -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
    tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(4 << 20))
        .max_frame_size(Some(4 << 20))
}

#[allow(clippy::too_many_lines)]
async fn handle_connection(
    stream: tokio::net::TcpStream,
    games: Games,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut ws = tokio_tungstenite::accept_async_with_config(stream, Some(ws_config())).await?;
    let mut game_id: Option<String> = None;
    while let Some(frame) = ws.next().await {
        let frame = frame?;
        if !frame.is_binary() {
            continue;
        }
        let envelope = Envelope::decode(frame.into_data())?;
        let Some(msg) = envelope.msg else {
            continue;
        };
        let replies: Vec<Envelope> = match msg {
            v1::envelope::Msg::CreateGame(create) => {
                let preset = match create.preset {
                    Some(msg) => match preset::from_proto(&msg) {
                        Ok(p) => p,
                        Err(reason) => {
                            ws.send(tokio_tungstenite::tungstenite::Message::Binary(
                                error(&format!("bad preset: {reason}"))
                                    .encode_to_vec()
                                    .into(),
                            ))
                            .await?;
                            continue;
                        }
                    },
                    // No preset: the dev acceptance duel.
                    None => acceptance_duel_preset(),
                };
                let id = uuid::Uuid::now_v7().to_string();
                let mut out = Vec::new();
                match Session::new(&preset) {
                    Some(session) => {
                        out.push(Envelope {
                            msg: Some(v1::envelope::Msg::GameCreated(v1::GameCreated {
                                game_id: id.clone(),
                                your_seat: 0,
                            })),
                        });
                        let mut games = games.lock().await;
                        games.insert(id.clone(), session);
                        let session = games.get_mut(&id).expect("just inserted");
                        // The seat roster and the print table come first: a
                        // client has no preset to build them from, and every
                        // view after this one refers to them.
                        session.describe(id.clone(), seat_names());
                        out.push(session.game_static_envelope(SEAT));
                        // Dev harness: one human seat — route everything
                        // addressed to it to this connection.
                        out.extend(session.pump().into_iter().map(|(_, env)| env));
                        game_id = Some(id);
                    }
                    None => out.push(error("could not start the game")),
                }
                out
            }
            v1::envelope::Msg::Join(join) => {
                // Re-attach to a live game. Note this *pumps*: joining is
                // how a fresh client starts playing. A reconnect that only
                // wants its state back sends `ResumeGame` instead.
                let mut games = games.lock().await;
                match games.get_mut(&join.game_id) {
                    Some(session) => {
                        game_id = Some(join.game_id.clone());
                        session.describe(join.game_id.clone(), seat_names());
                        let mut out = vec![session.game_static_envelope(SEAT)];
                        out.extend(session.pump().into_iter().map(|(_, env)| env));
                        out
                    }
                    None => vec![error("no such game")],
                }
            }
            v1::envelope::Msg::Resume(resume) => {
                // Read-only reconnect: never pump here, or resuming would play
                // the AI seats forward as a side effect of coming back.
                let games = games.lock().await;
                match games.get(&resume.game_id) {
                    Some(session) => {
                        // A resume follows a dropped socket, so the payload
                        // that only ever arrives at connect time is repeated.
                        // It cannot have changed, and a client that still has
                        // it simply overwrites it with the same thing.
                        let mut out = vec![session.game_static_envelope(SEAT)];
                        out.extend(session.resume(SEAT, resume.last_seq));
                        drop(games);
                        game_id = Some(resume.game_id.clone());
                        out
                    }
                    None => vec![error("no such game")],
                }
            }
            v1::envelope::Msg::PlayerAction(action_msg) => {
                let Some(id) = game_id.as_ref() else {
                    continue;
                };
                let Ok(action) = serde_json::from_slice::<PlayerAction>(&action_msg.action_json)
                else {
                    continue;
                };
                let mut games = games.lock().await;
                match games.get_mut(id) {
                    Some(session) => session
                        .act(baylee_core::ids::PlayerId::new(0), action)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(_, env)| env)
                        .collect(),
                    None => vec![error("no such game")],
                }
            }
            v1::envelope::Msg::Heartbeat(_) => {
                vec![Envelope {
                    msg: Some(v1::envelope::Msg::Heartbeat(v1::Heartbeat {
                        client_time_ms: now_ms(),
                    })),
                }]
            }
            _ => vec![],
        };
        for reply in replies {
            ws.send(tokio_tungstenite::tungstenite::Message::Binary(
                reply.encode_to_vec().into(),
            ))
            .await?;
        }
    }
    Ok(())
}

fn error(message: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::Error(v1::Error {
            code: 1,
            message: message.to_string(),
        })),
    }
}

fn now_ms() -> u64 {
    // Wall-clock ms for the heartbeat echo only; never enters the engine.
    0
}

/// The dev duel: Allytifact vs Victory, human on seat 0, AI on seat 1.
fn acceptance_duel_preset() -> GamePreset {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/acceptance-decks.txt"),
    )
    .expect("acceptance deck file");
    let allytifact = baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact");
    let victory = baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory");
    let mut preset = baylee_cards::decks::preset_for(1, &allytifact, &victory);
    // Seat 0 is the connecting human.
    preset.seats[0].controller = baylee_core::preset::SeatController::Open;
    preset
}

/// The attached engine: one socket to the gateway, one game, then exit.
mod attached {
    use super::{Attach, ws_config};
    use baylee_engine_server::EngineRunner;
    use baylee_protocol::v1::{self, Envelope};
    use futures_util::{SinkExt, StreamExt};
    use prost::Message as _;
    use tokio_tungstenite::tungstenite::Message;

    /// Plays one game against the gateway and returns when it ends.
    pub async fn run(attach: Attach) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (mut ws, _) =
            tokio_tungstenite::connect_async_with_config(&attach.url, Some(ws_config()), false)
                .await?;
        // The first frame proves this process is the one the gateway asked an
        // agent to start. Nothing else on this socket is authenticated,
        // because nothing else needs to be: the gateway closes it otherwise.
        send(
            &mut ws,
            &Envelope {
                msg: Some(v1::envelope::Msg::EngineHello(v1::EngineHello {
                    game_id: attach.game_id.clone(),
                    token: attach.token.clone(),
                })),
            },
        )
        .await?;
        tracing::info!(game_id = attach.game_id, url = attach.url, "attached");

        let mut runner = EngineRunner::new();
        // The deadline is anchored to what it was armed for, so it restarts
        // when the game actually moves rather than every time a frame from
        // the other seat wakes this task up.
        let mut armed: Option<(baylee_engine_server::Clock, tokio::time::Instant)> = None;
        loop {
            match (runner.clock(), armed) {
                (Some(now), Some((was, _))) if was == now => {}
                (Some(now), _) => {
                    let at = tokio::time::Instant::now()
                        + std::time::Duration::from_secs(u64::from(now.secs));
                    armed = Some((now, at));
                }
                (None, _) => armed = None,
            }
            tokio::select! {
                () = deadline(armed.map(|(_, at)| at)) => {
                    // The clock is the one thing the rules kernel must not
                    // own: it is deterministic and may not read a wall clock.
                    let Some((clock, _)) = armed else { continue };
                    tracing::info!(seat = clock.seat.get(), "decision timed out");
                    armed = None;
                    for envelope in runner.timeout(clock) {
                        send(&mut ws, &envelope).await?;
                    }
                }
                frame = ws.next() => {
                    let Some(frame) = frame else { break };
                    let frame = frame?;
                    if !frame.is_binary() {
                        continue;
                    }
                    let envelope = Envelope::decode(frame.into_data())?;
                    for out in runner.handle(envelope) {
                        send(&mut ws, &out).await?;
                    }
                }
            }
            if runner.finished() {
                break;
            }
        }
        // Everything queued is already flushed by `send`; this is the polite
        // close the gateway logs as an ordinary end rather than a drop.
        let _ = ws.close(None).await;
        Ok(())
    }

    /// Sleeps until a seat's deadline, or forever when nothing is on the clock.
    async fn deadline(at: Option<tokio::time::Instant>) {
        match at {
            Some(at) => tokio::time::sleep_until(at).await,
            None => std::future::pending().await,
        }
    }

    async fn send(
        ws: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        envelope: &Envelope,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        ws.send(Message::Binary(envelope.encode_to_vec().into()))
            .await?;
        Ok(())
    }
}
