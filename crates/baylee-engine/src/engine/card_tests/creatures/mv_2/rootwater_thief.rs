//! `cards/creatures/mv_2/rootwater_thief.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn rootwater_thief_gains_flying_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[index::ROOTWATER_THIEF, island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let thief = on_battlefield(&engine, p0, index::ROOTWATER_THIEF).expect("Rootwater Thief");
    assert!(!keywords(&engine, thief).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, index::ROOTWATER_THIEF, 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, thief).contains(KeywordSet::FLYING));
}
