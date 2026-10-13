//! Exile, reflexive targets and complete granted abilities through real actions.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::Cause;

mod activation_zones;
mod link_families;

fn cauldron(e: &Engine<RegistryLookup>) -> ObjectId {
    on_battlefield(e, P0, index::AGATHA_S_SOUL_CAULDRON).unwrap()
}

fn exile(e: &mut Engine<RegistryLookup>, donor: ObjectId, recipient: Option<ObjectId>) {
    let source = cauldron(e);
    e.state.set_tapped(source, false);
    e.refresh_offer();
    activate(e, P0, index::AGATHA_S_SOUL_CAULDRON, 2);
    target(e, vec![donor], vec![]);
    if let Some(recipient) = recipient {
        pass_until(e, |e| matches!(e.pending(), Pending::ChooseTargets { .. }));
        assert_eq!(
            e.state.object(donor).unwrap().zone,
            crate::zone::Zone::Exile
        );
        target(e, vec![recipient], vec![]);
    }
    pass_until(e, stack_is_empty);
}

fn grants(e: &Engine<RegistryLookup>, recipient: ObjectId) -> Vec<u32> {
    crate::effects::granted_activated(e.state(), recipient)
        .map(|ability| ability.id)
        .collect()
}

fn use_grant(e: &mut Engine<RegistryLookup>, recipient: ObjectId, id: u32) {
    e.refresh_offer();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: recipient,
            ability_index: id,
        },
    )
    .unwrap();
}

#[test]
fn batch46_agatha_reflexive_counter_chooses_after_exile_and_only_for_creatures() {
    for creature in [false, true] {
        let mut e = prepared(
            &[if creature {
                index::SHIVAN_DRAGON
            } else {
                index::FOREST
            }],
            &[index::AGATHA_S_SOUL_CAULDRON, index::GRIZZLY_BEARS],
            &[index::SERRA_ANGEL],
        );
        let donor = hand_to_graveyard(
            &mut e,
            P0,
            if creature {
                index::SHIVAN_DRAGON
            } else {
                index::FOREST
            },
        );
        let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
        let angel = on_battlefield(&e, P1, index::SERRA_ANGEL).unwrap();
        activate(&mut e, P0, index::AGATHA_S_SOUL_CAULDRON, 2);
        target(&mut e, vec![donor], vec![]);
        assert_eq!(counters_on(&e, bear, CounterKind::P1P1), 0);
        if creature {
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseTargets { .. })
            });
            assert_eq!(
                e.state.object(donor).unwrap().zone,
                crate::zone::Zone::Exile
            );
            let Pending::ChooseTargets { options, .. } = e.pending() else {
                unreachable!()
            };
            assert!(options.contains(&bear));
            assert!(!options.contains(&angel));
            target(&mut e, vec![bear], vec![]);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            counters_on(&e, bear, CounterKind::P1P1),
            u16::from(creature)
        );
        assert_eq!(grants(&e, bear).len(), usize::from(creature));
    }
}

#[test]
fn batch46_agatha_grants_opponents_exiled_ability_and_spends_colorless_as_red() {
    let mut e = prepared(
        &[],
        &[index::AGATHA_S_SOUL_CAULDRON, index::GRIZZLY_BEARS],
        &[index::SHIVAN_DRAGON],
    );
    let donor = on_battlefield(&e, P1, index::SHIVAN_DRAGON).unwrap();
    e.state
        .move_object(
            donor,
            ZoneLocation::Graveyard(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    exile(&mut e, donor, Some(bear));
    reset_pool(&mut e);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 1);
    let id = grants(&e, bear)[0];
    use_grant(&mut e, bear, id);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bear), (4, 3));
    assert_eq!(e.state.object(donor).unwrap().owner, P1);
}

#[test]
fn batch46_agatha_nine_donors_keep_stable_action_ids_after_one_link_leaves() {
    let mut e = prepared(
        &[index::SHIVAN_DRAGON; 9],
        &[index::AGATHA_S_SOUL_CAULDRON, index::GRIZZLY_BEARS],
        &[],
    );
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let mut donors = Vec::new();
    for _ in 0..9 {
        let donor = hand_to_graveyard(&mut e, P0, index::SHIVAN_DRAGON);
        exile(&mut e, donor, Some(bear));
        donors.push(donor);
    }
    let before = grants(&e, bear);
    assert_eq!(before.len(), 9);
    e.state
        .move_object(
            donors[0],
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(grants(&e, bear), before[1..]);
    reset_pool(&mut e);
    e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    e.refresh_offer();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: bear,
                ability_index: before[0]
            }
        )
        .is_err()
    );
    use_grant(&mut e, bear, before[8]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bear), (12, 11));
}

