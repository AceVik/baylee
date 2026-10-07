//! `cards/lands/utility/underdark_rift.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Underdark Rift` prints `{{T}}: Add {{C}}.` and `{{5}}, {{T}}, Exile this land: Roll a d10. Put target artifact, creature, or planeswalker into its owner's library just beneath the top X cards of that library, where X is the result. Activate only as a sorcery.`
///
/// Under `Coverage::Partial`, only the colorless mana ability is implemented because die rolling and depth-based library tucking are unsupported.
/// With a controlled creature and floating `{{5}}` mana, ability 0 is offered while ability 1 is withheld from `legal.abilities`.
/// Activating ability 0 produces one colorless mana and leaves `Underdark Rift` tapped.
#[test]
fn underdark_rift_taps_for_colorless_and_omits_unsupported_activation() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                underdark_rift(),
                young_wolf(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rift = on_battlefield(&engine, p0, underdark_rift()).expect("rift on battlefield");

    // Float {{5}} green mana from basic forests while keeping Underdark Rift untapped.
    tap_mana_except(&mut engine, p0, rift);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    assert!(!is_tapped(&engine, rift));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(rift, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(rift, 1)),
        "ability 1 is omitted under `Coverage::Partial` despite floating {{5}} and a creature target"
    );

    activate(&mut engine, p0, underdark_rift(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Green), 5);
    assert_eq!(pool.total(), 6);
    assert!(is_tapped(&engine, rift));
}
