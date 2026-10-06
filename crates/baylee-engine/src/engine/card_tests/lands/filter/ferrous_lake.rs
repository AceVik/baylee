//! `cards/lands/filter/ferrous_lake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ferrous Lake sits on the table as a land without a basic land type, so it
/// has no CR-305.6 mana ability and no other way to mana than the printed
/// line: "{1}, {T}: Add {U}{R}." Thus the player's mana pool is the only
/// source for the {1} — with an empty pool the ability may not even be in
/// the offer, and that is exactly the counterproof that the cost is read
/// and not merely printed. After that, a single tapped Island carries the
/// whole cost: the {1} disappears from the pool, and exactly one blue and
/// one red mana remain — the red half cannot have been produced by any
/// other card on this board.
#[test]
fn ferrous_lake_charges_one_generic_for_a_blue_and_a_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[ferrous_lake(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lake = on_battlefield(&engine, p0, ferrous_lake()).expect("the Lake is on the battlefield");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before anything is tapped"
    );
    assert!(
        !legal.abilities.contains(&(lake, 0)),
        "an empty pool cannot pay the {{1}} the ability charges, so it is \
         not offered: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the Island's one blue, and the Lake itself makes nothing untapped \
         without the {{1}}"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lake, 0)),
        "with the {{1}} floating the Lake's only line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, ferrous_lake(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "one blue out: the {{U}} the card prints against the one the {{1}} \
         took off the Island"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "and one red, which no other permanent on this board can produce"
    );
    assert_eq!(
        pool.total(),
        2,
        "one paid Island bought two mana, and the {{1}} is gone from the pool"
    );
    assert!(
        is_tapped(&engine, lake),
        "{{T}} was the other half of the cost"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
