//! `cards/lands/underdome.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Underdome: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to pay Un-costs."
/// Under `Coverage::Partial`, the Un-cost restricted mana ability is unsupported and omitted.
/// The land taps to produce {C} and offers no additional activated ability.
#[test]
fn underdome_taps_for_colorless_and_omits_un_costs_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(120, forest())
        .battlefield(0, &[underdome()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, underdome()).expect("Underdome deployed");

    activate(&mut engine, p0, underdome(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == land && *ai == 1),
        "no second ability is offered"
    );
}
