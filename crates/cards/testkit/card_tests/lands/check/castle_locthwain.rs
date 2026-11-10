//! `cards/lands/check/castle_locthwain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Castle Locthwain prints three sentences: it enters tapped unless you
/// control a Swamp, it taps for {B}, and {1}{B}{B} plus its own tap draws a
/// card and then charges life equal to the cards in your hand. The entry is
/// played twice over — once with only the opponent's Swamp standing, once
/// with Swamps of p0's own — because "you control" is the word the second
/// run is for and a land that came in tapped for an unrelated reason could
/// not tell the two apart. The ability is then played off the untapped copy,
/// and the life is read *after* the draw: hand size as the second sentence
/// finds it, which is the only number that shows the two effects resolve in
/// the order the card prints them.
#[test]
fn castle_locthwain_needs_a_swamp_of_your_own_to_enter_untapped_and_pays_hand_size_for_a_card() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);

    // First run: the only Swamp on the table belongs to the opponent.
    let mut theirs = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .battlefield(1, &[swamp()])
        .hand(0, &[castle_locthwain()])
        .start();
    keep_mulligans(&mut theirs);
    assert!(walk_to_own_main(&mut theirs, p0), "p0 reaches its own main");
    let across = play_land(&mut theirs, p0, castle_locthwain());
    assert!(
        entered_tapped(&theirs, across),
        "\"unless you control a Swamp\" — the Swamp across the table is not one"
    );

    // Second run: three Swamps of p0's own, so the Castle arrives untapped
    // with its {T} still available for the ability that wants it.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[castle_locthwain()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let castle = play_land(&mut engine, p0, castle_locthwain());
    assert!(
        !entered_tapped(&engine, castle),
        "a Swamp under your own control is the whole of the condition"
    );

    // {1}{B}{B} out of the three Swamps, with the Castle named so that
    // `tap_all_mana` leaves it standing — its own tap is half the price.
    tap_all_mana_but(&mut engine, p0, Some(castle_locthwain()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three black, and the Castle itself kept out of the pool"
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);

    // Ability 0 is the printed {T}: Add {B}; the draw is ability 1.
    activate(&mut engine, p0, castle_locthwain(), 1);
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(hand_after, hand_before + 1, "\"draw a card\"");
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the card came off the top of the library"
    );
    assert_eq!(
        engine.state().players[0].life,
        20 - hand_after as i32,
        "\"then you lose life equal to the number of cards in your hand\": the \
         hand as the draw left it, never the hand that was there when the \
         ability went on the stack"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the controller, and the seat across the table pays nothing"
    );
    assert!(is_tapped(&engine, castle), "the ability's {{T}} was paid");
}
