//! `cards/lands/utility/wintermoon_mesa.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wintermoon Mesa: "This land enters tapped." / "{T}: Add {C}." / "{2}, {T}, Sacrifice this land: Tap two target lands."
/// The two-target sacrifice ability is played in `activation_target_tests`.
/// Playing the land from hand verifies that it enters tapped, and on the next turn it untaps and taps for {C}.
#[test]
fn wintermoon_mesa_enters_tapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(111, forest())
        .hand(0, &[wintermoon_mesa()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mesa = play_land(&mut engine, p0, wintermoon_mesa());
    assert!(
        entered_tapped(&engine, mesa),
        "Wintermoon Mesa enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, mesa), "untaps on next turn");

    activate(&mut engine, p0, wintermoon_mesa(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, mesa));
}
