#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn quicksilver_amulet() -> CardIndex {
    card_index("e1096a98-a631-43f5-97d8-325001d608b3")
}

#[test]
fn quicksilver_amulet_puts_creature_from_hand_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), quicksilver_amulet()],
        )
        .hand(0, &[earth_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let amulet = on_battlefield(&engine, p0, quicksilver_amulet()).unwrap();
    let elem = in_hand(&engine, p0, earth_elemental()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: amulet,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards {
        min, ref options, ..
    } = engine.pending().clone()
    {
        assert_eq!(min, 0, "may put a creature");
        for id in options {
            assert!(
                engine
                    .state()
                    .object(*id)
                    .unwrap()
                    .characteristics()
                    .types
                    .contains(TypeSet::CREATURE)
            );
        }
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![elem],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, earth_elemental()).is_some());
    assert!(is_tapped(&engine, amulet));
}

#[test]
fn quicksilver_amulet_may_put_nothing_into_play() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(
        SEED,
        &[quicksilver_amulet()],
        &[],
        &[gray_ogre(), plains()],
        &[],
    );
    lk::float(&mut engine, p0, &[(ManaColor::Colorless, 4)]);
    activate(&mut engine, p0, quicksilver_amulet(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    lk::settle(&mut engine);
    assert!(in_hand(&engine, p0, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
}
