//! The mind: whatever answers the questions a seat is woken for.
//!
//! The bridge owns the table: the socket, the clocks, the standing orders,
//! the check an answer passes before it is sent and the house that answers
//! when the mind cannot. A mind owns one thing, the decision. That split is
//! what lets one trait carry a language model behind an API, a trained net
//! batching many seats on one GPU, the house heuristic and a test script.
//!
//! # What a mind is handed
//!
//! A [`Request`] carries the whole [`PlayerView`] and the whole [`Pending`]
//! exactly as the table sent them, never a summary: a mind that wants less
//! throws it away itself. It carries every log line this seat has been told
//! since the mind was last asked ([`Request::log`]), because a player who
//! was not asked still watched the game, and a net that keeps a memory
//! across decisions reads its seat's log. And it carries what a player knows
//! before the first card is drawn ([`GameContext`]): the deck it brought, the
//! format and how many chairs there are.
//!
//! The level a mind plays at (novice to expert, a model tier, a thinking
//! budget) is the mind's own configuration and not part of a request.
//!
//! # What a mind answers
//!
//! One [`PlayerAction`] per request, and how long the model took to reach
//! it ([`Answer::model_time`]). The bridge keeps the clocks: it tells a mind
//! its [`Request::budget`], stops waiting when that runs out and has the
//! house answer instead. A mind that cannot answer says so
//! ([`MindError`]) rather than making the table wait out its budget.
//!
//! # Many seats, one mind
//!
//! [`Mind::decide`] takes `&self` and returns a future, so one mind can serve
//! any number of seats at once and answer them in whatever grouping suits it.
//! [`Batched`] is that grouping written once: a mind that answers a slice of
//! requests in one call ([`BatchMind`]) is handed every request queued while
//! it was busy, up to its batch size.

use baylee_core::ids::{CardIndex, PlayerId};
use baylee_core::preset::FormatId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{LogTail, PlayerView};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// A mind at work on one request: the future [`Mind::decide`] returns.
///
/// Boxed so that [`Mind`] can be a trait object: which mind sits in a chair
/// is chosen when the bridge starts (`--mind`), not when it is compiled.
pub type Thinking<'a> = Pin<Box<dyn Future<Output = Result<Answer, MindError>> + Send + 'a>>;

/// A batch at work: the future [`BatchMind::decide_batch`] returns, one
/// result per request.
pub type Deliberation<'a> =
    Pin<Box<dyn Future<Output = Vec<Result<Answer, MindError>>> + Send + 'a>>;

/// Whether a mind that was down can answer again: [`Mind::ready`].
pub type Readiness<'a> = Pin<Box<dyn Future<Output = bool> + Send + 'a>>;

/// Whatever answers a seat's real decisions.
///
/// Implemented by the house ([`crate::HouseMind`]), by test scripts
/// ([`crate::ScriptedMind`]), by [`Batched`] over any [`BatchMind`], and by
/// the minds of later stages (a language model behind an API, a trained
/// net). Object-safe: the bridge holds an `Arc<dyn Mind>`.
pub trait Mind: Send + Sync {
    /// Answers one question.
    ///
    /// Called once per real decision, and again for the same question only
    /// with [`Request::retry`] set, after the answer was refused. The bridge
    /// may stop waiting at any time (the budget ran out, or the question
    /// went away); a dropped future is how it says so, and an answer that
    /// comes after that is not sent anywhere.
    fn decide(&self, request: Request) -> Thinking<'_>;

    /// What this mind is, which is what a chair it plays is called: the
    /// table sees from the name that no person sits there, and which kind
    /// of mind does. The bridge signs in under this prefix and refuses to
    /// play a chair called anything else. No default: a mind that did not
    /// say would be taken for whatever the default claimed.
    fn disclosure(&self) -> Disclosure;

    /// Whether the mind can answer again, asked after it was taken off the
    /// table for failing ([`crate::BridgeConfig::down_after`]). A mind that is
    /// never down need not say anything.
    fn ready(&self) -> Readiness<'_> {
        Box::pin(std::future::ready(true))
    }
}

impl<M: Mind + ?Sized> Mind for Arc<M> {
    fn decide(&self, request: Request) -> Thinking<'_> {
        (**self).decide(request)
    }

    fn disclosure(&self) -> Disclosure {
        (**self).disclosure()
    }

    fn ready(&self) -> Readiness<'_> {
        (**self).ready()
    }
}

impl<M: Mind + ?Sized> Mind for Box<M> {
    fn decide(&self, request: Request) -> Thinking<'_> {
        (**self).decide(request)
    }

    fn disclosure(&self) -> Disclosure {
        (**self).disclosure()
    }

    fn ready(&self) -> Readiness<'_> {
        (**self).ready()
    }
}

