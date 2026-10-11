//! `cards/lands/utility/mariposa_military_base.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Declining the optional tapped/rad entry preserves an untapped land.
/// With no rad counters its draw ability costs the printed five mana.
#[test]
fn mariposa_military_base_enters_untapped_and_draws_card_for_five_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[mariposa_military_base()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let base = play_land(&mut engine, p0, mariposa_military_base());
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(engine.state().players[0].rad, 0);
    assert!(!is_tapped(&engine, base));

    tap_all_mana_but(&mut engine, p0, Some(mariposa_military_base()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        5
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, mariposa_military_base(), 1);
    assert!(is_tapped(&engine, base));

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
