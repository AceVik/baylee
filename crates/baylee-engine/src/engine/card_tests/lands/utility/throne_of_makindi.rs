//! `cards/lands/utility/throne_of_makindi.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Throne of Makindi is a land whose implemented half is two printed
/// abilities: `{T}: Add {C}`, and `{1}, {T}: Put a charge counter on this
/// land`. Both are played across one turn cycle, because the second wants the
/// Throne untapped and the first is what taps it — the Forests beside it
/// coming back in the same untap step are what tells a real untap step from a
/// game that never advanced. The colourless reads the Throne alone (the
/// Forests print {G}), and the charge counter together with the {1} that left
/// the pool are the only evidence that the second line's cost was paid rather
/// than waived. The restricted `{T}, remove a charge counter: add two mana of
/// any one colour for kicked spells` is the `Coverage::Partial` gap and is
/// written nowhere on the card, so nothing here presses it.
#[test]
fn throne_of_makindi_taps_for_colorless_and_banks_a_charge_counter_for_a_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[throne_of_makindi()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land drop, so the Throne is really in play as a land and not a
    // permanent the harness believed onto the table.
    let throne = play_land(&mut engine, p0, throne_of_makindi());
    assert!(
        types(&engine, throne).contains(TypeSet::LAND),
        "a land, played as a land"
    );
    assert!(!is_tapped(&engine, throne), "and nothing keeps it down");

    // `{T}: Add {C}` — read off the board rather than off a filter: every land
    // this seat controls is tapped, and the one colourless in the pool has no
    // source other than the Throne's own printed mana ability (#159).
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        3,
        "two Forests and the Throne"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\""
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        2,
        "the Forests' {{G}}{{G}}"
    );
    assert!(is_tapped(&engine, throne), "paid with its own tap");

    // Across the opponent's turn and back: the untap step has to have run for
    // the second activation to be about the card's second line at all.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, throne),
        "the Throne untaps like any land"
    );
    assert!(
        all_on_battlefield(&engine, p0, forest())
            .iter()
            .all(|id| !is_tapped(&engine, *id)),
        "and the Forests came back in the same step, so the turn really moved"
    );

    // `{1}, {T}: Put a charge counter on this land`. The Throne is the one
    // source kept upright, because it is the permanent paying the {T} itself.
    tap_all_mana_but(&mut engine, p0, Some(throne_of_makindi()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the Forests, and the Throne kept back for its own cost"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "{{G}}{{G}} standing by to pay the {{1}}"
    );
    // Ability 0 is the mana ability, 1 the charge counter.
    activate(&mut engine, p0, throne_of_makindi(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, throne, CounterKind::Charge),
        1,
        "\"Put a charge counter on this land\""
    );
    assert!(
        is_tapped(&engine, throne),
        "the {{T}} in the cost tapped it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{1}} left the pool: two green floated in, one green is left"
    );
}
