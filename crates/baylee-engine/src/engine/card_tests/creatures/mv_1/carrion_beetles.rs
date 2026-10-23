#[allow(clippy::wildcard_imports)]
use super::*;

fn carrion_beetles() -> CardIndex {
    card_index("d64a51be-2dd1-485f-98fa-c93d4c296e68")
}

#[test]
fn carrion_beetles_exiles_up_to_three_cards_from_single_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(1, &[grizzly_bears(), gray_ogre()])
        .battlefield(0, &[carrion_beetles(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p1, grizzly_bears());
    hand_to_graveyard(&mut engine, p1, gray_ogre());
    let beetles = on_battlefield(&engine, p0, carrion_beetles()).expect("seated");
    let g1 = in_graveyard(&engine, p1, grizzly_bears()).expect("in gy");
    let g2 = in_graveyard(&engine, p1, gray_ogre()).expect("in gy");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: beetles,
                ability_index: 0,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![g1, g2],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_none());
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p1)).len(), 2);
}
