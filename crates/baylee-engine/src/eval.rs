//! Evaluation of DSL data: filters, amounts, target options.
//!
//! All evaluation is pure read access to [`GameState`]; `you` is the
//! ability/spell controller, `this` its source object.

use crate::object::{GameObject, Status};
use crate::state::GameState;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{Amount, Filter, PlayerRel, TargetSpec, ZoneSel};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::TypeSet;

/// Evaluates a [`Filter`] against an object.
#[allow(clippy::only_used_in_recursion)] // part of the eval API: future filters (auras, counts) need state
#[must_use]
pub fn matches(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    this: ObjectId,
) -> bool {
    match filter {
        Filter::Any => true,
        Filter::This => obj.id == this,
        Filter::Another => obj.id != this,
        Filter::And(parts) => parts.iter().all(|f| matches(f, state, obj, you, this)),
        Filter::Or(parts) => parts.iter().any(|f| matches(f, state, obj, you, this)),
        Filter::Not(f) => !matches(f, state, obj, you, this),
        Filter::HasType(t) => obj.characteristics().types.intersects(*t),
        Filter::LacksType(t) => !obj.characteristics().types.intersects(*t),
        Filter::HasSupertype(t) => obj.characteristics().supertypes.contains(*t),
        Filter::HasSubtype(s) => obj.characteristics().subtypes.contains(*s),
        Filter::HasColor(c) => obj.characteristics().colors.intersects(*c),
        Filter::IsColorless => obj.characteristics().colors.is_colorless(),
        Filter::ControlledByYou => obj.controller == you,
        Filter::ControlledByOpponent => obj.controller != you,
        Filter::OwnedByYou => obj.owner == you,
        Filter::Tapped => obj.status.contains(Status::TAPPED),
        Filter::Untapped => !obj.status.contains(Status::TAPPED),
        Filter::HasKeyword(k) => obj.characteristics().keywords.contains(*k),
        Filter::CmcAtMost(n) => obj.characteristics().mana_cost.cmc() <= *n,
        Filter::CmcAtLeast(n) => obj.characteristics().mana_cost.cmc() >= *n,
    }
}

/// Resolves a relative player reference to concrete players.
#[must_use]
pub fn players(rel: PlayerRel, state: &GameState, you: PlayerId) -> Vec<PlayerId> {
    match rel {
        PlayerRel::You => vec![you],
        PlayerRel::Opponent | PlayerRel::EachOpponent => state
            .players
            .iter()
            .filter(|p| p.id != you && !p.has_lost)
            .map(|p| p.id)
            .collect(),
        PlayerRel::EachPlayer => state
            .players
            .iter()
            .filter(|p| !p.has_lost)
            .map(|p| p.id)
            .collect(),
    }
}

/// Evaluates an [`Amount`].
#[must_use]
pub fn amount(
    amount: &Amount,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
    x: Option<u32>,
) -> u32 {
    match amount {
        Amount::Fixed(n) => *n,
        Amount::X => x.unwrap_or(0),
        Amount::CountOf { filter, zone } => {
            let objects: Vec<ObjectId> = match zone {
                ZoneSel::Battlefield => state.zones.list(ZoneLocation::Battlefield).clone(),
                ZoneSel::LibraryYou => state.zones.list(ZoneLocation::Library(you)).clone(),
                ZoneSel::GraveyardYou => state.zones.list(ZoneLocation::Graveyard(you)).clone(),
                ZoneSel::HandYou => state.zones.list(ZoneLocation::Hand(you)).clone(),
                ZoneSel::GraveyardAll => state
                    .players
                    .iter()
                    .flat_map(|p| {
                        state
                            .zones
                            .list(ZoneLocation::Graveyard(p.id))
                            .iter()
                            .copied()
                    })
                    .collect(),
            };
            objects
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| matches(filter, state, o, you, this))
                })
                .count() as u32
        }
    }
}

/// Legal target options for a [`TargetSpec`] (empty = cannot be chosen).
#[must_use]
pub fn target_options(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    match spec {
        TargetSpec::Object(filter) => state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| {
                state
                    .object(**id)
                    .is_some_and(|o| matches(filter, state, o, you, this))
            })
            .copied()
            .collect(),
        TargetSpec::Spell(filter) => state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter(|id| {
                state.object(**id).is_some_and(|o| {
                    o.characteristics().types.intersects(
                        TypeSet::INSTANT
                            .union(TypeSet::SORCERY)
                            .union(TypeSet::KINDRED),
                    ) && matches(filter, state, o, you, this)
                })
            })
            .copied()
            .collect(),
        TargetSpec::ThisObject => vec![this],
        TargetSpec::Player(_) => vec![], // player targeting: M2 (heads-up auto-resolves)
    }
}
