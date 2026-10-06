//! The seat: what happens between a frame from the table and an answer to
//! it, with no socket, no clock and no mind in it.
//!
//! [`SeatCore`] is fed what arrives ([`SeatCore::hear`], the bytes of one
//! frame; [`SeatCore::answered`], a mind's result; [`SeatCore::expired`], a
//! budget that ran out) and says what to do next as [`Step`]s: send this,
//! ask the mind that, stop waiting for the other. The bridge drives it over a
//! real socket and a real mind; the tests drive it over a `Session` in the
//! same process. Both run the same rules because the rules are all here.
//!
//! # A question's life
//!
//! A question the standing orders answer ([`crate::wake`]) is answered at
//! once. Any other is handed to the mind, with a budget. What the mind says
//! is held to the question ([`crate::referee`]); a foul, or a refusal by the
//! table, earns the mind [`BridgeConfig::retries`] more tries, told why. After
//! that, or when the mind fails, declines or runs out of budget, the house
//! answers ([`HouseMind`], on the view the house may see), and when even the
//! house's answer does not pass, the least answer the question allows
//! ([`least_answer`]). When nothing passes (a question that offers nothing
//! to choose from has no least answer), the seat leaves the table and says
//! which question it could not answer, rather than wait for a clock that an
//! untimed table does not have.
//!
//! # Telling questions apart
//!
//! The table sends a question again for several reasons. What tells them
//! apart is the frame's `seq`, which moves with every action the table
//! applies (anybody's) and never with a refused one:
//!
//! - the same bytes while the mind is still thinking: the question stands
//!   (another seat's keep in the opening-hand window, a setting somebody
//!   changed, moved `seq` under it). The mind goes on thinking.
//! - the same bytes at the same `seq` after an `Error`: the table refused
//!   the answer and asks again (`Session::reask`). The next author is asked.
//! - the same bytes at the same `seq` after the socket was replaced: nothing
//!   was applied, so the answer went down with the old socket. The same
//!   answer goes again; nobody is asked twice for one decision.
//! - the same bytes at the same `seq` otherwise: the same frame twice.
//! - a mulligan question already answered, at a later `seq`: the one window
//!   in which several seats are asked at once, where another seat's answer
//!   re-sends this seat's question before its own answer has landed.
//!
//! Anything else is a new question, even when its bytes and the whole view
//! match the last one: the game does come back to a seat in a state it
//! cannot tell from the one it just passed in (an opponent declining a "may"
//! changes nothing the seat sees), and that is a new decision.
//!
//! One race is left, and the client has it too: a setting another seat
//! changes (or another seat reconnecting) while this seat's answer is on the
//! wire re-sends the question at a later `seq`, and it is answered again. The
//! second answer is refused by the table when the question has moved on.

use crate::house::HouseMind;
use crate::memory::{Heard, TableMemory};
use crate::mind::{
    Answer, DeckList, Disclosure, GameContext, MindError, Refusal, RefusedBy, Request,
};
use crate::referee;
use crate::scripted::{kind, least_answer};
use crate::transcript::{Event, Note};
use crate::wake::{MIND_STOPS, Standing, Verdict, WakeFilter, Why};
use baylee_ai::AIProfile;
use baylee_client_core::automation::{RailRow, RailSide};
use baylee_core::preset::FormatId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::win::GameResult;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::PlayerView;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

/// The seconds a question gets at a `blitz` table, the fastest preset: too
/// few for a model that thinks, so the bridge refuses such a table unless
/// told otherwise ([`BridgeConfig::allow_blitz`]).
pub const BLITZ_SECS: u32 = 30;

/// How the bridge treats its mind.
#[derive(Clone, Debug)]
pub struct BridgeConfig {
    /// The longest the bridge waits for one answer, whatever the table's
    /// clock allows; on an untimed table, the only bound.
    pub think: Duration,
    /// Held back from the table's clock for the network: a mind's budget
    /// is at most what the clock has left, less this.
    pub margin: Duration,
    /// More askings after a refused answer (by the referee or the table),
    /// each told why the last one was refused.
    pub retries: u32,
    /// Answers in a row the mind fails to give (unavailable, out of budget)
    /// before the bridge takes it off the table: [`Step::MindDown`].
    pub down_after: u32,
    /// Sit at a table that gives a question [`BLITZ_SECS`] or fewer.
    pub allow_blitz: bool,
    /// The windows the standing orders stop at, when the seat could do
    /// something there ([`crate::wake::MIND_STOPS`] by default).
    pub stops: Vec<(RailSide, RailRow)>,
    /// The house the bridge falls back on.
    pub house: AIProfile,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            think: Duration::from_secs(60),
            margin: Duration::from_secs(3),
            retries: 1,
            down_after: 3,
            allow_blitz: false,
            stops: MIND_STOPS.to_vec(),
            house: AIProfile::default(),
        }
    }
}

