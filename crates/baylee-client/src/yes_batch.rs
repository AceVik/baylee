//! One explicit yes for the finite run of identical abilities already stacked.
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
pub fn candidates(view: &PlayerView, pending: &Pending) -> Vec<ObjectId> {
    if !matches!(pending, Pending::YesNo {
        player, prompt: YesNoPrompt::MayDo, ..
    } if *player == view.seat)
    {
        return Vec::new();
    }
    let Some(top) = view.stack.last().filter(|o| {
        o.controller == view.seat && matches!(o.stack_item, Some(StackItem::Ability { .. }))
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
    pub fn answer(&mut self, view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
        if self.sent_at == Some(view.seq) {
            return None;
        }
        let top = view.stack.last()?.id;
        if self.answered == Some(top) {
            return None;
        }
        if !self.remaining.contains(&top) {
            self.remaining.clear();
            return None;
        }
        let answer = match pending {
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } if *player == view.seat => {
                self.remaining.retain(|id| *id != top);
                self.answered = Some(top);
                PlayerAction::YesNo(true)
            }
            Pending::Priority { player, .. } if *player == view.seat => PlayerAction::PassPriority,
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
}
