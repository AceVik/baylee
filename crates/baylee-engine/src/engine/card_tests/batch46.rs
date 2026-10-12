//! Whole-package regressions for restricted mana, provenance and granted abilities.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::zone::ZonePosition;
use baylee_core::generated::index;
use baylee_core::mana::ManaPool;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

mod agatha;
mod mana_cards;
mod metadata;
mod pools;
mod spent_mana;

fn cast(e: &mut Engine<RegistryLookup>, card: ObjectId) {
    e.refresh_offer();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
}

fn target(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let player = e.pending().asked().expect("target question");
    e.apply(player, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}

fn yes(e: &mut Engine<RegistryLookup>, answer: bool) {
    let player = e.pending().asked().expect("yes/no question");
    e.apply(player, PlayerAction::YesNo(answer)).unwrap();
}

fn reset_pool(e: &mut Engine<RegistryLookup>) {
    e.state.players[0].mana_pool = ManaPool::new();
    e.refresh_offer();
}
