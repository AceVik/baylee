//! `cards/instants/mv_3/backlash.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::event::{DamageTarget, GameEvent};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "3db9bf7b-da4e-428f-aa5d-a0cd1afeb16c"
fn backlash() -> CardIndex {
    card_index("3db9bf7b-da4e-428f-aa5d-a0cd1afeb16c")
}

/// Backlash taps the target, then that creature — not Backlash — deals
/// damage equal to its power to its current controller (CR 608.2b).
#[test]
fn backlash_taps_the_target_which_damages_its_controller() {
    let mut engine = lk::game(
        SEED,
        &[index::SWAMP, index::SWAMP, index::MOUNTAIN],
        &[index::HILL_GIANT],
        &[backlash()],
        &[],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    cast_from_hand(&mut engine, P0, backlash());
    lk::aim(&mut engine, &[giant], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, giant));
    assert_eq!(lk::life(&engine, P1), 17);
    assert!(engine.journal().entries().iter().any(|entry| matches!(
        entry.event,
        GameEvent::DamageDealt {
            source: Some(source),
            target: DamageTarget::Player(P1),
            amount: 3,
            is_combat: false,
        } if source == giant
    )));
}