/// Who made an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum By {
    /// The standing orders.
    Standing,
    /// The mind.
    Mind,
    /// The house, for a mind that did not answer.
    House,
    /// The least answer the question allows, when not even the house's
    /// answer passed.
    Least,
}

/// What the driver does next.
#[derive(Debug)]
pub enum Step {
    /// Send this to the table.
    Send(Envelope),
    /// Send this answer to the table.
    Answer {
        /// The question it answers.
        question: u64,
        /// The answer.
        action: PlayerAction,
        /// Who made it.
        by: By,
        /// Whether the pacing floor applies: a woken decision answered, by
        /// the mind or for it. Never a standing answer or a payment's next
        /// step.
        paced: bool,
        /// The answer, encoded for the table.
        envelope: Envelope,
    },
    /// Ask the mind, and wait at most `request.budget`.
    Ask(Box<Request>),
    /// Stop waiting for the mind on this question: it is gone.
    Cancel(u64),
    /// The mind failed [`BridgeConfig::down_after`] times in a row. The
    /// bridge leaves the socket, so the table sees an absent player and its
    /// house holds the chair as away, until the mind is ready and the bridge
    /// comes back ([`SeatCore::resumed`]).
    MindDown,
    /// Leave the table for good, for this reason.
    Leave(String),
    /// The game is over.
    Over(GameResult),
}

/// How a seat's game went, counted.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Stats {
    /// Questions asked of this seat (a question asked again counts once).
    pub questions: u32,
    /// Answered by the standing orders, by which order.
    pub standing: StandingCounts,
    /// Questions the mind was woken for.
    pub wakes: u32,
    /// The same, by why.
    pub woken: WokenCounts,
    /// Next steps of a payment the mind began.
    pub continuations: u32,
    /// Questions while a plan of the mind's ran, which its plan answered or
    /// stopped at.
    pub planned: u32,
    /// Every time the mind was asked, retries included.
    pub asked: u32,
    /// Askings that were retries of a refused answer.
    pub retries: u32,
    /// Answers the referee would not send.
    pub refused_by_referee: u32,
    /// Answers the table refused.
    pub refused_by_table: u32,
    /// Answers sent, by who made them.
    pub answered: AnsweredCounts,
    /// Why the house answered instead of the mind.
    pub fallbacks: FallbackCounts,
    /// Questions nothing could answer; the seat left at the first.
    pub unanswerable: u32,
    /// Mind answers that came after their question was gone.
    pub late: u32,
    /// The model time the mind reported, in milliseconds.
    pub model_ms: u64,
    /// Times the mind was taken off the table.
    pub mind_down: u32,
    /// The last turn this seat saw.
    pub turns: u32,
    /// How the game ended for this seat, once it has.
    pub outcome: Option<Outcome>,
}

/// Standing answers, by order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct StandingCounts {
    /// Nothing to do, even by tapping.
    pub nothing_to_do: u32,
    /// A window the rail passes.
    pub quiet_window: u32,
    /// An attack with nothing that can attack.
    pub no_attackers: u32,
    /// A block with nothing that can block.
    pub no_blockers: u32,
}

impl StandingCounts {
    /// All of them.
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.nothing_to_do + self.quiet_window + self.no_attackers + self.no_blockers
    }

    const fn count(&mut self, why: Standing) {
        match why {
            Standing::NothingToDo => self.nothing_to_do += 1,
            Standing::QuietWindow => self.quiet_window += 1,
            Standing::NoAttackers => self.no_attackers += 1,
            Standing::NoBlockers => self.no_blockers += 1,
        }
    }
}

/// Wakes, by why.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct WokenCounts {
    /// A question the orders never answer.
    pub question: u32,
    /// A declaration with something in it.
    pub declaration: u32,
    /// A window the rail stops at.
    pub rail_stop: u32,
    /// The other side's stack.
    pub opposing_stack: u32,
    /// A cleanup window.
    pub cleanup: u32,
    /// A payment owed.
    pub owed: u32,
    /// Not this seat's question.
    pub not_asked: u32,
}

