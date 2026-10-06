//! `cards/lands/caves/hidden_cataract.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hidden Cataract: "This land enters tapped." / "{T}: Add {U}." / "{4}{U}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery."
/// Under `Coverage::Partial`, the discover ability is omitted because no effect performs discover.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for blue mana.
#[test]
fn hidden_cataract_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(201, forest())
        .hand(0, &[hidden_cataract()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hidden_cataract());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, hidden_cataract(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}
