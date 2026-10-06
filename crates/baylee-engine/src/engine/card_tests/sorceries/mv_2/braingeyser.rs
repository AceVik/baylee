//! `cards/sorceries/mv_2/braingeyser.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Braingeyser: "Target player draws X cards."
#[test]
fn braingeyser_draws_x_cards_for_its_target_player() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[braingeyser()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, braingeyser());
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before - 1 + 3,
        "\"draws X cards\" — minus the card itself, cast away"
    );
}
