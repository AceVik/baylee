//! `cards/lands/restricted/sea_gate_wreckage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sea Gate Wreckage: "{T}: Add {C}." / "{2}{C}, {T}: Draw a card. Activate only if you have no cards in hand."
/// Under `Coverage::Partial`, the zero-card hand draw ability is omitted.
/// Activating the land's implemented ability adds {C} to the mana pool and taps Sea Gate Wreckage.
#[test]
fn sea_gate_wreckage_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(123, forest())
        .battlefield(0, &[sea_gate_wreckage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land =
        on_battlefield(&engine, p0, sea_gate_wreckage()).expect("Sea Gate Wreckage deployed");
    activate(&mut engine, p0, sea_gate_wreckage(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Sea Gate Wreckage: "{2}{C}, {T}: Draw a card. Activate only if you have
/// no cards in hand." The condition is hellbent and the price is not
/// `{3}` — the `{C}` is a colourless *requirement*, which is why the board
/// carries a second colourless source beside the two Forests. The negative
/// is one card in hand, which is the whole of what the card asks.
#[test]
fn sea_gate_wreckage_draws_only_on_an_empty_hand() {
    let p0 = PlayerId::new(0);

    let mut holding = Duel::new(9205, forest())
        .battlefield(
            0,
            &[sea_gate_wreckage(), forest(), forest(), access_tunnel()],
        )
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut holding);
    reach_main_phase(&mut holding, p0);
    let wreck = on_battlefield(&holding, p0, sea_gate_wreckage()).expect("the Wreckage is seated");
    tap_mana_except(&mut holding, p0, wreck);
    let Pending::Priority { legal, .. } = holding.pending().clone() else {
        panic!("expected priority, got {:?}", holding.pending())
    };
    assert!(
        !legal.abilities.contains(&(wreck, 1)),
        "one card in hand is not \"no cards in hand\": {:?}",
        legal.abilities
    );

    let mut empty = Duel::new(9206, forest())
        .battlefield(
            0,
            &[sea_gate_wreckage(), forest(), forest(), access_tunnel()],
        )
        .start();
    keep_mulligans(&mut empty);
    reach_main_phase(&mut empty, p0);
    assert!(
        empty.state().zones.list(ZoneLocation::Hand(p0)).is_empty(),
        "the kit deals no opening hand, which is what hellbent wants"
    );
    let wreck = on_battlefield(&empty, p0, sea_gate_wreckage()).expect("the Wreckage is seated");
    tap_mana_except(&mut empty, p0, wreck);
    let library_before = library_size(&empty, p0);
    activate(&mut empty, p0, sea_gate_wreckage(), 1);
    pass_until(&mut empty, stack_is_empty);
    assert_eq!(
        library_size(&empty, p0),
        library_before - 1,
        "one card drawn"
    );
}
