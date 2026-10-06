//! `cards/lands/utility/eumidian_hatchery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eumidian Hatchery prints `{{T}}, Pay 1 life: Add {{B}}. Put a hatchling counter on this land.`
/// and `When this land is put into a graveyard from the battlefield, for each hatchling counter
/// on it, create a 1/1 black Insect creature token with flying.`
///
/// Under `Coverage::Partial`, the mana ability is implemented with its life payment and counter
/// placement (`counters::HATCHLING`), while the graveyard trigger is omitted.
/// Because its activation cost includes paying life rather than tapping alone, it is not tapped
/// by automatic mana helpers. Activating ability 0 manually pays 1 life, adds one black mana,
/// places one hatchling counter, and leaves ability index 1 absent from `legal.abilities`.
#[test]
fn eumidian_hatchery_pays_life_adds_black_and_puts_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[eumidian_hatchery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hatchery =
        on_battlefield(&engine, p0, eumidian_hatchery()).expect("eumidian hatchery on battlefield");
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(counters_on(&engine, hatchery, counters::HATCHLING), 0);
    assert!(!is_tapped(&engine, hatchery));

    activate(&mut engine, p0, eumidian_hatchery(), 0);

    assert_eq!(engine.state().players[0].life, 19);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.total(), 1);
    assert_eq!(counters_on(&engine, hatchery, counters::HATCHLING), 1);
    assert!(is_tapped(&engine, hatchery));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(source, idx)| *source == hatchery && *idx == 1),
        "under `Coverage::Partial`, graveyard trigger is omitted"
    );
}
