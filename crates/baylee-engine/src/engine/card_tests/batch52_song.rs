//! Titania's Song: layer cohort and continuous departure lifetime.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
fn obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}
fn priority(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player:p,..}if *p==player),
    );
}
fn cast(e: &mut Engine<RegistryLookup>, c: CardIndex) {
    priority(e, P0);
    mana(e);
    let card = in_hand(e, P0, c).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
}
fn target(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    let p = e.pending().asked().unwrap();
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![id],
            players: vec![],
        },
    )
    .unwrap();
}
fn cast_target(e: &mut Engine<RegistryLookup>, c: CardIndex, id: ObjectId) {
    cast(e, c);
    target(e, id);
    pass_until(e, stack_is_empty);
}
fn next_main(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == player
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player:p,..}if *p==player)
    });
}

mod acquisitions;
mod boundaries;
mod cohort;
mod lifetime;
