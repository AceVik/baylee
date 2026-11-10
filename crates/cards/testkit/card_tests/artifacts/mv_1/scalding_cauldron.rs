//! Oracle checklist: pay {3}; tap and sacrifice Cauldron as activation costs; target creature only; deal exactly 3 damage; preserve a larger target and an unrelated creature.
#[allow(clippy::wildcard_imports)]
use super::*;
fn scalding_cauldron() -> CardIndex {
    card_index("dceb96a4-d3c5-43a2-93f6-50316e1e7a1b")
}
#[test]
fn cauldron_sacrifices_itself_and_deals_three_to_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                scalding_cauldron(),
                forest(),
                forest(),
                forest(),
                forest(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[earth_elemental(), gray_ogre(), sol_ring()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let large = on_battlefield(&e, p1, earth_elemental()).unwrap();
    let small = on_battlefield(&e, p1, gray_ogre()).unwrap();
    let own = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let artifact = on_battlefield(&e, p1, sol_ring()).unwrap();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, scalding_cauldron(), 0);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending()
    else {
        panic!("the activated ability asks for a creature target")
    };
    assert!(options.contains(&large));
    assert!(options.contains(&small));
    assert!(options.contains(&own));
    assert!(!options.contains(&artifact));
    assert!(player_options.is_empty());
    aim_at(&mut e, p0, large);
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Green),
        1
    );
    assert!(
        in_graveyard(&e, p0, scalding_cauldron()).is_some(),
        "sacrifice is paid on activation"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(large).unwrap().damage, 3);
    assert_eq!(e.state().object(own).unwrap().damage, 0);
    assert_eq!(e.state().object(small).unwrap().damage, 0);
    assert!(on_battlefield(&e, p1, gray_ogre()).is_some());
}

#[test]
fn insufficient_mana_and_a_tapped_cauldron_are_not_offered() {
    let p0 = PlayerId::new(0);
    for tapped in [false, true] {
        let mut board = vec![scalding_cauldron()];
        board.extend(vec![forest(); if tapped { 3 } else { 2 }]);
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(1, &[gray_ogre()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, scalding_cauldron()).unwrap();
        if tapped {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(source)
                .unwrap()
                .status
                .insert(Status::TAPPED);
            e.refresh_offer();
        }
        assert_eq!(tap_all_mana(&mut e, p0), if tapped { 3 } else { 2 });
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("the player has priority")
        };
        assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
        assert!(on_battlefield(&e, p0, scalding_cauldron()).is_some());
    }
}
