//! Independent Alpha instants review.

#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

#[test]
fn fork_independent_copy_keeps_the_original_sacrificed_creatures_mana_value() {
    let p0 = PlayerId::new(0);
    let sacrifice = card_index("068b3692-411b-44d4-a7e9-005262760cfc");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), mountain(), mountain(), clockwork_beast()])
        .hand(0, &[sacrifice, fork()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let beast = on_battlefield(&engine, p0, clockwork_beast()).unwrap();
    cast_from_hand(&mut engine, p0, sacrifice);
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("sacrifice cost expected: {:?}", engine.pending());
    };
    assert!(options.contains(&beast));
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![beast],
            },
        )
        .unwrap();
    let original = top(&engine);
    cast_with_floating(&mut engine, p0, fork());
    aim(&mut engine, vec![original], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        12,
        "original and copy each use the sacrificed six-mana creature"
    );
    assert!(
        in_graveyard(&engine, p0, clockwork_beast()).is_some(),
        "only the original cost sacrifices a creature"
    );
}

#[test]
fn fork_independent_can_swap_two_targets_and_retains_original_target_count() {
    let p0 = PlayerId::new(0);
    let body = quiet_creature();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                mountain(),
                mountain(),
                body,
                body,
                body,
            ],
        )
        .hand(0, &[eerie_interlude(), fork()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creatures = all_on_battlefield(&engine, p0, body);
    cast_from_hand(&mut engine, p0, eerie_interlude());
    aim(&mut engine, vec![creatures[0], creatures[1]], vec![]);
    let original = top(&engine);
    cast_with_floating(&mut engine, p0, fork());
    aim(&mut engine, vec![original], vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, max, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(max, 1, "one original target is edited per question");
    assert!(
        options.contains(&creatures[1]),
        "a target that will move in the final set may be chosen for a swap"
    );
    aim(&mut engine, vec![creatures[1]], vec![]);
    aim(&mut engine, vec![creatures[0]], vec![]);
    let copy = top(&engine);
    assert_eq!(
        engine.state().object(copy).unwrap().targets.as_slice(),
        &[creatures[1], creatures[0]],
        "copy retains exactly two targets, with the legal swap"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(creatures[0]).unwrap().zone,
        Zone::Exile
    );
    assert_eq!(
        engine.state().object(creatures[1]).unwrap().zone,
        Zone::Exile
    );
    assert_eq!(
        engine.state().object(creatures[2]).unwrap().zone,
        Zone::Battlefield,
        "a third target cannot be added to the copy"
    );
}

#[test]
fn fork_independent_retargets_both_fight_target_groups_for_the_copy_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let ambush = card_index("37a55560-6e32-4f54-b9a8-fd157aea6eb5");
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), body])
        .hand(0, &[ambush])
        .battlefield(1, &[mountain(), mountain(), body, clockwork_beast()])
        .hand(1, &[fork()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let original_fighter = on_battlefield(&engine, p0, body).unwrap();
    let original_victim = on_battlefield(&engine, p1, body).unwrap();
    let copy_fighter = on_battlefield(&engine, p1, clockwork_beast()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, ambush);
    aim(&mut engine, vec![original_fighter], vec![]);
    aim(&mut engine, vec![original_victim], vec![]);
    let original = top(&engine);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, fork());
    aim(&mut engine, vec![original], vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        options.contains(&copy_fighter),
        "copy's first group uses copy controller"
    );
    aim(&mut engine, vec![copy_fighter], vec![]);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "second group must also be retargetable: {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&original_fighter),
        "copy's second group offers opponents' creatures"
    );
    assert!(
        !options.contains(&original_victim),
        "second target must not be controlled by copy controller"
    );
    aim(&mut engine, vec![original_fighter], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, body).is_some(),
        "copy fights new second target"
    );
    assert_eq!(engine.state().object(copy_fighter).unwrap().damage, 2);
    assert_eq!(
        engine.state().object(original_victim).unwrap().damage,
        0,
        "original fight has lost its first target"
    );
}

#[test]
fn power_sink_independent_taps_a_land_with_a_granted_mana_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let maze = card_index("38a12bd7-4394-44a8-91a0-6a4ff7fa4f71");
    let sapphire = card_index("d5ed1233-df87-4b90-8918-13922ec95249");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), maze])
        .hand(0, &[llanowar_elves()])
        .battlefield(
            1,
            &[island(), island(), yavimaya_cradle_of_growth(), sapphire],
        )
        .hand(1, &[power_sink()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let maze_id = on_battlefield(&engine, p0, maze).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        legal.mana_abilities.contains(&maze_id),
        "Yavimaya grants Forest mana to Maze"
    );
    tap_mana_except(&mut engine, p0, maze_id);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    let original = top(&engine);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    sink(&mut engine, p1, original, 1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(maze_id)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "granted mana ability is enough for forced tap"
    );
}

