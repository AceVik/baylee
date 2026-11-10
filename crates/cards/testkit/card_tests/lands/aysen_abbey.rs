//! `cards/lands/aysen_abbey.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aysen Abbey — Land: "{T}: Add {C}. {1}, {T}: Add {W}. {2}, {T}: Add {G} or
/// {U}." Three mana abilities on one land and no other text, so pressing each
/// in turn is the whole card: the {C} line asks nothing and costs nothing
/// beyond its own tap, the {1} line buys a white mana out of the pool, and the
/// {2} line arrives as a question carrying exactly the printed pair — a land
/// whose third ability defaulted to one of the two colours would satisfy every
/// pool total here except that one question.
///
/// Three copies stand on the same side because an activation taps the land
/// that paid it, and the three Forests beside them are the pool the two paid
/// lines are read against. They are pressed two, then one, then zero, which is
/// the order that leaves every mana count exact rather than merely plausible.
#[test]
fn aysen_abbey_presses_all_three_printed_mana_abilities() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4207, forest())
        .battlefield(
            0,
            &[aysen_abbey(), aysen_abbey(), forest(), forest(), forest()],
        )
        .hand(0, &[aysen_abbey()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    play_land(&mut engine, p0, aysen_abbey());
    assert_eq!(
        all_on_battlefield(&engine, p0, aysen_abbey()).len(),
        3,
        "two seeded copies and one played off its own land drop"
    );

    // The Abbeys stay standing: `tap_all_mana` would have spent all three on
    // ability 0, and each activation below is pressed by index.
    tap_all_mana_but(&mut engine, p0, Some(aysen_abbey()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests are the only mana on the board"
    );

    // Ability 2: `{2}, {T}: Add {G} or {U}`.
    activate(&mut engine, p0, aysen_abbey(), 2);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{G}} or {{U}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        2,
        "the printed pair, with no default to fall back on: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "green and blue, in some order: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.total(),
        2,
        "{{2}} out of a three-land pool and one mana back in"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        2,
        "two of the three Forests paid the {{2}} and the named green arrived"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "the other half of the choice was never added"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );

    // Ability 1: `{1}, {T}: Add {W}`.
    activate(&mut engine, p0, aysen_abbey(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.total(),
        2,
        "{{1}} out of the pool and one white back in"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the white the card prints"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and the generic came out of the green that was floating"
    );
    assert!(stack_is_empty(&engine), "no stack for this one either");

    // Ability 0: `{T}: Add {C}` — nothing in, nothing asked.
    activate(&mut engine, p0, aysen_abbey(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "the tap alone pays for the {{C}}"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and the green is untouched"
    );
    assert_eq!(pool.available(ManaColor::White), 1, "and so is the white");
    assert_eq!(pool.total(), 3, "one in and nothing out");
    assert!(
        all_on_battlefield(&engine, p0, aysen_abbey())
            .iter()
            .all(|id| is_tapped(&engine, *id)),
        "three activations, three copies, and each paid its own {{T}}"
    );
}
