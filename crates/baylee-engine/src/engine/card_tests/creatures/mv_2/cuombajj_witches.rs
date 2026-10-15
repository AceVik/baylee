//! Oracle scenarios for Cuombajj Witches; battle defense remains an explicit hold.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;
use baylee_core::generated::index;

fn cuombajj_witches() -> CardIndex {
    card_index("638eeb16-9e0e-4cc6-b97e-8ff0df81ca58")
}

fn choose(
    e: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    object: Option<ObjectId>,
    player: Option<PlayerId>,
) {
    e.apply(
        seat,
        PlayerAction::ChooseTargets {
            objects: object.into_iter().collect(),
            players: player.into_iter().collect(),
        },
    )
    .unwrap();
}

#[test]
fn cuombajj_witches_targets_are_chosen_in_order_by_different_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for same in [false, true] {
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(0, &[cuombajj_witches()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, cuombajj_witches()).unwrap();
        activate(&mut e, p0, cuombajj_witches(), 0);
        assert_eq!(e.pending().asked(), Some(p0));
        choose(&mut e, p0, None, Some(p1));
        assert_eq!(e.pending().asked(), Some(p1));
        let Pending::ChooseTargets { player_options, .. } = e.pending() else {
            panic!("opponent's target")
        };
        assert!(player_options.contains(&p0));
        assert!(player_options.contains(&p1));
        let second = if same { p1 } else { p0 };
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![second]
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
        choose(&mut e, p1, None, Some(second));
        assert_eq!(e.state().players[0].life, 20);
        assert_eq!(e.state().players[1].life, 20);
        assert!(is_tapped(&e, source));
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, if same { 20 } else { 19 });
        assert_eq!(e.state().players[1].life, if same { 18 } else { 19 });
        assert_eq!(e.state().object(source).unwrap().damage, 0);
    }
}

#[test]
fn cuombajj_witches_each_damage_can_hit_a_creature_with_the_same_source() {
    use crate::event::DamageTarget;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for same in [false, true] {
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(0, &[cuombajj_witches(), index::FANGREN_HUNTER])
            .battlefield(1, &[index::HILL_GIANT, index::SOL_RING])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, cuombajj_witches()).unwrap();
        let own = on_battlefield(&e, p0, index::FANGREN_HUNTER).unwrap();
        let enemy = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
        let artifact = on_battlefield(&e, p1, index::SOL_RING).unwrap();
        let from = e.journal().entries().len();
        activate(&mut e, p0, cuombajj_witches(), 0);
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            panic!("any target")
        };
        assert!(options.contains(&own));
        assert!(options.contains(&enemy));
        assert!(!options.contains(&artifact));
        choose(&mut e, p0, Some(enemy), None);
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            panic!("opponent's any target")
        };
        assert!(options.contains(&own));
        assert!(options.contains(&enemy));
        assert!(!options.contains(&artifact));
        choose(&mut e, p1, Some(if same { enemy } else { own }), None);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(enemy).unwrap().damage,
            if same { 2 } else { 1 }
        );
        assert_eq!(e.state().object(own).unwrap().damage, u16::from(!same));
        let events = damage_events(&e, from);
        assert_eq!(events.iter().map(|event| event.2).sum::<u32>(), 2);
        assert!(events.iter().all(|&(s, t, _, combat)| s == source
            && !combat
            && matches!(t,DamageTarget::Object(id) if id==enemy || id==own)));
        assert_eq!(e.state().players[0].life, 20);
        assert_eq!(e.state().players[1].life, 20);
    }
}

#[test]
fn cuombajj_witches_planeswalker_is_legal_for_either_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for second_walker in [false, true] {
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(0, &[cuombajj_witches()])
            .battlefield(1, &[index::KARN_THE_GREAT_CREATOR])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let walker = on_battlefield(&e, p1, index::KARN_THE_GREAT_CREATOR).unwrap();
        let loyalty = e
            .state()
            .object(walker)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty);
        activate(&mut e, p0, cuombajj_witches(), 0);
        choose(
            &mut e,
            p0,
            if second_walker { None } else { Some(walker) },
            if second_walker { Some(p1) } else { None },
        );
        choose(
            &mut e,
            p1,
            if second_walker { Some(walker) } else { None },
            if second_walker { None } else { Some(p0) },
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state()
                .object(walker)
                .unwrap()
                .counters
                .get(CounterKind::Loyalty),
            loyalty - 1
        );
        assert_eq!(
            e.state().players[0].life,
            if second_walker { 20 } else { 19 }
        );
        assert_eq!(
            e.state().players[1].life,
            if second_walker { 19 } else { 20 }
        );
    }
}

