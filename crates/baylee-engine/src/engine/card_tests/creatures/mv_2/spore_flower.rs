//! `cards/creatures/mv_2/spore_flower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn spore_flower() -> CardIndex {
    card_index("f352ec62-bf09-45f2-b5b7-75931177b15f")
}

/// A duel on turn one, main phase, `p0` with the Flower and a Gray Ogre.
fn board() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[spore_flower(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Declares the Ogre as the only attacker against the other seat and runs
/// combat out to the second main phase.
fn attack_with_the_ogre(engine: &mut Engine<RegistryLookup>) {
    let ogre = on_battlefield(engine, P0, gray_ogre()).expect("the Ogre is out");
    pass_until(engine, |e| e.state().turn.step == Step::DeclareAttackers);
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(P1))],
            },
        )
        .expect("the Ogre attacks");
    pass_until(engine, |e| e.state().turn.phase == Phase::SecondMain);
}

/// Spore Flower (f352ec62-bf09-45f2-b5b7-75931177b15f) — "At the beginning
/// of your upkeep, put a spore counter on this creature."
#[test]
fn spore_flower_gains_a_spore_counter_each_of_your_upkeeps() {
    let mut engine = board();
    let flower = on_battlefield(&engine, P0, spore_flower()).expect("the Flower is out");
    assert_eq!(
        counters_on(&engine, flower, counters::SPORE),
        1,
        "this turn's upkeep already put one"
    );

    // The opponent's upkeep is not mine: nothing is added there.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, flower, counters::SPORE), 1);

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, flower, counters::SPORE), 2);
}

/// "Remove three spore counters from this creature: Prevent all combat damage
/// that would be dealt this turn."
#[test]
fn spore_flower_prevents_all_combat_damage_this_turn() {
    // Control: with no prevention the Ogre connects.
    let mut engine = board();
    attack_with_the_ogre(&mut engine);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "unchecked, the 2/2 Ogre hits for two"
    );

    let mut engine = board();
    let flower = on_battlefield(&engine, P0, spore_flower()).expect("the Flower is out");
    crate::replacement::put_counters(&mut engine.state, flower, counters::SPORE, 2);
    engine.refresh_offer();
    assert_eq!(counters_on(&engine, flower, counters::SPORE), 3);

    activate(&mut engine, P0, spore_flower(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, flower, counters::SPORE),
        0,
        "three counters were the cost"
    );

    attack_with_the_ogre(&mut engine);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the Ogre's combat damage was prevented"
    );
}

/// Fewer than three counters: the ability is not available.
#[test]
fn spore_flower_needs_three_counters_to_prevent_damage() {
    let mut engine = board();
    let flower = on_battlefield(&engine, P0, spore_flower()).expect("the Flower is out");
    crate::replacement::put_counters(&mut engine.state, flower, counters::SPORE, 1);
    engine.refresh_offer();
    assert_eq!(counters_on(&engine, flower, counters::SPORE), 2);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: flower,
                    ability_index: 1
                }
            )
            .is_err(),
        "two counters cannot pay for three"
    );
    assert_eq!(counters_on(&engine, flower, counters::SPORE), 2);
}
