//! `cards/lands/tapland/llanowar_reborn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Llanowar Reborn prints `This land enters tapped`, `{T}: Add {G}`, and `Graft 1`.
/// The card is marked `Coverage::Partial` because moving a +1/+1 counter from a land onto an entering creature is unsupported.
/// When played, Llanowar Reborn enters tapped with one `CounterKind::P1P1` counter, untaps on the subsequent
/// turn cycle, and taps for one green mana at ability index 0.
#[test]
fn llanowar_reborn_enters_tapped_with_counter_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[llanowar_reborn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, llanowar_reborn());
    assert!(entered_tapped(&engine, land));
    assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 1);

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, llanowar_reborn(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}
