//! `cards/sorceries/mv_5/tidings.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tidings prints one sentence — "Draw four cards" — so the entire card is a
/// count, and the board has to make that count exact rather than merely
/// positive. Five Islands are `{3}{U}{U}` to the last mana, so the pool reads
/// empty both before the cast and after it, and the only thing that moved is
/// four cards off the top of the library into the hand. A draw that had
/// fetched one card, or that had counted the sorcery still sitting in hand,
/// would leave the same board with a different number on it.
#[test]
fn tidings_spends_five_mana_to_draw_four_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[tidings()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Read the board where the engine reads it: an empty pool is what says the
    // five Islands are still standing when the spell is announced, so every
    // card that moves below was paid for rather than granted.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating before the cast"
    );

    cast_from_hand(&mut engine, p0, tidings());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}}{{U}}{{U}} is the whole cost, so the five Islands leave nothing behind"
    );
    assert!(
        !stack_is_empty(&engine),
        "a sorcery resolves off the stack, so the Tidings is waiting on it"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 4,
        "\"Draw four cards\" — four cards left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1 + 4,
        "the Tidings left the hand to be cast and four cards arrived in its place"
    );
    assert!(
        in_graveyard(&engine, p0, tidings()).is_some(),
        "and the resolved sorcery is in its owner's graveyard"
    );
}
