//! Delayed effects are exercised through actual card activations and turns.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::Cause;
use crate::zone::ZonePosition;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
mod edges;
mod focused;
mod review_edges;

fn targets(e: &mut Engine<RegistryLookup>, ids: Vec<ObjectId>) {
    let player = e.pending().asked().unwrap();
    e.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: ids,
            players: vec![],
        },
    )
    .unwrap();
}
fn main_after(e: &mut Engine<RegistryLookup>, player: PlayerId, after: u32) {
    pass_until(e, |e| {
        e.state.turn.number > after
            && e.state.turn.active == player
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player)
    });
}
fn attack(
    e: &mut Engine<RegistryLookup>,
    player: PlayerId,
    creature: ObjectId,
    defender: PlayerId,
) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player: p, .. } if *p == player),
    );
    e.apply(
        player,
        PlayerAction::DeclareAttackers {
            attackers: vec![(creature, Defender::Player(defender))],
        },
    )
    .unwrap();
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseBlockers { .. }));
}
fn cast_target(e: &mut Engine<RegistryLookup>, card: CardIndex, object: ObjectId) {
    let card = in_hand(e, P0, card).unwrap();
    for color in ManaColor::ALL {
        e.state.players[0].mana_pool.add(color, 30);
    }
    e.refresh_offer();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    targets(e, vec![object]);
    pass_until(e, stack_is_empty);
}
fn blink(e: &mut Engine<RegistryLookup>, id: ObjectId, owner: PlayerId) {
    e.state
        .move_object(
            id,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            id,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
}
