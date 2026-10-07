//! `cards/lands/check/realm_of_koh.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Realm of Koh prints `This land enters tapped unless you control a basic land`, `{{T}}: Add {{B}}`, and a token creation ability.
/// The card is marked `Coverage::Partial` because the Spirit token's blocking restrictions are unsupported and omitted.
/// When played while controlling a basic `forest()`, Realm of Koh enters untapped and immediately taps to add black mana to the pool.
#[test]
fn realm_of_koh_enters_untapped_with_basic_land_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[realm_of_koh()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let koh = play_land(&mut engine, p0, realm_of_koh());
    assert!(!entered_tapped(&engine, koh));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}
