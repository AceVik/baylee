//! "At the beginning of the next end step" about one object
//! (`Effect::AtNextEndStep`, CR 603.7): a delayed triggered ability with the
//! creating ability's source and controller (CR 603.7e) that uses the stack,
//! and that does nothing to an object that has left its zone since, even if
//! it came back (CR 603.7c, 400.7).
//!
//! Stone Giant is the card that prints it: "{T}: Target creature you control
//! with toughness less than this creature's power gains flying until end of
//! turn. Destroy that creature at the beginning of the next end step."

use super::testkit::{
    Duel, RegistryLookup, card_index, keep_mulligans, on_battlefield, pass_until, stack_is_empty,
    walk_to_own_main,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

fn stone_giant() -> CardIndex {
    card_index("0b8e3f9b-a4da-49a3-8545-ce7a265e5856")
}

/// A 1/1: toughness 1, under the Giant's 3.
fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// The Giant and the Elves on p0's side, p0 in its first main phase, and
/// the Giant's ability resolved at the Elves.
fn thrown(seed: u64) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[stone_giant(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let giant = on_battlefield(&engine, p0, stone_giant()).expect("the Giant is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: giant,
                ability_index: 0,
            },
        )
        .expect("the Giant is untapped");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the ability asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !options.contains(&giant),
        "the Giant's own toughness 4 is not less than its power 3"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elves are a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(elf)
            .expect("still there")
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::FLYING),
        "the Elves fly until end of turn"
    );
    (engine, elf)
}

fn at_end_step(engine: &Engine<RegistryLookup>) -> bool {
    engine.state().turn.step == crate::turn::Step::End
}

/// The trigger goes on the stack at the beginning of the end step, and the
/// creature is destroyed as it resolves.
#[test]
fn the_creature_is_destroyed_by_a_trigger_at_the_next_end_step() {
    let p0 = PlayerId::new(0);
    let (mut engine, elf) = thrown(91);
    pass_until(&mut engine, at_end_step);
    assert!(
        !stack_is_empty(&engine),
        "the delayed trigger uses the stack (CR 603.7)"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and nothing is destroyed before it resolves"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(elf).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard),
        "that creature is destroyed"
    );
}

/// A creature that left the battlefield and came back is a new object, and
/// the trigger does nothing to it (CR 603.7c, 400.7).
#[test]
fn a_creature_that_left_and_came_back_is_not_destroyed() {
    let p0 = PlayerId::new(0);
    let (mut engine, elf) = thrown(93);
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    for to in [
        crate::zone::ZoneLocation::Hand(p0),
        crate::zone::ZoneLocation::Battlefield,
    ] {
        let _ = state.move_object(
            elf,
            to,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
    }
    pass_until(&mut engine, at_end_step);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(elf).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "the Elves that came back are a new object and stay"
    );
}
