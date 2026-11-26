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
//!   gateway ── FlushRecord ─> engine ── GameRecordChunk / RecordFlushed ─> gateway
//! ```
//!
//! Two sockets, two secrets, and neither of them is a player's. An agent
//! proves itself with the shared secret in the gateway's configuration; an
//! engine proves itself with a token issued for exactly one game, which the
//! gateway handed to the agent and the agent handed to the process it
//! started. A player's seat token opens neither.

use crate::{Shared, auth, lobby::LobbyState};
use axum::Extension;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse as _, Response};
use baylee_protocol::v1::{self, Envelope};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use uuid::Uuid;

pub(crate) mod execution;
mod lifecycle;
mod replica;
mod tower;
mod wire;
use wire::{
    Incoming, hello_of, next_envelope, outbox_of, refusal, send, send_queued, spectator_outbox_of,
};

/// How often a connected agent is asked to say it is still there.
const HEARTBEAT_SECS: u32 = 30;

/// How long a socket has to identify itself before it is dropped.
///
/// Both planes send their hello immediately; anything that does not is either
/// broken or is holding a connection open to see what happens.
const HELLO_TIMEOUT_SECS: u64 = 10;

/// Marks a request that came in on the gateway's unix socket
/// (`BAYLEE_UNIX_SOCKET`), so from this machine.
#[derive(Clone, Copy, Debug)]
pub struct ViaUnix;

/// An agent that is connected right now.
pub struct Agent {
    /// Human label from the agent's hello, for logs.
    pub name: String,
    /// Whether it connected on the unix socket, i.e. runs on this machine.
    pub local: bool,
    /// How many engines it will run at once. 0 means no limit.
    pub capacity: u32,
    /// Bounded identities of locally installed artifacts advertised on this connection.
    pub engines: Vec<baylee_protocol::execution::Identity>,
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
    /// Whether any agent is connected, so whether a game could start at all.
    ///
    /// Capacity is not asked: an agent that is full now has room again as
    /// soon as a game ends, and a lobby that flickered between "games can
    /// start" and "they cannot" with every game would be telling players
    /// about scheduling, not about whether this gateway is working.
    #[must_use]
    pub fn any_connected(&self) -> bool {
        !self.connected.is_empty()
    }

    /// The agent that should run the next game: the least busy one with room.
    ///
    /// Ties break on the id rather than on iteration order — a `HashMap` has
    /// no order to speak of, and "whichever one came out first" is not a
    /// scheduling policy anybody could reason about.
    #[cfg(test)]
    pub(crate) fn pick(&self) -> Option<String> {
        self.pick_executor(None)
    }

    pub(crate) fn pick_executor(&self, executor: Option<&str>) -> Option<String> {
        self.pick_variant(baylee_protocol::execution::Variant::StandardLoops, executor)
            .map(|(id, _)| id)
    }

    /// Sends one order to one agent, if it is still there.
    pub(crate) fn tell(&self, agent_id: &str, envelope: Envelope) -> bool {
        self.connected
            .get(agent_id)
            .is_some_and(|agent| agent.tx.send(envelope).is_ok())
    }
}

mod start;
pub use start::start_engine;
pub(crate) use start::start_replica;

/// Tells whoever is running a game that it can stop.
pub(crate) fn stop_engine(state: &Shared, game_id: &str) {
    let (agent_id, job) = {
        let mut lobby = state.lobby.lock();
        let Some(game) = lobby.games.get_mut(game_id) else {
            return;
        };
        (
            game.agent_id.take(),
            game.engine_job
                .take()
                .unwrap_or_else(|| game_id.to_string()),
        )
    };
    let Some(agent_id) = agent_id else { return };
    let mut agents = state.agents.lock();
    agents.tell(
        &agent_id,
        Envelope {
            msg: Some(v1::envelope::Msg::StopEngine(v1::StopEngine {
                game_id: job.clone(),
            })),
        },
    );
    if let Some(agent) = agents.connected.get_mut(&agent_id) {
        agent.games.retain(|g| g != &job);
    }
}

