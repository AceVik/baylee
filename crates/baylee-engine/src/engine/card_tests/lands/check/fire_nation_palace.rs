//! `cards/lands/check/fire_nation_palace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fire Nation Palace prints `This land enters tapped unless you control a basic land`, `{{T}}: Add {{R}}`, and a firebending ability.
/// The card is marked `Coverage::Partial` because the firebending activation is unsupported and omitted.
/// When played while controlling a basic `forest()`, Fire Nation Palace enters untapped and immediately taps to add red mana to the pool.
#[test]
fn fire_nation_palace_enters_untapped_with_basic_land_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[fire_nation_palace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let palace = play_land(&mut engine, p0, fire_nation_palace());
    assert!(!entered_tapped(&engine, palace));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}
