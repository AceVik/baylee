#[allow(clippy::wildcard_imports)]
use super::*;

fn walking_sponge() -> CardIndex {
    card_index("000e5d65-96c3-498b-bd01-72b1a1991850")
}

#[test]
fn walking_sponge_removes_flying_from_target_creature_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let flyer = baylee_core::generated::index::STORM_CROW;
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[walking_sponge()])
        .battlefield(1, &[flyer])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sponge = on_battlefield(&engine, p0, walking_sponge()).unwrap();
    let target = on_battlefield(&engine, p1, flyer).unwrap();
    assert!(keywords(&engine, target).contains(KeywordSet::FLYING));

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sponge,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, target).contains(KeywordSet::FLYING));
}
