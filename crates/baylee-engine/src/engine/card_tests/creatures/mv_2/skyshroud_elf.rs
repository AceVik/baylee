//! `cards/creatures/mv_2/skyshroud_elf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skyshroud Elf is `{1}{G}` for a 1/1 with **two** printed mana abilities —
/// "{T}: Add {G}" and "{1}: Add {R} or {W}" — and the whole scenario turns on
/// the difference between their costs. The second names no tap symbol, so the
/// Elf may use it the turn it lands (CR 302.6 reaches only the tap and untap
/// symbols) and does so off the mana its own cast left floating, where the
/// first has to wait for an untap step. Both halves are read against a board
/// that could not have produced either colour otherwise: three Forests make
/// nothing but green, so the colour that was named can only have come off the
/// second ability, and the single green the Elf's own tap makes a turn later
/// arrives in an empty pool with every Forest still standing.
#[test]
fn skyshroud_elf_lands_for_a_mana_and_then_taps_for_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(3771, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[skyshroud_elf()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{G} off the three Forests, which leaves one green in the pool — the
    // exact mana the second ability charges, already floating when the claim
    // below is made (`can_afford` reads the pool, not the untapped lands).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and the Elf is still in hand"
    );
    cast_with_floating(&mut engine, p0, skyshroud_elf());
    pass_until(&mut engine, stack_is_empty);

    let elf = on_battlefield(&engine, p0, skyshroud_elf()).expect("the Elf resolved");
    assert_eq!(pt(&engine, elf), (1, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}}{{G}} is spent and one green is left floating"
    );

    // Ability 1, "{1}: Add {R} or {W}". Its cost is a mana and no tap symbol,
    // so a creature that has been under its controller's control for no time
    // at all may still use it.
    activate(&mut engine, p0, skyshroud_elf(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::Red, ManaColor::White],
        "the two colours the card prints, and green is not one of them"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the two it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the red is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the green the cast left was the {{1}} that paid for it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one activation");
    assert!(
        !is_tapped(&engine, elf),
        "the second ability is a mana and not the tap symbol, so the Elf is \
         still standing"
    );

    // Now the printed tap, which needs an untap step first. A turn passes,
    // the pool empties at the end of the step the red was made in (CR 500.5),
    // and the reading below is about one tap of one creature and nothing left
    // over.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the red from last turn's main phase is gone"
    );
    assert!(
        !is_tapped(&engine, elf),
        "and the untap step stood the Elf back up"
    );

    // Ability 0 is "{T}: Add {G}", whose whole price is its own tap — exactly
    // the route `tap_all_mana` presses (#159). Asking for the object keeps
    // the three Forests out of the count, so "one green and nothing else" is
    // an exact claim about the Elf.
    let taken = tap_mana_where(&mut engine, p0, |id| id == elf);
    assert_eq!(taken, 1, "one route: the Elf's own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "{{T}}: Add {{G}}");
    assert_eq!(pool.total(), 1, "one mana, and nothing else came with it");
    assert!(is_tapped(&engine, elf), "the Elf paid its own {{T}}");
    assert!(
        all_on_battlefield(&engine, p0, forest())
            .iter()
            .all(|id| !is_tapped(&engine, *id)),
        "and the three Forests are untouched, so the green came off the Elf"
    );
}
