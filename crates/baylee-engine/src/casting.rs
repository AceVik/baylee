//! Casting and resolution.
//!
//! S2 scope: no modes/targets/X, auto-payment (CR 601.2h compressed).
//! The full stepwise `CastPlan` wizard (modes, alternative/additional
//! costs, targets, X, payment plans) lands with the ability runtime (M1.S3
//! / M2).

use crate::event::{Cause, GameEvent};
use crate::mana_pay;
use crate::object::ObjectKind;
use crate::state::{GameState, StateError};
use crate::turn::Phase;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::SpendRider;
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::{ManaColor, ManaCost, ManaFlags, ManaPool, ManaSpending, RestrictedMana};
use baylee_core::types::TypeSet;
use smallvec::SmallVec;

mod additional;
mod alt_grants;
mod escape;
mod face_options;
mod intrinsic;
mod lands;
mod library_top;
pub(crate) use additional::{
    additional_card_enters_graveyard, additional_payment_capacity, additional_payment_capacity_for,
    additional_payment_capacity_for_x, additional_spell_costs, distinct_answers_exist,
    extend_sacrifices_per_mana_symbol, pitch_costs_payable, remaining_spell_card_costs,
};
mod payment;
mod permissions;
pub(crate) mod player_locks;
mod pricing;
mod source_cost;
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use pricing::spell_increase;
pub(crate) use source_cost::*;
#[cfg(test)]
mod tests;

pub use alt_grants::alt_cast_land_open;
pub(crate) use alt_grants::{
    AltCastGrantRoute, alt_cast_face_open, alt_cast_face_with_flash, alt_cast_grant_options,
    alt_cast_graveyard_land_zone_open, alt_cast_route_still_open, alt_cast_zone_spell_open,
    consume_alt_cast_land_allowance, harmonize_max_power, has_alt_cast_grant,
};
pub(crate) use escape::{escape_grants, granted_escape_options, mana_cost_mode};
pub(crate) use face_options::{optional_flash_cost_options, ordinary_face_kind};
pub use intrinsic::*;
pub use lands::*;
pub use library_top::library_top_visible_to;
pub(crate) use library_top::{
    library_top_life_open, library_top_life_options, library_top_life_value, library_top_spell_open,
};
pub(crate) use payment::*;
#[cfg(feature = "test-support")]
pub use payment::{SpellForm, SpendFor, pay_mana_for, spendable_units};
pub(crate) use permissions::*;
pub(crate) use pricing::{
    PreviewCastKind, PreviewDynamicDelta, card_has_self_spell_cost_statics,
    collect_stack_modifiers, preview_cast_metric_bounds, preview_cast_metric_bounds_for,
    preview_max_mandatory_x,
};
#[cfg(feature = "test-support")]
pub use pricing::{
    SpellAdjustment, activation_increase, component_affordable, face_adjustment,
    normal_spell_affordable, north_star_effect, north_star_face_allowed, projected_spell,
    spell_adjustment, spell_adjustment_with_def, spell_mana_spending,
};
#[cfg(not(feature = "test-support"))]
pub(crate) use pricing::{
    SpellAdjustment, activation_increase, component_affordable, face_adjustment,
    normal_spell_affordable, north_star_effect, north_star_face_allowed, projected_spell,
    spell_adjustment, spell_adjustment_with_def, spell_mana_spending,
};
pub use pricing::{
    ability_mana_spending, commander_tax, convoke_sources, delve_sources, effective_blitz_cost,
    granted_stack_alternatives, has_convoke, has_delve, has_improvise, has_improvise_for,
    keyword_priced, keyword_priced_for, keyword_reduction, mana_cost_reduction, mana_is_wild,
    mana_spending, printed_reduction, reduction_amount, tap_colors, tap_colors_for,
    tap_to_pay_sources, tap_to_pay_sources_for, waterbend_sources,
};

/// Whether `player` can pay all non-mana flashback parts and mandatory
/// additional costs on `face` to cast `card_id` from the graveyard.
#[must_use]
pub fn flashback_parts_payable(
    state: &GameState,
    player: PlayerId,
    card_id: ObjectId,
    face: &'static baylee_cards_dsl::FaceDef,
) -> bool {
    additional_payment_capacity_for(
        state,
        player,
        card_id,
        face,
        Some(crate::choice::CastModeKind::Flashback),
        &[],
    )
    .is_some()
}

/// Why a card cannot be cast right now (validated before the action).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CastError {
    /// The card is not in the caster's hand.
    #[error("card is not in your hand")]
    NotInHand,
    /// Timing rules forbid it (main phase, own turn, empty stack).
    #[error("sorcery-speed timing not met")]
    BadTiming,
    /// The mana pool cannot pay the cost.
    #[error("not enough mana")]
    NotEnoughMana,
    /// The payable life total cannot pay this cast's fixed life costs.
    #[error("not enough life")]
    NotEnoughLife,
    /// Costs with {X}/{Y}/{Z} need the full wizard (M1.S3).
    #[error("variable costs not supported yet")]
    VariableCost,
    /// Every way of casting the card fails on its own terms: each one is
    /// either unpayable or has nothing to point at.
    ///
    /// The variant the note beside [`can_cast`]'s face probe has asked for
    /// since it was written — "a `CastError` variant that does not call a
    /// target problem *not enough mana*". A modal spell is where the
    /// difference stopped being cosmetic: Damn on three Swamps could pay for
    /// "destroy target creature" and had nothing to destroy, and could point
    /// the overload at everything and not pay `{2}{W}{W}` for it. Neither
    /// half is a mana problem, and answering with one sent a player looking
    /// for a fourth land.
    #[error("no way to cast this spell")]
    NoWayToCast,
    /// A continuous effect forbids this cast outright (CR 601.3a): Silence,
    /// Ranger-Captain of Eos.
    ///
    /// Its own variant rather than [`Self::BadTiming`], which is what a
    /// sorcery-speed lock answers: a player told "sorcery-speed timing not
    /// met" on their own main phase with an empty stack goes looking for a
    /// rule that is not there.
    #[error("an effect forbids casting this spell")]
    Forbidden,
}

/// An exiled Adventure grants only its resolved spell's controller permission
/// to play the normal face. The rider expires when the card leaves exile.
#[must_use]
pub fn adventure_permission(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    state.object(card).is_some_and(|o| {
        o.zone == Zone::Exile && o.riders.contains(&crate::object::Rider::Adventure(player))
    })
}

