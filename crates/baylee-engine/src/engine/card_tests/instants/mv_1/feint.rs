//! `cards/instants/mv_1/feint.rs`, played.
//!
//! "Tap all creatures blocking target attacking creature. Prevent all combat
//! damage that would be dealt this turn by that creature and each creature
//! blocking it."
//!
//! Every board here has two attackers, and Feint is aimed at only one: what
//! the second pair does in combat is the proof that the prevention is not a
//! Fog.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use crate::turn::Step;

// oracle_id = "1bb8fe05-abb3-40a8-9e80-5d99ed0e4284"
fn feint() -> CardIndex {
    card_index("1bb8fe05-abb3-40a8-9e80-5d99ed0e4284")
}

/// Casts Feint at `target` as p0 once the blockers are down, off a tapped
/// Mountain, and lets the stack empty; p0 holds priority afterwards, still in
/// the declare blockers step. Returns the menu the target question showed.
#[track_caller]
fn feint_the_attacker(engine: &mut Engine<RegistryLookup>, target: ObjectId) -> Vec<ObjectId> {
    let mut answers = kit::Answers::new().aim(&[target]);
    kit::walk_to_quiet_priority(engine, &mut answers, kit::P0, Step::DeclareBlockers);
    tap_all_mana(engine, kit::P0);
    kit::cast(engine, kit::P0, feint(), &mut answers);
    kit::settle(engine, kit::P0, &mut answers);
    answers
        .menus
        .first()
        .cloned()
        .expect("Feint asked for its target")
}

/// "Tap all creatures blocking target attacking creature. Prevent all combat
/// damage that would be dealt this turn by that creature and each creature
/// blocking it." Hill Giant (3/3) attacks and two Bears block it, a second
/// pair (Savannah Lions and a third Bear) fights beside it. Feint on the Giant
/// taps both of its blockers, and nobody in that fight is hurt: 4 damage from
/// the Bears would have killed the Giant, and the Giant's 3 would have killed
/// one of them. The other pair, which Feint never named, kills each other as
/// they would have: the control that the prevention is limited to the pair.
#[test]
fn feint_taps_the_blockers_and_prevents_combat_damage_by_the_attacker_and_its_blockers() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::hill_giant(), kit::savannah_lions(), mountain()])
        .battlefield(1, &[kit::bears(), kit::bears(), kit::bears()])
        .hand(0, &[feint()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let giant = on_battlefield(&engine, kit::P0, kit::hill_giant()).expect("the Giant is out");
    let lions = on_battlefield(&engine, kit::P0, kit::savannah_lions()).expect("the Lions are out");
    let bears = kit::objects_of(&engine, kit::P1, kit::bears());
    assert_eq!(bears.len(), 3, "three Bears across the table");
    let (first, second, third) = (bears[0], bears[1], bears[2]);

    kit::attack(&mut engine, kit::P0, &[giant, lions], kit::P1);
    kit::block(
        &mut engine,
        kit::P1,
        &[(first, giant), (second, giant), (third, lions)],
    );
    assert!(
        !is_tapped(&engine, first) && !is_tapped(&engine, second),
        "the blockers start the fight untapped"
    );
    feint_the_attacker(&mut engine, giant);
    assert!(
        is_tapped(&engine, first) && is_tapped(&engine, second),
        "\"tap all creatures blocking\" the target: both of its blockers are tapped"
    );
    assert!(
        !is_tapped(&engine, third),
        "and no other creature is: the Bear blocking the Lions is untapped"
    );

    kit::drive(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P0)
    });

    assert!(
        kit::on_the_table(&engine, giant),
        "the Giant took no damage and lives"
    );
    assert_eq!(kit::damage_on(&engine, giant), 0, "none at all");
    for bear in [first, second] {
        assert!(
            kit::on_the_table(&engine, bear),
            "a blocker of the Giant lives"
        );
        assert_eq!(
            kit::damage_on(&engine, bear),
            0,
            "and the Giant's damage was prevented"
        );
    }
    assert_eq!(
        kit::zone_of(&engine, lions),
        Zone::Graveyard,
        "the pair Feint did not name fought: the Bear's 2 killed the 2/1 Lions"
    );
    assert_eq!(
        kit::zone_of(&engine, third),
        Zone::Graveyard,
        "and the Lions' 2 killed that Bear"
    );
    assert_eq!(kit::life(&engine, kit::P1), 20, "no one was unblocked");
}

/// "Target attacking creature": only an attacker is a legal target. The Giant
/// and the Lions attack, a Bear at home on p0's side does not, and p1's Bear
/// blocks: the menu holds both attackers and neither the creature that stayed
/// home nor the blocker. The same menu is the positive control: the attackers
/// are on it.
#[test]
fn feint_offers_only_attacking_creatures() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                kit::hill_giant(),
                kit::savannah_lions(),
                kit::bears(),
                mountain(),
            ],
        )
        .battlefield(1, &[kit::bears()])
        .hand(0, &[feint()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let giant = on_battlefield(&engine, kit::P0, kit::hill_giant()).expect("the Giant is out");
    let lions = on_battlefield(&engine, kit::P0, kit::savannah_lions()).expect("the Lions are out");
    let at_home = on_battlefield(&engine, kit::P0, kit::bears()).expect("the Bear at home");
    let blocker = on_battlefield(&engine, kit::P1, kit::bears()).expect("the blocking Bear");

    kit::attack(&mut engine, kit::P0, &[giant, lions], kit::P1);
    kit::block(&mut engine, kit::P1, &[(blocker, giant)]);
    let menu = feint_the_attacker(&mut engine, giant);

    assert!(
        menu.contains(&giant) && menu.contains(&lions),
        "both attacking creatures are legal targets: {menu:?}"
    );
    assert!(
        !menu.contains(&at_home),
        "a creature that did not attack is not: {menu:?}"
    );
    assert!(
        !menu.contains(&blocker),
        "and neither is the creature blocking: {menu:?}"
    );
}

/// "Prevent all combat damage that would be dealt this turn by that creature"
/// holds for an attacker nobody blocked: with no blocker to tap, Feint on the
/// unblocked Giant still keeps its 3 from the player, while the unblocked Lions
/// beside it deal their 2. p1 ends at 18, which is neither 20 (everything
/// prevented) nor 15 (nothing prevented).
#[test]
fn feint_prevents_the_combat_damage_of_an_unblocked_attacker() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::hill_giant(), kit::savannah_lions(), mountain()])
        .battlefield(1, &[kit::wall_of_wood()])
        .hand(0, &[feint()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let giant = on_battlefield(&engine, kit::P0, kit::hill_giant()).expect("the Giant is out");
    let lions = on_battlefield(&engine, kit::P0, kit::savannah_lions()).expect("the Lions are out");

    kit::attack(&mut engine, kit::P0, &[giant, lions], kit::P1);
    kit::block(&mut engine, kit::P1, &[]);
    feint_the_attacker(&mut engine, giant);
    kit::drive(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P0)
    });

    assert_eq!(
        kit::life(&engine, kit::P1),
        18,
        "the Lions dealt 2; the Giant, named by Feint, dealt nothing"
    );
}
