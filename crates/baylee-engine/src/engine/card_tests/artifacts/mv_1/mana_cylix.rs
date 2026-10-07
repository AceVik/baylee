//! `cards/artifacts/mv_1/mana_cylix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mana Cylix prints one line — "{1}, {T}: Add one mana of any color." — and
/// the {1} is the whole of what keeps it from being a Sol Ring: the price is a
/// mana *and* the tap, so `tap_all_mana` leaves it standing (the helper presses
/// only abilities whose entire cost is their own {T}) and the ability has to be
/// activated by hand out of a pool that already covers the {1} (CR 601.2h).
/// The board is two Forests and nothing else, so the black mana that lands in
/// the pool after the answer has no other source on the table — a Forest makes
/// green, and the green that was floating is exactly what the {1} consumed, so
/// reading it gone is what proves the payment happened rather than a free
/// activation.
#[test]
fn mana_cylix_taps_and_spends_a_mana_for_one_of_the_five_colors() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[mana_cylix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two Forests tapped, {1} spent on the artifact, one green left floating.
    cast_from_hand(&mut engine, p0, mana_cylix());
    pass_until(&mut engine, stack_is_empty);
    let cylix = on_battlefield(&engine, p0, mana_cylix()).expect("the Cylix resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "two Forests pay the {{1}} and leave one green behind"
    );
    assert!(
        !is_tapped(&engine, cylix),
        "and the Cylix is untouched: nothing has paid its own price yet"
    );

    // Ability 0 is the printed "{1}, {T}: Add one mana of any color."
    activate(&mut engine, p0, mana_cylix(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off the artifact, with both Forests spent"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and the green that paid the {{1}} is gone, so the price was really paid"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, cylix), "the Cylix paid its own {{T}}");
}
