//! `cards/lands/utility/fomori_vault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fomori Vault prints `{{T}}: Add {{C}}.` and `{{3}}, {{T}}, Discard a card: Look at the top X cards of your library, where X is the number of artifacts you control. Put one of those cards into your hand and the rest on the bottom of your library in a random order.`
///
/// Under `Coverage::Partial`, the second ability is omitted because X is a dynamically computed artifact count while the DSL pick effect expects a fixed count.
/// With `{{3}}` floating from three `forest()` lands, an artifact on the battlefield (`quiet_artifact()`), a card in hand to discard (`plains()`), and Fomori Vault untapped, ability 1 is not offered in `legal.abilities`.
/// Activating ability 0 adds one colorless mana to `pool.available(ManaColor::Colorless)`.
#[test]
fn fomori_vault_taps_for_colorless_and_omits_artifact_count_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                fomori_vault(),
                forest(),
                forest(),
                forest(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = on_battlefield(&engine, p0, fomori_vault()).expect("fomori vault on battlefield");

    // Float the three Forests and the Sol Ring's {{C}}{{C}} while keeping
    // Fomori Vault untapped — five, and the omitted ability charges three.
    tap_mana_except(&mut engine, p0, vault);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    assert!(!is_tapped(&engine, vault));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(vault, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(vault, 1)),
        "with {{3}} floating, an artifact controlled, a card in hand, and vault untapped, ability 1 is omitted under `Coverage::Partial`"
    );

    activate(&mut engine, p0, fomori_vault(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        3,
        "the vault's one, on top of the Sol Ring's two"
    );
    assert_eq!(pool.available(ManaColor::Green), 3);
    assert_eq!(pool.total(), 6);
    assert!(is_tapped(&engine, vault));
}