/// An independent permission can cast the Adventure again (CR 716.3d).
#[must_use]
pub(crate) fn adventure_only_permission(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> bool {
    adventure_permission(state, player, card)
        && play_permission(state, player, card).is_none()
        && state.object(card).is_none_or(|o| {
            !o.riders
                .contains(&crate::object::Rider::PlayableFromExileFor(player))
        })
}

/// A castable back face: modal or Adventure, excluding land plays, disturb,
/// and an Adventure cast using only its own resolved Adventure permission.
pub fn castable_back_faces(
    def: &baylee_cards_dsl::CardDef,
    on_adventure: bool,
) -> impl Iterator<Item = (usize, &baylee_cards_dsl::FaceDef)> {
    castable_back_faces_in_zone(def, on_adventure, false)
}

/// Native disturb and aftermath are available only when their graveyard permission is active.
/// Ordinary callers still exclude them from hand, exile and free-cast menus.
fn castable_back_faces_in_zone(
    def: &baylee_cards_dsl::CardDef,
    on_adventure: bool,
    in_own_graveyard: bool,
) -> impl Iterator<Item = (usize, &baylee_cards_dsl::FaceDef)> {
    def.faces
        .iter()
        .enumerate()
        .skip(1)
        .filter(move |(_, face)| {
            (face.castable_from_hand || (in_own_graveyard && (face.disturb || face.aftermath)))
                && !face.types.contains(TypeSet::LAND)
                && !(on_adventure && face.adventure)
        })
}

/// Whether one face of `card` has enough legal targets to be cast at all.
///
/// A spell whose only target requirement cannot be met is a cast that ends
/// at the targeting step with nothing paid and nothing on the stack, so a
/// card offered on that footing is a button that only ever errors and an
/// agent picks it again on every pass, the state being unchanged.
///
/// Deliberately conservative, and answers `true` whenever it cannot be sure:
/// a modal spell chooses targets per mode, an X-counted requirement depends
/// on a number nobody has picked yet, and a player is not an object. All
/// three stay the wizard's problem.
///
/// The `face` parameter is the whole reason this is not a method on the
/// engine any more. It used to ask face 0 and nothing else, which was the
/// same answer for every way of casting a card — and an adventure
/// (CR 716) is a *different spell* with its own target line on the back of a
/// creature that has none. Swift Spiral has to find a nontoken creature;
/// Twining Twins in front of it has nothing to target, so the front face
/// answered "no requirement, therefore yes" for a mode that would then be
/// refused.
#[must_use]
pub fn face_has_a_legal_target(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    face: usize,
) -> bool {
    let Some(def) = state
        .object(card)
        .and_then(|o| o.card)
        .and_then(|c| lookup.card(c.index))
    else {
        return true;
    };
    let abilities = def.abilities_for_face(face);
    // A modal spell is answered mode by mode (CR 700.2): the card is castable
    // when as many of its modes as it must choose can each be pointed at
    // something, through every instance of the word each prints — one for
    // "choose one", two for "choose two" — and which ones is
    // `cast_options`' question, not this one's.
    let mut modal = abilities
        .iter()
        .filter_map(baylee_cards_dsl::AbilityDef::modal_spell);
    if let Some((modes, choose, can_repeat)) = modal.next() {
        return modal_face_has_legal_target(
            state,
            player,
            card,
            (def, face),
            modes,
            choose,
            can_repeat,
        );
    }

    let spell = abilities.iter().find_map(|a| match a {
        baylee_cards_dsl::AbilityDef::Spell {
            effects,
            targets,
            second_targets,
            third_targets,
            ..
        } => Some((*effects, *targets, *second_targets, *third_targets)),
        _ => None,
    });
    let (effects, req, second, third) = spell.unwrap_or_default();
    if let Some(r) = req
        && r.min > 0
        && def
            .faces
            .get(face)
            .is_some_and(|f| !f.mana_cost.has_variable())
        && let Some((_, total)) = crate::engine::division::effects_divided_spec(
            effects,
            state,
            player,
            crate::eval::live_context(state, card),
            state.object(card).map(|o| o.version),
            0,
        )
        && total < u32::from(r.min)
    {
        return false;
    }
    // Every instance of the word has to be satisfiable (CR 601.2c): Khalni
    // Ambush with no creature on the other side of the table is not a spell
    // that can be cast, however many of the caster's own it could name.
    requirements3_are_reachable_for(
        req,
        second,
        third,
        state,
        player,
        card,
        Some((def, face)),
        None,
    ) || def
        .faces
        .get(face)
        .and_then(|f| f.kicked_targets)
        .is_some_and(|kicked_req| {
            requirements3_are_reachable_for(
                Some(kicked_req),
                second,
                third,
                state,
                player,
                card,
                Some((def, face)),
                None,
            )
        })
}

fn modal_face_has_legal_target(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    printed: (&baylee_cards_dsl::CardDef, usize),
    modes: &'static [baylee_cards_dsl::SpellMode],
    choose: baylee_cards_dsl::ModeCount,
    can_repeat: bool,
) -> bool {
    let set_ok = |set: u8| {
        let wall_only = crate::eval::target_domain::modes_wall_only(modes, set);
        chosen_modes(modes, set).all(|(_, mode)| {
            requirements3_are_reachable_for(
                mode.targets,
                mode.second_targets,
                None,
                state,
                player,
                card,
                Some(printed),
                Some(wall_only),
            )
        })
    };
    if can_repeat {
        repeated_mode_sets(modes, choose)
            .into_iter()
            .any(|packed| set_ok(repeated_modes_distinct_mask(packed)))
    } else {
        mode_sets(modes, choose).any(set_ok)
    }
}

/// Whether `card` has at least one currently open castable face whose target
/// requirements can be satisfied (CR 601.2c, CR 709.3a, CR 715.3a).
///
/// Single-face cards (and cards on an Adventure in exile, where only face 0 may
/// be cast) check face 0 directly; split, Adventure, MDFC, disturb, and
/// aftermath cards check each face open in the card's current zone and timing
/// window so an unpointable front half never hides a legal second half.
#[must_use]
pub fn card_has_a_legal_target(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
) -> bool {
    let Some(raw) = state.object(card) else {
        return true;
    };
    let face_up = playable_exile_projection(state, player, raw);
    let obj: &crate::object::GameObject = &face_up;
    let Some(def) = obj.card.and_then(|c| lookup.card(c.index)) else {
        return true;
    };
    let in_hand = obj.zone == Zone::Hand && obj.zone_owner == Some(player);
    let in_own_graveyard = obj.zone == Zone::Graveyard && obj.zone_owner == Some(player);
    let on_adventure = adventure_only_permission(state, player, card);
    let disturb_ok =
        !in_hand && in_own_graveyard && def.faces.iter().any(|f| f.disturb || f.aftermath);
    if castable_back_faces_in_zone(def, on_adventure, disturb_ok)
        .next()
        .is_none()
    {
        return face_has_a_legal_target(state, lookup, player, card, 0);
    }
    let projected = split_front(state, obj);
    let front_obj = projected.as_ref().unwrap_or(obj);
    let fc = front_obj.characteristics();
    let front_open = !cast_is_forbidden(state, player, front_obj)
        && !fc.types.contains(TypeSet::LAND)
        && (graveyard_face_open(state, player, card, 0)
            || alt_cast_face_open(state, lookup, player, card, 0))
        && permitted_spell_types(state, player, card, fc.types, 0)
        && (object_timing_allows(state, player, front_obj)
            || alt_cast_face_with_flash(state, lookup, player, card, 0)
            || (in_sneak_window(state, player)
                && sneak_cost_for(state, player, card, Some(def), 0).is_some()))
        && spell_condition_allows(state, player, card, def, 0);
    if front_open && face_has_a_legal_target(state, lookup, player, card, 0) {
        return true;
    }
    castable_back_faces_in_zone(def, on_adventure, disturb_ok).any(|(i, _)| {
        (face_timing_allows(state, player, card, def, i)
            || alt_cast_face_with_flash(state, lookup, player, card, i))
            && !face_is_forbidden(state, player, obj, def, i)
            && (graveyard_face_open(state, player, card, i)
                || alt_cast_face_open(state, lookup, player, card, i))
            && face_has_a_legal_target(state, lookup, player, card, i)
    })
}

/// Whether a target spec's filter reads the first target (`Filter::Reference`
/// and its family): a second instance of "target" chosen relative to the
/// first (Gauntlets of Chaos, Glyph of Delusion).
pub(crate) fn spec_reads_reference(spec: baylee_cards_dsl::TargetSpec) -> bool {
    use baylee_cards_dsl::TargetSpec as S;
    match spec {
        S::Object(f)
        | S::ObjectOfEachOpponent(f)
        | S::OpponentOrObject(f)
        | S::ObjectOfFirstTargetsPlayer(f)
        | S::ObjectOfEventPlayer(f)
        | S::Spell(f)
        | S::StackOrBattlefield(f)
        | S::PlayerOrObject(f) => crate::state::filter_reads_reference(f),
        _ => false,
    }
}

/// The second instance of "target" can be satisfied (CR 601.2c). One whose
/// requirement names the first target is satisfied only together with a
/// first target it accepts: some legal first target must leave enough second
/// ones, read with that first target as the reference.
fn second_is_reachable(
    first: Option<baylee_cards_dsl::TargetReq>,
    second: Option<baylee_cards_dsl::TargetReq>,
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: Option<(&baylee_cards_dsl::CardDef, usize)>,
    wall_only: bool,
) -> bool {
    let (Some(first), Some(second_req)) = (first, second) else {
        return requirement_is_reachable_for(second, state, player, card, face, wall_only);
    };
    if second_req.min == 0 || !spec_reads_reference(second_req.spec) {
        return requirement_is_reachable_for(second, state, player, card, face, wall_only);
    }
    crate::eval::spell_target_options_for(&first.spec, state, player, card, face, None, wall_only)
        .into_iter()
        .any(|reference| {
            crate::eval::spell_target_options_relative(
                &second_req.spec,
                state,
                player,
                card,
                face,
                None,
                wall_only,
                Some(reference),
            )
            .len()
                >= usize::from(second_req.min)
        })
}

/// Whether the ordinary spell target requirement can be satisfied.
pub(crate) fn ordinary_targets_reachable(
    state: &GameState,
    def: &baylee_cards_dsl::CardDef,
    player: PlayerId,
    card: ObjectId,
) -> bool {
    def.abilities.iter().all(|ability| match ability {
        baylee_cards_dsl::AbilityDef::Spell {
            targets,
            second_targets,
            third_targets,
            ..
        } => requirements3_are_reachable_for(
            *targets,
            *second_targets,
            *third_targets,
            state,
            player,
            card,
            Some((def, 0)),
            None,
        ),
        _ => true,
    })
}

/// Complete printed mana price of a kicked spell, before increases/reductions:
/// the mana cost plus every additional cost's mana (CR 601.2f), with one
/// generic symbol as `ManaCost::combine` makes it.
#[must_use]
pub fn kicked_mana_cost(face: &baylee_cards_dsl::FaceDef) -> ManaCost {
    face.additional_costs
        .iter()
        .fold(face.mana_cost, |cost, extra| cost.combine(&extra.mana))
}

/// Combined printed mana cost of both halves of a split card cast fused
/// (`CR 702.102c`).
#[must_use]
pub fn fused_mana_cost(def: &baylee_cards_dsl::CardDef) -> ManaCost {
    def.faces
        .first()
        .map_or(ManaCost::ZERO, |left| left.mana_cost)
        .combine(
            &def.faces
                .get(1)
                .map_or(ManaCost::ZERO, |right| right.mana_cost),
        )
}

/// Whether a card prints a way of being cast at a price of its own, which
/// `LegalActions::cast_ways` lists: entwine, a mode with its own cost
/// (overload, spree), a kicker that changes the targets, an optional or
/// alternative cost, dash, mutate, a prototype, or a back face.
#[must_use]
pub fn has_priced_ways(def: &baylee_cards_dsl::CardDef) -> bool {
    let Some(face) = def.faces.first() else {
        return false;
    };
    face.entwine.is_some()
        || face.kicked_targets.is_some()
        || face.dash.is_some()
        || face.blitz.is_some()
        || face.warp.is_some()
        || face.emerge.is_some()
        || face.mutate.is_some()
        || face.bestow.is_some()
        || face.impending.is_some()
        || face.has_face_down_cast()
        || face.prototype.is_some()
        || face.optional_cost.is_some()
        || face.alternative_costs.iter().any(|alt| {
            alt.cost.mana != ManaCost::ZERO
                || matches!(alt.condition, baylee_cards_dsl::AltCondition::If(_))
        })
        || castable_back_faces(def, false).next().is_some()
        || def.abilities_for_face(0).iter().any(|ability| {
            matches!(
                ability,
                baylee_cards_dsl::AbilityDef::Static(sa)
                    if matches!(sa.modifier, baylee_cards_dsl::Modifier::Sneak(_))
            ) || ability.modal_spell().is_some_and(|(modes, _, _)| {
                modes
                    .iter()
                    .any(|mode| mode.cost_override.is_some() || mode.additional_cost.is_some())
            })
        })
}

/// Whether one mode of a modal spell (CR 700.2) can be pointed at anything.
///
/// The half of [`face_has_a_legal_target`] a *mode* needs, and the reason it
/// is separate: the card being castable and this particular mode being
/// takeable are two questions, and the wizard has to ask the second one about
/// every button it offers. It did not, which was invisible for as long as a
/// modal spell was also offered a mode-less `Normal` cast that resolved to
/// nothing — Cyclonic Rift on an empty board offered "return target nonland
/// permanent" and refused it with "not enough legal targets" the moment it
/// was pressed.
#[must_use]
pub fn mode_has_a_legal_target(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    mode: usize,
) -> bool {
    let Some(def) = state
        .object(card)
        .and_then(|o| o.card)
        .and_then(|c| lookup.card(c.index))
    else {
        return true;
    };
    let reqs = def.abilities.iter().find_map(|a| {
        let (modes, ..) = a.modal_spell()?;
        modes
            .get(mode)
            .map(|m| (m.effects, m.targets, m.second_targets))
    });
    match reqs {
        Some((effects, first, second)) => {
            if let Some(r) = first
                && r.min > 0
                && def
                    .faces
                    .first()
                    .is_some_and(|f| !f.mana_cost.has_variable())
                && let Some((_, total)) = crate::engine::division::effects_divided_spec(
                    effects,
                    state,
                    player,
                    crate::eval::live_context(state, card),
                    state.object(card).map(|o| o.version),
                    0,
                )
                && total < u32::from(r.min)
            {
                return false;
            }
            requirements_are_reachable(first, second, state, player, card)
        }
        None => true,
    }
}

/// Every mode of `modes` as a bitmask, for entwine (CR 702.42a).
#[must_use]
pub fn entwine_set(modes: &[baylee_cards_dsl::SpellMode]) -> u8 {
    let n = modes.len().min(8);
    if n == 0 {
        0
    } else {
        u8::try_from((1_u16 << n) - 1).unwrap_or(u8::MAX)
    }
}

/// Every set of modes a spell that chooses more than one may be cast with
/// (CR 700.2a), as a bitmask — bit `i` is mode `i` — in increasing order:
/// each holds as many modes as `choose` allows, none twice (CR 700.2d).
///
/// Eight modes at most, which is a byte. The pool's widest is four.
pub fn mode_sets(
    modes: &[baylee_cards_dsl::SpellMode],
    choose: baylee_cards_dsl::ModeCount,
) -> impl Iterator<Item = u8> {
    let every = 1_u16 << modes.len().min(8);
    (1..every).filter_map(move |set| {
        let set = u8::try_from(set).ok()?;
        let count = u8::try_from(set.count_ones()).ok()?;
        (choose.min <= count && count <= choose.max).then_some(set)
    })
}

/// The modes a set names, with their indices, in the order they are
/// printed — which is the order they are carried out in (CR 608.2c).
pub fn chosen_modes(
    modes: &'static [baylee_cards_dsl::SpellMode],
    set: u8,
) -> impl Iterator<Item = (usize, &'static baylee_cards_dsl::SpellMode)> {
    modes
        .iter()
        .enumerate()
        .filter(move |(i, _)| *i < 8 && set & (1 << i) != 0)
}

/// What a set of modes costs: `base`, the spell's own price, with every
/// chosen mode's cost added to it (CR 700.2h, 601.2f) — spree's "+ {1}".
///
/// With one generic symbol, as a printed cost has and as
/// [`ManaCost::combine`] keeps it: two "+ {1}" on {W} are {2}{W}, which is
/// what a row draws.
#[must_use]
pub fn mode_set_cost(
    base: ManaCost,
    modes: &'static [baylee_cards_dsl::SpellMode],
    set: u8,
) -> ManaCost {
    chosen_modes(modes, set).fold(base, |cost, (_, mode)| {
        mode.additional_cost
            .map_or(cost, |extra| cost.combine(&extra))
    })
}

/// Multiplicity (`0..=15`) of mode `mode_index` (`< 8`) inside a packed
/// `CastModeKind::RepeatedModes(packed)` value (`CR 700.2d`).
#[must_use]
pub const fn mode_multiplicity(packed: u32, mode_index: usize) -> u8 {
    if mode_index < 8 {
        ((packed >> (4 * mode_index)) & 0xF) as u8
    } else {
        0
    }
}

/// Bitmask of distinct modes with nonzero multiplicity in `packed`.
#[must_use]
pub const fn repeated_modes_distinct_mask(packed: u32) -> u8 {
    let mut mask = 0_u8;
    let mut i = 0;
    while i < 8 {
        if ((packed >> (4 * i)) & 0xF) != 0 {
            mask |= 1 << i;
        }
        i += 1;
    }
    mask
}

/// Every multiset of modes for a modal spell with `can_repeat_modes: true`
/// (`CR 700.2d`), packed 4 bits per mode `0..8`, in deterministic order.
#[must_use]
pub fn repeated_mode_sets(
    modes: &[baylee_cards_dsl::SpellMode],
    choose: baylee_cards_dsl::ModeCount,
) -> Vec<u32> {
    fn enumerate(
        mode_idx: usize,
        num_modes: usize,
        remaining: u8,
        packed: u32,
        out: &mut Vec<u32>,
    ) {
        if remaining == 0 {
            out.push(packed);
            return;
        }
        if mode_idx >= num_modes {
            return;
        }
        if mode_idx + 1 == num_modes {
            let add = u32::from(remaining) << (4 * mode_idx);
            out.push(packed | add);
            return;
        }
        for take in (0..=remaining).rev() {
            let add = u32::from(take) << (4 * mode_idx);
            enumerate(mode_idx + 1, num_modes, remaining - take, packed | add, out);
        }
    }
    let n = modes.len().min(8);
    let max = choose.max.min(8);
    if n == 0 || choose.min > max {
        return Vec::new();
    }
    let mut out = Vec::new();
    for total in choose.min..=max {
        enumerate(0, n, total, 0, &mut out);
    }
    out
}

/// Chosen mode occurrences for `packed` in printed order (`CR 700.2d`, `CR 608.2c`).
pub fn chosen_repeated_modes(
    modes: &'static [baylee_cards_dsl::SpellMode],
    packed: u32,
) -> impl Iterator<Item = (usize, &'static baylee_cards_dsl::SpellMode)> {
    modes.iter().enumerate().take(8).flat_map(move |(i, mode)| {
        std::iter::repeat_n((i, mode), usize::from(mode_multiplicity(packed, i)))
    })
}

/// Mana cost of `base` plus each chosen mode occurrence's `additional_cost` (`CR 700.2h`).
#[must_use]
pub fn repeated_mode_set_cost(
    base: ManaCost,
    modes: &'static [baylee_cards_dsl::SpellMode],
    packed: u32,
) -> ManaCost {
    chosen_repeated_modes(modes, packed).fold(base, |cost, (_, mode)| {
        mode.additional_cost
            .map_or(cost, |extra| cost.combine(&extra))
    })
}

/// Every target clause across the chosen modes of `abilities` in printed order
/// (`CR 700.2c`, `CR 608.2c`): `(mode_index, is_second_in_mode, req)`.
pub(crate) fn modal_target_clauses(
    abilities: &'static [baylee_cards_dsl::AbilityDef],
    set: u8,
) -> SmallVec<[(usize, bool, baylee_cards_dsl::TargetReq); 4]> {
    let mut out = SmallVec::new();
    for ability in abilities {
        if let Some((modes, ..)) = ability.modal_spell() {
            for (mode_index, mode) in chosen_modes(modes, set) {
                if let Some(first) = mode.targets {
                    out.push((mode_index, false, first));
                }
                if let Some(second) = mode.second_targets {
                    out.push((mode_index, true, second));
                }
            }
            break;
        }
    }
    out
}

/// Every target clause across the repeated mode occurrences of `abilities` in
/// printed order (`CR 700.2c`, `CR 700.2d`, `CR 608.2c`).
pub(crate) fn repeated_modal_target_clauses(
    abilities: &'static [baylee_cards_dsl::AbilityDef],
    packed: u32,
) -> SmallVec<[(usize, bool, baylee_cards_dsl::TargetReq); 4]> {
    let mut out = SmallVec::new();
    for ability in abilities {
        if let Some((modes, ..)) = ability.modal_spell() {
            for (mode_index, mode) in chosen_repeated_modes(modes, packed) {
                if let Some(first) = mode.targets {
                    out.push((mode_index, false, first));
                }
                if let Some(second) = mode.second_targets {
                    out.push((mode_index, true, second));
                }
            }
            break;
        }
    }
    out
}

/// Whether every mode in `packed` has legal targets (`CR 700.2a`, `CR 700.2d`).
#[must_use]
pub fn repeated_mode_set_has_legal_targets(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    packed: u32,
) -> bool {
    mode_set_has_legal_targets(
        state,
        lookup,
        player,
        card,
        repeated_modes_distinct_mask(packed),
    )
}

/// Whether every mode in a set can be pointed at something: a mode that
/// would be illegal can't be chosen (CR 700.2a), in a set as alone.
#[must_use]
pub fn mode_set_has_legal_targets(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    set: u8,
) -> bool {
    let Some(def) = state
        .object(card)
        .and_then(|object| object.card)
        .and_then(|card| lookup.card(card.index))
    else {
        return true;
    };
    def.abilities
        .iter()
        .filter_map(|ability| ability.modal_spell().map(|(modes, ..)| modes))
        .all(|modes| {
            let wall_only = crate::eval::target_domain::modes_wall_only(modes, set);
            chosen_modes(modes, set).all(|(_, mode)| {
                requirement_is_reachable_for(mode.targets, state, player, card, None, wall_only)
                    && second_is_reachable(
                        mode.targets,
                        mode.second_targets,
                        state,
                        player,
                        card,
                        None,
                        wall_only,
                    )
            })
        })
}

/// Whether choosing a mode is the **only** way to cast this face (CR 700.2).
///
/// The cast wizard's own guard, lifted out of it so the offer can ask the
/// same question with the same words. It refuses a mode-less `Normal` option
/// for a card whose every effect sits under a mode — such a spell would go
/// hand → stack → graveyard and do nothing — and the offer has to know that,
/// because a card the wizard will only ever price *per mode* is castable
/// exactly when one of its modes is.
///
/// The three clauses hold it to that sentence and no further: a permanent
/// spell arrives on the battlefield whether a mode was picked or not, and a
/// plain [`baylee_cards_dsl::AbilityDef::Spell`] printed beside the modes is
/// what resolution finds first.
#[must_use]
pub fn modes_are_the_only_way(def: &baylee_cards_dsl::CardDef, face: usize) -> bool {
    let Some(f) = def.faces.get(face) else {
        return false;
    };
    let abilities = def.abilities_for_face(face);
    abilities.iter().any(|a| a.modal_spell().is_some())
        && !abilities
            .iter()
            .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::Spell { .. }))
        && !f.types.is_permanent()
}

