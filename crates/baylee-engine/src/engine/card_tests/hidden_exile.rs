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

/// Windbrisk Heights with `attackers` creatures recorded as having attacked
/// this turn (distinct objects, all p0's), its activation `{W}, {T}` resolving
/// with the hidden Bears still in exile. Returns the engine and the Bears.
fn windbrisk_after_attacking_with(attackers: usize) -> (Engine<RegistryLookup>, ObjectId) {
    let (mut e, _, card) = hidden(index::WINDBRISK_HEIGHTS, index::GRIZZLY_BEARS);
    for id in library(&e, P0).into_iter().take(attackers) {
        let version = e.state.object(id).unwrap().version;
        e.state.per_turn.attacked.push((id, version, P0));
    }
    assert_eq!(e.state.per_turn.attacked.len(), attackers);
    e.refresh_offer();
    activate(&mut e, P0, index::WINDBRISK_HEIGHTS, 2);
    (e, card)
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
    assert!(crate::casting::play_permission(&e.state, P0, card).is_none());
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
