//! `cards/artifacts/mv_0/mox_sapphire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mox Sapphire is a `{0}` artifact printing one line: "{T}: Add {U}".
///
/// Neither half of that costs any mana, so the board is built so that
/// neither can be borrowed from anywhere else: the Mox is cast out of an
/// **empty** pool, and the only land beside it is a Forest that stays
/// untapped and makes {G}. The single blue in the pool afterwards can
/// therefore only have come off the Mox's own tap — no green source on this
/// board could have produced it, and nothing was spent to get it.
///
/// The whole price is the tap symbol, so nothing is tapped beforehand: the
/// empty pool is what makes "one blue and not one land's worth" an exact
/// claim (rule 19). And no `ChooseColor` is expected or asked — the card
/// prints `{U}` and not "one mana of any color", which is the whole
/// difference between this and Mox Diamond.
#[test]
fn mox_sapphire_taps_for_one_blue_off_an_empty_pool_and_an_untapped_forest() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_sapphire()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let card = in_hand(&engine, p0, mox_sapphire()).expect("the Mox is in hand");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats: a {{0}} artifact needs no mana, and the Forest is \
         not tapped for it"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "{{0}} is affordable on an empty pool, so the Mox is castable without \
         tapping a single source"
    );

    cast_with_floating(&mut engine, p0, mox_sapphire());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_sapphire()).expect("the Mox resolved onto the table");
    assert!(
        !is_tapped(&engine, land),
        "the Forest never paid for a zero-cost artifact"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool is still empty once the Mox has landed"
    );

    // Ability 0 is the printed "{T}: Add {U}", and its whole price is the tap.
    activate(&mut engine, p0, mox_sapphire(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "one blue, and the Forest beside it could not have made it"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "nothing on this board produces green, so the blue is the Mox's alone"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest is still standing, so the black-and-blue reading is \
         not a tapped land's"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again and was asked nothing on the way — \
         `{{U}}` is fixed, not a colour to choose, got {:?}",
        engine.pending()
    );
}