fn requirements_are_reachable(
    first: Option<baylee_cards_dsl::TargetReq>,
    second: Option<baylee_cards_dsl::TargetReq>,
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> bool {
    requirements3_are_reachable_for(first, second, None, state, player, card, None, None)
}

pub(crate) fn spell_clause_distinct_pool(
    req: baylee_cards_dsl::TargetReq,
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: Option<(&baylee_cards_dsl::CardDef, usize)>,
    wall_only: bool,
) -> crate::eval::DistinctGroupPool {
    let distinct_from_prior =
        req.constraint() == Some(baylee_cards_dsl::TargetConstraint::DistinctFromPriorTargetGroups);
    if req.min == 0
        || req.count_is_x
        || matches!(
            req.spec,
            baylee_cards_dsl::TargetSpec::Player(rel) if rel != baylee_cards_dsl::PlayerRel::You
        )
    {
        return crate::eval::DistinctGroupPool {
            needed: 0,
            distinct_from_prior,
            objects: Vec::new(),
            players: Vec::new(),
        };
    }
    let (objects, players) = if matches!(req.spec, baylee_cards_dsl::TargetSpec::ThisObject) {
        (vec![card], Vec::new())
    } else {
        (
            crate::eval::spell_target_options_for(
                &req.spec, state, player, card, face, None, wall_only,
            ),
            crate::eval::target_player_options_from(
                state,
                &req.spec,
                player,
                crate::eval::live_context(state, card),
            ),
        )
    };
    crate::eval::DistinctGroupPool {
        needed: usize::from(req.min),
        distinct_from_prior,
        objects,
        players,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn requirements3_are_reachable_for(
    first: Option<baylee_cards_dsl::TargetReq>,
    second: Option<baylee_cards_dsl::TargetReq>,
    third: Option<baylee_cards_dsl::TargetReq>,
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: Option<(&baylee_cards_dsl::CardDef, usize)>,
    selected_domain: Option<bool>,
) -> bool {
    let wall_only =
        selected_domain.unwrap_or_else(|| crate::eval::target_domain::walls3(first, second, third));
    if !requirement_is_reachable_for(first, state, player, card, face, wall_only)
        || !second_is_reachable(first, second, state, player, card, face, wall_only)
        || !requirement_is_reachable_for(third, state, player, card, face, wall_only)
    {
        return false;
    }
    if [second, third].into_iter().flatten().any(|r| {
        r.min > 0
            && !r.count_is_x
            && r.constraint()
                == Some(baylee_cards_dsl::TargetConstraint::DistinctFromPriorTargetGroups)
    }) {
        let pools: Vec<_> = [first, second, third]
            .into_iter()
            .flatten()
            .map(|r| spell_clause_distinct_pool(r, state, player, card, face, wall_only))
            .collect();
        if !crate::eval::can_complete_target_groups(&[], baylee_core::ids::SeatSet::new(), &pools) {
            return false;
        }
    }
    true
}

/// Target requirement for casting a spell bestowed (CR 702.103b): "enchant creature".
pub const BESTOW_TARGET_REQ: baylee_cards_dsl::TargetReq = baylee_cards_dsl::TargetReq::one(
    baylee_cards_dsl::TargetSpec::Object(&baylee_cards_dsl::Filter::CREATURE),
);

/// Target requirement for casting a creature spell for its mutate cost (CR 702.140a):
/// "target non-Human creature with the same owner as this spell".
pub const MUTATE_TARGET_REQ: baylee_cards_dsl::TargetReq = baylee_cards_dsl::TargetReq::one(
    baylee_cards_dsl::TargetSpec::Object(&baylee_cards_dsl::Filter::And(&[
        baylee_cards_dsl::Filter::CREATURE,
        baylee_cards_dsl::Filter::OwnedBySourceOwner,
        baylee_cards_dsl::Filter::Not(&baylee_cards_dsl::Filter::HasSubtype(
            baylee_core::generated::subtypes::creature::HUMAN,
        )),
    ])),
);

/// Kicker replaces the first targeting group, retaining the second and third groups.
pub(crate) fn kicked_targets_reachable(
    state: &GameState,
    def: &baylee_cards_dsl::CardDef,
    player: PlayerId,
    card: ObjectId,
    first: baylee_cards_dsl::TargetReq,
) -> bool {
    let (second, third) = def
        .abilities
        .iter()
        .find_map(|ability| match ability {
            baylee_cards_dsl::AbilityDef::Spell {
                second_targets,
                third_targets,
                ..
            } => Some((*second_targets, *third_targets)),
            _ => None,
        })
        .unwrap_or_default();
    requirements3_are_reachable_for(
        Some(first),
        second,
        third,
        state,
        player,
        card,
        Some((def, 0)),
        None,
    )
}

pub(crate) fn requirement_is_reachable_for(
    req: Option<baylee_cards_dsl::TargetReq>,
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: Option<(&baylee_cards_dsl::CardDef, usize)>,
    wall_only: bool,
) -> bool {
    let Some(req) = req else {
        return true;
    };
    if req.min == 0
        || req.count_is_x
        || matches!(req.spec, baylee_cards_dsl::TargetSpec::ThisObject)
        || matches!(
            req.spec,
            baylee_cards_dsl::TargetSpec::Player(rel) if rel != baylee_cards_dsl::PlayerRel::You
        )
    {
        return true;
    }
    // **Both** readers, because the question a target menu asks is one
    // question over two lists. `Pending::ChooseTargets` carries `options` and
    // `player_options` and a target is picked out of their union (CR 115.4),
    // so counting objects alone made an offer and a refusal disagree:
    // `TargetSpec::AnyTarget` used to be short-circuited to "reachable" by
    // name beside `AnyPlayer` and `AnyOpponent`, which hid it — and the day
    // it stopped being on that list, Lightning Bolt was uncastable at a board
    // with no creature on it, with the opponent sitting right there and on
    // the menu the wizard would have printed.
    //
    // Derived rather than listed, for the same reason: a new `TargetSpec` is
    // counted by whichever of the two readers knows about it, and a spec
    // neither knows is unreachable, which is the honest answer.
    let object_list: Vec<ObjectId> = match x_bounded(req.spec) {
        // "Target spell with mana value X" before any X is announced: a
        // target for some X (`eval::matches_for_some_x`). Enumerated through
        // the same reader with the filter widened, so what makes a spell or
        // permanent targetable at all is still asked the one way.
        Some((widened, filter)) => crate::eval::spell_target_options_for(
            &widened, state, player, card, face, None, wall_only,
        )
        .into_iter()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|o| crate::eval::matches_for_some_x(filter, state, o, player, card))
        })
        .collect(),
        None => crate::eval::spell_target_options_for(
            &req.spec, state, player, card, face, None, wall_only,
        ),
    };
    if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController) {
        return !crate::eval::eligible_same_controller_groups(
            state,
            &object_list,
            u32::from(req.min),
        )
        .is_empty();
    }
    if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
        return crate::eval::one_per_player_satisfied(
            state,
            &req.spec,
            player,
            &object_list,
            req.min,
        );
    }
    let objects = object_list.len();
    let players = crate::eval::target_player_options_from(
        state,
        &req.spec,
        player,
        crate::eval::live_context(state, card),
    )
    .len();
    objects + players >= req.min as usize
}

/// Whether `player` may begin casting a spell with these characteristics
/// right now.
///
/// CR 117.1a is the permission: an instant any time its controller has
/// priority, a noninstant during their own main phase with the stack empty
/// (CR 307.1 says the same of a sorcery in particular). Flash (CR 702.8a)
/// moves a card into the first group whatever its types say, Teferi's +1
/// does the same for that player's sorceries, and Teferi's static pulls
/// every opponent's spell back into the second.
///
/// Characteristics rather than an object, because two callers ask this
/// about two different things. [`can_cast`] passes the *projected*
/// characteristics of a card that is sitting in a zone, so a continuous
/// effect that granted it flash is read. The prepared cast passes the
/// **printed** face of a card that has no object at all — the copy does not
/// exist until it is cast, so nothing could have granted it anything — and
/// still wants the two player-scoped effects above, which is the whole
/// reason this is one function and not two.
fn teferi_sorcery_lock(state: &GameState, player: PlayerId) -> bool {
    state.effects.may_change_cast_timing_or_lock()
        && state.effects.iter().any(|fx| {
            matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::OpponentsCastAsSorcery
            ) && crate::effects::source_is_present(state, fx)
                && state.is_opponent(fx.controller, player)
        })
}

#[doc(hidden)]
pub fn timing_allows(
    state: &GameState,
    player: PlayerId,
    types: TypeSet,
    keywords: baylee_cards_dsl::KeywordSet,
) -> bool {
    // Teferi's restriction forces sorcery-speed timing on everything for
    // opponents.
    let teferi_lock = teferi_sorcery_lock(state, player);
    // Flash (CR 702.8a) makes a card castable whenever an instant could be
    // — Snapcaster Mage, Restoration Angel, the whole free-spell cycle.
    let is_instant =
        types.contains(TypeSet::INSTANT) || keywords.contains(baylee_cards_dsl::KeywordSet::FLASH);
    let granted_flash = !is_instant
        && state.effects.may_change_cast_timing_or_lock()
        && state.effects.iter().any(|fx| {
            fx.controller == player
                && crate::effects::source_is_present(state, fx)
                && match fx.modifier {
                    baylee_cards_dsl::Modifier::SorceriesHaveFlash => {
                        types.contains(TypeSet::SORCERY)
                    }
                    baylee_cards_dsl::Modifier::SpellsHaveFlash(allowed) => {
                        allowed.is_none_or(|mask| types.intersects(mask))
                    }
                    _ => false,
                }
        });
    if teferi_lock || (!is_instant && !granted_flash) {
        let main_phase = matches!(state.turn.phase, Phase::FirstMain | Phase::SecondMain);
        return main_phase && state.turn.active == player && state.zones.stack_is_empty();
    }
    true
}

/// Object-aware cast timing check (`CR 601.3`), extending [`timing_allows`] with
/// [`baylee_cards_dsl::Modifier::GrantFlash`] against the prospective spell.
pub(crate) fn object_timing_allows(
    state: &GameState,
    player: PlayerId,
    obj: &crate::object::GameObject,
) -> bool {
    let c = obj.characteristics();
    if timing_allows(state, player, c.types, c.keywords) {
        return true;
    }
    if teferi_sorcery_lock(state, player) || !state.effects.may_change_cast_timing_or_lock() {
        return false;
    }
    let mut prospective: Option<crate::object::GameObject> = None;
    state.effects.iter().any(|fx| {
        let baylee_cards_dsl::Modifier::GrantFlash { who, filter } = fx.modifier else {
            return false;
        };
        if !crate::effects::source_is_present(state, fx)
            || !crate::eval::players_for_source(who, state, fx.controller, fx.source)
                .is_some_and(|ps| ps.contains(&player))
        {
            return false;
        }
        let spell = prospective.get_or_insert_with(|| projected_spell(state, player, obj));
        crate::effects::matches_filter(state, fx, spell)
            && crate::eval::matches(
                filter,
                state,
                spell,
                fx.controller,
                fx.source.unwrap_or(spell.id),
            )
    })
}

/// Whether `player` may cast back face `face` of `def` now, as far as timing
/// goes (CR 601.3).
///
/// A face that is cast in place of the front is judged by its own
/// characteristics (CR 601.3e): only the face that will be up on the stack
/// is evaluated for an MDFC (CR 712.8c), and only the alternative
/// characteristics for an Adventure (CR 716.3a). So an instant printed on
/// the back of a creature or an enchantment is cast whenever an instant
/// could be, and the front is not.
pub(crate) fn face_timing_allows(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
) -> bool {
    def.faces.get(face).is_some_and(|f| {
        let face_timing =
            timing_allows(state, player, f.types, def.keywords_for_face(face))
                || state.object(card).is_some_and(|raw| {
                    let mut face_obj = raw.clone();
                    if face != 0 || def.faces.len() > 1 {
                        face_obj.base = std::sync::Arc::new(
                            crate::object::Characteristics::from_face(def, face, raw.base.name),
                        );
                        face_obj.cache.clear();
                        face_obj.face_index = u8::try_from(face).unwrap_or(0);
                    }
                    object_timing_allows(state, player, &face_obj)
                });
        face_timing && permitted_spell_types(state, player, card, f.types, face)
    }) && spell_condition_allows(state, player, card, def, face)
}

/// "Cast this spell only [when]" (CR 506.8; Berserk's "only before the
/// combat damage step"): whether the condition the face's spell prints, if
/// it prints one, holds now for `player` casting `card`. A restriction of
/// the card's own, asked beside the timing its type gives it (CR 601.3):
/// an instant restricted to combat is still cast whenever an instant could
/// be, inside that window.
pub(crate) fn spell_condition_allows(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
) -> bool {
    def.abilities_for_face(face).iter().all(|a| match a {
        baylee_cards_dsl::AbilityDef::Spell {
            condition: Some(condition),
            ..
        } => crate::eval::condition_holds(state, player, card, *condition),
        _ => true,
    })
}

/// Whether a continuous effect forbids `player` casting `obj` at all
/// (CR 601.3a).
///
/// `Modifier::OpponentsCantCast` is player-scoped like Teferi's lock above,
/// and unlike it names *which* spells: Silence forbids every one, and
/// Ranger-Captain of Eos only the noncreature ones. The filter is evaluated
/// from the **effect's** controller, because "noncreature spells" is that
/// player's sentence about somebody else's card — and `this` is the effect's
/// own source, so a filter naming `Filter::This` means the forbidding
/// permanent rather than the card being cast.
pub(crate) fn cast_is_forbidden(
    state: &GameState,
    player: PlayerId,
    obj: &crate::object::GameObject,
) -> bool {
    if obj.zone == Zone::Hand
        && obj
            .riders
            .contains(&crate::object::Rider::RevealedInHandCantPlayUntilTurn)
    {
        return true;
    }
    state.effects.may_change_cast_timing_or_lock()
        && state.effects.iter().any(|fx| match fx.modifier {
            baylee_cards_dsl::Modifier::SpellCastLimitPerTurn { who, limit } => {
                player_locks::affects_player(state, fx, who, player)
                    && state.per_turn.spells_cast_by(player) >= limit
            }
            baylee_cards_dsl::Modifier::MatchingSpellCastLimitPerTurn { who, filter, limit } => {
                player_locks::matching_limit_reached(state, fx, who, filter, limit, player, obj)
            }
            baylee_cards_dsl::Modifier::PlayersCantCast { who, filter }
            | baylee_cards_dsl::Modifier::PlayerCantCast { who, filter } => {
                player_locks::prevents(state, fx, who, filter, player, obj)
            }
            baylee_cards_dsl::Modifier::OpponentsCantCast(filter) => {
                state.is_opponent(fx.controller, player)
                    && crate::eval::matches(
                        filter,
                        state,
                        obj,
                        fx.controller,
                        fx.source.unwrap_or(obj.id),
                    )
            }
            baylee_cards_dsl::Modifier::PlayersCantPlay(filter) => crate::eval::matches(
                filter,
                state,
                obj,
                fx.controller,
                fx.source.unwrap_or(obj.id),
            ),
            _ => false,
        })
}

/// `obj` as the spell casting it the normal way would make, asked while it
/// is still a card: off the stack a split card is both of its halves (CR
/// 709.4), and its normal cast is the left half alone (CR 709.3a), so a
/// price, a timing or a lock read off the card would be read off both.
/// `None` for every other object, which is cast as it is.
pub(crate) fn split_front(
    state: &GameState,
    obj: &crate::object::GameObject,
) -> Option<crate::object::GameObject> {
    use crate::state::CardLookup;
    if obj.zone == Zone::Stack {
        return None;
    }
    let def = obj
        .card
        .and_then(|card| state.bases.card(card.index))
        .filter(|def| crate::object::reads_both_halves(def))?;
    Some(spell_as_face(state, obj, def, 0))
}

