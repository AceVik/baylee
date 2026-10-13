//! `cards/creatures/mv_2/blazing_effigy.rs`, played.
//!
//! "When this creature dies, it deals X damage to target creature, where X is
//! 3 plus the amount of damage dealt to this creature this turn by other
//! sources named Blazing Effigy." A 0/3.
//!
//! The creature the trigger is aimed at is a Wall of Stone (0/8) throughout:
//! it survives every amount in play, so the damage left on it afterwards *is*
//! X, read as a number and not as "did it die".

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use baylee_core::generated::index;

// oracle_id = "7879b37c-5e2a-4945-9378-47744b715a6c"
fn blazing_effigy() -> CardIndex {
    card_index("7879b37c-5e2a-4945-9378-47744b715a6c")
}

/// "It deals X damage to target creature", X being 3 when nothing named
/// Blazing Effigy hurt it: Lightning Bolt kills the Effigy, its trigger is
/// aimed at one of two creatures on the other side, and exactly that creature
/// carries 3 damage afterwards. The creature it was not aimed at carries none,
/// so the 3 went where the target said.
#[test]
fn blazing_effigy_dying_deals_three_damage_to_the_creature_it_targets() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blazing_effigy(), mountain()])
        .battlefield(1, &[kit::wall_of_stone(), kit::bears()])
        .hand(0, &[kit::bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let effigy = on_battlefield(&engine, kit::P0, blazing_effigy()).expect("the Effigy is out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");
    let bears = on_battlefield(&engine, kit::P1, kit::bears()).expect("the Bear is out");

    tap_all_mana(&mut engine, kit::P0);
    let mut answers = kit::Answers::new().aim(&[effigy]).aim(&[wall]);
    kit::cast(&mut engine, kit::P0, kit::bolt(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);

    assert!(
        in_graveyard(&engine, kit::P0, blazing_effigy()).is_some(),
        "Lightning Bolt dealt 3 to a 0/3: the Effigy died"
    );
    assert!(
        answers
            .menus
            .iter()
            .skip(1)
            .any(|menu| menu.contains(&wall) && menu.contains(&bears)),
        "its trigger offered both creatures across the table: {:?}",
        answers.menus
    );
    assert_eq!(
        kit::damage_on(&engine, wall),
        3,
        "X is 3 plus nothing: damage from Lightning Bolt is not from a Blazing Effigy"
    );
    assert_eq!(
        kit::damage_on(&engine, bears),
        0,
        "the Bear was not the target and takes nothing"
    );
}

/// "… plus the amount of damage dealt to this creature this turn by other
/// sources named Blazing Effigy": two Effigies, and Lightning Bolt kills the
/// first. Its trigger is aimed at the second, which it kills with its own 3
/// (a 0/3); the second Effigy's trigger counts that 3 from "another source
/// named Blazing Effigy" and deals 3 + 3 = 6 to the Wall of Stone. The first
/// Effigy's trigger, which had the Bolt's 3 on it and nothing from an Effigy,
/// is the sibling test above: 3.
#[test]
fn blazing_effigy_adds_the_damage_another_effigy_dealt_to_it_this_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blazing_effigy(), blazing_effigy(), mountain()])
        .battlefield(1, &[kit::wall_of_stone(), kit::bears()])
        .hand(0, &[kit::bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let effigies = kit::objects_of(&engine, kit::P0, blazing_effigy());
    assert_eq!(effigies.len(), 2, "two Effigies out");
    let (first, second) = (effigies[0], effigies[1]);
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");
    let bears = on_battlefield(&engine, kit::P1, kit::bears()).expect("the Bear is out");

    tap_all_mana(&mut engine, kit::P0);
    // Bolt the first Effigy; its trigger goes at the second; the second's goes
    // at the Wall.
    let mut answers = kit::Answers::new()
        .aim(&[first])
        .aim(&[second])
        .aim(&[wall]);
    kit::cast(&mut engine, kit::P0, kit::bolt(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);

    assert_eq!(
        kit::zone_of(&engine, first),
        Zone::Graveyard,
        "the first Effigy died to the Bolt"
    );
    assert_eq!(
        kit::zone_of(&engine, second),
        Zone::Graveyard,
        "its trigger dealt 3 to the second Effigy, a 0/3: it died too, and had its own trigger"
    );
    assert_eq!(
        kit::damage_on(&engine, wall),
        6,
        "the second Effigy's X is 3, plus the 3 the first one dealt to it"
    );
    assert_eq!(
        kit::damage_on(&engine, bears),
        0,
        "the Bear was never a target"
    );
}

/// "… by other sources named Blazing Effigy": any other source does not add.
/// Earth Elemental (4/5) attacks and the Effigy blocks it. The Elemental deals
/// 4 to the 0/3, which dies with 4 damage on it from a source that is not
/// named Blazing Effigy: X stays 3, not 3 + 4. Aimed at the Wall of Stone the
/// trigger leaves exactly 3.
#[test]
fn blazing_effigy_does_not_add_damage_from_a_source_with_another_name() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blazing_effigy()])
        .battlefield(1, &[index::EARTH_ELEMENTAL, kit::wall_of_stone()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let effigy = on_battlefield(&engine, kit::P0, blazing_effigy()).expect("the Effigy is out");
    let elemental =
        on_battlefield(&engine, kit::P1, index::EARTH_ELEMENTAL).expect("the Elemental is out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");

    kit::attack(&mut engine, kit::P1, &[elemental], kit::P0);
    kit::block(&mut engine, kit::P0, &[(effigy, elemental)]);
    let mut answers = kit::Answers::new().aim(&[wall]);
    kit::drive_with(&mut engine, &mut answers, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P1)
    });

    assert!(
        in_graveyard(&engine, kit::P0, blazing_effigy()).is_some(),
        "the Elemental dealt 4 to a 0/3: the Effigy died in combat"
    );
    assert!(
        answers.menus.iter().any(|menu| menu.contains(&wall)),
        "the trigger asked for a target and the Wall was on the menu: {:?}",
        answers.menus
    );
    assert_eq!(
        kit::damage_on(&engine, wall),
        3,
        "X is 3: the 4 damage the Elemental dealt does not count, it is not named \
         Blazing Effigy"
    );
}

/// Ruling (2004-10-04): "If it dies and there are no creatures to target, then
/// the ability does nothing." The Effigy is the only creature in the game and
/// dies to Lightning Bolt: nothing is asked (the trigger has no target, and a
/// player is not a creature), the stack empties, and both life totals stand.
/// The positive control is the first test of this file: the same death with a
/// creature across the table deals its damage.
#[test]
fn blazing_effigy_dying_with_no_creature_to_target_does_nothing() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blazing_effigy(), mountain()])
        .hand(0, &[kit::bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let effigy = on_battlefield(&engine, kit::P0, blazing_effigy()).expect("the Effigy is out");

    tap_all_mana(&mut engine, kit::P0);
    let mut answers = kit::Answers::new().aim(&[effigy]);
    kit::cast(&mut engine, kit::P0, kit::bolt(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);

    assert!(
        in_graveyard(&engine, kit::P0, blazing_effigy()).is_some(),
        "the Effigy died to the Bolt"
    );
    assert_eq!(
        answers.menus.len(),
        1,
        "only the Bolt asked for a target: the trigger had nobody to ask about, \
         got {:?}",
        answers.menus
    );
    assert_eq!(kit::life(&engine, kit::P0), 20, "no damage to p0");
    assert_eq!(kit::life(&engine, kit::P1), 20, "no damage to p1");
}
