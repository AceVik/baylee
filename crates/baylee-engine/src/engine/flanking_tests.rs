//! Flanking (CR 702.25a): "Whenever a creature without flanking blocks this
//! creature, the blocking creature gets -1/-1 until end of turn."
//!
//! The engine reads the keyword, as it reads prowess: a creature that gains
//! flanking (Agility) has the trigger, and one that loses it (Barbed
//! Foliage) does not. Mirage's cards used to print the keyword beside a
//! hand-written trigger, which no rule read, so Barbed Foliage's "loses
//! flanking" left the granted trigger in place.

use super::testkit::{
    Duel, card_index, cast_from_hand, keep_mulligans, on_battlefield, pass_until, pt,
    reach_main_phase, stack_is_empty,
};
use super::*;
use baylee_core::ids::{CardIndex, Defender};

const SEED: u64 = 0xf1a_4c1e;

/// "Enchant creature. Enchanted creature gets +1/+1 and has flanking."
fn agility() -> CardIndex {
    card_index("0f5f8b47-d104-4e29-b052-02d730d76ec0")
}

/// "Whenever a creature attacks you, it loses flanking until end of turn."
fn barbed_foliage() -> CardIndex {
    card_index("f9275c99-0616-44b3-a19e-98c683e87ad3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// Enchants p0's creature with Agility, attacks p1 with it and lets
/// p1 block with `blocker`. Returns the engine after the block's triggers
/// resolved, with the blocker's id.
fn attack_into(
    attacker_card: CardIndex,
    them: &[CardIndex],
    blocker: CardIndex,
) -> (Engine<testkit::RegistryLookup>, ObjectId) {
    let (us, they) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[agility()])
        .battlefield(0, &[attacker_card, mountain(), mountain()])
        .battlefield(1, them)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, us);
    let attacker = on_battlefield(&engine, us, attacker_card).unwrap();
    let original = pt(&engine, attacker);
    let blocking = on_battlefield(&engine, they, blocker).unwrap();
    cast_from_hand(&mut engine, us, agility());
    engine
        .apply(
            us,
            PlayerAction::ChooseTargets {
                objects: vec![attacker],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, attacker),
        (original.0 + 1, original.1 + 1),
        "Agility is on the attacker"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            us,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(they))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            they,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocking, attacker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, blocking)
}

/// A granted flanking shrinks the blocker once: one -1/-1, not two.
#[test]
fn a_creature_that_gains_flanking_shrinks_its_blocker_once() {
    let (engine, giant) = attack_into(grizzly_bears(), &[hill_giant()], hill_giant());
    assert_eq!(pt(&engine, giant), (2, 2));
}

/// The trigger is the keyword's: a creature that has lost flanking by the
/// time it is blocked has no trigger left.
#[test]
fn a_creature_that_lost_flanking_does_not_shrink_its_blocker() {
    let (engine, giant) = attack_into(
        grizzly_bears(),
        &[hill_giant(), barbed_foliage()],
        hill_giant(),
    );
    assert_eq!(pt(&engine, giant), (3, 3));
}

#[test]
fn agility_adds_a_second_instance_to_a_native_flanker() {
    let herder = card_index("0edf5bac-36a9-4f35-afd1-73f7e1aab51b");
    let (engine, giant) = attack_into(herder, &[hill_giant()], hill_giant());
    assert_eq!(pt(&engine, giant), (1, 1));
}
