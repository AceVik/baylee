//! The bridge: a [`SeatCore`] on a real socket, a real mind and a real
//! clock.
//!
//! Everything the seat decides is decided in [`SeatCore`]; this module only
//! carries it out. It reads frames and hands them over, runs the mind's
//! answer as a task with a deadline, sends what is to be sent, and owns the
//! three things a core has no way to own:
//!
//! - **time**: a question's budget runs out here ([`SeatCore::expired`]), and
//!   a woken decision is not answered sooner than [`PlayOptions::min_think`]
//!   after it was asked, so a person across the table can follow the game;
//! - **the socket**: a dropped one is dialled again ([`SeatLink`]) and the
//!   table asked for what the seat missed ([`SeatCore::resumed`]);
//! - **a mind that is down**: the socket is left, so the table sees an
//!   absent player and its house holds the chair, and the mind is asked
//!   every [`PlayOptions::mind_poll`] whether it is ready again;
//! - **a room that closed**: a table that could not start, or whose engine
//!   was lost, ends without a word on the seat's socket, which stays open;
//!   the room only leaves the lobby. So the lobby is asked whenever the
//!   socket has been quiet for [`PlayOptions::room_check`], before the seat
//!   dials again, and at every poll of a mind that is down
//!   ([`SeatLink::room_closed`]). A seat that finds its room gone ends its
//!   game with no result ([`SeatCore::closed`]).
//!
//! An answer the core gives while there is no socket is dropped: the table
//! asks the question again on the next socket, at the same `seq`, and the
//! core sends the same answer then.
//!
//! # A fallback profile
//!
//! A bridge may hold a chain of other minds in reserve ([`Standby`], in
//! order): other profiles, the first of which takes the seat over when its
//! mind cannot answer: the provider says it is rate limited, busy or out of
//! its quota ([`MindError::Limited`]), or the mind failed otherwise
//! ([`MindError::Unavailable`]: a 5xx, the network, a task or a CLI's
//! process that died). The owner's rule is that the house plays only in an
//! emergency (13.10.2026), and a provider's `502` mid-game is none; a
//! quota spent mid-game is none either (owner, 31.10.2026: the same model
//! elsewhere first, then the next most suitable). The question the mind
//! could not answer is asked of the fallback at once, within what is left
//! of that question's budget, so the core never hears the failure: the
//! house does not answer and it does not count towards
//! [`crate::BridgeConfig::down_after`]. The fallback is said to the table
//! before its first answer (the record writes the change where it
//! happened), and plays the rest of the game, until it cannot answer
//! either and the next one in the chain takes over in the same way; the
//! bridge never switches back to a mind that played before
//! (`docs/llm-seat.md` §"A fallback profile"). A fallback that cannot take
//! over (its own spend book refuses, say) is passed over, the transcript
//! saying why, and the next is tried; past the end of the chain the failure
//! is the mind's as without one, and the house answers. A mind that
//! declines a question ([`MindError::Declined`]) is not down: the house
//! answers that one, and the chain stands by.
//!
//! # A game's reports
//!
//! What a language model reported of the rules or the engine
//! ([`crate::reports`]) is gathered from every mind that played the game,
//! as each answer comes and before a mind leaves it, and sent when the
//! game is over for the seat (an end, a room that closed, a seat that
//! left): one bug report a report to the gateway under the seat's own
//! session, or all in one on the seat token of a chair taken on its host's
//! ticket, whose route takes one a game ([`PlayOptions::send_reports`],
//! [`SeatLink::reporter`]), each with the seat's own game log as the
//! core's memory holds it ([`crate::reports::game_log`]). Whatever becomes
//! of them is a line of the transcript (`llm_reports`, its `route` saying
//! which) and never touches the game.

use crate::link::{SeatLink, Socket};
use crate::mind::{Answer, Mind, MindError, Request};
use crate::seat::{By, SeatCore, Stats, Step};
use crate::transcript::Transcript;
use baylee_client_core::reconnect::Window;
use baylee_engine::win::GameResult;
use baylee_protocol::v1::{Envelope, SeatMind};
use futures_util::{SinkExt as _, StreamExt as _};
use prost::Message as _;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;

/// How the bridge paces itself.
#[derive(Clone, Debug)]
pub struct PlayOptions {
    /// The least time between a woken question and its answer.
    pub min_think: Duration,
    /// How often a mind that is down is asked whether it is ready.
    pub mind_poll: Duration,
    /// How long the socket may be quiet before the lobby is asked whether
    /// the room is still open.
    pub room_check: Duration,
    /// Whether the game's reports go to the gateway as its bug report when
    /// it ends ([`crate::reports`]); they are written to the mind's
    /// reports file either way (`join --no-send-reports` turns it off).
    pub send_reports: bool,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            min_think: Duration::from_millis(1500),
            mind_poll: Duration::from_secs(5),
            room_check: Duration::from_secs(20),
            send_reports: true,
        }
    }
}

