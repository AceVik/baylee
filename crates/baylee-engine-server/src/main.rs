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
#![deny(unsafe_code)]

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

mod pump_config;

use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::{SeatKind, Session, preset};
use baylee_protocol::v1::{self, Envelope};
use baylee_view::SeatSetting;
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

/// What `baylee-engine-server --help` prints, and a refused command line
/// after its reason.
const USAGE: &str = "\
usage: baylee-engine-server --attach <url> --game <id>
           play one game for a gateway, as an agent starts it; the game's
           token is in BAYLEE_ENGINE_TOKEN (--token <token> is read too, but
           an argument is in the process list for every user of the machine)
       baylee-engine-server
           the dev harness on 127.0.0.1:28765 (PORT, BAYLEE_BIND): no
           authentication, every hand to whoever asks; never bind it publicly
       baylee-engine-server --help | -h
           this text

BAYLEE_ATTACH_URL, BAYLEE_GAME and BAYLEE_ENGINE_TOKEN stand in for the
three flags; an attachment takes all three or none.";

#[tokio::main]
async fn main() {
    // The command line's shape first, before the environment is read or a
    // socket opened: only a launch that says nothing is the harness.
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| {
            arg.into_string()
                .unwrap_or_else(|_| refuse("an argument is not UTF-8"))
        })
        .collect();
    match Asked::read(&args).unwrap_or_else(|why| refuse(&why)) {
        Asked::Help => {
            println!("{USAGE}");
            return;
        }
        Asked::Start => {}
    }
    tracing_subscriber_init();
    let attach = Attach::discover(&args).unwrap_or_else(|partial| {
        tracing::error!(%partial, "refusing to start");
        std::process::exit(2);
    });
    if let Some(attach) = attach {
        let game_id = attach.game_id.clone();
        match Box::pin(attached::run(attach)).await {
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

/// Stops on a command line this program does not take: the reason and
/// [`USAGE`] on standard error, exit status 2, nothing opened.
fn refuse(why: &str) -> ! {
    eprintln!("baylee-engine-server: {why}\n\n{USAGE}");
    std::process::exit(2);
}

/// What a command line asks for, read before anything else is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asked {
    /// `-h` or `--help`: print [`USAGE`], start nothing.
    Help,
    /// Start: attached when [`Attach::read`] finds an attachment, the dev
    /// harness when the launch names none.
    Start,
}

impl Asked {
    /// The flags an attachment is named with, each followed by its value.
    const FLAGS: [&'static str; 3] = ["--attach", "--game", "--token"];

    /// Reads a command line's shape: the three flags, each at most once and
    /// each with a value, or a request for help.
    ///
    /// A refusal names a flag and never a value: a value may be a token.
    ///
    /// # Errors
    /// Any other argument, a flag given twice, or a flag with no value.
    fn read(args: &[String]) -> Result<Self, String> {
        let mut seen: Vec<&str> = Vec::new();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            let arg = arg.as_str();
            if arg == "-h" || arg == "--help" {
                return Ok(Self::Help);
            }
            let Some(flag) = Self::FLAGS.into_iter().find(|flag| *flag == arg) else {
                return Err(if arg.starts_with('-') {
                    let name = arg.split_once('=').map_or(arg, |(name, _)| name);
                    format!("{name:?} is not a flag this program takes")
                } else {
                    "a value with no flag before it".to_string()
                });
            };
            if seen.contains(&flag) {
                return Err(format!("{flag} is given twice"));
            }
            seen.push(flag);
            if rest.next().is_none_or(|value| value.starts_with('-')) {
                return Err(format!("{flag} takes a value"));
            }
        }
        Ok(Self::Start)
    }
}

/// Where an attached engine dials, and what it proves it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attach {
    /// The gateway's engine socket (`ws://…/engine/ws`), or its unix socket
    /// (`unix:<path>`), on which the engine plane is at `/engine/ws`.
    pub url: String,
    /// The game this process was started for.
    pub game_id: String,
    /// One game's worth of authority, issued by the gateway and handed over
    /// by the agent. It never leaves the machine the agent runs on.
    pub token: String,
}

