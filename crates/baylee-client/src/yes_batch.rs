//! One explicit yes for the finite run of identical abilities already stacked.
use baylee_client_core::decision::resource_player;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::{Pending, PlayerAction, YesNoPrompt};
use baylee_view::{PlayerView, StackItem};

/// Stack objects approved by the player. Newly created triggers never join.
#[derive(Default)]
pub struct YesBatch {
    remaining: Vec<ObjectId>,
    answered: Option<ObjectId>,
    sent_at: Option<u64>,
}

/// Consecutive identical abilities, starting with the current may decision.
///
/// The deciding player's: this seat's own, or those of the player it
/// controls while it answers that player's "may" (CR 724.5).
pub fn candidates(view: &PlayerView, pending: &Pending) -> Vec<ObjectId> {
    let deciding = resource_player(view);
    if !matches!(pending, Pending::YesNo {
        player, prompt: YesNoPrompt::MayDo, ..
    } if *player == deciding)
    {
        return Vec::new();
    }
    let Some(top) = view.stack.last().filter(|o| {
        o.controller == deciding && matches!(o.stack_item, Some(StackItem::Ability { .. }))
    }) else {
        return Vec::new();
    };
    view.stack
        .iter()
        .rev()
        .take_while(|o| {
            o.controller == top.controller && same_ability(o.stack_item, top.stack_item)
        })
        .map(|o| o.id)
        .collect()
}

fn same_ability(a: Option<StackItem>, b: Option<StackItem>) -> bool {
    a == b
        || matches!((a, b), (
        Some(StackItem::Ability { rules: Some(ar), text: Some(at), .. }),
        Some(StackItem::Ability { rules: Some(br), text: Some(bt), .. })
    ) if ar == br && at == bt)
}

impl YesBatch {
    /// Capture only the already visible series, never a standing order.
    pub fn begin(view: &PlayerView, pending: &Pending) -> Option<Self> {
        let remaining = candidates(view, pending);
        (remaining.len() > 1).then_some(Self {
            remaining,
            ..Self::default()
        })
    }

