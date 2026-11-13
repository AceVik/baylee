//! Escape is granted in the graveyard, then paid for the chosen spell face.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::{CastModeDesc, CastModeKind};

pub(crate) fn escape_grants(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> SmallVec<[u8; 2]> {
    let mut grants = SmallVec::new();
    if !state.effects.may_grant_graveyard_or_top_play() {
        return grants;
    }
    // The ability belongs to the card before casting begins. Jurassic Park's
    // Dinosaur may therefore choose its non-Dinosaur Adventure when escaping.
    let Some(object) = state.object(card) else {
        return grants;
    };
    if object.zone != Zone::Graveyard || object.zone_owner != Some(player) {
        return grants;
    }
    for effect in state.effects.iter() {
        if let baylee_cards_dsl::Modifier::GrantsEscape(exile) = effect.modifier
            && crate::effects::applies_to(state, effect, object)
            && !grants.contains(&exile)
        {
            grants.push(exile);
        }
    }
    grants
}

/// Shared by the priority offer and the cast wizard; alternative costs cannot
/// be combined with overload or another alternative (CR 118.9a). Prototype
/// instead changes the spell characteristics, including its cost (CR 719.3a).
pub(crate) fn granted_escape_options(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    paying: Paying,
) -> Vec<CastModeDesc> {
    let mut options = Vec::new();
    // First, because the walk below is asked of every card in a graveyard on
    // every offer, and almost nothing grants escape.
    let grants = escape_grants(state, player, card);
    if grants.is_empty()
        || !state.object(card).is_some_and(|object| {
            object.zone == Zone::Graveyard && object.zone_owner == Some(player)
        })
    {
        return options;
    }
    let Some(def) = state
        .object(card)
        .and_then(|object| object.card)
        .and_then(|card| lookup.card(card.index))
    else {
        return options;
    };
    let fodder = escape_exile_options(state, player, card).len();
    for choice in super::face_options::replaced_cost_options(
        state,
        lookup,
        player,
        card,
        paying,
        super::face_options::CostBasis::printed(),
    ) {
        if !choice.mana_ready {
            continue;
        }
        let Some(additional_fodder) = super::additional_payment_capacity_for(
            state,
            player,
            card,
            &def.faces[choice.face],
            choice.cost_choice(def),
            &[],
        ) else {
            continue;
        };
        let fodder = fodder.saturating_add(additional_fodder);
        for &exile in &grants {
            if fodder >= usize::from(exile) || paying == Paying::DuringTheCast {
                options.push(CastModeDesc {
                    index: options.len() as u32,
                    kind: CastModeKind::EscapeGranted {
                        face: choice.face,
                        modes: choice.modes,
                        exile,
                        kicked: choice.kicked,
                        prototype: choice.prototype,
                    },
                    cost: choice.cost,
                });
            }
        }
    }
    options
}

/// Whether a cast option pays the face's mana cost, possibly with additional
/// costs, rather than a second alternative cost (CR 118.9a).
pub(crate) fn mana_cost_mode(
    def: &baylee_cards_dsl::CardDef,
    face: usize,
    kind: CastModeKind,
) -> bool {
    match kind {
        CastModeKind::Normal | CastModeKind::Kicked => true,
        CastModeKind::Prototype => def.faces[face].prototype.is_some(),
        CastModeKind::Face(index) => index == face && !def.faces[face].disturb,
        CastModeKind::Mode(index) => def.abilities_for_face(face).iter().any(|ability| {
            matches!(ability, baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. }
                if modes.get(index).is_some_and(|mode| mode.cost_override.is_none()))
        }),
        CastModeKind::Modes(set) => def.abilities_for_face(face).iter().any(|ability| {
            matches!(ability, baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. }
                if chosen_modes(modes, set).all(|(_, mode)| mode.cost_override.is_none()))
        }),
        _ => false,
    }
}
