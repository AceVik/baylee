//! `cards/instants/mv_2/reap.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn reap() -> CardIndex {
    card_index("7fcb8472-1e77-4dcd-b18f-8abf8806da08")
}

/// `Reap` ({1}{G}, Instant, `Coverage::Partial`):
/// returns up to one target card from your graveyard to your hand.
#[test]
fn reap_returns_target_card_from_your_graveyard_to_your_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), forest(), forest()])
        .hand(0, &[lightning_bolt(), reap()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let bolt = in_graveyard(&engine, p0, lightning_bolt()).unwrap();

    cast_from_hand(&mut engine, p0, reap());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bolt],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, lightning_bolt()).is_none());
    assert!(in_hand(&engine, p0, lightning_bolt()).is_some());
}
