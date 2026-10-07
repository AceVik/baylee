//! `cards/lands/utility/alchemist_s_refuge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Alchemist's Refuge: "{T}: Add {C}." / "{G}{U}, {T}: You may cast spells this turn as though they had flash."
/// Under `Coverage::Partial`, the global flash-granting permission is unsupported and omitted.
/// The land taps to add {C} to the mana pool and offers no second ability.
#[test]
fn alchemist_s_refuge_taps_for_colorless_and_omits_flash_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(123, forest())
        .battlefield(0, &[alchemist_s_refuge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let refuge = on_battlefield(&engine, p0, alchemist_s_refuge()).expect("Refuge deployed");

    activate(&mut engine, p0, alchemist_s_refuge(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, refuge));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == refuge && *ai == 1),
        "no second ability is offered"
    );
}
