//! `cards/lands/filter/shadowblood_ridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shadowblood Ridge prints one line — "{1}, {T}: Add {B}{R}" — and it is the
/// whole card, so the scenario is that price paid rather than approximated.
/// The generic mana is the half worth reading: a filter land whose `{1}` was
/// dropped would be a strictly better land, and the two readings are told
/// apart off the same board — once with an empty pool, where the generic is
/// unpayable and the engine does not offer the line at all, and once after a
/// Forest has made the mana. Both colours land in the pool together, which
/// neither a `{T}: Add {B}` nor a `{T}: Add {R}` misreading would produce.
#[test]
fn shadowblood_ridge_charges_a_generic_mana_for_its_two_colours() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[shadowblood_ridge(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ridge = on_battlefield(&engine, p0, shadowblood_ridge()).expect("the Ridge is seated");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is seated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats yet"
    );

    // The price is `{1}` *and* its own tap, so it is no route `tap_all_mana`
    // may take (#159) — and before anything is tapped the generic is
    // unpayable, so `can_afford` keeps the ability out of the offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ridge, 0)),
        "`{{1}}` out of an empty pool is no price this seat can pay: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert!(
        !is_tapped(&engine, ridge),
        "the Ridge's whole price is not its own tap, so the helper leaves it \
         standing for the ability this test presses"
    );
    assert!(
        is_tapped(&engine, land),
        "and the Forest paid for the {{1}}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one green floating, which is exactly the generic"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ridge, 0)),
        "with the generic in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, shadowblood_ridge(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "Add {{B}}");
    assert_eq!(pool.available(ManaColor::Red), 1, "and Add {{R}}");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the green went to the {{1}}, so the pool is the two colours exactly"
    );
    assert_eq!(pool.total(), 2, "two mana made and one paid");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(
        is_tapped(&engine, ridge),
        "{{T}} was the other half of the price"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "no colour is ever asked — both halves are printed — so the seat holds \
         priority straight away, got {:?}",
        engine.pending()
    );
}
