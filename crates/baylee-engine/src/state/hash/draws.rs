//! Draw-replacement progress, normalized to zone positions for loop detection.
use super::{GameState, Hasher, ObjectId, Zone, moves_since};
use std::hash::Hash;

pub(super) fn hash(h: &mut Hasher, state: &GameState, position: &impl Fn(ObjectId) -> u32) {
    state.draws_to_offer.hash(h);
    let front = &state.draw_front;
    for objects in [&front.declined, &front.applied] {
        let mut positions: Vec<_> = objects.iter().map(|&id| position(id)).collect();
        positions.sort_unstable();
        positions.dedup();
        positions.hash(h);
    }
    front.dredge_declined.hash(h);
    front.replaced.hash(h);
    let looked: Vec<_> = front.looked.iter().map(|&id| position(id)).collect();
    looked.hash(h);
    if let Some((card, life, remaining)) = &front.denial {
        h.u8(1);
        h.u32(position(card.object));
        moves_since(state.object_or_departed(card.object), card.version).hash(h);
        life.hash(h);
        remaining.hash(h);
    } else {
        h.u8(0);
    }
    // A departed hand incarnation can never become eligible again. Its historical
    // ID and the absolute length of the ledger must not hide a repeating situation.
    let mut drawn: Vec<_> = state
        .per_turn
        .instruction_drawn
        .iter()
        .filter_map(|&(id, version)| {
            state
                .object(id)
                .filter(|o| o.zone == Zone::Hand && o.version == version)
                .map(|_| position(id))
        })
        .collect();
    drawn.sort_unstable();
    drawn.dedup();
    drawn.hash(h);
}
