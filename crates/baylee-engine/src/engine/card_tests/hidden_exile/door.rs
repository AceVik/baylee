#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn hidden_exile_all_six_lands_hide_privately_and_check_their_condition_on_resolution() {
    for land in LANDS {
        let (mut e, source, card) = hidden(land, index::GRIZZLY_BEARS);
        let object = e.state.object(card).unwrap();
        assert_eq!(object.zone, Zone::Exile);
        assert!(object.status.contains(Status::FACE_DOWN));
        assert!(object.characteristics().types.is_empty());
        assert!(object.characteristics().power.is_none());
        assert!(object.printed_abilities(&RegistryLookup).is_empty());
        assert!(object.may_look_at_exile(P0));
        assert!(!object.may_look_at_exile(P1));
        assert!(!is_tapped(&e, source));
        // None of these conditions restrict activation. They become true after it.
        activate(&mut e, P0, land, 2);
        satisfy(&mut e, land);
        pass_until(&mut e, playing);
        choose(&mut e, vec![card]);
        assert_eq!(e.state.object(card).unwrap().zone, Zone::Stack);
        assert!(
            !e.state
                .object(card)
                .unwrap()
                .status
                .contains(Status::FACE_DOWN)
        );
        pass_until(&mut e, stack_is_empty);
        let object = e.state.object(card).unwrap();
        assert_eq!(object.zone, Zone::Battlefield);
        assert_eq!(object.characteristics().power, Some(2));
        assert!(
            !object
                .riders
                .iter()
                .any(|r| matches!(r, Rider::LinkedExile { .. } | Rider::ExileVisibility { .. }))
        );
    }
}

