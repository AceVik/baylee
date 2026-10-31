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
//! A bridge may hold a second mind in reserve ([`Standby`]): another
//! profile, which takes the seat over when its mind cannot answer: the
//! provider says it is rate limited or busy ([`MindError::Limited`]), or
//! the mind failed otherwise ([`MindError::Unavailable`]: a 5xx, the
//! network, a task that died). The owner's rule is that the house plays
//! only in an emergency (13.10.2026), and a provider's `502` mid-game is
//! none. The question the mind could not answer is asked of the fallback
//! at once, within what is left of that question's budget, so the core
//! never hears the failure: the house does not answer and it does not
//! count towards [`crate::BridgeConfig::down_after`]. The fallback is said
//! to the table before its first answer (the record writes the change
//! where it happened), and plays the rest of the game; the bridge never
//! switches back (`docs/llm-seat.md` §"A fallback profile"). Where it
//! cannot take over (its own spend book refuses, say), the failure is the
//! mind's as before, the house answers, and the transcript says why. A
//! mind that declines a question ([`MindError::Declined`]) is not down:
//! the house answers that one, and the fallback stands by.
//!
//! # A game's reports
//!
//! What a language model reported of the rules or the engine
//! ([`crate::reports`]) is gathered from every mind that played the game,
//! as each answer comes and before a mind leaves it, and sent when the
//! game is over for the seat (an end, a room that closed, a seat that
//! left) as one bug report to the gateway under the seat's own session
//! ([`PlayOptions::send_reports`]). Whatever becomes of it is a line of the
//! transcript (`llm_reports`) and never touches the game.

use crate::link::{SeatLink, Socket};
use crate::mind::{Answer, Mind, MindError, Request};
use crate::seat::{By, SeatCore, Stats, Step};
use crate::transcript::Transcript;
use baylee_client_core::reconnect::Window;
use baylee_engine::win::GameResult;
use baylee_protocol::v1::{Envelope, SeatMind};
use futures_util::{SinkExt as _, StreamExt as _};
use prost::Message as _;
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
    /// it ends ([`crate::reports`]); they are written to the transcripts'
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

/// How long the game's bug report may take to be sent.
const REPORT_WAIT: Duration = Duration::from_secs(15);

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
    /// How the fallback profile played, where it took over.
    pub fallback: Option<FellBack>,
    /// What became of the game's reports, where its mind filed any.
    pub reports: Option<Filing>,
}

/// What became of a game's reports ([`crate::reports`]): how many, and
/// whether they reached the gateway as its bug report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Filing {
    /// The reports the mind filed.
    pub count: usize,
    /// Whether the gateway took the bug report.
    pub sent: bool,
    /// The service's id for it, when it took it.
    pub report_id: Option<String>,
    /// Why it was not sent, when it was not.
    pub why: Option<String>,
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
}

impl Limit {
    /// Why the fallback takes over, as the table's operator reads it:
    /// `the primary is rate limited (it lifts in N s)`, or `the primary
    /// failed: <why>`.
    #[must_use]
    pub fn said(&self) -> String {
        if self.failed {
            return format!("the primary failed: {}", self.why);
        }
        let lifts = self
            .lifts_in
            .map(|lifts| format!(" (it lifts in {} s)", lifts.as_secs()))
            .unwrap_or_default();
        format!("{LIMITED}{lifts}")
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

/// A fallback profile held in reserve: what takes the seat over when its
/// mind is rate limited or unavailable (the module's §"A fallback
/// profile").
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
    /// from then on (`stats.answered.mind` counts them too).
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
    play_swapping(link, core, mind, transcript, options, None, None)
}

/// [`play`], taking another mind whenever `swaps` hands one over: never in
/// the middle of a question, which the mind that was asked it answers, but
/// at the next one the seat asks a mind. With a `fallback`, that profile
/// takes the seat over when the mind is rate limited (the module's §"A
/// fallback profile").
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
    fallback: Option<Standby>,
) -> Playing<'a> {
    Box::pin(run(link, core, mind, transcript, options, swaps, fallback))
}

