//! `cards/enchantments/auras/mv_1/mind_harness.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "bba30360-c54a-4906-8b2e-84bb1209c43d"
fn mind_harness() -> CardIndex {
    card_index("bba30360-c54a-4906-8b2e-84bb1209c43d")
}

#[test]
fn mind_harness_gains_control_of_red_or_green_creature() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[mind_harness()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, mind_harness());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bears).unwrap().controller, P0);
}
