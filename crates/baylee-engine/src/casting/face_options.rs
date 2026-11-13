//! Face, mode and kicker choices shared by replacement-cost cast routes.
//!
//! The caller decides which zone permission applies and what replaces the
//! printed mana cost. This reader keeps timing, targets, price adjustments
//! and restricted-mana affordability identical between those routes.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::CastModeKind;

/// How a cast route obtains the mana part of each option's total cost.
#[derive(Clone, Copy)]
pub(crate) struct CostBasis {
    replacement: Option<ManaCost>,
    require_printed_cost: bool,
    allow_prototype: bool,
}

impl CostBasis {
    /// The printed face or prototype cost, as escape uses it.
    pub(crate) const fn printed() -> Self {
        Self {
            replacement: None,
            require_printed_cost: true,
            allow_prototype: true,
        }
    }

    /// One alternative cost instead of every printed mana cost. Prototype is
    /// another alternative cost and is therefore not combined with this route.
    pub(crate) const fn instead(cost: ManaCost) -> Self {
        Self {
            replacement: Some(cost),
            require_printed_cost: false,
            allow_prototype: false,
        }
    }
}

/// One fully enumerated choice, including whether its mana is available now.
#[derive(Clone, Copy)]
pub(crate) struct ReplacedCostOption {
    pub face: usize,
    pub modes: u8,
    pub kicked: bool,
    pub prototype: bool,
    pub cost: ManaCost,
    pub mana_ready: bool,
}

impl ReplacedCostOption {
    /// Keep kicker independent of the selected face and modal choice.
    pub(crate) fn cost_choice(
        self,
        def: &baylee_cards_dsl::CardDef,
    ) -> crate::engine::cast_wizard::CostChoice {
        crate::engine::cast_wizard::CostChoice {
            kicked: self.kicked,
            ..Some(self.ordinary_kind(def)).into()
        }
    }

    /// The ordinary wizard choice that carries this option's face and modes.
    /// The public replacement-cost route stays in `announced` beside it.
    pub(crate) fn ordinary_kind(self, def: &baylee_cards_dsl::CardDef) -> CastModeKind {
        if self.prototype {
            return CastModeKind::Prototype;
        }
        ordinary_face_kind(def, self.face, self.modes, self.kicked)
    }
}

/// Split a public route into the ordinary wizard choice used by later stages.
pub(crate) fn ordinary_face_kind(
    def: &baylee_cards_dsl::CardDef,
    face: usize,
    modes: u8,
    kicked: bool,
) -> CastModeKind {
    let single = def.abilities_for_face(face).iter().any(|ability| {
        matches!(ability, baylee_cards_dsl::AbilityDef::ModalSpell { choose, .. } if choose.is_one())
    });
    if modes != 0 && single {
        CastModeKind::Mode(modes.trailing_zeros() as usize)
    } else if modes != 0 {
        CastModeKind::Modes(modes)
    } else if face == 0 && kicked {
        CastModeKind::Kicked
    } else if face == 0 {
        CastModeKind::Normal
    } else {
        CastModeKind::Face(face)
    }
}

/// Enumerate every legal face/mode/kicker combination under `basis`.
///
/// Affordability is retained rather than used to discard a choice, so a caller
/// can distinguish an empty mana pool from an illegal target or another unpaid
/// non-mana cost. Only one card's faces and at most eight modes are examined.
#[allow(clippy::too_many_lines)] // one bounded face × prototype × kicker × mode enumeration
pub(crate) fn replaced_cost_options(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    paying: Paying,
    basis: CostBasis,
) -> Vec<ReplacedCostOption> {
    let mut options = Vec::new();
    let Some(object) = state.object(card) else {
        return options;
    };
    let Some(def) = object.card.and_then(|card| lookup.card(card.index)) else {
        return options;
    };
    for (index, face) in def.faces.iter().enumerate() {
        if face.types.contains(TypeSet::LAND)
            || (index != 0 && !face.castable_from_hand)
            || (basis.require_printed_cost && !has_a_printed_cost(&face.mana_cost))
            || !face_timing_allows(state, player, card, def, index)
            || face_is_forbidden(state, player, object, def, index)
        {
            continue;
        }
        let printed = printed_reduction(state, face, player, card);
        let keywords = keyword_reduction(state, face, player, card);
        for prototype in [false, true] {
            if prototype && (!basis.allow_prototype || face.prototype.is_none()) {
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
                } else if index == 0 {
                    SpendFor::Spell(card)
                } else {
                    SpendFor::SpellOnFace(card, index)
                };
                let restricted = planning_pool(state, player, what, 0);
                let pool = restricted
                    .as_ref()
                    .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
                let base = basis.replacement.unwrap_or_else(|| {
                    if prototype {
                        face.prototype.expect("checked prototype").cost
                    } else {
                        face.mana_cost
                    }
                });
                let base = if kicked {
                    face.additional_costs
                        .iter()
                        .fold(base, |cost, additional| cost.combine(&additional.mana))
                } else {
                    base
                };
                let mut add = |modes, raw: ManaCost| {
                    let cost = adjustment.price(raw, printed);
                    let affordable = cost.with_x(0);
                    let affordable = keyword_priced(state, face, player, &affordable, keywords);
                    let mana_ready = paying == Paying::DuringTheCast
                        || source_affordable(
                            state,
                            player,
                            what,
                            pool,
                            &affordable,
                            &[],
                            Some(adjustment.allowance(
                                state,
                                what,
                                printed.saturating_add(keywords),
                            )),
                        );
                    options.push(ReplacedCostOption {
                        face: index,
                        modes,
                        kicked,
                        prototype,
                        cost,
                        mana_ready,
                    });
                };
                let reachable = |first, second, selected_domain: Option<bool>| {
                    let first = if kicked {
                        face.kicked_targets.or(first)
                    } else {
                        first
                    };
                    requirement_is_reachable_for(
                        first,
                        state,
                        player,
                        card,
                        Some((def, index)),
                        selected_domain
                            .unwrap_or_else(|| crate::eval::target_domain::walls(first, second)),
                    ) && requirement_is_reachable_for(
                        second,
                        state,
                        player,
                        card,
                        Some((def, index)),
                        selected_domain
                            .unwrap_or_else(|| crate::eval::target_domain::walls(first, second)),
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
                            } => reachable(*targets, *second_targets, None),
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
                                && reachable(
                                    mode.targets,
                                    mode.second_targets,
                                    Some(crate::eval::target_domain::modes_wall_only(modes, set)),
                                )
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
