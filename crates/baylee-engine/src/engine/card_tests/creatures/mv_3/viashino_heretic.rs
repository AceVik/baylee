//! `cards/creatures/mv_3/viashino_heretic.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn viashino_heretic() -> CardIndex {
    card_index("36896550-151d-472c-bfc1-1d9bbf7cc04d")
}

#[test]
fn viashino_heretic_destroys_target_artifact_and_deals_cmc_damage_to_its_controller() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let artifact = baylee_core::generated::index::JAYEMDAE_TOME;
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[viashino_heretic(), mountain(), mountain()])
        .battlefield(1, &[artifact, gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let heretic = on_battlefield(&engine, p0, viashino_heretic()).unwrap();
    let tome = on_battlefield(&engine, p1, artifact).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: heretic,
                ability_index: 0,
            },
        )
        .unwrap();
    let bare = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("artifact targets")
    };
    assert!(!options.contains(&bare));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![tome],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, artifact).is_none());
    assert!(in_graveyard(&engine, p1, artifact).is_some());
    assert_eq!(engine.state().players[1].life, 16);
}