#[test]
fn cuombajj_witches_surviving_target_resolves_when_either_target_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for remove_first in [false, true] {
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(0, &[cuombajj_witches(), index::FANGREN_HUNTER])
            .battlefield(1, &[index::HILL_GIANT])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let first = on_battlefield(&e, p0, index::FANGREN_HUNTER).unwrap();
        let second = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
        activate(&mut e, p0, cuombajj_witches(), 0);
        choose(&mut e, p0, Some(first), None);
        choose(&mut e, p1, Some(second), None);
        crate::sba::destroy(
            e.dev_state_mut(p0).unwrap(),
            if remove_first { first } else { second },
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state()
                .object(if remove_first { second } else { first })
                .unwrap()
                .damage,
            1
        );
    }
}

#[test]
fn cuombajj_witches_tapped_or_summoning_sick_cannot_activate() {
    use crate::object::Status;
    let p0 = PlayerId::new(0);
    for sick in [false, true] {
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(0, &[cuombajj_witches()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, cuombajj_witches()).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        if sick {
            state.object_mut(source).unwrap().controlled_since =
                state.players[0].turn_start_timestamp + 1;
        } else {
            state
                .object_mut(source)
                .unwrap()
                .status
                .insert(Status::TAPPED);
        }
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.contains(&(source, 0)));
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
    }
}

#[test]
fn cuombajj_witches_opponent_chooses_targets_for_the_controllers_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[cuombajj_witches(), index::CARNAGE_TYRANT])
        .battlefield(1, &[index::CARNAGE_TYRANT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let original_own = on_battlefield(&e, p0, index::CARNAGE_TYRANT).unwrap();
    let original_enemy = on_battlefield(&e, p1, index::CARNAGE_TYRANT).unwrap();
    crate::resolve::gain_control(
        e.dev_state_mut(p0).unwrap(),
        &[(original_own, p1), (original_enemy, p0)],
    );
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    let (own, enemy) = (original_enemy, original_own);
    assert_eq!(e.state().object(own).unwrap().owner, p1);
    assert_eq!(e.state().object(enemy).unwrap().owner, p0);
    activate(&mut e, p0, cuombajj_witches(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("first any target")
    };
    assert!(options.contains(&own));
    assert!(!options.contains(&enemy));
    choose(&mut e, p0, None, Some(p1));
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("second any target")
    };
    assert!(options.contains(&own));
    assert!(!options.contains(&enemy));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![enemy],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    choose(&mut e, p1, Some(own), None);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(own).unwrap().damage, 1);
    assert_eq!(e.state().object(enemy).unwrap().damage, 0);
    assert_eq!(e.state().players[1].life, 19);
}

#[test]
fn cuombajj_witches_ability_remains_after_the_source_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[cuombajj_witches()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, cuombajj_witches()).unwrap();
    activate(&mut e, p0, cuombajj_witches(), 0);
    choose(&mut e, p0, None, Some(p1));
    choose(&mut e, p1, None, Some(p0));
    crate::sba::destroy(e.dev_state_mut(p0).unwrap(), source);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(e.state().players[1].life, 19);
}

#[test]
fn cuombajj_witches_controller_selects_the_opponent_independently_of_first_target() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut e = Duel::table(SEED, index::FOREST, 3)
        .battlefield(0, &[cuombajj_witches()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, cuombajj_witches(), 0);
    choose(&mut e, p0, None, Some(p1));
    let Pending::ChoosePlayer { player, options } = e.pending().clone() else {
        panic!("choose opponent after first target")
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![p1, p2]);
    let before = e.snapshot_hash();
    assert!(e.apply(p0, PlayerAction::ChoosePlayer(p0)).is_err());
    assert_eq!(e.snapshot_hash(), before);
    e.apply(p0, PlayerAction::ChoosePlayer(p2)).unwrap();
    assert_eq!(e.pending().asked(), Some(p2));
    choose(&mut e, p2, None, Some(p0));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(e.state().players[1].life, 19);
    assert_eq!(e.state().players[2].life, 20);
}
