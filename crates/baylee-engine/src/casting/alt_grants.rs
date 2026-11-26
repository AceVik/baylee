//! Continuous alternative-cost casting permissions (`Modifier::MayCastWithAltCost`,
//! `CR 118.9`, `CR 601.3e`).

use super::face_options::{CostBasis, replaced_cost_options};
use super::{
    CastError, Paying, additional_payment_capacity_for, adventure_permission,
    graveyard_face_permissions, library_top_life_value, may_begin_casting, warp_exile_permission,
};
use crate::choice::{CastModeDesc, CastModeKind};
use crate::effects::ContinuousEffect;
use crate::object::{Characteristics, GameObject, Rider};
use crate::state::{CardLookup, GameState};
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{AltCastPrice, AltCastZones, CardDef, Modifier, TypeSet};
use baylee_core::ids::{DamageSourceRef, EffectId, ObjectId, PlayerId};
use baylee_core::mana::ManaCost;
use std::sync::Arc;

/// Route metadata attached to a `CastWizard` option backed by `MayCastWithAltCost`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct AltCastGrantRoute {
    pub effect_id: EffectId,
    pub source: Option<ObjectId>,
    pub grant_zone_permission: bool,
    pub once_per_turn: bool,
    pub is_alt_cost: bool,
    pub prototype: bool,
    pub face: usize,
    pub modes: u8,
    pub kicked: bool,
    pub base_mana: Option<ManaCost>,
    pub extra_cost: Option<baylee_cards_dsl::Cost>,
    pub life: Option<u32>,
    pub exile_as_flashback: bool,
    pub exile_instead_of_graveyard: bool,
    pub harmonize: bool,
    pub is_flashback_grant: bool,
}

fn is_linked_to_source(state: &GameState, effect: &ContinuousEffect, obj: &GameObject) -> bool {
    let Some(source) = effect.source.and_then(|id| state.object(id)) else {
        return false;
    };
    let link = Rider::LinkedExile {
        source: DamageSourceRef {
            object: source.id,
            version: source.version,
        },
        acquisition: source.rules_acquisition(),
    };
    let with = Rider::ExiledWith {
        host: source.id,
        version: u16::try_from(source.version).unwrap_or(u16::MAX),
    };
    obj.riders.contains(&link) || obj.riders.contains(&with)
}

fn zone_matches_grant(
    state: &GameState,
    player: PlayerId,
    effect: &ContinuousEffect,
    obj: &GameObject,
    zones: AltCastZones,
) -> bool {
    match zones {
        AltCastZones::AnyPermitted => matches!(
            obj.zone,
            Zone::Hand | Zone::Graveyard | Zone::Library | Zone::Exile | Zone::Command
        ),
        AltCastZones::Zones(zs) => zs.iter().any(|z| obj.zone.matches_ref(*z)),
        AltCastZones::TopOfLibrary => {
            obj.zone == Zone::Library
                && obj.zone_owner == Some(player)
                && state.zones.list(ZoneLocation::Library(player)).last() == Some(&obj.id)
        }
        AltCastZones::LinkedExile => {
            obj.zone == Zone::Exile && is_linked_to_source(state, effect, obj)
        }
    }
}

/// Whether `player` already holds a non-alternative-cost zone permission to cast
/// `card`'s `face` (`CR 118.9a` + `MayPlayDontGrantZonePermissions`).
fn has_plain_zone_permission(
    state: &GameState,
    player: PlayerId,
    obj: &GameObject,
    def: &CardDef,
    face: usize,
) -> bool {
    let card = obj.id;
    let face_types = def.faces[face].types;
    match obj.zone {
        Zone::Hand => obj.zone_owner == Some(player),
        Zone::Command => {
            obj.zone_owner == Some(player)
                && state
                    .commanders
                    .get(usize::from(player.get()))
                    .is_some_and(|cs| cs.iter().any(|c| c.object == card))
        }
        Zone::Graveyard => {
            obj.zone_owner == Some(player)
                && !graveyard_face_permissions(state, player, card, face).is_empty()
        }
        Zone::Exile => {
            (face == 0 && adventure_permission(state, player, card))
                || warp_exile_permission(state, player, card)
                || obj.riders.contains(&Rider::PlayableFromExileFor(player))
                || state.play_permissions.iter().any(|p| {
                    !p.free
                        && p.grants_to(state, player, card, obj.version)
                        && p.spell_types.is_none_or(|t| t.intersects(face_types))
                })
        }
        Zone::Library | Zone::Battlefield | Zone::Stack | Zone::OutsideGame => false,
    }
}

