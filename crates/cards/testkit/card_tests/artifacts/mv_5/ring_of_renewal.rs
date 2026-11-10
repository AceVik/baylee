//! `cards/artifacts/mv_5/ring_of_renewal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ring_of_renewal() -> CardIndex {
    card_index("45964368-ea9e-415f-9faf-870c7bf550d4")
}

/// "{5}, {T}: Discard a card at random, then draw two cards." With a
/// one-card hand the random discard is that card, so the order is visible:
/// it is gone to the graveyard and two others have come in.
#[test]
fn it_discards_at_random_and_then_draws_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ring_of_renewal(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p0, ring_of_renewal()).unwrap();
    let lib = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ring_of_renewal(), 0);
    assert!(is_tapped(&engine, ring));
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, plains()).is_some(),
        "the only card was discarded"
    );
    assert!(in_hand(&engine, p0, plains()).is_none());
    assert_eq!(library_size(&engine, p0), lib - 2, "two drawn");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "one discarded, two drawn: no choice was asked"
    );
}