// ---------------------------------------------------------- control plane

/// `GET /agent/ws` — an agent offering to run engines.
///
/// @channel agent /agent/ws
/// @scope public
/// @summary An engine agent's link
/// @description The control link of an engine agent (`baylee-agent`),
/// which starts one engine process per game it is ordered to run. Every
/// frame is one protobuf `Envelope` (`baylee.v1`) in a binary frame. There
/// is no ticket: the agent's secret is in its first frame, an `AgentHello`
/// within 10 seconds carrying the gateway's `BAYLEE_AGENT_TOKEN`. A gateway
/// without `BAYLEE_AGENT_TOKEN`, a wrong token, or a first binary frame that
/// is not an `AgentHello` closes the socket without a word; an agent of
/// another protocol is sent an `Error` naming both and the socket closes.
/// Accepted, the agent is sent `AgentWelcome` and is ordered from then on:
/// `StartEngine` for each game it is to run (the least busy agent with room
/// is picked) and `StopEngine` once a game it runs is over. It says what
/// became of each engine with `EngineStatus`, and a start that failed ends
/// that game. A frame that does not decode, and a kind this link does not
/// carry, is passed over. An agent on the gateway's unix socket
/// (`BAYLEE_UNIX_SOCKET`) is local, and so is every game it runs. A
/// player's seat token or session opens no agent link. A frame is at most
/// 64 KiB, a larger one ends the connection, and there is no rate bound. An
/// address that is not this machine may hold 16 sockets here, on `/engine/ws`
/// and on `/seathost/ws` that have said no hello yet; past that the upgrade
/// is refused with `429`.
/// An agent silent for 90 seconds loses this link. Disconnect closes only
/// its assigned games and replay jobs; a new link cannot reclaim them.
/// An ordered game engine must attach within 30 seconds. Exit before its
/// hello releases the assignment; an attached engine drains its final
/// record on its own link before its exit is acted on.
/// @receive `AgentHello`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `agent_hello`, first: `token` (the gateway's `BAYLEE_AGENT_TOKEN`), `name` (for logs and the operator), `capacity` (engines at once, 0 for no bound) and `protocol_version` (the protocol it was built with; another, or 0, is refused)."
/// @receive `EngineStatus`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `engine_status`: what became of one engine, by `game_id`. `kind` is `KIND_STARTED`, `KIND_EXITED` or `KIND_FAILED` (it never started, which ends the game), and `detail` says why."
/// @receive `Heartbeat`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `heartbeat`, every `heartbeat_secs`: still here. It needs no answer."
/// @send `AgentWelcome`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `agent_welcome`: accepted. `agent_id` is the gateway's id for this link, and `heartbeat_secs` how often to send a `Heartbeat`."
/// @send `Error`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `error`, `code` 1: the agent's protocol is not the gateway's, and `message` names both. The only frame; the socket closes after it."
/// @send `StartEngine`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `start_engine`: start an engine for `game_id` (a replay job's id begins `replay-`). `engine_token` is one game's authority, which the engine shows in its `EngineHello` and which never leaves the agent's machine; `gateway_url` is where the engine dials back (`BAYLEE_ENGINE_URL`; a local agent hands its engines the unix socket instead)."
/// @send `StopEngine`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `stop_engine`: the game `game_id` is over and its engine can go."
pub async fn agent_ws(
    State(state): State<Shared>,
    via_unix: Option<Extension<ViaUnix>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let local = via_unix.is_some();
    let Ok(place) = hello_place(&state, peer, &headers, local) else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    ws.max_message_size(MAX_AGENT_FRAME)
        .max_frame_size(MAX_AGENT_FRAME)
        .on_upgrade(move |socket| run_agent_socket(state, socket, local, place))
}

/// The biggest frame an agent sends: a hello, a heartbeat, or an engine's
/// status with a line of detail. A stranger's first frame is held to this
/// before its secret is read.
const MAX_AGENT_FRAME: usize = 64 * 1024;

