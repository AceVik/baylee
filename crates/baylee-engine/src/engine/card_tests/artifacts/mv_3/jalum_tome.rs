//! `cards/artifacts/mv_3/jalum_tome.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jalum Tome: "{2}, {T}: Draw a card, then discard a card."
///
/// The order is the rules point — the 2020-11-10 ruling: "You draw a card
/// and discard a card all while Jalum Tome's ability is resolving." The one
/// card in hand is put on top of the library first, so the draw is the only
/// thing that can put a card in the hand, and the discard that follows
/// (CR 608.2c: the instructions are followed in order) is forced to name
/// exactly the card just drawn.
#[test]
fn jalum_tome_draws_before_it_makes_you_discard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jalum_tome(), island(), island()])
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tome = on_battlefield(&engine, p0, jalum_tome()).expect("the Tome is out");
    let drawn = hand_to_library_top(&mut engine, p0, giant_growth());
    let library = library_size(&engine, p0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        0,
        "the only card is on the library now"
    );

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, jalum_tome(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the discard")
    };
    assert_eq!(player, p0, "the drawing player discards");
    assert_eq!(library_size(&engine, p0), library - 1, "the draw ran first");
    assert_eq!(min, 1);
    assert_eq!(max, 1, "one card, and no choice of how many");
    assert_eq!(
        options,
        vec![drawn],
        "the drawn card is the only one the discard can name"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![drawn],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, tome), "the {{T}} was paid");
    assert!(
        in_graveyard(&engine, p0, giant_growth()).is_some(),
        "drawn, then discarded"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} was spent"
    );
}
