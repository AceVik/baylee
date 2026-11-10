//! `cards/creatures/mv_3/phyrexian_rager.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Rager prints one line — "When this creature enters, you draw a
/// card and you lose 1 life" — and both halves have to be read off the board
/// rather than off the card file. The draw is asserted as the *named* top card
/// of the library arriving in hand, so an effect that merely emptied the
/// library would not pass; the loss is read against both seats, so "you" is
/// checked and not assumed. Three Swamps pay {2}{B} and are named nowhere else,
/// which leaves the life total the only thing on this board that can move.
#[test]
fn phyrexian_rager_draws_a_card_and_costs_its_controller_one_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(211, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[phyrexian_rager()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Named before the cast: the card the enters-trigger is about to draw is
    // the one on top now, and nothing between here and the assertion touches
    // the library. `list` runs bottom-first, so the top is the last entry.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, phyrexian_rager());
    pass_until(&mut engine, stack_is_empty);

    let rager = on_battlefield(&engine, p0, phyrexian_rager()).expect("the Rager resolved");
    assert_eq!(pt(&engine, rager), (2, 2), "the printed 2/2 body");

    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"you lose 1 life\" — the controller pays for the card"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the loss is not the opponent's, which is the other reading of `you`"
    );

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "\"you draw a card\": the very card that was on top of the library is \
         in hand, and not some other card that happened to move"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "one card left the top of the library and nothing put one back"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Rager left the hand when it was cast and one card came back for \
         it — a count that never moved would be satisfied by a trigger that \
         drew nothing and a cast that never happened"
    );
    assert_eq!(
        engine.state().players[p1.get() as usize].life,
        20,
        "\"its controller\": the seat across the table paid nothing for a \
         card it never drew"
    );
}