#[test]
fn batch46_agatha_counter_removal_rejects_new_activations_but_stack_keeps_its_definition() {
    let mut e = prepared(
        &[index::SHIVAN_DRAGON],
        &[index::AGATHA_S_SOUL_CAULDRON, index::GRIZZLY_BEARS],
        &[],
    );
    let donor = hand_to_graveyard(&mut e, P0, index::SHIVAN_DRAGON);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    exile(&mut e, donor, Some(bear));
    let id = grants(&e, bear)[0];
    use_grant(&mut e, bear, id);
    e.state
        .object_mut(bear)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 0);
    e.refresh_offer();
    assert!(grants(&e, bear).is_empty());
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: bear,
                ability_index: id
            }
        )
        .is_err()
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bear), (3, 2));
}

#[test]
fn batch46_agatha_link_does_not_survive_cauldron_blink_or_unrelated_reexile() {
    for blink_source in [false, true] {
        let mut e = prepared(
            &[index::SHIVAN_DRAGON],
            &[index::AGATHA_S_SOUL_CAULDRON, index::GRIZZLY_BEARS],
            &[],
        );
        let donor = hand_to_graveyard(&mut e, P0, index::SHIVAN_DRAGON);
        let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
        exile(&mut e, donor, Some(bear));
        let id = grants(&e, bear)[0];
        let changed = if blink_source { cauldron(&e) } else { donor };
        e.state
            .move_object(
                changed,
                ZoneLocation::Hand(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        e.state
            .move_object(
                changed,
                if blink_source {
                    ZoneLocation::Battlefield
                } else {
                    ZoneLocation::Exile(P0)
                },
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        e.sync_static_effects();
        e.refresh_offer();
        assert!(grants(&e, bear).is_empty());
        assert!(
            e.apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: bear,
                    ability_index: id
                }
            )
            .is_err()
        );
    }
}

#[test]
fn batch46_agatha_granted_ballista_target_and_counter_cost_are_the_recipients() {
    let mut e = prepared(
        &[index::WALKING_BALLISTA],
        &[index::AGATHA_S_SOUL_CAULDRON, index::GRIZZLY_BEARS],
        &[],
    );
    let donor = hand_to_graveyard(&mut e, P0, index::WALKING_BALLISTA);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    exile(&mut e, donor, Some(bear));
    let id = grants(&e, bear)[1];
    let life = e.state.players[1].life;
    use_grant(&mut e, bear, id);
    target(&mut e, vec![], vec![P1]);
    assert_eq!(counters_on(&e, bear, CounterKind::P1P1), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 1);
    assert!(grants(&e, bear).is_empty());
}

#[test]
fn batch46_agatha_granted_artisan_retains_x_sacrifice_and_sorcery_timing() {
    let mut e = prepared(
        &[
            index::FIEND_ARTISAN,
            index::RAGING_GOBLIN,
            index::GIANT_GROWTH,
        ],
        &[
            index::AGATHA_S_SOUL_CAULDRON,
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
        ],
        &[],
    );
    let donor = hand_to_graveyard(&mut e, P0, index::FIEND_ARTISAN);
    let goblin = hand_to_library_top(&mut e, P0, index::RAGING_GOBLIN);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    exile(&mut e, donor, Some(bear));
    assert_eq!(
        grants(&e, bear).len(),
        1,
        "the donor's static ability is not granted"
    );
    let id = grants(&e, bear)[0];
    let growth = in_hand(&e, P0, index::GIANT_GROWTH).unwrap();
    cast(&mut e, growth);
    target(&mut e, vec![bear], vec![]);
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: bear,
                ability_index: id
            }
        )
        .is_err()
    );
    pass_until(&mut e, stack_is_empty);
    reset_pool(&mut e);
    e.state.players[0].mana_pool.add(ManaColor::Blue, 3);
    use_grant(&mut e, bear, id);
    assert!(matches!(e.pending(), Pending::ChooseNumber { .. }));
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseCards { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert!(options.contains(&elves));
    assert!(!options.contains(&bear));
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![elves],
        },
    )
    .unwrap();
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![goblin],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(goblin).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(
        e.state.object(elves).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert!(is_tapped(&e, bear));
}