impl WokenCounts {
    const fn count(&mut self, why: Why) {
        match why {
            Why::Question => self.question += 1,
            Why::Declaration => self.declaration += 1,
            Why::RailStop => self.rail_stop += 1,
            Why::OpposingStack => self.opposing_stack += 1,
            Why::Cleanup => self.cleanup += 1,
            Why::Owed => self.owed += 1,
            Why::NotAsked => self.not_asked += 1,
        }
    }
}

/// Answers sent, by author.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct AnsweredCounts {
    /// By the standing orders.
    pub standing: u32,
    /// By the mind.
    pub mind: u32,
    /// By the house.
    pub house: u32,
    /// The least answer.
    pub least: u32,
}

/// Why the house answered.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct FallbackCounts {
    /// The budget ran out.
    pub expired: u32,
    /// The mind could not answer.
    pub unavailable: u32,
    /// The mind would not answer this kind of question.
    pub declined: u32,
    /// The mind's answers were refused as often as it may be asked.
    pub refused: u32,
    /// The table's clock left no time to ask.
    pub no_time: u32,
    /// The mind was down.
    pub down: u32,
}

impl FallbackCounts {
    /// All of them.
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.expired + self.unavailable + self.declined + self.refused + self.no_time + self.down
    }
}

/// How a game ended for one seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// This seat, or its side, won.
    Won,
    /// Somebody else won.
    Lost,
    /// Nobody won.
    Draw,
}

/// Where the seat's current question stands.
#[derive(Clone, Debug)]
enum State {
    /// The mind is thinking.
    Thinking,
    /// An answer went to the table.
    Sent { action: PlayerAction, by: By },
    /// Nothing passed; the seat left.
    Unanswerable,
}

/// The question the seat owes or has just answered.
#[derive(Clone, Debug)]
struct Current {
    question: u64,
    seq: u64,
    raw: Vec<u8>,
    pending: Pending,
    view: PlayerView,
    /// Times the mind has been asked this question.
    tries: u32,
    /// Whether it woke the mind (for the pacing floor and the counts),
    /// and why.
    wake: bool,
    why: Option<Why>,
    /// Whether it is a payment's next step.
    continuing: bool,
    /// Whether the socket was replaced since the answer went out.
    resumed: bool,
    state: State,
}

/// Whether the seat is still at the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Life {
    Playing,
    /// The game ended.
    Over,
    /// The bridge left, and hears nothing more.
    Left,
}

/// The seat, without its socket or its mind.
pub struct SeatCore {
    config: BridgeConfig,
    deck: DeckList,
    disclosure: Disclosure,
    house: HouseMind,
    memory: TableMemory,
    wake: WakeFilter,
    context: Option<Arc<GameContext>>,
    current: Option<Current>,
    /// What the table said after the last answer went out, if it refused
    /// something.
    refused: Option<String>,
    questions: u64,
    ready_sent: bool,
    checked: bool,
    down: bool,
    failures: u32,
    life: Life,
    stats: Stats,
    notes: Vec<Note>,
}

impl SeatCore {
    /// A seat that will play `deck` and must be called as `disclosure` says.
    #[must_use]
    pub fn new(config: BridgeConfig, deck: DeckList, disclosure: Disclosure) -> Self {
        Self {
            house: HouseMind::new(config.house),
            wake: WakeFilter::new(&config.stops),
            config,
            deck,
            disclosure,
            memory: TableMemory::default(),
            context: None,
            current: None,
            refused: None,
            questions: 0,
            ready_sent: false,
            checked: false,
            down: false,
            failures: 0,
            life: Life::Playing,
            stats: Stats::default(),
            notes: Vec::new(),
        }
    }

    /// Reads one frame from the table.
    pub fn hear(&mut self, bytes: &[u8]) -> Vec<Step> {
        if self.life == Life::Left {
            return Vec::new();
        }
        match self.memory.hear(bytes) {
            Heard::Statics | Heard::View => self.settle(),
            Heard::Question { seq, pending, raw } => self.question(seq, *pending, raw),
            Heard::Refused(message) => {
                if matches!(
                    self.current.as_ref().map(|c| &c.state),
                    Some(State::Sent { .. })
                ) {
                    self.refused = Some(message.clone());
                }
                self.note(0, Event::TableSaid { message });
                Vec::new()
            }
            Heard::Incompatible(why) => self.leave(why),
            Heard::Unreadable(why) => {
                self.note(0, Event::TableSaid { message: why });
                Vec::new()
            }
            Heard::Curtain | Heard::ClockProbe { .. } | Heard::Other => Vec::new(),
        }
    }

