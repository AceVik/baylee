//! `cards/sorceries/mv_3/doomsday.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::DOOMSDAY;

/// `Doomsday` ({B}{B}{B}, Sorcery) causes you to lose half your life, rounded
/// up.
#[test]
fn doomsday_causes_you_to_lose_half_your_life_rounded_up() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[DOOMSDAY])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, DOOMSDAY);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 10);
}
