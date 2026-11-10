//! `cards/creatures/mv_2/nomadic_elf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nomadic Elf prints a *mana* ability — "{1}{G}: Add one mana of any color"
/// — on a {1}{G} 2/2 body, and the price is the whole card: read as a
/// tap-only line it would make mana out of nothing, and read as a fixed
/// color it would never ask. Two Forests pay exactly {1}{G}, so the pool
/// afterwards holds one mana of the named color and nothing else; and the
/// Elf is still standing afterwards, because the cost is mana and not the
/// tap symbol some other mana creatures print. The question itself is five
/// colors wide with no colorless on it, which is "any color" (CR 105.4).
#[test]
fn nomadic_elf_spends_two_mana_for_one_of_any_color_and_keeps_its_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[nomadic_elf(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, nomadic_elf()).expect("the Elf is on the table");
    assert_eq!(pt(&engine, elf), (2, 2), "the body the card prints");

    // `can_afford` reads the pool and not the untapped lands, so the mana is
    // floated first and the offer read afterwards. `tap_all_mana` presses only
    // abilities whose whole price is their own `{T}` (#159), and this one's
    // price is `{1}{G}`, so the Elf is not one of the routes it takes.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, two green"
    );
    assert!(
        !is_tapped(&engine, elf),
        "the price is mana; nothing has tapped the Elf"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(elf, 0)),
        "with {{1}}{{G}} floating the line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, nomadic_elf(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
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
        pool.available(ManaColor::Green),
        0,
        "both Forests paid the {{1}}{{G}}, so no green is left beside it"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off a two-mana price — a free activation would leave three"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        !is_tapped(&engine, elf),
        "and the price was mana alone: the Elf is untapped and still a 2/2"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "which the ability never touched");
}
