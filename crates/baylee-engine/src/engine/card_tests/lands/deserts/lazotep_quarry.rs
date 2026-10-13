//! `cards/lands/deserts/lazotep_quarry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn lazotep_quarry_sacrifices_creature_to_produce_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(307, forest())
        .battlefield(0, &[lazotep_quarry(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    activate(&mut engine, p0, lazotep_quarry(), 1);

    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert!(options.contains(&elf));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    let quarry = on_battlefield(&engine, p0, lazotep_quarry()).expect("quarry on battlefield");
    assert!(is_tapped(&engine, quarry));
}

#[test]
fn quarry_exiles_the_announced_value_and_copies_abilities_with_zombie_exceptions() {
    use baylee_core::generated::{index, subtypes};
    let p0 = PlayerId::new(0);
    for doubled in [false, true] {
        for gone in [false, true] {
            let mut board = vec![lazotep_quarry(), llanowar_elves(), index::GRIZZLY_BEARS];
            if doubled {
                board.push(index::DOUBLING_SEASON);
            }
            let mut e = Duel::new(SEED, forest()).battlefield(0, &board).start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, p0);
            let land = on_battlefield(&e, p0, lazotep_quarry()).unwrap();
            let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
            let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
            bury(&mut e, &[elf, bear]);
            e.dev_state_mut(p0).unwrap().players[0]
                .mana_pool
                .add(ManaColor::Colorless, 3);
            e.refresh_offer();
            activate(&mut e, p0, lazotep_quarry(), 2);
            assert!(matches!(e.pending(), Pending::ChooseNumber { .. }));
            e.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
            let Pending::ChooseTargets { options, .. } = e.pending() else {
                panic!("targets: {:?}", e.pending())
            };
            assert!(options.contains(&elf));
            assert!(!options.contains(&bear));
            aim_at(&mut e, p0, elf);
            assert!(matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::CostSacrifice,
                    ..
                }
            ));
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![land],
                },
            )
            .unwrap();
            if gone {
                e.dev_state_mut(p0)
                    .unwrap()
                    .move_object(
                        elf,
                        ZoneLocation::Exile(p0),
                        ZonePosition::Top,
                        Cause::Effect,
                    )
                    .unwrap();
            }
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
            assert_eq!(e.state().object(elf).unwrap().zone, Zone::Exile);
            let copies: Vec<_> = e
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .copied()
                .filter(|id| e.state().object(*id).unwrap().card.is_none())
                .collect();
            assert_eq!(
                copies.len(),
                if gone {
                    0
                } else if doubled {
                    2
                } else {
                    1
                }
            );
            for id in copies {
                let object = e.state().object(id).unwrap();
                let chars = object.characteristics();
                assert_eq!(pt(&e, id), (4, 4));
                assert_eq!(
                    chars.colors,
                    baylee_core::color::ColorSet::of(baylee_core::color::Color::Black)
                );
                assert!(chars.subtypes.contains(subtypes::creature::ZOMBIE));
                assert!(!chars.subtypes.contains(subtypes::creature::ELF));
                assert!(!chars.subtypes.contains(subtypes::creature::DRUID));
                assert!(
                    e.state()
                        .printed_ability_list(id)
                        .unwrap()
                        .abilities
                        .iter()
                        .any(baylee_cards_dsl::AbilityDef::is_mana_ability)
                );
            }
        }
    }
}

/// A game at p0's main phase: a Lazotep Quarry (and `extra` beside it) and a
/// creature card in p0's graveyard for the exile cost's target.
fn quarry_game(extra: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut board = vec![lazotep_quarry()];
    board.extend_from_slice(extra);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(
            0,
            &[
                llanowar_elves(),
                baylee_core::generated::index::LIGHTNING_BOLT,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, lazotep_quarry()).unwrap();
    let elf = bury_from_hand(&mut e, p0, llanowar_elves());
    (e, land, elf)
}

fn float_for_quarry(e: &mut Engine<RegistryLookup>) {
    e.dev_state_mut(PlayerId::new(0)).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
}

fn quarry_exile_offered(e: &Engine<RegistryLookup>, land: ObjectId) -> bool {
    let Pending::Priority { player, legal } = e.pending() else {
        return false;
    };
    *player == PlayerId::new(0) && legal.abilities.contains(&(land, 2))
}

/// "Activate only as a sorcery": p0's creature card and the mana are there the
/// whole time, so the only thing that changes between the four checks is the
/// timing. Offered in p0's main phase with an empty stack; refused in p1's
/// turn, refused with a spell on the stack, offered again once it resolved.
#[test]
fn quarry_exile_ability_is_offered_only_at_sorcery_timing() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut e, land, _elf) = quarry_game(&[]);
    float_for_quarry(&mut e);
    assert!(
        quarry_exile_offered(&e, land),
        "control: own main phase, empty stack, creature card and mana ready"
    );

    // With a spell on the stack (p0 keeps priority after casting a Bolt).
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, index::LIGHTNING_BOLT);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    assert!(!stack_is_empty(&e), "the Bolt waits on the stack");
    assert!(
        !quarry_exile_offered(&e, land),
        "not with a non-empty stack: sorcery timing needs an empty stack"
    );
    let before = e.state().snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 2,
            },
        )
        .is_err(),
        "and a direct attempt is refused"
    );
    assert_eq!(
        e.state().snapshot_hash(),
        before,
        "the refusal changed nothing"
    );
    pass_until(&mut e, stack_is_empty);
    assert!(
        quarry_exile_offered(&e, land),
        "control: same turn, stack empty again, offered again"
    );

    // In the opponent's turn: p0 gets priority only as the non-active player.
    pass_until(&mut e, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_for_quarry(&mut e);
    assert!(
        !quarry_exile_offered(&e, land),
        "not in the opponent's turn, though the creature card and the mana are there"
    );
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 2,
            },
        )
        .is_err(),
        "a direct attempt in the opponent's turn is refused too"
    );
}

/// "Sacrifice a Desert": the menu is the Deserts we control. A Forest beside
/// the Quarry and the opponent's Desert are not on it, another Desert of ours
/// is, and choosing that other Desert pays the cost without losing the Quarry.
#[test]
fn quarry_sacrifice_menu_is_our_deserts_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[lazotep_quarry(), forest(), cactus_preserve()])
        .battlefield(1, &[cactus_preserve()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, lazotep_quarry()).unwrap();
    let forest_id = on_battlefield(&e, p0, forest()).unwrap();
    let own_desert = on_battlefield(&e, p0, cactus_preserve()).unwrap();
    let their_desert = on_battlefield(&e, p1, cactus_preserve()).unwrap();
    let elf = bury_from_hand(&mut e, p0, llanowar_elves());
    float_for_quarry(&mut e);
    activate(&mut e, p0, lazotep_quarry(), 2);
    e.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    aim_at(&mut e, p0, elf);
    let Pending::ChooseCards {
        options,
        prompt: ChoicePrompt::CostSacrifice,
        ..
    } = e.pending().clone()
    else {
        panic!("expected the sacrifice choice, got {:?}", e.pending())
    };
    assert!(options.contains(&land), "the Quarry is a Desert we control");
    assert!(
        options.contains(&own_desert),
        "another Desert we control is a legal sacrifice"
    );
    assert!(
        !options.contains(&forest_id),
        "a Forest is not a Desert: the cost is not payable with it"
    );
    assert!(
        !options.contains(&their_desert),
        "\"Sacrifice a Desert\" is a Desert we control, not the opponent's"
    );
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![own_desert],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(own_desert).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(forest_id).unwrap().zone, Zone::Battlefield);
}
