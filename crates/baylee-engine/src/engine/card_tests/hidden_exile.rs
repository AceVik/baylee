//! The seven linked-exile cards, through real activations and resumable choices.
#[allow(clippy::wildcard_imports)]
pub use super::*;
pub use super::{
    counter_resources::prepared,
    library_choices::{choose, library},
};
pub use baylee_core::generated::index;
pub use baylee_engine::event::{Cause, GameEvent};
pub use baylee_engine::object::Rider;
pub const P0: PlayerId = PlayerId::new(0);
pub const P1: PlayerId = PlayerId::new(1);
pub const LANDS: [CardIndex; 6] = [
    index::CLIVE_S_HIDEAWAY,
    index::SPINEROCK_KNOLL,
    index::HOWLTOOTH_HOLLOW,
    index::MOSSWORT_BRIDGE,
    index::WINDBRISK_HEIGHTS,
    index::SHELLDOCK_ISLE,
];

pub fn finish_hideaway(e: &mut Engine<RegistryLookup>) {
    pass_until(e, inspecting);
    let card = e.inspected_cards()[0];
    choose(e, vec![card]);
    pass_until(e, stack_is_empty);
}

pub fn inspecting(e: &Engine<RegistryLookup>) -> bool {
    !e.inspected_cards().is_empty()
}
pub fn playing(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::PlayFromExile,
            ..
        }
    )
}

pub fn hidden(land: CardIndex, card: CardIndex) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    hidden_with_board(land, card, &[])
}

pub fn hidden_with_board(
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

pub fn satisfy(e: &mut Engine<RegistryLookup>, land: CardIndex) {
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
                object.kind = baylee_engine::object::ObjectKind::Permanent;
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

pub fn vault() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = prepared(&[index::GRIZZLY_BEARS], &[index::KNOWLEDGE_VAULT], &[]);
    let card = hand_to_library_top(&mut e, P0, index::GRIZZLY_BEARS);
    let source = on_battlefield(&e, P0, index::KNOWLEDGE_VAULT).unwrap();
    e.refresh_offer();
    activate(&mut e, P0, index::KNOWLEDGE_VAULT, 0);
    pass_until(&mut e, stack_is_empty);
    (e, source, card)
}

/// Turns the top cards of `P0`'s library into permanents on the battlefield,
/// one per `(types, supertypes)` entry, and returns them in that order.
pub fn plant(
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
        object.kind = baylee_engine::object::ObjectKind::Permanent;
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

/// Windbrisk Heights with `attackers` creatures recorded as having attacked
/// this turn (distinct objects, all p0's), its activation `{W}, {T}` resolving
/// with the hidden Bears still in exile. Returns the engine and the Bears.
pub fn windbrisk_after_attacking_with(attackers: usize) -> (Engine<RegistryLookup>, ObjectId) {
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
