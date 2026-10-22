//! `cards/enchantments/mv_3/inner_sanctum.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, INNER_SANCTUM, LIGHTNING_BOLT};

/// `Inner Sanctum` ({1}{W}{W}, Enchantment) has cumulative upkeep—Pay 2 life
/// and prevents all damage that would be dealt to creatures you control.
#[test]
fn inner_sanctum_prevents_damage_to_your_creatures_and_has_cumulative_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[INNER_SANCTUM, GRIZZLY_BEARS, mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 18);

    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
    assert!(on_battlefield(&engine, p0, INNER_SANCTUM).is_some());
}