#[test]
fn batch46_agatha_color_permission_does_not_turn_colored_mana_into_colorless() {
    let mut e = prepared(
        &[],
        &[index::AGATHA_S_SOUL_CAULDRON, index::SEA_GATE_WRECKAGE],
        &[],
    );
    let wreckage = on_battlefield(&e, P0, index::SEA_GATE_WRECKAGE).unwrap();
    // An animated land remains a creature source with its printed ability.
    let base = e.state.object_mut(wreckage).unwrap().base_mut();
    base.types = base.types.union(TypeSet::CREATURE);
    base.power = Some(3);
    base.toughness = Some(3);
    let hand = e.state.zones.list(ZoneLocation::Hand(P0)).clone();
    for card in hand {
        e.state
            .move_object(
                card,
                ZoneLocation::Exile(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    reset_pool(&mut e);
    e.state.players[0].mana_pool.add(ManaColor::Blue, 3);
    e.refresh_offer();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: wreckage,
                ability_index: 1
            }
        )
        .is_err()
    );
    e.state.players[0].mana_pool = ManaPool::new();
    e.state.players[0].mana_pool.add(ManaColor::Blue, 2);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 1);
    e.refresh_offer();
    activate(&mut e, P0, index::SEA_GATE_WRECKAGE, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.zones.list(ZoneLocation::Hand(P0)).len(), 1);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
}

#[test]
fn batch46_two_cauldron_sources_keep_their_donors_and_recipients_separate() {
    let mut e = prepared(
        &[index::SHIVAN_DRAGON],
        &[index::AGATHA_S_SOUL_CAULDRON, index::GRIZZLY_BEARS],
        &[
            index::AGATHA_S_SOUL_CAULDRON,
            index::LLANOWAR_ELVES,
            index::SERRA_ANGEL,
        ],
    );
    let donor = hand_to_graveyard(&mut e, P0, index::SHIVAN_DRAGON);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    exile(&mut e, donor, Some(bear));
    let elves = on_battlefield(&e, P1, index::LLANOWAR_ELVES).unwrap();
    let angel = on_battlefield(&e, P1, index::SERRA_ANGEL).unwrap();
    e.state
        .move_object(
            elves,
            ZoneLocation::Graveyard(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let other_cauldron = on_battlefield(&e, P1, index::AGATHA_S_SOUL_CAULDRON).unwrap();
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: other_cauldron,
            ability_index: 2,
        },
    )
    .unwrap();
    target(&mut e, vec![elves], vec![]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    target(&mut e, vec![angel], vec![]);
    pass_until(&mut e, stack_is_empty);
    let mine = grants(&e, bear);
    let theirs = grants(&e, angel);
    assert_eq!((mine.len(), theirs.len()), (1, 1));
    assert_ne!(mine, theirs);
    let before = e.state.players[0].mana_pool.total();
    // My creature has only the Dragon ability, never the other source's Elf ability.
    use_grant(&mut e, bear, mine[0]);
    assert!(!stack_is_empty(&e), "the Dragon ability uses the stack");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bear), (4, 3));
    assert_eq!(e.state.players[0].mana_pool.total(), before - 1);
}

#[test]
fn batch46_agatha_grants_loyalty_to_a_creature_and_preserves_once_each_turn() {
    let mut e = prepared(
        &[index::GRIST_THE_HUNGER_TIDE, index::LLANOWAR_ELVES],
        &[index::AGATHA_S_SOUL_CAULDRON, index::GRIZZLY_BEARS],
        &[],
    );
    let grist = hand_to_graveyard(&mut e, P0, index::GRIST_THE_HUNGER_TIDE);
    // Grist is a 1/1 Insect creature card off the battlefield through its own
    // static ability (CR 113.6), and in the graveyard it already is one. The
    // Cauldron's "When a creature card is exiled this way" is checked at the
    // moment of the exile, though, and there the exiled card still shows its
    // printed types: the static is registered for the object in exile only on
    // the engine's next pass (measured: the exiled Grist read as a planeswalker
    // alone when the reflexive trigger was looked for, and as a creature one
    // pass later). So the creature characteristic is also written onto the
    // card's base here, which keeps the test about the Cauldron's grant of the
    // -5 and not about that ordering.
    let base = e.state.object_mut(grist).unwrap().base_mut();
    base.types = base.types.union(TypeSet::CREATURE);
    base.power = Some(1);
    base.toughness = Some(1);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    exile(&mut e, grist, Some(bear));
    let elves = hand_to_graveyard(&mut e, P0, index::LLANOWAR_ELVES);
    e.state
        .object_mut(bear)
        .unwrap()
        .counters
        .set(CounterKind::Loyalty, 10);
    e.refresh_offer();
    // Grist prints a +1, a -2 and a -5, all three granted to the Bears; the
    // one this test plays is the -5, picked by its loyalty cost and not by
    // its place in the list.
    let id = crate::effects::granted_activated(e.state(), bear)
        .find(|grant| {
            matches!(
                grant.definition,
                baylee_cards_dsl::AbilityDef::Loyalty { cost: -5, .. }
            )
        })
        .map(|grant| grant.id)
        .expect("Grist's -5 is among the abilities the Bears were granted");
    let life = e.state.players[1].life;
    use_grant(&mut e, bear, id);
    assert_eq!(counters_on(&e, bear, CounterKind::Loyalty), 5);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 1);
    assert_eq!(
        e.state.object(elves).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: bear,
                ability_index: id
            }
        )
        .is_err()
    );
}