/// `obj` as the spell it would be cast as `face` of `def`: that face's
/// characteristics and no other's, its own name among them (CR 709.3a,
/// 715.3a).
fn spell_as_face(
    state: &GameState,
    obj: &crate::object::GameObject,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
) -> crate::object::GameObject {
    let mut spell = obj.clone();
    spell.base = std::sync::Arc::new(crate::object::Characteristics::from_face(
        def,
        face,
        state.names.printed(def.faces[face].name),
    ));
    spell.cache.clear();
    spell.face_index = u8::try_from(face).expect("bounded faces");
    spell
}

/// Whether a continuous effect forbids `player` casting `obj` as `face` of
/// `def`, which is [`cast_is_forbidden`] asked of the spell that cast would
/// make: only the half or the Adventure being cast is evaluated (CR 709.3a,
/// 715.3a), so a lock on one face says nothing of another.
pub(crate) fn face_is_forbidden(
    state: &GameState,
    player: PlayerId,
    obj: &crate::object::GameObject,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
) -> bool {
    // A split card off the stack is both halves at face 0 (CR 709.4), so
    // its left half too is asked as the spell it makes.
    let as_it_is = usize::from(obj.face_index) == face
        && (obj.zone == Zone::Stack || !crate::object::reads_both_halves(def));
    if as_it_is || !state.effects.may_change_cast_timing_or_lock() {
        return cast_is_forbidden(state, player, obj);
    }
    cast_is_forbidden(state, player, &spell_as_face(state, obj, def, face))
}

/// Whether a continuous effect grants `card` flashback (CR 702.34) right now.
///
/// One reader, because a grant arrives in either of two shapes and a caller
/// that knew only one of them offered nothing. `Effect::GrantFlashback`
/// names its target and registers `EffectFilter::ObjectIs`; a resolved
/// graveyard-wide grant binds the cards present when it resolves (CR 611.2c).
/// Static grants may remain dynamic. Both use the shared effect matcher.
#[must_use]
pub fn flashback_granted(state: &GameState, card: ObjectId) -> bool {
    if !state.effects.may_grant_graveyard_or_top_play() {
        return false;
    }
    let Some(obj) = state.object(card) else {
        return false;
    };
    state.effects.iter().any(|fx| {
        matches!(fx.modifier, baylee_cards_dsl::Modifier::GrantsFlashback)
            && crate::effects::applies_to(state, fx, obj)
    })
}

/// Whether `card` has jump-start granted by an active continuous effect (`CR 702.133a`).
#[must_use]
pub fn jump_start_granted(state: &GameState, card: ObjectId) -> bool {
    if !state.effects.may_grant_graveyard_or_top_play() {
        return false;
    }
    let Some(obj) = state.object(card) else {
        return false;
    };
    state.effects.iter().any(|fx| {
        matches!(fx.modifier, baylee_cards_dsl::Modifier::GrantsJumpStart)
            && crate::effects::applies_to(state, fx, obj)
    })
}

/// Strict main-phase empty-stack window for `player` (`CR 116.2k`,
/// `CR 702.170a`, `CR 702.170d`): active player, main phase, and an empty stack.
#[must_use]
pub(crate) fn sorcery_window(state: &GameState, player: PlayerId) -> bool {
    matches!(state.turn.phase, Phase::FirstMain | Phase::SecondMain)
        && state.turn.active == player
        && state.zones.stack_is_empty()
}

/// Whether this exact exiled incarnation was foretold by `player` on an
/// earlier turn. The printed card supplies the alternative cost separately.
pub(crate) fn foretell_permission(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    state.object(card).is_some_and(|object| {
        object.zone == Zone::Exile
            && object.zone_owner == Some(player)
            && object.riders.iter().any(|rider| {
                matches!(rider, crate::object::Rider::Foretold { turn } if *turn < state.turn.number)
            })
    })
}

/// Whether this exact face-up exiled card is plotted and `player` is its owner
/// on a turn strictly after the turn it became plotted (`CR 702.170d`).
#[must_use]
pub(crate) fn plot_permission(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    state.object(card).is_some_and(|object| {
        object.zone == Zone::Exile
            && object.owner == player
            && !object.status.contains(crate::object::Status::FACE_DOWN)
            && object.riders.contains(&crate::object::Rider::Plotted)
            && object
                .riders
                .iter()
                .find_map(|rider| match rider {
                    crate::object::Rider::PlottedOnTurn(turn) => Some(*turn),
                    _ => None,
                })
                .is_none_or(|turn| turn < state.turn.number)
    })
}

/// Whether this exact face-up exiled card was exiled by Warp's delayed
/// triggered ability on a turn strictly before the current turn and `player`
/// is its owner (`CR 702.185b`).
#[must_use]
pub fn warp_exile_permission(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    state.object(card).is_some_and(|object| {
        object.zone == Zone::Exile
            && object.owner == player
            && !object.status.contains(crate::object::Status::FACE_DOWN)
            && object.is_card()
            && object.riders.iter().any(|rider| {
                matches!(rider, crate::object::Rider::WarpedInExile(turn) if *turn < state.turn.number)
            })
    })
}

/// Whether `card` can be cast by `player` right now (printed cost or any
/// alternative/mode).
///
/// # Errors
/// [`CastError`] describing the first legality violation.
pub fn can_cast(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
) -> Result<(), CastError> {
    can_cast_paying(state, lookup, player, card, Paying::FromThePool)
}

/// Whether `card` could be cast by `player` right now if its mana is made
/// while it is being cast (CR 601.2g): every rule [`can_cast`] asks, except
/// that no mana cost is asked to be in the pool already.
///
/// What it does not know is whether the seat's mana abilities can make that
/// mana, which is a question about colours and amounts a player answers by
/// tapping; a cast that cannot be paid is reversed in its payment window
/// (CR 601.2h, 734.1).
///
/// # Errors
/// [`CastError`] describing the first legality violation.
pub fn can_cast_paying_later(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
) -> Result<(), CastError> {
    can_cast_paying(state, lookup, player, card, Paying::DuringTheCast)
}

/// Where a cast's mana is asked to come from: the pool as it stands, or
/// mana abilities activated during the cast (CR 601.2g).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Paying {
    /// The mana must already be floating.
    FromThePool,
    /// No mana cost is asked of the pool.
    DuringTheCast,
}

fn can_cast_paying(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    paying: Paying,
) -> Result<(), CastError> {
    let mut best = if library_top_life_open(state, player, card) {
        match library_top_life_options(state, lookup, player, card, paying) {
            Ok(_) => return Ok(()),
            Err(err) => Err(err),
        }
    } else {
        let res = can_cast_form_paying(state, lookup, player, card, None, paying);
        if res.is_ok() {
            return res;
        }
        res
    };
    if !library_top_life_open(state, player, card)
        && let Some(face) = state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| lookup.card(c.index))
            .and_then(|d| d.faces.first())
    {
        for form in [
            face.prototype.map(SpellForm::Prototype),
            face.has_face_down_cast().then_some(SpellForm::Disguise),
            face.bestow.map(SpellForm::Bestow),
            face.impending.map(|(_, c)| SpellForm::Impending(c)),
        ]
        .into_iter()
        .flatten()
        {
            match can_cast_form_paying(state, lookup, player, card, Some(form), paying) {
                Ok(()) => return Ok(()),
                Err(CastError::NotEnoughMana) => best = Err(CastError::NotEnoughMana),
                Err(_) => {}
            }
        }
    }
    let (alt_opts, alt_diag) = alt_cast_grant_options(state, lookup, player, card, paying);
    if !alt_opts.is_empty() {
        return Ok(());
    }
    if let Some(diag) = alt_diag
        && (diag == CastError::NotEnoughMana
            || matches!(best, Err(CastError::NotInHand | CastError::NoWayToCast)))
    {
        best = Err(diag);
    }
    if matches!(best, Err(CastError::BadTiming | CastError::NotEnoughMana)) {
        let flash_options = optional_flash_cost_options(state, lookup, player, card, paying);
        if flash_options.iter().any(|option| option.mana_ready) {
            return Ok(());
        }
        if !flash_options.is_empty() && matches!(best, Err(CastError::BadTiming)) {
            best = Err(CastError::NotEnoughMana);
        }
    }
    best
}

/// Whether `player` may begin to cast a spell at all right now (CR 601.3).
///
/// Two sentences say no to every spell there is, so they are asked here,
/// beside the table's other refusals, by every door a cast comes through: the
/// priority offer, a free cast an effect makes (cascade, rebound, suspend), a
/// miracle, madness, a prepared copy. Conduit of Worlds' "If you do, you can't
/// cast additional spells this turn" names the player, and split second names
/// them all ([`split_second_on_stack`]).
#[must_use]
pub(crate) fn may_begin_casting(state: &GameState, player: PlayerId) -> bool {
    !split_second_on_stack(state)
        && !state
            .per_turn
            .no_more_spells
            .get(player.get() as usize)
            .copied()
            .unwrap_or(false)
}

/// Split second (CR 702.61a): "As long as this spell is on the stack, players
/// can't cast other spells or activate abilities that aren't mana abilities."
///
/// Read off the projected keywords of every spell on the stack, so a copy of
/// a split-second spell locks it too: a copy is a spell.
pub(crate) fn split_second_on_stack(state: &GameState) -> bool {
    state.zones.list(ZoneLocation::Stack).iter().any(|id| {
        state.object(*id).is_some_and(|o| {
            o.kind == ObjectKind::Spell
                && o.characteristics()
                    .keywords
                    .contains(baylee_cards_dsl::KeywordSet::SPLIT_SECOND)
        })
    })
}