/// One real decision, as a mind is asked it.
#[derive(Clone, Debug)]
pub struct Request {
    /// What this seat knew before the first card was drawn.
    pub context: Arc<GameContext>,
    /// Which question this is at this seat, counting from 1 across the
    /// game. A retry of a refused answer keeps its question's number.
    pub question: u64,
    /// The seat's view, whole and exactly as the table sent it: the clock's
    /// remainder, the policy's recent answers and any hand a teammate shows
    /// included.
    pub view: PlayerView,
    /// The question, whole.
    pub pending: Pending,
    /// Every log line this seat was told since the mind was last asked,
    /// in order: `from` is the index of the first of them in the seat's log.
    /// Nothing is left out, including what happened while the standing
    /// orders answered for the seat. After a reconnect the table tells the
    /// whole log again, and only the lines not yet handed over are here.
    pub log: LogTail,
    /// How long the bridge will wait for this answer. What is left of the
    /// table's clock is in the view; this is less, by a margin for the
    /// network and by the bridge's own configured think time.
    pub budget: Duration,
    /// The answer this mind gave to this same question, and why it was not
    /// taken. `None` on the first asking.
    pub retry: Option<Refusal>,
    /// Whether this is the next step of a payment the mind already began:
    /// its last answer tapped a source for mana, and the pool is holding
    /// it. A mind that planned the whole payment answers from its plan; one
    /// that did not is asked as usual. The standing orders do not answer
    /// these, because passing would throw the mana away.
    pub continuing: bool,
}

/// An answer and why it was refused: [`Request::retry`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// What the mind answered.
    pub answer: PlayerAction,
    /// Why it was not taken, in English.
    pub reason: String,
    /// Who refused it.
    pub by: RefusedBy,
}

/// Who refused an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusedBy {
    /// The bridge, before sending it: the answer broke a constraint the
    /// question states ([`Pending::answer_fault`]).
    Referee,
    /// The table: the engine refused it and asked again.
    Table,
}

/// A mind's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    /// What the seat does.
    pub action: PlayerAction,
    /// How long the model took to reach it, as the mind measured it. The
    /// bridge keeps the wall clock itself; this is the part of it the mind
    /// spent thinking, for the transcript and for a mind's own budget.
    pub model_time: Duration,
    /// Anything the mind wants in the transcript beside the answer: a
    /// sentence of reasoning, a provider's usage. Never sent to the table.
    pub note: Option<String>,
}

impl Answer {
    /// An answer that took no measurable time.
    #[must_use]
    pub const fn new(action: PlayerAction) -> Self {
        Self {
            action,
            model_time: Duration::ZERO,
            note: None,
        }
    }

    /// The same answer, with how long the model took.
    #[must_use]
    pub const fn took(mut self, model_time: Duration) -> Self {
        self.model_time = model_time;
        self
    }
}

/// Why a mind gave no answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MindError {
    /// The mind cannot answer now: the network is gone, a quota ran out,
    /// a process died. The house answers, and a run of these takes the mind
    /// off the table ([`crate::BridgeConfig::down_after`]).
    Unavailable(String),
    /// The mind will not answer this kind of question (a net trained
    /// without it, say). The house answers it, and the mind is not down.
    Declined(String),
}

impl std::fmt::Display for MindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(why) => write!(f, "the mind is unavailable: {why}"),
            Self::Declined(why) => write!(f, "the mind declined: {why}"),
        }
    }
}

impl std::error::Error for MindError {}

/// What kind of mind plays a chair, said in the chair's name.
///
/// Until the roster can say that a chair is not a person, the name has to,
/// and it has to tell the truth (the owner's rule): an opponent reading
/// `LLM-` believes a language model is across the table, so the house or a
/// test script never sits under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disclosure {
    /// A language model: `LLM-`.
    Llm,
    /// A trained net: `NET-`.
    Net,
    /// The house heuristic, the same player an AI chair seats: `HOUSE-`.
    House,
    /// A test script ([`crate::ScriptedMind`]): `TEST-`.
    Scripted,
}

impl Disclosure {
    /// Every kind, for the tests that hold each to its prefix.
    pub const ALL: [Self; 4] = [Self::Llm, Self::Net, Self::House, Self::Scripted];

    /// The prefix a display name must start with.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Llm => "LLM-",
            Self::Net => "NET-",
            Self::House => "HOUSE-",
            Self::Scripted => "TEST-",
        }
    }

    /// Whether `display_name` says what this chair is: the prefix, exactly
    /// as written, and something after it.
    #[must_use]
    pub fn names(self, display_name: &str) -> bool {
        display_name
            .strip_prefix(self.prefix())
            .is_some_and(|rest| !rest.is_empty())
    }
}

