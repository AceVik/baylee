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
mod tests;
