//! `cards/lands/school_of_the_unseen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `School of the Unseen` taps for `{{C}}` and filters mana via `{{2}}, {{T}}` to add one mana of any
/// color under `Coverage::Implemented`.
/// Tapping two Forests for generic mana allows ability 1 to be activated, consuming the two mana
/// and adding the chosen color to the mana pool.
#[test]
fn school_of_the_unseen_filters_generic_mana_into_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1922, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[school_of_the_unseen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, school_of_the_unseen());
    assert!(!entered_tapped(&engine, land));

    tap_all_mana_but(&mut engine, p0, Some(school_of_the_unseen()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, school_of_the_unseen(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
