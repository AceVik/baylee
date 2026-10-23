#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn cloud_of_faeries() -> CardIndex {
    card_index("adc1f3b4-cce7-42c9-a208-dbab1d1924f5")
}

#[test]
fn cloud_of_faeries_untaps_up_to_two_lands_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[cloud_of_faeries()])
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, cloud_of_faeries());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let lands: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index)
                == Some(island())
        })
        .collect();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: lands.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    for l in lands {
        assert!(
            !engine
                .state()
                .object(l)
                .unwrap()
                .status
                .contains(Status::TAPPED)
        );
    }
}

#[test]
fn cloud_of_faeries_can_untap_opponents_lands_and_can_cycle() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[],
        &[mountain(), forest(), island(), gray_ogre()],
        &[cloud_of_faeries(), cloud_of_faeries()],
        &[],
    );
    let land = lk::obj(&engine, p1, mountain());
    let second = lk::obj(&engine, p1, forest());
    let third = lk::obj(&engine, p1, island());
    for id in [land, second, third] {
        engine
            .dev_state_mut(p0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }
    lk::float(&mut engine, p0, &[(ManaColor::Blue, 4)]);
    cast_with_floating(&mut engine, p0, cloud_of_faeries());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        min,
        max,
        ref options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (0, 2));
    assert!(options.contains(&land));
    assert!(options.contains(&second));
    assert!(options.contains(&third));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![land, second, third]
                }
            )
            .is_err(),
        "at most two lands"
    );
    let ogre = lk::obj(&engine, p1, gray_ogre());
    assert!(!options.contains(&ogre));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land, second],
            },
        )
        .unwrap();
    lk::settle(&mut engine);
    assert!(!is_tapped(&engine, land));
    assert!(!is_tapped(&engine, second));
    assert!(is_tapped(&engine, third), "only chosen lands untap");
    let card = in_hand(&engine, p0, cloud_of_faeries()).unwrap();
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: card,
                ability_index: 1,
            },
        )
        .unwrap();
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, p0, cloud_of_faeries()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before
    );
}
