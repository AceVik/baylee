//! `cards/lands/utility/balduvian_trading_post.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Balduvian Trading Post: "If this land would enter, sacrifice an untapped Mountain instead..." / "{T}: Add {C}{R}." / "{1}, {T}: This land deals 1 damage to target attacking creature."
/// Under `Coverage::Partial`, the entry replacement sacrificing an untapped Mountain is omitted.
/// Playing this land allows it to enter untapped and immediately tap for one colorless and one red mana.
#[test]
fn balduvian_trading_post_taps_for_colorless_and_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(330, forest())
        .hand(0, &[balduvian_trading_post()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, balduvian_trading_post());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, balduvian_trading_post(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}
