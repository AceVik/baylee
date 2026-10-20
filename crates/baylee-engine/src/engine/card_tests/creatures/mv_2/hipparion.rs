//! `cards/creatures/mv_2/hipparion.rs`, played.
//!
//! Oracle:
//! - "This creature can't block creatures with power 3 or greater unless you pay {1}."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hipparion() -> CardIndex {
    card_index("306fd9d2-5ded-4857-90f4-e62d73c6aed2")
}

#[test]
fn hipparion_casts_and_blocks_in_combat() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[], &[hipparion()]);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(
        &mut engine,
        P1,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    lk::cast_at(&mut engine, P1, hipparion(), &[], &[]);
    let horse = lk::obj(&engine, P1, hipparion());
    assert_eq!(power_of(&engine, horse), Some(1));

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let offer = lk::declare(&mut engine, &[bears]);
    assert!(lk::may_block(&offer, horse, bears));
}