/// The biggest frame an engine sends, which is the transport's own bound on
/// a frame. A view, a record piece and a tape piece are all far under it,
/// and an engine sends none in several frames.
const MAX_ENGINE_FRAME: usize = 16 << 20;

/// How many sockets one address may hold on the control and engine planes
/// before they said hello.
///
/// These doors are public and read a frame before anything about the sender
/// is known, so this is what a stranger's frames can cost: a bound on the
/// memory one address holds, not on a real peer, whose hello follows its
/// dial at once. An agent starting a burst of engines has this many in
/// flight only if every one of them is slower than the next is started.
pub(crate) const HELLOS_PER_ADDRESS: usize = 16;

/// A place among the sockets that have said no hello yet, or `Err` while
/// the dialling address holds all of its own.
pub(crate) fn hello_place(
    state: &crate::AppState,
    peer: SocketAddr,
    headers: &HeaderMap,
    local: bool,
) -> Result<Option<auth::Held>, ()> {
    crate::address_place(&state.hellos, &state.trusted_proxies, peer, headers, local)
}

async fn run_agent_socket(
    state: Shared,
    mut socket: WebSocket,
    local: bool,
    place: Option<auth::Held>,
) {
    let hello = hello_of(&mut socket).await;
    drop(place);
    let Some(v1::envelope::Msg::AgentHello(hello)) = hello else {
        return;
    };
    let Some(expected) = state.agent_token.as_deref() else {
        tracing::warn!("an agent connected but BAYLEE_AGENT_TOKEN is not set; refused");
        return;
    };
    // The name is a stranger's until the secret is right, so it is not logged.
    if !auth::ct_eq(&auth::token_hash(&hello.token), &auth::token_hash(expected)) {
        tracing::warn!("agent presented the wrong token");
        return;
    }
    // After the secret, so a stranger learns nothing here it could not read
    // off `/info`. Said rather than dropped: the operator restarting this
    // agent in a loop needs the sentence, not a closed socket (#271).
    if let Some(why) = baylee_protocol::version_refusal("This agent", hello.protocol_version) {
        tracing::warn!(name = hello.name, why, "agent refused");
        let _ = send(&mut socket, &refusal(&why)).await;
        return;
    }
    let agent_id = auth::new_id();
    if hello.executor.len() > 160
        || hello
            .executor
            .bytes()
            .any(|byte| !(32..=126).contains(&byte))
    {
        return;
    }
    let Some(engines) = baylee_protocol::execution::capabilities_from_metadata(&hello.engines)
    else {
        let _ = send(&mut socket, &refusal("invalid engine capabilities")).await;
        return;
    };
    let (tx, mut rx) = mpsc::unbounded_channel();
    let first = {
        let mut agents = state.agents.lock();
        let first = !agents.any_connected();
        agents.connected.insert(
            agent_id.clone(),
            Agent {
                name: hello.name.clone(),
                local,
                capacity: hello.capacity,
                engines,
                tx,
                games: Vec::new(),
            },
        );
        first
    };
    tracing::info!(
        agent_id,
        name = hello.name,
        capacity = hello.capacity,
        local,
        "agent registered"
    );
    // Every lobby listing says whether a game can start (`agents_available`);
    // the first agent in and the last one out change that answer.
    if first {
        state.lobby_moved();
    }
    let welcome = Envelope {
        msg: Some(v1::envelope::Msg::AgentWelcome(v1::AgentWelcome {
            agent_id: agent_id.clone(),
            heartbeat_secs: HEARTBEAT_SECS,
        })),
    };
    if send(&mut socket, &welcome).await.is_ok() {
        // A game restored after a restart may be waiting for this agent.
        crate::keep::order_restored(&state);
        pump_agent(&state, &agent_id, &mut socket, &mut rx).await;
    }
    let (lost, last) = {
        let mut agents = state.agents.lock();
        let lost = agents.connected.remove(&agent_id);
        (lost, !agents.any_connected())
    };
    // A new connection gets a new identity; there is no protocol for
    // transferring its old engines. Releasing their owner cannot leave
    // running tables behind, including engines that never sent a hello.
    if let Some(agent) = lost {
        for game_id in agent.games {
            lifecycle::owner_lost(&state, &game_id);
        }
    }
    tracing::info!(agent_id, "agent gone");
    if last {
        state.lobby_moved();
    }
}

