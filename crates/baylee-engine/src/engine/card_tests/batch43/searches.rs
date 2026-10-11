//! Batch 43: searches behavior through real cards.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch43_myriad_requires_shared_land_type_but_allows_one_or_zero() {
    for count in 0..=2 {
        let mut e = prepared(
            &[index::PLAINS, index::PLAINS, index::ISLAND],
            &[index::MYRIAD_LANDSCAPE],
            &[],
        );
        let a = hand_to_library_top(&mut e, P0, index::PLAINS);
        let b = hand_to_library_top(&mut e, P0, index::PLAINS);
        let wrong = hand_to_library_top(&mut e, P0, index::ISLAND);
        activate(&mut e, P0, index::MYRIAD_LANDSCAPE, 1);
        pass_until(&mut e, searched);
        let inspected = e.inspected_cards().to_vec();
        choose(&mut e, if count == 0 { vec![] } else { vec![a] });
        if count > 0 {
            assert_eq!(e.inspected_cards(), inspected);
            assert_eq!(e.state().object(a).unwrap().zone, Zone::Library);
            let Pending::ChooseCards { options, .. } = e.pending() else {
                panic!("second choice")
            };
            assert_eq!(options, &vec![b]);
            let hash = e.snapshot_hash();
            assert!(
                e.apply(
                    P0,
                    PlayerAction::ChooseObjects {
                        objects: vec![wrong]
                    }
                )
                .is_err()
            );
            assert_eq!(e.snapshot_hash(), hash);
            choose(&mut e, if count == 2 { vec![b] } else { vec![] });
        }
        pass_until(&mut e, stack_is_empty);
        for (id, found) in [(a, count > 0), (b, count > 1)] {
            assert_eq!(
                e.state().object(id).unwrap().zone == Zone::Battlefield,
                found
            );
            if found {
                assert!(is_tapped(&e, id));
            }
        }
    }
}

