//! `cards/lands/filter/skycloud_expanse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skycloud Expanse prints one line and it has two halves: "{1}, {T}: Add
/// {W}{U}". The price is *not* the land's own tap, so `tap_all_mana` leaves it
/// alone by design (#159) and the activation is pressed by hand — with the {1}
/// already floating, because `legal.abilities` is filtered by `can_afford`
/// against the pool and an empty pool is not a {1}. The number that proves the
/// cost was charged is the pool total: one white off the Plains plus two mana
/// printed out is **two** only if the generic came out of the pool, and three
/// if "{1}" were a label. Both colours arriving from the one activation is the
/// other half — "{W}{U}" is two named colours and not a choice.
#[test]
fn skycloud_expanse_charges_its_generic_and_gives_one_white_and_one_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[skycloud_expanse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Played rather than seeded: the harness' `starting_battlefield` places a
    // permanent with `Cause::Setup`, which is not an entry and pays no price.
    let expanse = play_land(&mut engine, p0, skycloud_expanse());
    assert!(
        !entered_tapped(&engine, expanse),
        "it prints no entry clause"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(expanse, 0)),
        "the {{1}} is read off the pool, and nothing is floating yet: {:?}",
        legal.abilities
    );

    // The generic comes out of the pool, so the Plains is tapped first and the
    // Expanse is kept back — its {T} is the other half of the price the very
    // activation pays.
    tap_all_mana_but(&mut engine, p0, Some(skycloud_expanse()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one white off the Plains, and the Expanse still standing"
    );
    assert!(!is_tapped(&engine, expanse), "the land is untapped");

    activate(&mut engine, p0, skycloud_expanse(), 0);

    // A mana ability uses no stack (CR 605.3b) and asks nothing: the two
    // colours are printed, not chosen, so no `ChooseColor` is in the way.
    assert!(
        stack_is_empty(&engine),
        "nothing is on the stack, got {:?}",
        engine.pending()
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, expanse), "{{T}} was paid");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "{{W}}");
    assert_eq!(pool.available(ManaColor::Blue), 1, "and {{U}}");
    assert_eq!(
        pool.total(),
        2,
        "two mana and not three: the {{1}} came out of the pool the Plains \
         filled, which is the half of \"{{1}}, {{T}}\" a card file cannot show"
    );
}