async fn run(
    link: &mut SeatLink,
    mut core: SeatCore,
    mind: Arc<dyn Mind>,
    transcript: &mut Transcript,
    options: &PlayOptions,
    swaps: Option<Swaps>,
    fallback: Option<Standby>,
) -> anyhow::Result<Played> {
    let mut bridge = Bridge::new(mind, options.clone(), swaps, fallback);
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
                bridge.file_reports(link, transcript).await;
                transcript.flush();
                anyhow::bail!("the seat left the table: {reason}");
            }
        }
    }
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
    /// A fallback profile, until it takes over (or a mind handed over
    /// makes it another profile's).
    fallback: Option<Standby>,
    /// How the fallback played, once it took over.
    fell_back: Option<FellBack>,
    /// Whether the fallback plays now: its answers are counted.
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
        fallback: Option<Standby>,
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
            fallback,
            fell_back: None,
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

    /// Sends the game's reports as one bug report to the gateway, under
    /// the seat's session, where the bridge may ([`PlayOptions::send_reports`])
    /// and has one; says in the transcript what became of them. Never
    /// fails the game: a report not sent stays in the reports file.
    async fn file_reports(
        &mut self,
        link: &SeatLink,
        transcript: &mut Transcript,
    ) -> Option<Filing> {
        self.gather();
        let body = crate::reports::submission(&self.reports)?;
        let count = body["client"]["reports"].as_array().map_or(0, Vec::len);
        let sent = match (self.options.send_reports, link.reporter()) {
            (false, _) => Err("not sent: the bridge was told not to".to_string()),
            (true, None) => Err("not sent: the chair has no session of its own".to_string()),
            (true, Some((lobby, session))) => {
                match tokio::time::timeout(REPORT_WAIT, lobby.report(session, body)).await {
                    Ok(Ok(id)) => Ok(id),
                    Ok(Err(e)) => Err(format!("not sent: {e:#}")),
                    Err(_) => Err("not sent: the gateway did not answer in time".to_string()),
                }
            }
        };
        let filing = match sent {
            Ok(id) => Filing {
                count,
                sent: true,
                report_id: Some(id),
                why: None,
            },
            Err(why) => {
                tracing::warn!(reports = count, "the game's reports were {why}");
                Filing {
                    count,
                    sent: false,
                    report_id: None,
                    why: Some(why),
                }
            }
        };
        transcript.write_value(&serde_json::json!({ "llm_reports": filing }));
        Some(filing)
    }

    /// Takes every mind handed over since the last question, the latest
    /// last: the mind the next question is asked of. A mind handed over
    /// plays another profile, so a fallback held for the old one is let go.
    fn take_swaps(&mut self) {
        while let Some(swap) = self.swaps.as_mut().and_then(|swaps| swaps.try_recv().ok()) {
            self.end_mind();
            self.fallback = None;
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
    /// where a fallback stands by is not the core's to hear: the fallback
    /// is asked the same question ([`Self::take_over`]), and there is
    /// nothing to send yet.
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
            ) if self.fallback.is_some() => {
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

    /// The fallback takes the seat over on a limit or a failure of the
    /// mind: built and reserved now ([`Standby::arm`]), it is asked
    /// `request` within what is left before `deadline`, and plays the rest
    /// of the game. `Err` is `error`, its sentence followed by why the
    /// fallback could not take over; it stands by no longer either way.
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
        let Some(standby) = self.fallback.take() else {
            return Err(error(why));
        };
        let limit = Limit {
            turn: request.view.turn,
            why,
            lifts_in,
            failed,
        };
        let swap = match (standby.arm)(&limit) {
            Ok(swap) => swap,
            Err(refused) => {
                tracing::warn!(
                    fallback = standby.label,
                    refused,
                    "the fallback could not take over"
                );
                self.notes.push(serde_json::json!({
                    "fallback_refused": { "profile": standby.label, "why": refused },
                }));
                return Err(error(format!(
                    "{}; the fallback {} could not take over: {refused}",
                    limit.why, standby.label
                )));
            }
        };
        tracing::info!(
            fallback = standby.label,
            turn = limit.turn,
            cause = limit.cause(),
            "the fallback takes over"
        );
        self.notes.push(serde_json::json!({
            "fallback": {
                "profile": standby.label,
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
        self.fell_back = Some(FellBack {
            label: standby.label,
            turn: limit.turn,
            answered: 0,
        });
        self.on_fallback = true;
        // Asked as a first asking: an answer refused before was the other
        // mind's.
        request.retry = None;
        request.budget = deadline.saturating_duration_since(Instant::now());
        let mind = Arc::clone(&self.mind);
        let task = tokio::spawn(async move { mind.decide(request).await });
        self.thinking = Some(Thinking {
            question,
            deadline,
            task,
            request: None,
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
        let reports = self.file_reports(link, transcript).await;
        transcript.flush();
        Played {
            stats: core.stats().clone(),
            result,
            dials: link.dials(),
            fallback: self.fell_back.clone(),
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
                        && let Some(fell_back) = self.fell_back.as_mut()
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
                    let kept = self.fallback.is_some().then(|| request.clone());
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
mod tests {
    use super::*;
    use crate::mind::{DeckList, Disclosure, Request, Thinking};
    use crate::seat::BridgeConfig;
    use crate::transcript::Event;
    use baylee_client_core::test_support::{ViewBuilder, statics};
    use baylee_core::ids::PlayerId;
    use baylee_core::mana::ManaColor;
    use baylee_engine::choice::{Pending, PlayerAction};
    use baylee_protocol::v1;
    use baylee_view::{PlayerView, SeatIdentity};
    use std::sync::{Mutex, PoisonError};

    const ME: PlayerId = PlayerId::new(0);

    fn frame(msg: v1::envelope::Msg) -> Vec<u8> {
        Envelope { msg: Some(msg) }.encode_to_vec()
    }

    /// A seat at a two-chair table, its own called as its mind must be.
    fn seated() -> SeatCore {
        let mut game = statics(0);
        game.seats = [(ME, "LLM-seat"), (PlayerId::new(1), "Alice")]
            .into_iter()
            .map(|(player, name)| SeatIdentity {
                player,
                display_name: name.into(),
                is_ai: false,
                away: false,
                team: None,
            })
            .collect();
        let mut core = SeatCore::new(
            BridgeConfig::default(),
            DeckList::default(),
            Disclosure::Llm,
        );
        core.hear(&frame(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
            game_id: game.game_id.clone(),
            view_version: game.view_version,
            static_json: serde_json::to_vec(&game).unwrap(),
        })));
        core
    }

    /// The steps of the seat being asked to choose a colour at `seq`: a
    /// question its standing orders never answer, so the mind is asked.
    fn asked(core: &mut SeatCore, seq: u64) -> Vec<Step> {
        asked_with(core, seq, |_| {})
    }

    /// [`asked`], in turn `turn`.
    fn asked_in_turn(core: &mut SeatCore, seq: u64, turn: u32) -> Vec<Step> {
        asked_with(core, seq, |view| view.turn = turn)
    }

    /// [`asked`], the view as `tweak` leaves it.
    fn asked_with(core: &mut SeatCore, seq: u64, tweak: impl FnOnce(&mut PlayerView)) -> Vec<Step> {
        let steps = put_with(core, seq, tweak);
        assert!(
            steps.iter().any(|s| matches!(s, Step::Ask(_))),
            "the mind is asked: {steps:?}"
        );
        steps
    }

    /// The question of [`asked_in_turn`], whoever answers it.
    fn put_in_turn(core: &mut SeatCore, seq: u64, turn: u32) -> Vec<Step> {
        put_with(core, seq, |view| view.turn = turn)
    }

    /// The question of [`asked_with`], whoever answers it.
    fn put_with(core: &mut SeatCore, seq: u64, tweak: impl FnOnce(&mut PlayerView)) -> Vec<Step> {
        let mut view = ViewBuilder::new(2).build();
        view.seq = seq;
        tweak(&mut view);
        core.hear(&frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: "test-game".into(),
            seq,
            view_json: serde_json::to_vec(&view).unwrap(),
            log_json: Vec::new(),
        })));
        let pending = Pending::ChooseColor {
            player: ME,
            options: vec![ManaColor::Red, ManaColor::Green],
        };
        core.hear(&frame(v1::envelope::Msg::ChoiceRequest(
            v1::ChoiceRequest {
                game_id: "test-game".into(),
                seq,
                pending_json: serde_json::to_vec(&pending).unwrap(),
            },
        )))
    }

    /// A mind that answers `colour`, and writes down that it was asked.
    struct Named {
        colour: ManaColor,
        asked: Arc<Mutex<Vec<ManaColor>>>,
    }

    impl Mind for Named {
        fn decide(&self, _request: Request) -> Thinking<'_> {
            self.asked.lock().unwrap().push(self.colour);
            let colour = self.colour;
            Box::pin(async move { Ok(Answer::new(PlayerAction::ChooseColor(colour))) })
        }

        fn disclosure(&self) -> Disclosure {
            Disclosure::Llm
        }
    }

    /// A mind handed over during the game answers from the next question
    /// on, never the question already being thought about; and once it
    /// does, the swap says so (where the old mind's spend is settled).
    #[tokio::test]
    async fn a_swapped_mind_answers_from_the_next_question_on() {
        let asked_of = Arc::new(Mutex::new(Vec::new()));
        let mind = |colour| -> Arc<dyn Mind> {
            Arc::new(Named {
                colour,
                asked: Arc::clone(&asked_of),
            })
        };
        let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
        let mut bridge = Bridge::new(
            mind(ManaColor::Red),
            PlayOptions::default(),
            Some(swaps),
            None,
        );
        let mut core = seated();
        let first = asked(&mut core, 2);
        // Handed over before the question reaches the mind: it is the new
        // mind's, at the moment it is asked.
        let taken = Arc::new(Mutex::new(0));
        let count = Arc::clone(&taken);
        send.send(Swap {
            mind: mind(ManaColor::Green),
            think: None,
            taken: Some(Box::new(move || *count.lock().unwrap() += 1)),
            declared: None,
        })
        .unwrap_or_else(|_| panic!("the bridge listens"));
        bridge.carry_out(first, None).await;
        assert_eq!(*taken.lock().unwrap(), 1);
        let thinking = bridge.thinking.take().unwrap();
        core.answered(thinking.question, thinking.task.await.unwrap());
        assert_eq!(*asked_of.lock().unwrap(), [ManaColor::Green]);
        // One handed over while a question is being thought about waits
        // for the next question.
        let second = asked(&mut core, 3);
        bridge.carry_out(second, None).await;
        send.send(Swap {
            mind: mind(ManaColor::Red),
            think: None,
            taken: None,
            declared: None,
        })
        .unwrap_or_else(|_| panic!("the bridge listens"));
        let thinking = bridge.thinking.take().unwrap();
        core.answered(thinking.question, thinking.task.await.unwrap());
        assert_eq!(
            *asked_of.lock().unwrap(),
            [ManaColor::Green, ManaColor::Green]
        );
        let third = asked(&mut core, 4);
        bridge.carry_out(third, None).await;
        let thinking = bridge.thinking.take().unwrap();
        core.answered(thinking.question, thinking.task.await.unwrap());
        assert_eq!(
            *asked_of.lock().unwrap(),
            [ManaColor::Green, ManaColor::Green, ManaColor::Red]
        );
        assert_eq!(*taken.lock().unwrap(), 1, "called once");
    }

    /// The budget of the question the core asks in `steps`.
    fn budget(steps: &[Step]) -> Option<Duration> {
        steps.iter().find_map(|s| match s {
            Step::Ask(request) => Some(request.budget),
            _ => None,
        })
    }

    /// A mind handed over brings its profile's think time: every question
    /// after it is bounded by that, and the one it is first asked as soon
    /// as it is taken, where that is shorter.
    #[tokio::test]
    async fn a_swapped_mind_thinks_for_as_long_as_its_profile_says() {
        let mind = |colour| -> Arc<dyn Mind> {
            Arc::new(Named {
                colour,
                asked: Arc::new(Mutex::new(Vec::new())),
            })
        };
        let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
        let mut bridge = Bridge::new(
            mind(ManaColor::Red),
            PlayOptions::default(),
            Some(swaps),
            None,
        );
        let mut core = seated();
        let swap = |think| Swap {
            mind: mind(ManaColor::Green),
            think: Some(Duration::from_secs(think)),
            taken: None,
            declared: None,
        };
        send.send(swap(7))
            .unwrap_or_else(|_| panic!("the bridge listens"));
        // Taken before the core hears the question: it asks under 7 s, not
        // the bridge's 60.
        bridge.settle(&mut core);
        let first = asked(&mut core, 2);
        assert_eq!(budget(&first), Some(Duration::from_secs(7)));
        // Taken only as the question reaches the mind: held to 2 s at once.
        send.send(swap(2))
            .unwrap_or_else(|_| panic!("the bridge listens"));
        let before = Instant::now();
        bridge.carry_out(first, None).await;
        let thinking = bridge.thinking.take().unwrap();
        assert!(thinking.deadline <= before + Duration::from_secs(3));
        core.answered(thinking.question, thinking.task.await.unwrap());
        bridge.settle(&mut core);
        let second = asked(&mut core, 3);
        assert_eq!(budget(&second), Some(Duration::from_secs(2)));
    }

    /// A mind swapped during the game says what it is before it answers:
    /// the declaration is carried out ahead of the new mind's first
    /// answer, so the record writes the change where it happened.
    #[tokio::test]
    async fn a_swapped_mind_is_declared_before_its_first_answer() {
        let mind = |colour| -> Arc<dyn Mind> {
            Arc::new(Named {
                colour,
                asked: Arc::new(Mutex::new(Vec::new())),
            })
        };
        let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
        let mut bridge = Bridge::new(
            mind(ManaColor::Red),
            PlayOptions::default(),
            Some(swaps),
            None,
        );
        let mut core = seated();
        let first = asked(&mut core, 2);
        send.send(Swap {
            mind: mind(ManaColor::Green),
            think: None,
            taken: None,
            declared: Some(v1::SeatMind {
                kind: v1::seat_mind::Kind::LlmApi as i32,
                provider: "anthropic".into(),
                model: "claude-sonnet-5-5".into(),
                effort: "high".into(),
                level: String::new(),
            }),
        })
        .unwrap_or_else(|_| panic!("the bridge listens"));
        bridge.carry_out(first, None).await;
        let joined = (&mut bridge.thinking.as_mut().unwrap().task).await;
        let answer = bridge.answered(&mut core, joined).unwrap();
        let declared: Vec<_> = bridge
            .declaring
            .iter()
            .filter_map(|step| match step {
                Step::Send(Envelope {
                    msg: Some(v1::envelope::Msg::SeatMind(mind)),
                }) => Some(mind.model.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(declared, ["claude-sonnet-5-5"]);
        assert!(
            answer
                .iter()
                .any(|step| matches!(step, Step::Answer { .. })),
            "the new mind's answer follows"
        );
        bridge.carry_out(answer, None).await;
        assert!(
            bridge.declaring.is_empty(),
            "sent once, ahead of the answer"
        );
    }
    struct Ending(Arc<Mutex<Vec<(String, u8)>>>);
    impl Mind for Ending {
        fn decide(&self, _request: Request) -> Thinking<'_> {
            Box::pin(std::future::pending())
        }
        fn disclosure(&self) -> Disclosure {
            Disclosure::Llm
        }
        fn end_game(&self, game: &str, seat: u8) {
            self.0.lock().unwrap().push((game.into(), seat));
        }
    }

    #[tokio::test]
    async fn swapping_and_dropping_a_bridge_release_its_owned_session() {
        let ended = Arc::new(Mutex::new(Vec::new()));
        let mind: Arc<dyn Mind> = Arc::new(Ending(ended.clone()));
        let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(std::future::pending());
        let aborted = task.abort_handle();
        let mut bridge = Bridge::new(mind.clone(), PlayOptions::default(), Some(swaps), None);
        bridge.game_seat = Some(("game-owned".into(), 2));
        bridge.thinking = Some(super::Thinking {
            question: 1,
            deadline: Instant::now(),
            task,
            request: None,
        });
        send.send(Swap {
            mind: Arc::new(Ending(ended.clone())),
            think: None,
            taken: None,
            declared: None,
        })
        .unwrap();
        bridge.take_swaps();
        assert_eq!(*ended.lock().unwrap(), [("game-owned".into(), 2)]);
        drop(bridge);
        tokio::task::yield_now().await;
        assert!(aborted.is_finished());
        assert_eq!(
            *ended.lock().unwrap(),
            [("game-owned".into(), 2), ("game-owned".into(), 2)]
        );
        // Other users may still own the old mind; teardown is tied to this
        // bridge's game/seat rather than waiting for its last Arc to disappear.
        assert_eq!(Arc::strong_count(&mind), 1);
    }

    /// The turn the fallback tests play in, so that a turn the fallback
    /// reports cannot be a default.
    const TURN: u32 = 7;

    /// What a mind was asked, and which games it was told to end.
    #[derive(Default)]
    struct Seen {
        asked: Vec<Asked>,
        ended: Vec<(String, u8)>,
    }

    /// One question as a mind got it.
    struct Asked {
        question: u64,
        retried: bool,
        budget: Duration,
    }

    /// What a [`Moving`] mind wrote down, shared with the test.
    type Record = Arc<Mutex<Seen>>;

    /// What a [`Moving`] mind does at a call.
    #[derive(Clone, Copy)]
    enum Move {
        /// Answers this colour.
        Choose(ManaColor),
        /// The provider says it is rate limited (429, `Retry-After` 30 s).
        Limited,
        /// The provider failed otherwise ([`FAILED`], no limit).
        Unavailable,
        /// The mind will not answer this kind of question.
        Declined,
    }

    /// What a [`Move::Unavailable`] mind says: the staging acceptance's
    /// `502` (13.10.2026), the failure the house answered then.
    const FAILED: &str = "502: server_error: The website reported an error for this message";

    /// A mind that plays `moves` call by call, the last one from then on,
    /// and writes down what it was asked.
    struct Moving {
        moves: Vec<Move>,
        seen: Record,
    }

    impl Moving {
        fn mind(moves: Vec<Move>, seen: &Record) -> Arc<dyn Mind> {
            Arc::new(Self {
                moves,
                seen: Arc::clone(seen),
            })
        }
    }

    impl Mind for Moving {
        fn decide(&self, request: Request) -> Thinking<'_> {
            let call = {
                let mut seen = self.seen.lock().unwrap();
                seen.asked.push(Asked {
                    question: request.question,
                    retried: request.retry.is_some(),
                    budget: request.budget,
                });
                seen.asked.len()
            };
            let step = self.moves[(call - 1).min(self.moves.len() - 1)];
            Box::pin(async move {
                match step {
                    Move::Choose(colour) => Ok(Answer::new(PlayerAction::ChooseColor(colour))),
                    Move::Limited => Err(MindError::Limited {
                        why: "429: slow down".into(),
                        lifts_in: Some(Duration::from_secs(30)),
                    }),
                    Move::Unavailable => Err(MindError::Unavailable(FAILED.into())),
                    Move::Declined => Err(MindError::Declined("not this question".into())),
                }
            })
        }

        fn disclosure(&self) -> Disclosure {
            Disclosure::Llm
        }

        fn end_game(&self, game: &str, seat: u8) {
            // Called as the bridge is dropped, even by a test that failed.
            let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
            seen.ended.push((game.into(), seat));
        }
    }

    /// The limits a standby's arm was called with.
    type Armed = Arc<Mutex<Vec<Limit>>>;

    /// What a model behind an API says it is, for the table.
    fn api_mind(model: &str) -> v1::SeatMind {
        v1::SeatMind {
            kind: v1::seat_mind::Kind::LlmApi as i32,
            provider: "openai".into(),
            model: model.into(),
            effort: String::new(),
            level: String::new(),
        }
    }

    /// What stands by in a [`Scene`].
    enum Reserve {
        /// A fallback `backup` that takes over with a mind playing these
        /// moves, declared as `model-b`, thinking for 20 s.
        Takes(Vec<Move>),
        /// A fallback `backup` whose arm refuses, with this reason.
        Refuses(&'static str),
        /// No fallback.
        Absent,
    }

    /// A bridge whose primary mind plays `primary`, a core seated before it
    /// (every question in turn [`TURN`]), and what the minds and the arm
    /// wrote down. The bridge is driven as `run` drives it, a step at a
    /// time, with no socket unless the test gives it one.
    struct Scene {
        bridge: Bridge,
        core: SeatCore,
        primary: Record,
        fallback: Record,
        armed: Armed,
        wire: Option<Socket>,
    }

    impl Scene {
        fn new(primary: Vec<Move>, reserve: Reserve) -> Self {
            let (primary_seen, fallback_seen) = (Record::default(), Record::default());
            let armed = Armed::default();
            let standby = match reserve {
                Reserve::Takes(moves) => {
                    let mind = Moving::mind(moves, &fallback_seen);
                    let (armed, declared) = (Arc::clone(&armed), api_mind("model-b"));
                    Some(Standby {
                        label: "backup".into(),
                        arm: Box::new(move |limit| {
                            armed.lock().unwrap().push(limit.clone());
                            Ok(Swap {
                                mind,
                                think: Some(Duration::from_secs(20)),
                                taken: None,
                                declared: Some(declared),
                            })
                        }),
                    })
                }
                Reserve::Refuses(why) => {
                    let armed = Arc::clone(&armed);
                    Some(Standby {
                        label: "backup".into(),
                        arm: Box::new(move |limit| {
                            armed.lock().unwrap().push(limit.clone());
                            Err(why.into())
                        }),
                    })
                }
                Reserve::Absent => None,
            };
            let options = PlayOptions {
                min_think: Duration::ZERO,
                ..PlayOptions::default()
            };
            Self {
                bridge: Bridge::new(Moving::mind(primary, &primary_seen), options, None, standby),
                core: seated(),
                primary: primary_seen,
                fallback: fallback_seen,
                armed,
                wire: None,
            }
        }

        /// Carries `steps` out, on the socket if there is one.
        async fn carry(&mut self, steps: Vec<Step>) {
            self.bridge.carry_out(steps, self.wire.as_mut()).await;
        }

        /// The seat is asked a question at `seq` and the bridge hands it to
        /// its mind; the budget the core gave it.
        async fn ask(&mut self, seq: u64) -> Duration {
            let steps = asked_in_turn(&mut self.core, seq, TURN);
            let budget = budget(&steps).expect("the mind is asked");
            self.carry(steps).await;
            budget
        }

        /// The mind that is thinking comes back, and the core's steps for
        /// what it said.
        async fn back(&mut self) -> Vec<Step> {
            let task = &mut self
                .bridge
                .thinking
                .as_mut()
                .expect("a question is being thought about")
                .task;
            let joined = task.await;
            self.bridge
                .answered(&mut self.core, joined)
                .expect("the question was being thought about")
        }

        /// [`Self::back`] and what comes of it carried out: who answered
        /// with what. Where the mind was limited and a fallback took the
        /// question over, the fallback is waited for too.
        async fn answer(&mut self) -> Vec<(By, PlayerAction)> {
            let mut steps = self.back().await;
            if steps.is_empty() && self.bridge.thinking.is_some() {
                self.carry(steps).await;
                steps = self.back().await;
            }
            let made = answers(&steps);
            self.carry(steps).await;
            made
        }
    }

    /// Who answered with what, in `steps`.
    fn answers(steps: &[Step]) -> Vec<(By, PlayerAction)> {
        steps
            .iter()
            .filter_map(|step| match step {
                Step::Answer { by, action, .. } => Some((*by, action.clone())),
                _ => None,
            })
            .collect()
    }

    /// The models a bridge is about to declare to the table.
    fn declaring(bridge: &Bridge) -> Vec<String> {
        bridge
            .declaring
            .iter()
            .filter_map(|step| match step {
                Step::Send(Envelope {
                    msg: Some(v1::envelope::Msg::SeatMind(mind)),
                }) => Some(mind.model.clone()),
                _ => None,
            })
            .collect()
    }

    /// The error of every `MindFailed` note the core wrote.
    fn failures(core: &mut SeatCore) -> Vec<String> {
        core.take_notes()
            .into_iter()
            .filter_map(|note| match note.event {
                Event::MindFailed { error } => Some(error),
                _ => None,
            })
            .collect()
    }

    /// A limit where a fallback stands by never reaches the core: the same
    /// question is asked of the fallback, which answers it and the ones
    /// after it, and nothing is the house's. (Before, the limit was an
    /// `Unavailable`: the house answered that question.)
    #[tokio::test]
    async fn a_limit_where_a_fallback_stands_by_is_answered_by_the_fallback() {
        let mut scene = Scene::new(
            vec![Move::Choose(ManaColor::Red), Move::Limited],
            Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
        );
        // The primary answers while it can, and the fallback is not armed.
        scene.ask(2).await;
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
        );
        assert!(scene.armed.lock().unwrap().is_empty());
        assert_eq!(scene.bridge.fell_back, None);

        // The second question is the one the primary cannot answer.
        let budget = scene.ask(3).await;
        let thinking = scene.bridge.thinking.as_ref().unwrap();
        let (question, deadline) = (thinking.question, thinking.deadline);
        let steps = scene.back().await;
        assert!(steps.is_empty(), "the core hears nothing of it: {steps:?}");
        let thinking = scene
            .bridge
            .thinking
            .as_ref()
            .expect("asked of the fallback");
        assert_eq!(
            (thinking.question, thinking.deadline),
            (question, deadline),
            "the same question, under the same deadline"
        );
        scene.carry(steps).await;
        let context = scene.core.context().unwrap().clone();
        assert_eq!(
            *scene.armed.lock().unwrap(),
            [Limit {
                turn: TURN,
                why: "429: slow down".into(),
                lifts_in: Some(Duration::from_secs(30)),
                failed: false,
            }],
            "armed once, on the limit"
        );
        assert_eq!(
            scene.primary.lock().unwrap().ended,
            [(context.game_id.clone(), context.seat.get())],
            "the primary plays no more of this game"
        );
        assert_eq!(
            scene.bridge.notes,
            [serde_json::json!({
                "fallback": {
                    "profile": "backup",
                    "turn": TURN,
                    "cause": "limited",
                    "why": "429: slow down",
                },
            })]
        );
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))],
            "the fallback answers the question the primary could not"
        );
        {
            let fallback = scene.fallback.lock().unwrap();
            assert_eq!(fallback.asked.len(), 1);
            assert_eq!(fallback.asked[0].question, question);
            assert!(!fallback.asked[0].retried);
            assert!(fallback.asked[0].budget <= budget, "within what was left");
            assert!(fallback.asked[0].budget + Duration::from_secs(5) > budget);
        }

        // And the questions after it.
        scene.ask(4).await;
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
        );
        assert_eq!(scene.primary.lock().unwrap().asked.len(), 2);
        assert_eq!(scene.fallback.lock().unwrap().asked.len(), 2);
        assert_eq!(scene.armed.lock().unwrap().len(), 1, "armed once");
        assert_eq!(
            scene.bridge.fell_back,
            Some(FellBack {
                label: "backup".into(),
                turn: TURN,
                answered: 2,
            }),
            "the fallback's two answers, not the primary's"
        );
        let stats = scene.core.stats();
        assert_eq!(
            (stats.answered.mind, stats.answered.house, stats.asked),
            (3, 0, 3),
            "three questions, each asked of the core's mind once"
        );
        assert_eq!(stats.fallbacks.total(), 0, "no house answer, no failure");
        assert_eq!(stats.mind_down, 0);
        assert!(failures(&mut scene.core).is_empty());
    }

    /// The fallback is asked the question afresh: an answer the primary
    /// gave and the referee refused was the primary's, so the retry is
    /// cleared, and the time it has is what is left to the deadline the
    /// primary was under, not a new budget.
    #[tokio::test]
    async fn a_fallback_is_asked_the_question_afresh_within_the_time_that_is_left() {
        // Blue is not among the colours on offer: the referee refuses it,
        // and the primary is asked again, told why, and is limited then.
        let mut scene = Scene::new(
            vec![Move::Choose(ManaColor::Blue), Move::Limited],
            Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
        );
        scene.ask(2).await;
        let retry = scene.back().await;
        assert!(
            matches!(&retry[..], [Step::Ask(request)] if request.retry.is_some()),
            "asked again with the refusal: {retry:?}"
        );
        let retry_budget = budget(&retry).unwrap();
        scene.carry(retry).await;
        let deadline = scene.bridge.thinking.as_ref().unwrap().deadline;

        assert!(scene.back().await.is_empty(), "the limit is not the core's");
        assert_eq!(scene.bridge.thinking.as_ref().unwrap().deadline, deadline);
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
        );
        let (retried, question) = {
            let primary = scene.primary.lock().unwrap();
            assert_eq!(primary.asked.len(), 2);
            (primary.asked[1].retried, primary.asked[1].question)
        };
        assert!(retried, "the primary was asked with the refusal");
        let fallback = scene.fallback.lock().unwrap();
        assert_eq!(fallback.asked.len(), 1);
        assert!(
            !fallback.asked[0].retried,
            "a first asking for the fallback"
        );
        assert_eq!(fallback.asked[0].question, question);
        assert!(fallback.asked[0].budget <= retry_budget);
        assert!(fallback.asked[0].budget + Duration::from_secs(5) > retry_budget);
    }

    /// The fallback says what it is to the table before its first answer,
    /// so the game's record writes the change where it happened: on the
    /// wire the primary's answer comes first, then the declaration, then
    /// the fallback's.
    #[tokio::test]
    async fn the_fallback_is_declared_before_its_first_answer() {
        let mut scene = Scene::new(
            vec![Move::Choose(ManaColor::Red), Move::Limited],
            Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
        );
        let (socket, mut table) = wired().await;
        scene.wire = Some(socket);
        scene.ask(2).await;
        scene.answer().await;
        scene.ask(3).await;
        // The limit is taken over as `run` takes it: nothing to send but
        // the declaration, which waits to be carried out.
        let steps = scene.back().await;
        assert!(steps.is_empty());
        assert_eq!(declaring(&scene.bridge), ["model-b"]);
        scene.carry(steps).await;
        assert!(scene.bridge.declaring.is_empty(), "sent once, at once");
        scene.answer().await;
        let mut wire = Vec::new();
        for _ in 0..3 {
            let envelope = tokio::time::timeout(Duration::from_secs(5), table.recv())
                .await
                .expect("the table is sent three frames")
                .expect("the socket stays open");
            wire.push(match envelope.msg {
                Some(v1::envelope::Msg::PlayerAction(msg)) => {
                    let action: PlayerAction = serde_json::from_slice(&msg.action_json).unwrap();
                    format!("answer {action:?}")
                }
                Some(v1::envelope::Msg::SeatMind(mind)) => format!("mind {}", mind.model),
                other => format!("{other:?}"),
            });
        }
        assert_eq!(
            wire,
            [
                "answer ChooseColor(Red)",
                "mind model-b",
                "answer ChooseColor(Green)"
            ]
        );
    }

    /// The fallback brings its profile's think time: the questions after
    /// the take-over are bounded by it, not by the primary's.
    #[tokio::test]
    async fn the_fallback_thinks_for_as_long_as_its_profile_says() {
        let mut scene = Scene::new(
            vec![Move::Limited],
            Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
        );
        let before = scene.ask(2).await;
        assert_eq!(before, Duration::from_secs(60), "the primary's bound");
        scene.answer().await;
        assert_eq!(scene.ask(3).await, Duration::from_secs(20));
    }

    /// Where the fallback cannot take over (its own spend book refuses, say)
    /// the limit is the core's as without one: the house answers and plays
    /// the rest of the game, the transcript says why the fallback did not,
    /// and the fallback stands by no longer.
    #[tokio::test]
    async fn a_fallback_that_cannot_take_over_leaves_the_limit_to_the_house() {
        let refusal = "the day's cap of $3.00 is spent";
        let mut scene = Scene::new(
            vec![Move::Choose(ManaColor::Red), Move::Limited],
            Reserve::Refuses(refusal),
        );
        scene.ask(2).await;
        scene.answer().await;
        scene.ask(3).await;
        let steps = scene.back().await;
        let made = answers(&steps);
        assert!(
            matches!(&made[..], [(By::House, _)]),
            "the house answers: {made:?}"
        );
        scene.carry(steps).await;

        let failed = failures(&mut scene.core);
        assert_eq!(failed.len(), 1, "{failed:?}");
        assert!(failed[0].contains("rate limited"), "{}", failed[0]);
        assert!(
            failed[0].contains("the fallback backup could not take over"),
            "{}",
            failed[0]
        );
        assert!(failed[0].contains(refusal), "{}", failed[0]);
        assert_eq!(
            scene.bridge.notes,
            [serde_json::json!({
                "fallback_refused": { "profile": "backup", "why": refusal },
            })]
        );
        assert!(scene.bridge.fallback.is_none(), "it stands by no longer");
        assert_eq!(scene.bridge.fell_back, None);
        assert!(
            scene.primary.lock().unwrap().ended.is_empty(),
            "the primary plays on"
        );
        let stats = scene.core.stats();
        assert_eq!(
            (stats.answered.house, stats.fallbacks.unavailable),
            (1, 1),
            "the same road as the mind unavailable"
        );

        // The house has the rest of the game (`SeatCore::limited`): neither
        // the mind is asked again nor the fallback armed again.
        scene.bridge.notes.clear();
        let steps = put_in_turn(&mut scene.core, 4, TURN);
        assert!(budget(&steps).is_none(), "the mind was asked again");
        assert!(matches!(&answers(&steps)[..], [(By::House, _)]));
        scene.carry(steps).await;
        assert_eq!(scene.armed.lock().unwrap().len(), 1, "armed once");
        assert!(scene.bridge.notes.is_empty(), "{:?}", scene.bridge.notes);
        assert_eq!(scene.primary.lock().unwrap().asked.len(), 2);
        assert!(scene.core.stats().housed);
    }

    /// With no fallback a limit hands the rest of the game to the house at
    /// once (feedback of 28.10.2026): the house is declared to the table,
    /// answers the question the mind could not, and every one after it; the
    /// mind is asked nothing more and the seat stays at the table. Before,
    /// a run of three limits took the mind off the table: the bridge left
    /// its socket, the table held the chair as away for its whole reconnect
    /// window before its stand-in played, and the mind came back once it
    /// had cooled down, to be limited again.
    #[tokio::test]
    async fn without_a_fallback_a_limit_hands_the_rest_of_the_game_to_the_house() {
        let mut scene = Scene::new(vec![Move::Limited], Reserve::Absent);
        scene.ask(2).await;
        let steps = scene.back().await;
        let house = steps.iter().position(|step| {
            matches!(step, Step::Send(Envelope { msg: Some(v1::envelope::Msg::SeatMind(mind)) })
                if mind.kind == v1::seat_mind::Kind::House as i32)
        });
        let answered = steps
            .iter()
            .position(|step| matches!(step, Step::Answer { by: By::House, .. }));
        assert!(
            matches!((house, answered), (Some(h), Some(a)) if h < a),
            "the house is declared ahead of its first answer: {steps:?}"
        );
        assert!(!steps.iter().any(|step| matches!(step, Step::MindDown)));
        scene.carry(steps).await;
        for seq in [3, 4, 5] {
            let steps = put_in_turn(&mut scene.core, seq, TURN);
            assert!(budget(&steps).is_none(), "the mind was asked at {seq}");
            assert!(
                matches!(&answers(&steps)[..], [(By::House, _)]),
                "{steps:?}"
            );
            assert!(!steps.iter().any(|step| matches!(step, Step::MindDown)));
            scene.carry(steps).await;
        }
        assert_eq!(scene.primary.lock().unwrap().asked.len(), 1);
        let stats = scene.core.stats();
        assert_eq!(
            (stats.answered.house, stats.mind_down, stats.housed),
            (4, 0, true)
        );
        assert_eq!(
            failures(&mut scene.core)[0],
            "the mind is rate limited: 429: slow down (it lifts in 30 s)"
        );
        assert!(scene.bridge.notes.is_empty());
        assert_eq!(scene.bridge.fell_back, None);
        assert!(scene.armed.lock().unwrap().is_empty());
    }

    /// A fallback that is limited too has nothing behind it: the limit is
    /// the core's, the house answers, and only what the fallback answered
    /// is counted as its own.
    #[tokio::test]
    async fn a_fallback_that_is_limited_too_leaves_the_limit_to_the_house() {
        let mut scene = Scene::new(
            vec![Move::Limited],
            Reserve::Takes(vec![Move::Choose(ManaColor::Green), Move::Limited]),
        );
        scene.ask(2).await;
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
        );
        scene.ask(3).await;
        let made = scene.answer().await;
        assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
        assert_eq!(scene.armed.lock().unwrap().len(), 1, "armed once");
        assert_eq!(scene.fallback.lock().unwrap().asked.len(), 2);
        assert_eq!(
            scene.bridge.fell_back.as_ref().map(|fell| fell.answered),
            Some(1),
            "the house's answer is not the fallback's"
        );
        let stats = scene.core.stats();
        assert_eq!((stats.answered.house, stats.fallbacks.unavailable), (1, 1));
    }

    /// A mind handed over during the game plays another profile, so the
    /// fallback held for the old one is let go: a limit of the new mind
    /// goes to the core, and the fallback is never armed.
    #[tokio::test]
    async fn a_swapped_mind_drops_the_fallback_held_for_the_old_one() {
        let mut scene = Scene::new(
            vec![Move::Choose(ManaColor::Red)],
            Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
        );
        let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
        scene.bridge.swaps = Some(swaps);
        let handed = Record::default();
        send.send(Swap {
            mind: Moving::mind(vec![Move::Limited], &handed),
            think: None,
            taken: None,
            declared: None,
        })
        .unwrap_or_else(|_| panic!("the bridge listens"));
        scene.bridge.settle(&mut scene.core);
        assert!(scene.bridge.fallback.is_none());
        scene.ask(2).await;
        let made = scene.answer().await;
        assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
        assert_eq!(handed.lock().unwrap().asked.len(), 1);
        assert!(scene.armed.lock().unwrap().is_empty());
        assert!(scene.fallback.lock().unwrap().asked.is_empty());
        assert_eq!(scene.core.stats().fallbacks.unavailable, 1);
    }

    /// The fallback's answers are counted for the fallback only while it
    /// plays: a mind handed over after it took over is another profile's,
    /// and what it answers is not the fallback's.
    #[tokio::test]
    async fn a_mind_handed_over_after_the_fallback_is_not_counted_as_the_fallback() {
        let mut scene = Scene::new(
            vec![Move::Limited],
            Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
        );
        scene.ask(2).await;
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
        );
        let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
        scene.bridge.swaps = Some(swaps);
        let handed = Record::default();
        send.send(Swap {
            mind: Moving::mind(vec![Move::Choose(ManaColor::Red)], &handed),
            think: None,
            taken: None,
            declared: None,
        })
        .unwrap_or_else(|_| panic!("the bridge listens"));
        scene.bridge.settle(&mut scene.core);
        scene.ask(3).await;
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
        );
        assert_eq!(
            scene.bridge.fell_back.as_ref().map(|fell| fell.answered),
            Some(1),
            "only the fallback's own answer"
        );
        assert_eq!(scene.core.stats().answered.mind, 2);
    }

    /// The staging acceptance (13.10.2026): the primary answered `502`
    /// mid-game, an `Unavailable`, and the house took the question though a
    /// fallback stood by. A failure is now taken over as a limit is: the
    /// same question goes to the fallback under the same deadline, it plays
    /// the rest of the game, and the house answers nothing.
    #[tokio::test]
    async fn a_primary_that_fails_where_a_fallback_stands_by_is_answered_by_the_fallback() {
        let mut scene = Scene::new(
            vec![
                Move::Choose(ManaColor::Red),
                Move::Choose(ManaColor::Red),
                Move::Unavailable,
            ],
            Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
        );
        for seq in [2, 3] {
            scene.ask(seq).await;
            assert_eq!(
                scene.answer().await,
                [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
            );
        }
        assert!(scene.armed.lock().unwrap().is_empty(), "not armed yet");

        // The third question is the one the primary fails at.
        let budget = scene.ask(4).await;
        let thinking = scene.bridge.thinking.as_ref().unwrap();
        let (question, deadline) = (thinking.question, thinking.deadline);
        let steps = scene.back().await;
        assert!(steps.is_empty(), "the core hears nothing of it: {steps:?}");
        let thinking = scene
            .bridge
            .thinking
            .as_ref()
            .expect("asked of the fallback");
        assert_eq!((thinking.question, thinking.deadline), (question, deadline));
        assert_eq!(declaring(&scene.bridge), ["model-b"], "declared first");
        scene.carry(steps).await;
        let limit = Limit {
            turn: TURN,
            why: FAILED.into(),
            lifts_in: None,
            failed: true,
        };
        assert_eq!(*scene.armed.lock().unwrap(), std::slice::from_ref(&limit));
        assert_eq!(limit.said(), format!("the primary failed: {FAILED}"));
        assert!(
            !limit.said().contains(LIMITED),
            "a seat agent does not hear a limit"
        );
        assert_eq!(
            scene.bridge.notes,
            [serde_json::json!({
                "fallback": {
                    "profile": "backup",
                    "turn": TURN,
                    "cause": "failed",
                    "why": FAILED,
                },
            })]
        );
        let context = scene.core.context().unwrap().clone();
        assert_eq!(
            scene.primary.lock().unwrap().ended,
            [(context.game_id.clone(), context.seat.get())],
            "the primary plays no more of this game"
        );
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))],
            "the fallback answers the question the primary failed at"
        );
        {
            let fallback = scene.fallback.lock().unwrap();
            assert_eq!(fallback.asked.len(), 1);
            assert_eq!(fallback.asked[0].question, question);
            assert!(!fallback.asked[0].retried);
            assert!(fallback.asked[0].budget <= budget, "within what was left");
        }

        // And the questions after it, the primary never asked again.
        for seq in [5, 6] {
            scene.ask(seq).await;
            assert_eq!(
                scene.answer().await,
                [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
            );
        }
        assert_eq!(scene.primary.lock().unwrap().asked.len(), 3);
        assert_eq!(scene.fallback.lock().unwrap().asked.len(), 3);
        assert_eq!(scene.armed.lock().unwrap().len(), 1, "armed once");
        assert_eq!(
            scene.bridge.fell_back,
            Some(FellBack {
                label: "backup".into(),
                turn: TURN,
                answered: 3,
            })
        );
        let stats = scene.core.stats();
        assert_eq!(
            (stats.answered.mind, stats.answered.house, stats.asked),
            (5, 0, 5)
        );
        assert_eq!(stats.fallbacks.total(), 0, "no house answer, no failure");
        assert_eq!(stats.mind_down, 0);
        assert!(failures(&mut scene.core).is_empty());
    }

    /// Without a fallback a failure of the mind is the house's, as it was:
    /// it counts as the mind unavailable, a run of them takes the mind off
    /// the table, and the transcript says what failed.
    #[tokio::test]
    async fn without_a_fallback_a_failure_is_the_house_s_to_answer() {
        let mut scene = Scene::new(vec![Move::Unavailable], Reserve::Absent);
        for (seq, down) in [(2, false), (3, false), (4, true)] {
            scene.ask(seq).await;
            let steps = scene.back().await;
            let made = answers(&steps);
            assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
            assert_eq!(
                steps.iter().any(|step| matches!(step, Step::MindDown)),
                down,
                "taken off the table on the third in a row"
            );
            scene.carry(steps).await;
        }
        let stats = scene.core.stats();
        assert_eq!(
            (
                stats.answered.house,
                stats.fallbacks.unavailable,
                stats.mind_down
            ),
            (3, 3, 1)
        );
        assert_eq!(
            failures(&mut scene.core)[0],
            format!("the mind is unavailable: {FAILED}")
        );
        assert!(scene.bridge.notes.is_empty());
        assert_eq!(scene.bridge.fell_back, None);
    }

    /// Where the fallback cannot take over a failure, the house answers
    /// and the failure says both why the mind failed and why the fallback
    /// did not; it is still the mind unavailable, never a limit.
    #[tokio::test]
    async fn a_fallback_that_cannot_take_over_a_failure_leaves_it_to_the_house() {
        let refusal = "the day's cap of $3.00 is spent";
        let mut scene = Scene::new(vec![Move::Unavailable], Reserve::Refuses(refusal));
        scene.ask(2).await;
        let steps = scene.back().await;
        let made = answers(&steps);
        assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
        scene.carry(steps).await;
        assert_eq!(
            failures(&mut scene.core),
            [format!(
                "the mind is unavailable: {FAILED}; the fallback backup could not take over: \
                 {refusal}"
            )]
        );
        assert_eq!(
            *scene.armed.lock().unwrap(),
            [Limit {
                turn: TURN,
                why: FAILED.into(),
                lifts_in: None,
                failed: true,
            }]
        );
        assert_eq!(
            scene.bridge.notes,
            [serde_json::json!({
                "fallback_refused": { "profile": "backup", "why": refusal },
            })]
        );
        assert!(scene.bridge.fallback.is_none(), "it stands by no longer");
        assert_eq!(scene.bridge.fell_back, None);
        let stats = scene.core.stats();
        assert_eq!((stats.answered.house, stats.fallbacks.unavailable), (1, 1));
    }

    /// A mind that declines a question is not down: the house answers that
    /// one, the fallback is not armed and stands by still, and the mind
    /// answers the next.
    #[tokio::test]
    async fn a_declined_question_is_the_house_s_and_the_fallback_stands_by() {
        let mut scene = Scene::new(
            vec![Move::Declined, Move::Choose(ManaColor::Red)],
            Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
        );
        scene.ask(2).await;
        let made = scene.answer().await;
        assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
        assert!(scene.armed.lock().unwrap().is_empty());
        assert!(scene.bridge.fallback.is_some(), "it stands by still");
        scene.ask(3).await;
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
        );
        assert!(scene.fallback.lock().unwrap().asked.is_empty());
        assert_eq!(scene.bridge.fell_back, None);
    }

    /// What the operator reads of a limit is the words a seat agent holds
    /// the profile back on, with when it lifts; never the provider's own.
    #[test]
    fn a_limit_is_said_in_the_words_a_seat_agent_reads() {
        let limit = Limit {
            turn: 3,
            why: "429: slow down".into(),
            lifts_in: Some(Duration::from_secs(120)),
            failed: false,
        };
        assert_eq!(
            limit.said(),
            "the primary is rate limited (it lifts in 120 s)"
        );
        let unsaid = Limit {
            lifts_in: None,
            ..limit
        };
        assert_eq!(unsaid.said(), LIMITED);
    }

    /// A stand-in table on this machine: a socket to it, and every frame it
    /// is sent, decoded.
    async fn wired() -> (Socket, tokio::sync::mpsc::UnboundedReceiver<Envelope>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (frames, table) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            while let Some(Ok(message)) = socket.next().await {
                if let Message::Binary(bytes) = message {
                    let _ = frames.send(Envelope::decode(&bytes[..]).unwrap());
                }
            }
        });
        let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}"))
            .await
            .unwrap();
        (socket, table)
    }
}
