#[allow(clippy::wildcard_imports)]
use super::*;

fn intervene() -> CardIndex {
    card_index("380b386f-90c8-45c5-a611-1ed6803b52d2")
}

#[test]
fn intervene_counters_target_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), gray_ogre()])
        .battlefield(1, &[mountain()])
        .hand(0, &[intervene()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let bolt_id = in_hand(&engine, p1, lightning_bolt()).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, p0, intervene());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, bolt_id);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
}
