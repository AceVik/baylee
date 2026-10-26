//! Exchange entire blocking assignments only when the resulting blocks are legal.
use super::{BlockerInfo, GameState, ObjectId, block_bound, can_block};

/// Retarget existing blocks without declaring new blockers (CR 509.5).
/// A creature already blocking both attackers keeps both assignments.
pub(crate) fn exchange_blockers(state: &mut GameState, first: ObjectId, second: ObjectId) -> bool {
    if first == second || !state.combat.is_attacking(first) || !state.combat.is_attacking(second) {
        return false;
    }
    let proposed: Vec<BlockerInfo> = state
        .combat
        .blockers
        .iter()
        .map(|pair| BlockerInfo {
            blocker: pair.blocker,
            attacker: if pair.attacker == first {
                second
            } else if pair.attacker == second {
                first
            } else {
                pair.attacker
            },
        })
        .collect();
    for pair in proposed
        .iter()
        .filter(|pair| pair.attacker == first || pair.attacker == second)
    {
        let Some(blocker) = state.object(pair.blocker) else {
            return false;
        };
        if !can_block(state, blocker.controller, pair.blocker, pair.attacker) {
            return false;
        }
    }
    for attacker in [first, second] {
        let count = proposed
            .iter()
            .filter(|pair| pair.attacker == attacker)
            .count();
        if let Some(bound) = block_bound(state, attacker)
            && count > 0
            && (count < bound.min_blockers as usize || count > bound.max_blockers as usize)
        {
            return false;
        }
    }
    // Existing blocked status persists even when a creature loses its blockers.
    // Newly blocked attackers acquire that status without emitting a declaration event.
    let newly_blocked: Vec<_> = state
        .combat
        .attackers
        .iter()
        .filter(|attacker| {
            !attacker.blocked
                && proposed
                    .iter()
                    .any(|pair| pair.attacker == attacker.creature)
        })
        .filter_map(|attacker| state.source_identity(attacker.creature))
        .collect();
    for reference in newly_blocked {
        state.record_blocked_or_blocking(reference, reference);
    }
    for attacker in &mut state.combat.attackers {
        if proposed
            .iter()
            .any(|pair| pair.attacker == attacker.creature)
        {
            attacker.blocked = true;
        }
    }
    state.combat.blockers = proposed;
    true
}