    /// What the mind made of a question.
    pub fn answered(&mut self, question: u64, result: Result<Answer, MindError>) -> Vec<Step> {
        if !self.thinking_about(question) {
            self.stats.late += 1;
            self.note(question, Event::Late);
            return Vec::new();
        }
        match result {
            Ok(answer) => {
                self.failures = 0;
                if let (Some(current), Some(context)) = (self.current.as_ref(), &self.context) {
                    self.wake.apply_orders(
                        answer.stops.as_deref(),
                        answer.orders,
                        answer.planned,
                        &current.view,
                        &context.teams,
                    );
                }
                self.stats.model_ms = self.stats.model_ms.saturating_add(
                    u64::try_from(answer.model_time.as_millis()).unwrap_or(u64::MAX),
                );
                let foul = self
                    .current
                    .as_ref()
                    .and_then(|current| referee::check(&current.pending, &answer.action).err());
                match foul {
                    None => self.send(answer.action.clone(), By::Mind, Some(&answer)),
                    Some(foul) => {
                        self.stats.refused_by_referee += 1;
                        let reason = foul.reason().to_string();
                        self.note(
                            question,
                            Event::Refused {
                                by: RefusedBy::Referee,
                                action: answer.action.clone(),
                                reason: reason.clone(),
                            },
                        );
                        self.mind_refused(Refusal {
                            answer: answer.action,
                            reason,
                            by: RefusedBy::Referee,
                        })
                    }
                }
            }
            Err(error) => {
                self.note(
                    question,
                    Event::MindFailed {
                        error: error.to_string(),
                    },
                );
                match error {
                    MindError::Declined(_) => {
                        self.stats.fallbacks.declined += 1;
                        self.fall_back(By::House)
                    }
                    MindError::Unavailable(_) => {
                        self.stats.fallbacks.unavailable += 1;
                        self.failed()
                    }
                }
            }
        }
    }

    /// The mind's budget for `question` ran out.
    pub fn expired(&mut self, question: u64) -> Vec<Step> {
        if !self.thinking_about(question) {
            return Vec::new();
        }
        self.note(question, Event::Expired);
        self.stats.fallbacks.expired += 1;
        self.failed()
    }

    /// A new socket replaced the old one: the table is asked for what the
    /// seat missed, the seat says it is ready again once it has its table,
    /// and a mind that was down is on again.
    pub fn resumed(&mut self) -> Vec<Step> {
        self.ready_sent = false;
        self.down = false;
        self.failures = 0;
        self.refused = None;
        // Only an answer already sent can have gone down with the old
        // socket; one still being thought about goes out on the new one.
        if let Some(current) = self
            .current
            .as_mut()
            .filter(|c| matches!(c.state, State::Sent { .. }))
        {
            current.resumed = true;
        }
        self.context
            .as_ref()
            .map(|context| {
                Step::Send(Envelope {
                    msg: Some(v1::envelope::Msg::Resume(v1::ResumeGame {
                        game_id: context.game_id.clone(),
                        // The socket is bound to the seat; the token proves
                        // nothing more here and would only travel further.
                        seat_token: String::new(),
                        last_seq: self.memory.last_seq(),
                    })),
                })
            })
            .into_iter()
            .collect()
    }

    /// The notes written since the last call, oldest first.
    pub fn take_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    /// The counts so far.
    #[must_use]
    pub const fn stats(&self) -> &Stats {
        &self.stats
    }

    /// What the seat knew before the first card, once it has its table.
    #[must_use]
    pub const fn context(&self) -> Option<&Arc<GameContext>> {
        self.context.as_ref()
    }

    /// What the chair must be called: the kind of mind the seat plays for.
    #[must_use]
    pub const fn disclosure(&self) -> Disclosure {
        self.disclosure
    }

    /// The seat's memory of the table.
    #[must_use]
    pub const fn memory(&self) -> &TableMemory {
        &self.memory
    }

    /// The question the mind is thinking about, if any.
    #[must_use]
    pub fn thinking(&self) -> Option<u64> {
        self.current
            .as_ref()
            .filter(|c| matches!(c.state, State::Thinking))
            .map(|c| c.question)
    }

