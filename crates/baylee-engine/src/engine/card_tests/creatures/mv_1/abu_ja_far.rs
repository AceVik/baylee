//! `cards/creatures/mv_1/abu_ja_far.rs`, played.
//!
//! "When this creature dies, destroy all creatures blocking or blocked by it.
//! They can't be regenerated." A 0/1, so it never kills anything by itself in
//! combat: every creature that is gone afterwards was destroyed by the trigger,
//! which is what makes the boards below readable.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use crate::turn::Step;

// oracle_id = "a2404d88-0621-49ae-9908-052c23a96ac6"
fn abu_ja_far() -> CardIndex {
    card_index("a2404d88-0621-49ae-9908-052c23a96ac6")
}

/// "Destroy all creatures blocking … it": Abu Ja'far attacks and two Bears
/// block it. Its 0 power assigns no combat damage (CR 510.1a), so the Bears
/// leave the table only through the trigger, which reads the creatures
/// blocking it as it last existed on the battlefield (CR 603.10a, CR 608.2h) —
/// it is out of combat by then. A third Bear that was never in combat with it
/// is the control: "all creatures blocking or blocked by it" does not mean
/// all creatures.
#[test]
fn abu_ja_far_dying_in_combat_destroys_every_creature_blocking_it_and_no_other() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[abu_ja_far()])
        .battlefield(1, &[kit::bears(), kit::bears(), kit::bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let abu = on_battlefield(&engine, kit::P0, abu_ja_far()).expect("Abu Ja'far is out");
    let bears = kit::objects_of(&engine, kit::P1, kit::bears());
    assert_eq!(bears.len(), 3, "three Bears across the table");
    let (first_blocker, second_blocker, bystander) = (bears[0], bears[1], bears[2]);

    kit::attack(&mut engine, kit::P0, &[abu], kit::P1);
    kit::block(
        &mut engine,
        kit::P1,
        &[(first_blocker, abu), (second_blocker, abu)],
    );
    kit::drive(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P0)
    });

    assert!(
        in_graveyard(&engine, kit::P0, abu_ja_far()).is_some(),
        "two Bears dealt 4 to a creature with toughness 1: Abu Ja'far died"
    );
    assert_eq!(
        kit::zone_of(&engine, first_blocker),
        Zone::Graveyard,
        "the first creature blocking it is destroyed"
    );
    assert_eq!(
        kit::zone_of(&engine, second_blocker),
        Zone::Graveyard,
        "and so is the second: \"all creatures\" blocking it, not one of them"
    );
    assert!(
        kit::on_the_table(&engine, bystander),
        "the Bear that was never in combat with it is still on the table"
    );
    assert_eq!(
        kit::damage_on(&engine, bystander),
        0,
        "and was not touched either"
    );
}

/// "… or blocked by it": Abu Ja'far blocks a Bear. The Bear deals 2 to the
/// 0/1 and Abu Ja'far, which dealt nothing, dies; the creature it was blocking
/// is then destroyed by the trigger. A second attacker it was not blocking
/// survives, and its damage is dealt to the player: the trigger takes the one
/// creature in combat with it and not the rest of the attack.
#[test]
fn abu_ja_far_dying_while_blocking_destroys_the_creature_it_blocked() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[abu_ja_far()])
        .battlefield(1, &[kit::bears(), kit::bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let abu = on_battlefield(&engine, kit::P0, abu_ja_far()).expect("Abu Ja'far is out");
    let bears = kit::objects_of(&engine, kit::P1, kit::bears());
    assert_eq!(bears.len(), 2, "two Bears to attack with");
    let (blocked, unblocked) = (bears[0], bears[1]);

    kit::attack(&mut engine, kit::P1, &[blocked, unblocked], kit::P0);
    kit::block(&mut engine, kit::P0, &[(abu, blocked)]);
    kit::drive(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P1)
    });

    assert!(
        in_graveyard(&engine, kit::P0, abu_ja_far()).is_some(),
        "the Bear it blocked dealt 2 to a creature with toughness 1"
    );
    assert_eq!(
        kit::zone_of(&engine, blocked),
        Zone::Graveyard,
        "the creature Abu Ja'far was blocking is destroyed — \"blocked by it\" \
         reads the block it was in, though Abu Ja'far dealt it no damage"
    );
    assert!(
        kit::on_the_table(&engine, unblocked),
        "the attacker it was not in combat with survives"
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        18,
        "and was unblocked, so its 2 combat damage reached the player: the \
         attack went on around the trigger"
    );
}

