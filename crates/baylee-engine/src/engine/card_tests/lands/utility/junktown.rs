//! `cards/lands/utility/junktown.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Junktown prints `{{T}}: Add {{C}}.` and `{{4}}{{R}}, {{T}}, Sacrifice this land: Create three Junk tokens.`
///
/// Under `Coverage::Partial`, the Junk-token ability is omitted because Junk tokens are not in `crate::tokens` and the play-from-exile permission has no `Effect` variant.
/// With Junktown and five basic lands (four `forest()` and one `mountain()`) on the battlefield under `PlayerId::new(0)`,
/// floating `{{4}}{{R}}` while keeping Junktown untapped shows that ability 0 is offered while ability 1 is withheld from `legal.abilities`.
/// Activating ability 0 adds one colorless mana to the pool, reaching a total of 6 mana.
#[test]
fn junktown_taps_for_colorless_and_omits_junk_token_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                junktown(),
                forest(),
                forest(),
                forest(),
                forest(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let town = on_battlefield(&engine, p0, junktown()).expect("junktown on battlefield");

    // Float {{4}}{{R}} while keeping Junktown untapped.
    tap_mana_except(&mut engine, p0, town);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    assert!(!is_tapped(&engine, town));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(town, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(town, 1)),
        "ability 1 is omitted under `Coverage::Partial` even with {{4}}{{R}} floating"
    );

    activate(&mut engine, p0, junktown(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Green), 4);
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert_eq!(pool.total(), 6);
    assert!(is_tapped(&engine, town));
}