    /// The room closed with no result for this seat: the table could not
    /// start (a seat never finished loading), or its engine was lost. The
    /// seat's socket hears neither: the room only leaves the lobby, which is
    /// where the bridge learns it. What the mind is thinking about is
    /// dropped, and nothing more is heard.
    pub fn closed(&mut self) -> Vec<Step> {
        if self.life != Life::Playing {
            return Vec::new();
        }
        self.life = Life::Over;
        let steps = self.cancel_thinking();
        self.note(0, Event::Closed);
        steps
    }

    /// Whether the game is over for this seat.
    #[must_use]
    pub const fn is_over(&self) -> bool {
        matches!(self.life, Life::Over)
    }

    fn thinking_about(&self, question: u64) -> bool {
        self.thinking() == Some(question) && self.life == Life::Playing
    }

    /// After the table or a view arrived: learn the table once, check that
    /// this chair may sit at it, and say the seat is ready.
    fn settle(&mut self) -> Vec<Step> {
        let (Some(statics), Some(view)) = (self.memory.statics(), self.memory.view()) else {
            return Vec::new();
        };
        if !self.checked {
            self.checked = true;
            let name = statics
                .seats
                .get(usize::from(statics.your_seat.get()))
                .map_or("", |seat| seat.display_name.as_str());
            if !self.disclosure.names(name) {
                let why = format!(
                    "this chair is called «{name}», and a chair a mind plays must be called {}…",
                    self.disclosure.prefix()
                );
                return self.leave(why);
            }
            if let Some(secs) = statics.decision_secs.filter(|s| *s <= BLITZ_SECS)
                && !self.config.allow_blitz
            {
                return self.leave(format!(
                    "the table gives {secs} s a question, too few for a mind that thinks \
                     (allow it with --allow-blitz)"
                ));
            }
            let commander = view.seats.iter().any(|seat| !seat.commanders.is_empty());
            self.context = Some(Arc::new(GameContext {
                game_id: statics.game_id.clone(),
                seat: statics.your_seat,
                seats: u8::try_from(statics.seats.len()).unwrap_or(u8::MAX),
                teams: statics.seats.iter().map(|seat| seat.team).collect(),
                names: statics
                    .seats
                    .iter()
                    .map(|seat| seat.display_name.clone())
                    .collect(),
                format: if commander {
                    FormatId::Commander
                } else {
                    FormatId::Freeform
                },
                deck: self.deck.clone(),
                decision_secs: statics.decision_secs,
            }));
        }
        self.stats.turns = self.stats.turns.max(view.turn);
        if self.ready_sent {
            return Vec::new();
        }
        self.ready_sent = true;
        vec![Step::Send(Envelope {
            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
        })]
    }

    fn leave(&mut self, reason: String) -> Vec<Step> {
        self.life = Life::Left;
        let mut steps = self.cancel_thinking();
        self.note(
            0,
            Event::Left {
                reason: reason.clone(),
            },
        );
        steps.push(Step::Leave(reason));
        steps
    }

    fn cancel_thinking(&mut self) -> Vec<Step> {
        self.thinking().map(Step::Cancel).into_iter().collect()
    }

    fn question(&mut self, seq: u64, pending: Pending, raw: Vec<u8>) -> Vec<Step> {
        if self.life != Life::Playing {
            return Vec::new();
        }
        if let Pending::GameOver(result) = pending {
            return self.game_over(result);
        }
        let (Some(context), Some(view)) = (self.context.clone(), self.memory.view().cloned())
        else {
            self.note(
                0,
                Event::TableSaid {
                    message: "a question before the table was set; left to the clock".into(),
                },
            );
            return Vec::new();
        };
        if pending.asked() != Some(context.seat) {
            return Vec::new();
        }
        if let Some(steps) = self.again(seq, &pending, &raw) {
            return steps;
        }
        let withdrawn = self.thinking();
        let mut steps: Vec<Step> = withdrawn.map(Step::Cancel).into_iter().collect();
        if let Some(gone) = withdrawn {
            self.note(gone, Event::Withdrawn);
        }
        self.refused = None;
        self.questions += 1;
        self.stats.questions += 1;
        let verdict = self.wake.judge(&view, &pending, &context.teams);
        self.current = Some(Current {
            question: self.questions,
            seq,
            raw,
            pending,
            view,
            tries: 0,
            wake: false,
            why: None,
            continuing: verdict == Verdict::Continue,
            resumed: false,
            state: State::Thinking,
        });
        if self
            .current
            .as_ref()
            .is_some_and(|current| referee::offers_nothing(&current.pending))
        {
            steps.extend(self.unanswerable());
            return steps;
        }
        steps.extend(match verdict {
            Verdict::Standing { action, why } => self.standing(action, why),
            Verdict::Wake(why) => self.wake_mind(why),
            Verdict::Continue => {
                self.stats.continuations += 1;
                self.ask(None)
            }
            Verdict::Planned => {
                self.stats.planned += 1;
                self.ask(None)
            }
        });
        steps
    }