/// One agent's conversation: orders out, status in.
async fn pump_agent(
    state: &Shared,
    agent_id: &str,
    socket: &mut WebSocket,
    rx: &mut mpsc::UnboundedReceiver<Envelope>,
) {
    let silence = std::time::Duration::from_secs(u64::from(HEARTBEAT_SECS) * 3);
    let mut heard = tokio::time::Instant::now();
    loop {
        tokio::select! {
            () = tokio::time::sleep_until(heard + silence) => return,
            order = rx.recv() => {
                let Some(order) = order else { return };
                if !matches!(tokio::time::timeout(silence, send_queued(socket, &order, rx)).await, Ok(Ok(()))) {
                    return;
                }
            }
            frame = socket.recv() => {
                heard = tokio::time::Instant::now();
                // Heartbeats need no answer; they exist so a dead socket stops
                // looking like a live agent with nothing to do.
                match next_envelope(frame) {
                    Incoming::Msg(v1::envelope::Msg::EngineStatus(status)) => {
                        report(state, agent_id, &status);
                    }
                    Incoming::Msg(_) | Incoming::Ignored => {}
                    Incoming::Closed => return,
                }
            }
        }
    }
}

/// What an agent says became of one engine.
fn report(state: &Shared, agent_id: &str, status: &v1::EngineStatus) {
    if !state
        .agents
        .lock()
        .connected
        .get(agent_id)
        .is_some_and(|agent| agent.games.contains(&status.game_id))
    {
        return;
    }
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
            if status
                .game_id
                .starts_with(crate::ranked::replicate::JOB_PREFIX)
            {
                lifecycle::owner_lost(state, &status.game_id);
            } else if status.game_id.starts_with(crate::replays::JOB_PREFIX) {
                crate::replays::failed(state, &status.game_id);
            } else {
                end_game(state, &status.game_id);
            }
        }
        v1::engine_status::Kind::Exited => {
            // An exit may precede the engine's first hello. Once attached,
            // let its own ordered socket drain the final record/GameEnded;
            // the status link can overtake those frames.
            if status.game_id.starts_with(crate::replays::JOB_PREFIX) {
                // Replay jobs already have an independent deadline and
                // must likewise drain the final tape before being failed.
                return;
            }
            lifecycle::end_unattached(state, &status.game_id, agent_id);
        }
        v1::engine_status::Kind::Unspecified => tracing::info!(
            game_id = status.game_id,
            detail = status.detail,
            "engine exited"
        ),
    }
}

// ----------------------------------------------------------- engine plane

