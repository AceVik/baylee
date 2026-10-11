//! Escape is granted in the graveyard, then paid for the chosen spell face.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::{CastModeDesc, CastModeKind};

pub(crate) fn escape_grants(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    _face: usize,
) -> SmallVec<[u8; 2]> {
    let mut grants = SmallVec::new();
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
/// instead changes the spell characteristics, including its cost (CR 718.3a).
#[allow(clippy::too_many_lines)] // Enumerates faces, prototype, kicker and modes under the same payment rules.
pub(crate) fn granted_escape_options(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    paying: Paying,
) -> Vec<CastModeDesc> {
    let mut options = Vec::new();
    let Some(object) = state
        .object(card)
        .filter(|object| object.zone == Zone::Graveyard && object.zone_owner == Some(player))
    else {
        return options;
    };
    let Some(def) = object.card.and_then(|card| lookup.card(card.index)) else {
        return options;
    };
    let fodder = escape_exile_options(state, player, card).len();
    for (index, face) in def.faces.iter().enumerate() {
        if face.types.contains(TypeSet::LAND)
            || (index != 0 && !face.castable_from_hand)
            || !has_a_printed_cost(&face.mana_cost)
            || !face_timing_allows(state, player, card, def, index)
        {
            continue;
        }
        let Some(additional_fodder) = additional_payment_capacity(state, player, card, face, &[])
        else {
            continue;
        };
        let fodder = fodder.saturating_add(additional_fodder);
        let grants = escape_grants(state, player, card, index);
        if grants.is_empty() {
            continue;
        }
        let printed = printed_reduction(state, face, player, card);
        let keywords = keyword_reduction(state, face, player, card);
        for prototype in [false, true] {
            if prototype && face.prototype.is_none() {
                continue;
            }
            let form = if prototype {
                face.prototype.map(SpellForm::Prototype)
            } else {
                None
            };
            let adjustment = face_adjustment(state, player, card, def, index, form);
            for kicked in [false, true] {
                if kicked && face.additional_costs.is_empty() {
                    continue;
                }
                let what = if kicked {
                    SpendFor::KickedSpell(card, index, form)
                } else if let Some(form) = form {
                    SpendFor::SpellAs(card, form)
                } else {
                    SpendFor::SpellOnFace(card, index)
                };
                let restricted = planning_pool(state, player, what, 0);
                let pool = restricted
                    .as_ref()
                    .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
                let base = if prototype {
                    face.prototype.expect("checked prototype").cost
                } else {
                    face.mana_cost
                };
                let base = if kicked {
                    face.additional_costs
                        .iter()
                        .fold(base, |cost, additional| cost.combine(&additional.mana))
                } else {
                    base
                };
                let mut add = |modes, raw: ManaCost| {
                    let cost = adjustment.price(raw, printed);
                    if paying == Paying::FromThePool
                        && !spell_affordable(
                            state,
                            player,
                            card,
                            pool,
                            &cost.with_x(0).with_less_generic(keywords),
                        )
                    {
                        return;
                    }
                    for &exile in &grants {
                        if fodder >= usize::from(exile) || paying == Paying::DuringTheCast {
                            options.push(CastModeDesc {
                                index: options.len() as u32,
                                kind: CastModeKind::EscapeGranted {
                                    face: index,
                                    modes,
                                    exile,
                                    kicked,
                                    prototype,
                                },
                                cost,
                            });
                        }
                    }
                };
                let reachable = |first, second| {
                    let first = if kicked {
                        face.kicked_targets.or(first)
                    } else {
                        first
                    };
                    requirement_is_reachable_for(first, state, player, card, Some((def, index)))
                        && requirement_is_reachable_for(
                            second,
                            state,
                            player,
                            card,
                            Some((def, index)),
                        )
                };
                if !modes_are_the_only_way(def, index)
                    && def
                        .abilities_for_face(index)
                        .iter()
                        .all(|ability| match ability {
                            baylee_cards_dsl::AbilityDef::Spell {
                                targets,
                                second_targets,
                                ..
                            } => reachable(*targets, *second_targets),
                            _ => true,
                        })
                {
                    add(0, base);
                }
                for ability in def.abilities_for_face(index) {
                    let baylee_cards_dsl::AbilityDef::ModalSpell { modes, choose } = ability else {
                        continue;
                    };
                    for set in mode_sets(modes, *choose) {
                        if chosen_modes(modes, set).all(|(_, mode)| {
                            mode.cost_override.is_none()
                                && reachable(mode.targets, mode.second_targets)
                        }) {
                            add(set, mode_set_cost(base, modes, set));
                        }
                    }
                }
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