    /// A question whose bytes are the current one's: what the steps are, or
    /// `None` when it is a new question after all (module docs).
    fn again(&mut self, seq: u64, pending: &Pending, raw: &[u8]) -> Option<Vec<Step>> {
        let current = self.current.as_mut().filter(|c| c.raw == raw)?;
        let question = current.question;
        let State::Sent { action, by } = current.state.clone() else {
            // Still thinking, or given up on: the question stands, as the
            // table sent it last (a refusal is asked again at that `seq`).
            current.seq = seq;
            return Some(Vec::new());
        };
        if current.seq == seq {
            // Nothing was applied in between: every action the table takes
            // moves `seq`, and a refused one does not.
            if let Some(reason) = self.refused.take() {
                return Some(self.table_refused(action, by, reason));
            }
            if !current.resumed {
                return Some(Vec::new());
            }
            // Asked again on a new socket, with nothing applied since: the
            // answer went down with the old one.
            current.resumed = false;
            self.note(question, Event::Resent);
            return Some(vec![Step::Answer {
                question,
                envelope: action_envelope(self.context.as_deref(), &action),
                action,
                by,
                paced: false,
            }]);
        }
        // Several seats are asked at once only in the opening-hand window,
        // where another seat's answer moves `seq` under a question this seat
        // has answered; and once answered, it is never asked the same again
        // (a mulligan taken changes the count, and the bottom is chosen once).
        if matches!(
            pending,
            Pending::Mulligan { .. } | Pending::MulliganBottom { .. }
        ) {
            return Some(Vec::new());
        }
        None
    }

    fn standing(&mut self, action: PlayerAction, why: Standing) -> Vec<Step> {
        let pending = self.current.as_ref().map(|c| c.pending.clone());
        if let Some(foul) = pending.and_then(|p| referee::check(&p, &action).err()) {
            // The orders were wrong about this question; the mind decides.
            self.stats.refused_by_referee += 1;
            let question = self.questions;
            self.note(
                question,
                Event::Refused {
                    by: RefusedBy::Referee,
                    action,
                    reason: foul.reason().to_string(),
                },
            );
            return self.wake_mind(Why::Question);
        }
        self.stats.standing.count(why);
        let kind = self.current.as_ref().map_or("", |c| kind(&c.pending));
        self.note(
            self.questions,
            Event::Standing {
                kind,
                why,
                action: action.clone(),
            },
        );
        self.send(action, By::Standing, None)
    }

    fn wake_mind(&mut self, why: Why) -> Vec<Step> {
        self.stats.wakes += 1;
        self.stats.woken.count(why);
        if let Some(current) = self.current.as_mut() {
            current.wake = true;
            current.why = Some(why);
        }
        self.ask(None)
    }

    /// Hands the current question to the mind.
    fn ask(&mut self, retry: Option<Refusal>) -> Vec<Step> {
        if self.down {
            self.stats.fallbacks.down += 1;
            return self.fall_back(By::House);
        }
        let Some(context) = self.context.clone() else {
            return Vec::new();
        };
        let Some(view) = self.current.as_ref().map(|c| c.view.clone()) else {
            return Vec::new();
        };
        let budget = self.budget(&view);
        if budget.is_zero() {
            self.stats.fallbacks.no_time += 1;
            return self.fall_back(By::House);
        }
        let log = self.memory.hand_over_log();
        let Some(current) = self.current.as_mut() else {
            return Vec::new();
        };
        current.tries += 1;
        current.state = State::Thinking;
        let why = current.why;
        let request = Request {
            context,
            question: current.question,
            view,
            pending: current.pending.clone(),
            log,
            budget,
            retry,
            continuing: current.continuing,
            held: self.wake.take_held(),
        };
        self.stats.asked += 1;
        if request.retry.is_some() {
            self.stats.retries += 1;
        }
        self.note(
            request.question,
            Event::Asked {
                kind: kind(&request.pending),
                why,
                step: request.view.step,
                their_turn: request.view.active != request.context.seat,
                continuing: request.continuing,
                retry: request.retry.is_some(),
                budget_ms: u64::try_from(budget.as_millis()).unwrap_or(u64::MAX),
                log_lines: request.log.entries.len(),
            },
        );
        vec![Step::Ask(Box::new(request))]
    }

