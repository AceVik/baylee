//! The control and engine planes: agents, and the processes they start.
//!
//! The gateway runs no rules. A game lives in an engine process that an
//! *agent* starts and that dials the gateway back; from then on the gateway
//! only routes bytes between that process and the seats.
//!
//! ```text
//!   gateway ── StartEngine ──> agent ── spawn ──> engine
//!   gateway <── EngineHello / SeatFrame / GameEnded ── engine
//!   gateway ── GameSetup / SeatAttached / SeatFrame ─> engine
//! ```
//!
//! Two sockets, two secrets, and neither of them is a player's. An agent
//! proves itself with the shared secret in the gateway's configuration; an
//! engine proves itself with a token issued for exactly one game, which the
//! gateway handed to the agent and the agent handed to the process it
//! started. A player's seat token opens neither.

use crate::{Shared, auth, lobby::LobbyState};
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use baylee_protocol::v1::{self, Envelope};
use prost::Message as _;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// How often a connected agent is asked to say it is still there.
const HEARTBEAT_SECS: u32 = 30;

/// How long a socket has to identify itself before it is dropped.
///
/// Both planes send their hello immediately; anything that does not is either
/// broken or is holding a connection open to see what happens.
const HELLO_TIMEOUT_SECS: u64 = 10;

/// An agent that is connected right now.
pub struct Agent {
    /// Human label from the agent's hello, for logs.
    pub name: String,
    /// How many engines it will run at once. 0 means no limit.
    pub capacity: u32,
    /// Orders to that agent.
    pub tx: mpsc::UnboundedSender<Envelope>,
    /// Games it was asked to run.
    pub games: Vec<String>,
}

impl Agent {
    /// Whether one more game fits.
    fn has_room(&self) -> bool {
        self.capacity == 0 || self.games.len() < self.capacity as usize
    }
}

/// Every agent connected to this gateway.
#[derive(Default)]
pub struct Agents {
    /// Agents by the id the gateway gave them.
    pub connected: HashMap<String, Agent>,
}

impl Agents {
    /// The agent that should run the next game: the least busy one with room.
    ///
    /// Ties break on the id rather than on iteration order — a `HashMap` has
    /// no order to speak of, and "whichever one came out first" is not a
    /// scheduling policy anybody could reason about.
    fn pick(&self) -> Option<String> {
        self.connected
            .iter()
            .filter(|(_, agent)| agent.has_room())
            .min_by(|a, b| {
                a.1.games
                    .len()
                    .cmp(&b.1.games.len())
                    .then_with(|| a.0.cmp(b.0))
            })
            .map(|(id, _)| id.clone())
    }

    /// Sends one order to one agent, if it is still there.
    fn tell(&self, agent_id: &str, envelope: Envelope) -> bool {
        self.connected
            .get(agent_id)
            .is_some_and(|agent| agent.tx.send(envelope).is_ok())
    }
}

/// Asks an agent to start the engine for a game.
///
/// Called once, when a game's seats are decided. The seat sockets do not wait
/// on this: they wait on the engine actually attaching, which is a different
/// and later thing.
///
/// # Errors
/// When no agent has room, or when the game is not one that needs an engine.
pub fn start_engine(state: &Shared, game_id: &str) -> Result<(), &'static str> {
    let engine_token = auth::new_token();
    let agent_id = {
        let agents = state.agents.lock();
        agents.pick().ok_or("no engine capacity")?
    };
    {
        let mut lobby = state.lobby.lock();
        let game = lobby.games.get_mut(game_id).ok_or("no such game")?;
        game.engine_token_hash = Some(auth::token_hash(&engine_token));
        game.agent_id = Some(agent_id.clone());
    }
    let order = Envelope {
        msg: Some(v1::envelope::Msg::StartEngine(v1::StartEngine {
            game_id: game_id.to_string(),
            engine_token,
            gateway_url: state.engine_url.clone(),
        })),
    };
    let mut agents = state.agents.lock();
    let Some(agent) = agents.connected.get_mut(&agent_id) else {
        return Err("the agent went away");
    };
    if agent.tx.send(order).is_err() {
        return Err("the agent went away");
    }
    agent.games.push(game_id.to_string());
    tracing::info!(game_id, agent_id, agent = agent.name, "engine ordered");
    Ok(())
}

