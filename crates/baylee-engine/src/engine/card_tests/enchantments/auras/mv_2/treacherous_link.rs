#[allow(clippy::wildcard_imports)]
use super::*;

fn treacherous_link() -> CardIndex {
    card_index("b7902113-9ddf-4d81-a76a-b54b8a36c154")
}

#[test]
fn treacherous_link_prevents_damage_to_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), gray_ogre()])
        .battlefield(1, &[earth_elemental()])
        .hand(0, &[treacherous_link()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let elemental = on_battlefield(&engine, p1, earth_elemental()).unwrap();
    cast_from_hand(&mut engine, p0, treacherous_link());
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

    reach_their_main_phase(&mut engine, p1);
    let _ = attack_and_collect_blocks(&mut engine, elemental, p0);
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, elemental)],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
}