    /// How long the mind may take over a question asked in `view`.
    fn budget(&self, view: &PlayerView) -> Duration {
        view.decision_remaining_ms.map_or(self.config.think, |ms| {
            let left = Duration::from_millis(u64::from(ms)).saturating_sub(self.config.margin);
            left.min(self.config.think)
        })
    }

    /// The mind did not answer (unavailable, or out of budget): the house
    /// answers, and a run of these takes the mind off the table.
    fn failed(&mut self) -> Vec<Step> {
        self.failures += 1;
        let mut steps = self.fall_back(By::House);
        if self.failures >= self.config.down_after && !self.down {
            self.down = true;
            self.stats.mind_down += 1;
            self.note(self.questions, Event::MindDown);
            steps.push(Step::MindDown);
        }
        steps
    }

    /// The mind's answer was refused: another try, told why, or the house.
    fn mind_refused(&mut self, refusal: Refusal) -> Vec<Step> {
        let tries = self.current.as_ref().map_or(u32::MAX, |c| c.tries);
        if tries <= self.config.retries {
            return self.ask(Some(refusal));
        }
        self.stats.fallbacks.refused += 1;
        self.fall_back(By::House)
    }

    /// The table refused the answer `by` made and asked again.
    fn table_refused(&mut self, action: PlayerAction, by: By, reason: String) -> Vec<Step> {
        self.stats.refused_by_table += 1;
        let question = self.questions;
        self.note(
            question,
            Event::Refused {
                by: RefusedBy::Table,
                action: action.clone(),
                reason: reason.clone(),
            },
        );
        // The question came again with the view it stands in now.
        if let (Some(current), Some(view)) = (self.current.as_mut(), self.memory.view()) {
            current.view = view.clone();
            current.state = State::Thinking;
        }
        match by {
            By::Standing => self.wake_mind(Why::Question),
            By::Mind => self.mind_refused(Refusal {
                answer: action,
                reason,
                by: RefusedBy::Table,
            }),
            By::House => self.fall_back(By::Least),
            By::Least => self.unanswerable(),
        }
    }

    /// Sends the first answer, from `from` down the fallback order (the
    /// house, then the least answer), that the referee passes.
    fn fall_back(&mut self, from: By) -> Vec<Step> {
        let (Some(context), Some(current)) = (self.context.clone(), self.current.clone()) else {
            return Vec::new();
        };
        let authors: &[By] = if from == By::Least {
            &[By::Least]
        } else {
            &[By::House, By::Least]
        };
        for &by in authors {
            let action = match by {
                By::House => Some(self.house.answer(&context, &current.view, &current.pending)),
                _ => least_answer(&context, &current.view, &current.pending),
            };
            let Some(action) = action else {
                continue;
            };
            match referee::check(&current.pending, &action) {
                Ok(()) => return self.send(action, by, None),
                Err(foul) => {
                    self.stats.refused_by_referee += 1;
                    self.note(
                        current.question,
                        Event::Refused {
                            by: RefusedBy::Referee,
                            action,
                            reason: foul.reason().to_string(),
                        },
                    );
                }
            }
        }
        self.unanswerable()
    }

    /// Nothing the seat could send passes: not the mind's answer, not the
    /// house's, not the least one (a question that offers nothing to choose
    /// from has none). Waiting for the table's clock would hang an untimed
    /// table in silence, so the seat leaves, saying which question it could
    /// not answer: the table sees an absent player, and whoever runs the
    /// bridge sees why.
    fn unanswerable(&mut self) -> Vec<Step> {
        self.stats.unanswerable += 1;
        let asked = self
            .current
            .as_ref()
            .map_or("unknown", |current| crate::scripted::kind(&current.pending));
        if let Some(current) = self.current.as_mut() {
            current.state = State::Unanswerable;
        }
        self.note(self.questions, Event::Unanswerable);
        self.leave(format!(
            "nothing this seat could send answers the {asked} question it was asked"
        ))
    }

