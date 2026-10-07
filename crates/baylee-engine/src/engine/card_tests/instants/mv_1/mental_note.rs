//! `cards/instants/mv_1/mental_note.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mental Note ({U}, Instant) prints exactly two lines: "Mill two cards" and
/// "Draw a card." Both move the same library into two different zones, so
/// no single length counter can tell them apart — a card that only milled
/// or only drew would satisfy every single delta. Therefore each zone is
/// read and the objects are named: the top two cards from before the spell
/// are now in the graveyard, and the card in hand is exactly the third from
/// the top, which nails down the printed order (first mill, then draw).
#[test]
fn mental_note_mills_the_top_two_and_then_draws_the_next_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[mental_note()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The library is `forest()` all the way down — sixty copies of one
    // printing — so the *objects* are the only thing that can say which card
    // went where.
    let library_before: Vec<ObjectId> =
        engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];
    let third = library_before[library_before.len() - 3];
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, mental_note());
    pass_until(&mut engine, stack_is_empty);

    let graveyard = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    assert!(
        graveyard.contains(&top) && graveyard.contains(&second),
        "\"mill two cards\": the top two of the library, and neither of them \
         is anywhere else: {graveyard:?}"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 3,
        "two milled and one drawn is three cards off the top"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&third),
        "\"draw a card\": the mill moved the first two, so the card that \
         reaches the hand is exactly the third from the top"
    );
    // The hand is the size it was, and that is arithmetic rather than a
    // disappointment: the Note left the hand as the drawn card entered it.
    // The claim that a card was drawn is the identity above — a mill alone
    // would have left the hand one card *shorter*, which is what this
    // number rules out.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card out (the Note) and one card in (the draw)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the spell's {{U}} came out of the pool the Island filled"
    );
    assert!(
        in_graveyard(&engine, p0, mental_note()).is_some(),
        "and the instant itself is in its owner's graveyard once it has \
         resolved"
    );
}