/// Tells whoever is running a game that it can stop.
fn stop_engine(state: &Shared, game_id: &str) {
    let agent_id = {
        let mut lobby = state.lobby.lock();
        let Some(game) = lobby.games.get_mut(game_id) else {
            return;
        };
        game.agent_id.take()
    };
    let Some(agent_id) = agent_id else { return };
    let mut agents = state.agents.lock();
    agents.tell(
        &agent_id,
        Envelope {
            msg: Some(v1::envelope::Msg::StopEngine(v1::StopEngine {
                game_id: game_id.to_string(),
            })),
        },
    );
    if let Some(agent) = agents.connected.get_mut(&agent_id) {
        agent.games.retain(|g| g != game_id);
    }
}

// ---------------------------------------------------------- control plane

/// `GET /agent/ws` — an agent offering to run engines.
pub async fn agent_ws(State(state): State<Shared>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| run_agent_socket(state, socket))
}

async fn run_agent_socket(state: Shared, mut socket: WebSocket) {
    let Some(v1::envelope::Msg::AgentHello(hello)) = hello_of(&mut socket).await else {
        return;
    };
    let Some(expected) = state.agent_token.as_deref() else {
        tracing::warn!("an agent connected but BAYLEE_AGENT_TOKEN is not set; refused");
        return;
    };
    if !auth::ct_eq(&auth::token_hash(&hello.token), &auth::token_hash(expected)) {
        tracing::warn!(name = hello.name, "agent presented the wrong token");
        return;
    }
    let agent_id = auth::new_id();
    let (tx, mut rx) = mpsc::unbounded_channel();
    state.agents.lock().connected.insert(
        agent_id.clone(),
        Agent {
            name: hello.name.clone(),
            capacity: hello.capacity,
            tx,
            games: Vec::new(),
        },
    );
    tracing::info!(
        agent_id,
        name = hello.name,
        capacity = hello.capacity,
        "agent registered"
    );
    let welcome = Envelope {
        msg: Some(v1::envelope::Msg::AgentWelcome(v1::AgentWelcome {
            agent_id: agent_id.clone(),
            heartbeat_secs: HEARTBEAT_SECS,
        })),
    };
    if send(&mut socket, &welcome).await.is_ok() {
        pump_agent(&state, &mut socket, &mut rx).await;
    }
    state.agents.lock().connected.remove(&agent_id);
    tracing::info!(agent_id, "agent gone");
}

/// One agent's conversation: orders out, status in.
async fn pump_agent(
    state: &Shared,
    socket: &mut WebSocket,
    rx: &mut mpsc::UnboundedReceiver<Envelope>,
) {
    loop {
        tokio::select! {
            order = rx.recv() => {
                let Some(order) = order else { return };
                if send(socket, &order).await.is_err() {
                    return;
                }
            }
            frame = socket.recv() => {
                // Heartbeats need no answer; they exist so a dead socket stops
                // looking like a live agent with nothing to do.
                match next_envelope(frame) {
                    Incoming::Msg(v1::envelope::Msg::EngineStatus(status)) => {
                        report(state, &status);
                    }
                    Incoming::Msg(_) | Incoming::Ignored => {}
                    Incoming::Closed => return,
                }
            }
        }
    }
}

/// What an agent says became of one engine.
fn report(state: &Shared, status: &v1::EngineStatus) {
    let kind = v1::engine_status::Kind::try_from(status.kind)
        .unwrap_or(v1::engine_status::Kind::Unspecified);
    match kind {
        v1::engine_status::Kind::Started => {
            tracing::info!(game_id = status.game_id, "engine started");
        }
        v1::engine_status::Kind::Failed => {
            tracing::error!(
                game_id = status.game_id,
                detail = status.detail,
                "engine failed to start"
            );
            // Nobody is coming. Saying so beats a table that waits forever.
            end_game(state, &status.game_id);
        }
        _ => tracing::info!(
            game_id = status.game_id,
            detail = status.detail,
            "engine exited"
        ),
    }
}

