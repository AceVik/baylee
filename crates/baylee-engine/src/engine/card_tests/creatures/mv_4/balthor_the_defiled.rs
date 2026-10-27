//! `cards/creatures/mv_4/balthor_the_defiled.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn balthor_the_defiled_pumps_minions_and_exiles_to_return_each_players_black_and_red_creatures() {
    let balthor = card_index("dfd8e6c2-e425-41fa-af55-985271f771b3");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                index::CABAL_TRAINEE,
                index::DRUDGE_SKELETONS,
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[gray_ogre(), savannah_lions()])
        .hand(0, &[balthor])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p0_dead = [
        on_battlefield(&engine, p0, index::DRUDGE_SKELETONS).unwrap(),
        on_battlefield(&engine, p0, grizzly_bears()).unwrap(),
    ];
    let p1_dead = [
        on_battlefield(&engine, p1, gray_ogre()).unwrap(),
        on_battlefield(&engine, p1, savannah_lions()).unwrap(),
    ];
    bury(&mut engine, &p0_dead);
    bury(&mut engine, &p1_dead);

    let trainee = on_battlefield(&engine, p0, index::CABAL_TRAINEE).unwrap();
    assert_eq!(pt(&engine, trainee), (1, 1));

    cast_from_hand(&mut engine, p0, balthor);
    pass_until(&mut engine, stack_is_empty);

    let balthor_perm = on_battlefield(&engine, p0, balthor).expect("Balthor on battlefield");
    assert_eq!(pt(&engine, trainee), (2, 2));

    let untapped_swamps: Vec<_> = all_on_battlefield(&engine, p0, swamp())
        .into_iter()
        .filter(|&id| !is_tapped(&engine, id))
        .take(3)
        .collect();
    for s in untapped_swamps {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: balthor_perm,
                ability_index: 1,
            },
        )
        .expect("activate Balthor");
    assert_eq!(
        engine.state().object(balthor_perm).unwrap().zone,
        Zone::Exile,
        "Balthor exiled as cost"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::DRUDGE_SKELETONS).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p1, savannah_lions()).is_some());
}
