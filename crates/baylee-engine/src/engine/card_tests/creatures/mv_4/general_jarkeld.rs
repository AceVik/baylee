//! `cards/creatures/mv_4/general_jarkeld.rs`, played.
//!
//! Oracle:
//! - "{T}: Choose two target blocked attacking creatures. If each of those creatures could be blocked by all creatures that the other is blocked by, each creature that's blocking exactly one of those attacking creatures stops blocking it and is blocking the other attacking creature. Activate only during the declare blockers step."

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
        ],
        &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
        &[],
        &[],
    );
    let jarkeld = lk::obj(&engine, P0, general_jarkeld());
    let a1 = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let a2 = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let b1 = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let b2 = lk::obj(&engine, P1, index::LLANOWAR_ELVES);

    assert!(!lk::offered(&engine, general_jarkeld(), P0, 0));
    let _ = lk::attack(&mut engine, &[a1, a2]);
    lk::block(&mut engine, &[(b1, a1), (b2, a2)]);
    assert!(lk::offered(&engine, general_jarkeld(), P0, 0));
    lk::use_ability(&mut engine, P0, general_jarkeld(), 0, &[a1, a2], &[]);
    assert!(lk::tapped(&engine, jarkeld));
}