    /// Sends an answer the referee has passed.
    fn send(&mut self, action: PlayerAction, by: By, answer: Option<&Answer>) -> Vec<Step> {
        let Some(current) = self.current.as_mut() else {
            return Vec::new();
        };
        self.wake.heard(&current.view, &action);
        current.state = State::Sent {
            action: action.clone(),
            by,
        };
        let question = current.question;
        let paced = current.wake && by != By::Standing;
        self.refused = None;
        match by {
            By::Standing => self.stats.answered.standing += 1,
            By::Mind => self.stats.answered.mind += 1,
            By::House => self.stats.answered.house += 1,
            By::Least => self.stats.answered.least += 1,
        }
        let mut steps = Vec::new();
        // A plan's next tap is the mind's answer too, made with no model
        // time and nothing new to say: only what the model thought about
        // goes into the AI log.
        if by == By::Mind
            && let Some(answer) = answer.filter(|a| !a.model_time.is_zero())
            && let Some(said) = Self::reasoning(answer.note.as_deref(), answer.thinking.as_deref())
        {
            steps.push(said);
        }
        if by != By::Standing {
            self.note(
                question,
                Event::Answered {
                    by,
                    action: action.clone(),
                    model_ms: answer
                        .map(|a| u64::try_from(a.model_time.as_millis()).unwrap_or(u64::MAX)),
                    note: answer.and_then(|a| a.note.clone()),
                    thinking: answer.and_then(|a| a.thinking.clone()),
                },
            );
        }
        steps.push(Step::Answer {
            question,
            envelope: action_envelope(self.context.as_deref(), &action),
            action,
            by,
            paced,
        });
        steps
    }

    fn game_over(&mut self, result: GameResult) -> Vec<Step> {
        self.life = Life::Over;
        let mut steps = self.cancel_thinking();
        if let Some(context) = self.context.as_ref() {
            let team = context
                .teams
                .get(usize::from(context.seat.get()))
                .copied()
                .flatten();
            self.stats.outcome = Some(match result.winner {
                None => Outcome::Draw,
                Some(victor) if victor.includes(context.seat, team) => Outcome::Won,
                Some(_) => Outcome::Lost,
            });
        }
        self.note(
            0,
            Event::Over {
                result: format!("{result:?}"),
            },
        );
        steps.push(Step::Over(result));
        steps
    }

    fn note(&mut self, question: u64, event: Event) {
        let view = self.memory.view();
        self.notes.push(Note {
            question,
            seq: self.memory.last_seq(),
            turn: view.map_or(0, |v| v.turn),
            event,
        });
    }

    /// What the mind said beside an answer (`v1::AiLog`), in a debug build;
    /// `None` in a release build, and when the mind said nothing.
    ///
    /// The AI log is a debugging and sparring tool, open to the whole table
    /// and only in debug builds (`docs/protocol.md` §"An AI seat's
    /// reasoning"): a model's reasoning reads its whole view out loud, its
    /// hand included, so a release bridge never sends it, and a release
    /// engine drops one from a debug bridge. Each one counts against the
    /// socket's rate like any frame, and each of its texts is cut at a
    /// character boundary to [`AI_LOG_BYTES`], so it never nears the
    /// gateway's bound on a seat's frame.
    fn reasoning(note: Option<&str>, thinking: Option<&str>) -> Option<Step> {
        if !cfg!(debug_assertions) || (note.is_none() && thinking.is_none()) {
            return None;
        }
        Some(Step::Send(Envelope {
            msg: Some(v1::envelope::Msg::AiLog(v1::AiLog {
                seat: 0,
                note: cut(note.unwrap_or_default(), AI_LOG_BYTES).to_owned(),
                thinking: cut(thinking.unwrap_or_default(), AI_LOG_BYTES).to_owned(),
            })),
        }))
    }
}

/// The most bytes of each text an `AiLog` carries: two of them stay well
/// inside the gateway's 64 KiB bound on a seat's frame.
const AI_LOG_BYTES: usize = 16 * 1024;

/// `text`, cut to at most `max` bytes at a character boundary.
fn cut(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let end = (0..=max)
        .rev()
        .find(|at| text.is_char_boundary(*at))
        .unwrap_or(0);
    &text[..end]
}

/// An answer, as the table reads it.
fn action_envelope(context: Option<&GameContext>, action: &PlayerAction) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: context.map(|c| c.game_id.clone()).unwrap_or_default(),
            // The socket is bound to one seat of one game already.
            seat_token: String::new(),
            action_json: serde_json::to_vec(action).unwrap_or_default(),
        })),
    }
}

#[cfg(test)]
mod tests;
