//! `cards/instants/mv_2/reality_ripple.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "504d5c29-7c37-4c31-8549-2a49eeef74c8"
fn reality_ripple() -> CardIndex {
    card_index("504d5c29-7c37-4c31-8549-2a49eeef74c8")
}

#[test]
fn reality_ripple_phases_out_target_creature() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[reality_ripple()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    lk::cast_at(&mut engine, P0, reality_ripple(), &[bears], &[]);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
}
