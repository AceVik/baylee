//! `cards/creatures/mv_3/blood_hound.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "ace532a3-fa6e-479d-b169-1fb24c3d0f8c"
fn blood_hound() -> CardIndex {
    card_index("ace532a3-fa6e-479d-b169-1fb24c3d0f8c")
}

/// Blood Hound ({2}{R} 1/1 Creature — Dog):
/// "Whenever you're dealt damage, you may put that many +1/+1 counters on this creature."
/// "At the beginning of your end step, remove all +1/+1 counters from this creature."
#[test]
fn blood_hound_gains_counters_when_controller_is_dealt_damage_and_loses_them_at_end_step() {
    let mut engine = lk::game(SEED, &[blood_hound()], &[], &[index::LIGHTNING_BOLT], &[]);
    let hound = lk::obj(&engine, P0, blood_hound());
    assert_eq!(pt(&engine, hound), (1, 1));

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[], &[P0]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 17);
    assert_eq!(
        pt(&engine, hound),
        (4, 4),
        "3 damage dealt to P0 put three +1/+1 counters on Blood Hound"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        pt(&engine, hound),
        (1, 1),
        "at the beginning of P0's end step, all +1/+1 counters were removed from Blood Hound"
    );
}
