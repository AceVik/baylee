//! `cards/instants/mv_6/betrayal_of_flesh.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn betrayal_of_flesh() -> CardIndex {
    card_index("43cee9b6-9853-4337-869a-45540c9db10f")
}

/// Betrayal of Flesh — {5}{B} Instant
/// "Choose one —
/// • Destroy target creature.
/// • Return target creature card from your graveyard to the battlefield.
/// Entwine—Sacrifice three lands."
#[test]
fn betrayal_of_flesh_entwined_sacrifices_three_lands_to_destroy_and_reanimate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[betrayal_of_flesh()])
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            bear,
            ZoneLocation::Graveyard(p0),
            ZonePosition::Top,
            Cause::Setup,
        )
        .unwrap();
    let gy_bear = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    let swamps: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().controller == p0)
        .collect();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, betrayal_of_flesh());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", engine.pending());
    };
    let entwine_idx = options
        .iter()
        .position(|o| o.kind == CastModeKind::Modes(3))
        .expect("entwine mode offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(entwine_idx))
        .unwrap();

    // Mode 0 target: Hill Giant
    aim_at(&mut engine, p0, giant);
    // Mode 1 target: Grizzly Bears in P0's graveyard
    aim_at(&mut engine, p0, gy_bear);

    // Entwine cost: sacrifice three lands (asked one by one).
    for &land in &swamps[..3] {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![land],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    for &land in &swamps[..3] {
        assert_eq!(engine.state().object(land).unwrap().zone, Zone::Graveyard);
    }
    assert_eq!(all_on_battlefield(&engine, p0, swamp()).len(), 3);
    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_none());
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}
