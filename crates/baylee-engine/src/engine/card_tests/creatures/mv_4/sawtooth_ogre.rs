//! `cards/creatures/mv_4/sawtooth_ogre.rs`, played.
//!
//! The delayed damage reaching "that creature" is the engine fix of
//! `e817e2c39` (`DealDamage` to `EventObject` from a delayed trigger); these
//! tests need it.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sawtooth_ogre() -> CardIndex {
    card_index("473c9f27-2091-4d87-b279-64b3662c9af8")
}

fn horned_turtle() -> CardIndex {
    card_index("7a4ae76b-6241-4fa8-b239-5ec941a0e7df")
}

fn border_guard() -> CardIndex {
    card_index("1ef5003c-f540-4cdc-913f-7d5280ad9f62")
}

/// Sawtooth Ogre (473c9f27-2091-4d87-b279-64b3662c9af8), "becomes blocked by":
/// the 3/3 Ogre attacks and a 1/4 Horned Turtle blocks. Combat damage leaves
/// the Turtle alive with exactly 3 marked (the 1 is not dealt early), and only
/// at end of combat does the Ogre's 1 damage kill it. The Ogre has taken the
/// Turtle's 1 and nothing more, and a Border Guard that did not block is not
/// touched.
#[test]
fn sawtooth_ogre_damages_the_creature_that_blocks_it_at_end_of_combat() {
    let mut e = lk::game(
        SEED,
        &[sawtooth_ogre()],
        &[horned_turtle(), border_guard()],
        &[],
        &[],
    );
    let ogre = lk::obj(&e, P0, sawtooth_ogre());
    let turtle = lk::obj(&e, P1, horned_turtle());
    let guard = lk::obj(&e, P1, border_guard());

    lk::attack(&mut e, &[ogre]);
    lk::block(&mut e, &[(turtle, ogre)]);
    lk::to_step(&mut e, P0, Step::CombatDamage);
    assert_eq!(
        lk::damage_on(&e, turtle),
        3,
        "only the Ogre's combat damage so far"
    );
    assert!(
        on_battlefield(&e, P1, horned_turtle()).is_some(),
        "3 damage on a 1/4 is not lethal"
    );

    lk::to_step(&mut e, P0, Step::CombatEnd);
    lk::settle(&mut e);

    assert!(
        in_graveyard(&e, P1, horned_turtle()).is_some(),
        "the 1 damage at end of combat is the fourth"
    );
    assert_eq!(
        lk::damage_on(&e, ogre),
        1,
        "the Ogre took the Turtle's 1 damage only"
    );
    assert!(on_battlefield(&e, P0, sawtooth_ogre()).is_some());
    assert_eq!(
        lk::damage_on(&e, guard),
        0,
        "a creature that did not block is untouched"
    );
}

/// Sawtooth Ogre, "blocks": on the opponent's turn the Ogre blocks the attacking
/// Turtle and the same end-of-combat damage finishes it (3 combat + 1).
#[test]
fn sawtooth_ogre_damages_the_creature_it_blocks_at_end_of_combat() {
    let mut e = lk::game(SEED, &[sawtooth_ogre()], &[horned_turtle()], &[], &[]);
    let ogre = lk::obj(&e, P0, sawtooth_ogre());
    let turtle = lk::obj(&e, P1, horned_turtle());

    // Player 0 does not attack on their own turn.
    lk::attackers_offered(&mut e);
    e.apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    lk::attack(&mut e, &[turtle]);
    lk::block(&mut e, &[(ogre, turtle)]);
    lk::to_step(&mut e, P1, Step::CombatDamage);
    assert_eq!(lk::damage_on(&e, turtle), 3);
    assert!(
        on_battlefield(&e, P1, horned_turtle()).is_some(),
        "still alive after combat damage"
    );

    lk::to_step(&mut e, P1, Step::CombatEnd);
    lk::settle(&mut e);

    assert!(
        in_graveyard(&e, P1, horned_turtle()).is_some(),
        "dead at end of combat"
    );
    assert_eq!(lk::damage_on(&e, ogre), 1);
}

/// Sawtooth Ogre — an unblocked Ogre has no creature to hurt: the Border Guard
/// stays at 0 damage and the player takes 3.
#[test]
fn sawtooth_ogre_deals_no_extra_damage_when_unblocked() {
    let mut e = lk::game(SEED, &[sawtooth_ogre()], &[border_guard()], &[], &[]);
    let ogre = lk::obj(&e, P0, sawtooth_ogre());
    let guard = lk::obj(&e, P1, border_guard());

    lk::attack(&mut e, &[ogre]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    assert_eq!(lk::damage_on(&e, guard), 0);
    assert_eq!(lk::life(&e, P1), 17, "3 combat damage to the player");
}
