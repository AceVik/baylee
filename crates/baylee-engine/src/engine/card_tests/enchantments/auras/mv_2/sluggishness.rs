#[allow(clippy::wildcard_imports)]
use super::*;

fn sluggishness() -> CardIndex {
    card_index("39eb4062-68a0-4f64-a9cc-79be87fd8173")
}

#[test]
fn sluggishness_grants_cant_block_and_returns_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), gray_ogre()])
        .hand(0, &[sluggishness(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, sluggishness());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, ogre).contains(baylee_cards_dsl::KeywordSet::CANT_BLOCK));

    cast_from_hand(&mut engine, p0, lightning_bolt());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, sluggishness()).is_some());
}
