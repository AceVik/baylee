//! `cards/creatures/mv_4/wall_of_distortion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "cc9cbbbc-8b02-42ef-a4d8-e2837e742c88"
fn wall_of_distortion() -> CardIndex {
    card_index("cc9cbbbc-8b02-42ef-a4d8-e2837e742c88")
}

/// `Wall of Distortion` (`Coverage::Implemented`):
/// "Defender. {2}{B}, {T}: Target player discards a card. Activate only as a
/// sorcery."
#[test]
fn wall_of_distortion_makes_target_player_discard_as_sorcery() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9708, swamp())
        .battlefield(0, &[wall_of_distortion(), swamp(), swamp(), swamp()])
        .hand(1, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let wall = on_battlefield(&engine, p0, wall_of_distortion()).expect("Wall of Distortion");
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    {
        assert_eq!(player, p1);
        engine
            .apply(
                p1,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before - 1
    );
}
