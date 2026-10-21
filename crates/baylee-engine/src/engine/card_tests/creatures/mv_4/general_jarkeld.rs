//! `cards/creatures/mv_4/general_jarkeld.rs`, played.
//!
//! Oracle:
//! - "{T}: Choose two target blocked attacking creatures. If each of those creatures could be blocked by all creatures that the other is blocked by, each creature that's blocking exactly one of those attacking creatures stops blocking it and is blocking the other attacking creature. Activate only during the declare blockers step."
//!
//! The card is `Coverage::Partial`: the reblock itself is not played. What
//! is played is the activation — its timing and its two targets — and the
//! last assertion pins that nothing moves yet, so the day the reblock lands
//! this test has to change with it.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn general_jarkeld() -> CardIndex {
    card_index("64df77f6-4f7e-442a-971c-8ad3f7d1c979")
}

#[test]
fn general_jarkeld_activates_only_during_declare_blockers_on_two_blocked_attackers() {
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
    // The gap the card's `Partial` names: no blocker has moved.
    assert_eq!(engine.state().combat.blocked_by(b1), vec![a1]);
    assert_eq!(engine.state().combat.blocked_by(b2), vec![a2]);
}
