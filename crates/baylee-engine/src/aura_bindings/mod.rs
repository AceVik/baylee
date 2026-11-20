//! Aura attachment rules and exact reanimation relationships.
//!
//! Enchant legality is shared by entry, relocation and state-based actions.

use crate::object::{GameObject, Status};
use crate::state::GameState;
use crate::text_changes::RuleContext;
use crate::zone::Zone;
use baylee_cards_dsl::{AbilityDef, Effect, Filter, PlayerRel, TargetSpec};
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};
use baylee_core::types::TypeSet;

const ANY_TARGET_PERMANENT: Filter = Filter::HasType(
    TypeSet::CREATURE
        .union(TypeSet::PLANESWALKER)
        .union(TypeSet::BATTLE),
);

mod reanimation;
mod relocation;

pub(crate) use reanimation::{
    ReanimationFinish, begin_reanimation, finish_reanimation, sacrifice_event,
};
pub(crate) use relocation::{destroy_event_and_offer, resume_attach};

/// The changed enchant ability is tied to one Aura incarnation. `None`
/// still records that the enchant ability changed when returning the card
/// failed; the Aura then cannot satisfy that enchant ability (CR 303.4c).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ReanimatedAura {
    pub(crate) aura: DamageSourceRef,
    pub(crate) returned: Option<DamageSourceRef>,
}