/// "When this creature dies" — dies, wherever. Bolted in its owner's own main
/// phase, with two Bears across the table that are not in combat with
/// anything, it destroys nothing: the creatures it destroys are the ones in
/// combat with it, and a Bear nobody is blocking is neither. The sibling tests
/// above are the positive control: the same trigger destroys exactly the
/// creatures in combat with it.
#[test]
fn abu_ja_far_dying_outside_combat_destroys_nothing_else() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[abu_ja_far(), mountain()])
        .battlefield(1, &[kit::bears(), kit::bears()])
        .hand(0, &[kit::bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let abu = on_battlefield(&engine, kit::P0, abu_ja_far()).expect("Abu Ja'far is out");
    let bears = kit::objects_of(&engine, kit::P1, kit::bears());
    assert_eq!(bears.len(), 2, "two Bears across the table");

    tap_all_mana(&mut engine, kit::P0);
    let mut answers = kit::Answers::new().aim(&[abu]);
    kit::cast(&mut engine, kit::P0, kit::bolt(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);

    assert!(
        in_graveyard(&engine, kit::P0, abu_ja_far()).is_some(),
        "Lightning Bolt dealt 3 to a creature with toughness 1: Abu Ja'far died"
    );
    for bear in &bears {
        assert!(
            kit::on_the_table(&engine, *bear),
            "a Bear that was neither blocking nor blocked by it is untouched"
        );
    }
}

/// "They can't be regenerated." Two Lotleth Trolls each buy a regeneration
/// shield for `{B}` after blockers are declared. The first blocks Abu Ja'far;
/// the second blocks a Bear, which deals it lethal damage. Combat damage kills
/// Abu Ja'far and the second Troll at the same moment: the second Troll's
/// shield saves it (CR 701.19a, tapped and removed from combat), and then the
/// trigger destroys the first Troll through its identical shield (CR 701.19c:
/// the shield is simply not applied). One combat, one kind of shield, two
/// outcomes: the saved Troll is the positive control that the shield works.
#[test]
fn abu_ja_far_destroys_its_blockers_through_a_regeneration_shield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[abu_ja_far(), kit::bears()])
        .battlefield(1, &[kit::troll(), kit::troll(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let abu = on_battlefield(&engine, kit::P0, abu_ja_far()).expect("Abu Ja'far is out");
    let bear = on_battlefield(&engine, kit::P0, kit::bears()).expect("the Bear is out");
    let trolls = kit::objects_of(&engine, kit::P1, kit::troll());
    assert_eq!(trolls.len(), 2, "two Trolls");
    let (abu_s_blocker, bear_s_blocker) = (trolls[0], trolls[1]);

    kit::attack(&mut engine, kit::P0, &[abu, bear], kit::P1);
    kit::block(
        &mut engine,
        kit::P1,
        &[(abu_s_blocker, abu), (bear_s_blocker, bear)],
    );

    // The defender holds priority once the attacker has passed it.
    let mut answers = kit::Answers::new().shield(true);
    kit::walk_to_quiet_priority(&mut engine, &mut answers, kit::P1, Step::DeclareBlockers);
    tap_all_mana(&mut engine, kit::P1);
    raise_a_shield(&mut engine, kit::P1, abu_s_blocker, kit::TROLL_REGENERATES);
    raise_a_shield(&mut engine, kit::P1, bear_s_blocker, kit::TROLL_REGENERATES);

    kit::drive_with(&mut engine, &mut answers, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P0)
    });

    assert!(
        in_graveyard(&engine, kit::P0, abu_ja_far()).is_some(),
        "the Troll blocking it dealt 2 to a creature with toughness 1"
    );
    assert!(
        kit::on_the_table(&engine, bear_s_blocker),
        "positive control: the Troll that took lethal damage from the Bear was \
         saved by its shield — shields work in this combat"
    );
    assert!(
        is_tapped(&engine, bear_s_blocker),
        "and a regenerated creature is tapped (CR 701.19a)"
    );
    assert_eq!(
        kit::zone_of(&engine, abu_s_blocker),
        Zone::Graveyard,
        "the Troll blocking Abu Ja'far stood behind an identical shield and is \
         destroyed anyway: \"they can't be regenerated\""
    );
}

/// "Destroy all creatures blocking or blocked by it": a creature that blocks
/// the *same attacker* as Abu Ja'far is neither. Abu Ja'far and a Wall of Wood
/// both block a Bear; Lightning Bolt kills Abu Ja'far before damage. The Bear,
/// which Abu Ja'far was blocking, is destroyed by the trigger (the positive
/// control that it fired); the Wall, which only stands beside it in the same
/// block, is not.
#[test]
fn abu_ja_far_dying_leaves_a_creature_that_blocked_the_same_attacker_alone() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[abu_ja_far(), kit::wall_of_wood(), mountain()])
        .battlefield(1, &[kit::bears()])
        .hand(0, &[kit::bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let abu = on_battlefield(&engine, kit::P0, abu_ja_far()).expect("Abu Ja'far is out");
    let wall = on_battlefield(&engine, kit::P0, kit::wall_of_wood()).expect("the Wall is out");
    let bear = on_battlefield(&engine, kit::P1, kit::bears()).expect("the Bear is out");

    kit::attack(&mut engine, kit::P1, &[bear], kit::P0);
    kit::block(&mut engine, kit::P0, &[(abu, bear), (wall, bear)]);
    let mut answers = kit::Answers::new().aim(&[abu]);
    kit::walk_to_quiet_priority(&mut engine, &mut answers, kit::P0, Step::DeclareBlockers);
    tap_all_mana(&mut engine, kit::P0);
    kit::cast(&mut engine, kit::P0, kit::bolt(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    kit::walk_to_second_main(&mut engine, &mut answers, kit::P0);

    assert!(
        in_graveyard(&engine, kit::P0, abu_ja_far()).is_some(),
        "Lightning Bolt killed Abu Ja'far while it was blocking"
    );
    assert_eq!(
        kit::zone_of(&engine, bear),
        Zone::Graveyard,
        "the creature it was blocking is destroyed by the trigger"
    );
    assert!(
        kit::on_the_table(&engine, wall),
        "the creature that blocked the same attacker is neither blocking nor blocked \
         by Abu Ja'far, and lives"
    );
}
