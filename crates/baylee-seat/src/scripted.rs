//! A mind that answers from a script, for tests and smoke runs.
//!
//! A script is a list of rules, each a function from a request to an answer
//! or to nothing; the first rule that answers wins, and a question no rule
//! answers gets the least answer the question allows ([`least_answer`]).
//! With no rules at all that is a seat that keeps its hand, passes, attacks
//! with nothing and blocks with nothing: a player who shows up and does not
//! play.
//!
//! Everything the mind is asked is written down ([`ScriptedMind::seen`]), so
//! a test can say what reached the mind and what the standing orders kept
//! from it. The mind can also be made slow ([`ScriptedMind::slow`]) or
//! unreachable ([`ScriptedMind::failing`]), which is how the bridge's budget
//! and fallback are tested without a model.

use crate::mind::{Answer, Disclosure, GameContext, Mind, MindError, Request, Thinking};
use baylee_engine::choice::{Pending, PlayerAction, default_arrangement, timeout_answer};
use baylee_view::PlayerView;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// One rule of a script: an answer to this request, or `None` to leave it
/// to the next rule.
pub type Rule = Box<dyn FnMut(&Request) -> Option<PlayerAction> + Send>;

/// What a scripted mind was asked, one line per request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seen {
    /// The request's question number.
    pub question: u64,
    /// The question's kind, as `Pending`'s variant name.
    pub kind: &'static str,
    /// The view's sequence number.
    pub seq: u64,
    /// The first log line the request handed over, and how many.
    pub log_from: u32,
    /// How many log lines the request handed over.
    pub log_lines: usize,
    /// Whether it was a retry of a refused answer.
    pub retry: bool,
    /// Whether it continued a payment.
    pub continuing: bool,
    /// What the mind answered, if it answered.
    pub answered: Option<PlayerAction>,
}

/// A mind that plays a script.
pub struct ScriptedMind {
    rules: Mutex<Vec<Rule>>,
    seen: Arc<Mutex<Vec<Seen>>>,
    slow: Option<Duration>,
    failing: Mutex<u32>,
}

impl Default for ScriptedMind {
    fn default() -> Self {
        Self::idle()
    }
}

impl ScriptedMind {
    /// A script with no rules: every question gets the least answer it
    /// allows.
    #[must_use]
    pub fn idle() -> Self {
        Self {
            rules: Mutex::new(Vec::new()),
            seen: Arc::new(Mutex::new(Vec::new())),
            slow: None,
            failing: Mutex::new(0),
        }
    }

    /// The same script with one more rule, consulted after the ones before
    /// it.
    #[must_use]
    pub fn rule(self, rule: impl FnMut(&Request) -> Option<PlayerAction> + Send + 'static) -> Self {
        self.rules
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Box::new(rule));
        self
    }

    /// Takes this long over every answer: a mind slower than its budget.
    #[must_use]
    pub const fn slow(mut self, delay: Duration) -> Self {
        self.slow = Some(delay);
        self
    }

    /// Answers the first `times` requests with [`MindError::Unavailable`].
    #[must_use]
    pub fn failing(self, times: u32) -> Self {
        *self.failing.lock().unwrap_or_else(PoisonError::into_inner) = times;
        self
    }

    /// Everything this mind has been asked, shared: it keeps filling after
    /// the mind is handed to a bridge.
    #[must_use]
    pub fn seen(&self) -> Arc<Mutex<Vec<Seen>>> {
        Arc::clone(&self.seen)
    }

    fn script(&self, request: &Request) -> PlayerAction {
        let mut rules = self.rules.lock().unwrap_or_else(PoisonError::into_inner);
        rules
            .iter_mut()
            .find_map(|rule| rule(request))
            .or_else(|| least_answer(&request.context, &request.view, &request.pending))
            .unwrap_or(PlayerAction::PassPriority)
    }
}