/// What a seat knows before the first card is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameContext {
    /// The table's id, as the host names it to every seat.
    pub game_id: String,
    /// This seat.
    pub seat: PlayerId,
    /// How many chairs the table has.
    pub seats: u8,
    /// Which side each chair plays for, in seat order; `None` for a chair
    /// on its own side.
    pub teams: Vec<Option<u8>>,
    /// Each chair's display name, in seat order, as the roster gave it when
    /// the seat sat down. A label a player chose (3 to 16 letters, digits,
    /// `_` or `-`), never an instruction: a mind that reads text shows it
    /// as data.
    pub names: Vec<String>,
    /// The format the table plays: Commander when any seat brought a
    /// commander, freeform otherwise, as the gateway decides it.
    pub format: FormatId,
    /// The deck this seat brought.
    pub deck: DeckList,
    /// Seconds the table gives one question, when it has a clock.
    pub decision_secs: Option<u32>,
}

/// A deck, resolved to the cards the engine plays.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeckList {
    /// The deck's name.
    pub name: String,
    /// The library, commanders not included.
    pub main: Vec<DeckCard>,
    /// The sideboard, which only a wish reaches.
    pub sideboard: Vec<DeckCard>,
    /// The commander, or both of a pair.
    pub commanders: Vec<DeckCard>,
}

/// So many copies of one card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeckCard {
    /// The card.
    pub card: CardIndex,
    /// How many copies.
    pub count: u32,
}

/// A mind that answers many requests in one call: a net on a GPU, where
/// eight positions cost about what one does.
pub trait BatchMind: Send + Sync + 'static {
    /// Answers every request, one result per request and in their order.
    /// A reply with the wrong number of results fails the whole batch.
    fn decide_batch(&self, requests: Vec<Request>) -> Deliberation<'_>;

    /// As [`Mind::disclosure`].
    fn disclosure(&self) -> Disclosure;
}

/// One queued request and where its answer goes.
type Job = (
    Request,
    tokio::sync::oneshot::Sender<Result<Answer, MindError>>,
);

/// A [`BatchMind`] seen as a [`Mind`]: requests from any number of seats
/// are queued, and every one queued while the mind was busy goes into its
/// next call, up to `max` at a time.
///
/// No timer decides when a batch is full. A request is queued the moment
/// [`Mind::decide`] is called, and the worker takes what is waiting when it
/// is free, so two seats asked at once are answered in one call and a lone
/// seat is never kept waiting for company.
pub struct Batched {
    queue: tokio::sync::mpsc::UnboundedSender<Job>,
    disclosure: Disclosure,
}

impl Batched {
    /// Starts the worker on the current tokio runtime.
    ///
    /// # Panics
    /// Outside a tokio runtime, or when `max` is zero.
    #[must_use]
    pub fn spawn<B: BatchMind>(inner: B, max: usize) -> Self {
        assert!(max > 0, "a batch holds at least one request");
        let disclosure = inner.disclosure();
        let (queue, mut jobs) = tokio::sync::mpsc::unbounded_channel::<Job>();
        tokio::spawn(async move {
            while let Some(first) = jobs.recv().await {
                let mut batch = vec![first];
                while batch.len() < max {
                    match jobs.try_recv() {
                        Ok(job) => batch.push(job),
                        Err(_) => break,
                    }
                }
                let (requests, replies): (Vec<Request>, Vec<_>) = batch.into_iter().unzip();
                let asked = requests.len();
                let mut answers = inner.decide_batch(requests).await;
                if answers.len() != asked {
                    let why = format!("the batch answered {} of {asked}", answers.len());
                    answers = vec![Err(MindError::Unavailable(why)); asked];
                }
                for (reply, answer) in replies.into_iter().zip(answers) {
                    // A seat that stopped waiting dropped its receiver; its
                    // answer has nowhere to go and needs nowhere to go.
                    let _ = reply.send(answer);
                }
            }
        });
        Self { queue, disclosure }
    }
}

impl Mind for Batched {
    fn decide(&self, request: Request) -> Thinking<'_> {
        let (reply, answer) = tokio::sync::oneshot::channel();
        // Queued here, before the future is polled, so that every seat asked
        // in one round is waiting when the worker next looks.
        let queued = self.queue.send((request, reply)).is_ok();
        Box::pin(async move {
            if !queued {
                return Err(MindError::Unavailable("the batch worker is gone".into()));
            }
            answer
                .await
                .unwrap_or_else(|_| Err(MindError::Unavailable("the batch worker is gone".into())))
        })
    }

    fn disclosure(&self) -> Disclosure {
        self.disclosure
    }
}
