//! Full transformation, linked-return and source-mana regressions for batch 48.
#[allow(clippy::wildcard_imports)]
use super::*;
mod brass;
mod brass_edges;
mod brass_large_hand;
mod havengul;
mod havengul_edges;
mod havengul_final;
mod havengul_lost_copy;
mod smithy;
mod smithy_edges;
mod smithy_source_departure;

fn cast_exact(e: &mut Engine<RegistryLookup>, card: CardIndex) {
    priority(e, P0);
    let card = in_hand(e, P0, card).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
}
