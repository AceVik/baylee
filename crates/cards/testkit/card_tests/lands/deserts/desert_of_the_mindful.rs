//! `cards/lands/deserts/desert_of_the_mindful.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Desert of the Mindful prints three sentences — it enters tapped, it taps
/// for `{U}`, and it cycles for `{1}{U}` — and all three are played in one
/// main phase. The land drop has to be a real `PlayLand`, because a permanent
/// placed by `starting_battlefield` never runs what it enters with, so only a
/// played copy can show the printed entry. The cycle spends the card itself
/// out of hand for exactly the mana the two Islands floated, and the printed
/// `{T}` is read off the seated third copy, which stands untapped because the
/// copy that just entered tapped may not be tapped again before its
/// controller's untap step.
#[test]
fn desert_of_the_mindful_enters_tapped_taps_for_blue_and_cycles_for_a_card() {
    let p0 = PlayerId::new(0);
    // Three copies: one seated, one for the land drop, one for the cycle.
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[island(), island(), desert_of_the_mindful()])
        .hand(0, &[desert_of_the_mindful(), desert_of_the_mindful()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let seated = on_battlefield(&engine, p0, desert_of_the_mindful()).expect("a Desert is seated");

    let played = play_land(&mut engine, p0, desert_of_the_mindful());
    assert_ne!(played, seated, "the drop is one of the two in hand");
    assert!(
        entered_tapped(&engine, played),
        "\"This land enters tapped\" — the played copy arrives tapped while \
         the seated one stands untapped"
    );

    // Mana before the claim below: the offer reads the pool and not the
    // untapped lands. The seated Desert is named as the one source kept back,
    // because its own `{T}` is the ability this test presses at the end.
    tap_mana_except(&mut engine, p0, seated);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "two Islands in the pool, and the seated Desert was left standing"
    );

    // Cycling {1}{U}: index 1, the mana ability being 0, and offered from the
    // hand and nowhere else.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, desert_of_the_mindful(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, desert_of_the_mindful()).is_some(),
        "\"Discard this card\" spends the card itself: it is in the graveyard"
    );
    assert!(
        in_hand(&engine, p0, desert_of_the_mindful()).is_none(),
        "and no copy of it is left in hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — cycling draws exactly one"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card out and one card in, so the hand is the size it was"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}} came out of the pool, leaving nothing floating"
    );

    // The printed mana ability, off the copy the cycle did not spend. Its pool
    // is empty, so one blue is an exact claim rather than a total that happens
    // to match.
    assert!(
        !is_tapped(&engine, seated),
        "the seated Desert was never asked for mana on the way here"
    );
    activate(&mut engine, p0, desert_of_the_mindful(), 0);
    assert!(
        is_tapped(&engine, seated),
        "\"{{T}}\" is the whole price of the ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "{{U}} off one tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing else came with it"
    );
}