#[allow(clippy::too_many_lines)] // one gate per casting rule
pub(crate) fn can_cast_form_paying(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    form: Option<SpellForm>,
    paying: Paying,
) -> Result<(), CastError> {
    // Every mana probe below asks the pool, unless the mana is made during
    // the cast; the other parts of a cost are asked either way.
    let affordable = |state: &GameState,
                      player: PlayerId,
                      pool: &ManaPool,
                      cost: &ManaCost,
                      what: SpendFor,
                      allowance: SourceAllowance| {
        paying == Paying::DuringTheCast
            || source_affordable(state, player, what, pool, cost, &[], Some(allowance))
    };
    let obj = state.object(card).ok_or(CastError::NotInHand)?;
    if obj
        .riders
        .iter()
        .any(|r| matches!(r, crate::object::Rider::MergedInto(_)))
    {
        return Err(CastError::NotInHand);
    }
    let face_up = form
        .is_none()
        .then(|| playable_exile_projection(state, player, obj));
    let obj = face_up.as_deref().unwrap_or(obj);
    let in_hand = obj.zone == Zone::Hand && obj.zone_owner == Some(player);
    let in_own_graveyard = obj.zone == Zone::Graveyard && obj.zone_owner == Some(player);
    // Flashback (CR 702.34a): a card may be cast from its owner's graveyard
    // when a grant or its own printed flashback says so. The printed one
    // has its own price, probed below.
    let printed_flashback = obj
        .card
        .and_then(|c| lookup.card(c.index))
        .and_then(|def| def.faces[0].flashback);
    let flashback_ok = !in_hand
        && in_own_graveyard
        && (printed_flashback.is_some() || flashback_granted(state, card));
    // Escape (CR 702.138a): from its owner's graveyard, for its own cost.
    let printed_escape = obj
        .card
        .and_then(|c| lookup.card(c.index))
        .and_then(|def| def.faces[0].escape);
    let escape_ok = !in_hand && in_own_graveyard && printed_escape.is_some();
    let beam_cost = obj
        .card
        .and_then(|card| lookup.card(card.index))
        .and_then(|def| def.faces[0].beam_me_up);
    let beam_ok = in_own_graveyard
        && beam_cost.is_some()
        && !beam_return_options(state, player, card).is_empty();
    let mayhem_cost = obj
        .card
        .and_then(|card| lookup.card(card.index))
        .and_then(|def| def.faces[0].mayhem);
    let mayhem_ok =
        in_own_graveyard && mayhem_cost.is_some() && mayhem_discarded_this_turn(state, player, obj);
    // Disturb (CR 702.146) and Aftermath (CR 702.127a): a back face with
    // disturb or aftermath is castable from the owner's graveyard.
    let disturb_ok = !in_hand
        && in_own_graveyard
        && obj
            .card
            .and_then(|c| lookup.card(c.index))
            .is_some_and(|def| def.faces.iter().any(|f| f.disturb || f.aftermath));
    let jump_start_ok = !in_hand
        && in_own_graveyard
        && (obj
            .card
            .and_then(|c| lookup.card(c.index))
            .is_some_and(|def| def.faces[0].jump_start)
            || jump_start_granted(state, card));
    // Retrace (CR 702.81a): castable from its owner's graveyard by discarding
    // a land card in addition to paying its other costs.
    let retrace_ok = !in_hand
        && in_own_graveyard
        && obj
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::RETRACE);
    // Foretell (CR 702.143a): this exact exiled incarnation may be cast by
    // its owner after the turn on which it was foretold, for its printed
    // alternative cost. Its hidden face is restored by SpellForm::Foretell.
    let foretell_cost = obj
        .card
        .and_then(|c| lookup.card(c.index))
        .and_then(|def| def.faces[0].foretell);
    let foretell_ok = foretell_cost.is_some() && foretell_permission(state, player, card);
    if matches!(form, Some(SpellForm::Foretell(_))) && !foretell_ok {
        return Err(CastError::NotInHand);
    }
    let plot_ok = plot_permission(state, player, card);
    if matches!(form, Some(SpellForm::Plot)) && !plot_ok {
        return Err(CastError::NotInHand);
    }
    let warp_exile_ok = !in_hand && warp_exile_permission(state, player, card);
    // Adventure (CR 716): a card on an adventure may be cast from exile.
    let on_adventure = adventure_only_permission(state, player, card);
    let adventure_ok = !in_hand && adventure_permission(state, player, card);
    // Opposition Agent: cards exiled by the takeover are playable by the
    // agent from exile.
    let takeover_ok = !in_hand
        && obj.zone == Zone::Exile
        && obj
            .riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::PlayableFromExileFor(p) if *p == player));
    // Commander (CR 903.8): a commander in the command zone may be cast
    // from there by its owner, at the same timing it would have from a
    // hand. The marker list is what makes it a commander — an emblem is in
    // the same zone and is not castable by anybody.
    let commander_ok = !in_hand
        && obj.zone == Zone::Command
        && obj.zone_owner == Some(player)
        && state
            .commanders
            .get(player.get() as usize)
            .is_some_and(|cs| cs.iter().any(|c| c.object == card));
    // "You may play that card this turn" (Dauthi Voidwalker, Expressive
    // Iteration): a permission for this object, from whatever exile it lies
    // in — the owner's, which is not the caster's.
    let permission = if in_hand {
        None
    } else {
        play_permission(state, player, card)
    };
    // Generic legality probes do not preselect a mode. With no other exile
    // permission, the only such mode is the later-turn foretell cost.
    let form = form
        .or_else(|| {
            (foretell_ok && permission.is_none()).then(|| {
                SpellForm::Foretell(foretell_cost.expect("foretell permission has a cost"))
            })
        })
        .or_else(|| (plot_ok && permission.is_none() && !foretell_ok).then_some(SpellForm::Plot));
    let granted_escape =
        in_own_graveyard && !granted_escape_options(state, lookup, player, card, paying).is_empty();
    // Wrenn's emblem and Muldrotha: a permanent card cast from its owner's
    // graveyard at its own price.
    let graveyard_ok = !in_hand && graveyard_cast_permission(state, player, obj).is_some();
    let gy_blitz_ok = in_own_graveyard
        && graveyard_alt_open(
            state,
            player,
            card,
            baylee_cards_dsl::GraveyardAltKind::Blitz,
        );
    let gy_warp_ok = in_own_graveyard
        && graveyard_alt_open(
            state,
            player,
            card,
            baylee_cards_dsl::GraveyardAltKind::Warp,
        );
    let gy_mutate_ok = in_own_graveyard
        && graveyard_alt_open(
            state,
            player,
            card,
            baylee_cards_dsl::GraveyardAltKind::Mutate,
        );
    let gy_sneak_ok = in_own_graveyard
        && graveyard_alt_open(
            state,
            player,
            card,
            baylee_cards_dsl::GraveyardAltKind::Sneak,
        );
    let gy_alt_any = gy_blitz_ok || gy_warp_ok || gy_mutate_ok || gy_sneak_ok;
    let gy_alt_only = gy_alt_any
        && !graveyard_ok
        && !flashback_ok
        && !jump_start_ok
        && !retrace_ok
        && !escape_ok
        && !beam_ok
        && !granted_escape
        && !disturb_ok
        && permission.is_none();
    // A granted flashback or graveyard-alt cost cannot be combined with a second alternative cost.
    let flashback_only =
        in_own_graveyard && flashback_granted(state, card) && !graveyard_ok && permission.is_none();
    if (flashback_only || gy_alt_only)
        && matches!(
            form,
            Some(
                SpellForm::Disguise
                    | SpellForm::Bestow(_)
                    | SpellForm::Prototype(_)
                    | SpellForm::Impending(_)
            )
        )
    {
        return Err(CastError::NoWayToCast);
    }
    if !in_hand
        && !graveyard_ok
        && !takeover_ok
        && !commander_ok
        && !warp_exile_ok
        && matches!(
            form,
            Some(
                SpellForm::Disguise
                    | SpellForm::Bestow(_)
                    | SpellForm::Prototype(_)
                    | SpellForm::Impending(_)
            )
        )
        && !state
            .play_permissions
            .iter()
            .any(|p| p.grants_to(state, player, card, obj.version) && !p.free)
    {
        return Err(CastError::NoWayToCast);
    }
    let graveyard_back = in_own_graveyard
        && obj
            .card
            .and_then(|card| lookup.card(card.index))
            .is_some_and(|def| {
                castable_back_faces(def, false)
                    .any(|(face, _)| graveyard_face_open(state, player, card, face))
            });
    if !in_hand
        && !graveyard_ok
        && !graveyard_back
        && !flashback_ok
        && !jump_start_ok
        && !retrace_ok
        && !escape_ok
        && !beam_ok
        && !mayhem_ok
        && !granted_escape
        && !disturb_ok
        && !adventure_ok
        && !takeover_ok
        && !commander_ok
        && !foretell_ok
        && !plot_ok
        && !warp_exile_ok
        && !gy_alt_any
        && permission.is_none()
    {
        return Err(CastError::NotInHand);
    }
    // The front as it is cast: a form's projection, or a split card's left
    // half where the card is both (`split_front`).
    let raw_obj = obj;
    let projected = form
        .map(|f| f.project(obj))
        .or_else(|| split_front(state, obj));
    let obj = projected.as_ref().unwrap_or(obj);
    let c = obj.characteristics();
    let printed = obj.card.and_then(|c| lookup.card(c.index));
    let fused_allowed = form.is_none()
        && in_hand
        && printed.is_some_and(|def| {
            def.fuse
                && crate::object::reads_both_halves(def)
                && !cast_is_forbidden(state, player, raw_obj)
        });
    // A cast an effect forbids outright (CR 601.3a), asked before timing
    // because the two answers are different and only one of them is true:
    // Silence does not move a spell to sorcery speed, it removes the
    // permission, and a player reading "sorcery-speed timing not met" on
    // their own main phase would go looking for a rule that is not there.
    // Asked of the spell each way of casting makes: a front the effect stops
    // leaves a back face it does not (`face_is_forbidden`), so it is the card
    // as a whole that is refused here only when no face is left.
    let front_forbidden = cast_is_forbidden(state, player, obj);
    let a_back_face_allowed = || {
        form.is_none()
            && printed.is_some_and(|def| {
                castable_back_faces_in_zone(def, on_adventure, disturb_ok)
                    .any(|(i, _)| !face_is_forbidden(state, player, obj, def, i))
            })
    };
    if !may_begin_casting(state, player)
        || (front_forbidden && !a_back_face_allowed() && !fused_allowed)
    {
        return Err(CastError::Forbidden);
    }
    if granted_escape && form.is_none() {
        return Ok(());
    }
    // Timing (CR 601.3). Read off the projected characteristics, so a
    // granted flash counts. That is the front face's timing, and when it
    // says no, a back face cast in its place may still say yes by its own
    // (`face_timing_allows`): Vantress Visions is an instant on the back of
    // an enchantment, and it is cast in answer to an ability on the stack
    // or not at all. A prototype or a disguise is the front, cast another
    // way.
    let gy_front_ok = !gy_alt_only || gy_blitz_ok || gy_warp_ok || gy_mutate_ok;
    let front_now = !front_forbidden
        && !c.types.contains(TypeSet::LAND)
        && gy_front_ok
        && graveyard_face_open(state, player, card, 0)
        && permitted_spell_types(state, player, card, c.types, 0)
        && if matches!(form, Some(SpellForm::Plot)) {
            sorcery_window(state, player)
        } else {
            object_timing_allows(state, player, obj)
        }
        && printed.is_none_or(|def| spell_condition_allows(state, player, card, def, 0));
    let sneak_now = form.is_none()
        && !front_forbidden
        && !c.types.contains(TypeSet::LAND)
        && graveyard_face_open(state, player, card, 0)
        && permitted_spell_types(state, player, card, c.types, 0)
        && in_sneak_window(state, player)
        && sneak_cost_for(state, player, card, printed, 0).is_some()
        && printed.is_none_or(|def| spell_condition_allows(state, player, card, def, 0));
    let face_open = |def: &baylee_cards_dsl::CardDef, i: usize| {
        face_timing_allows(state, player, card, def, i)
            && !face_is_forbidden(state, player, obj, def, i)
    };
    let a_back_face_now = printed.is_some_and(|def| {
        castable_back_faces_in_zone(def, on_adventure, disturb_ok).any(|(i, _)| face_open(def, i))
    });
    let fused_now = fused_allowed
        && permitted_spell_types(state, player, card, raw_obj.characteristics().types, 0)
        && object_timing_allows(state, player, raw_obj)
        && printed.is_some_and(|def| {
            spell_condition_allows(state, player, card, def, 0)
                && spell_condition_allows(state, player, card, def, 1)
        });
    if !front_now && !sneak_now && (form.is_some() || (!a_back_face_now && !fused_now)) {
        return Err(CastError::BadTiming);
    }
    // "As an additional cost to cast this spell, sacrifice a creature": a
    // board with nothing to sacrifice cannot pay it, so the spell is not a
    // cast this player can make (CR 601.2h). The reader is the one the cast
    // wizard's `Sacrifice` stage builds its question from, so the offer and
    // the question cannot disagree — Force of Will's pitch is the precedent.
    let front_additional_ok = matches!(form, Some(SpellForm::Disguise))
        || obj
            .card
            .and_then(|c| lookup.card(c.index))
            .is_none_or(|def| {
                additional_payment_capacity_for(
                    state,
                    player,
                    card,
                    &def.faces[0],
                    match form {
                        Some(SpellForm::Bestow(_)) => Some(crate::choice::CastModeKind::Bestow),
                        Some(SpellForm::Impending(_)) => {
                            Some(crate::choice::CastModeKind::Impending)
                        }
                        _ => None,
                    },
                    &[],
                )
                .is_some()
            });
    if !front_additional_ok && (form.is_some() || !a_back_face_now) {
        return Err(CastError::NoWayToCast);
    }
    let front_now = front_now && front_additional_ok;
    let sneak_now = sneak_now && front_additional_ok;
    let fused_now = fused_now && front_additional_ok;
    if matches!(form, Some(SpellForm::Bestow(_)))
        && printed.is_none_or(|def| {
            !kicked_targets_reachable(state, def, player, card, BESTOW_TARGET_REQ)
        })
    {
        return Err(CastError::NoWayToCast);
    }
    // Restricted mana this spell may be paid with counts towards it; see
    // [`spendable_pool`].
    let with_restricted = planning_pool(
        state,
        player,
        form.map_or(SpendFor::Spell(card), |f| SpendFor::SpellAs(card, f)),
        0,
    );
    let pool = with_restricted
        .as_ref()
        .unwrap_or(&state.players[player.get() as usize].mana_pool);
    // Commander tax (CR 903.8). A cost *increase*, so it lands on every way
    // of casting the card — printed cost, alternative cost and mode alike
    // (CR 601.2f) — which is why it is folded into each probe below rather
    // than into the first one.
    let raw_adjustment = spell_adjustment_with_def(state, player, obj, printed);
    let preview_delta = if matches!(form, Some(SpellForm::Disguise)) {
        PreviewDynamicDelta::default()
    } else {
        let preview_kind = if matches!(form, Some(SpellForm::Bestow(_))) {
            PreviewCastKind::Bestow
        } else {
            PreviewCastKind::Face
        };
        printed.map_or_else(PreviewDynamicDelta::default, |def| {
            preview_cast_metric_bounds_for(
                state,
                player,
                card,
                def,
                0,
                form,
                preview_kind,
                &raw_adjustment,
            )
        })
    };
    let adjustment = raw_adjustment.for_preview(preview_delta);
    if adjustment.life_increase > 0
        && !state.can_pay_life(
            player,
            i32::try_from(adjustment.life_increase).unwrap_or(i32::MAX),
        )
    {
        return Err(CastError::NoWayToCast);
    }
    let tax = adjustment.increase;
    if let Some(form) = form {
        let form_printed = if matches!(form, SpellForm::Disguise) {
            0
        } else {
            printed.map_or(0, |def| {
                printed_reduction(state, &def.faces[0], player, card)
            })
        };
        if matches!(form, SpellForm::Plot)
            && let Some(def) = printed
            && modes_are_the_only_way(def, 0)
        {
            let allow = adjustment.allowance(state, SpendFor::SpellAs(card, form), form_printed);
            let probe_plot = |cost: &ManaCost| {
                affordable(
                    state,
                    player,
                    pool,
                    &adjustment.price(*cost, form_printed),
                    SpendFor::SpellAs(card, form),
                    allow,
                )
            };
            let any_mode = def.abilities.iter().any(|a| {
                let Some((modes, choose, can_repeat)) = a.modal_spell() else {
                    return false;
                };
                if can_repeat {
                    repeated_mode_sets(modes, choose).into_iter().any(|packed| {
                        probe_plot(&repeated_mode_set_cost(ManaCost::ZERO, modes, packed).with_x(0))
                            && repeated_mode_set_has_legal_targets(
                                state, lookup, player, card, packed,
                            )
                    })
                } else if !choose.is_one() {
                    mode_sets(modes, choose).any(|set| {
                        probe_plot(&mode_set_cost(ManaCost::ZERO, modes, set).with_x(0))
                            && mode_set_has_legal_targets(state, lookup, player, card, set)
                    })
                } else {
                    modes.iter().enumerate().any(|(i, m)| {
                        m.cost_override.is_none()
                            && probe_plot(&ManaCost::ZERO)
                            && mode_has_a_legal_target(state, lookup, player, card, i)
                    })
                }
            });
            if any_mode {
                return Ok(());
            }
            let any_targetable = def.abilities.iter().any(|a| {
                let Some((modes, choose, can_repeat)) = a.modal_spell() else {
                    return false;
                };
                if can_repeat {
                    repeated_mode_sets(modes, choose).into_iter().any(|packed| {
                        repeated_mode_set_has_legal_targets(state, lookup, player, card, packed)
                    })
                } else if !choose.is_one() {
                    mode_sets(modes, choose)
                        .any(|set| mode_set_has_legal_targets(state, lookup, player, card, set))
                } else {
                    modes.iter().enumerate().any(|(i, m)| {
                        m.cost_override.is_none()
                            && mode_has_a_legal_target(state, lookup, player, card, i)
                    })
                }
            });
            return Err(if any_targetable {
                CastError::NotEnoughMana
            } else {
                CastError::NoWayToCast
            });
        }
        let form_cost = if matches!(form, SpellForm::Blitz) {
            printed
                .and_then(|def| effective_blitz_cost(state, player, card, &def.faces[0], 0))
                .map_or(ManaCost::ZERO, |c| c.mana)
        } else {
            form.cost()
        };
        return if affordable(
            state,
            player,
            pool,
            &adjustment.price(form_cost.with_x(0), form_printed),
            SpendFor::SpellAs(card, form),
            adjustment.allowance(state, SpendFor::SpellAs(card, form), form_printed),
        ) || matches!(form, SpellForm::Prototype(_))
            && north_star_face_allowed(state, player, card, 0)
            && normal_spell_affordable(
                state,
                player,
                SpendFor::SpellAs(card, form),
                pool,
                &adjustment.price(form_cost, form_printed),
                &form_cost,
                adjustment.allowance(state, SpendFor::SpellAs(card, form), form_printed),
            ) {
            Ok(())
        } else {
            Err(CastError::NotEnoughMana)
        };
    }
    // Convoke and delve are *reductions* of the generic part, so they go on
    // the same probes the tax does and in the other direction. Read off the
    // printed face: a granted convoke does not exist.
    let printed_face = printed.map(|def| &def.faces[0]);
    let reduction = adjustment
        .reduction
        .saturating_add(printed_face.map_or(0, |face| {
            keyword_reduction(state, face, player, card)
                .saturating_add(printed_reduction(state, face, player, card))
        }));
    let allowance = adjustment.allowance(
        state,
        SpendFor::Spell(card),
        reduction.saturating_sub(adjustment.reduction),
    );
    // Convoke's creatures pay coloured symbols too (CR 702.51a), so the
    // reduction is applied through `keyword_priced` and not as generic alone.
    let priced = |cost: &ManaCost| {
        let cost = cost
            .combine(&adjustment.mana_increase)
            .with_more_generic(tax)
            .with_less_mana(&adjustment.colored_only_reduction, true)
            .with_less_mana(&adjustment.mana_reduction, false);
        printed_face.map_or_else(
            || cost.with_less_generic(reduction),
            |face| keyword_priced_for(state, card, 0, face, player, &cost, reduction),
        )
    };
    let probe = |cost: &ManaCost| {
        affordable(
            state,
            player,
            pool,
            &priced(cost),
            SpendFor::Spell(card),
            allowance,
        )
    };
    let probe_normal = |cost: &ManaCost| {
        probe(cost)
            || north_star_face_allowed(state, player, card, 0)
                && normal_spell_affordable(
                    state,
                    player,
                    SpendFor::Spell(card),
                    pool,
                    &priced(cost),
                    &c.mana_cost,
                    allowance,
                )
    };
    let has_castable_backs = form.is_none()
        && printed.is_some_and(|def| {
            castable_back_faces_in_zone(def, on_adventure, disturb_ok)
                .next()
                .is_some()
        });
    let front_target_ok =
        !has_castable_backs || face_has_a_legal_target(state, lookup, player, card, 0);
    // A waived mana cost still pays increases (CR 601.2f).
    if permission.is_some_and(|p| p.free) {
        let free_base = printed_face.map_or(ManaCost::ZERO, |f| {
            face_additional_mana_floor(state, player, card, f)
        });
        let payable = if let Some(def) = printed.filter(|def| modes_are_the_only_way(def, 0)) {
            def.abilities_for_face(0).iter().any(|a| {
                let Some((modes, choose, can_repeat)) = a.modal_spell() else {
                    return false;
                };
                if can_repeat {
                    repeated_mode_sets(modes, choose).into_iter().any(|packed| {
                        probe(&repeated_mode_set_cost(free_base, modes, packed))
                            && repeated_mode_set_has_legal_targets(
                                state, lookup, player, card, packed,
                            )
                    })
                } else if !choose.is_one() {
                    mode_sets(modes, choose).any(|set| {
                        probe(&mode_set_cost(free_base, modes, set))
                            && mode_set_has_legal_targets(state, lookup, player, card, set)
                    })
                } else {
                    probe(&free_base)
                        && modes.iter().enumerate().any(|(i, mode)| {
                            mode.cost_override.is_none()
                                && mode_has_a_legal_target(state, lookup, player, card, i)
                        })
                }
            })
        } else {
            probe(&free_base)
        };
        let back_targetable = printed.is_some_and(|def| {
            castable_back_faces(def, on_adventure).any(|(face, _)| {
                face_open(def, face) && face_has_a_legal_target(state, lookup, player, card, face)
            })
        });
        let back_payable = printed.is_some_and(|def| {
            castable_back_faces(def, on_adventure).any(|(face, _)| {
                face_open(def, face)
                    && face_has_a_legal_target(state, lookup, player, card, face)
                    && free_face_affordable(state, player, card, def, face)
            })
        });
        return if (front_now && front_target_ok && payable) || back_payable {
            Ok(())
        } else if has_castable_backs && !(front_now && front_target_ok) && !back_targetable {
            Err(CastError::NoWayToCast)
        } else {
            Err(CastError::NotEnoughMana)
        };
    }
    let probe_back = |def: &'static baylee_cards_dsl::CardDef, i: usize| {
        let face = &def.faces[i];
        if additional_payment_capacity_for(
            state,
            player,
            card,
            face,
            Some(crate::choice::CastModeKind::Face(i)),
            &[],
        )
        .is_none()
        {
            return false;
        }
        let restricted = planning_pool(state, player, SpendFor::SpellOnFace(card, i), 0);
        let pool = restricted
            .as_ref()
            .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
        let raw_adj = face_adjustment(state, player, card, def, i, None);
        let adjustment = raw_adj.for_preview(preview_cast_metric_bounds(
            state, player, card, def, i, &raw_adj,
        ));
        let tax = adjustment.increase;
        let reduction = adjustment
            .reduction
            .saturating_add(printed_reduction(state, face, player, card))
            .saturating_add(keyword_reduction(state, face, player, card));
        let allowance = adjustment.allowance(
            state,
            SpendFor::SpellOnFace(card, i),
            reduction.saturating_sub(adjustment.reduction),
        );
        let base_mana = face
            .mana_cost
            .combine(&face_additional_mana_floor(state, player, card, face));
        let cost = keyword_priced_for(
            state,
            card,
            i,
            face,
            player,
            &base_mana
                .with_x(0)
                .combine(&adjustment.mana_increase)
                .with_more_generic(tax)
                .with_less_mana(&adjustment.colored_only_reduction, true)
                .with_less_mana(&adjustment.mana_reduction, false),
            reduction,
        );
        affordable(
            state,
            player,
            pool,
            &cost,
            SpendFor::SpellOnFace(card, i),
            allowance,
        ) || !face.disturb
            && north_star_face_allowed(state, player, card, i)
            && normal_spell_affordable(
                state,
                player,
                SpendFor::SpellOnFace(card, i),
                pool,
                &cost,
                &face.mana_cost,
                allowance,
            )
    };
    // A back face that may be cast now, affordable and with something to
    // point at: the same three questions the wizard asks of it.
    let a_back_face_castable = || {
        printed.is_some_and(|def| {
            castable_back_faces_in_zone(def, on_adventure, disturb_ok).any(|(i, _)| {
                face_open(def, i)
                    && graveyard_face_open(state, player, card, i)
                    && probe_back(def, i)
                    && face_has_a_legal_target(state, lookup, player, card, i)
            })
        })
    };
    let a_fused_castable = || {
        fused_now
            && printed.is_some_and(|def| {
                additional_payment_capacity_for(
                    state,
                    player,
                    card,
                    &def.faces[1],
                    Some(crate::choice::CastModeKind::Face(1)),
                    &[],
                )
                .is_some()
                    && face_has_a_legal_target(state, lookup, player, card, 0)
                    && face_has_a_legal_target(state, lookup, player, card, 1)
                    && {
                        let raw_adj = spell_adjustment_with_def(state, player, raw_obj, Some(def));
                        let adj = raw_adj.for_preview(preview_cast_metric_bounds_for(
                            state,
                            player,
                            card,
                            def,
                            0,
                            None,
                            PreviewCastKind::Fused,
                            &raw_adj,
                        ));
                        let printed_red = printed_reduction(state, &def.faces[0], player, card)
                            .saturating_add(printed_reduction(state, &def.faces[1], player, card));
                        let kw_red = keyword_reduction(state, &def.faces[0], player, card);
                        let allow = adj.allowance(
                            state,
                            SpendFor::Spell(card),
                            printed_red.saturating_add(kw_red),
                        );
                        let cost = keyword_priced_for(
                            state,
                            card,
                            0,
                            &def.faces[0],
                            player,
                            &adj.price(fused_mana_cost(def).with_x(0), printed_red),
                            kw_red,
                        );
                        affordable(state, player, pool, &cost, SpendFor::Spell(card), allow)
                    }
            })
    };
    let a_back_face_targetable = || {
        printed.is_some_and(|def| {
            castable_back_faces_in_zone(def, on_adventure, disturb_ok).any(|(i, _)| {
                face_open(def, i)
                    && graveyard_face_open(state, player, card, i)
                    && face_has_a_legal_target(state, lookup, player, card, i)
            })
        })
    };
    let a_fused_targetable = || {
        fused_now
            && printed.is_some_and(|def| {
                additional_payment_capacity_for(
                    state,
                    player,
                    card,
                    &def.faces[1],
                    Some(crate::choice::CastModeKind::Face(1)),
                    &[],
                )
                .is_some()
                    && face_has_a_legal_target(state, lookup, player, card, 0)
                    && face_has_a_legal_target(state, lookup, player, card, 1)
            })
    };
    // The front may not be cast normally now, so unless sneak makes it castable
    // only a back face or fused cast can make the card castable.
    if !front_now && !sneak_now {
        return if a_back_face_castable() || a_fused_castable() {
            Ok(())
        } else if has_castable_backs && !a_back_face_targetable() && !a_fused_targetable() {
            Err(CastError::NoWayToCast)
        } else {
            Err(CastError::NotEnoughMana)
        };
    }
    // A printed flashback is paid "rather than its mana cost" (CR 702.34a),
    if beam_ok && beam_cost.is_some_and(|cost| probe(&cost)) {
        return Ok(());
    }
    if in_own_graveyard
        && beam_cost.is_some()
        && !graveyard_ok
        && !flashback_ok
        && !escape_ok
        && !mayhem_ok
    {
        return Err(CastError::NoWayToCast);
    }
    if mayhem_ok && let Some(cost) = mayhem_cost {
        let parts_payable = printed.is_some_and(|def| {
            additional_payment_capacity_for(
                state,
                player,
                card,
                &def.faces[0],
                Some(crate::choice::CastModeKind::Mayhem),
                &[],
            )
            .is_some()
        });
        if parts_payable && probe(&cost.with_x(0)) {
            return Ok(());
        }
        if !flashback_granted(state, card)
            && !graveyard_ok
            && !flashback_ok
            && !escape_ok
            && !jump_start_ok
            && !retrace_ok
        {
            return Err(if parts_payable {
                CastError::NotEnoughMana
            } else {
                CastError::NoWayToCast
            });
        }
    }
    // so from the graveyard its cost is the price — or, beside a grant
    // (Past in Flames), one of two, the grant's being the mana cost below.
    if flashback_ok && let Some(cost) = printed_flashback {
        let parts_payable = printed.is_some_and(|def| {
            additional_payment_capacity_for(
                state,
                player,
                card,
                &def.faces[0],
                Some(crate::choice::CastModeKind::Flashback),
                &[],
            )
            .is_some()
        });
        if parts_payable && probe(&cost.with_x(0)) {
            return Ok(());
        }
        if !flashback_granted(state, card) && !graveyard_ok && !jump_start_ok && !retrace_ok {
            return Err(if parts_payable {
                CastError::NotEnoughMana
            } else {
                CastError::NoWayToCast
            });
        }
    }
    if jump_start_ok {
        let is_instant_or_sorcery = c.types.intersects(TypeSet::INSTANT.union(TypeSet::SORCERY));
        let parts_payable = is_instant_or_sorcery
            && printed.is_some_and(|def| {
                additional_payment_capacity_for(
                    state,
                    player,
                    card,
                    &def.faces[0],
                    Some(crate::choice::CastModeKind::JumpStart),
                    &[],
                )
                .is_some()
            });
        if parts_payable && probe_normal(&c.mana_cost.with_x(0)) {
            return Ok(());
        }
        if !flashback_granted(state, card) && !graveyard_ok && !escape_ok && !retrace_ok {
            return Err(if parts_payable {
                CastError::NotEnoughMana
            } else {
                CastError::NoWayToCast
            });
        }
    }
    if retrace_ok {
        let parts_payable = !c.types.contains(TypeSet::LAND)
            && printed.is_some_and(|def| {
                additional_payment_capacity_for(
                    state,
                    player,
                    card,
                    &def.faces[0],
                    Some(crate::choice::CastModeKind::Retrace),
                    &[],
                )
                .is_some()
            });
        if parts_payable && probe_normal(&c.mana_cost.with_x(0)) {
            return Ok(());
        }
        if !flashback_granted(state, card) && !graveyard_ok && !escape_ok {
            return if a_back_face_castable() {
                Ok(())
            } else if parts_payable {
                Err(CastError::NotEnoughMana)
            } else {
                Err(CastError::NoWayToCast)
            };
        }
    }
    // Escape's price is its mana and the other cards it exiles; beside a
    // graveyard permission (Muldrotha) the mana cost below is a second way.
    if escape_ok && let Some(escape) = printed_escape {
        let fodder = escape_exile_options(state, player, card).len() >= usize::from(escape.exile);
        if fodder && probe(&escape.cost.with_x(0)) {
            return Ok(());
        }
        // Too few other cards is a cost that cannot be paid (CR 601.2h),
        // whatever the pool holds, as a missing sacrifice is above.
        if !graveyard_ok {
            return Err(if fodder {
                CastError::NotEnoughMana
            } else {
                CastError::NoWayToCast
            });
        }
    }
    if let Some(def) = printed
        && !def.faces[0].additional_costs.is_empty()
    {
        let face = &def.faces[0];
        let kicked_parts_ok = additional_payment_capacity_for(
            state,
            player,
            card,
            face,
            Some(crate::choice::CastModeKind::Kicked),
            &[],
        )
        .is_some();
        let restricted = planning_pool(state, player, SpendFor::KickedSpell(card, 0, form), 0);
        let kicked_pool = restricted
            .as_ref()
            .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
        let reachable = face.kicked_targets.map_or_else(
            || face_has_a_legal_target(state, lookup, player, card, 0),
            |req| kicked_targets_reachable(state, def, player, card, req),
        );
        let kicked_adj =
            if face.kicked_targets.is_some() && raw_adjustment.has_target_conditional_reduction {
                raw_adjustment.for_preview(preview_cast_metric_bounds_for(
                    state,
                    player,
                    card,
                    def,
                    0,
                    form,
                    PreviewCastKind::Kicked,
                    &raw_adjustment,
                ))
            } else {
                adjustment
            };
        let kicked_cost = kicked_mana_cost(face)
            .with_x(0)
            .combine(&kicked_adj.mana_increase)
            .with_more_generic(kicked_adj.increase)
            .with_less_mana(&kicked_adj.colored_only_reduction, true)
            .with_less_mana(&kicked_adj.mana_reduction, false)
            .with_less_generic(
                kicked_adj
                    .reduction
                    .saturating_add(reduction.saturating_sub(adjustment.reduction)),
            );
        if kicked_parts_ok
            && reachable
            && (affordable(
                state,
                player,
                kicked_pool,
                &kicked_cost,
                SpendFor::KickedSpell(card, 0, form),
                allowance,
            ) || north_star_face_allowed(state, player, card, 0)
                && normal_spell_affordable(
                    state,
                    player,
                    SpendFor::KickedSpell(card, 0, form),
                    kicked_pool,
                    &kicked_cost,
                    &face.mana_cost,
                    allowance,
                ))
        {
            return Ok(());
        }
    }
    if let Some(def) = printed
        && let Some(opt_cost) = def.faces[0].optional_cost
        && front_now
        && front_target_ok
        && !modes_are_the_only_way(def, 0)
        && has_a_printed_cost(&c.mana_cost)
    {
        let face = &def.faces[0];
        let opt_parts_ok = additional_payment_capacity_for(
            state,
            player,
            card,
            face,
            Some(crate::choice::CastModeKind::OptionalCost),
            &[],
        )
        .is_some();
        if opt_parts_ok {
            let mut opt_spell = obj.clone();
            if !opt_spell
                .riders
                .contains(&crate::object::Rider::OptionalCostPaid)
            {
                opt_spell
                    .riders
                    .push(crate::object::Rider::OptionalCostPaid);
            }
            let opt_adj = spell_adjustment_with_def(state, player, &opt_spell, Some(def));
            let opt_printed = printed_reduction(state, face, player, card);
            let opt_kw = if face.waterbend {
                u32::try_from(waterbend_sources(state, player).len())
                    .unwrap_or(u32::MAX)
                    .min(opt_cost.mana.generic_total())
            } else {
                keyword_reduction(state, face, player, card)
            };
            let opt_priced = keyword_priced_for(
                state,
                card,
                0,
                face,
                player,
                &opt_adj.price(
                    c.mana_cost
                        .combine(&opt_cost.mana)
                        .combine(&face_additional_mana_floor(state, player, card, face))
                        .with_x(0),
                    opt_printed,
                ),
                opt_kw,
            );
            let opt_allow = opt_adj.allowance(
                state,
                SpendFor::Spell(card),
                opt_printed.saturating_add(opt_kw),
            );
            if affordable(
                state,
                player,
                pool,
                &opt_priced,
                SpendFor::Spell(card),
                opt_allow,
            ) || (north_star_face_allowed(state, player, card, 0)
                && normal_spell_affordable(
                    state,
                    player,
                    SpendFor::Spell(card),
                    pool,
                    &opt_priced,
                    &face.mana_cost,
                    opt_allow,
                ))
            {
                return Ok(());
            }
        }
    }
    if let Some(def) = printed
        && let Some(req) = def.faces[0].kicked_targets
    {
        let face = &def.faces[0];
        let kicked_parts_ok = additional_payment_capacity_for(
            state,
            player,
            card,
            face,
            Some(crate::choice::CastModeKind::Kicked),
            &[],
        )
        .is_some();
        let ordinary_targets = ordinary_targets_reachable(state, def, player, card);
        let kicked_targets =
            kicked_parts_ok && kicked_targets_reachable(state, def, player, card, req);
        let ordinary = ordinary_targets && probe_normal(&c.mana_cost.with_x(0));
        let kicked = kicked_targets && probe_normal(&kicked_mana_cost(face).with_x(0));
        // Short of mana with something to point at is what `payable` lists:
        // answered as no way to cast, Tear Asunder over an artifact could
        // never be cast by tapping for it.
        return if ordinary || kicked {
            Ok(())
        } else if ordinary_targets || kicked_targets {
            Err(CastError::NotEnoughMana)
        } else {
            Err(CastError::NoWayToCast)
        };
    }
    // A disturb cast is not the front face at any price. CR 702.146 casts the
    // card *transformed*, for the back's disturb cost, and `cast_options` has
    // always known it — its disturb branch returns the backs and nothing
    // else, so there is no front-cost option here for this probe to be
    // agreeing with. It asked about the front's mana cost anyway: Mirrorhall
    // Mimic is `{3}{U}` in front of a `{3}{U}{U}` disturb, so four mana put
    // it in `legal.castable` and the wizard then refused it with "no way to
    // cast this spell".
    if disturb_ok {
        let targetable_disturb = printed.is_some_and(|def| {
            def.faces.iter().enumerate().skip(1).any(|(i, f)| {
                (f.disturb || f.aftermath)
                    && graveyard_face_open(state, player, card, i)
                    && face_timing_allows(state, player, card, def, i)
                    && face_has_a_legal_target(state, lookup, player, card, i)
            })
        });
        let affordable_disturb = printed.is_some_and(|def| {
            def.faces.iter().enumerate().skip(1).any(|(i, f)| {
                (f.disturb || f.aftermath)
                    && graveyard_face_open(state, player, card, i)
                    && face_timing_allows(state, player, card, def, i)
                    && probe_back(def, i)
                    && face_has_a_legal_target(state, lookup, player, card, i)
            })
        });
        if affordable_disturb {
            return Ok(());
        }
        // Muldrotha or Wrenn's emblem cast the *front* from the graveyard
        // too, at its own price — the probes below.
        if !graveyard_ok {
            return Err(if targetable_disturb {
                CastError::NotEnoughMana
            } else {
                CastError::NoWayToCast
            });
        }
    }
    // Printed cost probed with X = 0, and after a reduction printed on the
    // card itself; the full payment is validated when the wizard finishes.
    // A face with no printed cost has no normal way to be cast at all
    // (CR 202.1b) and falls straight through to the alternatives.
    let normal_cost = printed_face.map_or(c.mana_cost, |f| {
        c.mana_cost
            .combine(&face_additional_mana_floor(state, player, card, f))
    });
    // `c.mana_cost` and not `normal_cost`: the question is what the card
    // *prints*, and cost arithmetic does not preserve the answer —
    // `with_less_generic` rebuilds a cost symbol by symbol and drops a
    // `Generic(0)`, so `{0}` reduced by nothing is indistinguishable from a
    // blank. Asking before the arithmetic is also the right rule, because a
    // reduction that takes `{1}` down to nothing leaves a spell that is cast
    // for free and was always castable.
    // A card whose only spell ability is modal has no normal way to be cast
    // at all — `cast_options` offers none — so its printed price is not an
    // answer about the card, and taking it as one is how Damn was offered on
    // a board that could take neither of its modes. The price `{B}{B}`
    // belongs to "destroy target creature", which had nothing to destroy; the
    // mode that needed no target was `{2}{W}{W}` on three Swamps. Both
    // questions were asked and both were answered yes, about different modes.
    let modal_only = printed.is_some_and(|def| modes_are_the_only_way(def, 0));
    if !front_now
        || !front_target_ok
        || modal_only
        || gy_alt_only
        || !has_a_printed_cost(&c.mana_cost)
        || !probe_normal(&normal_cost.with_x(0))
    {
        // Alternative costs may still make it castable (pitch/evoke). The
        // wizard computes the exact options; this decides only whether there
        // is one, and asks about the whole cost to do it.
        let Some(card_ref) = obj.card else {
            return Err(CastError::NotEnoughMana);
        };
        let Some(def) = lookup.card(card_ref.index) else {
            return Err(CastError::NotEnoughMana);
        };
        let face = &def.faces[0];
        // Mana is not the whole of an alternative cost, and this probe used
        // to behave as though it were: a Force of Will with no other blue
        // card in hand has a zero mana cost, so it went into
        // `legal.castable` and the wizard then reversed the cast at the pitch
        // stage. Every part is asked about here, the way every condition is
        // asked about below — the two halves of the same offer.
        let granted_alts = granted_stack_alternatives(state, player, card, 0);
        let any_alt = front_now
            && front_target_ok
            && !flashback_only
            && !gy_alt_only
            && (face.alternative_costs.iter().enumerate().any(|(i, alt)| {
                // Every condition, not merely the one that was written first.
                // `CommanderControlled` fell through the old `!matches!` as
                // "true", so Flawless Maneuver and Fierce Guardianship were
                // offered as castable with no commander anywhere — and the
                // wizard, which does check the condition, then found no way to
                // cast them at all.
                let condition_ok = match alt.condition {
                    baylee_cards_dsl::AltCondition::Always => true,
                    baylee_cards_dsl::AltCondition::NotYourTurn => state.turn.active != player,
                    baylee_cards_dsl::AltCondition::CommanderControlled => {
                        controls_a_commander(state, player)
                    }
                    baylee_cards_dsl::AltCondition::If(condition) => {
                        crate::eval::condition_holds(state, player, card, *condition)
                    }
                };
                condition_ok
                    && probe(&alt.cost.mana)
                    && alternative_parts_payable(state, player, card, alt.cost.parts)
                    && additional_payment_capacity_for(
                        state,
                        player,
                        card,
                        face,
                        Some(crate::choice::CastModeKind::Alternative(i)),
                        &[],
                    )
                    .is_some()
            }) || granted_alts
                .iter()
                .enumerate()
                .any(|(k, (cost, condition))| {
                    let idx = face.alternative_costs.len() + k;
                    crate::eval::condition_holds(state, player, card, *condition)
                        && probe(cost)
                        && additional_payment_capacity_for(
                            state,
                            player,
                            card,
                            face,
                            Some(crate::choice::CastModeKind::Alternative(idx)),
                            &[],
                        )
                        .is_some()
                }));
        // Affordable *and* pointable, of the **same** mode. A mode's price
        // and a mode's target line are the two halves of one way to cast the
        // card, and this probe used to ask only the first while
        // `has_a_legal_target` asked the second of whichever mode happened to
        // answer yes — so a board where one mode was payable and a *different*
        // one was targetable offered a card the wizard then refused. It is the
        // same intersection `cast_options` makes two files away, which is the
        // half that was already right.
        let any_mode = front_now
            && !gy_alt_only
            && def.abilities.iter().any(|a| {
                let Some((modes, choose, can_repeat)) = a.modal_spell() else {
                    return false;
                };
                if can_repeat {
                    repeated_mode_sets(modes, choose).into_iter().any(|packed| {
                        (!flashback_only
                            || mana_cost_mode(
                                def,
                                0,
                                crate::choice::CastModeKind::RepeatedModes(packed),
                            ))
                            && probe_normal(
                                &repeated_mode_set_cost(face.mana_cost, modes, packed).with_x(0),
                            )
                            && repeated_mode_set_has_legal_targets(
                                state, lookup, player, card, packed,
                            )
                    })
                } else if !choose.is_one() {
                    mode_sets(modes, choose).any(|set| {
                        (!flashback_only
                            || mana_cost_mode(def, 0, crate::choice::CastModeKind::Modes(set)))
                            && probe_normal(&mode_set_cost(face.mana_cost, modes, set).with_x(0))
                            && mode_set_has_legal_targets(state, lookup, player, card, set)
                    })
                } else {
                    modes.iter().enumerate().any(|(i, m)| {
                        (!flashback_only || m.cost_override.is_none())
                            && (if m.cost_override.is_some() {
                                probe(&m.cost_override.unwrap_or(face.mana_cost).with_x(0))
                            } else {
                                probe_normal(&face.mana_cost.with_x(0))
                            })
                            && mode_has_a_legal_target(state, lookup, player, card, i)
                    })
                }
            });
        // And every other face the wizard would offer, at its own timing and
        // with its own target check (`a_back_face_castable` / `a_fused_castable`).
        let any_face = a_back_face_castable() || a_fused_castable();
        // Dash (CR 702.109a), the offer's `CastModeKind::Dash`.
        let any_dash = front_now
            && front_target_ok
            && !flashback_only
            && !gy_alt_only
            && face.dash.is_some_and(|dash| probe(&dash));
        // Blitz (CR 702.152a), the offer's `CastModeKind::Blitz`.
        let any_blitz = front_now
            && front_target_ok
            && !flashback_only
            && (!gy_alt_only || gy_blitz_ok)
            && effective_blitz_cost(state, player, card, face, 0).is_some_and(|blitz| {
                let blitz_adj =
                    face_adjustment(state, player, card, def, 0, Some(SpellForm::Blitz));
                let blitz_preview = blitz_adj.for_preview(preview_cast_metric_bounds_for(
                    state,
                    player,
                    card,
                    def,
                    0,
                    Some(SpellForm::Blitz),
                    PreviewCastKind::Face,
                    &blitz_adj,
                ));
                let blitz_printed = printed_reduction(state, face, player, card);
                let blitz_kw = keyword_reduction(state, face, player, card);
                let blitz_cost = keyword_priced_for(
                    state,
                    card,
                    0,
                    face,
                    player,
                    &blitz_preview.price(blitz.mana.with_x(0), blitz_printed),
                    blitz_kw,
                );
                let blitz_what = SpendFor::SpellAs(card, SpellForm::Blitz);
                let blitz_pool = planning_pool(state, player, blitz_what, 0);
                let blitz_pool_ref = blitz_pool
                    .as_ref()
                    .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
                let blitz_allow = blitz_preview.allowance(
                    state,
                    blitz_what,
                    blitz_printed.saturating_add(blitz_kw),
                );
                affordable(
                    state,
                    player,
                    blitz_pool_ref,
                    &blitz_cost,
                    blitz_what,
                    blitz_allow,
                ) && alternative_parts_payable(state, player, card, blitz.parts)
                    && additional_payment_capacity_for(
                        state,
                        player,
                        card,
                        face,
                        Some(crate::choice::CastModeKind::Blitz),
                        &[],
                    )
                    .is_some()
            });
        // Warp (CR 702.185a), the offer's `CastModeKind::Warp` (from hand or graveyard permission).
        let any_warp = front_now
            && front_target_ok
            && (in_hand || gy_warp_ok)
            && !flashback_only
            && face.warp.is_some_and(|warp| {
                probe(&warp.mana.with_x(0))
                    && alternative_parts_payable(state, player, card, warp.parts)
                    && additional_payment_capacity_for(
                        state,
                        player,
                        card,
                        face,
                        Some(crate::choice::CastModeKind::Warp),
                        &[],
                    )
                    .is_some()
            });
        // Awaken (CR 702.113a), the offer's `CastModeKind::Awaken`.
        let any_awaken = front_target_ok
            && !flashback_only
            && !gy_alt_only
            && crate::engine::cast_wizard::wave10b::face_awaken(def, 0).is_some_and(
                |(_, awaken)| {
                    crate::engine::cast_wizard::wave10b::awaken_has_legal_land(state, player, card)
                        && probe(&awaken.mana.with_x(0))
                        && alternative_parts_payable(state, player, card, awaken.parts)
                        && additional_payment_capacity_for(
                            state,
                            player,
                            card,
                            face,
                            crate::engine::cast_wizard::CostChoice {
                                option: Some(crate::choice::CastModeKind::Awaken),
                                awaken_parts: awaken.parts,
                                ..Default::default()
                            },
                            &[],
                        )
                        .is_some()
                },
            );
        // Emerge (CR 702.119a), the offer's `CastModeKind::Emerge`.
        let any_emerge = front_now
            && front_target_ok
            && !flashback_only
            && !gy_alt_only
            && face.emerge.is_some_and(|emerge| {
                emerge_candidates(state, player, card, face, emerge)
                    .into_iter()
                    .any(|(_, mv)| {
                        let red = reduction.saturating_add(mv);
                        let cost = keyword_priced(
                            state,
                            face,
                            player,
                            &emerge
                                .cost
                                .with_x(0)
                                .combine(&adjustment.mana_increase)
                                .with_more_generic(tax)
                                .with_less_mana(&adjustment.colored_only_reduction, true)
                                .with_less_mana(&adjustment.mana_reduction, false),
                            red,
                        );
                        let allow = adjustment.allowance(
                            state,
                            SpendFor::Spell(card),
                            red.saturating_sub(adjustment.reduction),
                        );
                        affordable(state, player, pool, &cost, SpendFor::Spell(card), allow)
                    })
            });
        // Mutate (CR 702.140a), the offer's `CastModeKind::Mutate`.
        let any_mutate = front_now
            && front_target_ok
            && !flashback_only
            && (!gy_alt_only || gy_mutate_ok)
            && face.mutate.is_some_and(|mutate| {
                kicked_targets_reachable(state, def, player, card, MUTATE_TARGET_REQ) && {
                    if raw_adjustment.has_target_conditional_reduction {
                        let m_adj = raw_adjustment.for_preview(preview_cast_metric_bounds_for(
                            state,
                            player,
                            card,
                            def,
                            0,
                            None,
                            PreviewCastKind::Mutate,
                            &raw_adjustment,
                        ));
                        let m_cost = m_adj.price(
                            mutate.with_x(0),
                            reduction.saturating_sub(adjustment.reduction),
                        );
                        affordable(
                            state,
                            player,
                            pool,
                            &m_cost,
                            SpendFor::Spell(card),
                            allowance,
                        )
                    } else {
                        probe(&mutate)
                    }
                }
            });
        // Sneak (CR 702.190a), the offer's `CastModeKind::Sneak`.
        let any_sneak = sneak_now
            && front_target_ok
            && !flashback_only
            && (!gy_alt_only || gy_sneak_ok)
            && sneak_cost_for(state, player, card, Some(def), 0).is_some_and(|sneak| {
                probe(&sneak.mana.with_x(0))
                    && alternative_parts_payable(state, player, card, sneak.parts)
                    && !sneak_candidates(state, player, card, face, sneak).is_empty()
            });
        if !any_alt
            && !any_mode
            && !any_face
            && !any_dash
            && !any_blitz
            && !any_warp
            && !any_awaken
            && !any_emerge
            && !any_mutate
            && !any_sneak
        {
            // Which of the two refused matters to whoever reads it. A mode
            // that was affordable and had nothing to point at is not a
            // player one land short, and telling them it is sends them
            // looking for the land.
            let a_mode_was_affordable = front_now
                && def.abilities.iter().any(|a| {
                    let Some((modes, choose, can_repeat)) = a.modal_spell() else {
                        return false;
                    };
                    if can_repeat {
                        repeated_mode_sets(modes, choose).into_iter().any(|packed| {
                            probe_normal(
                                &repeated_mode_set_cost(face.mana_cost, modes, packed).with_x(0),
                            )
                        })
                    } else if !choose.is_one() {
                        mode_sets(modes, choose).any(|set| {
                            probe_normal(&mode_set_cost(face.mana_cost, modes, set).with_x(0))
                        })
                    } else {
                        modes.iter().any(|m| {
                            if m.cost_override.is_some() {
                                probe(&m.cost_override.unwrap_or(face.mana_cost).with_x(0))
                            } else {
                                probe_normal(&face.mana_cost.with_x(0))
                            }
                        })
                    }
                });
            let front_castable_with_target = front_target_ok && (front_now || sneak_now);
            let no_face_targetable = has_castable_backs
                && !front_castable_with_target
                && !a_back_face_targetable()
                && !a_fused_targetable();
            return Err(if a_mode_was_affordable || no_face_targetable {
                CastError::NoWayToCast
            } else {
                CastError::NotEnoughMana
            });
        }
    }
    Ok(())
}

