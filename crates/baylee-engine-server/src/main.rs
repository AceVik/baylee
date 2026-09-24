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
//!
//! The dev harness seats one socket per seat, and a socket may name an AI
//! chair — see [`seat_from_token`]. That is the rules-side counterpart of the
//! client's `dev-control` harness: a program can hold an opponent's controls
//! over an ordinary socket, see exactly what that seat is entitled to see and
//! answer for it, and hand the chair back to the house AI by hanging up.

use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::{SeatKind, Session, preset};
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message;

/// Default port (dev).
const DEFAULT_PORT: u16 = 28765;

/// The seat a connection gets when it names none.
///
/// Every client written before a socket could name a seat lands here, which
/// is where it used to land.
const SEAT: PlayerId = PlayerId::new(0);

/// Seat names for the dev duel's roster.
fn seat_names() -> Vec<String> {
    vec!["You".to_string(), "House AI".to_string()]
}

/// Default bind address: loopback only. This is a dev harness with no
/// authentication — a connection names its own seat and is handed that
/// seat's hidden information, and every action runs as the seat it named —
/// so it must not be reachable from the network unless the operator
/// explicitly says so (`BAYLEE_BIND=0.0.0.0`).
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
        Self::read(&args, |name| std::env::var(name).ok())
    }

    /// The attachment a given command line and environment describe.
    ///
    /// Split from [`Attach::discover`] so that the precedence and the
    /// all-or-nothing rule can be asserted: a function that read the process
    /// arguments itself could not be tested for what it does with them, which
    /// is the same bargain `import::plan` makes with the clock.
    ///
    /// The all-or-nothing half is the one worth having a test for, because
    /// answering `None` is not an error here — it is a different program.
    /// A launch missing any one of the three falls through to [`listen`], the
    /// dev harness, which has no authentication at all and hands a socket
    /// whichever seat it names.
    fn read(args: &[String], env: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let flag = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        // An empty variable is not an answer. A shell that exports
        // `BAYLEE_ENGINE_TOKEN=` from an unset value would otherwise attach
        // with an empty token and be refused by the gateway, which reads as
        // a rejected engine rather than as a launch that was never given one.
        let from_env = |name: &str| env(name).filter(|v| !v.is_empty());
        let url = flag("--attach").or_else(|| from_env("BAYLEE_ATTACH_URL"))?;
        let game_id = flag("--game").or_else(|| from_env("BAYLEE_GAME"))?;
        let token = flag("--token").or_else(|| from_env("BAYLEE_ENGINE_TOKEN"))?;
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

/// One live game and everything sitting at it.
struct Table {
    /// The game itself.
    session: Session,
    /// Per-seat envelopes on their way to whichever socket holds that seat.
    ///
    /// This exists because a question is produced by whoever *moved* the
    /// game, not by whoever has to answer it: seat 0 casting a spell is what
    /// produces seat 1's choice request. With one socket per seat that frame
    /// has to reach a connection other than the one holding the lock, so
    /// every `(seat, envelope)` a session hands back goes through here and
    /// each connection sends on the ones addressed to its own seat.
    fan: tokio::sync::broadcast::Sender<(u8, Envelope)>,
}

/// How far behind a socket may fall before it is rebuilt from scratch.
///
/// On loopback this is never reached. It is a bound rather than a tuning
/// knob, and the recovery is a full snapshot, so a socket that does hit it
/// loses time and never loses a question.
const FAN_DEPTH: usize = 1024;

/// All live games on this server (v1: process-local; federation is a
/// gateway milestone).
type Games = std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, Table>>>;

/// Which seat a connection is asking for.
///
/// The dev harness has no authentication, so `seat_token` carries no
/// authority here and would otherwise be read by nothing — which is exactly
/// what makes it the right field to name a seat with. A plain seat number
/// needs no new message and no second meaning for the frame: against the
/// gateway the same field still says "prove it", and there it is still
/// checked. An empty or unreadable token is [`SEAT`].
fn seat_from_token(token: &str) -> PlayerId {
    PlayerId::new(token.trim().parse::<u8>().unwrap_or(0))
}

/// Whether a connection may sit at `seat`, and whether doing so takes an AI
/// chair over.
///
/// Three answers, deliberately different. A free human chair is sat in; an AI
/// chair is taken over and owed back; anything else is refused — a seat that
/// is not at this table, or one another socket is already answering for.
/// Two sockets on one AI chair is not a shared seat, it is two programs
/// holding the same hand, and the second one would race the first for every
/// question.
fn sit(session: &mut Session, seat: PlayerId) -> Result<bool, &'static str> {
    match session.seat_kind(seat) {
        None => Err("no such seat"),
        Some(SeatKind::Driven(_)) => Err("that seat is already being driven"),
        // Unreachable here: the reconnect window is `EngineRunner`'s, and
        // this harness has no clock at all. Spelled out rather than folded
        // into the human arm, because sitting down at a chair the house is
        // holding is a different thing from joining an empty one, and the day
        // this harness grows a clock it should say so rather than guess.
        Some(SeatKind::StandIn(_)) => Err("that seat is being held for a player"),
        Some(SeatKind::Human) => Ok(false),
        Some(SeatKind::Ai(_)) => {
            session.take_over(seat);
            Ok(true)
        }
    }
}

