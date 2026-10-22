//! `cards/enchantments/mv_4/psychic_vortex.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::PSYCHIC_VORTEX;

/// `Psychic Vortex` ({2}{U}{U}, Enchantment, Partial) adds an age counter and
/// lets you draw a card for each age counter at upkeep, and sacrifices a land
/// and discards your hand at your end step.
#[test]
fn psychic_vortex_draws_at_upkeep_and_sacrifices_land_and_discards_hand_at_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[PSYCHIC_VORTEX, island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let land = on_battlefield(&engine, p0, island()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, island()).is_some());
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 0);
}
