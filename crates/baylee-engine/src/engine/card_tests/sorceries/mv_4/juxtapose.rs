//! Oracle scenarios for Juxtapose: two sequential, independent exchanges.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;
use baylee_core::generated::index;

fn juxtapose() -> CardIndex {
    card_index("a52ccb40-e950-4cd0-93ab-50daa52b03f3")
}

fn cast_at(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    let p0 = PlayerId::new(0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Blue, 4);
    e.refresh_offer();
    cast_from_hand(e, p0, juxtapose());
    let Pending::ChoosePlayer {
        player: chooser,
        options,
    } = e.pending()
    else {
        panic!("target player, got {:?}", e.pending());
    };
    assert_eq!(*chooser, p0);
    assert!(options.contains(&PlayerId::new(0)));
    assert!(options.contains(&PlayerId::new(1)));
    e.apply(p0, PlayerAction::ChoosePlayer(player)).unwrap();
}

#[test]
fn juxtapose_exchanges_greatest_mana_values_in_both_categories_indefinitely() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                fangren_hunter(),
                gray_ogre(),
                index::JAYEMDAE_TOME,
                sol_ring(),
                forest(),
            ],
        )
        .battlefield(
            1,
            &[
                index::HILL_GIANT,
                index::GRIZZLY_BEARS,
                index::ICY_MANIPULATOR,
                sol_ring(),
                forest(),
            ],
        )
        .hand(0, &[juxtapose()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let exchanged = [
        (on_battlefield(&e, p0, fangren_hunter()).unwrap(), p1, p0),
        (on_battlefield(&e, p1, index::HILL_GIANT).unwrap(), p0, p1),
        (
            on_battlefield(&e, p0, index::JAYEMDAE_TOME).unwrap(),
            p1,
            p0,
        ),
        (
            on_battlefield(&e, p1, index::ICY_MANIPULATOR).unwrap(),
            p0,
            p1,
        ),
    ];
    let unchanged = [
        (on_battlefield(&e, p0, gray_ogre()).unwrap(), p0),
        (on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap(), p1),
        (on_battlefield(&e, p0, sol_ring()).unwrap(), p0),
        (on_battlefield(&e, p1, sol_ring()).unwrap(), p1),
        (on_battlefield(&e, p0, forest()).unwrap(), p0),
    ];
    cast_at(&mut e, p1);
    pass_until(&mut e, stack_is_empty);
    for &(id, controller, owner) in &exchanged {
        assert_eq!(e.state().object(id).unwrap().controller, controller);
        assert_eq!(e.state().object(id).unwrap().owner, owner);
    }
    for &(id, controller) in &unchanged {
        assert_eq!(e.state().object(id).unwrap().controller, controller);
    }
    pass_until(&mut e, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    for &(id, controller, owner) in &exchanged {
        assert_eq!(e.state().object(id).unwrap().controller, controller);
        assert_eq!(e.state().object(id).unwrap().owner, owner);
    }
}

#[test]
fn juxtapose_missing_one_category_does_not_prevent_the_other_exchange() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for missing_creature in [false, true] {
        let mine = if missing_creature {
            sol_ring()
        } else {
            gray_ogre()
        };
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[mine])
            .battlefield(1, &[fangren_hunter(), index::JAYEMDAE_TOME])
            .hand(0, &[juxtapose()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let own = on_battlefield(&e, p0, mine).unwrap();
        let creature = on_battlefield(&e, p1, fangren_hunter()).unwrap();
        let artifact = on_battlefield(&e, p1, index::JAYEMDAE_TOME).unwrap();
        cast_at(&mut e, p1);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(own).unwrap().controller, p1);
        assert_eq!(
            e.state().object(creature).unwrap().controller,
            if missing_creature { p1 } else { p0 }
        );
        assert_eq!(
            e.state().object(artifact).unwrap().controller,
            if missing_creature { p0 } else { p1 }
        );
    }
}

#[test]
fn juxtapose_artifact_exchange_sees_the_completed_creature_exchange() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::JUGGERNAUT, sol_ring()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[juxtapose()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let juggernaut = on_battlefield(&e, p0, index::JUGGERNAUT).unwrap();
    let ring = on_battlefield(&e, p0, sol_ring()).unwrap();
    let ogre = on_battlefield(&e, p1, gray_ogre()).unwrap();
    cast_at(&mut e, p1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(juggernaut).unwrap().controller,
        p0,
        "exchanged twice"
    );
    assert_eq!(e.state().object(ring).unwrap().controller, p1);
    assert_eq!(e.state().object(ogre).unwrap().controller, p0);
    assert_eq!(e.state().object(juggernaut).unwrap().owner, p0);
}

#[test]
fn juxtapose_can_target_its_caster_without_changing_control() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[gray_ogre(), sol_ring()])
        .hand(0, &[juxtapose()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let ring = on_battlefield(&e, p0, sol_ring()).unwrap();
    cast_at(&mut e, p0);
    pass_until(&mut e, stack_is_empty);
    for id in [ogre, ring] {
        assert_eq!(e.state().object(id).unwrap().controller, p0);
    }
}

#[test]
fn juxtapose_each_controller_breaks_their_own_ties_in_each_category() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::HILL_GIANT,
                index::WAR_MAMMOTH,
                index::JAYEMDAE_TOME,
                index::ICY_MANIPULATOR,
                sol_ring(),
            ],
        )
        .battlefield(
            1,
            &[
                index::HILL_GIANT,
                index::WAR_MAMMOTH,
                index::JAYEMDAE_TOME,
                index::ICY_MANIPULATOR,
                gray_ogre(),
            ],
        )
        .hand(0, &[juxtapose()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let chosen = [
        on_battlefield(&e, p0, index::WAR_MAMMOTH).unwrap(),
        on_battlefield(&e, p1, index::HILL_GIANT).unwrap(),
        on_battlefield(&e, p0, index::ICY_MANIPULATOR).unwrap(),
        on_battlefield(&e, p1, index::JAYEMDAE_TOME).unwrap(),
    ];
    let rejected = [
        on_battlefield(&e, p0, index::HILL_GIANT).unwrap(),
        on_battlefield(&e, p1, index::WAR_MAMMOTH).unwrap(),
        on_battlefield(&e, p0, index::JAYEMDAE_TOME).unwrap(),
        on_battlefield(&e, p1, index::ICY_MANIPULATOR).unwrap(),
    ];
    cast_at(&mut e, p1);
    for (n, &id) in chosen.iter().enumerate() {
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = e.pending().clone()
        else {
            unreachable!()
        };
        let controller = if n % 2 == 0 { p0 } else { p1 };
        assert_eq!(player, controller);
        assert_eq!((min, max), (1, 1));
        assert_eq!(options.len(), 2);
        assert!(options.contains(&id));
        assert!(options.contains(&rejected[n]));
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                if controller == p0 { p1 } else { p0 },
                PlayerAction::ChooseObjects { objects: vec![id] }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
        e.apply(
            controller,
            PlayerAction::ChooseObjects { objects: vec![id] },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    for (n, &id) in chosen.iter().enumerate() {
        assert_eq!(
            e.state().object(id).unwrap().controller,
            if n % 2 == 0 { p1 } else { p0 }
        );
    }
    for (n, &id) in rejected.iter().enumerate() {
        assert_eq!(
            e.state().object(id).unwrap().controller,
            if n % 2 == 0 { p0 } else { p1 }
        );
    }
}

#[test]
fn juxtapose_uses_current_controller_and_can_exchange_hexproof_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[gray_ogre()])
        .battlefield(1, &[index::CARNAGE_TYRANT, fangren_hunter()])
        .hand(0, &[juxtapose()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let tyrant = on_battlefield(&e, p1, index::CARNAGE_TYRANT).unwrap();
    let hunter = on_battlefield(&e, p1, fangren_hunter()).unwrap();
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    crate::resolve::gain_control(e.dev_state_mut(p0).unwrap(), &[(hunter, p0)]);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    cast_at(&mut e, p1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(tyrant).unwrap().controller, p0);
    assert_eq!(e.state().object(tyrant).unwrap().owner, p1);
    assert_eq!(e.state().object(hunter).unwrap().controller, p1);
    assert_eq!(e.state().object(ogre).unwrap().controller, p0);
}

#[test]
fn juxtapose_compares_mana_value_rather_than_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[fangren_hunter(), index::JUGGERNAUT])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[juxtapose()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let hunter = on_battlefield(&e, p0, fangren_hunter()).unwrap();
    let juggernaut = on_battlefield(&e, p0, index::JUGGERNAUT).unwrap();
    let ogre = on_battlefield(&e, p1, gray_ogre()).unwrap();
    cast_at(&mut e, p1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(hunter).unwrap().controller,
        p1,
        "MV5 Hunter beats MV4 Juggernaut despite lower power"
    );
    assert_eq!(
        e.state().object(juggernaut).unwrap().controller,
        p0,
        "no opposing artifact to exchange"
    );
    assert_eq!(e.state().object(ogre).unwrap().controller, p0);
}

#[test]
fn juxtapose_snapshots_distinguish_first_choice_while_second_player_decides() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let setup = || {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::HILL_GIANT, index::WAR_MAMMOTH])
            .battlefield(1, &[index::HILL_GIANT, index::WAR_MAMMOTH])
            .hand(0, &[juxtapose()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        cast_at(&mut e, p1);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        e
    };
    let mut e = setup();
    let mut other = setup();
    assert_eq!(e.snapshot_hash(), other.snapshot_hash());
    let giant = on_battlefield(&e, p0, index::HILL_GIANT).unwrap();
    let mammoth = on_battlefield(&e, p0, index::WAR_MAMMOTH).unwrap();
    let enemy = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();
    other
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mammoth],
            },
        )
        .unwrap();
    for branch in [&e, &other] {
        assert_eq!(branch.pending().asked(), Some(p1));
        assert_eq!(branch.state().object(giant).unwrap().controller, p0);
        assert_eq!(branch.state().object(mammoth).unwrap().controller, p0);
    }
    assert_ne!(
        e.snapshot_hash(),
        other.snapshot_hash(),
        "suspended first selections affect future behavior"
    );
    for branch in [&mut e, &mut other] {
        branch
            .apply(
                p1,
                PlayerAction::ChooseObjects {
                    objects: vec![enemy],
                },
            )
            .unwrap();
        pass_until(branch, stack_is_empty);
    }
    assert_eq!(e.state().object(giant).unwrap().controller, p1);
    assert_eq!(e.state().object(mammoth).unwrap().controller, p0);
    assert_eq!(other.state().object(giant).unwrap().controller, p0);
    assert_eq!(other.state().object(mammoth).unwrap().controller, p1);
}
