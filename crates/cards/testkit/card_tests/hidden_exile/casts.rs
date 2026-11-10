#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn hidden_exile_multiple_linked_cards_can_all_be_played_in_chosen_order() {
    let (mut e, source, first) = hidden(index::SHELLDOCK_ISLE, index::GRIZZLY_BEARS);
    let second = library(&e, P0)[0];
    e.state
        .move_object(
            second,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let reference = e.state.source_identity(source).unwrap();
    e.state
        .hide_exiled_card(second, reference, 0, [P0].into_iter().collect(), true);
    satisfy(&mut e, index::SHELLDOCK_ISLE);
    e.state.players[0].lands_played_this_turn = 0;
    activate(&mut e, P0, index::SHELLDOCK_ISLE, 2);
    pass_until(&mut e, playing);
    choose(&mut e, vec![first]);
    assert!(
        playing(&e),
        "CR 607.3 applies the play instruction to each linked card"
    );
    assert_eq!(e.state.object(first).unwrap().zone, Zone::Stack);
    choose(&mut e, vec![second]);
    assert_eq!(e.state.object(second).unwrap().zone, Zone::Battlefield);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(first).unwrap().zone, Zone::Battlefield);
}

#[test]
fn hidden_exile_an_adventure_or_mdfc_land_back_can_be_played() {
    for (card_index, kind) in [
        (index::TWINING_TWINS, CastModeKind::Face(1)),
        (index::BALA_GED_RECOVERY, CastModeKind::PlayLandFace(1)),
    ] {
        let (mut e, _, card) =
            hidden_with_board(index::SHELLDOCK_ISLE, card_index, &[index::GRIZZLY_BEARS]);
        satisfy(&mut e, index::SHELLDOCK_ISLE);
        e.state.players[0].lands_played_this_turn = 0;
        activate(&mut e, P0, index::SHELLDOCK_ISLE, 2);
        pass_until(&mut e, playing);
        choose(&mut e, vec![card]);
        if let Pending::ChooseCastMode { options, .. } = e.pending() {
            let mode = options
                .iter()
                .find(|o| o.kind == kind)
                .expect("the non-front face is offered")
                .index;
            e.apply(P0, PlayerAction::ChooseMode(mode as usize))
                .unwrap();
        }
        if kind == CastModeKind::Face(1) {
            let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
            e.apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![bear],
                    players: vec![],
                },
            )
            .unwrap();
            assert_eq!(e.state.object(card).unwrap().zone, Zone::Stack);
        } else {
            assert_eq!(e.state.object(card).unwrap().zone, Zone::Battlefield);
            assert_eq!(e.state.object(card).unwrap().face_index, 1);
        }
        pass_until(&mut e, stack_is_empty);
    }
}

#[test]
fn hidden_exile_reacquiring_copied_abilities_cannot_reuse_the_previous_link() {
    let (mut e, source, old) = hidden(index::SHELLDOCK_ISLE, index::GRIZZLY_BEARS);
    let target = library(&e, P0)[0];
    // Taking the same text anew is still a fresh pair of linked abilities.
    let original = e.state.printed_ability_list(source).unwrap();
    let mut replacement = original.clone();
    replacement.acquisition = e.state.next_timestamp();
    e.state
        .object_mut(source)
        .unwrap()
        .take_abilities(replacement);
    e.state.invalidate_projections();
    satisfy(&mut e, index::SHELLDOCK_ISLE);
    activate(&mut e, P0, index::SHELLDOCK_ISLE, 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(old).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.object(target).unwrap().zone, Zone::Library);
}

#[test]
fn hidden_exile_cost_increases_can_be_paid_with_mana_abilities_during_the_free_cast() {
    let (mut e, _, card) = hidden_with_board(
        index::SHELLDOCK_ISLE,
        index::SWORDS_TO_PLOWSHARES,
        &[
            index::GLOOM,
            index::FOREST,
            index::FOREST,
            index::FOREST,
            index::GRIZZLY_BEARS,
        ],
    );
    satisfy(&mut e, index::SHELLDOCK_ISLE);
    let forests: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state
                .object(*id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == index::FOREST)
        })
        .collect();
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    e.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    e.refresh_offer();
    activate(&mut e, P0, index::SHELLDOCK_ISLE, 2);
    pass_until(&mut e, playing);
    choose(&mut e, vec![card]);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .unwrap();
    assert!(matches!(e.pending(), Pending::Priority { player: P0, .. }));
    for source in forests {
        e.apply(P0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    assert_eq!(e.state.object(card).unwrap().zone, Zone::Stack);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
}

#[test]
fn hidden_exile_free_cast_still_sacrifices_for_mandatory_additional_costs() {
    let (mut e, source, card) = hidden(index::SHELLDOCK_ISLE, index::CROP_ROTATION);
    satisfy(&mut e, index::SHELLDOCK_ISLE);
    activate(&mut e, P0, index::SHELLDOCK_ISLE, 2);
    pass_until(&mut e, playing);
    choose(&mut e, vec![card]);
    assert!(matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::CostSacrifice,
            ..
        }
    ));
    choose(&mut e, vec![source]);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state.object(card).unwrap().zone, Zone::Stack);
    pass_until(&mut e, library_choices::searched);
    choose(&mut e, vec![]);
    pass_until(&mut e, stack_is_empty);
}
