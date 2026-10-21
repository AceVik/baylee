//! `cards/artifacts/mv_6/celestial_sword.rs`, played.
//!
//! Oracle:
//! - "{3}, {T}: Target creature you control gets +3/+3 until end of turn. Its controller sacrifices it at the beginning of the next end step."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "8f64c3d4-8ed6-43be-9583-33de4b7761fa"
fn celestial_sword() -> CardIndex {
    card_index("8f64c3d4-8ed6-43be-9583-33de4b7761fa")
}

#[test]
fn celestial_sword_pumps_creature_you_control_then_sacrifices_it_at_end_step() {
    let mut engine = lk::game(
        SEED,
        &[celestial_sword(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 3)]);
    activate(&mut engine, P0, celestial_sword(), 0);
    let options = aim_at(&mut engine, P0, bears);
    assert!(
        !options.contains(&giant),
        "target creature you control: the opponent's Hill Giant is not offered"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears),
        (5, 5),
        "target creature you control gets +3/+3 until end of turn"
    );

    // Advance through the next end step: the delayed trigger sacrifices the creature.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "its controller sacrifices it at the beginning of the next end step"
    );
}
