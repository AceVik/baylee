//! `cards/artifacts/mv_4/living_armor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn living_armor() -> CardIndex {
    card_index("7a32b0d1-d9dc-4d24-ae60-3f3a0e18ec03")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Living Armor sacrificed at `target` (p0's or p1's), resolved.
fn armored(
    target_mine: bool,
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut all_mine = vec![living_armor()];
    all_mine.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &all_mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let (seat, card) = if target_mine {
        (p0, mine[0])
    } else {
        (p1, theirs[0])
    };
    let creature = on_battlefield(&engine, seat, card).expect("the target");
    activate(&mut engine, p0, living_armor(), 0);
    // The target is chosen before the cost is paid (CR 602.2b, 601.2c, 601.2h).
    aim_at(&mut engine, p0, creature);
    assert!(
        on_battlefield(&engine, p0, living_armor()).is_none(),
        "tapped and sacrificed as the price, before the ability resolves"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, living_armor()).is_some());
    (engine, creature)
}

/// Living Armor (7a32b0d1-d9dc-4d24-ae60-3f3a0e18ec03) — "{T}, Sacrifice this
/// artifact: Put X +0/+1 counters on target creature, where X is that
/// creature's mana value."
///
/// Serra Angel ({3}{W}{W}, 4/4): five counters, toughness 9, power untouched.
#[test]
fn living_armor_puts_mana_value_toughness_counters_on_its_target() {
    let (engine, angel) = armored(true, &[serra_angel()], &[]);
    assert_eq!(pt(&engine, angel), (4, 9), "4/4 plus five +0/+1 counters");
}

/// Any creature will do, and X is read off the creature that is hit: the
/// opponent's Hill Giant ({3}{R}, mana value 4) gets four.
#[test]
fn living_armor_reads_the_targets_own_mana_value() {
    let (engine, giant) = armored(false, &[grizzly_bears()], &[hill_giant()]);
    assert_eq!(pt(&engine, giant), (3, 7), "3/3 plus four +0/+1 counters");
}