/// `GET /engine/ws` — the engine process for one game, dialling back.
///
/// @channel engine /engine/ws
/// @scope public
/// @summary An engine's link
/// @description The link of one engine process (`baylee-engine-server`)
/// for one game, or for one replay job, dialled back after its agent was
/// sent `StartEngine`. Every frame is one protobuf `Envelope` (`baylee.v1`)
/// in a binary frame. There is no ticket: the engine's secret is in its
/// first frame, an `EngineHello` within 10 seconds carrying the game's id
/// and the token `StartEngine` named. A game that is not here or not being
/// played, a wrong token, a second engine for a game that has one, or a
/// first binary frame that is not an `EngineHello` closes the socket
/// without a word. An engine of another protocol is sent an `Error` naming
/// both and the socket closes, and the game ends with it (it leaves the
/// listing and its agent is sent `StopEngine`). Accepted, the engine is
/// sent `GameSetup`, and the seat sockets waiting for it go on. From then on
/// the gateway tells it which seat sockets come and go (`SeatAttached`,
/// `SeatDetached`), forwards each seat's frames (`SeatFrameFromSeat`) and
/// the spectator count (`SpectatorsChanged`), asks for the record as it
/// stands for a bug report (`FlushRecord`), and passes on an admin's
/// `CloseGame`. The engine sends each seat's frames (`SeatFrameToSeat`, for
/// the seat's socket as they are), the spectators' stream
/// (`SpectatorFrame`), the game record in pieces (`GameRecordChunk`, stored
/// and never forwarded to anyone), an answer to every `FlushRecord`
/// (`RecordFlushed`), the seats that are out of the game
/// (`SeatParticipationEnded`), and `GameEnded` last. Losing this link ends
/// the game, since its state lives in that process. An engine started for a
/// replay job (an id beginning `replay-`) is sent `ProjectRecord` instead
/// of `GameSetup`, sends `TapeChunk`s back until the last, and is closed. A
/// frame that does not decode, and a kind this link does not carry, is
/// passed over. A frame is at most 16 MiB, a larger one ends the connection,
/// and there is no rate bound; `/agent/ws` says how many sockets one address
/// may hold here before their hello.
/// @receive `EngineHello`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `engine_hello`, first: `game_id` and `token` as `StartEngine` named them, `protocol_version` (another, or 0, is refused), `pid` (its process id, which a gateway on the same machine reads for the admin console; 0 says none) and `build` (its build, kept beside the game's record; empty from an older engine)."
/// @receive `SeatFrameToSeat`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `seat_frame`: `envelope`, an encoded player-facing `Envelope`, for the socket of seat `seat`, which is handed it as it is. Dropped while that seat has no socket."
/// @receive `SpectatorFrame`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `spectator_frame`: `envelope`, an encoded player-facing `Envelope` of the public stream, handed as it is to every watch socket of the game."
/// @receive `GameRecordChunk`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `game_record_chunk`: a piece of the game's record. `seq` counts from 0, `data` is one gzip member of JSON Lines (so the pieces in `seq` order are one gzip stream), and `last` marks the piece sent just before `GameEnded`. Stored, never forwarded to a seat."
/// @receive `RecordFlushed`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `record_flushed`: everything of the record up to the `FlushRecord` with this `nonce` was sent before this frame; `pieces` is how many pieces the engine has sent so far."
/// @receive `SeatParticipationEnded`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `seat_participation_ended`: these `seats` lost, conceded, or were left to the house with no human playing. Their players may start or join another game on this gateway again."
/// @receive `GameEnded`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `game_ended`: the game is over. `winners` (empty for a draw or a game an admin closed), `reason`, and `turn` (0 from an older engine). The gateway marks the game over and its agent is sent `StopEngine`."
/// @receive `TapeChunk`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `tape_chunk`, replay jobs only: a piece of the tape for `job`; `data` is one gzip member, the pieces in `seq` order are one gzip stream, and the `last` piece ends the job."
/// @send `GameSetup`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `game_setup`: everything needed to build the game. `preset_json` is the game's `GamePreset` as JSON; `seat_names` are the seats' names in seat order."
/// @send `RankedEngineSetup`: proto(baylee.v1.Envelope) application/x-protobuf "Protected duel opening with the immutable game, role and generation."
/// @send `RankedEngineStep`: proto(baylee.v1.Envelope) application/x-protobuf "One ordered, authenticated journal operation."
/// @receive `RankedEngineVote`: proto(baylee.v1.Envelope) application/x-protobuf "Private execution evidence for independent certification."
/// @send `RankedEngineCommit`: proto(baylee.v1.Envelope) application/x-protobuf "Release the certified position. Only the active primary may publish seat frames."
/// @send `RankedEngineSnapshot`: proto(baylee.v1.Envelope) application/x-protobuf "Request the authorized seat's current certified projection."
/// @send `SeatAttached`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `seat_attached`: seat `seat` has a socket now. With `resync` true the seat is already in the game and its socket fell behind: send it a snapshot rather than advancing the game."
/// @send `SeatDetached`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `seat_detached`: seat `seat` has no socket any more. With `left` true its player left on purpose (`POST /lobby/depart`): the house takes the chair at once, or the game ends when no other player is at the table."
/// @send `SeatFrameFromSeat`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `seat_frame`: `envelope` is a frame exactly as seat `seat`'s socket sent it, the seat stamped by the gateway."
/// @send `SpectatorsChanged`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `spectators_changed`: `count` watch sockets are open now; `joined` asks for a whole spectator snapshot (one opened, or one fell behind)."
/// @send `FlushRecord`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `flush_record`, for a bug report at a game that goes on: send what has gathered of the record now as a piece that is not `last`, then a `RecordFlushed` with the same `nonce`."
/// @send `CloseGame`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `close_game`: an admin ends the game. Tell every seat and spectator `GameClosed` with `reason`, send the rest of the record and then `GameEnded` with no winners."
/// @send `ProjectRecord`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `project_record`, replay jobs only, instead of `GameSetup`: play the stored record (`record`, one gzip stream) again and send back the tape of one perspective of it (a seat, the spectators, or a seat with other hands beside it)."
/// @send `Error`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `error`, `code` 1: the engine's protocol is not the gateway's, and `message` names both. The only frame; the socket closes after it."
pub async fn engine_ws(
    State(state): State<Shared>,
    via_unix: Option<Extension<ViaUnix>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let Ok(place) = hello_place(&state, peer, &headers, via_unix.is_some()) else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    ws.max_message_size(MAX_ENGINE_FRAME)
        .max_frame_size(MAX_ENGINE_FRAME)
        .read_buffer_size(ENGINE_READ_BUFFER)
        .on_upgrade(move |socket| run_engine_socket(state, socket, place))
}