/// Enchant is read from the same typed clause that supplied spell targets.
/// Its wording comes from that individual copied or printed ability.
#[derive(Clone, Copy, Debug)]
pub(crate) enum EnchantRestriction {
    Permanent(&'static Filter, RuleContext),
    Player(TargetSpec),
    PlayerOrPermanent {
        any_player: bool,
        filter: &'static Filter,
        context: RuleContext,
    },
    GraveyardCard(&'static Filter, PlayerRel, RuleContext),
    ReturnedBy(DamageSourceRef),
}

pub(crate) fn printed_restriction_with(
    state: &GameState,
    aura: &GameObject,
    lookup: &impl crate::state::CardLookup,
) -> Option<EnchantRestriction> {
    if aura.riders.contains(&crate::object::Rider::Bestowed) {
        return Some(EnchantRestriction::Permanent(
            &Filter::CREATURE,
            crate::eval::live_context(state, aura.id),
        ));
    }
    aura.ability_list(lookup)
        .abilities
        .iter()
        .enumerate()
        .find_map(|(index, ability)| {
            let AbilityDef::Spell { effects, .. } = ability else {
                return None;
            };
            let context = RuleContext {
                source: aura.id,
                text: state.ability_text(aura.id, u32::try_from(index).ok()?),
                resolved_name: None,
                resolved_subtype: None,
                reference: None,
            };
            effects.iter().find_map(|effect| match effect {
                Effect::AttachSelf {
                    target: TargetSpec::Object(filter),
                } => Some(EnchantRestriction::Permanent(filter, context)),
                Effect::AttachSelf {
                    target:
                        spec @ (TargetSpec::AnyPlayer
                        | TargetSpec::AnyOpponent
                        | TargetSpec::Player(_)
                        | TargetSpec::PlayerWhoAttackedThisTurn
                        | TargetSpec::OpponentWithMoreThanActivePlayer(_)
                        | TargetSpec::OpponentWithMoreThanPlayer(..)),
                } => Some(EnchantRestriction::Player(*spec)),
                Effect::AttachSelf {
                    target: TargetSpec::PlayerOrObject(filter),
                } => Some(EnchantRestriction::PlayerOrPermanent {
                    any_player: true,
                    filter,
                    context,
                }),
                Effect::AttachSelf {
                    target: TargetSpec::OpponentOrObject(filter),
                } => Some(EnchantRestriction::PlayerOrPermanent {
                    any_player: false,
                    filter,
                    context,
                }),
                Effect::AttachSelf {
                    target: TargetSpec::AnyTarget,
                } => Some(EnchantRestriction::PlayerOrPermanent {
                    any_player: true,
                    filter: &ANY_TARGET_PERMANENT,
                    context,
                }),
                Effect::AttachSelf {
                    target: TargetSpec::CardInGraveyard(filter, who),
                } => Some(EnchantRestriction::GraveyardCard(filter, *who, context)),
                _ => None,
            })
        })
}

/// A changed enchant ability follows this Aura incarnation, never a new
/// copy or a later battlefield incarnation of the same physical card.
pub(crate) fn restriction(state: &GameState, aura: &GameObject) -> Option<EnchantRestriction> {
    restriction_with(state, aura, state.bases.as_ref())
}

pub(crate) fn restriction_with(
    state: &GameState,
    aura: &GameObject,
    lookup: &impl crate::state::CardLookup,
) -> Option<EnchantRestriction> {
    state
        .reanimated_auras
        .iter()
        .find_map(|binding| {
            (binding.aura.object == aura.id && binding.aura.version == aura.version)
                .then_some(EnchantRestriction::ReturnedBy(binding.aura))
        })
        .or_else(|| printed_restriction_with(state, aura, lookup))
}

/// The shared attachment rule, also usable before the Aura enters.
pub(crate) fn can_attach(state: &GameState, aura: ObjectId, host: ObjectId) -> bool {
    let Some(object) = state.object(aura) else {
        return false;
    };
    let restriction = restriction(state, object).unwrap_or(EnchantRestriction::Permanent(
        &Filter::Any,
        crate::eval::live_context(state, aura),
    ));
    crate::permanent_guards::permits_new_enchantment(state, host, aura)
        && can_enchant(state, aura, host, restriction)
}

/// Whether `aura` may attach to `player` (CR 303.4f, CR 303.4j, CR 702.16c).
/// Untargeted Aura entry and attachment checks ignore hexproof/shroud, asking
/// only the Aura's own enchant restriction and protection.
pub(crate) fn can_attach_player(state: &GameState, aura: ObjectId, player: PlayerId) -> bool {
    let Some(object) = state.object(aura) else {
        return false;
    };
    let Some(restriction) = restriction(state, object) else {
        return false;
    };
    can_enchant_player(state, aura, player, restriction)
}

/// Whether `aura` may enchant `player` under `restriction` (CR 303.4c, CR 702.16c).
pub(crate) fn can_enchant_player(
    state: &GameState,
    aura: ObjectId,
    player: PlayerId,
    restriction: EnchantRestriction,
) -> bool {
    let Some(aura_obj) = state.object(aura) else {
        return false;
    };
    if state.has_left(player)
        || aura_obj
            .characteristics()
            .types
            .intersects(TypeSet::CREATURE.union(TypeSet::BATTLE))
        || aura_obj.status.contains(Status::PHASED_OUT)
    {
        return false;
    }
    let permitted = match restriction {
        EnchantRestriction::Player(spec) => match spec {
            TargetSpec::AnyPlayer => true,
            TargetSpec::AnyOpponent => state.is_opponent(player, aura_obj.controller),
            TargetSpec::Player(rel) => {
                crate::eval::players_for_source(rel, state, aura_obj.controller, Some(aura))
                    .is_some_and(|seats| seats.contains(&player))
            }
            TargetSpec::PlayerWhoAttackedThisTurn => state
                .per_turn
                .attacked
                .iter()
                .any(|&(_, _, who)| who == player),
            TargetSpec::OpponentWithMoreThanActivePlayer(_) => {
                state.is_opponent(player, aura_obj.controller)
            }
            TargetSpec::OpponentWithMoreThanPlayer(filter, reference) => {
                let context = crate::eval::live_context(state, aura);
                state.is_opponent(player, reference)
                    && crate::eval::count_controlled_by(
                        state,
                        player,
                        aura_obj.controller,
                        filter,
                        context,
                    ) > crate::eval::count_controlled_by(
                        state,
                        reference,
                        aura_obj.controller,
                        filter,
                        context,
                    )
            }
            _ => false,
        },
        EnchantRestriction::PlayerOrPermanent { any_player, .. } => {
            any_player || state.is_opponent(player, aura_obj.controller)
        }
        EnchantRestriction::Permanent(..)
        | EnchantRestriction::GraveyardCard(..)
        | EnchantRestriction::ReturnedBy(_) => false,
    };
    permitted && !crate::eval::player_protected_from(state, player, aura)
}

/// Attachment legality is distinct from targeting. Shroud and hexproof do
/// not prohibit Kudzu's untargeted relocation (CR 303.4j, 701.3b).
pub(crate) fn can_enchant(
    state: &GameState,
    aura: ObjectId,
    host: ObjectId,
    restriction: EnchantRestriction,
) -> bool {
    let (Some(aura), Some(host)) = (state.object(aura), state.object(host)) else {
        return false;
    };
    if aura.id == host.id
        || aura
            .characteristics()
            .types
            .intersects(TypeSet::CREATURE.union(TypeSet::BATTLE))
        || aura.status.contains(Status::PHASED_OUT)
        || host.status.contains(Status::PHASED_OUT)
    {
        return false;
    }
    let permitted = match restriction {
        EnchantRestriction::Permanent(filter, context)
        | EnchantRestriction::PlayerOrPermanent {
            filter, context, ..
        } => {
            host.zone == Zone::Battlefield
                && crate::eval::matches_with_context(filter, state, host, aura.controller, context)
        }
        EnchantRestriction::Player(_) => false,
        EnchantRestriction::GraveyardCard(filter, who, context) => {
            host.zone == Zone::Graveyard
                && host.card.is_some()
                && match who {
                    PlayerRel::EachPlayer => true,
                    PlayerRel::You => host.owner == aura.controller,
                    PlayerRel::Opponent | PlayerRel::EachOpponent => {
                        state.is_opponent(host.owner, aura.controller)
                    }
                    _ => false,
                }
                && crate::eval::matches_with_context(filter, state, host, aura.controller, context)
        }
        EnchantRestriction::ReturnedBy(source) => {
            host.zone == Zone::Battlefield
                && host.characteristics().types.contains(TypeSet::CREATURE)
                && state.reanimated_auras.iter().any(|binding| {
                    binding.aura == source
                        && binding.returned.is_some_and(|reference| {
                            reference.object == host.id && reference.version == host.version
                        })
                })
        }
    };
    permitted
        && (host.zone != Zone::Battlefield
            || (!crate::eval::protected_from(state, host.id, aura.id)
                && crate::eval::permits_enchantment(state, host.id, aura.id)))
}