#[test]
fn power_sink_independent_does_not_tap_a_land_that_lost_its_mana_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let revolt = card_index("0df0503b-25e7-499b-a703-a867f2da790c");
    let thopter = card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba");
    let sapphire = card_index("d5ed1233-df87-4b90-8918-13922ec95249");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[thopter])
        .battlefield(1, &[revolt, plains(), sol_ring(), sapphire, sapphire])
        .hand(1, &[final_showdown(), power_sink()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, forest()).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, final_showdown());
    choose_modes(&mut engine, p1, 1);
    pass_until(&mut engine, stack_is_empty);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        !legal.mana_abilities.contains(&land),
        "animated Forest lost all abilities"
    );
    cast_with_floating(&mut engine, p0, thopter);
    let original = top(&engine);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    sink(&mut engine, p1, original, 1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !engine
            .state()
            .object(land)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "a land with no remaining mana ability is spared"
    );
}

#[test]
fn power_sink_independent_uncounterable_spell_still_taps_lands_and_loses_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let verdict = card_index("0230de18-8d15-4cfa-9d42-7ccddd9f9570");
    let body = quiet_creature();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), island(), forest(), forest(), forest()],
        )
        .hand(0, &[verdict])
        .battlefield(1, &[island(), island(), island(), body])
        .hand(1, &[power_sink()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spare = on_battlefield(&engine, p0, forest()).unwrap();
    tap_mana_except(&mut engine, p0, spare);
    cast_with_floating(&mut engine, p0, verdict);
    let original = top(&engine);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    sink(&mut engine, p1, original, 2);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(spare)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(
        in_graveyard(&engine, p1, body).is_some(),
        "uncounterable Verdict resolves and destroys creature"
    );
}

#[test]
fn fork_independent_player_target_can_change_to_its_original_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(1, &[fork()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, vec![], vec![p1]);
    let original = top(&engine);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, fork());
    aim(&mut engine, vec![original], vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, p1, "copy controller chooses");
    assert!(player_options.contains(&p0));
    aim(&mut engine, vec![], vec![p0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        17,
        "retargeted copy hits original caster"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "original spell retains its target"
    );
}

#[test]
fn fork_independent_player_target_can_be_kept() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(1, &[fork()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, vec![], vec![p1]);
    let original = top(&engine);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, fork());
    aim(&mut engine, vec![original], vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { min, .. } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(*min, 0, "empty answer keeps the target");
    aim(&mut engine, vec![], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(
        engine.state().players[1].life,
        14,
        "may retain original player target"
    );
}

#[test]
fn power_sink_independent_decline_taps_only_mana_lands_and_empties_pool() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let maze = card_index("38a12bd7-4394-44a8-91a0-6a4ff7fa4f71");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), maze])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(); 3])
        .hand(1, &[power_sink()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spare = on_battlefield(&engine, p0, forest()).unwrap();
    let maze_id = on_battlefield(&engine, p0, maze).unwrap();
    tap_mana_except(&mut engine, p0, spare);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    let original = top(&engine);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(
        !engine
            .state()
            .object(spare)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    sink(&mut engine, p1, original, 2);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(spare)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "unpaid tax taps the spare mana land"
    );
    assert!(
        !engine
            .state()
            .object(maze_id)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Maze has no mana ability"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "all unspent mana lost"
    );
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
}

#[test]
fn power_sink_independent_payment_preserves_spare_lands_and_excess_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(); 2])
        .hand(1, &[power_sink()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spare = on_battlefield(&engine, p0, forest()).unwrap();
    tap_mana_except(&mut engine, p0, spare);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    let original = top(&engine);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    sink(&mut engine, p1, original, 1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !engine
            .state()
            .object(spare)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "only the tax is paid"
    );
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}

#[test]
fn fork_independent_counterspell_copy_can_target_the_resolving_fork() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 3])
        .hand(0, &[lightning_bolt(), fork()])
        .battlefield(1, &[island(); 2])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, vec![], vec![p1]);
    let bolt = top(&engine);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, counterspell());
    aim(&mut engine, vec![bolt], vec![]);
    let counter = top(&engine);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut engine, p0, fork());
    let fork_spell = in_hand(&engine, p0, fork()).unwrap_or_else(|| top(&engine));
    aim(&mut engine, vec![counter], vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        options.contains(&fork_spell),
        "Fork remains a legal target while its copy's targets are chosen"
    );
    aim(&mut engine, vec![fork_spell], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "copy target is gone; original Counterspell still counters Bolt"
    );
    assert!(in_graveyard(&engine, p0, lightning_bolt()).is_some());
}
