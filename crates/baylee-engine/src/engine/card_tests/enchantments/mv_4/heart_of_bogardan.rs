//! `cards/enchantments/mv_4/heart_of_bogardan.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HEART_OF_BOGARDAN;

/// `Heart of Bogardan` ({2}{R}{R}, Enchantment, Partial) has cumulative upkeep
/// `{2}`.
#[test]
fn heart_of_bogardan_sacrificed_when_cumulative_upkeep_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[HEART_OF_BOGARDAN, mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);

    assert!(on_battlefield(&engine, p0, HEART_OF_BOGARDAN).is_some());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, HEART_OF_BOGARDAN).is_some());
}
