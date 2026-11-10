//! `cards/lands/restricted/leechridden_swamp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Leechridden Swamp: "Leechridden Swamp enters tapped." / "{B}, {T}: Each opponent loses 1 life. Activate only if you control two or more black permanents."
/// Under `Coverage::Implemented`, playing this land enters tapped.
/// With two black permanents in play, paying `{B}` and tapping drains 1 life from the opponent.
#[test]
fn leechridden_swamp_enters_tapped_and_drains_opponent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(106, forest())
        .battlefield(0, &[baleful_strix(), baleful_strix(), swamp()])
        .hand(0, &[leechridden_swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, leechridden_swamp());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    let p1_before = engine.state().players[1].life;
    tap_all_mana_but(&mut engine, p0, Some(leechridden_swamp()));
    activate(&mut engine, p0, leechridden_swamp(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, p1_before - 1);
    assert!(is_tapped(&engine, land));
}