/// Whether `player` could begin to cast `card` by putting it onto the stack
/// from their hand right now, taking cast prohibitions and timing into account
/// (CR 702.62a, CR 702.62c).
#[must_use]
pub(crate) fn can_begin_suspend(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
) -> bool {
    let Some(obj) = state.object(card) else {
        return false;
    };
    if obj.zone != Zone::Hand || obj.zone_owner != Some(player) {
        return false;
    }
    if split_second_on_stack(state) || !may_begin_casting(state, player) {
        return false;
    }
    let projected = split_front(state, obj);
    let obj = projected.as_ref().unwrap_or(obj);
    let c = obj.characteristics();
    let printed = obj.card.and_then(|c| lookup.card(c.index));
    !c.types.contains(TypeSet::LAND)
        && !cast_is_forbidden(state, player, obj)
        && object_timing_allows(state, player, obj)
        && printed.is_none_or(|def| spell_condition_allows(state, player, card, def, 0))
}
/// Does `player` control one of their own commanders right now?
///
/// This is card text, not a rule — "if you control a commander" is what
/// Fierce Guardianship and Flawless Maneuver print — but it lives here
/// because two readers ask it: the legality probe below and the wizard's
/// list of cast options. A probe that answered differently from the wizard
/// offers a spell the wizard then refuses to cast, which is exactly the bug
/// this replaces. It reads the marker list rather than the command zone,
/// since a commander on the battlefield has left that zone by definition.
#[must_use]
pub fn controls_a_commander(state: &GameState, player: PlayerId) -> bool {
    state
        .commanders
        .get(player.get() as usize)
        .is_some_and(|cs| {
            cs.iter().any(|c| {
                state
                    .object(c.object)
                    .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == player)
            })
        })
}

