//! `cards/lands/utility/balduvian_trading_post.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pay the untapped Mountain entry replacement, then tap for {C}{R}.
#[test]
fn balduvian_trading_post_taps_for_colorless_and_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(330, forest())
        .hand(0, &[balduvian_trading_post()])
        .battlefield(0, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, balduvian_trading_post());
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let payment = on_battlefield(&engine, p0, mountain()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![payment],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(payment).unwrap().zone,
        Zone::Graveyard
    );
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, balduvian_trading_post(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}
