//! `cards/lands/cycling/sheltered_thicket.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sheltered Thicket prints three lines: "This land enters tapped", "Cycling
/// {2}" and, because it is a Mountain and a Forest, "{T}: Add {R} or {G}".
/// The card is played from hand and not put onto the battlefield —
/// `starting_battlefield` is a placement without entering (CR 614.1c),
/// which would not demonstrate the tapped arrival at all —, while the
/// second hand copy is cycled: only that way does "Discard this card" lie
/// in the graveyard and the land next to it remains on the battlefield.
/// The mana line is read on the put-onto-the-battlefield copy, which
/// still stands because a land that entered tapped cannot tap for anything
/// in its turn of arrival.
#[test]
fn sheltered_thicket_enters_tapped_cycles_for_two_and_offers_red_or_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), sheltered_thicket()])
        .hand(0, &[sheltered_thicket(), sheltered_thicket()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Played from hand: the one copy enters tapped.
    let played = play_land(&mut engine, p0, sheltered_thicket());
    assert!(
        entered_tapped(&engine, played),
        "\"This land enters tapped.\""
    );
    assert!(
        types(&engine, played).contains(TypeSet::LAND),
        "und es ist als Land angekommen: {:?}",
        types(&engine, played)
    );

    // Cycling {2} from hand: the two Forests pay, the second copy is the
    // card that goes, and "Draw a card" brings exactly one back.
    let library_before = library_size(&engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(sheltered_thicket()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, two mana, and both Thickets remained standing"
    );
    activate(&mut engine, p0, sheltered_thicket(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, sheltered_thicket()).is_some(),
        "\"Discard this card\": the cycled hand copy lies in the graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, sheltered_thicket()).len(),
        2,
        "the put-onto-the-battlefield and the played copy remain — the \
         graveyard got the hand card and not the land"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\""
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came from the Pool"
    );

    // The mana line on the copy that is still untapped, with an empty pool.
    let standing = all_on_battlefield(&engine, p0, sheltered_thicket())
        .into_iter()
        .find(|id| !is_tapped(&engine, *id))
        .expect("die hingesetzte Kopie steht noch");
    activate(&mut engine, p0, sheltered_thicket(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"{{T}}: Add {{R}} or {{G}}\" ist eine Frage mit zwei Feldern, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that taps names the colour");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "\"{{R}} or {{G}}\": {options:?}"
    );
    assert_eq!(options.len(), 2, "and no third: {options:?}");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("Grün war eines der angebotenen Felder");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "the named colour");
    assert_eq!(pool.total(), 1, "ein Mana, ein Tap, und sonst nichts");
    assert!(is_tapped(&engine, standing), "the {{T}} was the price");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability does not use the stack"
    );
}
