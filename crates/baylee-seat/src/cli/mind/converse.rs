//! A conversation outside any game: the deck builder's (`crate::deckbuild`),
//! held by a CLI mind's process as a seat's conversation is, under the
//! same lockdown, the same budget and the same cooldown, and answered with
//! one object held to the mind's [`super::Brief`].

use super::{
    CliMind, CliSeat, Duration, GameContext, Gone, Instant, LOST, MindError, Mutex, Outcome, Reply,
    Session, Usage, Value, limited, lock, scrub,
};
use crate::mind::DeckList;
use baylee_core::ids::PlayerId;
use baylee_core::preset::FormatId;
use std::sync::Arc;

/// What the model answered to one message.
#[derive(Clone, Debug, PartialEq)]
pub struct Said {
    /// The answer object, where the reply held one.
    pub value: Option<Value>,
    /// What it wrote beside it.
    pub text: String,
    /// What the call used, where the tool said.
    pub usage: Option<Usage>,
}

impl CliMind {
    /// Sends `message` in the conversation `conversation` names, within
    /// `budget`: to the process that holds it, or, for a conversation it
    /// begins (the first message, one past its size, one whose process was
    /// lost), to a new process that hears `prefix` first, and after a loss
    /// that the earlier conversation was lost. Its process ends with the
    /// mind, or when it is idle past [`super::Limits::idle`].
    ///
    /// # Errors
    /// As a seat's question: [`MindError::Limited`] for a rate limit or a
    /// spent quota, [`MindError::Declined`] for a spent budget or no time
    /// left, [`MindError::Unavailable`] for a process that died, hung or
    /// broke its lockdown.
    pub async fn converse(
        &self,
        conversation: &str,
        prefix: &str,
        message: &str,
        budget: Duration,
    ) -> Result<Said, MindError> {
        let started = Instant::now();
        if let Some(why) = lock(&self.locked_out).clone() {
            return Err(MindError::Unavailable(why));
        }
        let context = GameContext {
            game_id: conversation.to_string(),
            seat: PlayerId::new(0),
            seats: 1,
            teams: vec![None],
            format: FormatId::Freeform,
            deck: DeckList::default(),
            decision_secs: None,
        };
        self.reap(&context);
        let seat = self.seat(&context);
        if let Some(why) = lock(&self.tally).spent_under(&self.settings) {
            return Err(MindError::Declined(why));
        }
        if let Some(left) = self.cooling() {
            return Err(MindError::Limited {
                why: format!("{} s left of the cooldown", left.as_secs().max(1)),
                lifts_in: Some(left),
            });
        }
        let (mut text, mut resumed) = self.opened(&seat, &context, prefix, message)?;
        loop {
            let left = budget.saturating_sub(started.elapsed());
            if left.is_zero() {
                return Err(MindError::Declined("no time left to ask the model".into()));
            }
            let question = {
                let mut state = lock(&seat);
                state.seat.asked += 1;
                state.seat.asked
            };
            let reply = match tokio::time::timeout(left, self.ask(&seat, &text, question)?).await {
                Ok(Ok(reply)) => reply,
                Ok(Err(_)) => Reply::Gone(Gone::Ended),
                Err(_) => {
                    Self::note_conversation(&mut lock(&seat));
                    if let Some(session) = Self::lose(&seat) {
                        session.end(Duration::ZERO);
                    }
                    return Err(MindError::Unavailable(format!(
                        "no reply within {} s",
                        left.as_secs()
                    )));
                }
            };
            Self::note_conversation(&mut lock(&seat));
            match reply {
                Reply::Gone(gone)
                    if resumed
                        && (matches!(gone, Gone::Strayed)
                            || (matches!(gone, Gone::Ended) && !Self::heard(&seat))) =>
                {
                    // The tool could not go on with it: begun again, for
                    // this very message, from the start.
                    text = self
                        .begin_again(&seat, &context, prefix, message)
                        .map_err(MindError::Unavailable)?;
                    resumed = false;
                }
                Reply::Outcome(Outcome::Answer {
                    value,
                    text: said,
                    usage,
                }) => {
                    self.cooled();
                    return Ok(Said {
                        value,
                        text: said,
                        usage,
                    });
                }
                Reply::Outcome(Outcome::RateLimited { why, lifts_in }) => {
                    self.cool(lifts_in);
                    return Err(limited(&why, lifts_in));
                }
                Reply::Outcome(Outcome::Failed(why)) => {
                    return Err(MindError::Unavailable(scrub(&why, None)));
                }
                Reply::Gone(gone) => return Err(self.gone(&seat, gone).await),
            }
        }
    }

    /// The message to send, and whether it goes to a process that resumes
    /// the conversation: a process started for the conversation where it
    /// begins (as a seat's question decides it), its first message
    /// `prefix` and then `message`.
    fn opened(
        &self,
        seat: &Arc<Mutex<CliSeat>>,
        context: &GameContext,
        prefix: &str,
        message: &str,
    ) -> Result<(String, bool), MindError> {
        let mut state = lock(seat);
        let dialect = &self.launch.dialect;
        if dialect.one_shot() {
            if let Some(done) = state.session.take() {
                done.end(self.limits.grace);
            }
            if !dialect.resumes() {
                state.lost = false;
            }
        }
        if state.session.as_mut().is_some_and(Session::died) {
            if let Some(dead) = state.session.take() {
                dead.end(Duration::ZERO);
            }
            state.lost = true;
        }
        let tokens = self.settings.conversation_tokens;
        let fresh = if dialect.resumes() {
            let idle = self.limits.idle;
            let goes_on = state.conversation.as_ref().is_some_and(|going| {
                going.kept() && going.used.elapsed() < idle && going.sent.div_ceil(3) <= tokens
            });
            if goes_on {
                state.lost = false;
            } else if let Some(over) = state.conversation.take() {
                state.lost = over.id.is_some() && (over.used.elapsed() >= idle || !over.kept());
            }
            !goes_on
        } else {
            state
                .session
                .as_ref()
                .is_none_or(|session| session.sent.div_ceil(3) > tokens)
        };
        let text = if fresh {
            first(state.lost, prefix, message)
        } else {
            message.to_string()
        };
        let mut resumed = false;
        if fresh || state.session.is_none() {
            resumed = self
                .open(&mut state, seat, context, fresh)
                .map_err(MindError::Unavailable)?;
        }
        Ok((text, resumed))
    }

    /// Begins the conversation again after its tool could not resume it:
    /// its first message again, saying the last one was lost.
    fn begin_again(
        &self,
        seat: &Arc<Mutex<CliSeat>>,
        context: &GameContext,
        prefix: &str,
        message: &str,
    ) -> Result<String, String> {
        let mut state = lock(seat);
        if let Some(dead) = state.session.take() {
            dead.end(Duration::ZERO);
        }
        state.conversation = None;
        state.lost = true;
        let text = first(true, prefix, message);
        self.open(&mut state, seat, context, true)?;
        Ok(text)
    }
}

/// A conversation's first message: that the last was lost where it was,
/// then `prefix`, then `message`.
fn first(lost: bool, prefix: &str, message: &str) -> String {
    if lost {
        format!("{LOST}\n\n{prefix}\n\n{message}")
    } else {
        format!("{prefix}\n\n{message}")
    }
}
