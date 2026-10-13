//! The seven linked-exile cards, through real activations and resumable choices.
#[allow(clippy::wildcard_imports)]
use super::*;
use super::{
    counter_resources::prepared,
    library_choices::{choose, library},
};
use crate::event::{Cause, GameEvent};
use crate::object::Rider;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const LANDS: [CardIndex; 6] = [
    index::CLIVE_S_HIDEAWAY,
    index::SPINEROCK_KNOLL,
    index::HOWLTOOTH_HOLLOW,
    index::MOSSWORT_BRIDGE,
    index::WINDBRISK_HEIGHTS,
    index::SHELLDOCK_ISLE,
];

pub(super) fn finish_hideaway(e: &mut Engine<RegistryLookup>) {
    pass_until(e, inspecting);
    let card = e.inspected_cards()[0];
    choose(e, vec![card]);
    pass_until(e, stack_is_empty);
}

fn inspecting(e: &Engine<RegistryLookup>) -> bool {
    !e.inspected_cards().is_empty()
}
fn playing(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::PlayFromExile,
            ..
        }
    )
}

fn hidden(land: CardIndex, card: CardIndex) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    hidden_with_board(land, card, &[])
}

fn hidden_with_board(
    land: CardIndex,
    card: CardIndex,
    board: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = prepared(&[land, card], board, &[]);
    let card = hand_to_library_top(&mut e, P0, card);
    e.refresh_offer();
    let source = play_land(&mut e, P0, land);
    pass_until(&mut e, inspecting);
    assert_eq!(e.inspected_cards().len(), 4);
    choose(&mut e, vec![card]);
    pass_until(&mut e, stack_is_empty);
    e.state.set_tapped(source, false);
    e.refresh_offer();
    (e, source, card)
}

fn satisfy(e: &mut Engine<RegistryLookup>, land: CardIndex) {
    match land {
        index::SHELLDOCK_ISLE => {
            let excess: Vec<_> = library(e, P1).into_iter().skip(20).collect();
            for card in excess {
                e.state
                    .move_object(
                        card,
                        ZoneLocation::Graveyard(P1),
                        ZonePosition::Top,
                        Cause::Effect,
                    )
                    .unwrap();
            }
        }
        index::SPINEROCK_KNOLL => e.state.per_turn.damage_dealt_to[1] = 7,
        index::HOWLTOOTH_HOLLOW => {
            for player in [P0, P1] {
                for card in e.state.zones.list(ZoneLocation::Hand(player)).clone() {
                    e.state
                        .move_object(
                            card,
                            ZoneLocation::Graveyard(player),
                            ZonePosition::Top,
                            Cause::Effect,
                        )
                        .unwrap();
                }
            }
        }
        _ => {
            let cards: Vec<_> = library(e, P0).into_iter().take(4).collect();
            for (i, card) in cards.iter().copied().enumerate() {
                e.state
                    .move_object(
                        card,
                        ZoneLocation::Battlefield,
                        ZonePosition::Top,
                        Cause::Effect,
                    )
                    .unwrap();
                let name = e.state.names.intern(&format!("Distinct test creature {i}"));
                let object = e.state.object_mut(card).unwrap();
                object.kind = crate::object::ObjectKind::Permanent;
                let base = object.base_mut();
                base.name = name;
                base.types = baylee_core::types::TypeSet::CREATURE;
                base.supertypes = baylee_core::types::SupertypeSet::LEGENDARY;
                base.power = Some(3);
                base.toughness = Some(3);
                let version = object.version;
                e.state.per_turn.attacked.push((card, version, P0));
            }
            e.state.invalidate_projections();
            e.state.refresh_characteristics();
        }
    }
    e.refresh_offer();
}

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
        assert!(crate::casting::play_permission(&e.state, P0, card).is_none());
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

fn vault() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = prepared(&[index::GRIZZLY_BEARS], &[index::KNOWLEDGE_VAULT], &[]);
    let card = hand_to_library_top(&mut e, P0, index::GRIZZLY_BEARS);
    let source = on_battlefield(&e, P0, index::KNOWLEDGE_VAULT).unwrap();
    e.refresh_offer();
    activate(&mut e, P0, index::KNOWLEDGE_VAULT, 0);
    pass_until(&mut e, stack_is_empty);
    (e, source, card)
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

mod casts;
mod permissions;
mod vault_edges;

mod land_entry;

/// Turns the top cards of `P0`'s library into permanents on the battlefield,
/// one per `(types, supertypes)` entry, and returns them in that order.
fn plant(
    e: &mut Engine<RegistryLookup>,
    shapes: &[(
        baylee_core::types::TypeSet,
        baylee_core::types::SupertypeSet,
    )],
) -> Vec<ObjectId> {
    let cards: Vec<_> = library(e, P0).into_iter().take(shapes.len()).collect();
    assert_eq!(cards.len(), shapes.len(), "the library is long enough");
    for (i, (card, (types, supertypes))) in cards.iter().copied().zip(shapes).enumerate() {
        e.state
            .move_object(
                card,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let name = e.state.names.intern(&format!("Planted permanent {i}"));
        let object = e.state.object_mut(card).unwrap();
        object.kind = crate::object::ObjectKind::Permanent;
        let base = object.base_mut();
        base.name = name;
        base.types = *types;
        base.supertypes = *supertypes;
        if types.contains(baylee_core::types::TypeSet::CREATURE) {
            base.power = Some(3);
            base.toughness = Some(3);
        }
    }
    e.state.invalidate_projections();
    e.state.refresh_characteristics();
    e.refresh_offer();
    cards
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
    assert!(crate::casting::play_permission(&e.state, P0, card).is_none());

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
