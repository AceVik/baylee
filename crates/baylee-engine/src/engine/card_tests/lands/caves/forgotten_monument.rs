//! `cards/lands/caves/forgotten_monument.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forgotten Monument: "{T}: Add {C}." / "Other Caves you control have '{T}, Pay 1 life: Add one mana of any color.'"
/// Under `Coverage::Partial`, the ability-granting static to other Caves is unsupported and omitted.
/// The land taps for its intrinsic printed ability to add {C} to the mana pool.
#[test]
fn forgotten_monument_taps_for_colorless_mana_and_omits_ability_grant() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, forest())
        .battlefield(0, &[forgotten_monument()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let monument = on_battlefield(&engine, p0, forgotten_monument()).expect("Monument deployed");

    activate(&mut engine, p0, forgotten_monument(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, monument));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == monument && *ai == 1),
        "no second ability is offered on Forgotten Monument"
    );
}
