//! `cards/lands/check/glacial_fortress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Glacial Fortress` enters tapped unless its controller controls a Plains or an Island,
/// and taps for `{{W}}` or `{{U}}` under `Coverage::Implemented`.
/// Controlling a Plains allows it to enter untapped and immediately produce blue mana through its choice.
#[test]
fn glacial_fortress_enters_untapped_with_plains_and_taps_for_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1902, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[glacial_fortress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, glacial_fortress());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, glacial_fortress(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    assert!(is_tapped(&engine, land));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
}
