//! Public activator permissions, command-zone entry and activation pricing.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::Cause;
use crate::zone::ZonePosition;
use baylee_core::generated::index;
use baylee_core::mana::ManaPool;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

mod derevi;
mod edge_cases;
mod payment_lock;
mod power_artifact;
mod public_activations;
mod regeneration;
mod regeneration_concession;
mod rhystic;

fn activate_as(
    e: &mut Engine<RegistryLookup>,
    player: PlayerId,
    source: ObjectId,
    ability_index: u32,
) {
    e.refresh_offer();
    e.apply(
        player,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .unwrap();
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let player = e.pending().asked().unwrap();
    e.apply(player, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}

fn cast_at(e: &mut Engine<RegistryLookup>, card: CardIndex, objects: Vec<ObjectId>) {
    let card = in_hand(e, P0, card).unwrap();
    e.refresh_offer();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    targets(e, objects, vec![]);
    pass_until(e, stack_is_empty);
}

fn set_pool(e: &mut Engine<RegistryLookup>, player: PlayerId, amounts: &[(ManaColor, u32)]) {
    let mana = &mut e.state.players[usize::from(player.get())].mana_pool;
    *mana = ManaPool::new();
    for &(color, amount) in amounts {
        mana.add(color, amount);
    }
    e.refresh_offer();
}
