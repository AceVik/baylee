//! `cards/creatures/mv_2/fire_sprites.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Fire Sprites` prints `KeywordSet::FLYING` and a mana ability with cost `{{G}}, {{T}}`
/// adding `{{R}}` under `Coverage::Implemented`.
/// Because its cost exceeds a bare tap, `tap_all_mana` taps only the basic Forest, floating `{{G}}`.
/// Calling `activate` manually spends the floating green mana and taps the Sprites to produce `{{R}}`.
#[test]
fn fire_sprites_filters_green_into_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1619, forest())
        .battlefield(0, &[fire_sprites(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sprites = on_battlefield(&engine, p0, fire_sprites()).expect("sprites are seated");
    assert!(keywords(&engine, sprites).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0
    );
    assert!(
        !is_tapped(&engine, sprites),
        "Sprites cost {{G}}, {{T}} so tap_all_mana left them untapped"
    );

    activate(&mut engine, p0, fire_sprites(), 0);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert!(is_tapped(&engine, sprites));
}
