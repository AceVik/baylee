//! `cards/lands/restricted/sliver_hive.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sliver Hive` taps for `{{C}}`, taps for restricted mana of any color spendable only on
/// Sliver spells, and creates a 1/1 Sliver token for `{{5}}, {{T}}` under `Coverage::Implemented`.
/// Activating ability 1 prompts for a color choice and deposits the mana into `pool.restricted()`
/// rather than general available mana.
#[test]
fn sliver_hive_adds_restricted_sliver_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1921, forest()).hand(0, &[sliver_hive()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, sliver_hive());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, sliver_hive(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Green);
    assert!(is_tapped(&engine, land));
}
