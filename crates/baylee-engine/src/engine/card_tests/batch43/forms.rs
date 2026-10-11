//! Batch 43: forms behavior through real cards.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch43_primal_clay_selects_all_three_copiable_shapes() {
    for (shape, power, toughness, flying, wall) in [
        (0, 3, 3, false, false),
        (1, 2, 2, true, false),
        (2, 1, 6, false, true),
    ] {
        let mut e = prepared(&[index::PRIMAL_CLAY], &[], &[]);
        let clay = in_hand(&e, P0, index::PRIMAL_CLAY).unwrap();
        cast_with_floating(&mut e, P0, index::PRIMAL_CLAY);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        e.apply(P0, PlayerAction::ChooseMode(shape)).unwrap();
        pass_until(&mut e, stack_is_empty);
        let object = e.state().object(clay).unwrap();
        let c = object.characteristics();
        assert_eq!((c.power, c.toughness), (Some(power), Some(toughness)));
        assert_eq!(c.keywords.contains(KeywordSet::FLYING), flying);
        assert_eq!(c.keywords.contains(KeywordSet::DEFENDER), wall);
        assert_eq!(c.subtypes.contains(subtypes::creature::WALL), wall);
    }
}

#[test]
fn batch43_shapeshifter_entry_choices_cover_zero_through_seven_and_leave_zero_seven() {
    for chosen in 0..=7 {
        let mut e = prepared(&[index::SHAPESHIFTER], &[], &[]);
        let card = in_hand(&e, P0, index::SHAPESHIFTER).unwrap();
        cast_with_floating(&mut e, P0, index::SHAPESHIFTER);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        let hash = e.snapshot_hash();
        assert!(e.apply(P0, PlayerAction::ChooseNumber(8)).is_err());
        assert_eq!(e.snapshot_hash(), hash);
        e.apply(P0, PlayerAction::ChooseNumber(chosen)).unwrap();
        pass_until(&mut e, stack_is_empty);
        let object = e.state().object(card).unwrap();
        if chosen == 7 {
            assert_eq!(object.zone, Zone::Graveyard);
            assert_eq!(
                (
                    object.characteristics().power,
                    object.characteristics().toughness
                ),
                (Some(0), Some(7))
            );
        } else {
            assert_eq!(object.zone, Zone::Battlefield);
            assert_eq!(
                (
                    object.characteristics().power,
                    object.characteristics().toughness
                ),
                (Some(chosen as i16), Some(7 - chosen as i16))
            );
        }
    }
}

#[test]
fn batch43_sidequest_takes_only_eligible_cards_and_transforms_only_when_taken() {
    for (card, take) in [
        (index::SOL_RING, true),
        (index::LLANOWAR_ELVES, false),
        (index::FOREST, false),
    ] {
        let mut e = prepared(&[card, index::SIDEQUEST_CATCH_A_FISH], &[], &[]);
        cast_with_floating(&mut e, P0, index::SIDEQUEST_CATCH_A_FISH);
        pass_until(&mut e, stack_is_empty);
        let source = on_battlefield(&e, P0, index::SIDEQUEST_CATCH_A_FISH).unwrap();
        let top = hand_to_library_top(&mut e, P0, card);
        next_upkeep(&mut e);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::PutIntoHand,
                    ..
                }
            )
        });
        let before = e.state().battlefield_seen().count();
        choose(&mut e, if take { vec![top] } else { vec![] });
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(source).unwrap().face_index, u8::from(take));
        assert_eq!(
            e.state().battlefield_seen().count(),
            before + usize::from(take)
        );
        assert_eq!(
            e.state().object(top).unwrap().zone,
            if take { Zone::Hand } else { Zone::Library }
        );
    }
}

#[test]
fn batch43_drop_of_honey_ties_are_choices_not_targets_and_regeneration_cannot_save_them() {
    let mut e = prepared(
        &[index::DROP_OF_HONEY],
        &[index::LLANOWAR_ELVES],
        &[index::LLANOWAR_ELVES, index::SERRA_ANGEL],
    );
    cast_with_floating(&mut e, P0, index::DROP_OF_HONEY);
    pass_until(&mut e, stack_is_empty);
    let victim = on_battlefield(&e, P1, index::LLANOWAR_ELVES).unwrap();
    next_upkeep(&mut e);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Generic,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(options.len(), 2);
    e.state.object_mut(victim).unwrap().regeneration_shields = 1;
    choose(&mut e, vec![victim]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(victim).unwrap().zone, Zone::Graveyard);
    assert!(on_battlefield(&e, P1, index::SERRA_ANGEL).is_some());
}