fn effect_allows_player(
    state: &GameState,
    effect: &ContinuousEffect,
    player: PlayerId,
    who: baylee_cards_dsl::PlayerRel,
    once_per_turn: bool,
) -> bool {
    crate::effects::source_is_present(state, effect)
        && (!once_per_turn
            || !state
                .per_turn
                .graveyard_allowances_used
                .contains(&(player, effect.id)))
        && crate::eval::players_for_source(who, state, effect.controller, effect.source)
            .is_some_and(|ps| ps.contains(&player))
}

fn projected_face_spell(obj: &GameObject, def: &'static CardDef, face: usize) -> GameObject {
    let mut projected = obj.clone();
    if face != 0 || def.faces.len() > 1 {
        projected.base = Arc::new(Characteristics::from_face(def, face, obj.base.name));
        projected.cache.clear();
        projected.face_index = u8::try_from(face).unwrap_or(0);
    }
    projected
}

fn grant_matches_face(
    state: &GameState,
    player: PlayerId,
    effect: &ContinuousEffect,
    obj: &GameObject,
    def: &'static CardDef,
    face: usize,
) -> bool {
    let Modifier::MayCastWithAltCost {
        who,
        zones,
        grant_zone_permission,
        max_mana_value,
        once_per_turn,
        ..
    } = effect.modifier
    else {
        return false;
    };
    if def.faces[face].types.contains(TypeSet::LAND)
        || !effect_allows_player(state, effect, player, who, once_per_turn)
        || !zone_matches_grant(state, player, effect, obj, zones)
        || (!grant_zone_permission && !has_plain_zone_permission(state, player, obj, def, face))
    {
        return false;
    }
    if let Some(bound) = max_mana_value {
        let src = effect.source.unwrap_or(obj.id);
        let max_mv = crate::eval::amount(&bound, state, effect.controller, src, None);
        if def.faces[face].mana_cost.cmc() > max_mv {
            return false;
        }
    }
    let prospective = projected_face_spell(obj, def, face);
    crate::effects::matches_filter(state, effect, &prospective)
}

fn flashback_cost_matches_face(
    state: &GameState,
    player: PlayerId,
    effect: &ContinuousEffect,
    obj: &GameObject,
    def: &'static CardDef,
    face: usize,
) -> Option<ManaCost> {
    let Modifier::GrantsFlashbackCost(cost) = effect.modifier else {
        return None;
    };
    if obj.zone != Zone::Graveyard
        || obj.zone_owner != Some(player)
        || !def.faces[face]
            .types
            .intersects(TypeSet::INSTANT.union(TypeSet::SORCERY))
        || !crate::effects::applies_to(state, effect, obj)
    {
        return None;
    }
    Some(cost)
}

fn harmonize_cost_matches_face(
    state: &GameState,
    player: PlayerId,
    effect: &ContinuousEffect,
    obj: &GameObject,
    def: &'static CardDef,
    face: usize,
) -> Option<ManaCost> {
    let Modifier::GrantsHarmonizeManaCost = effect.modifier else {
        return None;
    };
    if obj.zone != Zone::Graveyard
        || obj.zone_owner != Some(player)
        || !def.faces[face]
            .types
            .intersects(TypeSet::INSTANT.union(TypeSet::SORCERY))
        || !crate::effects::applies_to(state, effect, obj)
        || !super::has_a_printed_cost(&def.faces[face].mana_cost)
    {
        return None;
    }
    Some(def.faces[face].mana_cost)
}