    /// Whether no object is approved: [`Self::answer`] then answers nothing
    /// and changes nothing that matters (an empty list cleared).
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.remaining.is_empty()
    }

    /// Answer once per snapshot and stop at any unrelated decision.
    ///
    /// Only for the player whose abilities were approved. While this seat
    /// controls another player, its own priority in between (CR 724.8) is a
    /// decision the approval never covered, and it is left to the player.
    pub fn answer(&mut self, view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
        if self.sent_at == Some(view.seq) {
            return None;
        }
        let top = view.stack.last()?;
        let (top, approved_for) = (top.id, top.controller);
        if self.answered == Some(top) {
            return None;
        }
        if !self.remaining.contains(&top) {
            self.remaining.clear();
            return None;
        }
        let deciding = resource_player(view);
        let ours =
            |player: &baylee_core::ids::PlayerId| *player == deciding && deciding == approved_for;
        let answer = match pending {
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } if ours(player) => {
                self.remaining.retain(|id| *id != top);
                self.answered = Some(top);
                PlayerAction::YesNo(true)
            }
            // A payment window is a priority this seat owes mana in: passing
            // it refuses the payment (#01), so the batch stops there.
            Pending::Priority { player, .. }
                if ours(player) && view.owed.is_some() && view.awaiting == Some(view.seat) =>
            {
                self.remaining.clear();
                return None;
            }
            Pending::Priority { player, .. } if ours(player) => PlayerAction::PassPriority,
            Pending::Priority { .. } => return None,
            _ => {
                self.remaining.clear();
                return None;
            }
        };
        self.sent_at = Some(view.seq);
        Some(answer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::PlayerId;

    #[test]
    fn batch_is_finite_does_not_repeat_and_never_answers_a_price() {
        let mut view = ViewBuilder::new(2).build();
        for id in 1..=101 {
            let mut o = token(id, 0, "Ondu Cleric", 1, 1);
            o.stack_item = Some(StackItem::Ability {
                token: None,
                source: ObjectId::new(500 + id, 0),
                ability: None,
                rules: Some(baylee_view::RulesFace {
                    card: baylee_core::ids::CardIndex::new(12),
                    face: 0,
                }),
                text: Some(baylee_view::StackText {
                    face: 0,
                    line: 0,
                    of: 1,
                }),
            });
            view.stack.push(o);
        }
        let yes = Pending::YesNo {
            player: PlayerId::new(0),
            prompt: YesNoPrompt::MayDo,
            source: None,
        };
        let mut batch = YesBatch::begin(&view, &yes).unwrap();
        assert_eq!(candidates(&view, &yes).len(), 101);
        assert_eq!(batch.answer(&view, &yes), Some(PlayerAction::YesNo(true)));
        view.seq += 1;
        assert_eq!(
            batch.answer(&view, &yes),
            None,
            "a second may in the same ability is not approved"
        );
        view.stack.pop();
        let priority = Pending::Priority {
            player: view.seat,
            legal: Box::default(),
        };
        assert_eq!(
            batch.answer(&view, &priority),
            Some(PlayerAction::PassPriority)
        );
        assert_eq!(
            batch.answer(&view, &yes),
            None,
            "never send twice for one state"
        );
        view.seq += 1;
        assert_eq!(batch.answer(&view, &yes), Some(PlayerAction::YesNo(true)));
        view.stack.pop();
        view.seq += 1;
        let price = Pending::YesNo {
            player: view.seat,
            prompt: YesNoPrompt::PayLifeOrEnterTapped { amount: 2 },
            source: None,
        };
        assert_eq!(batch.answer(&view, &price), None);
        assert_eq!(
            batch.answer(&view, &yes),
            None,
            "an intervening decision cancels the batch"
        );
        let mut batch = YesBatch::begin(&view, &yes).unwrap();
        let mut new = view.stack.last().unwrap().clone();
        new.id = ObjectId::new(999, 0);
        view.stack.push(new);
        assert_eq!(
            batch.answer(&view, &yes),
            None,
            "future triggers are not approved"
        );
    }

    /// A seat with two identical abilities stacked, a batch begun over the
    /// first "may" and answered once.
    fn a_batch_answered_once() -> (YesBatch, PlayerView) {
        let mut view = ViewBuilder::new(2).build();
        for id in 1..=2 {
            let mut o = token(id, 0, "Ondu Cleric", 1, 1);
            o.stack_item = Some(StackItem::Ability {
                token: None,
                source: ObjectId::new(500 + id, 0),
                ability: None,
                rules: Some(baylee_view::RulesFace {
                    card: baylee_core::ids::CardIndex::new(12),
                    face: 0,
                }),
                text: Some(baylee_view::StackText {
                    face: 0,
                    line: 0,
                    of: 1,
                }),
            });
            view.stack.push(o);
        }
        let yes = Pending::YesNo {
            player: view.seat,
            prompt: YesNoPrompt::MayDo,
            source: None,
        };
        let mut batch = YesBatch::begin(&view, &yes).unwrap();
        assert_eq!(batch.answer(&view, &yes), Some(PlayerAction::YesNo(true)));
        view.stack.pop();
        view.seq += 1;
        (batch, view)
    }

    /// Report 01: the yes-to-all batch passes its seat's priority between
    /// answers, but not a payment window, which is a priority this seat owes
    /// mana in and which a pass would refuse. Before the fix it sent
    /// `PassPriority` there too. The control is the same window owing nothing.
    #[test]
    fn the_batch_stops_at_a_payment_window_instead_of_passing_it() {
        let priority = |view: &PlayerView| Pending::Priority {
            player: view.seat,
            legal: Box::default(),
        };

        let (mut batch, view) = a_batch_answered_once();
        assert_eq!(
            batch.answer(&view, &priority(&view)),
            Some(PlayerAction::PassPriority),
            "a quiet window is passed, as before"
        );

        let (mut batch, mut view) = a_batch_answered_once();
        view.owed = Some(baylee_core::mana::ManaPayment::Fixed(
            baylee_core::mana::ManaCost::parse("{3}{U}{U}"),
        ));
        view.awaiting = Some(view.seat);
        assert_eq!(batch.answer(&view, &priority(&view)), None);
        assert!(batch.is_idle(), "and it does not go on after it");
    }

    /// Mindslaver (CR 724.5): the controlled player's stacked "may"
    /// abilities are this seat's to answer, so one yes covers their run and
    /// the controlled player's priority between them is passed. This seat's
    /// own priority in the same turn (CR 724.8) is its own decision, which
    /// the approval did not cover.
    #[test]
    fn a_controlled_players_run_is_batched_and_the_seats_own_priority_is_not_passed() {
        let them = PlayerId::new(1);
        let mut view = ViewBuilder::new(2).controlling(1, vec![]).build();
        for id in 1..=3 {
            let mut o = token(id, 1, "Ondu Cleric", 1, 1);
            o.stack_item = Some(StackItem::Ability {
                token: None,
                source: ObjectId::new(500 + id, 0),
                ability: None,
                rules: Some(baylee_view::RulesFace {
                    card: baylee_core::ids::CardIndex::new(12),
                    face: 0,
                }),
                text: Some(baylee_view::StackText {
                    face: 0,
                    line: 0,
                    of: 1,
                }),
            });
            view.stack.push(o);
        }
        let may = |player| Pending::YesNo {
            player,
            prompt: YesNoPrompt::MayDo,
            source: None,
        };
        assert!(
            YesBatch::begin(&view, &may(view.seat)).is_none(),
            "this seat is not the one asked"
        );
        let mut batch = YesBatch::begin(&view, &may(them)).expect("their run");
        assert_eq!(
            batch.answer(&view, &may(them)),
            Some(PlayerAction::YesNo(true))
        );
        view.stack.pop();
        view.seq += 1;
        let priority = |player| Pending::Priority {
            player,
            legal: Box::default(),
        };
        assert_eq!(
            batch.answer(&view, &priority(them)),
            Some(PlayerAction::PassPriority),
            "the controlled player's priority between the answers"
        );
        view.seq += 1;
        view.decision_player = Some(view.seat);
        assert_eq!(
            batch.answer(&view, &priority(view.seat)),
            None,
            "this seat's own priority is left to it"
        );
        assert!(!batch.is_idle(), "and the run waits for it");
    }
}