#[test]
fn batch43_myriad_agent_controls_both_choices() {
    let mut e = prepared(&[], &[index::MYRIAD_LANDSCAPE], &[index::OPPOSITION_AGENT]);
    activate(&mut e, P0, index::MYRIAD_LANDSCAPE, 1);
    pass_until(&mut e, searched);
    assert_eq!(e.pending().asked(), Some(P1));
    let a = first_option(&e);
    choose(&mut e, vec![a]);
    assert_eq!(e.pending().asked(), Some(P1));
    let b = first_option(&e);
    choose(&mut e, vec![b]);
    pass_until(&mut e, stack_is_empty);
    for id in [a, b] {
        assert_eq!(e.state().object(id).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn batch43_field_gathers_both_choices_before_simultaneous_untapped_entries() {
    for land in [index::COMMAND_TOWER, index::DARKSTEEL_CITADEL] {
        let mut e = prepared(&[], &[index::FIELD_OF_RUIN], &[land]);
        let victim = on_battlefield(&e, P1, land).unwrap();
        activate(&mut e, P0, index::FIELD_OF_RUIN, 1);
        target(&mut e, vec![victim], vec![]);
        pass_until(&mut e, searched);
        assert_eq!(e.pending().asked(), Some(P0));
        let before0 = library(&e, P0);
        let before1 = library(&e, P1);
        let a = first_option(&e);
        choose(&mut e, vec![a]);
        assert_eq!(e.pending().asked(), Some(P1));
        assert_eq!(library(&e, P0), before0);
        assert_eq!(library(&e, P1), before1);
        assert_eq!(e.state().object(a).unwrap().zone, Zone::Library);
        let b = first_option(&e);
        choose(&mut e, vec![b]);
        pass_until(&mut e, stack_is_empty);
        for (id, player) in [(a, P0), (b, P1)] {
            let object = e.state().object(id).unwrap();
            assert_eq!(object.zone, Zone::Battlefield);
            assert_eq!(object.controller, player);
            assert!(!is_tapped(&e, id));
        }
        assert_eq!(
            e.state().object(victim).unwrap().zone,
            if land == index::DARKSTEEL_CITADEL {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
    }
}

#[test]
fn batch43_transmute_sacrifices_only_on_resolution_then_pays_the_difference() {
    for (found, pay) in [
        (index::MANA_VAULT, false),
        (index::WINTER_ORB, false),
        (index::WINTER_ORB, true),
    ] {
        let mut e = prepared(&[index::TRANSMUTE_ARTIFACT, found], &[index::SOL_RING], &[]);
        let card = hand_to_library_top(&mut e, P0, found);
        let sacrifice = on_battlefield(&e, P0, index::SOL_RING).unwrap();
        cast_with_floating(&mut e, P0, index::TRANSMUTE_ARTIFACT);
        assert_eq!(e.state().object(sacrifice).unwrap().zone, Zone::Battlefield);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::CostSacrifice,
                    ..
                }
            )
        });
        choose(&mut e, vec![sacrifice]);
        assert!(searched(&e));
        assert_eq!(e.state().object(sacrifice).unwrap().zone, Zone::Graveyard);
        choose(&mut e, vec![card]);
        if found == index::WINTER_ORB {
            assert!(
                matches!(e.pending(),Pending::YesNo{prompt:crate::choice::YesNoPrompt::PayMana{cost},..} if *cost == baylee_core::mana::ManaCost::from_symbol_generic(1))
            );
            yes(&mut e, pay);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(card).unwrap().zone,
            if pay || found == index::MANA_VAULT {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
    }
}

#[test]
fn batch43_transmute_search_can_fail_and_agent_exile_skips_payment() {
    for agent in [false, true] {
        let opponents = if agent {
            vec![index::OPPOSITION_AGENT]
        } else {
            vec![]
        };
        let mut e = prepared(
            &[index::TRANSMUTE_ARTIFACT, index::WINTER_ORB],
            &[index::SOL_RING],
            &opponents,
        );
        let card = hand_to_library_top(&mut e, P0, index::WINTER_ORB);
        let sacrifice = on_battlefield(&e, P0, index::SOL_RING).unwrap();
        cast_with_floating(&mut e, P0, index::TRANSMUTE_ARTIFACT);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::CostSacrifice,
                    ..
                }
            )
        });
        choose(&mut e, vec![sacrifice]);
        assert_eq!(e.pending().asked(), Some(if agent { P1 } else { P0 }));
        choose(&mut e, if agent { vec![card] } else { vec![] });
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(card).unwrap().zone,
            if agent { Zone::Exile } else { Zone::Library }
        );
    }
}

#[test]
fn batch43_transmute_opens_a_real_mana_ability_window_to_pay_the_difference() {
    let mut e = prepared(
        &[index::TRANSMUTE_ARTIFACT, index::WINTER_ORB],
        &[index::SOL_RING, index::FOREST],
        &[],
    );
    let card = hand_to_library_top(&mut e, P0, index::WINTER_ORB);
    let sacrifice = on_battlefield(&e, P0, index::SOL_RING).unwrap();
    cast_with_floating(&mut e, P0, index::TRANSMUTE_ARTIFACT);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::CostSacrifice,
                ..
            }
        )
    });
    choose(&mut e, vec![sacrifice]);
    e.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    choose(&mut e, vec![card]);
    yes(&mut e, true);
    assert!(matches!(e.pending(),Pending::Priority{player,..} if *player==P0));
    activate(&mut e, P0, index::FOREST, 0);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn batch43_field_does_not_search_when_its_only_target_became_illegal() {
    let mut e = prepared(&[], &[index::FIELD_OF_RUIN], &[index::COMMAND_TOWER]);
    let land = on_battlefield(&e, P1, index::COMMAND_TOWER).unwrap();
    activate(&mut e, P0, index::FIELD_OF_RUIN, 1);
    target(&mut e, vec![land], vec![]);
    let before0 = library(&e, P0);
    let before1 = library(&e, P1);
    e.state
        .move_object(
            land,
            ZoneLocation::Hand(P1),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library(&e, P0), before0);
    assert_eq!(library(&e, P1), before1);
}