// ----------------------------------------------------------- engine plane

/// `GET /engine/ws` — the engine process for one game, dialling back.
pub async fn engine_ws(State(state): State<Shared>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| run_engine_socket(state, socket))
}

async fn run_engine_socket(state: Shared, mut socket: WebSocket) {
    let Some(v1::envelope::Msg::EngineHello(hello)) = hello_of(&mut socket).await else {
        return;
    };
    let game_id = hello.game_id.clone();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let setup = {
        let names = crate::seat_names(&state, &game_id);
        let mut lobby = state.lobby.lock();
        let Some(game) = lobby.games.get_mut(&game_id) else {
            tracing::warn!(game_id, "engine attached to a game that is not here");
            return;
        };
        // The token is the whole authorization on this socket: it was issued
        // for this game and handed to one agent. A second engine for the same
        // game is refused rather than allowed to take over — the game's state
        // lives in the process that already has it.
        let authorized = game
            .engine_token_hash
            .as_ref()
            .is_some_and(|h| auth::ct_eq(h, &auth::token_hash(&hello.token)));
        if !authorized || game.state != LobbyState::Playing || game.engine.is_some() {
            tracing::warn!(game_id, "engine attach refused");
            return;
        }
        let Some(preset) = game.preset.as_ref() else {
            return;
        };
        let Ok(preset_json) = serde_json::to_vec(preset) else {
            return;
        };
        game.engine = Some(tx);
        Envelope {
            msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
                game_id: game_id.clone(),
                preset_json,
                seat_names: names,
            })),
        }
    };
    if send(&mut socket, &setup).await.is_err() {
        return;
    }
    // Only now: a seat socket that sees this waits no longer, and everything
    // it then says has somewhere to go.
    //
    // `send_replace`, never `send`: a `watch::Sender` with no receivers left
    // refuses `send` *and keeps the old value*, and this channel routinely
    // has none — the engine can attach before the player who ordered it has
    // finished dialling their own seat. That socket then subscribes to a
    // channel still reading `false` and waits out the full timeout for an
    // engine that arrived before it did.
    {
        let lobby = state.lobby.lock();
        if let Some(game) = lobby.games.get(&game_id) {
            game.ready.send_replace(true);
        }
    }
    tracing::info!(game_id, "engine attached");
    pump_engine(&state, &game_id, &mut socket, &mut rx).await;
    // The socket is the game's lifeline: its state lives in that process and
    // nowhere else, so a link that closes before `GameEnded` is a game that
    // cannot be continued. Marking it over is the honest answer.
    let finished = {
        let mut lobby = state.lobby.lock();
        match lobby.games.get_mut(&game_id) {
            Some(game) => {
                let was_playing = game.state == LobbyState::Playing;
                game.finish(auth::now_secs());
                was_playing
            }
            None => false,
        }
    };
    if finished {
        tracing::warn!(game_id, "engine link lost; game closed");
    }
    state.lobby_moved();
    stop_engine(&state, &game_id);
}

/// One game's routing: seat frames both ways, until the game ends.
async fn pump_engine(
    state: &Shared,
    game_id: &str,
    socket: &mut WebSocket,
    rx: &mut mpsc::UnboundedReceiver<Envelope>,
) {
    loop {
        tokio::select! {
            outbound = rx.recv() => {
                let Some(outbound) = outbound else { return };
                if send(socket, &outbound).await.is_err() {
                    return;
                }
            }
            frame = socket.recv() => {
                match next_envelope(frame) {
                    Incoming::Msg(v1::envelope::Msg::SeatFrame(seat_frame)) => {
                        let Ok(seat) = u8::try_from(seat_frame.seat) else { continue };
                        let lobby = state.lobby.lock();
                        if let Some(game) = lobby.games.get(game_id) {
                            // No receivers is fine: a seat with no live socket
                            // is a seat nobody is waiting at.
                            let _ = game.updates.send((seat, seat_frame.envelope));
                        }
                    }
                    Incoming::Msg(v1::envelope::Msg::GameEnded(ended)) => {
                        tracing::info!(game_id, winners = ?ended.winners, reason = ended.reason, "game over");
                        end_game(state, game_id);
                        return;
                    }
                    Incoming::Msg(_) | Incoming::Ignored => {}
                    Incoming::Closed => return,
                }
            }
        }
    }
}

