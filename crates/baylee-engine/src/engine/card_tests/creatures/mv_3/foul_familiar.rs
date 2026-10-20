//! `cards/creatures/mv_3/foul_familiar.rs`, played.
//!
//! Oracle:
//! - "This creature can't block."
//! - "{B}, Pay 1 life: Return this creature to its owner's hand."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn foul_familiar() -> CardIndex {
    card_index("90767f52-fa6d-4ef3-ae54-d8e3881971c1")
}

#[test]
fn foul_familiar_cant_block_and_returns_to_hand_for_b_and_1_life() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[foul_familiar()], &[], &[]);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let familiar = lk::obj(&engine, P1, foul_familiar());

    let offer = lk::attack(&mut engine, &[bears]);
    assert!(
        !lk::may_block(&offer, familiar, bears),
        "Foul Familiar cannot block"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    lk::pass_once(&mut engine);
    lk::float(&mut engine, P1, &[(ManaColor::Black, 1)]);
    activate(&mut engine, P1, foul_familiar(), 0);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P1), 17);
    assert!(in_hand(&engine, P1, foul_familiar()).is_some());
}