fn targeted_gy_grant_matches_face(
    state: &GameState,
    player: PlayerId,
    effect: &ContinuousEffect,
    obj: &GameObject,
    def: &'static CardDef,
    face: usize,
) -> bool {
    let Modifier::MayCastFromGraveyardTargeted { .. } = effect.modifier else {
        return false;
    };
    !def.faces[face].types.contains(TypeSet::LAND)
        && player == effect.controller
        && obj.zone == Zone::Graveyard
        && crate::effects::applies_to(state, effect, obj)
}

/// Maximum projected power among untapped creatures `player` controls for
/// Harmonize (`CR 702.183a`).
#[must_use]
pub(crate) fn harmonize_max_power(state: &GameState, player: PlayerId) -> u32 {
    state
        .battlefield_seen()
        .filter_map(|id| state.object(id))
        .filter(|o| {
            o.controller == player
                && o.characteristics().types.contains(TypeSet::CREATURE)
                && !o.status.contains(crate::object::Status::TAPPED)
        })
        .filter_map(|o| o.characteristics().power)
        .map(|p| u32::try_from(p.max(0)).unwrap_or(u32::MAX))
        .max()
        .unwrap_or(0)
}

fn effect_matches_land_object(
    state: &GameState,
    effect: &ContinuousEffect,
    obj: &GameObject,
) -> bool {
    if crate::effects::matches_filter(state, effect, obj) {
        return true;
    }
    let Some(def) = obj.card.and_then(|c| state.bases.card(c.index)) else {
        return false;
    };
    def.land_faces_from_hand().any(|face| {
        let prospective = projected_face_spell(obj, def, face);
        crate::effects::matches_filter(state, effect, &prospective)
    })
}

/// Whether any active `MayCastWithAltCost` with `grant_zone_permission && allow_lands`
/// opens `card` to be played as a land by `player`.
#[must_use]
pub fn alt_cast_land_open(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    if !state.effects.may_grant_graveyard_or_top_play() {
        return false;
    }
    let Some(raw) = state.object(card) else {
        return false;
    };
    let face_up = super::playable_exile_projection(state, player, raw);
    let obj: &GameObject = &face_up;
    state.effects.iter().any(|effect| {
        let Modifier::MayCastWithAltCost {
            who,
            zones,
            grant_zone_permission: true,
            allow_lands: true,
            once_per_turn,
            ..
        } = effect.modifier
        else {
            return false;
        };
        effect_allows_player(state, effect, player, who, once_per_turn)
            && zone_matches_grant(state, player, effect, obj, zones)
            && effect_matches_land_object(state, effect, obj)
    })
}

/// Whether any active `MayCastWithAltCost` with `grant_zone_permission && allow_lands`
/// can reach `Zone::Graveyard` for `player`.
#[must_use]
pub(crate) fn alt_cast_graveyard_land_zone_open(state: &GameState, player: PlayerId) -> bool {
    state.effects.may_grant_graveyard_or_top_play()
        && state.effects.iter().any(|effect| {
            let Modifier::MayCastWithAltCost {
                who,
                zones,
                grant_zone_permission: true,
                allow_lands: true,
                once_per_turn,
                ..
            } = effect.modifier
            else {
                return false;
            };
            let reaches_gy = match zones {
                AltCastZones::AnyPermitted => true,
                AltCastZones::Zones(zs) => zs.iter().any(|z| Zone::Graveyard.matches_ref(*z)),
                AltCastZones::TopOfLibrary | AltCastZones::LinkedExile => false,
            };
            reaches_gy && effect_allows_player(state, effect, player, who, once_per_turn)
        })
}

