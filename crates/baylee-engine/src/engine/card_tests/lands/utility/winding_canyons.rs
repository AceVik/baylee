//! `cards/lands/utility/winding_canyons.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Winding Canyons: "{T}: Add {C}." / "{2}, {T}: You may cast creature spells this turn as though they had flash."
/// Under `Coverage::Partial`, granting flash to creature spells in hand is unsupported and omitted.
/// The land taps to produce {C} and offers no second ability.
#[test]
fn winding_canyons_taps_for_colorless_and_omits_creature_flash_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(133, forest())
        .battlefield(0, &[winding_canyons()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let canyons = on_battlefield(&engine, p0, winding_canyons()).expect("Canyons deployed");

    activate(&mut engine, p0, winding_canyons(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, canyons));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == canyons && *ai == 1),
        "no second ability is offered"
    );
}
