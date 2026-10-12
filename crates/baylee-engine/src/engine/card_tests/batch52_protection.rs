//! Actual-action regressions for The One Ring and Teferi’s Protection.
mod resolution_target;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
mod basics;
mod cast_provenance;
mod life_and_targeting;
mod phasing;
fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}
fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}
fn obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p),
    );
}
fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e);
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}
fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}
fn cast_target(e: &mut Engine<RegistryLookup>, c: CardIndex, target: ObjectId) {
    cast(e, P0, c);
    targets(e, vec![target], vec![]);
    pass_until(e, stack_is_empty);
}
fn activate(e: &mut Engine<RegistryLookup>, c: CardIndex, i: u32) {
    priority(e, P0);
    mana(e);
    let source = obj(e, P0, c);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source,
            ability_index: i,
        },
    )
    .unwrap();
}
fn next_main(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == p
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player,..} if *player==p)
    });
}
fn choose(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseObjects { objects }).unwrap();
}
