//! `cards/instants/mv_2/final_fortune.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::FINAL_FORTUNE;

const P0: PlayerId = PlayerId::new(0);

/// Final Fortune (8d0adc5c-3fdd-4e22-b783-5651f8e57b65) — {R}{R} Instant:
/// "Take an extra turn after this one. At the beginning of that turn's end
/// step, you lose the game."
#[test]
fn final_fortune_grants_caster_an_extra_turn() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[FINAL_FORTUNE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, FINAL_FORTUNE);
    pass_until(&mut engine, stack_is_empty);

    // Advance to turn 2: P0 takes the extra turn instead of P1.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(engine.state().turn.active, P0);
}
