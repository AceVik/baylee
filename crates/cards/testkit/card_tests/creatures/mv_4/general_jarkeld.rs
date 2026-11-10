//! `cards/creatures/mv_4/general_jarkeld.rs`, played.
//!
//! Oracle:
//! - "{T}: Choose two target blocked attacking creatures. If each of those creatures could be blocked by all creatures that the other is blocked by, each creature that's blocking exactly one of those attacking creatures stops blocking it and is blocking the other attacking creature. Activate only during the declare blockers step."
//!
//! The activation's timing and target restrictions are played through, as are
//! a legal blocker exchange and the atomic refusal of an illegal exchange.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn general_jarkeld() -> CardIndex {
    card_index("64df77f6-4f7e-442a-971c-8ad3f7d1c979")
}

#[test]
fn general_jarkeld_exchanges_blockers_during_declare_blockers() {
    let mut engine = lk::game(
        SEED,
        &[
            general_jarkeld(),
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::HILL_GIANT,
        ],
        &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
        &[],
        &[],
    );
    let jarkeld = lk::obj(&engine, P0, general_jarkeld());
    let a1 = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let a2 = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let unblocked = lk::obj(&engine, P0, index::HILL_GIANT);
    let b1 = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let b2 = lk::obj(&engine, P1, index::LLANOWAR_ELVES);

    assert!(!lk::offered(&engine, general_jarkeld(), P0, 0));
    let _ = lk::attack(&mut engine, &[a1, a2, unblocked]);
    lk::block(&mut engine, &[(b1, a1), (b2, a2)]);
    assert!(lk::offered(&engine, general_jarkeld(), P0, 0));
    activate(&mut engine, P0, general_jarkeld(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&a1) && options.contains(&a2));
    assert!(
        !options.contains(&unblocked),
        "an unblocked attacker is not a blocked attacking creature"
    );
    assert!(
        !options.contains(&b1) && !options.contains(&b2),
        "a blocker is not an attacking creature"
    );
    lk::aim(&mut engine, &[a1, a2], &[]);
    lk::settle(&mut engine);
    assert!(lk::tapped(&engine, jarkeld));
    assert_eq!(engine.state().combat.blocked_by(b1), vec![a2]);
    assert_eq!(engine.state().combat.blocked_by(b2), vec![a1]);

    lk::through_combat(&mut engine);
    assert_eq!(lk::zone_of(&engine, a2), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, b2), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, a1), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, b1), Zone::Battlefield);
    assert_eq!(engine.state().players[1].life, 17);
}

/// A ground blocker cannot take over a block on a flying attacker. That one
/// illegal resulting pair prevents the entire exchange, including the legal half.
#[test]
fn general_jarkeld_leaves_every_block_unchanged_if_either_exchange_is_illegal() {
    let mut engine = lk::game(
        SEED,
        &[general_jarkeld(), index::GRIZZLY_BEARS, index::WIND_DRAKE],
        &[index::GRIZZLY_BEARS, index::WIND_DRAKE],
        &[],
        &[],
    );
    let ground_attacker = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let flying_attacker = lk::obj(&engine, P0, index::WIND_DRAKE);
    let ground_blocker = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let flying_blocker = lk::obj(&engine, P1, index::WIND_DRAKE);

    let _ = lk::attack(&mut engine, &[ground_attacker, flying_attacker]);
    lk::block(
        &mut engine,
        &[
            (ground_blocker, ground_attacker),
            (flying_blocker, flying_attacker),
        ],
    );
    activate(&mut engine, P0, general_jarkeld(), 0);
    lk::aim(&mut engine, &[ground_attacker, flying_attacker], &[]);
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().combat.blocked_by(ground_blocker),
        vec![ground_attacker],
        "the otherwise legal half does not happen on its own"
    );
    assert_eq!(
        engine.state().combat.blocked_by(flying_blocker),
        vec![flying_attacker],
        "the ground Bear cannot be moved onto the flying attacker"
    );
}

/// Every creature that blocks exactly one of the two attackers switches, so a
/// double block moves as a pair and the other attacker's lone blocker moves
/// the other way.
#[test]
fn general_jarkeld_moves_every_blocker_of_a_double_block() {
    let mut engine = lk::game(
        SEED,
        &[
            general_jarkeld(),
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
        ],
        &[
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::HILL_GIANT,
        ],
        &[],
        &[],
    );
    let a1 = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let a2 = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let b1 = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let b2 = lk::obj(&engine, P1, index::LLANOWAR_ELVES);
    let b3 = lk::obj(&engine, P1, index::HILL_GIANT);

    let _ = lk::attack(&mut engine, &[a1, a2]);
    lk::block(&mut engine, &[(b1, a1), (b2, a1), (b3, a2)]);
    activate(&mut engine, P0, general_jarkeld(), 0);
    lk::aim(&mut engine, &[a1, a2], &[]);
    lk::settle(&mut engine);

    assert_eq!(engine.state().combat.blocked_by(b1), vec![a2]);
    assert_eq!(engine.state().combat.blocked_by(b2), vec![a2]);
    assert_eq!(engine.state().combat.blocked_by(b3), vec![a1]);
}
