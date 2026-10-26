//! `cards/enchantments/mv_3/citadel_of_pain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn citadel_of_pain_damages_active_player_for_untapped_lands_at_end_step() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[index::CITADEL_OF_PAIN, mountain()])
        .battlefield(0, &[mountain(); 3])
        .battlefield(1, &[forest(); 3])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Citadel of Pain ({2}{R}), then play an untapped Mountain for the turn.
    cast_from_hand(&mut engine, p0, index::CITADEL_OF_PAIN);
    pass_until(&mut engine, stack_is_empty);
    play_land(&mut engine, p0, mountain());

    // Advance to p1's main phase (passing through p0's end step).
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(
        engine.state().players[0].life,
        19,
        "p0 had 1 untapped Mountain at end step and takes 1 damage"
    );

    // Advance to p0's next main phase (passing through p1's end step with 3 untapped Forests).
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "p1 had 3 untapped Forests at end step and takes 3 damage"
    );
}