/// Records the `once_per_turn` allowance used when `card` is played as a land
/// via `Modifier::MayCastWithAltCost`, preferring any unlimited grant first.
pub(crate) fn consume_alt_cast_land_allowance(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
) {
    if !state.effects.may_grant_graveyard_or_top_play() {
        return;
    }
    let Some(raw) = state.object(card) else {
        return;
    };
    let face_up = super::playable_exile_projection(state, player, raw);
    let obj: &GameObject = &face_up;
    let mut once_id = None;
    for effect in state.effects.iter() {
        let Modifier::MayCastWithAltCost {
            who,
            zones,
            grant_zone_permission: true,
            allow_lands: true,
            once_per_turn,
            ..
        } = effect.modifier
        else {
            continue;
        };
        if effect_allows_player(state, effect, player, who, once_per_turn)
            && zone_matches_grant(state, player, effect, obj, zones)
            && effect_matches_land_object(state, effect, obj)
        {
            if !once_per_turn {
                return;
            }
            if once_id.is_none() {
                once_id = Some(effect.id);
            }
        }
    }
    if let Some(id) = once_id {
        state.per_turn.graveyard_allowances_used.push((player, id));
    }
}

/// Whether `face` of `def` is opened for casting by any active `MayCastWithAltCost`.
#[must_use]
pub(crate) fn alt_cast_face_open(
    state: &GameState,
    lookup: &impl CardLookup,
    player: PlayerId,
    card: ObjectId,
    face: usize,
) -> bool {
    if !state.effects.may_grant_graveyard_or_top_play() {
        return false;
    }
    let Some(raw) = state.object(card) else {
        return false;
    };
    let face_up = super::playable_exile_projection(state, player, raw);
    let obj: &GameObject = &face_up;
    let Some(def) = obj.card.and_then(|c| lookup.card(c.index)) else {
        return false;
    };
    face < def.faces.len()
        && state.effects.iter().any(|effect| {
            (matches!(
                effect.modifier,
                Modifier::MayCastWithAltCost {
                    grant_zone_permission: true,
                    ..
                }
            ) && grant_matches_face(state, player, effect, obj, def, face))
                || targeted_gy_grant_matches_face(state, player, effect, obj, def, face)
                || harmonize_cost_matches_face(state, player, effect, obj, def, face).is_some()
        })
}

/// Whether `face` of `def` is granted flash by an active `MayCastWithAltCost { with_flash: true }`.
#[must_use]
pub(crate) fn alt_cast_face_with_flash(
    state: &GameState,
    lookup: &impl CardLookup,
    player: PlayerId,
    card: ObjectId,
    face: usize,
) -> bool {
    if !state.effects.may_grant_graveyard_or_top_play() {
        return false;
    }
    let Some(raw) = state.object(card) else {
        return false;
    };
    let face_up = super::playable_exile_projection(state, player, raw);
    let obj: &GameObject = &face_up;
    let Some(def) = obj.card.and_then(|c| lookup.card(c.index)) else {
        return false;
    };
    face < def.faces.len()
        && super::timing_allows(
            state,
            player,
            TypeSet::INSTANT,
            baylee_cards_dsl::KeywordSet::EMPTY,
        )
        && state.effects.iter().any(|effect| {
            matches!(
                effect.modifier,
                Modifier::MayCastWithAltCost {
                    with_flash: true,
                    ..
                }
            ) && grant_matches_face(state, player, effect, obj, def, face)
        })
}

