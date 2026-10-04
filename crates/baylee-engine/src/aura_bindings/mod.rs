//! Aura attachment rules and exact reanimation relationships.
//!
//! Enchant legality is shared by entry, relocation and state-based actions.

use crate::object::{GameObject, Status};
use crate::state::GameState;
use crate::text_changes::RuleContext;
use crate::zone::Zone;
use baylee_cards_dsl::{AbilityDef, Effect, Filter, PlayerRel, TargetSpec};
use baylee_core::ids::{DamageSourceRef, ObjectId};
use baylee_core::types::TypeSet;

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
    GraveyardCard(&'static Filter, PlayerRel, RuleContext),
    ReturnedBy(DamageSourceRef),
}

pub(crate) fn printed_restriction(
    state: &GameState,
    aura: &GameObject,
) -> Option<EnchantRestriction> {
    aura.ability_list(state.bases.as_ref())
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
            };
            effects.iter().find_map(|effect| match effect {
                Effect::AttachSelf {
                    target: TargetSpec::Object(filter),
                } => Some(EnchantRestriction::Permanent(filter, context)),
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
    state
        .reanimated_auras
        .iter()
        .find_map(|binding| {
            (binding.aura.object == aura.id && binding.aura.version == aura.version)
                .then_some(EnchantRestriction::ReturnedBy(binding.aura))
        })
        .or_else(|| printed_restriction(state, aura))
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
    can_enchant(state, aura, host, restriction)
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
        EnchantRestriction::Permanent(filter, context) => {
            host.zone == Zone::Battlefield
                && crate::eval::matches_with_context(filter, state, host, aura.controller, context)
        }
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
