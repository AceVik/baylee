//! `cards/creatures/mv_4/angelic_protector.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn angelic_protector() -> CardIndex {
    card_index("cff434e1-e567-4a80-8dc5-593d70b0f348")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

/// p0 has Angelic Protector, a Grizzly Bears, a Mountain and a Lightning Bolt
/// in hand; p1 has a Grizzly Bears and a Prodigal Sorcerer. p0's main phase.
fn a_protector_and_a_bolt() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[angelic_protector(), grizzly_bears(), mountain()])
        .battlefield(1, &[grizzly_bears(), prodigal_sorcerer()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Angelic Protector (cff434e1-e567-4a80-8dc5-593d70b0f348) — {3}{W} Creature — Angel:
/// "Flying. Whenever this creature becomes the target of a spell or ability, this creature gets +0/+3 until end of turn."
#[test]
fn angelic_protector_flies_and_gets_plus_zero_plus_three_when_a_spell_targets_it() {
    let mut engine = a_protector_and_a_bolt();
    let protector = on_battlefield(&engine, P0, angelic_protector()).unwrap();
    assert_eq!(pt(&engine, protector), (2, 2));
    assert!(keywords(&engine, protector).contains(KeywordSet::FLYING));

    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, protector);
    pass_until(&mut engine, stack_is_empty);

    // The trigger resolves before the Bolt, so the 3 damage meets toughness 5.
    assert!(on_battlefield(&engine, P0, angelic_protector()).is_some());
    assert_eq!(pt(&engine, protector), (2, 5));
}

#[test]
fn angelic_protector_is_back_to_two_two_after_the_turn_ends() {
    let mut engine = a_protector_and_a_bolt();
    let protector = on_battlefield(&engine, P0, angelic_protector()).unwrap();

    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, protector);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, protector), (2, 5));

    reach_their_main_phase(&mut engine, P1);

    assert!(
        on_battlefield(&engine, P0, angelic_protector()).is_some(),
        "damage and the bonus both wear off in the cleanup step"
    );
    assert_eq!(pt(&engine, protector), (2, 2), "until end of turn");
}

#[test]
fn angelic_protector_gets_nothing_when_another_creature_is_targeted() {
    let mut engine = a_protector_and_a_bolt();
    let protector = on_battlefield(&engine, P0, angelic_protector()).unwrap();
    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, their_bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
    assert_eq!(
        pt(&engine, protector),
        (2, 2),
        "only becoming the target itself counts"
    );
}

#[test]
fn angelic_protector_also_answers_an_ability_that_targets_it() {
    let mut engine = a_protector_and_a_bolt();
    let protector = on_battlefield(&engine, P0, angelic_protector()).unwrap();
    reach_their_main_phase(&mut engine, P1);

    activate(&mut engine, P1, prodigal_sorcerer(), 0);
    aim_at(&mut engine, P1, protector);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, protector),
        (2, 5),
        "targeted by Prodigal Sorcerer's ping: the +0/+3 is on, the 1 damage is marked"
    );
    assert!(on_battlefield(&engine, P0, angelic_protector()).is_some());
}