/// The cards that could be exiled from `player`'s hand to pay an
/// `ExileFromHand` cost on `card` — Force of Will's blue card, Solitude's
/// white one.
///
/// One reader, three callers, and that is the whole point of it being a
/// function. `cast_wizard`'s `PitchChoice` stage builds its prompt from this
/// list; [`can_cast`] asks whether the list is empty before calling the card
/// castable; `Engine::can_afford` asks the same before offering the
/// alternative cost as a mode. The two askers used to answer "yes" without
/// looking, so a Force of Will with no other blue card was lit up as castable
/// and then reversed itself on reaching the pitch stage — the shape a dead
/// offer always has, and the one a second predicate written to *agree* with
/// the stage would have kept, because it would have agreed with itself.
///
/// The card being cast is not a candidate: it is what is being paid for, and
/// it is what `eval::matches` is handed as `this`, so a filter saying
/// "another" reads the same word here as anywhere else.
#[must_use]
pub fn pitchable(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    filter: &baylee_cards_dsl::Filter,
) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Hand(player))
        .iter()
        .copied()
        .filter(|id| {
            *id != card
                && state
                    .object(*id)
                    .is_some_and(|o| crate::eval::matches(filter, state, o, player, card))
        })
        .collect()
}

/// The permanents `player` controls on the battlefield and cards in `player`'s
/// hand (other than `card`) that may be chosen to behold `filter` (CR 701.60a).
#[must_use]
pub fn behold_options(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    filter: &baylee_cards_dsl::Filter,
) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|id| {
            *id != card
                && state.object(*id).is_some_and(|o| {
                    o.controller == player && crate::eval::matches(filter, state, o, player, card)
                })
        })
        .collect();
    out.extend(
        state
            .zones
            .list(ZoneLocation::Hand(player))
            .iter()
            .copied()
            .filter(|id| {
                *id != card
                    && state
                        .object(*id)
                        .is_some_and(|o| crate::eval::matches(filter, state, o, player, card))
            }),
    );
    out
}

