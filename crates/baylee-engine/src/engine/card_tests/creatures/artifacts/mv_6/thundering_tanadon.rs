//! `cards/creatures/artifacts/mv_6/thundering_tanadon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thundering_tanadon() -> CardIndex {
    card_index("487382ea-0050-4a3f-bcf6-0ce41d6f4ebf")
}

/// Thundering Tanadon — {4}{G/P}{G/P}, 5/4 trample. With four Forests'
/// worth of mana only the generic cost is covered, so both Phyrexian symbols
/// are paid with 2 life each (CR 107.4f).
#[test]
fn thundering_tanadon_is_paid_in_life_for_its_phyrexian_symbols() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[thundering_tanadon()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, thundering_tanadon());
    while matches!(engine.pending(), Pending::YesNo { .. }) {
        engine
            .apply(p0, PlayerAction::YesNo(true))
            .expect("pay life");
    }
    pass_until(&mut engine, stack_is_empty);
    let tanadon = on_battlefield(&engine, p0, thundering_tanadon()).expect("resolved");
    assert_eq!(pt(&engine, tanadon), (5, 4));
    assert!(is_artifact_creature(&engine, tanadon));
    assert_eq!(life_of(&engine, 0), 16, "2 life for each {{G/P}}");
}

#[test]
fn thundering_tanadon_tramples_over_a_blocker() {
    tramples_over_a_blocker(thundering_tanadon(), 5);
}
