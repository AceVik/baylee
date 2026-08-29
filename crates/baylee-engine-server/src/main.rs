//! baylee-engine-server — one process per game, websocket transport (M3).
//!
//! v1 protocol: protobuf `Envelope` framing over binary websocket frames;
//! complex engine structures (`Pending`, `PlayerAction`) travel as
//! `serde_json` payloads inside `ChoiceRequest` / `PlayerActionMsg`.
//!
//! The game logic (engine + AI seats) lives in [`session`]; this file is
//! pure transport: decode frames → session → encode frames.

use baylee_core::preset::GamePreset;
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::{Session, preset};
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message;

/// Default port (dev).
const DEFAULT_PORT: u16 = 28765;

#[tokio::main]
async fn main() {
    tracing_subscriber_init();
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
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
    // Minimal stderr logging without pulling tracing-subscriber's fmt
    // feature matrix into the dependency tree.
    if std::env::var("RUST_LOG").is_ok() {
        eprintln!("baylee-engine-server: logging enabled");
    }
}

/// All live games on this server (v1: process-local; federation is a
/// gateway milestone).
type Games = std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, Session>>>;

#[allow(clippy::too_many_lines)]
async fn handle_connection(
    stream: tokio::net::TcpStream,
    games: Games,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut ws = tokio_tungstenite::accept_async(stream).await?;
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
                // Re-attach to a live game (v1: re-send the current view +
                // pending; full resume with seq comes with protocol v2).
                let mut games = games.lock().await;
                match games.get_mut(&join.game_id) {
                    Some(session) => {
                        game_id = Some(join.game_id.clone());
                        session.pump().into_iter().map(|(_, env)| env).collect()
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
    let allytifact = baylee_ai::decks::load_acceptance(&text, "Allytifact").expect("Allytifact");
    let victory = baylee_ai::decks::load_acceptance(&text, "Victory").expect("Victory");
    let mut preset = baylee_ai::decks::preset_for(1, &allytifact, &victory);
    // Seat 0 is the connecting human.
    preset.seats[0].controller = baylee_core::preset::SeatController::Open;
    preset
}
