//! `cards/lands/utility/starlit_sanctum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Starlit Sanctum` prints `{{T}}: Add {{C}}.`, `{{W}}, {{T}}, Sacrifice a Cleric creature: You gain life equal to the sacrificed creature's toughness.`, and `{{B}}, {{T}}, Sacrifice a Cleric creature: Target player loses life equal to the sacrificed creature's power.`
///
/// Under `Coverage::Partial`, only the colorless mana ability is implemented because reading power or toughness from a sacrificed creature is unsupported.
/// With `Starlit Sanctum`, `ondu_cleric()`, a `plains()`, and a `swamp()` on the battlefield, floating `{{W}}` and `{{B}}` while keeping `Starlit Sanctum` untapped shows that ability 0 is offered while both Cleric-sacrifice abilities are omitted from `legal.abilities`.
/// Activating ability 0 adds one colorless mana to the pool and leaves the land tapped.
#[test]
fn starlit_sanctum_taps_for_colorless_and_omits_cleric_sacrifice_abilities() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[starlit_sanctum(), plains(), swamp(), ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sanctum = on_battlefield(&engine, p0, starlit_sanctum()).expect("sanctum on battlefield");

    // Float {{W}} and {{B}} while keeping Starlit Sanctum untapped.
    tap_mana_except(&mut engine, p0, sanctum);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    assert!(!is_tapped(&engine, sanctum));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(sanctum, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(sanctum, 1)),
        "ability 1 is omitted under `Coverage::Partial` despite floating {{W}} and a controlled Cleric"
    );
    assert!(
        !legal.abilities.contains(&(sanctum, 2)),
        "ability 2 is omitted under `Coverage::Partial` despite floating {{B}} and a controlled Cleric"
    );

    activate(&mut engine, p0, starlit_sanctum(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::White), 1);
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.total(), 3);
    assert!(is_tapped(&engine, sanctum));
}