#[test]
fn batch46_agatha_a_new_copy_acquisition_does_not_reuse_an_old_exile_link() {
    let mut e = prepared(
        &[index::SHIVAN_DRAGON],
        &[
            index::AGATHA_S_SOUL_CAULDRON,
            index::GRIZZLY_BEARS,
            index::SOL_RING,
        ],
        &[index::AGATHA_S_SOUL_CAULDRON],
    );
    let donor = hand_to_graveyard(&mut e, P0, index::SHIVAN_DRAGON);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    exile(&mut e, donor, Some(bear));
    let id = grants(&e, bear)[0];
    let source = cauldron(&e);
    let ring = on_battlefield(&e, P0, index::SOL_RING).unwrap();
    let other = on_battlefield(&e, P1, index::AGATHA_S_SOUL_CAULDRON).unwrap();
    for target in [ring, other] {
        assert!(crate::copiable_abilities::apply_copy(
            &mut e.state,
            crate::copiable_abilities::CopyApplication {
                source,
                target,
                mods: &[],
                copy_index: 0,
                until_eot: true,
                resolving: None,
                text: crate::text_changes::TextChangeMap::IDENTITY,
            }
        ));
        e.sync_static_effects();
        e.refresh_offer();
    }
    assert_eq!(
        e.state.object(donor).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert!(grants(&e, bear).is_empty());
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: bear,
                ability_index: id
            }
        )
        .is_err()
    );
}

#[test]
fn batch46_palace_copies_a_granted_ability_even_after_tidebinder_counters_the_original() {
    for counter_original in [false, true] {
        let mut e = prepared(
            &[index::SHIVAN_DRAGON, index::TISHANA_S_TIDEBINDER],
            &[
                index::AGATHA_S_SOUL_CAULDRON,
                index::GRIZZLY_BEARS,
                index::SUNKEN_PALACE,
            ],
            &[],
        );
        let donor = hand_to_graveyard(&mut e, P0, index::SHIVAN_DRAGON);
        let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
        exile(&mut e, donor, Some(bear));
        reset_pool(&mut e);
        e.state.players[0].mana_pool.add(ManaColor::Colorless, 1);
        e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
        seed_graveyard(&mut e, P0, 7);
        let palace = on_battlefield(&e, P0, index::SUNKEN_PALACE).unwrap();
        e.state.set_tapped(palace, false);
        e.refresh_offer();
        activate(&mut e, P0, index::SUNKEN_PALACE, 1);
        for _ in 0..7 {
            let Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } = e.pending().clone()
            else {
                break;
            };
            let count = usize::from(min);
            e.apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: options[..count].to_vec(),
                },
            )
            .unwrap();
        }
        assert_eq!(e.state.players[0].mana_pool.total(), 1);
        let id = grants(&e, bear)[0];
        use_grant(&mut e, bear, id);
        assert_eq!(
            e.state.players[0].mana_pool.total(),
            0,
            "Palace blue pays the inherited red cost via Agatha"
        );
        let original = e
            .state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .copied()
            .find(|object| {
                e.state
                    .object(*object)
                    .unwrap()
                    .ability
                    .is_some_and(|loc| loc.source == bear && loc.index == id)
            })
            .unwrap();
        if counter_original {
            e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
            e.state.players[0].mana_pool.add(ManaColor::Colorless, 2);
            let tidebinder = in_hand(&e, P0, index::TISHANA_S_TIDEBINDER).unwrap();
            cast(&mut e, tidebinder);
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseTargets { .. })
            });
            target(&mut e, vec![original], vec![]);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(pt(&e, bear), (if counter_original { 4 } else { 5 }, 3));
    }
}
