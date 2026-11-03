//! Casting a creature spell from start to finish with a real card
//! (Llanowar Elves, {G}): it goes on the stack (CR 601.2a), its caster
//! gets priority back (CR 117.3c), it resolves onto the battlefield once
//! both players pass (CR 608.3), it cannot be cast without the mana or at
//! the wrong time (CR 307.1 for sorcery timing of a creature), and it is
//! summoning sick until its controller's next turn (CR 302.6).

use super::testkit::{
    Duel, basic_forest, cast_from_hand, in_hand, keep_mulligans, on_battlefield, on_stack,
    pass_until, quiet_creature, stack_is_empty,
};
use super::*;

const SEED: u64 = 0xca57_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn main_of(engine: &mut Engine<super::testkit::RegistryLookup>, seat: PlayerId) {
    let start = engine.state().turn.number;
    let here = |e: &Engine<super::testkit::RegistryLookup>| {
        e.state().turn.active == seat
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
            && e.state().zones.stack_is_empty()
    };
    let already = here(engine);
    pass_until(engine, |e| {
        here(e) && (!already || e.state().turn.number > start)
    });
}

fn duel(forests: usize) -> Engine<super::testkit::RegistryLookup> {
    let forest = basic_forest();
    let mut engine = Duel::new(SEED, forest)
        .hand(0, &[quiet_creature()])
        .hand(1, &[quiet_creature()])
        .battlefield(0, &vec![forest; forests])
        .battlefield(1, &vec![forest; forests])
        .start();
    keep_mulligans(&mut engine);
    main_of(&mut engine, P0);
    engine
}

#[test]
fn a_cast_creature_waits_on_the_stack_and_its_caster_keeps_priority() {
    let mut engine = duel(1);
    cast_from_hand(&mut engine, P0, quiet_creature());
    assert!(on_stack(&engine, quiet_creature()).is_some());
    assert!(in_hand(&engine, P0, quiet_creature()).is_none());
    assert!(on_battlefield(&engine, P0, quiet_creature()).is_none());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
        "the caster receives priority again, got {:?}",
        engine.pending()
    );
}

#[test]
fn the_creature_resolves_onto_its_casters_battlefield() {
    let mut engine = duel(1);
    cast_from_hand(&mut engine, P0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, quiet_creature()).is_some());
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_none());
    assert_eq!(engine.state().turn.active, P0, "still the caster's turn");
}

#[test]
fn a_creature_without_the_mana_for_it_is_not_cast() {
    let mut engine = duel(0);
    let card = in_hand(&engine, P0, quiet_creature()).expect("in hand");
    let before = engine.state().snapshot_hash();
    assert!(engine.apply(P0, PlayerAction::CastSpell { card }).is_err());
    assert_eq!(engine.state().snapshot_hash(), before);
    assert!(in_hand(&engine, P0, quiet_creature()).is_some());
}

#[test]
fn a_creature_is_not_cast_on_the_opponents_turn() {
    let mut engine = duel(1);
    main_of(&mut engine, P1);
    // Seat 0 gets priority at some point of seat 1's turn; a creature is
    // still not castable for it.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    assert_eq!(engine.state().turn.active, P1);
    let card = in_hand(&engine, P0, quiet_creature()).expect("in hand");
    assert!(engine.apply(P0, PlayerAction::CastSpell { card }).is_err());
}

#[test]
fn a_creature_is_not_cast_while_the_stack_holds_something() {
    let mut engine = duel(2);
    cast_from_hand(&mut engine, P0, quiet_creature());
    // A second copy for seat 0 is not in hand, so ask seat 1, who has one,
    // when it receives priority with the elf still on the stack.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P1));
    let card = in_hand(&engine, P1, quiet_creature()).expect("in hand");
    assert!(
        engine.apply(P1, PlayerAction::CastSpell { card }).is_err(),
        "sorcery timing needs an empty stack and the active player"
    );
}

#[test]
fn a_new_creature_cannot_attack_until_its_controllers_next_turn() {
    let mut engine = duel(1);
    cast_from_hand(&mut engine, P0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, P0, quiet_creature()).expect("resolved");
    assert!(!crate::combat::can_attack(engine.state(), P0, elf));
    main_of(&mut engine, P1);
    assert!(
        !crate::combat::can_attack(engine.state(), P0, elf),
        "it is not its controller's turn"
    );
    main_of(&mut engine, P0);
    assert!(crate::combat::can_attack(engine.state(), P0, elf));
}

#[test]
fn paying_for_the_spell_taps_the_land() {
    let mut engine = duel(1);
    let forest = on_battlefield(&engine, P0, basic_forest()).expect("a forest");
    cast_from_hand(&mut engine, P0, quiet_creature());
    assert!(
        engine
            .state()
            .object(forest)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}
