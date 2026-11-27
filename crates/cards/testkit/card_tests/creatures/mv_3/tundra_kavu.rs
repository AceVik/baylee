//! `cards/creatures/mv_3/tundra_kavu.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn tundra_kavu() -> CardIndex {
    card_index("b0b7d88b-694b-4bbe-888d-05245e7d1c3e")
}

/// Tundra Kavu turns target land into a Plains or an Island until end of turn,
/// replacing its basic land subtypes and restoring them on the next turn.
#[test]
fn tundra_kavu_sets_land_type_to_plains_or_island_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tundra_kavu(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let kavu_target_land = on_battlefield(&engine, p0, forest()).unwrap();
    assert!(
        engine
            .state()
            .object(kavu_target_land)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::FOREST)
    );

    activate(&mut engine, p0, tundra_kavu(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![kavu_target_land],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let plains_mode = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine
        .apply(p0, PlayerAction::ChooseMode(plains_mode))
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let chars = engine
        .state()
        .object(kavu_target_land)
        .unwrap()
        .characteristics();
    assert!(chars.subtypes.contains(subtypes::land::PLAINS));
    assert!(!chars.subtypes.contains(subtypes::land::FOREST));

    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: kavu_target_land,
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );

    reach_their_main_phase(&mut engine, p1);
    let restored = engine
        .state()
        .object(kavu_target_land)
        .unwrap()
        .characteristics();
    assert!(restored.subtypes.contains(subtypes::land::FOREST));
    assert!(!restored.subtypes.contains(subtypes::land::PLAINS));
}