impl Mind for ScriptedMind {
    fn decide(&self, request: Request) -> Thinking<'_> {
        let failing = {
            let mut left = self.failing.lock().unwrap_or_else(PoisonError::into_inner);
            let failing = *left > 0;
            *left = left.saturating_sub(1);
            failing
        };
        let answer = (!failing).then(|| self.script(&request));
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Seen {
                question: request.question,
                kind: kind(&request.pending),
                seq: request.view.seq,
                log_from: request.log.from,
                log_lines: request.log.entries.len(),
                retry: request.retry.is_some(),
                continuing: request.continuing,
                answered: answer.clone(),
            });
        let slow = self.slow;
        Box::pin(async move {
            if let Some(delay) = slow {
                tokio::time::sleep(delay).await;
            }
            answer
                .map(Answer::new)
                .ok_or_else(|| MindError::Unavailable("scripted to fail".into()))
        })
    }

    fn disclosure(&self) -> Disclosure {
        Disclosure::Scripted
    }
}

/// The least answer a question allows: the answer that does nothing where
/// the engine names one (`timeout_answer`: pass, keep, attack and block with
/// nothing, decline), and otherwise the first of whatever is offered, as few
/// of them as the question takes. `None` once the game is over.
///
/// Not a strategy. It is what a test seat plays, and the bridge's last
/// answer when even the house's was refused.
#[must_use]
pub fn least_answer(
    context: &GameContext,
    view: &PlayerView,
    pending: &Pending,
) -> Option<PlayerAction> {
    if let Some(nothing) = timeout_answer(pending) {
        return Some(nothing);
    }
    let first =
        |n: usize, from: &[baylee_core::ids::ObjectId]| from.iter().copied().take(n).collect();
    let hand: Vec<_> = view.hand.iter().map(|card| card.id).collect();
    Some(match pending {
        Pending::ChooseDamageEffect {
            choice, options, ..
        } => PlayerAction::ChooseDamageEffect {
            choice: *choice,
            effect: options.first()?.id,
        },
        Pending::AllocatePrevention {
            choice,
            damage,
            total,
            ..
        } => {
            let mut remaining = *total;
            let allocation = damage
                .iter()
                .map(|part| {
                    let amount = part.amount.min(remaining);
                    remaining -= amount;
                    (part.id, amount)
                })
                .collect();
            if remaining != 0 {
                return None;
            }
            PlayerAction::AllocatePrevention {
                choice: *choice,
                allocation,
            }
        }
        Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
            PlayerAction::ChooseObjects {
                objects: first(usize::from(*count), &hand),
            }
        }
        Pending::LegendChoice { options, .. } => PlayerAction::ChooseObjects {
            objects: first(1, options),
        },
        Pending::ChooseCards { options, min, .. } => PlayerAction::ChooseObjects {
            objects: first(usize::from(*min), options),
        },
        Pending::ChooseTargets {
            options,
            player_options,
            min,
            ..
        } => {
            let minimum = usize::try_from(*min).unwrap_or(usize::MAX);
            let objects: Vec<_> = first(minimum, options);
            let players = player_options
                .iter()
                .copied()
                .take(minimum.saturating_sub(objects.len()))
                .collect();
            PlayerAction::ChooseTargets { objects, players }
        }
        Pending::ChooseSubtype { options, .. } => PlayerAction::ChooseSubtype(*options.first()?),
        Pending::ChooseColor { options, .. } => PlayerAction::ChooseColor(*options.first()?),
        Pending::ChoosePlayer { options, .. } => PlayerAction::ChoosePlayer(*options.first()?),
        Pending::ChooseCastMode { .. } | Pending::ChoosePile { .. } => PlayerAction::ChooseMode(0),
        Pending::ChooseNumber { min, .. } => PlayerAction::ChooseNumber(*min),
        Pending::ChooseCardName { .. } => {
            // A name the pool prints (CR 201.4): the seat's own first card,
            // which the pool has, or it could not have been dealt.
            let card = context
                .deck
                .main
                .first()
                .map(|entry| entry.card)
                .or_else(|| view.hand.first().map(|card| card.card.index))?;
            PlayerAction::ChooseCardName { card, face: 0 }
        }
        Pending::Arrange { cards, piles, .. } => PlayerAction::Arrange {
            piles: default_arrangement(cards, piles)?,
        },
        Pending::YesNo { .. } => PlayerAction::YesNo(false),
        // `timeout_answer` names these four today; said again rather than
        // trusted, so a change there cannot leave a seat with no answer.
        Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
        Pending::Priority { .. } => PlayerAction::PassPriority,
        Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
            attackers: Vec::new(),
        },
        Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers {
            blockers: Vec::new(),
        },
        Pending::GameOver(_) => return None,
    })
}