/// Mandatory additional mana owed when non-mana alternatives (`behold_or_pay`
/// or `CostPart::Either`) have no non-mana branch payable on the current board.
#[must_use]
pub fn face_additional_mana_floor(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: &'static baylee_cards_dsl::FaceDef,
) -> ManaCost {
    let mut extra = if let Some(behold) = face.behold_or_pay
        && behold_options(state, player, card, behold.filter).is_empty()
    {
        behold.cost
    } else {
        ManaCost::ZERO
    };
    let default_choice = crate::engine::cast_wizard::CostChoice::default();
    let (total_eithers, _) = crate::engine::cast_wizard::next_either_cost(face, default_choice);
    let n = total_eithers.min(8);
    if n > 0 {
        let mut best_mana: Option<ManaCost> = None;
        for mask in 0..(1_u8 << n) {
            let sub = crate::engine::cast_wizard::CostChoice {
                either_decided: n,
                either_first_mask: mask,
                ..default_choice
            };
            if additional_payment_capacity_for(state, player, card, face, sub, &[]).is_some() {
                let m = crate::engine::cast_wizard::settled_either_mana(face, sub);
                if m == ManaCost::ZERO {
                    best_mana = Some(ManaCost::ZERO);
                    break;
                }
                if best_mana.as_ref().is_none_or(|prev| m.cmc() < prev.cmc()) {
                    best_mana = Some(m);
                }
            }
        }
        if let Some(m) = best_mana {
            extra = extra.combine(&m);
        }
    }
    let mandatory_wb = face
        .mandatory_additional_costs
        .iter()
        .filter_map(|p| match p {
            baylee_cards_dsl::CostPart::Waterbend(n) => Some(u32::from(*n)),
            _ => None,
        })
        .fold(0_u32, u32::saturating_add);
    if mandatory_wb > 0 {
        extra = extra.with_more_generic(mandatory_wb);
    }
    extra
}

/// The minimum printed mana cost of `face` when cast by `player`: includes
/// `behold_or_pay`'s mana cost when `player` has nothing matching to behold,
/// and any `CostPart::Either` mana branch when no mana-free branch is payable.
#[must_use]
pub fn face_base_mana_cost(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: &'static baylee_cards_dsl::FaceDef,
) -> ManaCost {
    face.mana_cost
        .combine(&face_additional_mana_floor(state, player, card, face))
}

/// Whether the non-mana parts of an alternative cost pass their local checks.
///
/// Object choices are answered exactly by the immediately following
/// `additional_payment_capacity_for` call: it uses `cost_wizard::options` and
/// a bipartite matching, so two parts cannot spend the same permanent. The
/// wizard presents choices from that same reader. `zones.list` is ordered,
/// preserving determinism.
///
/// Every variant is named rather than swept into a wildcard:
/// `cast_wizard::paid_as_an_alternative_cost` says which ones the payment
/// pays, and `offer_tests` holds unsupported parts off this list entirely.
#[allow(
    clippy::too_many_lines,
    reason = "exhaustive alternative-cost payment check"
)]
fn alternative_parts_payable(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    parts: &[baylee_cards_dsl::CostPart],
) -> bool {
    use baylee_cards_dsl::CostPart;
    if !pitch_costs_payable(state, player, card, parts, &[]) {
        return false;
    }
    parts.iter().all(|part| match part {
        CostPart::DiscardRandom => {
            state
                .zones
                .list(ZoneLocation::Hand(player))
                .iter()
                .filter(|id| **id != card)
                .count()
                >= parts
                    .iter()
                    .filter(|part| matches!(part, CostPart::DiscardRandom))
                    .count()
        }
        CostPart::PayLife(n) => state.can_pay_life(player, i32::from(*n)),
        CostPart::DiscardLastDrawnThisTurn => state.last_drawn_in_hand(player).is_some(),
        // The same question `can_afford` asks of an activation, asked of the
        // card about to be cast. It answers `false` for every card in hand,
        // because a card in a hand carries no counters — and that is the
        // truthful answer rather than a special case: an alternative cost
        // this pool does not print is not silently declared payable.
        CostPart::RemoveCounterSelf { kind, n } => state
            .object(card)
            .is_some_and(|o| o.counters.get(*kind) >= *n),
        CostPart::RemoveAnyCounter { .. } => {
            crate::engine::cost_wizard::counter_remove::can_afford_counter_parts(
                state,
                player,
                crate::eval::live_context(state, card),
                parts,
                0,
                &[],
            )
        }
        // The rest are payable, most of them because they are paid off the
        // card or the board rather than out of a count. `RemoveCounterSelfX`
        // joins them for its own reason: zero is a legal number to announce,
        // so there is nothing here an alternative cost could fail to pay.
        // Counted activation costs are not spell additional costs.
        CostPart::ExertSelf
        | CostPart::RevealChosenPlayer
        | CostPart::SacrificeGrantor
        | CostPart::PayEnergy(_)
        | CostPart::PayEnergyX
        | CostPart::TapX(_)
        | CostPart::SacrificeX(_)
        | CostPart::DiscardX(_)
        | CostPart::RevealFromHandX(_)
        | CostPart::ExileFromGraveyardX(_)
        | CostPart::SacrificeAll(_)
        | CostPart::PayCommanderColors
        | CostPart::PayAttachedManaCost
        | CostPart::ReturnUnblockedAttacker
        | CostPart::CraftMaterials { .. }
        | CostPart::ExileTopOfLibrary(_) => false,
        // `pitch_costs_payable` already proved a distinct card for every
        // exile-from-hand part; the rest are paid off the card or the board.
        CostPart::ExileFromHand(_)
        | CostPart::DiscardHand
        | CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::SacrificeSelf
        | CostPart::Sacrifice(_)
        | CostPart::ExilePermanent(_)
        | CostPart::ExileFromSingleGraveyard { .. }
        | CostPart::Discard(_)
        | CostPart::TapN { .. }
        | CostPart::TapOther(_)
        | CostPart::Crew(_)
        | CostPart::Saddle(_)
        | CostPart::ReturnToHand(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::DiscardSelf
        | CostPart::RevealSelfFromHand
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::RemoveAnyCounterX { .. }
        | CostPart::PutCounterSelf { .. }
        | CostPart::DealDamageToYou(_)
        | CostPart::Forage
        | CostPart::Waterbend(_)
        | CostPart::WaterbendX
        | CostPart::PayLifeX => true,
        CostPart::Behold(filter) | CostPart::BeholdAndExile(filter) => {
            !behold_options(state, player, card, filter).is_empty()
        }
        CostPart::PutCounterOnOther { .. } | CostPart::RevealFromHand(_) => {
            !crate::engine::cost_wizard::options(state, player, card, part).is_empty()
        }
        CostPart::CollectEvidence(mv) => {
            if *mv == 0 {
                true
            } else {
                let total: u32 = state
                    .zones
                    .list(ZoneLocation::Graveyard(player))
                    .iter()
                    .filter(|&&id| id != card)
                    .filter_map(|&id| state.object(id).filter(|o| o.is_card()))
                    .fold(0u32, |acc, o| {
                        acc.saturating_add(o.characteristics().mana_value())
                    });
                total >= u32::from(*mv)
            }
        }
        CostPart::Either(a, b) => {
            alternative_parts_payable(state, player, card, a.parts)
                || alternative_parts_payable(state, player, card, b.parts)
        }
    })
}

/// Eligible sacrifice candidates for `Emerge` (`CR 702.119a–c`), paired with
/// their current mana value, excluding any candidate whose choice would leave
/// mandatory additional costs unpayable.
pub(crate) fn emerge_candidates(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: &'static baylee_cards_dsl::FaceDef,
    emerge: baylee_cards_dsl::Emerge,
) -> Vec<(ObjectId, u32)> {
    crate::engine::cost_wizard::options(
        state,
        player,
        card,
        &baylee_cards_dsl::CostPart::Sacrifice(emerge.from),
    )
    .into_iter()
    .filter(|&candidate| {
        candidate != card
            && additional_payment_capacity_for(
                state,
                player,
                card,
                face,
                crate::engine::cast_wizard::CostChoice {
                    option: Some(crate::choice::CastModeKind::Emerge),
                    emerge_sacrifice: Some(candidate),
                    ..Default::default()
                },
                &[],
            )
            .is_some()
    })
    .map(|candidate| {
        let mv = state
            .object(candidate)
            .map_or(0, |o| o.characteristics().mana_value());
        (candidate, mv)
    })
    .collect()
}

/// Whether `player` is in the instant-speed window during their own declare
/// blockers step (`Sneak`, `CR 702.190a`).
#[must_use]
pub fn in_sneak_window(state: &GameState, player: PlayerId) -> bool {
    state.turn.active == player
        && state.turn.step == crate::turn::Step::DeclareBlockers
        && timing_allows(
            state,
            player,
            baylee_core::types::TypeSet::INSTANT,
            baylee_cards_dsl::KeywordSet::EMPTY,
        )
}

/// Effective `Sneak` cost (`CR 702.190a`) on `card`'s `face`, read from printed
/// stack-functioning statics or active continuous effects on `Layer::Ability`.
#[must_use]
pub fn sneak_cost_for(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: Option<&'static baylee_cards_dsl::CardDef>,
    face: usize,
) -> Option<baylee_cards_dsl::Cost> {
    let obj = state.object(card)?;
    let chars = obj.characteristics();
    if chars.abilities_lost.is_none()
        && !chars.rules_text_lost
        && let Some(def) = def
    {
        for ability in def.abilities_for_face(face) {
            if let baylee_cards_dsl::AbilityDef::Static(sa) = ability
                && Zone::Stack.matches_ref(sa.source_zone)
                && let baylee_cards_dsl::Modifier::Sneak(cost) = sa.modifier
                && sa
                    .condition
                    .is_none_or(|cond| crate::eval::condition_holds(state, player, card, cond))
            {
                return Some(cost);
            }
        }
    }
    state
        .effects
        .iter()
        .filter_map(|fx| {
            if let baylee_cards_dsl::Modifier::Sneak(cost) = fx.modifier
                && crate::effects::applies_to(state, fx, obj)
            {
                Some(cost)
            } else {
                None
            }
        })
        .last()
}

/// Eligible unblocked attacking creatures controlled by `player` that can be
/// returned to hand to pay `Sneak` (`CR 702.190a`) without leaving mandatory
/// additional costs unpayable.
pub(crate) fn sneak_candidates(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: &'static baylee_cards_dsl::FaceDef,
    sneak: baylee_cards_dsl::Cost,
) -> Vec<ObjectId> {
    crate::engine::cost_wizard::options(
        state,
        player,
        card,
        &baylee_cards_dsl::CostPart::ReturnUnblockedAttacker,
    )
    .into_iter()
    .filter(|&candidate| {
        candidate != card
            && additional_payment_capacity_for(
                state,
                player,
                card,
                face,
                crate::engine::cast_wizard::CostChoice {
                    option: Some(crate::choice::CastModeKind::Sneak),
                    returned: Some(candidate),
                    sneak_parts: sneak.parts,
                    ..Default::default()
                },
                &[],
            )
            .is_some()
    })
    .collect()
}

/// Casting/playing failure.
#[derive(Debug, thiserror::Error)]
pub enum CastFailure {
    /// Legality violation.
    #[error("illegal action: {0}")]
    Legality(#[from] CastError),
    /// Stale handle.
    #[error("object vanished")]
    NoSuchObject,
    /// Zone machinery error.
    #[error("state error: {0}")]
    State(#[from] StateError),
}
