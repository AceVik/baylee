//! `cards/lands/utility/mosswort_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mosswort Bridge: "Hideaway 4" / "This land enters tapped." / "{T}: Add {G}." / "{G}, {T}: You may play the exiled card without paying its mana cost if creatures you control have total power 10 or greater."
/// Under `Coverage::Partial`, Hideaway 4 and the conditional free cast are omitted because face-down exile and total-power conditions are inexpressible.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for green mana.
#[test]
fn mosswort_bridge_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(229, forest())
        .hand(0, &[mosswort_bridge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mosswort_bridge());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, mosswort_bridge(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}
