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
mod escape;
mod intrinsic;
mod lands;
pub(crate) use additional::{
    additional_card_enters_graveyard, additional_payment_capacity, additional_payment_capacity_for,
    pitch_costs_payable,
};
mod payment;
mod permissions;
mod player_locks;
mod pricing;
mod source_cost;
#[cfg(test)]
pub(crate) use pricing::spell_increase;
pub(crate) use source_cost::*;
#[cfg(test)]
mod tests;

pub(crate) use escape::{escape_grants, granted_escape_options, mana_cost_mode};
pub use intrinsic::*;
pub use lands::*;
pub(crate) use payment::*;
pub(crate) use permissions::*;
pub use pricing::{
    ability_mana_spending, commander_tax, convoke_sources, delve_sources, has_improvise,
    keyword_reduction, mana_is_wild, mana_spending, printed_reduction, reduction_amount,
    tap_to_pay_sources, waterbend_sources,
};
pub(crate) use pricing::{
    activation_increase, component_affordable, face_adjustment, face_increase,
    normal_spell_affordable, north_star_effect, north_star_face_allowed, spell_adjustment,
    spell_mana_spending,
};

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

/// An independent permission can cast the Adventure again (CR 715.3d).
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

/// Native disturb is available only when its graveyard permission is active.
/// Ordinary callers still exclude it from hand, exile and free-cast menus.
fn castable_back_faces_in_zone(
    def: &baylee_cards_dsl::CardDef,
    on_adventure: bool,
    disturb: bool,
) -> impl Iterator<Item = (usize, &baylee_cards_dsl::FaceDef)> {
    def.faces
        .iter()
        .enumerate()
        .skip(1)
        .filter(move |(_, face)| {
            (face.castable_from_hand || disturb && face.disturb)
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
/// (CR 715) is a *different spell* with its own target line on the back of a
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
    let mut modal = abilities.iter().filter_map(|a| match a {
        baylee_cards_dsl::AbilityDef::ModalSpell { modes, choose } => Some((modes, choose)),
        _ => None,
    });
    if let Some((modes, choose)) = modal.next() {
        return mode_sets(modes, *choose).any(|set| {
            let wall_only = crate::eval::target_domain::modes_wall_only(modes, set);
            chosen_modes(modes, set).all(|(_, mode)| {
                [mode.targets, mode.second_targets].into_iter().all(|req| {
                    requirement_is_reachable_for(
                        req,
                        state,
                        player,
                        card,
                        Some((def, face)),
                        wall_only,
                    )
                })
            })
        });
    }

    let spell = abilities.iter().find_map(|a| match a {
        baylee_cards_dsl::AbilityDef::Spell {
            targets,
            second_targets,
            ..
        } => Some((*targets, *second_targets)),
        _ => None,
    });
    let (req, second) = spell.unwrap_or_default();
    // Every instance of the word has to be satisfiable (CR 601.2c): Khalni
    // Ambush with no creature on the other side of the table is not a spell
    // that can be cast, however many of the caster's own it could name.
    (requirement_is_reachable_for(
        req,
        state,
        player,
        card,
        Some((def, face)),
        crate::eval::target_domain::walls(req, second),
    ) || def
        .faces
        .get(face)
        .and_then(|f| f.kicked_targets)
        .is_some_and(|req| {
            requirement_is_reachable_for(
                Some(req),
                state,
                player,
                card,
                Some((def, face)),
                crate::eval::target_domain::walls(Some(req), second),
            )
        }))
        && second_is_reachable(
            req,
            second,
            state,
            player,
            card,
            Some((def, face)),
            crate::eval::target_domain::walls(req, second),
        )
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
            ..
        } => requirements_are_reachable(*targets, *second_targets, state, player, card),
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
    let reqs = def.abilities.iter().find_map(|a| match a {
        baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. } => {
            modes.get(mode).map(|m| (m.targets, m.second_targets))
        }
        _ => None,
    });
    // Both instances of the word, as `face_has_a_legal_target` asks them
    // (CR 601.2c): Archdruid's Charm's second mode with no creature on the
    // other side of the table is not a mode that can be chosen (CR 700.2a).
    match reqs {
        Some((first, second)) => requirements_are_reachable(first, second, state, player, card),
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
        .filter_map(|ability| match ability {
            baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. } => Some(*modes),
            _ => None,
        })
        .all(|modes| {
            let wall_only = crate::eval::target_domain::modes_wall_only(modes, set);
            chosen_modes(modes, set).all(|(_, mode)| {
                [mode.targets, mode.second_targets].into_iter().all(|req| {
                    requirement_is_reachable_for(req, state, player, card, None, wall_only)
                })
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
    abilities
        .iter()
        .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::ModalSpell { .. }))
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
    let wall_only = crate::eval::target_domain::walls(first, second);
    requirement_is_reachable_for(first, state, player, card, None, wall_only)
        && second_is_reachable(first, second, state, player, card, None, wall_only)
}

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

/// Kicker replaces the first targeting group, retaining the second group.
pub(crate) fn kicked_targets_reachable(
    state: &GameState,
    def: &baylee_cards_dsl::CardDef,
    player: PlayerId,
    card: ObjectId,
    first: baylee_cards_dsl::TargetReq,
) -> bool {
    let second = def.abilities.iter().find_map(|ability| match ability {
        baylee_cards_dsl::AbilityDef::Spell { second_targets, .. } => *second_targets,
        _ => None,
    });
    requirements_are_reachable(Some(first), second, state, player, card)
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
    let objects = match x_bounded(req.spec) {
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
        .count(),
        None => crate::eval::spell_target_options_for(
            &req.spec, state, player, card, face, None, wall_only,
        )
        .len(),
    };
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
pub(crate) fn timing_allows(
    state: &GameState,
    player: PlayerId,
    types: TypeSet,
    keywords: baylee_cards_dsl::KeywordSet,
) -> bool {
    // Teferi's restriction forces sorcery-speed timing on everything for
    // opponents.
    let teferi_lock = state.effects.may_change_cast_timing_or_lock()
        && state.effects.iter().any(|fx| {
            matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::OpponentsCastAsSorcery
            ) && state.is_opponent(fx.controller, player)
        });
    // Flash (CR 702.8a) makes a card castable whenever an instant could be
    // — Snapcaster Mage, Restoration Angel, the whole free-spell cycle.
    let is_instant =
        types.contains(TypeSet::INSTANT) || keywords.contains(baylee_cards_dsl::KeywordSet::FLASH);
    let granted_flash = !is_instant
        && state.effects.may_change_cast_timing_or_lock()
        && state.effects.iter().any(|fx| {
            fx.controller == player
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

/// Whether `player` may cast back face `face` of `def` now, as far as timing
/// goes (CR 601.3).
///
/// A face that is cast in place of the front is judged by its own
/// characteristics (CR 601.3e): only the face that will be up on the stack
/// is evaluated for an MDFC (CR 712.11c), and only the alternative
/// characteristics for an Adventure (CR 715.3a). So an instant printed on
/// the back of a creature or an enchantment is cast whenever an instant
/// could be, and the front is not. Read off the printed face: nothing can
/// have granted a face that is not up anything, and the two player-scoped
/// effects `timing_allows` reads still apply.
pub(crate) fn face_timing_allows(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
) -> bool {
    def.faces.get(face).is_some_and(|f| {
        timing_allows(state, player, f.types, def.keywords_for_face(face))
            && permitted_spell_types(state, player, card, f.types, face)
    }) && spell_condition_allows(state, player, card, def, face)
}

/// "Cast this spell only [when]" (CR 506.7; Berserk's "only before the
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
                crate::eval::players(who, state, fx.controller)
                    .is_some_and(|players| players.contains(&player))
                    && state.per_turn.spells_cast_by(player) >= limit
            }
            baylee_cards_dsl::Modifier::PlayersCantCast { who, filter } => {
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
/// (CR 601.2h, 732.1).
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
    let mut best = can_cast_form_paying(state, lookup, player, card, None, paying);
    if best.is_ok() {
        return best;
    }
    if let Some(face) = state
        .object(card)
        .and_then(|o| o.card)
        .and_then(|c| lookup.card(c.index))
        .and_then(|d| d.faces.first())
    {
        for form in [
            face.prototype.map(SpellForm::Prototype),
            (face.disguise.is_some() || face.morph.is_some()).then_some(SpellForm::Disguise),
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
    best
}

/// Whether `player` may begin to cast a spell at all this turn (CR 601.3).
///
/// Conduit of Worlds' "If you do, you can't cast additional spells this
/// turn" is the one sentence that says no: it names the player and every
/// spell, so it is asked here, beside the table's other refusals, by every
/// door a cast comes through — the priority offer, a free cast an effect
/// makes (cascade, rebound, suspend), a miracle, a prepared copy.
#[must_use]
pub(crate) fn may_begin_casting(state: &GameState, player: PlayerId) -> bool {
    !state
        .per_turn
        .no_more_spells
        .get(player.get() as usize)
        .copied()
        .unwrap_or(false)
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
    // Disturb (CR 702.146): a face with disturb is castable from the
    // owner's graveyard.
    let disturb_ok = !in_hand
        && in_own_graveyard
        && obj
            .card
            .and_then(|c| lookup.card(c.index))
            .is_some_and(|def| def.faces.iter().any(|f| f.disturb));
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
    // Adventure (CR 715): a card on an adventure may be cast from exile.
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
    let form = form.or_else(|| {
        (foretell_ok && permission.is_none())
            .then(|| SpellForm::Foretell(foretell_cost.expect("foretell permission has a cost")))
    });
    let granted_escape =
        in_own_graveyard && !granted_escape_options(state, lookup, player, card, paying).is_empty();
    // Wrenn's emblem and Muldrotha: a permanent card cast from its owner's
    // graveyard at its own price.
    let graveyard_ok = !in_hand && graveyard_cast_permission(state, player, obj).is_some();
    // A granted flashback cost cannot be combined with a second alternative cost.
    let flashback_only =
        in_own_graveyard && flashback_granted(state, card) && !graveyard_ok && permission.is_none();
    if flashback_only && matches!(form, Some(SpellForm::Disguise)) {
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
        && !escape_ok
        && !beam_ok
        && !granted_escape
        && !disturb_ok
        && !adventure_ok
        && !takeover_ok
        && !commander_ok
        && !foretell_ok
        && permission.is_none()
    {
        return Err(CastError::NotInHand);
    }
    let projected = form.map(|f| f.project(obj));
    let obj = projected.as_ref().unwrap_or(obj);
    let c = obj.characteristics();
    // A cast an effect forbids outright (CR 601.3a), asked before timing
    // because the two answers are different and only one of them is true:
    // Silence does not move a spell to sorcery speed, it removes the
    // permission, and a player reading "sorcery-speed timing not met" on
    // their own main phase would go looking for a rule that is not there.
    if cast_is_forbidden(state, player, obj) || !may_begin_casting(state, player) {
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
    let printed = obj.card.and_then(|c| lookup.card(c.index));
    let front_now = !c.types.contains(TypeSet::LAND)
        && graveyard_face_open(state, player, card, 0)
        && permitted_spell_types(state, player, card, c.types, 0)
        && timing_allows(state, player, c.types, c.keywords)
        && printed.is_none_or(|def| spell_condition_allows(state, player, card, def, 0));
    let a_back_face_now = printed.is_some_and(|def| {
        castable_back_faces_in_zone(def, on_adventure, disturb_ok)
            .any(|(i, _)| face_timing_allows(state, player, card, def, i))
    });
    if !front_now && (form.is_some() || !a_back_face_now) {
        return Err(CastError::BadTiming);
    }
    // "As an additional cost to cast this spell, sacrifice a creature": a
    // board with nothing to sacrifice cannot pay it, so the spell is not a
    // cast this player can make (CR 601.2h). The reader is the one the cast
    // wizard's `Sacrifice` stage builds its question from, so the offer and
    // the question cannot disagree — Force of Will's pitch is the precedent.
    if let Some(def) = obj.card.and_then(|card| lookup.card(card.index))
        && additional_payment_capacity(state, player, card, &def.faces[0], &[]).is_none()
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
    let adjustment = spell_adjustment(state, player, obj);
    let tax = adjustment.increase;
    if let Some(form) = form {
        return if affordable(
            state,
            player,
            pool,
            &adjustment.price(form.cost().with_x(0), 0),
            SpendFor::SpellAs(card, form),
            adjustment.allowance(state, SpendFor::SpellAs(card, form), 0),
        ) || matches!(form, SpellForm::Prototype(_))
            && north_star_face_allowed(state, player, card, 0)
            && normal_spell_affordable(
                state,
                player,
                SpendFor::SpellAs(card, form),
                pool,
                &adjustment.price(form.cost(), 0),
                &form.cost(),
                adjustment.allowance(state, SpendFor::SpellAs(card, form), 0),
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
    let probe = |cost: &ManaCost| {
        affordable(
            state,
            player,
            pool,
            &cost
                .combine(&adjustment.mana_increase)
                .with_more_generic(tax)
                .with_less_generic(reduction),
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
                    &cost
                        .combine(&adjustment.mana_increase)
                        .with_more_generic(tax)
                        .with_less_generic(reduction),
                    &c.mana_cost,
                    allowance,
                )
    };
    // A waived mana cost still pays increases (CR 601.2f).
    if permission.is_some_and(|p| p.free) {
        let free_base = printed_face
            .and_then(|f| f.behold_or_pay)
            .filter(|b| behold_options(state, player, card, b.filter).is_empty())
            .map_or(ManaCost::ZERO, |b| b.cost);
        let payable = if let Some(def) = printed.filter(|def| modes_are_the_only_way(def, 0)) {
            def.abilities_for_face(0).iter().any(|a| match a {
                baylee_cards_dsl::AbilityDef::ModalSpell { modes, choose } if !choose.is_one() => {
                    mode_sets(modes, *choose).any(|set| {
                        probe(&mode_set_cost(free_base, modes, set))
                            && mode_set_has_legal_targets(state, lookup, player, card, set)
                    })
                }
                baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. } => {
                    probe(&free_base)
                        && modes.iter().enumerate().any(|(i, mode)| {
                            mode.cost_override.is_none()
                                && mode_has_a_legal_target(state, lookup, player, card, i)
                        })
                }
                _ => false,
            })
        } else {
            probe(&free_base)
        };
        let back_payable = printed.is_some_and(|def| {
            castable_back_faces(def, on_adventure).any(|(face, _)| {
                face_timing_allows(state, player, card, def, face)
                    && face_has_a_legal_target(state, lookup, player, card, face)
                    && free_face_affordable(state, player, card, def, face)
            })
        });
        return if (front_now && payable) || back_payable {
            Ok(())
        } else {
            Err(CastError::NotEnoughMana)
        };
    }
    let probe_back = |def: &baylee_cards_dsl::CardDef, i: usize| {
        let restricted = planning_pool(state, player, SpendFor::SpellOnFace(card, i), 0);
        let pool = restricted
            .as_ref()
            .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
        let face = &def.faces[i];
        let adjustment = face_adjustment(state, player, card, def, i, None);
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
        let cost = face
            .mana_cost
            .with_x(0)
            .combine(&adjustment.mana_increase)
            .with_more_generic(tax)
            .with_less_generic(reduction);
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
                face_timing_allows(state, player, card, def, i)
                    && graveyard_face_open(state, player, card, i)
                    && probe_back(def, i)
                    && face_has_a_legal_target(state, lookup, player, card, i)
            })
        })
    };
    // The front may not be cast now, so no front-face price is an answer:
    // only a back face can make the card castable.
    if !front_now {
        return if a_back_face_castable() {
            Ok(())
        } else {
            Err(CastError::NotEnoughMana)
        };
    }
    // A printed flashback is paid "rather than its mana cost" (CR 702.34a),
    if beam_ok && beam_cost.is_some_and(|cost| probe(&cost)) {
        return Ok(());
    }
    if in_own_graveyard && beam_cost.is_some() && !graveyard_ok && !flashback_ok && !escape_ok {
        return Err(CastError::NoWayToCast);
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
        if !flashback_granted(state, card) && !graveyard_ok {
            return Err(CastError::NotEnoughMana);
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
        let restricted = planning_pool(state, player, SpendFor::KickedSpell(card, 0, form), 0);
        let kicked_pool = restricted
            .as_ref()
            .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
        let reachable = face.kicked_targets.map_or_else(
            || face_has_a_legal_target(state, lookup, player, card, 0),
            |req| kicked_targets_reachable(state, def, player, card, req),
        );
        let kicked_cost = kicked_mana_cost(face)
            .with_x(0)
            .combine(&adjustment.mana_increase)
            .with_more_generic(tax)
            .with_less_generic(reduction);
        if reachable
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
        && let Some(req) = def.faces[0].kicked_targets
    {
        let ordinary = ordinary_targets_reachable(state, def, player, card)
            && probe_normal(&c.mana_cost.with_x(0));
        let kicked = kicked_targets_reachable(state, def, player, card, req)
            && probe_normal(&kicked_mana_cost(&def.faces[0]).with_x(0));
        return if ordinary || kicked {
            Ok(())
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
        let affordable_disturb = printed.is_some_and(|def| {
            def.faces.iter().enumerate().skip(1).any(|(i, f)| {
                f.disturb
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
            return Err(CastError::NotEnoughMana);
        }
    }
    // Printed cost probed with X = 0, and after a reduction printed on the
    // card itself; the full payment is validated when the wizard finishes.
    // A face with no printed cost has no normal way to be cast at all
    // (CR 202.1b) and falls straight through to the alternatives.
    let normal_cost = if let Some(behold) = printed_face.and_then(|f| f.behold_or_pay)
        && behold_options(state, player, card, behold.filter).is_empty()
    {
        c.mana_cost.combine(&behold.cost)
    } else {
        c.mana_cost
    };
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
    if modal_only || !has_a_printed_cost(&c.mana_cost) || !probe_normal(&normal_cost.with_x(0)) {
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
        let any_alt = !flashback_only
            && face.alternative_costs.iter().enumerate().any(|(i, alt)| {
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
            });
        // Affordable *and* pointable, of the **same** mode. A mode's price
        // and a mode's target line are the two halves of one way to cast the
        // card, and this probe used to ask only the first while
        // `has_a_legal_target` asked the second of whichever mode happened to
        // answer yes — so a board where one mode was payable and a *different*
        // one was targetable offered a card the wizard then refused. It is the
        // same intersection `cast_options` makes two files away, which is the
        // half that was already right.
        let any_mode = def.abilities.iter().any(|a| match a {
            baylee_cards_dsl::AbilityDef::ModalSpell { modes, choose } if !choose.is_one() => {
                mode_sets(modes, *choose).any(|set| {
                    (!flashback_only
                        || mana_cost_mode(def, 0, crate::choice::CastModeKind::Modes(set)))
                        && probe_normal(&mode_set_cost(face.mana_cost, modes, set).with_x(0))
                        && mode_set_has_legal_targets(state, lookup, player, card, set)
                })
            }
            baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. } => {
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
            _ => false,
        });
        // And every other face the wizard would offer, which this probe knew
        // nothing about. Twining Twins is a `{2}{U}{U}` creature in front of
        // a `{1}{W}` instant, so the adventure was unreachable on exactly the
        // boards it is for: the ones where the creature cannot be paid for.
        //
        // Each face at its own timing (`face_timing_allows`), here and in
        // the wizard: a `{1}{W}` instant on the back of a creature is cast
        // whenever an instant could be, and a sorcery on the back of a
        // creature with flash is not.
        //
        // The target line is asked per face and beside the price, because
        // both have to hold of the *same* face for it to be a way of casting
        // the card. What is still coarse is the other direction:
        // `compute_legal` asks `has_a_legal_target` about face 0 before it
        // gets here, so a front that cannot be targeted keeps a targetable
        // back off the offer. No card in the pool is that shape, and closing
        // it means folding the face-0 question into the probes below and
        // giving `CastError` a variant that does not call a target problem
        // "not enough mana".
        let any_face = a_back_face_castable();
        // Dash (CR 702.109a), the offer's `CastModeKind::Dash`.
        let any_dash = !flashback_only && face.dash.is_some_and(|dash| probe(&dash));
        // Mutate (CR 702.140a), the offer's `CastModeKind::Mutate`.
        let any_mutate = !flashback_only
            && face.mutate.is_some_and(|mutate| {
                kicked_targets_reachable(state, def, player, card, MUTATE_TARGET_REQ)
                    && probe(&mutate)
            });
        if !any_alt && !any_mode && !any_face && !any_dash && !any_mutate {
            // Which of the two refused matters to whoever reads it. A mode
            // that was affordable and had nothing to point at is not a
            // player one land short, and telling them it is sends them
            // looking for the land.
            let a_mode_was_affordable = def.abilities.iter().any(|a| match a {
                baylee_cards_dsl::AbilityDef::ModalSpell { modes, choose } if !choose.is_one() => {
                    mode_sets(modes, *choose).any(|set| {
                        probe_normal(&mode_set_cost(face.mana_cost, modes, set).with_x(0))
                    })
                }
                baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. } => modes.iter().any(|m| {
                    if m.cost_override.is_some() {
                        probe(&m.cost_override.unwrap_or(face.mana_cost).with_x(0))
                    } else {
                        probe_normal(&face.mana_cost.with_x(0))
                    }
                }),
                _ => false,
            });
            return Err(if a_mode_was_affordable {
                CastError::NoWayToCast
            } else {
                CastError::NotEnoughMana
            });
        }
    }
    Ok(())
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

/// The minimum printed mana cost of `face` when cast by `player`: includes
/// `behold_or_pay`'s mana cost when `player` has nothing matching to behold.
#[must_use]
pub fn face_base_mana_cost(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: &baylee_cards_dsl::FaceDef,
) -> ManaCost {
    if let Some(behold) = face.behold_or_pay
        && behold_options(state, player, card, behold.filter).is_empty()
    {
        face.mana_cost.combine(&behold.cost)
    } else {
        face.mana_cost
    }
}

/// Whether the non-mana parts of an alternative cost could be paid right now.
///
/// [`can_cast`] reaches this only when the printed cost is unaffordable and
/// the face prints an alternative cost, which in this pool is four cards, so
/// the hand scan costs nothing anybody can measure. `zones.list` is ordered,
/// so it costs no determinism either.
///
/// The nine parts that answer `true` are named rather than swept into a
/// wildcard: `cast_wizard::paid_as_an_alternative_cost` says which two of
/// them the payment ever pays, and `offer_tests` holds the rest off this list
/// entirely — so a twelfth `CostPart` has to be looked at here too rather
/// than being quietly declared payable.
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
        // The rest are payable, most of them because they are paid off the
        // card or the board rather than out of a count. `RemoveCounterSelfX`
        // joins them for its own reason: zero is a legal number to announce,
        // so there is nothing here an alternative cost could fail to pay.
        // Counted activation costs are not spell additional costs.
        CostPart::ExertSelf
        | CostPart::PayEnergy(_)
        | CostPart::PayEnergyX
        | CostPart::TapX(_)
        | CostPart::SacrificeX(_)
        | CostPart::SacrificeAll(_)
        | CostPart::PayCommanderColors
        | CostPart::PayAttachedManaCost
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
        | CostPart::PutCounterOnOther { .. }
        | CostPart::ExileFromSingleGraveyard { .. }
        | CostPart::Discard(_)
        | CostPart::TapN { .. }
        | CostPart::TapOther(_)
        | CostPart::Crew(_)
        | CostPart::Saddle(_)
        | CostPart::ReturnToHand(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. }
        | CostPart::PayLifeX => true,
    })
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
