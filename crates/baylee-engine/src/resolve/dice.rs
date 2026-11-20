//! Dice rolling and `Modifier::RollAdditionalDieIgnoreLowest` (CR 706).

use crate::event::GameEvent;
use crate::state::GameState;
use baylee_cards_dsl::Modifier;
use baylee_core::ids::PlayerId;

/// Roll `count` `sides`-sided dice for `player`, applying active
/// `Modifier::RollAdditionalDieIgnoreLowest` replacements (CR 706.6),
/// journalling each kept natural roll via `GameEvent::DiceRolled`, and adding
/// `modifier` (clamped at `0`, CR 706.2).
pub(super) fn roll_dice(
    state: &mut GameState,
    player: PlayerId,
    count: u32,
    sides: u8,
    modifier: i8,
) -> u32 {
    if count == 0 || sides == 0 {
        return 0;
    }
    let extra = state
        .effects
        .iter()
        .filter(|fx| {
            fx.controller == player
                && matches!(fx.modifier, Modifier::RollAdditionalDieIgnoreLowest)
        })
        .count();
    let total_dice = usize::try_from(count)
        .unwrap_or(usize::MAX)
        .saturating_add(extra);
    let mut rolls: Vec<(u32, bool)> = (0..total_dice)
        .map(|_| ((state.rng.below(u64::from(sides)) as u32) + 1, false))
        .collect();
    // CR 706.6: ignored rolls are considered never to have happened, so they
    // neither contribute to the result nor emit `GameEvent::DiceRolled`.
    for _ in 0..extra {
        if let Some((lowest_idx, _)) = rolls
            .iter()
            .enumerate()
            .filter(|(_, (_, ignored))| !*ignored)
            .min_by_key(|(idx, (value, _))| (*value, *idx))
        {
            rolls[lowest_idx].1 = true;
        }
    }
    let mut sum = 0_u32;
    let face_count = u32::from(sides);
    for (natural, ignored) in rolls {
        if !ignored {
            state.journal.record(GameEvent::DiceRolled {
                player,
                sides: face_count,
                result: natural,
            });
            sum = sum.saturating_add(natural);
        }
    }
    i64::from(sum)
        .saturating_add(i64::from(modifier))
        .max(0)
        .try_into()
        .unwrap_or(u32::MAX)
}