/// Marks a game finished and releases its engine.
fn end_game(state: &Shared, game_id: &str) {
    {
        let mut lobby = state.lobby.lock();
        if let Some(game) = lobby.games.get_mut(game_id) {
            game.finish(auth::now_secs());
        }
    }
    state.lobby_moved();
    stop_engine(state, game_id);
}

// ------------------------------------------------------------------ frames

/// Reads the first frame of a control or engine socket.
///
/// Whatever identifies the peer has to be the first thing it says: a socket
/// that opens and then waits is not one this gateway keeps.
async fn hello_of(socket: &mut WebSocket) -> Option<v1::envelope::Msg> {
    let deadline = std::time::Duration::from_secs(HELLO_TIMEOUT_SECS);
    loop {
        let frame = tokio::time::timeout(deadline, socket.recv()).await.ok()?;
        match frame? {
            Ok(Message::Binary(data)) => return Envelope::decode(data).ok()?.msg,
            Ok(_) => {}
            Err(_) => return None,
        }
    }
}

/// What one received frame amounts to.
///
/// "Nothing to act on" and "the socket is done" are different answers and only
/// one of them ends the loop, which is why this is three cases and not an
/// `Option`.
enum Incoming {
    /// A message this plane may act on.
    Msg(v1::envelope::Msg),
    /// A frame that carried nothing — not a reason to hang up.
    Ignored,
    /// The socket is gone.
    Closed,
}

/// Reads one received frame.
fn next_envelope(frame: Option<Result<Message, axum::Error>>) -> Incoming {
    match frame {
        Some(Ok(Message::Binary(data))) => Envelope::decode(data)
            .ok()
            .and_then(|e| e.msg)
            .map_or(Incoming::Ignored, Incoming::Msg),
        // Pings are answered by axum; other frame kinds are not this protocol.
        Some(Ok(_)) => Incoming::Ignored,
        Some(Err(_)) | None => Incoming::Closed,
    }
}

/// Sends one envelope, or reports that the socket is gone.
async fn send(socket: &mut WebSocket, envelope: &Envelope) -> Result<(), ()> {
    futures_util::SinkExt::send(socket, Message::Binary(envelope.encode_to_vec().into()))
        .await
        .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(games: usize, capacity: u32) -> Agent {
        Agent {
            name: "test".to_string(),
            capacity,
            tx: mpsc::unbounded_channel().0,
            games: (0..games).map(|i| i.to_string()).collect(),
        }
    }

    #[test]
    fn the_least_busy_agent_gets_the_next_game() {
        let mut agents = Agents::default();
        agents.connected.insert("b".to_string(), agent(0, 0));
        agents.connected.insert("a".to_string(), agent(3, 0));
        assert_eq!(agents.pick().as_deref(), Some("b"));
    }

    #[test]
    fn a_full_agent_is_not_picked_and_no_agent_is_not_a_panic() {
        let mut agents = Agents::default();
        assert_eq!(agents.pick(), None, "nothing to pick from");
        agents.connected.insert("a".to_string(), agent(2, 2));
        assert_eq!(agents.pick(), None, "at capacity");
        agents.connected.insert("b".to_string(), agent(2, 0));
        assert_eq!(
            agents.pick().as_deref(),
            Some("b"),
            "capacity 0 means no limit, however many it is already running"
        );
    }

    /// Two agents with the same load must not be picked by hash order, or
    /// "which agent runs this game" would differ between runs of the same
    /// gateway with the same agents connected.
    #[test]
    fn ties_break_on_the_id() {
        let mut agents = Agents::default();
        for id in ["m", "a", "z"] {
            agents.connected.insert(id.to_string(), agent(1, 0));
        }
        assert_eq!(agents.pick().as_deref(), Some("a"));
    }
}