/// Hands every per-seat envelope to the socket that holds that seat.
fn publish(table: &Table, out: Vec<(PlayerId, Envelope)>) {
    for (seat, envelope) in out {
        // This fails only when nobody is listening — a table with no sockets
        // on it, which has nothing to deliver to and nothing to log.
        let _ = table.fan.send((seat.get(), envelope));
    }
}

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

/// What this connection is, and where it sits.
struct Sitting {
    /// The game it has joined, once it has joined one.
    game_id: Option<String>,
    /// The seat it answers for.
    seat: PlayerId,
    /// Whether it took an AI chair over and therefore owes it back.
    driving: bool,
    /// Its end of the table's fan, once it is at a table.
    rx: Option<tokio::sync::broadcast::Receiver<(u8, Envelope)>>,
}

/// Why the connection task woke up.
enum Wake {
    /// A frame arrived from this socket (`None` once it has hung up).
    Frame(
        Option<
            Result<tokio_tungstenite::tungstenite::Message, tokio_tungstenite::tungstenite::Error>,
        >,
    ),
    /// The table sent something to one of its seats.
    Fan(Result<(u8, Envelope), tokio::sync::broadcast::error::RecvError>),
}

/// Waits on the fan, or forever when this connection is not at a table yet.
async fn fanned(
    rx: Option<&mut tokio::sync::broadcast::Receiver<(u8, Envelope)>>,
) -> Result<(u8, Envelope), tokio::sync::broadcast::error::RecvError> {
    match rx {
        Some(rx) => rx.recv().await,
        None => std::future::pending().await,
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    games: Games,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut ws = tokio_tungstenite::accept_async_with_config(stream, Some(ws_config())).await?;
    let mut state = Sitting {
        game_id: None,
        seat: SEAT,
        driving: false,
        rx: None,
    };
    let served = serve(&mut ws, &games, &mut state).await;
    // The chair goes back whichever way the socket ended, including the
    // error paths above — that is why this is not inside `serve`. A
    // developer who disconnects mid-game leaves a playable opponent behind
    // rather than a table that stops at the next question nobody answers.
    if state.driving
        && let Some(id) = state.game_id.as_ref()
    {
        let mut games = games.lock().await;
        if let Some(table) = games.get_mut(id) {
            table.session.release(state.seat);
            let out = table.session.pump();
            publish(table, out);
            tracing::info!(seat = state.seat.get(), "seat handed back to the house AI");
        }
    }
    served
}

#[allow(clippy::too_many_lines)]
async fn serve(
    ws: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    games: &Games,
    state: &mut Sitting,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let wake = tokio::select! {
            frame = ws.next() => Wake::Frame(frame),
            got = fanned(state.rx.as_mut()) => Wake::Fan(got),
        };
        let replies: Vec<Envelope> = match wake {
            // The socket hung up, or the table is gone — and on this server
            // the table only goes when the process does.
            Wake::Frame(None)
            | Wake::Fan(Err(tokio::sync::broadcast::error::RecvError::Closed)) => break,
            Wake::Frame(Some(frame)) => {
                let frame = frame?;
                if !frame.is_binary() {
                    continue;
                }
                let envelope = Envelope::decode(frame.into_data())?;
                let Some(msg) = envelope.msg else {
                    continue;
                };
                match msg {
                    v1::envelope::Msg::CreateGame(create) => {
                        let preset = match create.preset {
                            Some(msg) => preset::from_proto(&msg),
                            // No preset: the dev acceptance duel.
                            None => Ok(acceptance_duel_preset()),
                        };
                        let preset = match preset {
                            Ok(preset) => preset,
                            Err(reason) => {
                                let reply = error(&format!("bad preset: {reason}"));
                                ws.send(tokio_tungstenite::tungstenite::Message::Binary(
                                    reply.encode_to_vec().into(),
                                ))
                                .await?;
                                continue;
                            }
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
                                let (fan, rx) = tokio::sync::broadcast::channel(FAN_DEPTH);
                                let mut games = games.lock().await;
                                games.insert(id.clone(), Table { session, fan });
                                let table = games.get_mut(&id).expect("just inserted");
                                // Subscribed before the pump, so the frames
                                // it produces for this seat are waiting on
                                // the fan rather than already gone.
                                state.rx = Some(rx);
                                state.seat = SEAT;
                                // The seat roster and the print table come
                                // first: a client has no preset to build them
                                // from, and every view after this one refers
                                // to them.
                                table.session.describe(id.clone(), seat_names());
                                out.push(table.session.game_static_envelope(SEAT));
                                let pumped = table.session.pump();
                                publish(table, pumped);
                                state.game_id = Some(id);
                            }
                            None => out.push(error("could not start the game")),
                        }
                        out
                    }
                    v1::envelope::Msg::Join(join) => {
                        // Re-attach to a live game. Note this *pumps*: joining
                        // is how a fresh client starts playing. A reconnect
                        // that only wants its state back sends `ResumeGame`.
                        let seat = seat_from_token(&join.seat_token);
                        let mut games = games.lock().await;
                        match games.get_mut(&join.game_id) {
                            Some(table) => match sit(&mut table.session, seat) {
                                Err(reason) => vec![error(reason)],
                                Ok(driving) => {
                                    state.rx = Some(table.fan.subscribe());
                                    state.seat = seat;
                                    state.driving = driving;
                                    state.game_id = Some(join.game_id.clone());
                                    table.session.describe(join.game_id.clone(), seat_names());
                                    let out = vec![table.session.game_static_envelope(seat)];
                                    let pumped = table.session.pump();
                                    publish(table, pumped);
                                    out
                                }
                            },
                            None => vec![error("no such game")],
                        }
                    }
                    v1::envelope::Msg::Resume(resume) => {
                        // Read-only reconnect: never pump here, or resuming
                        // would play the AI seats forward as a side effect of
                        // coming back. A driven chair is taken over again,
                        // because hanging up gave it back.
                        let seat = seat_from_token(&resume.seat_token);
                        let mut games = games.lock().await;
                        match games.get_mut(&resume.game_id) {
                            Some(table) => match sit(&mut table.session, seat) {
                                Err(reason) => vec![error(reason)],
                                Ok(driving) => {
                                    state.rx = Some(table.fan.subscribe());
                                    state.seat = seat;
                                    state.driving = driving;
                                    state.game_id = Some(resume.game_id.clone());
                                    // A resume follows a dropped socket, so
                                    // the payload that only ever arrives at
                                    // connect time is repeated. It cannot have
                                    // changed, and a client that still has it
                                    // simply overwrites it with the same
                                    // thing.
                                    let mut out = vec![table.session.game_static_envelope(seat)];
                                    out.extend(table.session.resume(seat, resume.last_seq));
                                    out
                                }
                            },
                            None => vec![error("no such game")],
                        }
                    }
                    v1::envelope::Msg::PlayerAction(action_msg) => {
                        let Some(id) = state.game_id.clone() else {
                            continue;
                        };
                        let Ok(action) =
                            serde_json::from_slice::<PlayerAction>(&action_msg.action_json)
                        else {
                            continue;
                        };
                        let mut games = games.lock().await;
                        match games.get_mut(&id) {
                            Some(table) => match table.session.act(state.seat, action) {
                                Ok(out) => {
                                    publish(table, out);
                                    Vec::new()
                                }
                                // Said out loud rather than swallowed: a
                                // program driving a seat has no screen to
                                // notice that nothing happened.
                                Err(reason) => vec![error(&reason)],
                            },
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
                }
            }
            Wake::Fan(Ok((seat, envelope))) => {
                if seat == state.seat.get() {
                    vec![envelope]
                } else {
                    Vec::new()
                }
            }
            Wake::Fan(Err(tokio::sync::broadcast::error::RecvError::Lagged(missed))) => {
                tracing::warn!(
                    missed,
                    seat = state.seat.get(),
                    "socket fell behind; rebuilding it"
                );
                let games = games.lock().await;
                match state.game_id.as_ref().and_then(|id| games.get(id)) {
                    Some(table) => table.session.snapshot(state.seat),
                    None => Vec::new(),
                }
            }
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
        // Each deadline is anchored to what it was armed for, so it restarts
        // when its seat is asked something new rather than every time a frame
        // from another seat wakes this task up.
        let mut armed = baylee_engine_server::Armed::default();
        loop {
            let now = tokio::time::Instant::now();
            armed.sync(&runner.clocks(), |clock| {
                now + std::time::Duration::from_secs(u64::from(clock.secs))
            });
            let next = armed.next();
            tokio::select! {
                () = deadline(next.map(|(_, at)| at)) => {
                    // The clock is the one thing the rules kernel must not
                    // own: it is deterministic and may not read a wall clock.
                    let Some((clock, _)) = next else { continue };
                    tracing::info!(seat = clock.seat.get(), "decision timed out");
                    armed.fired(clock);
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
                    // Read the clocks before handing the frame over: this
                    // frame may move the game, and after it has, a number
                    // belongs to a question nobody is being asked any more.
                    // Saturating rather than clamped-signed — a deadline
                    // already past is nought left, not a negative countdown.
                    let read = tokio::time::Instant::now();
                    let remaining = armed.remaining(|at| {
                        u32::try_from(at.saturating_duration_since(read).as_millis())
                            .unwrap_or(u32::MAX)
                    });
                    for out in runner.handle(envelope, &remaining) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo,
        SeatCapabilities, SeatController, SeatSpec,
    };

    fn preset() -> GamePreset {
        let entry = DeckEntry {
            card: baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
                .expect("registry contains Forest")
                .index,
            print: PrintRef::new(0),
        };
        let seat = |ai: bool| SeatSpec {
            controller: if ai {
                SeatController::Ai(AIProfile::default())
            } else {
                SeatController::Open
            },
            capabilities: SeatCapabilities::default(),
            deck: (0..60).map(|_| entry).collect(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        GamePreset {
            format: FormatId::Freeform,
            seed: 3,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            // Seat 0 is a person's chair, seat 1 is the house's.
            seats: vec![seat(false), seat(true)],
        }
    }

    /// Every launch is one of two programs, and three strings decide which.
    ///
    /// **All three or none**: an attachment missing any one of them is not a
    /// half-configured attach, it is a fall-through to [`listen`] — the dev
    /// harness, which has no authentication and lets a socket name its own
    /// seat. That is the right default for a developer on loopback and the
    /// wrong one for anything else, so the three-way requirement is the thing
    /// standing between them.
    #[test]
    fn an_attachment_takes_all_three_or_this_process_is_a_different_program() {
        let none = |_: &str| None;
        let full = [
            "--attach".to_string(),
            "ws://gateway/engine/ws".to_string(),
            "--game".to_string(),
            "g1".to_string(),
            "--token".to_string(),
            "t1".to_string(),
        ];

        assert_eq!(
            Attach::read(&full, none),
            Some(Attach {
                url: "ws://gateway/engine/ws".to_string(),
                game_id: "g1".to_string(),
                token: "t1".to_string(),
            })
        );

        for missing in ["--attach", "--game", "--token"] {
            let cut: Vec<String> = full
                .chunks(2)
                .filter(|pair| pair[0] != missing)
                .flatten()
                .cloned()
                .collect();
            assert_eq!(
                Attach::read(&cut, none),
                None,
                "without {missing} this launch is the dev harness, not an attach"
            );
        }

        assert_eq!(
            Attach::read(&["--attach".to_string()], none),
            None,
            "a flag with nothing behind it names nothing"
        );
        assert_eq!(Attach::read(&[], none), None);
    }

    /// **The agent writes these three flags and this reads them**, and each
    /// side was held against nothing but its own spelling.
    ///
    /// Nothing in the build links the two: the agent depends on the control
    /// plane and on nothing else in this workspace, which is what lets it run
    /// on a machine that has never heard of a card. So a rename on either
    /// side leaves both crates green, and what it produces is an engine that
    /// comes up as the **dev harness** instead — a loopback listener with no
    /// authentication that hands a socket whichever seat it names — reported
    /// to nobody, while the gateway waits for an attachment that never
    /// arrives. The test that would have caught it runs all three processes
    /// and is `#[ignore]`d.
    ///
    /// A dev-dependency is the whole cost, and it points the way round a
    /// test may point: the reader depends on the writer.
    #[test]
    fn the_three_flags_are_the_ones_the_agent_writes() {
        let argv = baylee_agent::engine_argv("ws://gw:28766/engine/ws", "g-7", "engine-token");

        assert_eq!(
            Attach::read(&argv, |_| None),
            Some(Attach {
                url: "ws://gw:28766/engine/ws".to_string(),
                game_id: "g-7".to_string(),
                token: "engine-token".to_string(),
            }),
            "what the agent wrote is what this process attaches with"
        );
    }

    /// The command line wins, field by field, because that is how the agent
    /// starts this process — and the environment is how a person starts it by
    /// hand, so a person's variable must not quietly override the agent's
    /// argument in a launch that has both.
    ///
    /// An **empty** variable is not an answer either: it attaches with an
    /// empty token otherwise, and the gateway refusing that reads as a
    /// rejected engine rather than as a launch that was never given one.
    #[test]
    fn the_command_line_beats_the_environment_field_by_field() {
        let env = |name: &str| {
            Some(
                match name {
                    "BAYLEE_ATTACH_URL" => "ws://from-env/engine/ws",
                    "BAYLEE_GAME" => "from-env",
                    "BAYLEE_ENGINE_TOKEN" => "env-token",
                    _ => return None,
                }
                .to_string(),
            )
        };

        let mixed = Attach::read(
            &[
                "--attach".to_string(),
                "ws://from-flag/engine/ws".to_string(),
            ],
            env,
        )
        .expect("the environment supplies the other two");
        assert_eq!(mixed.url, "ws://from-flag/engine/ws", "the flag wins");
        assert_eq!(
            mixed.game_id, "from-env",
            "and the rest still comes from the environment"
        );
        assert_eq!(mixed.token, "env-token");

        let empty = |name: &str| {
            if name == "BAYLEE_ENGINE_TOKEN" {
                Some(String::new())
            } else {
                env(name)
            }
        };
        assert_eq!(
            Attach::read(&[], empty),
            None,
            "an exported-but-empty token is a launch with no token"
        );
    }

    /// **The dev harness binds loopback**, and that is a decision rather than
    /// a convenience: it has no authentication, so a socket names its own
    /// seat and is handed that seat's hidden information. Binding it
    /// anywhere else hands out every hand at the table.
    ///
    /// Asserted through `is_loopback` and not by retyping the constant, so
    /// the test is about the property and not about the spelling.
    #[test]
    fn the_dev_harness_binds_somewhere_only_this_machine_can_reach() {
        let address: std::net::IpAddr = DEFAULT_BIND.parse().expect("a bindable address");
        assert!(
            address.is_loopback(),
            "{DEFAULT_BIND} is reachable from off this machine, and this \
             listener hands a socket whichever seat it asks for"
        );
    }

    /// The dev harness has no authentication, so this field carries no
    /// authority here and would otherwise be read by nothing — which is
    /// exactly what makes it the right place to put a seat number. Against
    /// the gateway the same field still says "prove it".
    #[test]
    fn a_seat_token_is_a_seat_number_here_and_anything_unreadable_is_the_first_seat() {
        assert_eq!(seat_from_token("1"), PlayerId::new(1));
        assert_eq!(seat_from_token(" 2 "), PlayerId::new(2), "trimmed");
        assert_eq!(seat_from_token(""), PlayerId::new(0));
        assert_eq!(seat_from_token("not a number"), PlayerId::new(0));
        assert_eq!(seat_from_token("-1"), PlayerId::new(0));
        assert_eq!(
            seat_from_token("999"),
            PlayerId::new(0),
            "a number no seat could be is the first seat and not a wrap"
        );
    }

    /// Three answers, deliberately different. A free human chair is sat in,
    /// an AI chair is taken over and owed back, and anything else is
    /// refused — including a seat another socket is already answering for.
    /// Two sockets on one AI chair is not a shared seat, it is two programs
    /// holding the same hand, racing each other for every question.
    #[test]
    fn sitting_down_takes_over_a_house_chair_and_never_a_taken_one() {
        let mut session = Session::new(&preset()).expect("a two-seat table");

        assert_eq!(sit(&mut session, PlayerId::new(0)), Ok(false), "a person's");
        assert_eq!(
            sit(&mut session, PlayerId::new(1)),
            Ok(true),
            "the house chair is taken over, and owed back"
        );
        assert!(
            matches!(
                session.seat_kind(PlayerId::new(1)),
                Some(SeatKind::Driven(_))
            ),
            "and it says so afterwards"
        );
        assert!(
            sit(&mut session, PlayerId::new(1)).is_err(),
            "a second socket on one AI chair would race the first"
        );
        assert!(
            sit(&mut session, PlayerId::new(9)).is_err(),
            "a seat that is not at this table"
        );
    }

    /// Decoding happens *before* the preset can be validated, so this
    /// budget is the only thing between an unauthenticated peer and an
    /// arbitrary allocation. tungstenite's own default is 64 MiB, three
    /// orders of magnitude above the largest message this protocol has —
    /// dropping the setting is a change nothing else would notice.
    #[test]
    fn the_frame_budget_is_set_and_is_far_below_the_library_default() {
        let config = ws_config();
        assert_eq!(config.max_message_size, Some(4 << 20));
        assert_eq!(config.max_frame_size, Some(4 << 20));
        assert!(
            config.max_message_size < Some(64 << 20),
            "the default would be the budget again"
        );
    }
}
