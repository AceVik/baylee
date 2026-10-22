//! `cards/instants/mv_2/abeyance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn abeyance() -> CardIndex {
    card_index("6300de53-6e71-4f0e-87e1-b0acd25c59a8")
}

/// `Abeyance` ({1}{W}, Instant, `Coverage::Partial`) targets a player and
/// draws a card.
#[test]
fn abeyance_targets_player_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[abeyance()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, abeyance());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(hand_after, hand_before);
}