/// Whether any active `MayCastWithAltCost` with `grant_zone_permission: true`
/// opens `card`'s zone for casting by `player`.
#[must_use]
pub(crate) fn alt_cast_zone_spell_open(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> bool {
    if !state.effects.may_grant_graveyard_or_top_play() {
        return false;
    }
    let Some(raw) = state.object(card) else {
        return false;
    };
    let face_up = super::playable_exile_projection(state, player, raw);
    let obj: &GameObject = &face_up;
    state.effects.iter().any(|effect| match effect.modifier {
        Modifier::MayCastWithAltCost {
            who,
            zones,
            grant_zone_permission: true,
            once_per_turn,
            ..
        } => {
            effect_allows_player(state, effect, player, who, once_per_turn)
                && zone_matches_grant(state, player, effect, obj, zones)
        }
        Modifier::MayCastFromGraveyardTargeted { .. } => {
            obj.zone == Zone::Graveyard
                && player == effect.controller
                && crate::effects::applies_to(state, effect, obj)
        }
        Modifier::GrantsHarmonizeManaCost => {
            obj.zone == Zone::Graveyard
                && obj.zone_owner == Some(player)
                && crate::effects::applies_to(state, effect, obj)
        }
        _ => false,
    })
}

/// Whether any active `MayCastWithAltCost` grant could apply to `card` for `player`.
#[must_use]
pub(crate) fn has_alt_cast_grant(
    state: &GameState,
    lookup: &impl CardLookup,
    player: PlayerId,
    card: ObjectId,
) -> bool {
    if !state.effects.may_grant_graveyard_or_top_play() {
        return false;
    }
    let Some(raw) = state.object(card) else {
        return false;
    };
    let face_up = super::playable_exile_projection(state, player, raw);
    let obj: &GameObject = &face_up;
    let Some(def) = obj.card.and_then(|c| lookup.card(c.index)) else {
        return false;
    };
    state.effects.iter().any(|effect| {
        (matches!(effect.modifier, Modifier::MayCastWithAltCost { .. })
            && (0..def.faces.len())
                .any(|face| grant_matches_face(state, player, effect, obj, def, face)))
            || (0..def.faces.len()).any(|face| {
                flashback_cost_matches_face(state, player, effect, obj, def, face).is_some()
                    || harmonize_cost_matches_face(state, player, effect, obj, def, face).is_some()
                    || targeted_gy_grant_matches_face(state, player, effect, obj, def, face)
            })
    })
}

fn price_basis(
    price: AltCastPrice,
    with_flash: bool,
) -> (
    CostBasis,
    Option<ManaCost>,
    Option<baylee_cards_dsl::Cost>,
    bool,
) {
    match price {
        AltCastPrice::Mana(cost) => (
            CostBasis::instead(cost).with_flash(with_flash),
            Some(cost),
            None,
            true,
        ),
        AltCastPrice::PayLifeEqualToManaValue => (
            CostBasis::instead(ManaCost::ZERO).with_flash(with_flash),
            None,
            None,
            true,
        ),
        AltCastPrice::Normal => (
            CostBasis::printed().with_flash(with_flash),
            None,
            None,
            false,
        ),
        AltCastPrice::NormalPlus(cost) => (
            CostBasis::printed_plus(cost.mana).with_flash(with_flash),
            None,
            Some(cost),
            false,
        ),
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn collect_effect_grant_options(
    state: &GameState,
    lookup: &impl CardLookup,
    player: PlayerId,
    card: ObjectId,
    obj: &GameObject,
    def: &'static CardDef,
    effect: &ContinuousEffect,
    paying: Paying,
    mana_short: &mut bool,
    life_short: &mut bool,
    out: &mut Vec<(CastModeDesc, AltCastGrantRoute)>,
) {
    let (
        grant_zone_permission,
        with_flash,
        once_per_turn,
        price,
        exile_as_flashback,
        exile_instead_of_graveyard,
        harmonize,
        is_flashback_grant,
    ) = match effect.modifier {
        Modifier::MayCastWithAltCost {
            grant_zone_permission,
            with_flash,
            once_per_turn,
            price,
            ..
        } => (
            grant_zone_permission,
            with_flash,
            once_per_turn,
            price,
            false,
            false,
            false,
            false,
        ),
        Modifier::GrantsFlashbackCost(cost) => {
            if !(0..def.faces.len()).any(|face| {
                flashback_cost_matches_face(state, player, effect, obj, def, face).is_some()
            }) {
                return;
            }
            (
                true,
                false,
                false,
                AltCastPrice::Mana(cost),
                true,
                false,
                false,
                true,
            )
        }
        Modifier::GrantsHarmonizeManaCost => {
            if !(0..def.faces.len()).any(|face| {
                harmonize_cost_matches_face(state, player, effect, obj, def, face).is_some()
            }) {
                return;
            }
            (
                true,
                false,
                false,
                AltCastPrice::Normal,
                true,
                false,
                true,
                false,
            )
        }
        Modifier::MayCastFromGraveyardTargeted {
            free,
            exile_instead_of_graveyard,
            ..
        } => {
            if !(0..def.faces.len())
                .any(|face| targeted_gy_grant_matches_face(state, player, effect, obj, def, face))
            {
                return;
            }
            let p = if free {
                AltCastPrice::Mana(ManaCost::ZERO)
            } else {
                AltCastPrice::Normal
            };
            (
                true,
                false,
                false,
                p,
                false,
                exile_instead_of_graveyard,
                false,
                false,
            )
        }
        _ => return,
    };
    let is_targeted_gy = matches!(
        effect.modifier,
        Modifier::MayCastFromGraveyardTargeted { .. }
    );
    if !is_flashback_grant
        && !harmonize
        && !is_targeted_gy
        && !(0..def.faces.len())
            .any(|face| grant_matches_face(state, player, effect, obj, def, face))
    {
        return;
    }
    let (basis, base_mana, extra_cost, mut is_alt_cost) = price_basis(price, with_flash);
    if harmonize {
        is_alt_cost = true;
    }
    for choice in replaced_cost_options(state, lookup, player, card, paying, basis) {
        let face_ok = if is_flashback_grant {
            flashback_cost_matches_face(state, player, effect, obj, def, choice.face).is_some()
        } else if harmonize {
            harmonize_cost_matches_face(state, player, effect, obj, def, choice.face).is_some()
        } else if is_targeted_gy {
            targeted_gy_grant_matches_face(state, player, effect, obj, def, choice.face)
        } else {
            grant_matches_face(state, player, effect, obj, def, choice.face)
        };
        if !face_ok {
            continue;
        }
        let mut costs = choice.cost_choice(def);
        if let Some(extra) = extra_cost {
            costs.route_extra_parts = extra.parts;
        }
        let mv_life = match price {
            AltCastPrice::PayLifeEqualToManaValue => Some(library_top_life_value(def, choice.face)),
            AltCastPrice::Mana(_) | AltCastPrice::Normal | AltCastPrice::NormalPlus(_) => None,
        };
        let fixed = crate::engine::cast_wizard::fixed_life_cost(&def.faces[choice.face], costs);
        let total_life = mv_life
            .map_or(0_i32, |life| i32::try_from(life).unwrap_or(i32::MAX))
            .saturating_add(fixed);
        if total_life > 0 && !state.can_pay_life(player, total_life) {
            *life_short = true;
            continue;
        }
        let fixed_energy =
            crate::engine::cast_wizard::fixed_energy_cost(&def.faces[choice.face], costs);
        if fixed_energy > u32::from(state.players[usize::from(player.get())].energy) {
            continue;
        }
        if additional_payment_capacity_for(state, player, card, &def.faces[choice.face], costs, &[])
            .is_none()
        {
            continue;
        }
        let harmonize_ok = harmonize && paying != Paying::DuringTheCast && {
            let max_tap = harmonize_max_power(state, player);
            max_tap > 0 && {
                let what = if choice.face == 0 {
                    super::SpendFor::Spell(card)
                } else {
                    super::SpendFor::SpellOnFace(card, choice.face)
                };
                let restricted = super::planning_pool(state, player, what, 0);
                let pool = restricted
                    .as_ref()
                    .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
                let reduced = choice.cost.with_x(0).with_less_generic(max_tap);
                super::source_affordable(
                    state,
                    player,
                    what,
                    pool,
                    &reduced,
                    &[],
                    Some(super::live_allowance(state, player, what).reduced(max_tap)),
                )
            }
        };
        if !choice.mana_ready && !harmonize_ok {
            *mana_short = true;
            continue;
        }
        let kind = match mv_life {
            Some(life) => CastModeKind::LibraryTopLife {
                life,
                face: choice.face,
                modes: choice.modes,
                kicked: choice.kicked,
            },
            None => CastModeKind::Permission(effect.source),
        };
        out.push((
            CastModeDesc {
                index: 0,
                kind,
                cost: choice.cost,
            },
            AltCastGrantRoute {
                effect_id: effect.id,
                source: effect.source,
                grant_zone_permission,
                once_per_turn,
                is_alt_cost,
                prototype: choice.prototype,
                face: choice.face,
                modes: choice.modes,
                kicked: choice.kicked,
                base_mana,
                extra_cost,
                life: mv_life,
                exile_as_flashback,
                exile_instead_of_graveyard,
                harmonize,
                is_flashback_grant,
            },
        ));
    }
}

/// Enumerate all legal `(CastModeDesc, AltCastGrantRoute)` pairs from active
/// `MayCastWithAltCost` effects, returning a diagnostic `CastError` if matching
/// grants existed only short of life or mana.
pub(crate) fn alt_cast_grant_options(
    state: &GameState,
    lookup: &impl CardLookup,
    player: PlayerId,
    card: ObjectId,
    paying: Paying,
) -> (Vec<(CastModeDesc, AltCastGrantRoute)>, Option<CastError>) {
    let mut out = Vec::new();
    if !state.effects.may_grant_graveyard_or_top_play() {
        return (out, None);
    }
    let Some(raw) = state.object(card) else {
        return (out, None);
    };
    let face_up = super::playable_exile_projection(state, player, raw);
    let obj: &GameObject = &face_up;
    let Some(def) = obj.card.and_then(|c| lookup.card(c.index)) else {
        return (out, None);
    };
    if !may_begin_casting(state, player) {
        return (out, Some(CastError::Forbidden));
    }
    let mut mana_short = false;
    let mut life_short = false;
    for effect in state.effects.iter() {
        collect_effect_grant_options(
            state,
            lookup,
            player,
            card,
            obj,
            def,
            effect,
            paying,
            &mut mana_short,
            &mut life_short,
            &mut out,
        );
    }
    let diag = if !out.is_empty() {
        None
    } else if life_short {
        Some(CastError::NotEnoughLife)
    } else if mana_short {
        Some(CastError::NotEnoughMana)
    } else {
        None
    };
    (out, diag)
}

/// Re-validates at `pay_and_cast` time that `route`'s continuous effect is still
/// active and its once-per-turn allowance has not been spent.
#[must_use]
pub(crate) fn alt_cast_route_still_open(
    state: &GameState,
    lookup: &impl CardLookup,
    player: PlayerId,
    card: ObjectId,
    route: &AltCastGrantRoute,
) -> bool {
    let Some(raw) = state.object(card) else {
        return false;
    };
    let face_up = super::playable_exile_projection(state, player, raw);
    let obj: &GameObject = &face_up;
    let Some(def) = obj.card.and_then(|c| lookup.card(c.index)) else {
        return false;
    };
    state
        .effects
        .iter()
        .find(|fx| fx.id == route.effect_id)
        .is_some_and(|effect| {
            if route.is_flashback_grant {
                flashback_cost_matches_face(state, player, effect, obj, def, route.face).is_some()
            } else if route.harmonize {
                harmonize_cost_matches_face(state, player, effect, obj, def, route.face).is_some()
            } else if matches!(
                effect.modifier,
                Modifier::MayCastFromGraveyardTargeted { .. }
            ) {
                targeted_gy_grant_matches_face(state, player, effect, obj, def, route.face)
            } else {
                grant_matches_face(state, player, effect, obj, def, route.face)
            }
        })
}
