//! Batch 45 validates the entire package through real casts and activations.
#[allow(clippy::wildcard_imports)]
use super::*;
use super::{
    counter_resources::prepared,
    library_choices::{choose, library},
};
use crate::event::Cause;
use crate::object::Rider;
use crate::zone::ZonePosition;
use baylee_core::generated::{index, subtypes};
use baylee_core::mana::ManaPool;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
mod boundaries;
mod copy_payment_cases;
mod disturb_cases;
mod escape_cases;
mod graveyard;
mod payments;
mod resolution;
mod ring_cases;
mod saga_cases;

fn cast(e: &mut Engine<RegistryLookup>, card: ObjectId) {
    e.refresh_offer();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
}
fn target(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let player = e.pending().asked().unwrap();
    e.apply(player, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}
fn yes(e: &mut Engine<RegistryLookup>, answer: bool) {
    let player = e.pending().asked().unwrap();
    e.apply(player, PlayerAction::YesNo(answer)).unwrap();
}
fn mode(e: &mut Engine<RegistryLookup>, kind: CastModeKind) {
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        return;
    };
    let option = options
        .iter()
        .find(|option| option.kind == kind)
        .unwrap_or_else(|| panic!("missing {kind:?} in {options:?}"))
        .index;
    e.apply(P0, PlayerAction::ChooseMode(option as usize))
        .unwrap();
}
fn pool(e: &mut Engine<RegistryLookup>, color: ManaColor, amount: u32) {
    e.state.players[0].mana_pool = ManaPool::new();
    e.state.players[0].mana_pool.add(color, amount);
    e.refresh_offer();
}
fn castable(e: &Engine<RegistryLookup>, card: ObjectId) -> bool {
    crate::casting::can_cast(e.state(), &RegistryLookup, P0, card).is_ok()
}
fn exile_choice(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::PlayFromExile,
            ..
        }
    )
}
fn cards_offered(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    let Pending::ChooseCards { options, .. } = e.pending() else {
        panic!("{:?}", e.pending());
    };
    options.clone()
}