/// How long each of the game's bug reports may take to be sent.
const REPORT_WAIT: Duration = Duration::from_secs(15);

/// How long all of a game's bug reports may take to be sent, together: a
/// slow gateway holds a seat after its game for at most this long, not
/// [`crate::reports::MOST`] times [`REPORT_WAIT`]. Those not sent by then
/// stay in the reports file, as any not sent do.
const REPORTS_WAIT: Duration = Duration::from_secs(60);

/// How a seat's game went.
#[derive(Debug)]
pub struct Played {
    /// The counts.
    pub stats: Stats,
    /// How the game ended; `None` when the room closed without showing
    /// the seat ([`SeatCore::closed`]).
    pub result: Option<GameResult>,
    /// Sockets opened, the first included.
    pub dials: u32,
    /// How each fallback profile that took over played, in the order they
    /// took over; empty where none did.
    pub fallbacks: Vec<FellBack>,
    /// What became of the game's reports, where its mind filed any.
    pub reports: Option<Filing>,
}

/// What became of a game's reports ([`crate::reports`]): how many, how
/// many bug reports they made, and which of those the gateway took.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Filing {
    /// The reports filed: each dedupe key once, at most
    /// [`crate::reports::MOST`].
    pub count: usize,
    /// The bug reports they made: one a report under a session, one in
    /// all on a chair's route ([`Self::route`]).
    pub bug_reports: usize,
    /// Whether the gateway took every one of them.
    pub sent: bool,
    /// The service's id for the first it took.
    pub report_id: Option<String>,
    /// The service's ids for every one it took, in the order sent.
    pub report_ids: Vec<String>,
    /// Why one was not sent, when one was not: the first reason. A refusal
    /// or no answer stops the rest, which the reports file keeps.
    pub why: Option<String>,
    /// The gateway route they went to, or would have: `"reports"` under
    /// the seat's own session (`POST /reports`, one a report), `"chair"` on
    /// the seat token of a chair taken on its host's ticket
    /// (`POST …/chair/report`, which takes one a chair a game: all in one).
    pub route: &'static str,
}

/// The failure a fallback is asked to take over on: a limit, or the mind
/// unavailable otherwise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limit {
    /// The turn of the question the mind could not answer.
    pub turn: u32,
    /// Why, in the provider's words ([`MindError::Limited`],
    /// [`MindError::Unavailable`]).
    pub why: String,
    /// When the provider said the limit lifts, if it said; `None` where
    /// the mind failed otherwise.
    pub lifts_in: Option<Duration>,
    /// Whether the mind failed otherwise than by a limit
    /// ([`MindError::Unavailable`]): no limit, so nothing lifts, and a seat
    /// agent is not told the profile is rate limited.
    pub failed: bool,
    /// Who could not answer: `None` for the mind the chair sat down with,
    /// else the fallback ([`Standby::label`]) that had taken over from it.
    pub by: Option<String>,
}

impl Limit {
    /// Why the fallback takes over, as the table's operator reads it:
    /// `the primary is rate limited (it lifts in N s)`, `the primary
    /// failed: <why>`, or the same of `the fallback <profile>` that played
    /// before it ([`fallback_limited`]).
    #[must_use]
    pub fn said(&self) -> String {
        let who = self.by.as_ref().map_or_else(
            || "the primary".to_owned(),
            |by| format!("the fallback {by}"),
        );
        if self.failed {
            return format!("{who} failed: {}", self.why);
        }
        let lifts = self
            .lifts_in
            .map(|lifts| format!(" (it lifts in {} s)", lifts.as_secs()))
            .unwrap_or_default();
        match &self.by {
            None => format!("{LIMITED}{lifts}"),
            Some(by) => format!("{}{lifts}", fallback_limited(by)),
        }
    }

    /// The transcript's word for it: `limited` or `failed`.
    fn cause(&self) -> &'static str {
        if self.failed { "failed" } else { "limited" }
    }
}

/// What a bridge says when its model is rate limited and a fallback stood
/// by ([`Limit::said`]): a seat agent reads it to hold the profile back
/// from new games until the limit lifts (`baylee-seathost`,
/// `launch::LIMITED`). A failure that is no limit never says it.
pub const LIMITED: &str = "the primary is rate limited";

/// What a bridge says when a fallback that took over is rate limited in
/// its turn and another stood by behind it: `the fallback <profile> is rate
/// limited`, read by a seat agent as [`LIMITED`] is, of that profile
/// (`baylee-seathost`, `launch::heard_in`).
#[must_use]
pub fn fallback_limited(label: &str) -> String {
    format!("the fallback {label} is rate limited")
}

