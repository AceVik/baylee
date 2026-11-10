//! `cards/artifacts/mv_2/mystic_compass.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn mystic_compass() -> CardIndex {
    card_index("87bafa97-38fd-41a2-8b1d-315e88d9d0bf")
}

/// Mystic Compass ({2} Artifact):
/// "{1}, {T}: Target land becomes the basic land type of your choice until end of turn."
#[test]
fn mystic_compass_changes_target_land_to_chosen_basic_land_type_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mystic_compass(), forest()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let compass = on_battlefield(&engine, p0, mystic_compass()).unwrap();
    let opp_mtn = on_battlefield(&engine, p1, mountain()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: compass,
                ability_index: 0,
            },
        )
        .unwrap();
    // "Target land": the Compass itself is no land.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![compass],
                    players: vec![],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![opp_mtn],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    // Mode 1 = Island.
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let chars = engine.state().object(opp_mtn).unwrap().characteristics();
    assert!(chars.subtypes.contains(subtypes::land::ISLAND));
    assert!(!chars.subtypes.contains(subtypes::land::MOUNTAIN));

    // Reverts at end of turn.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    let chars_after = engine.state().object(opp_mtn).unwrap().characteristics();
    assert!(chars_after.subtypes.contains(subtypes::land::MOUNTAIN));
    assert!(!chars_after.subtypes.contains(subtypes::land::ISLAND));
}