#[test]
fn hidden_exile_false_conditions_resolve_without_a_play_or_lingering_permission() {
    for land in LANDS {
        let (mut e, _, card) = hidden(land, index::GRIZZLY_BEARS);
        let spare = library(&e, P0)[0];
        e.state
            .move_object(
                spare,
                ZoneLocation::Hand(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        e.refresh_offer();
        activate(&mut e, P0, land, 2);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.object(card).unwrap().zone, Zone::Exile);
        assert!(baylee_engine::casting::play_permission(&e.state, P0, card).is_none());
    }
}

#[test]
fn hidden_exile_inspection_reorders_only_the_unselected_top_cards_and_handles_short_libraries() {
    for count in 0..=5 {
        let mut e = prepared(&[index::CLIVE_S_HIDEAWAY], &[], &[]);
        for card in library(&e, P0).into_iter().skip(count) {
            e.state
                .move_object(
                    card,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        let before = library(&e, P0);
        let versions: Vec<_> = before
            .iter()
            .map(|id| e.state.object(*id).unwrap().version)
            .collect();
        e.refresh_offer();
        let source = play_land(&mut e, P0, index::CLIVE_S_HIDEAWAY);
        assert!(
            !is_tapped(&e, source),
            "modern hideaway does not itself enter tapped"
        );
        if count == 0 {
            pass_until(&mut e, stack_is_empty);
            continue;
        }
        pass_until(&mut e, inspecting);
        assert_eq!(
            e.inspected_cards(),
            &before.iter().rev().take(4).copied().collect::<Vec<_>>()
        );
        let selected = *before.last().unwrap();
        choose(&mut e, vec![selected]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library(&e, P0).len(), count - 1);
        for (card, version) in before.iter().zip(versions) {
            assert_eq!(
                e.state.object(*card).unwrap().version,
                version + u32::from(*card == selected)
            );
        }
        assert!(
            !e.state
                .journal
                .entries()
                .iter()
                .any(|entry| matches!(entry.event, GameEvent::Revealed { .. }))
        );
    }
}

#[test]
fn hidden_exile_vault_nobody_looks_and_sacrifice_happens_only_on_resolution() {
    let (mut e, source, card) = vault();
    assert!(!e.state.object(card).unwrap().may_look_at_exile(P0));
    assert!(!e.state.object(card).unwrap().may_look_at_exile(P1));
    let old_hand = e.state.zones.list(ZoneLocation::Hand(P0)).clone();
    activate(&mut e, P0, index::KNOWLEDGE_VAULT, 1);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state.object(card).unwrap().zone, Zone::Hand);
    assert!(
        !e.state
            .object(card)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        old_hand
            .iter()
            .all(|id| e.state.object(*id).unwrap().zone == Zone::Graveyard)
    );
}

#[test]
fn hidden_exile_vault_departure_returns_to_graveyard_and_stops_an_older_activation() {
    let (mut e, source, card) = vault();
    let hand = e.state.zones.list(ZoneLocation::Hand(P0)).clone();
    activate(&mut e, P0, index::KNOWLEDGE_VAULT, 1);
    e.state
        .move_object(
            source,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(card).unwrap().zone, Zone::Graveyard);
    assert!(
        hand.iter()
            .all(|id| e.state.object(*id).unwrap().zone == Zone::Hand)
    );
}

/// "You may play the exiled card without paying its mana cost if you control
/// four or more legendary creatures." Three legendary creatures, a creature
/// that is not legendary and a legendary artifact are five permanents that
/// each fit one half of the sentence and make four of neither count; the
/// exiled card stays where it is. The fourth legendary creature opens it.
#[test]
fn clive_s_hideaway_counts_legendary_creatures_only() {
    use baylee_core::types::{SupertypeSet, TypeSet};
    let (mut e, source, card) = hidden(index::CLIVE_S_HIDEAWAY, index::GRIZZLY_BEARS);
    let planted = plant(
        &mut e,
        &[
            (TypeSet::CREATURE, SupertypeSet::LEGENDARY),
            (TypeSet::CREATURE, SupertypeSet::LEGENDARY),
            (TypeSet::CREATURE, SupertypeSet::LEGENDARY),
            (TypeSet::CREATURE, SupertypeSet::EMPTY),
            (TypeSet::ARTIFACT, SupertypeSet::LEGENDARY),
        ],
    );
    assert_eq!(e.state.zones.list(ZoneLocation::Exile(P0)).len(), 1);

    activate(&mut e, P0, index::CLIVE_S_HIDEAWAY, 2);
    pass_until(&mut e, |e| playing(e) || stack_is_empty(e));
    assert!(
        !playing(&e),
        "three legendary creatures, a nonlegendary creature and a legendary artifact are not four legendary creatures"
    );
    assert_eq!(
        e.state.object(card).unwrap().zone,
        Zone::Exile,
        "the card stays in exile"
    );
    assert!(baylee_engine::casting::play_permission(&e.state, P0, card).is_none());

    // The fourth legendary creature: the same ability, the same card.
    let state = &mut e.state;
    state.object_mut(planted[3]).unwrap().base_mut().supertypes = SupertypeSet::LEGENDARY;
    state.invalidate_projections();
    state.refresh_characteristics();
    state.set_tapped(source, false);
    e.refresh_offer();
    activate(&mut e, P0, index::CLIVE_S_HIDEAWAY, 2);
    pass_until(&mut e, |e| playing(e) || stack_is_empty(e));
    assert!(
        playing(&e),
        "four legendary creatures: the card may be played"
    );
    choose(&mut e, vec![card]);
    assert_eq!(e.state.object(card).unwrap().zone, Zone::Stack);
}

/// "…if each player has no cards in hand": the opponent's hand counts, not
/// only yours. With your hand empty and one card in the opponent's, the
/// ability resolves without a play or a permission and the card stays hidden
/// in exile; once the opponent's hand is empty too, the same card plays free.
#[test]
fn hidden_exile_howltooth_hollow_waits_for_the_opponents_hand_to_empty_too() {
    let (mut e, source, card) = hidden(index::HOWLTOOTH_HOLLOW, index::GRIZZLY_BEARS);
    assert!(
        e.state.zones.list(ZoneLocation::Hand(P0)).is_empty(),
        "our hand is empty"
    );
    let spare = library(&e, P1)[0];
    e.state
        .move_object(
            spare,
            ZoneLocation::Hand(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    activate(&mut e, P0, index::HOWLTOOTH_HOLLOW, 2);
    pass_until(&mut e, |e| playing(e) || stack_is_empty(e));
    assert!(
        !playing(&e),
        "the opponent holds a card, so the exiled card may not be played"
    );
    assert_eq!(
        e.state.object(card).unwrap().zone,
        Zone::Exile,
        "the hidden card stays in exile"
    );
    assert!(
        baylee_engine::casting::play_permission(&e.state, P0, card).is_none(),
        "and no permission lingers"
    );

    // The opponent's hand empties; the same ability, the same card.
    e.state
        .move_object(
            spare,
            ZoneLocation::Graveyard(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state.set_tapped(source, false);
    e.refresh_offer();
    activate(&mut e, P0, index::HOWLTOOTH_HOLLOW, 2);
    pass_until(&mut e, |e| playing(e) || stack_is_empty(e));
    assert!(playing(&e), "each hand empty: the card may be played");
    choose(&mut e, vec![card]);
    assert_eq!(e.state.object(card).unwrap().zone, Zone::Stack);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(card).unwrap().zone,
        Zone::Battlefield,
        "played without paying its mana cost"
    );
}

/// "if you attacked with three or more creatures this turn": two attackers are
/// not three. The ability resolves, offers no play and leaves the card
/// face down in exile with no permission to play it.
#[test]
fn hidden_exile_windbrisk_two_attackers_do_not_play_the_exiled_card() {
    let (mut e, card) = windbrisk_after_attacking_with(2);
    pass_until(&mut e, stack_is_empty);
    assert!(!playing(&e), "two attackers is below \"three or more\"");
    assert_eq!(e.state.object(card).unwrap().zone, Zone::Exile);
    assert!(baylee_engine::casting::play_permission(&e.state, P0, card).is_none());
}

/// The control for the test above: exactly three attackers is "three or more",
/// so the same board, mana and exiled card now prompt for the free play.
#[test]
fn hidden_exile_windbrisk_three_attackers_offer_the_exiled_card() {
    let (mut e, card) = windbrisk_after_attacking_with(3);
    pass_until(&mut e, playing);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("{:?}", e.pending());
    };
    assert_eq!(options, vec![card], "the hidden card is what may be played");
}
