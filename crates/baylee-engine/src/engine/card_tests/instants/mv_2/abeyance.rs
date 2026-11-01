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

/// "Draw a card" is the *caster's*, even with the opponent as the target: the
/// caster's library is a card shorter and the target's is as it was.
#[test]
fn abeyance_draws_for_the_caster_and_not_for_the_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[abeyance()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let (mine, theirs) = (library_size(&engine, p0), library_size(&engine, p1));
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_from_hand(&mut engine, p0, abeyance());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), mine - 1, "the caster drew");
    assert_eq!(library_size(&engine, p1), theirs, "the target did not");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand
    );
}
