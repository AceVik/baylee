//! Thran Portal and Quarum Trench Gnomes actual-action acceptance.
#[allow(clippy::wildcard_imports)]
use super::*;
mod gnomes;
mod payment;
mod replacement_edges;
mod thran;
use super::counter_resources::prepared;
use crate::event::Cause;
use crate::zone::ZonePosition;
use baylee_core::{
    generated::{index, subtypes},
    mana::ManaPool,
};
const P0: PlayerId = PlayerId::new(0);
fn reset(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        p.mana_pool = ManaPool::new();
    }
    e.refresh_offer();
}
fn tap_source(e: &mut Engine<RegistryLookup>, player: PlayerId, source: ObjectId) {
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    if !legal.mana_abilities.contains(&source) {
        let ability_index = legal
            .abilities
            .iter()
            .find(|(id, _)| *id == source)
            .expect("mana ability offered")
            .1;
        e.apply(
            player,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
        return;
    }
    e.apply(player, PlayerAction::ActivateManaAbility { source })
        .unwrap();
}
fn target(e: &mut Engine<RegistryLookup>, object: ObjectId) {
    let player = e.pending().asked().unwrap();
    e.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: vec![object],
            players: vec![],
        },
    )
    .unwrap();
}
fn next_main(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == player
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player:p,..} if *p==player)
    });
}

mod text_change;
