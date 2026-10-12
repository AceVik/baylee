//! Archon/Kismet acceptance through registered cards and actual entry/cast actions.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
mod archon;
mod kismet;
mod ordering;
fn obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p),
    );
}
fn mana(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    for c in ManaColor::ALL {
        e.state.players[usize::from(p.get())].mana_pool.add(c, 30);
    }
    e.refresh_offer();
}
fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e, p);
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}
fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}
fn next_main(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == p
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player,..}if *player==p)
    });
}
fn reject(e: &mut Engine<RegistryLookup>, p: PlayerId, card: ObjectId) {
    priority(e, p);
    mana(e, p);
    let hash = e.snapshot_hash();
    assert!(e.apply(p, PlayerAction::CastSpell { card }).is_err());
    assert_eq!(e.snapshot_hash(), hash);
}

mod shock_and_copy;

mod copied_lost_entry;

mod mass_entry;
