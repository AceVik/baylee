//! Life-loss replacements apply to results, never to damage amounts.

use super::{DamageTarget, DamageWork, GameState, KeywordSet};
use baylee_cards_dsl::Modifier;
use baylee_core::ids::PlayerId;

/// Remaining damage life loss permitted for each seat in this event.
pub(super) struct LifeLossBudgets(Vec<Option<i64>>);

impl LifeLossBudgets {
    pub(super) fn new(work: &DamageWork, state: &GameState) -> Self {
        let mut budgets: Vec<_> = state
            .players
            .iter()
            .map(|player| {
                floor(state, player.id).map(|minimum| i64::from(player.life) - i64::from(minimum))
            })
            .collect();
        // CR 120.4c–d (2026-09-25), including the Worship/Awe Strike
        // example: simultaneous gains participate in the event's net life
        // change before the floor replaces its life-loss result. Lich's
        // draws are not gains and cannot enlarge this allowance.
        for (player, amount) in work.life_gains.iter().copied().chain(
            work.parts
                .iter()
                .filter(|part| part.keywords.contains(KeywordSet::LIFELINK))
                .map(|part| (part.controller, part.view.amount)),
        ) {
            if !state.has_left(player)
                && !state.life_total_cant_change(player)
                && !state.life_gain_draws_instead(player)
                && let Some(budget) = &mut budgets[usize::from(player.get())]
            {
                *budget =
                    budget.saturating_add(i64::from(i32::try_from(amount).unwrap_or(i32::MAX)));
            }
        }
        for budget in budgets.iter_mut().flatten() {
            *budget = (*budget).max(0);
        }
        Self(budgets)
    }

    pub(super) fn loss(&mut self, recipient: DamageTarget, amount: u32) -> i32 {
        let DamageTarget::Player(player) = recipient else {
            return 0;
        };
        let amount = i32::try_from(amount).unwrap_or(i32::MAX);
        let Some(budget) = &mut self.0[usize::from(player.get())] else {
            return amount;
        };
        let loss = i64::from(amount).min(*budget);
        *budget -= loss;
        i32::try_from(loss).expect("loss is bounded by an i32 damage result")
    }
}

fn floor(state: &GameState, player: PlayerId) -> Option<i32> {
    state
        .effects
        .iter()
        .filter_map(|effect| {
            let Modifier::DamageLifeLossFloor { players, minimum } = effect.modifier else {
                return None;
            };
            crate::eval::players(players, state, effect.controller)
                .is_some_and(|players| players.contains(&player))
                .then_some(minimum)
        })
        .max()
}

#[cfg(test)]
#[path = "life_result_tests.rs"]
mod tests;