/// A question's kind, by its variant's name.
#[must_use]
pub const fn kind(pending: &Pending) -> &'static str {
    match pending {
        Pending::ChooseDamageEffect { .. } => "ChooseDamageEffect",
        Pending::AllocatePrevention { .. } => "AllocatePrevention",
        Pending::Mulligan { .. } => "Mulligan",
        Pending::MulliganBottom { .. } => "MulliganBottom",
        Pending::Priority { .. } => "Priority",
        Pending::ChooseAttackers { .. } => "ChooseAttackers",
        Pending::ChooseBlockers { .. } => "ChooseBlockers",
        Pending::DiscardChoice { .. } => "DiscardChoice",
        Pending::LegendChoice { .. } => "LegendChoice",
        Pending::ChooseCards { .. } => "ChooseCards",
        Pending::ChooseTargets { .. } => "ChooseTargets",
        Pending::ChooseSubtype { .. } => "ChooseSubtype",
        Pending::ChooseCardName { .. } => "ChooseCardName",
        Pending::ChooseColor { .. } => "ChooseColor",
        Pending::YesNo { .. } => "YesNo",
        Pending::ChooseCastMode { .. } => "ChooseCastMode",
        Pending::ChooseNumber { .. } => "ChooseNumber",
        Pending::ChoosePlayer { .. } => "ChoosePlayer",
        Pending::Arrange { .. } => "Arrange",
        Pending::ChoosePile { .. } => "ChoosePile",
        Pending::GameOver(_) => "GameOver",
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_core::preset::FormatId;
    use baylee_engine::choice::{ArrangePile, ArrangePlace, ArrangePrompt};

    const ME: PlayerId = PlayerId::new(0);

    /// Questions that offer nothing to choose from, or piles that cannot
    /// hold the cards: the engine asks none of them today, and each has no
    /// least answer. The seat leaves on them (`SeatCore`'s `unanswerable`)
    /// instead of waiting in silence.
    pub(crate) fn nothing_to_choose_from() -> Vec<Pending> {
        vec![
            Pending::ChooseSubtype {
                player: ME,
                options: Vec::new(),
            },
            Pending::ChooseColor {
                player: ME,
                options: Vec::new(),
            },
            Pending::ChoosePlayer {
                player: ME,
                options: Vec::new(),
            },
            Pending::Arrange {
                player: ME,
                cards: vec![ObjectId::new(1, 0), ObjectId::new(2, 0)],
                piles: vec![ArrangePile::up_to(ArrangePlace::LibraryTop, 1)],
                prompt: ArrangePrompt::Scry,
            },
        ]
    }

    #[test]
    fn a_question_with_nothing_to_choose_from_has_no_least_answer() {
        let context = GameContext {
            game_id: "test-game".into(),
            seat: ME,
            seats: 2,
            teams: vec![None, None],
            names: vec!["TEST-a".into(), "TEST-b".into()],
            format: FormatId::Freeform,
            deck: crate::DeckList::default(),
            decision_secs: None,
        };
        let view = ViewBuilder::new(2).build();
        assert!(view.hand.is_empty());
        // A card name needs a card: none in an empty deck or an empty hand.
        let mut nothing = nothing_to_choose_from();
        nothing.push(Pending::ChooseCardName { player: ME });
        for pending in &nothing {
            assert_eq!(
                least_answer(&context, &view, pending),
                None,
                "{}",
                kind(pending)
            );
        }
    }
}
