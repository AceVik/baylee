//! Draw-replacement progress, normalized to zone positions for loop detection.
use super::{GameState, Hasher, ObjectId, moves_since};
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
    // Past instruction draws are not a future rule input on their own: a new
    // instruction starts at the ledger's current length. An active loot hashes
    // its eligible hand positions through resolve::loot::loop_progress instead.
    // The full ledger remains part of snapshot_hash through PerTurn::hash.
}