/// A launch that names an attachment and leaves part of it out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialAttach {
    /// What it would need: the flag of each value it has none for.
    missing: Vec<&'static str>,
}

impl std::fmt::Display for PartialAttach {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "an attachment takes all of --attach, --game and --token and this launch has no {}; \
             give all three, or none for the dev harness",
            self.missing.join(", ")
        )
    }
}

impl Attach {
    /// The attachment this launch was given, if it was given one.
    ///
    /// Command line first (that is how the agent starts it), environment
    /// second (that is how a person starts it by hand).
    ///
    /// # Errors
    /// [`PartialAttach`], as [`Attach::read`].
    fn discover(args: &[String]) -> Result<Option<Self>, PartialAttach> {
        Self::read(args, |name| std::env::var(name).ok())
    }

    /// The attachment a given command line and environment describe.
    ///
    /// Split from [`Attach::discover`] so that the precedence and the
    /// all-or-nothing rule can be asserted: a function that read the process
    /// arguments itself could not be tested for what it does with them, which
    /// is the same bargain `import::plan` makes with the clock.
    ///
    /// The all-or-nothing half is the one worth having a test for. `None` is
    /// [`listen`], the dev harness, which has no authentication at all and
    /// hands a socket whichever seat it names, so only a launch that says
    /// nothing about an attachment gets it. A launch that says anything and
    /// leaves the rest out was meant to attach, and it stops.
    ///
    /// # Errors
    /// [`PartialAttach`] when some of the three are named and not all.
    fn read(
        args: &[String],
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Option<Self>, PartialAttach> {
        let flag = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        // An empty variable is no value. A shell that exports
        // `BAYLEE_ENGINE_TOKEN=` from an unset value would otherwise attach
        // with an empty token and be refused by the gateway, which reads as
        // a rejected engine rather than as a launch that was never given one.
        // It still says that this launch was meant to attach.
        let from_env = |name: &str| env(name).filter(|v| !v.is_empty());
        let named = |flag: &str, var: &str| args.iter().any(|a| a == flag) || env(var).is_some();
        let url = flag("--attach").or_else(|| from_env("BAYLEE_ATTACH_URL"));
        let game_id = flag("--game").or_else(|| from_env("BAYLEE_GAME"));
        let token = flag("--token").or_else(|| from_env("BAYLEE_ENGINE_TOKEN"));
        if let (Some(url), Some(game_id), Some(token)) = (&url, &game_id, &token) {
            return Ok(Some(Self {
                url: url.clone(),
                game_id: game_id.clone(),
                token: token.clone(),
            }));
        }
        let said_anything = named("--attach", "BAYLEE_ATTACH_URL")
            || named("--game", "BAYLEE_GAME")
            || named("--token", "BAYLEE_ENGINE_TOKEN");
        if !said_anything {
            return Ok(None);
        }
        let missing = [
            ("--attach", url.is_none()),
            ("--game", game_id.is_none()),
            ("--token", token.is_none()),
        ]
        .into_iter()
        .filter_map(|(flag, absent)| absent.then_some(flag))
        .collect();
        Err(PartialAttach { missing })
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
    let port = listener
        .local_addr()
        .expect("a bound listener has an address")
        .port();
    tracing::info!(port, "baylee-engine-server listening");
    if let Some(path) = std::env::var_os("BAYLEE_PORT_FILE") {
        write_port_file(std::path::Path::new(&path), port)
            .unwrap_or_else(|e| panic!("BAYLEE_PORT_FILE {}: {e}", path.display()));
    }
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

/// Writes the port the harness is listening on to `path`, for whoever
/// started it with `PORT=0` and needs to know where it went (#279).
///
/// Its e2e tests are that caller. They used to bind `127.0.0.1:0`, read the
/// number, let go and start this binary on it, and any other process could
/// take the port in between. The gateway's `write_port_file` closed the same
/// gap in its own suite; this is its twin, because the gateway links no
/// engine crate and this binary links no gateway.
///
/// Written beside the target and renamed over it, so a reader polling for
/// the file never reads half a number. Called once the socket is bound, so
/// the file's appearance also says a dial will be accepted.
fn write_port_file(path: &std::path::Path, port: u16) -> std::io::Result<()> {
    let partial = path.with_extension("partial");
    std::fs::write(&partial, format!("{port}\n"))?;
    std::fs::rename(&partial, path)
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

/// Tells a seat the table is open, behind the frames already on the fan for
/// it.
///
/// This harness has no curtain to wait behind: it is loopback and dev-only,
/// runs no clock, and pumps the moment a game is created. It says `Curtain`
/// anyway, last in every batch that seats a socket, because every host that
/// serves a seat does (#256); a client pointed here would otherwise wait
/// behind a curtain nothing raises. Through the fan rather than the reply, so
/// it lands after the pump's views rather than before them.
fn publish_curtain(table: &Table, seat: PlayerId) {
    let _ = table
        .fan
        .send((seat.get(), baylee_engine_server::curtain()));
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
///
/// And a read buffer of 8 KiB rather than 128: what arrives here is a seat's
/// answer or an attach, hundreds of bytes, and tungstenite zeroes the whole
/// buffer before every read.
fn ws_config() -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
    tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(4 << 20))
        .max_frame_size(Some(4 << 20))
        .read_buffer_size(8 << 10)
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
    /// Whether the table's session still owes house AI steps after the last
    /// slice on this connection.
    pumping: bool,
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
    /// The table still owes house AI steps after a bounded slice.
    Pump,
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
    // Nagle's algorithm off, as on every socket of the topology
    // (`attached::dial`).
    if let Err(e) = stream.set_nodelay(true) {
        tracing::debug!(error = %e, "TCP_NODELAY was not set on a connection");
    }
    let mut ws = tokio_tungstenite::accept_async_with_config(stream, Some(ws_config())).await?;
    let mut state = Sitting {
        game_id: None,
        seat: SEAT,
        driving: false,
        rx: None,
        pumping: false,
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
            table.session.tell_time(wall_ms());
            table.session.release(state.seat);
            let out = table
                .session
                .pump_slice(baylee_engine_server::ATTACHED_PUMP_SLICE);
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
            biased;
            frame = ws.next() => Wake::Frame(frame),
            got = fanned(state.rx.as_mut()) => Wake::Fan(got),
            () = tokio::task::yield_now(), if state.pumping => Wake::Pump,
        };
        let replies: Vec<Envelope> = match wake {
            // The socket hung up, or the table is gone — and on this server
            // the table only goes when the process does.
            Wake::Frame(None)
            | Wake::Fan(Err(tokio::sync::broadcast::error::RecvError::Closed)) => break,
            Wake::Pump => {
                let mut games = games.lock().await;
                if let Some(table) = state.game_id.as_ref().and_then(|id| games.get_mut(id)) {
                    table.session.tell_time(wall_ms());
                    let out = table
                        .session
                        .pump_slice(baylee_engine_server::ATTACHED_PUMP_SLICE);
                    state.pumping = table.session.needs_pump();
                    publish(table, out);
                } else {
                    state.pumping = false;
                }
                Vec::new()
            }
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
                                table.session.tell_time(wall_ms());
                                table.session.describe(id.clone(), seat_names());
                                out.push(table.session.game_static_envelope(SEAT));
                                let pumped = table
                                    .session
                                    .pump_slice(baylee_engine_server::ATTACHED_PUMP_SLICE);
                                state.pumping = table.session.needs_pump();
                                publish(table, pumped);
                                publish_curtain(table, SEAT);
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
                                    // A fresh client holds none of the log.
                                    table.session.retell(seat);
                                    table.session.tell_time(wall_ms());
                                    let pumped = table
                                        .session
                                        .pump_slice(baylee_engine_server::ATTACHED_PUMP_SLICE);
                                    state.pumping = table.session.needs_pump();
                                    publish(table, pumped);
                                    publish_curtain(table, seat);
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
                                    state.pumping = table.session.needs_pump();
                                    // A resume follows a dropped socket, so
                                    // the payload that only ever arrives at
                                    // connect time is repeated. It cannot have
                                    // changed, and a client that still has it
                                    // simply overwrites it with the same
                                    // thing.
                                    let mut out = vec![table.session.game_static_envelope(seat)];
                                    out.extend(table.session.resume(seat, resume.last_seq));
                                    out.push(baylee_engine_server::curtain());
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
                            baylee_core::wire::decode::<PlayerAction>(&action_msg.action)
                        else {
                            continue;
                        };
                        let mut games = games.lock().await;
                        if let Some(table) = games.get_mut(&id) {
                            table.session.tell_time(wall_ms());
                        }
                        match games.get_mut(&id) {
                            Some(table) => match table.session.act_slice(
                                state.seat,
                                action,
                                baylee_engine_server::ATTACHED_PUMP_SLICE,
                            ) {
                                Ok(out) => {
                                    state.pumping = table.session.needs_pump();
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
                    v1::envelope::Msg::SeatSetting(setting_msg) => {
                        let Some(id) = state.game_id.clone() else {
                            continue;
                        };
                        let Ok(setting) =
                            baylee_core::wire::decode::<SeatSetting>(&setting_msg.setting)
                        else {
                            continue;
                        };
                        let mut games = games.lock().await;
                        if let Some(table) = games.get_mut(&id) {
                            table.session.tell_time(wall_ms());
                        }
                        match games.get_mut(&id) {
                            Some(table) => match table.session.seat_setting(state.seat, setting) {
                                Ok(out) => {
                                    publish(table, out);
                                    Vec::new()
                                }
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
                let mut batch = Vec::new();
                if seat == state.seat.get() {
                    batch.push(envelope);
                }
                if let Some(rx) = state.rx.as_mut() {
                    while let Ok((next_seat, next_env)) = rx.try_recv() {
                        if next_seat == state.seat.get() {
                            batch.push(next_env);
                        }
                    }
                }
                batch
            }
            Wake::Fan(Err(tokio::sync::broadcast::error::RecvError::Lagged(missed))) => {
                tracing::warn!(
                    missed,
                    seat = state.seat.get(),
                    "socket fell behind; rebuilding it"
                );
                let mut games = games.lock().await;
                match state.game_id.as_ref().and_then(|id| games.get_mut(id)) {
                    Some(table) => table.session.snapshot(state.seat),
                    None => Vec::new(),
                }
            }
        };
        if !replies.is_empty() {
            for reply in replies {
                ws.feed(tokio_tungstenite::tungstenite::Message::Binary(
                    reply.encode_to_vec().into(),
                ))
                .await?;
            }
            ws.flush().await?;
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

/// The wall time the game log stamps its lines with (#300). It reaches the
/// session, never the engine, which reads no clock.
fn wall_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// The dev duel: Allytifact vs Victory, human on seat 0, AI on seat 1.
fn acceptance_duel_preset() -> GamePreset {
    // Embedded, not read from the build machine's checkout at run time: an
    // installed binary carries no source tree beside it.
    const TEXT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/acceptance-decks.txt"
    ));
    let allytifact = baylee_cards::decks::load_acceptance(TEXT, "Allytifact").expect("Allytifact");
    let victory = baylee_cards::decks::load_acceptance(TEXT, "Victory").expect("Victory");
    let mut preset = baylee_cards::decks::preset_for(1, &allytifact, &victory);
    // Seat 0 is the connecting human.
    preset.seats[0].controller = baylee_core::preset::SeatController::Open;
    preset.house_rules.order_triggers = true;
    preset.house_rules.divide_trample = true;
    preset
}

/// The attached engine: one socket to the gateway, one game, then exit.
mod attached {
    use super::{Attach, wall_ms, ws_config};
    use baylee_engine_server::EngineRunner;
    use baylee_protocol::v1::{self, Envelope};
    use futures_util::{SinkExt, StreamExt};
    use prost::Message as _;
    use tokio_tungstenite::tungstenite::Message;

    /// Plays one game against the gateway and returns when it ends.
    pub async fn run(attach: Attach) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pump_budget = super::pump_config::from_env()?;
        let runner = EngineRunner::new()
            .with_pump_budget(pump_budget)
            .with_foresight(foresight()?);
        // `unix:<path>` is the gateway's unix socket (`BAYLEE_UNIX_SOCKET`),
        // which an agent on the same machine hands its engines; the engine
        // plane is at the usual path on it.
        #[cfg(unix)]
        if let Some(path) = baylee_protocol::unix_socket(&attach.url) {
            let socket = tokio::net::UnixStream::connect(path).await?;
            let (ws, _) = tokio_tungstenite::client_async_with_config(
                "ws://localhost/engine/ws",
                socket,
                Some(ws_config()),
            )
            .await?;
            return Box::pin(play(ws, attach, runner)).await;
        }
        let ws = dial(&attach.url).await?;
        Box::pin(play(ws, attach, runner)).await
    }

    /// What the house's search may spend here, from the operator's settings
    /// (`baylee_engine_server::foresight`), said once in the log.
    fn foresight()
    -> Result<Option<baylee_gamehost::HostForesight>, Box<dyn std::error::Error + Send + Sync>>
    {
        let settings = baylee_engine_server::foresight::Settings::from_env()?;
        let host = settings.host()?;
        match (&host, settings.on) {
            (Some(_), _) => tracing::info!(
                searches = settings.searches,
                work_percent = settings.percent,
                wall_ms = settings.wall_ms,
                "the house searches within its levels' bounds"
            ),
            (None, true) => tracing::warn!(
                "BAYLEE_FORESIGHT_SLOTS is not set (the agent sets it): the house answers \
                 without searching"
            ),
            (None, false) => {
                tracing::info!("BAYLEE_FORESIGHT=off: the house answers without searching");
            }
        }
        Ok(host)
    }

    /// Dials the gateway over TCP with Nagle's algorithm off (`TCP_NODELAY`).
    ///
    /// An engine sends its frames in bursts (a view, then the question about
    /// it), and with Nagle on each after the first waits until the gateway
    /// acknowledges the one before it, which Linux delays by up to 40 ms (the
    /// gateway's `send_at_once` has the numbers).
    pub(super) async fn dial(
        url: &str,
    ) -> Result<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tokio_tungstenite::tungstenite::Error,
    > {
        let (ws, _) =
            tokio_tungstenite::connect_async_with_config(url, Some(ws_config()), true).await?;
        Ok(ws)
    }

    /// The game itself, on whichever socket reached the gateway.
    async fn play<S>(
        mut ws: tokio_tungstenite::WebSocketStream<S>,
        attach: Attach,
        mut runner: EngineRunner,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        // The first frame proves this process is the one the gateway asked an
        // agent to start. Nothing else on this socket is authenticated,
        // because nothing else needs to be: the gateway closes it otherwise.
        send(&mut ws, &engine_hello(&attach)).await?;
        tracing::info!(game_id = attach.game_id, url = attach.url, "attached");

        // Each deadline is anchored to what it was armed for, so it restarts
        // when its seat is asked something new rather than every time a frame
        // from another seat wakes this task up.
        let mut armed = baylee_engine_server::Armed::default();
        // The curtain's deadline (#256). The table's rather than a seat's, so
        // not a `Clock`: armed on the first turn of this loop after the game
        // is built, whether or not any seat has attached, and dropped once
        // the curtain is up.
        let mut curtain_at = None;
        loop {
            let now = tokio::time::Instant::now();
            armed.sync(&runner.clocks(), |clock| {
                now + std::time::Duration::from_secs(u64::from(clock.secs))
            });
            curtain_at =
                (runner.curtain_pending() && runner.entrance_deadline().is_none()).then(|| {
                    curtain_at.unwrap_or_else(|| {
                        now + std::time::Duration::from_secs(u64::from(
                            baylee_engine_server::CURTAIN_SECS,
                        ))
                    })
                });
            let arrival = runner
                .entrance_deadline()
                .map(|at| now + std::time::Duration::from_millis(at.saturating_sub(wall_ms())));
            // The record that has waited long enough goes to the gateway even
            // when nothing moves (#323). Wall time, like the entrance.
            let record_at = runner
                .record_deadline()
                .map(|at| now + std::time::Duration::from_millis(at.saturating_sub(wall_ms())));
            let next = armed.next();
            let needs_pump = runner.needs_pump();
            tokio::select! {
                biased;
                () = deadline(record_at) => {
                    runner.tell_time(wall_ms());
                    send_batch(&mut ws, &runner.flush_record_due()).await?;
                }
                () = deadline(arrival) => {
                    runner.tell_time(wall_ms());
                    send_batch(&mut ws, &runner.finish_entrance()).await?;
                }
                () = deadline(curtain_at) => {
                    tracing::info!(game_id = attach.game_id, "table preparation timed out");
                    runner.tell_time(wall_ms());
                    send_batch(&mut ws, &runner.preparation_expired()).await?;
                }
                () = deadline(next.map(|(_, at)| at)) => {
                    // The clock is the one thing the rules kernel must not
                    // own: it is deterministic and may not read a wall clock.
                    let Some((clock, _)) = next else { continue };
                    tracing::info!(seat = clock.seat.get(), "decision timed out");
                    armed.fired(clock);
                    runner.tell_time(wall_ms());
                    send_batch(&mut ws, &runner.timeout(clock)).await?;
                }
                frame = ws.next() => {
                    let Some(frame) = frame else { break };
                    let frame = frame?;
                    if !frame.is_binary() {
                        continue;
                    }
                    let envelope = Envelope::decode(frame.into_data())?;
                    // The gateway's one word to an engine it will not keep
                    // (#271): said as this process's failure, in its words.
                    if let Some(v1::envelope::Msg::Error(refused)) = envelope.msg {
                        return Err(refused.message.into());
                    }
                    // Started for a replay job, not a game (`docs/protocol.md`
                    // §"Replays"): project the record, send the tape, exit.
                    if runner.session().is_none()
                        && let Some(v1::envelope::Msg::ProjectRecord(order)) = envelope.msg
                    {
                        return project(ws, order).await;
                    }
                    let remaining = remaining_now(&armed);
                    runner.tell_time(wall_ms());
                    send_batch(&mut ws, &runner.handle(envelope, &remaining)).await?;
                }
                () = tokio::task::yield_now(), if needs_pump => {
                    let remaining = remaining_now(&armed);
                    runner.tell_time(wall_ms());
                    send_batch(&mut ws, &runner.pump_slice(&remaining)).await?;
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

    fn engine_hello(attach: &Attach) -> Envelope {
        Envelope {
            msg: Some(v1::envelope::Msg::EngineHello(v1::EngineHello {
                game_id: attach.game_id.clone(),
                token: attach.token.clone(),
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
                pid: std::process::id(),
                build: baylee_build::short().to_owned(),
            })),
        }
    }

    /// Reads the currently armed clocks against `Instant::now()`.
    fn remaining_now(
        armed: &baylee_engine_server::Armed<tokio::time::Instant>,
    ) -> Vec<(baylee_engine_server::Clock, u32)> {
        let read = tokio::time::Instant::now();
        armed.remaining(|at| {
            u32::try_from(at.saturating_duration_since(read).as_millis()).unwrap_or(u32::MAX)
        })
    }

    /// A replay job (`baylee_engine_server::projection`): the projection runs
    /// on a blocking thread and its pieces go out as they are made; then the
    /// socket closes and the process exits.
    async fn project<S>(
        mut ws: tokio_tungstenite::WebSocketStream<S>,
        order: v1::ProjectRecord,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let job = order.job.clone();
        let worker = tokio::task::spawn_blocking(move || {
            baylee_engine_server::projection::run(&order, baylee_build::short(), &mut |piece| {
                let _ = tx.send(piece);
            })
        });
        while let Some(piece) = rx.recv().await {
            send(
                &mut ws,
                &Envelope {
                    msg: Some(v1::envelope::Msg::TapeChunk(piece)),
                },
            )
            .await?;
        }
        let stop = worker.await?;
        tracing::info!(job, stop = ?stop.stop, inputs = stop.inputs, "replay projected");
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

    async fn send<S>(
        ws: &mut tokio_tungstenite::WebSocketStream<S>,
        envelope: &Envelope,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        ws.send(Message::Binary(envelope.encode_to_vec().into()))
            .await?;
        Ok(())
    }

    async fn send_batch<S>(
        ws: &mut tokio_tungstenite::WebSocketStream<S>,
        envelopes: &[Envelope],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        if envelopes.is_empty() {
            return Ok(());
        }
        for envelope in envelopes {
            ws.feed(Message::Binary(envelope.encode_to_vec().into()))
                .await?;
        }
        ws.flush().await?;
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
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
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
            rules: baylee_core::card_metadata::RulesMetadata::default(),
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

    fn line(args: &[&str]) -> Vec<String> {
        args.iter().map(ToString::to_string).collect()
    }

    /// Help is asked for anywhere a flag may stand, and starts nothing.
    #[test]
    fn help_starts_nothing() {
        for args in [
            &["--help"][..],
            &["-h"][..],
            &["--attach", "ws://gw/engine/ws", "--help"][..],
        ] {
            assert_eq!(Asked::read(&line(args)), Ok(Asked::Help), "{args:?}");
        }
        assert_eq!(Asked::read(&[]), Ok(Asked::Start), "the dev harness");
        let argv = baylee_agent::engine_argv("unix:/run/baylee/gateway.sock", "g-7");
        assert_eq!(
            Asked::read(&argv),
            Ok(Asked::Start),
            "what the agent writes"
        );
        let with_token = line(&["--attach", "ws://gw", "--game", "g", "--token", "t"]);
        assert_eq!(Asked::read(&with_token), Ok(Asked::Start));
    }

    /// Anything else on the command line stops the start, and the refusal
    /// names a flag, never a value: a value may be a token.
    #[test]
    fn an_argument_it_does_not_take_is_refused_and_no_value_is_repeated() {
        for (args, said) in [
            (&["--version"][..], "\"--version\""),
            (&["--port", "28765"][..], "\"--port\""),
            (&["--token=s3cret"][..], "\"--token\""),
            (&["--bind=0.0.0.0"][..], "\"--bind\""),
            (&["-H"][..], "\"-H\""),
            (&["s3cret"][..], "no flag"),
            (&["--bogus", "--help"][..], "\"--bogus\""),
            (&["--attach"][..], "--attach takes a value"),
            (
                &["--game", "--attach", "ws://gw"][..],
                "--game takes a value",
            ),
            (&["--game", "a", "--game", "b"][..], "--game is given twice"),
            (
                &["--attach", "ws://gw", "--game", "g", "s3cret"][..],
                "no flag",
            ),
        ] {
            let refused = Asked::read(&line(args)).expect_err(said);
            assert!(refused.contains(said), "{args:?}: {refused}");
            assert!(!refused.contains("s3cret"), "{args:?}: {refused}");
        }
        assert!(USAGE.contains("127.0.0.1:28765"));
        assert_eq!(DEFAULT_BIND, "127.0.0.1");
        assert_eq!(DEFAULT_PORT, 28765);
    }

    /// Every launch is one of two programs, and three strings decide which.
    ///
    /// **All three or none**: [`listen`] is the dev harness, which has no
    /// authentication and lets a socket name its own seat. That is the right
    /// default for a developer on loopback who said nothing, and the wrong
    /// one for a launch that was meant to attach, so an attachment missing
    /// any one of them starts neither program.
    #[test]
    fn an_attachment_takes_all_three_or_this_process_does_not_start() {
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
            Ok(Some(Attach {
                url: "ws://gateway/engine/ws".to_string(),
                game_id: "g1".to_string(),
                token: "t1".to_string(),
            }))
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
                Err(PartialAttach {
                    missing: vec![missing]
                }),
                "without {missing} this launch starts nothing"
            );
        }

        assert_eq!(
            Attach::read(&["--attach".to_string()], none),
            Err(PartialAttach {
                missing: vec!["--attach", "--game", "--token"]
            }),
            "a flag with nothing behind it says this launch was meant to attach"
        );
        assert_eq!(Attach::read(&[], none), Ok(None), "the dev harness");
    }

    /// Named anywhere, with any value at all, an attachment is what was
    /// asked for: a variable left empty must not turn a launch into the
    /// unauthenticated harness.
    #[test]
    fn a_launch_that_names_part_of_an_attachment_is_never_the_dev_harness() {
        for var in ["BAYLEE_ATTACH_URL", "BAYLEE_GAME", "BAYLEE_ENGINE_TOKEN"] {
            for value in ["", "something"] {
                let env = |name: &str| (name == var).then(|| value.to_string());
                assert!(Attach::read(&[], env).is_err(), "{var}={value:?}");
            }
        }
        let unrelated = |name: &str| (name == "PORT").then(|| "0".to_string());
        assert_eq!(Attach::read(&[], unrelated), Ok(None));
    }

    /// What a refused launch says names the flags and carries no value.
    #[test]
    fn a_refused_launch_says_what_is_missing_and_nothing_it_was_given() {
        let env = |name: &str| (name == "BAYLEE_ENGINE_TOKEN").then(|| "s3cret".to_string());
        let said = Attach::read(&[], env).unwrap_err().to_string();
        assert!(said.contains("--attach, --game"), "{said}");
        assert!(!said.contains("s3cret"), "{said}");
    }

    /// **The agent writes these three flags and this reads them**, and each
    /// side was held against nothing but its own spelling.
    ///
    /// Nothing in the build links the two: the agent depends on the control
    /// plane and on nothing else in this workspace, which is what lets it run
    /// on a machine that has never heard of a card. So a rename on either
    /// side leaves both crates green, and what it produces is an engine that
    /// does not start, while the gateway waits for an attachment that never
    /// arrives. The test that would have caught it runs all three processes
    /// and is `#[ignore]`d.
    ///
    /// A dev-dependency is the whole cost, and it points the way round a
    /// test may point: the reader depends on the writer.
    #[test]
    fn what_the_agent_writes_is_what_this_process_reads() {
        let argv = baylee_agent::engine_argv("ws://gw:28766/engine/ws", "g-7");
        assert!(
            !argv.iter().any(|arg| arg.contains("token")),
            "a token in an argument is in the process list: {argv:?}"
        );
        let env = |name: &str| {
            (name == baylee_agent::ENGINE_TOKEN_VAR).then(|| "engine-token".to_string())
        };

        assert_eq!(
            Attach::read(&argv, env),
            Ok(Some(Attach {
                url: "ws://gw:28766/engine/ws".to_string(),
                game_id: "g-7".to_string(),
                token: "engine-token".to_string(),
            })),
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
        .expect("a whole attachment")
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
            Err(PartialAttach {
                missing: vec!["--token"]
            }),
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

    /// The port file (#279) holds the port and a newline, replaces what an
    /// earlier run left, and leaves no half-written file behind it.
    #[test]
    fn the_port_file_says_the_port_and_nothing_else() {
        let dir =
            std::env::temp_dir().join(format!("baylee-engine-port-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("engine.port");
        std::fs::write(&path, "28765\n").unwrap();

        write_port_file(&path, 49_152).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "49152\n");
        let left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, [std::ffi::OsString::from("engine.port")]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The engine's socket to the gateway is opened with Nagle's algorithm
    /// off.
    #[tokio::test]
    async fn the_gateway_is_dialled_with_nagle_off() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/engine/ws", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = tokio_tungstenite::accept_async(stream).await;
        });
        let ws = attached::dial(&url).await.unwrap();
        let tokio_tungstenite::MaybeTlsStream::Plain(tcp) = ws.get_ref() else {
            panic!("a plain TCP socket");
        };
        assert!(tcp.nodelay().unwrap());
    }
}