#[test]
fn batch43_shapeshifter_upkeep_can_decline_or_choose_another_number() {
    for change in [false, true] {
        let mut e = prepared(&[index::SHAPESHIFTER], &[], &[]);
        let card = in_hand(&e, P0, index::SHAPESHIFTER).unwrap();
        cast_with_floating(&mut e, P0, index::SHAPESHIFTER);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
        pass_until(&mut e, stack_is_empty);
        next_upkeep(&mut e);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        yes(&mut e, change);
        if change {
            assert!(matches!(
                e.pending(),
                Pending::ChooseNumber { min: 0, max: 7, .. }
            ));
            e.apply(P0, PlayerAction::ChooseNumber(5)).unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        let c = e.state().object(card).unwrap().characteristics();
        assert_eq!(
            (c.power, c.toughness),
            if change {
                (Some(5), Some(2))
            } else {
                (Some(2), Some(5))
            }
        );
    }
}

#[test]
fn batch43_shapeshifter_number_is_linked_to_its_rules_acquisition() {
    let mut e = prepared(&[index::SHAPESHIFTER], &[], &[]);
    let source = in_hand(&e, P0, index::SHAPESHIFTER).unwrap();
    cast_with_floating(&mut e, P0, index::SHAPESHIFTER);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    e.apply(P0, PlayerAction::ChooseNumber(5)).unwrap();
    pass_until(&mut e, stack_is_empty);
    copy_rules_fixture(&mut e, source, source, false);
    let c = e.state().object(source).unwrap().characteristics();
    assert_eq!((c.power, c.toughness), (Some(0), Some(7)));
    next_upkeep(&mut e);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut e, stack_is_empty);
    let c = e.state().object(source).unwrap().characteristics();
    assert_eq!((c.power, c.toughness), (Some(3), Some(4)));
}

#[test]
fn batch43_primal_clay_copy_keeps_inherited_flying_when_it_selects_three_three() {
    let mut e = prepared(&[index::PRIMAL_CLAY, index::CLONE], &[], &[]);
    let clay = in_hand(&e, P0, index::PRIMAL_CLAY).unwrap();
    let clone = in_hand(&e, P0, index::CLONE).unwrap();
    cast_with_floating(&mut e, P0, index::PRIMAL_CLAY);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e.apply(P0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut e, stack_is_empty);
    cast_with_floating(&mut e, P0, index::CLONE);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    target(&mut e, vec![clay], vec![]);
    assert!(matches!(e.pending(), Pending::ChooseCastMode { .. }));
    e.apply(P0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut e, stack_is_empty);
    let c = e.state().object(clone).unwrap().characteristics();
    assert_eq!((c.power, c.toughness), (Some(3), Some(3)));
    assert!(c.keywords.contains(KeywordSet::FLYING));
}

#[test]
fn batch43_drop_of_honey_uses_signed_minimum_and_keeps_an_indestructible_minimum() {
    for indestructible in [false, true] {
        let mut e = prepared(
            &[index::DROP_OF_HONEY],
            &[index::LLANOWAR_ELVES],
            &[index::SERRA_ANGEL],
        );
        let smallest = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
        let honey = in_hand(&e, P0, index::DROP_OF_HONEY).unwrap();
        cast_with_floating(&mut e, P0, index::DROP_OF_HONEY);
        pass_until(&mut e, stack_is_empty);
        // Fixture isolates signed power, untargetability, and destruction prevention.
        let object = e.state.object_mut(smallest).unwrap();
        let base = std::sync::Arc::make_mut(&mut object.base);
        base.power = Some(-2);
        base.keywords = base.keywords.union(KeywordSet::SHROUD);
        if indestructible {
            base.keywords = base.keywords.union(KeywordSet::INDESTRUCTIBLE);
        }
        e.state.invalidate_projections();
        e.refresh_offer();
        next_upkeep(&mut e);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(smallest).unwrap().zone,
            if indestructible {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
        assert!(on_battlefield(&e, P1, index::SERRA_ANGEL).is_some());
        assert_eq!(e.state().object(honey).unwrap().zone, Zone::Battlefield);
    }
}
