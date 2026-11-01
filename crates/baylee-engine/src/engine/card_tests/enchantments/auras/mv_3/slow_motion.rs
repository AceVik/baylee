//! `cards/enchantments/auras/mv_3/slow_motion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn slow_motion() -> CardIndex {
    card_index("151388a4-a24b-4089-a323-e7c1d5e6490a")
}

#[test]
fn slow_motion_sacrifices_unpaid_and_returns_to_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[slow_motion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, slow_motion());
    if let Pending::ChooseTargets { options, .. } = engine.pending() {
        assert!(options.contains(&ogre));
        for land in engine.state().zones.list(ZoneLocation::Battlefield) {
            if engine
                .state()
                .object(*land)
                .unwrap()
                .characteristics()
                .types
                .contains(baylee_core::types::TypeSet::LAND)
            {
                assert!(!options.contains(land), "enchant creature excludes lands");
            }
        }
    }
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

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert!(in_hand(&engine, p0, slow_motion()).is_some());
}

#[test]
fn slow_motion_allows_the_creatures_controller_to_pay() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[gray_ogre()], &[slow_motion()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Blue, 3)]);
    let ogre = lk::obj(&engine, lk::P1, gray_ogre());
    lk::cast_at(&mut engine, lk::P0, slow_motion(), &[ogre], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, lk::P1, "the enchanted creature's controller pays");
    engine.dev_state_mut(lk::P1).unwrap().players[1]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    engine.apply(lk::P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, lk::P1, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, lk::P0, slow_motion()).is_some());
    reach_their_main_phase(&mut engine, lk::P0);
    assert!(
        on_battlefield(&engine, lk::P1, gray_ogre()).is_some(),
        "Aura controller's upkeep does not tax it"
    );
}
