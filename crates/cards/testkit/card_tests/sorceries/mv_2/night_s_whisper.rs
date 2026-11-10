//! `cards/sorceries/mv_2/night_s_whisper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Night's Whisper prints one line — "You draw two cards and lose 2 life" —
/// on a `{1}{B}` sorcery, and neither half is visible in the other's reading.
/// The draws are asserted off the library *and* the hand, because a library
/// two cards shorter with a hand that never grew would satisfy a count alone;
/// the life is read against both seats so that "you" is checked rather than
/// assumed. And the 2 life is read while the spell still stands on the stack —
/// it is an effect and not a cost, so it is nowhere near CR 601.2h, and a card
/// that had paid it at cast time would show 18 there already.
#[test]
fn nights_whisper_draws_two_cards_and_takes_two_life_from_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[night_s_whisper()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Swamps are tapped"
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert!(
        in_hand(&engine, p0, night_s_whisper()).is_some(),
        "the spell starts where it is cast from"
    );

    // Two Swamps pay `{1}{B}`, so the cast is a real payment and not a label.
    cast_from_hand(&mut engine, p0, night_s_whisper());
    assert!(
        on_stack(&engine, night_s_whisper()).is_some(),
        "the sorcery is on the stack, and neither half of it has happened yet"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "losing the 2 life is the spell's effect, not its cost, so the life \
         is still there while the stack holds it (CR 601.2c before 601.2h)"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the card left the hand for the stack"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "and nothing has been drawn off the top of the library yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the `{{1}}{{B}}` came out of the pool the two Swamps filled"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "\"you draw two cards\" — two off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "one card gone to the stack and two drawn brings the hand one up, \
         which a library that merely emptied could not show"
    );
    assert_eq!(
        engine.state().players[0].life,
        18,
        "\"and lose 2 life\" — the life belongs to the seat that cast it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and never to the opponent: the card says \"you\" twice"
    );
    assert!(
        in_graveyard(&engine, p0, night_s_whisper()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
}
