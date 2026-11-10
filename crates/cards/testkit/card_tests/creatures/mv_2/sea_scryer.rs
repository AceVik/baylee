//! `cards/creatures/mv_2/sea_scryer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sea Scryer prints two mana abilities: "{T}: Add {C}" and "{1}, {T}:
/// Add {U}". The first has its own tap symbol as its entire cost, the second
/// additionally demands a mana — and because `can_afford` reads the *pool*,
/// the second is not even offered at an empty pool. Precisely for that reason
/// the test goes through a full turn cycle: the Scryer taps for {C}, untaps,
/// and a single Forest pays the {1}, whereupon green disappears from the pool
/// and the printed blue comes back.
#[test]
fn sea_scryer_taps_for_colorless_then_pays_a_mana_for_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[sea_scryer()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 erreicht seine eigene Hauptphase"
    );

    // The two Islands pay {1}{U} and nothing else: the Forest stays untapped,
    // so that “empty pool” is an exact statement after the spell.
    tap_all_mana_but(&mut engine, p0, Some(forest()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands, two blue"
    );
    cast_with_floating(&mut engine, p0, sea_scryer());
    pass_until(&mut engine, stack_is_empty);
    let scryer = on_battlefield(&engine, p0, sea_scryer()).expect("der Scryer ist gelandet");
    assert_eq!(pt(&engine, scryer), (1, 1), "ein gedruckter 1/1");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}} came from the pool and nothing remained"
    );

    // Its own next turn: `{T}` in a cost is not payable for a creature
    // that arrived this turn (CR 302.6). To expect the line during the
    // turn of casting would mean explaining away the summoning sickness.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(scryer, 0)),
        "{{T}}: Add {{C}} charges its own tap symbol and nothing else: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(scryer, 1)),
        "{{1}}, {{T}} is not payable from an empty pool and therefore \
         not even in the offer: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sea_scryer(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "ein farbloses Mana"
    );
    assert_eq!(pool.total(), 1, "und sonst nichts");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability does not use the stack"
    );
    assert!(is_tapped(&engine, scryer), "the Scryer paid its {{T}}");

    // A turn cycle so that the creature really untaps: tapped, it offers
    // neither the one nor the other line.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, scryer),
        "der Enttappschritt hat den Scryer wieder aufgestellt"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool empties at the end of a step (CR 500.5)"
    );

    // Only the Forest is tapped: one green mana, which pays exactly the {1}
    // of the second line, while the Scryer stays untapped for its {T}.
    let grove = on_battlefield(&engine, p0, forest()).expect("the Forest is still there");
    tap_mana_where(&mut engine, p0, |id| id == grove);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(scryer, 1)),
        "mit einem schwebenden Mana ist {{1}}, {{T}}: Add {{U}} bezahlbar: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sea_scryer(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "das grüne Mana war das {{1}} der Kosten"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "and the ability gives back exactly the blue that it prints"
    );
    assert_eq!(pool.total(), 1, "ein Mana hinein, ein Mana hinaus");
    assert!(
        stack_is_empty(&engine),
        "this mana ability doesn't use the stack either"
    );
    assert!(is_tapped(&engine, scryer), "paid with its own {{T}}");
}
