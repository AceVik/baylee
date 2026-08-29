//! baylee-engine-server — one process per game, websocket transport (M3).
//!
//! v1 protocol: protobuf `Envelope` framing over binary websocket frames;
//! complex engine structures (`Pending`, `PlayerAction`) travel as
//! `serde_json` payloads inside `ChoiceRequest` / `PlayerActionMsg`.
//!
//! The game logic (engine + AI seats) lives in [`session`]; this file is
//! pure transport: decode frames → session → encode frames.

mod session;
mod view;

use baylee_core::preset::GamePreset;
use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use session::Session;

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
    while let Ok((stream, peer)) = listener.accept().await {
        tracing::info!(%peer, "client connected");
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream).await {
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

async fn handle_connection(
    stream: tokio::net::TcpStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut ws = tokio_tungstenite::accept_async(stream).await?;
    let mut session: Option<Session> = None;
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
            v1::envelope::Msg::CreateGame(_create) => {
                // v1: every connection gets the acceptance-deck duel
                // (preset transfer from the gateway lands with M3 server
                // federation; the duel harness is the dev path).
                let preset = acceptance_duel_preset();
                session = Session::new(&preset);
                match session.as_mut() {
                    Some(s) => {
                        let mut out = vec![Envelope {
                            msg: Some(v1::envelope::Msg::GameCreated(v1::GameCreated {
                                game_id: uuid::Uuid::now_v7().to_string(),
                                your_seat: 0,
                            })),
                        }];
                        out.extend(s.pump());
                        out
                    }
                    None => vec![error("could not start the game")],
                }
            }
            v1::envelope::Msg::PlayerAction(action_msg) => {
                let Some(s) = session.as_mut() else {
                    continue;
                };
                let Ok(action) = serde_json::from_slice::<PlayerAction>(&action_msg.action_json)
                else {
                    continue;
                };
                s.act(action)
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
    baylee_ai::decks::preset_for(1, &allytifact, &victory)
}