/// How much an engine's socket reads from the network at once: a view is
/// tens of KiB, and tungstenite zeroes the whole buffer before each read
/// (see `SEAT_READ_BUFFER`), so its 128 KiB default cost more than it saved.
const ENGINE_READ_BUFFER: usize = 32 << 10;

mod game;
use game::run_engine_socket;

/// The engine an agent started for a replay job (`replays.rs`): sent the
/// job's order once its token is the job's, and heard until the tape's last
/// piece. Never a game: nothing here touches the lobby.
async fn run_job_socket(state: &Shared, mut socket: WebSocket, hello: &v1::EngineHello) {
    let job_id = hello.game_id.as_str();
    let Some(order) = crate::replays::order_for(state, hello) else {
        tracing::warn!("replay engine attach refused");
        return;
    };
    let mut gather = crate::replays::Gather::new(job_id);
    if let Some(why) = baylee_protocol::version_refusal("This engine", hello.protocol_version) {
        tracing::error!(job = job_id, why, "replay engine refused");
        let _ = send(&mut socket, &refusal(&why)).await;
        gather.finish(state, false);
        return;
    }
    let order = Envelope {
        msg: Some(v1::envelope::Msg::ProjectRecord(order)),
    };
    if send(&mut socket, &order).await.is_err() {
        gather.finish(state, false);
        return;
    }
    let whole = loop {
        match next_envelope(socket.recv().await) {
            Incoming::Msg(v1::envelope::Msg::TapeChunk(piece)) => {
                if gather.take(&piece) {
                    break piece.last;
                }
            }
            Incoming::Msg(_) | Incoming::Ignored => {}
            Incoming::Closed => break false,
        }
    };
    gather.finish(state, whole);
    let _ = futures_util::SinkExt::close(&mut socket).await;
}

/// Marks a game finished and releases its engine.
pub(crate) fn end_game(state: &Shared, game_id: &str) {
    crate::tower::ended(state, game_id);
    {
        let mut lobby = state.lobby.lock();
        if let Some(game) = lobby.games.get_mut(game_id) {
            game.finish(auth::now_secs());
        }
    }
    // Not kept any more: a restart must not bring it back.
    state.keeper.touch(game_id);
    state.lobby_moved();
    stop_engine(state, game_id);
}

#[cfg(test)]
mod tests;