/// A fallback profile held in reserve: what takes the seat over when its
/// mind is rate limited or unavailable (the module's §"A fallback
/// profile"). A bridge holds a chain of them, in the order they take over.
pub struct Standby {
    /// What it is called in the summary: the profile's name.
    pub label: String,
    /// Takes over, once: reserves the fallback's game in its own spend book
    /// and builds its mind, as a [`Swap`] (its think time and its
    /// declaration with it). `Err` says why it cannot.
    pub arm: Arm,
}

/// [`Standby::arm`]: the fallback's mind, made when it takes over.
pub type Arm = Box<dyn FnOnce(&Limit) -> Result<Swap, String> + Send>;

impl std::fmt::Debug for Standby {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Standby")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

/// How a fallback profile played once it took over.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct FellBack {
    /// Its name ([`Standby::label`]).
    pub label: String,
    /// The turn it took over in.
    pub turn: u32,
    /// The answers it gave that went to the table: the mind's answers
    /// from then on, until the next fallback took over from it
    /// (`stats.answered.mind` counts them too).
    pub answered: u32,
}

/// A question the mind is working on.
struct Thinking {
    question: u64,
    deadline: Instant,
    task: JoinHandle<Result<Answer, MindError>>,
    /// The question as it was asked, kept while a fallback stands by, so
    /// that the fallback can be asked it.
    request: Option<Box<Request>>,
}

/// What woke the loop.
enum Woken {
    Frame(Option<Result<Message, tokio_tungstenite::tungstenite::Error>>),
    Joined(Result<Result<Answer, MindError>, tokio::task::JoinError>),
    Expired,
    Quiet,
}

/// What a batch of steps came to.
enum Next {
    Go,
    Lost,
    Down,
    Over(GameResult),
    Left(String),
}

/// A game being played, boxed: it holds the seat's whole memory of the
/// table, too large a future for a caller's stack.
pub type Playing<'a> = Pin<Box<dyn Future<Output = anyhow::Result<Played>> + Send + 'a>>;

/// Another mind for the seat, from its next decision on: what a debug
/// bridge is ordered during its game (`docs/llm-seat.md` §"Changing a
/// chair during the game").
pub struct Swap {
    /// The mind that answers from the next question the seat is asked.
    pub mind: Arc<dyn Mind>,
    /// The longest one answer may take from then on, as the new mind's
    /// profile says (`think_secs`); `None` keeps the bridge's.
    pub think: Option<Duration>,
    /// Called once the swap is made, at that question: the old mind has
    /// answered its last (its spend can be settled).
    pub taken: Option<Box<dyn FnOnce() + Send>>,
    /// What the new mind says it is (`docs/protocol.md` §"Who answers a
    /// seat, as it says"): told to the table before the new mind's first
    /// answer, so the game's record writes the change where it happened.
    pub declared: Option<SeatMind>,
}

impl std::fmt::Debug for Swap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Swap")
            .field("mind", &self.mind.disclosure())
            .field("think", &self.think)
            .field("taken", &self.taken.is_some())
            .field(
                "declared",
                &self.declared.as_ref().map(|m| m.model.as_str()),
            )
            .finish()
    }
}

/// Where [`Swap`]s come from.
pub type Swaps = tokio::sync::mpsc::UnboundedReceiver<Swap>;

/// Plays the seat's game to its end.
///
/// # Errors
/// When the seat leaves the table (a chair not called as the mind must be,
/// a table too fast for it, a version it cannot read), or the table cannot
/// be reached again.
pub fn play<'a>(
    link: &'a mut SeatLink,
    core: SeatCore,
    mind: Arc<dyn Mind>,
    transcript: &'a mut Transcript,
    options: &'a PlayOptions,
) -> Playing<'a> {
    play_swapping(link, core, mind, transcript, options, None, Vec::new())
}

/// [`play`], taking another mind whenever `swaps` hands one over: never in
/// the middle of a question, which the mind that was asked it answers, but
/// at the next one the seat asks a mind. With `fallbacks`, those profiles
/// take the seat over in turn when the mind playing cannot answer (the
/// module's §"A fallback profile").
///
/// # Errors
/// As [`play`].
pub fn play_swapping<'a>(
    link: &'a mut SeatLink,
    core: SeatCore,
    mind: Arc<dyn Mind>,
    transcript: &'a mut Transcript,
    options: &'a PlayOptions,
    swaps: Option<Swaps>,
    fallbacks: Vec<Standby>,
) -> Playing<'a> {
    Box::pin(run(link, core, mind, transcript, options, swaps, fallbacks))
}

async fn run(
    link: &mut SeatLink,
    mut core: SeatCore,
    mind: Arc<dyn Mind>,
    transcript: &mut Transcript,
    options: &PlayOptions,
    swaps: Option<Swaps>,
    fallbacks: Vec<Standby>,
) -> anyhow::Result<Played> {
    let mut bridge = Bridge::new(mind, options.clone(), swaps, fallbacks);
    link.hold_for(Window::of(core.memory().statics()));
    let mut socket = link.connect().await?;
    if socket.is_none() {
        return Ok(bridge.closed(&mut core, transcript, link).await);
    }
    // The host may change a local or hosted model's deck until Start.
    // Read it after connecting, when it is final, before any model context.
    if let Some(deck) = link.selected_deck().await? {
        core.set_deck(deck);
    }
    loop {
        let Some(ws) = socket.as_mut() else {
            socket = if link.room_closed().await {
                None
            } else {
                link.hold_for(Window::of(core.memory().statics()));
                link.connect().await?
            };
            if socket.is_none() {
                return Ok(bridge.closed(&mut core, transcript, link).await);
            }
            bridge.settle(&mut core);
            let steps = core.resumed();
            record(&mut core, transcript);
            if let Next::Lost = bridge.carry_out(steps, socket.as_mut()).await {
                socket = None;
            }
            continue;
        };
        let woken = bridge.wait(ws).await;
        let steps = match woken {
            Woken::Frame(Some(Ok(Message::Binary(bytes)))) => {
                bridge.settle(&mut core);
                core.hear(&bytes)
            }
            Woken::Frame(Some(Ok(Message::Close(_)) | Err(_)) | None) => {
                tracing::info!("the seat socket closed");
                socket = None;
                continue;
            }
            Woken::Frame(Some(Ok(_))) => continue,
            Woken::Joined(joined) => {
                let steps = bridge.answered(&mut core, joined);
                for note in bridge.notes.drain(..) {
                    transcript.write_value(&note);
                }
                match steps {
                    Some(steps) => steps,
                    None => continue,
                }
            }
            Woken::Expired => match bridge.expired(&mut core) {
                Some(steps) => steps,
                None => continue,
            },
            Woken::Quiet => {
                if !link.room_closed().await {
                    continue;
                }
                if let Some(mut ws) = socket.take() {
                    let _ = ws.close(None).await;
                }
                return Ok(bridge.closed(&mut core, transcript, link).await);
            }
        };
        record(&mut core, transcript);
        match bridge.carry_out(steps, socket.as_mut()).await {
            Next::Go => {}
            Next::Lost => socket = None,
            Next::Down => {
                if let Some(mut ws) = socket.take() {
                    let _ = ws.close(None).await;
                }
                if !bridge.wait_for_mind(link).await {
                    return Ok(bridge.closed(&mut core, transcript, link).await);
                }
            }
            Next::Over(result) => {
                if let Some(mut ws) = socket.take() {
                    let _ = ws.close(None).await;
                }
                return Ok(bridge.played(&core, Some(result), transcript, link).await);
            }
            Next::Left(reason) => {
                if let Some(mut ws) = socket.take() {
                    let _ = ws.close(None).await;
                }
                bridge.file_reports(link, transcript, core.memory()).await;
                transcript.flush();
                anyhow::bail!("the seat left the table: {reason}");
            }
        }
    }
}

/// Sends `bodies` in order with `send`: each sealed first (one holding a
/// secret is not sent, and the next goes), each given at most
/// [`REPORT_WAIT`] and all of them `all`, from the first. A refusal, no
/// answer in time or `all` spent stops the rest. The ids the gateway gave,
/// in order, and the first reason one was not sent.
async fn send_each<F, Fut>(
    bodies: Vec<serde_json::Value>,
    secrets: &[baylee_client_core::bugreport::Secret<'_>],
    all: Duration,
    mut send: F,
) -> (Vec<String>, Option<String>)
where
    F: FnMut(serde_json::Value) -> Fut,
    Fut: Future<Output = anyhow::Result<String>>,
{
    let deadline = Instant::now() + all;
    let mut ids = Vec::new();
    let mut why = None;
    for body in bodies {
        if let Err(leaked) = baylee_client_core::bugreport::seal(&body, secrets) {
            why.get_or_insert_with(|| format!("not sent: {leaked}"));
            continue;
        }
        let now = Instant::now();
        if now >= deadline {
            why.get_or_insert_with(|| spent(all));
            break;
        }
        let failed =
            match tokio::time::timeout_at(deadline.min(now + REPORT_WAIT), send(body)).await {
                Ok(Ok(id)) => {
                    ids.push(id);
                    continue;
                }
                Ok(Err(e)) => format!("not sent: {e:#}"),
                Err(_) if Instant::now() >= deadline => spent(all),
                Err(_) => "not sent: the gateway did not answer in time".to_string(),
            };
        why.get_or_insert(failed);
        break;
    }
    (ids, why)
}

/// Why the rest of a game's reports were not sent once `all` was spent.
fn spent(all: Duration) -> String {
    format!(
        "not sent: the game's reports took longer than {} s in all",
        all.as_secs()
    )
}

/// Writes what the core noted.
fn record(core: &mut SeatCore, transcript: &mut Transcript) {
    for note in core.take_notes() {
        transcript.write(&note);
    }
}

struct Bridge {
    mind: Arc<dyn Mind>,
    game_seat: Option<(String, u8)>,
    options: PlayOptions,
    thinking: Option<Thinking>,
    /// The question last handed to the mind, and when it was first asked.
    asked: Option<(u64, Instant)>,
    /// Other minds, handed over during the game.
    swaps: Option<Swaps>,
    /// The think time of a mind taken since the core was last told it.
    rethink: Option<Duration>,
    /// What a mind taken since the core was last told it says it is.
    redeclare: Option<SeatMind>,
    /// The declaration's steps, sent ahead of the next steps carried out.
    declaring: Vec<Step>,
    /// The fallback profiles that stand by, in the order they take over:
    /// each leaves the chain as it takes over or cannot, and a mind handed
    /// over lets them all go (it plays another profile).
    fallbacks: VecDeque<Standby>,
    /// How each fallback that took over played, in that order: the last
    /// is the one that plays now where [`Self::on_fallback`].
    fell_back: Vec<FellBack>,
    /// Whether a fallback plays now: its answers are counted.
    on_fallback: bool,
    /// Lines for the transcript, written by the loop.
    notes: Vec<serde_json::Value>,
    /// The reports the game's minds filed, gathered as they answer and
    /// before a mind leaves the game ([`Mind::reports`]).
    reports: Vec<crate::reports::Filed>,
}

impl Bridge {
    fn new(
        mind: Arc<dyn Mind>,
        options: PlayOptions,
        swaps: Option<Swaps>,
        fallbacks: Vec<Standby>,
    ) -> Self {
        Self {
            mind,
            game_seat: None,
            options,
            thinking: None,
            asked: None,
            swaps,
            rethink: None,
            redeclare: None,
            declaring: Vec::new(),
            fallbacks: fallbacks.into(),
            fell_back: Vec::new(),
            on_fallback: false,
            notes: Vec::new(),
            reports: Vec::new(),
        }
    }

    /// Gathers the reports the mind filed since it was last asked.
    fn gather(&mut self) {
        if let Some((game, seat)) = &self.game_seat {
            let filed = self.mind.reports(game, *seat);
            self.reports.extend(filed);
        }
    }

    /// The mind plays no more of this game: its reports are gathered
    /// first.
    fn end_mind(&mut self) {
        self.gather();
        if let Some((game, seat)) = &self.game_seat {
            self.mind.end_game(game, *seat);
        }
    }

    /// Sends the game's reports to the gateway, where the bridge may
    /// ([`PlayOptions::send_reports`]), with the seat's own game log out of
    /// `memory` ([`crate::reports::game_log`]): one bug report a report
    /// under the seat's own session ([`crate::reports::submissions`]), or,
    /// for a chair taken on its host's ticket, all of them in one on its
    /// seat token ([`crate::reports::bundle`]), since that route takes one
    /// a chair a game and the gateway files it under the host's pseudonym
    /// ([`SeatLink::reporter`]). Says in the transcript what became of them
    /// and by which route. Never fails the game: a report not sent stays in
    /// the reports file.
    ///
    /// Like a person's report, each is sealed first (`bugreport::seal`):
    /// one that holds the session or the seat token anywhere is not sent,
    /// and the others still go. A refusal or no answer within
    /// [`REPORT_WAIT`] stops the rest, which would meet the same (the
    /// account's budget an hour spent, a gateway without a feedback
    /// service), and so does [`REPORTS_WAIT`] for all of them
    /// ([`send_each`]).
    async fn file_reports(
        &mut self,
        link: &SeatLink,
        transcript: &mut Transcript,
        memory: &crate::TableMemory,
    ) -> Option<Filing> {
        use crate::link::Reporter;
        use baylee_client_core::bugreport::{Keyring, Secret};
        self.gather();
        if self.reports.is_empty() {
            return None;
        }
        let log = crate::reports::game_log(memory);
        let reporter = link.reporter();
        let (route, session, bodies) = match reporter {
            Reporter::Session(_, session) => (
                "reports",
                Some(session.bearer()),
                crate::reports::submissions(&self.reports, log.as_ref()),
            ),
            Reporter::Chair(..) => (
                "chair",
                None,
                crate::reports::bundle(&self.reports, log.as_ref())
                    .into_iter()
                    .collect(),
            ),
        };
        if bodies.is_empty() {
            return None;
        }
        let count = crate::reports::count(&self.reports);
        let bug_reports = bodies.len();
        let mut secrets = vec![Secret {
            label: Keyring::SEAT,
            value: &link.chair().seat_token,
        }];
        secrets.extend(session.map(|value| Secret {
            label: Keyring::SESSION,
            value,
        }));
        let (report_ids, why) = if self.options.send_reports {
            let send = |body| async move {
                match reporter {
                    Reporter::Session(lobby, session) => lobby.report(session, body).await,
                    Reporter::Chair(lobby, chair) => lobby.chair_report(chair, body).await,
                }
            };
            send_each(bodies, &secrets, REPORTS_WAIT, send).await
        } else {
            (
                Vec::new(),
                Some("not sent: the bridge was told not to".to_string()),
            )
        };
        let filing = Filing {
            count,
            bug_reports,
            sent: report_ids.len() == bug_reports,
            report_id: report_ids.first().cloned(),
            report_ids,
            why,
            route,
        };
        if let Some(why) = &filing.why {
            tracing::warn!(
                reports = count,
                taken = filing.report_ids.len(),
                route,
                "the game's reports were {why}"
            );
        }
        transcript.write_value(&serde_json::json!({ "llm_reports": filing }));
        Some(filing)
    }

    /// Takes every mind handed over since the last question, the latest
    /// last: the mind the next question is asked of. A mind handed over
    /// plays another profile, so the fallbacks held for the old one are let
    /// go.
    fn take_swaps(&mut self) {
        while let Some(swap) = self.swaps.as_mut().and_then(|swaps| swaps.try_recv().ok()) {
            self.end_mind();
            self.fallbacks.clear();
            self.on_fallback = false;
            self.mind = swap.mind;
            self.rethink = swap.think.or(self.rethink);
            if let Some(declared) = swap.declared {
                self.redeclare = Some(declared);
            }
            if let Some(taken) = swap.taken {
                taken();
            }
        }
    }

    /// Before the core is told anything: takes the minds handed over where
    /// no question is being thought about (the old mind has answered its
    /// last), and tells the core the think time of the mind now playing, so
    /// the questions it asks are bounded by it.
    fn settle(&mut self, core: &mut SeatCore) {
        if self.thinking.is_none() {
            self.take_swaps();
        }
        if let Some(think) = self.rethink.take() {
            core.set_think(think);
        }
        if let Some(declared) = self.redeclare.take() {
            self.declaring.extend(core.declare(declared));
        }
    }

    /// The mind answered (or its task ended): the core's steps, `None`
    /// where no question was being thought about. A limit or a failure
    /// where a fallback stands by is not the core's to hear: the next
    /// fallback is asked the same question ([`Self::take_over`]), and there
    /// is nothing to send yet.
    fn answered(
        &mut self,
        core: &mut SeatCore,
        joined: Result<Result<Answer, MindError>, tokio::task::JoinError>,
    ) -> Option<Vec<Step>> {
        let Thinking {
            question,
            deadline,
            request,
            ..
        } = self.thinking.take()?;
        self.gather();
        let result = joined
            .unwrap_or_else(|e| Err(MindError::Unavailable(format!("the mind stopped: {e}"))));
        let result = match (result, request) {
            (
                Err(error @ (MindError::Limited { .. } | MindError::Unavailable(_))),
                Some(request),
            ) if !self.fallbacks.is_empty() => {
                match self.take_over(*request, question, deadline, error) {
                    Ok(()) => {
                        // Told to the core now, so the declaration goes out
                        // ahead of the fallback's first answer.
                        self.settle(core);
                        return Some(Vec::new());
                    }
                    Err(error) => Err(error),
                }
            }
            (result, _) => result,
        };
        self.settle(core);
        Some(core.answered(question, result))
    }

    /// The next fallback of the chain takes the seat over on a limit or a
    /// failure of the mind that plays now: built and reserved now
    /// ([`Standby::arm`]), it is asked `request` within what is left before
    /// `deadline`, and plays on until it cannot answer either. A fallback
    /// that cannot take over is passed over, the transcript saying why, and
    /// the one after it is tried. `Err` is `error`, its sentence followed by
    /// why each fallback tried could not take over: the chain was used up.
    /// Every fallback tried stands by no longer, whatever came of it.
    fn take_over(
        &mut self,
        mut request: Request,
        question: u64,
        deadline: Instant,
        error: MindError,
    ) -> Result<(), MindError> {
        let (why, lifts_in, failed) = match error {
            MindError::Limited { why, lifts_in } => (why, lifts_in, false),
            MindError::Unavailable(why) => (why, None, true),
            declined @ MindError::Declined(_) => return Err(declined),
        };
        // The same kind of failure, with `why` in its place.
        let error = |why: String| {
            if failed {
                MindError::Unavailable(why)
            } else {
                MindError::Limited { why, lifts_in }
            }
        };
        let limit = Limit {
            turn: request.view.turn,
            why,
            lifts_in,
            failed,
            by: self
                .fell_back
                .last()
                .filter(|_| self.on_fallback)
                .map(|fell| fell.label.clone()),
        };
        // Each fallback tried that could not take over, said after `why`.
        let mut refusals = vec![limit.why.clone()];
        let (label, swap) = loop {
            let Some(standby) = self.fallbacks.pop_front() else {
                return Err(error(refusals.join("; ")));
            };
            match (standby.arm)(&limit) {
                Ok(swap) => break (standby.label, swap),
                Err(refused) => {
                    tracing::warn!(
                        fallback = standby.label,
                        refused,
                        "the fallback could not take over"
                    );
                    refusals.push(format!(
                        "the fallback {} could not take over: {refused}",
                        standby.label
                    ));
                    self.notes.push(serde_json::json!({
                        "fallback_refused": { "profile": standby.label, "why": refused },
                    }));
                }
            }
        };
        tracing::info!(
            fallback = label,
            turn = limit.turn,
            cause = limit.cause(),
            "the fallback takes over"
        );
        self.notes.push(serde_json::json!({
            "fallback": {
                "profile": label,
                "turn": limit.turn,
                "cause": limit.cause(),
                "why": limit.why,
            },
        }));
        // The mind that could not answer plays no more of this game.
        self.end_mind();
        self.mind = swap.mind;
        self.rethink = swap.think.or(self.rethink);
        if let Some(declared) = swap.declared {
            self.redeclare = Some(declared);
        }
        if let Some(taken) = swap.taken {
            taken();
        }
        self.fell_back.push(FellBack {
            label,
            turn: limit.turn,
            answered: 0,
        });
        self.on_fallback = true;
        // Asked as a first asking: an answer refused before was the other
        // mind's.
        request.retry = None;
        request.budget = deadline.saturating_duration_since(Instant::now());
        // Kept while another stands by behind it, which is asked the same
        // question if this one cannot answer it either.
        let kept = (!self.fallbacks.is_empty()).then(|| Box::new(request.clone()));
        let mind = Arc::clone(&self.mind);
        let task = tokio::spawn(async move { mind.decide(request).await });
        self.thinking = Some(Thinking {
            question,
            deadline,
            task,
            request: kept,
        });
        Ok(())
    }

    /// The mind ran out of time: the core's steps, `None` where no question
    /// was being thought about.
    fn expired(&mut self, core: &mut SeatCore) -> Option<Vec<Step>> {
        let thinking = self.thinking.take()?;
        thinking.task.abort();
        self.settle(core);
        Some(core.expired(thinking.question))
    }

    /// Waits for a frame, the mind's answer, the mind's deadline, or a
    /// quiet spell long enough to ask the lobby about the room.
    async fn wait(&mut self, ws: &mut Socket) -> Woken {
        let deadline = self.thinking.as_ref().map(|t| t.deadline);
        let task = self.thinking.as_mut().map(|t| &mut t.task);
        tokio::select! {
            frame = ws.next() => Woken::Frame(frame),
            joined = async {
                match task {
                    Some(task) => task.await,
                    None => std::future::pending().await,
                }
            } => Woken::Joined(joined),
            () = async {
                match deadline {
                    Some(deadline) => tokio::time::sleep_until(deadline).await,
                    None => std::future::pending().await,
                }
            } => Woken::Expired,
            () = tokio::time::sleep(self.options.room_check) => Woken::Quiet,
        }
    }

    /// The room closed without a result: whatever the mind is thinking
    /// about is dropped, and the seat's game is over.
    async fn closed(
        &mut self,
        core: &mut SeatCore,
        transcript: &mut Transcript,
        link: &SeatLink,
    ) -> Played {
        tracing::info!("the room closed without a result for this seat");
        let steps = core.closed();
        self.carry_out(steps, None).await;
        record(core, transcript);
        self.played(core, None, transcript, link).await
    }

    /// How the game went, once it is over for the seat: its reports filed
    /// first, the transcript flushed.
    async fn played(
        &mut self,
        core: &SeatCore,
        result: Option<GameResult>,
        transcript: &mut Transcript,
        link: &SeatLink,
    ) -> Played {
        let reports = self.file_reports(link, transcript, core.memory()).await;
        transcript.flush();
        Played {
            stats: core.stats().clone(),
            result,
            dials: link.dials(),
            fallbacks: self.fell_back.clone(),
            reports,
        }
    }

    /// Carries out the core's steps, in order.
    async fn carry_out(&mut self, steps: Vec<Step>, mut ws: Option<&mut Socket>) -> Next {
        let mut next = Next::Go;
        let mut dirty = false;
        let declaring = std::mem::take(&mut self.declaring);
        for step in declaring.into_iter().chain(steps) {
            match step {
                Step::Send(envelope) => {
                    if feed(ws.as_deref_mut(), &envelope).await {
                        dirty = true;
                    } else {
                        next = Next::Lost;
                    }
                }
                Step::Answer {
                    question,
                    by,
                    paced,
                    envelope,
                    ..
                } => {
                    if by == By::Mind
                        && self.on_fallback
                        && let Some(fell_back) = self.fell_back.last_mut()
                    {
                        fell_back.answered += 1;
                    }
                    if paced {
                        if dirty {
                            dirty = false;
                            if !flush(ws.as_deref_mut()).await {
                                next = Next::Lost;
                            }
                        }
                        let since = self
                            .asked
                            .filter(|(asked, _)| *asked == question)
                            .map_or_else(Instant::now, |(_, at)| at);
                        tokio::time::sleep_until(since + self.options.min_think).await;
                    }
                    if feed(ws.as_deref_mut(), &envelope).await {
                        dirty = true;
                    } else {
                        next = Next::Lost;
                    }
                }
                Step::Ask(mut request) => {
                    let now = Instant::now();
                    if self
                        .asked
                        .is_none_or(|(asked, _)| asked != request.question)
                    {
                        self.asked = Some((request.question, now));
                    }
                    self.take_swaps();
                    self.game_seat =
                        Some((request.context.game_id.clone(), request.context.seat.get()));
                    // A mind taken only now was asked under the old think
                    // time: held to its own where that is shorter (the
                    // core is told before the next question).
                    if let Some(think) = self.rethink {
                        request.budget = request.budget.min(think);
                    }
                    let question = request.question;
                    let deadline = now + request.budget;
                    let kept = (!self.fallbacks.is_empty()).then(|| request.clone());
                    let mind = Arc::clone(&self.mind);
                    let task = tokio::spawn(async move { mind.decide(*request).await });
                    if let Some(old) = self.thinking.replace(Thinking {
                        question,
                        deadline,
                        task,
                        request: kept,
                    }) {
                        old.task.abort();
                    }
                }
                Step::Cancel(question) => {
                    if let Some(thinking) = self.thinking.take_if(|t| t.question == question) {
                        thinking.task.abort();
                    }
                }
                Step::MindDown => next = Next::Down,
                Step::Leave(reason) => {
                    if dirty {
                        let _ = flush(ws).await;
                    }
                    return Next::Left(reason);
                }
                Step::Over(result) => {
                    if dirty {
                        let _ = flush(ws).await;
                    }
                    return Next::Over(result);
                }
            }
        }
        if dirty && !flush(ws).await {
            next = Next::Lost;
        }
        next
    }

    /// Asks a mind that is down whether it is ready, until it is (`true`)
    /// or the room has closed meanwhile (`false`): the house plays on for a
    /// seat whose socket is gone, and may finish the game before the mind
    /// comes back, if it ever does.
    async fn wait_for_mind(&mut self, link: &mut SeatLink) -> bool {
        loop {
            tokio::time::sleep(self.options.mind_poll).await;
            // A mind handed over meanwhile is the one asked.
            self.take_swaps();
            if self.mind.ready().await {
                tracing::info!("the mind is ready again");
                return true;
            }
            if link.room_closed().await {
                return false;
            }
        }
    }
}

/// Feeds one envelope into the socket buffer without flushing; `false` when
/// there is no socket or it failed.
async fn feed(ws: Option<&mut Socket>, envelope: &Envelope) -> bool {
    let Some(ws) = ws else {
        return false;
    };
    ws.feed(Message::Binary(envelope.encode_to_vec().into()))
        .await
        .is_ok()
}

/// Flushes buffered envelopes on `ws`; `false` when there is no socket or it
/// failed.
async fn flush(ws: Option<&mut Socket>) -> bool {
    let Some(ws) = ws else {
        return false;
    };
    ws.flush().await.is_ok()
}

impl Drop for Bridge {
    fn drop(&mut self) {
        if let Some(thinking) = self.thinking.take() {
            thinking.task.abort();
        }
        if let Some((game, seat)) = &self.game_seat {
            self.mind.end_game(game, *seat);
        }
    }
}

#[cfg(test)]
mod report_tests;

#[cfg(test)]
mod tests;
