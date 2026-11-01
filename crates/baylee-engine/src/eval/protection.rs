//! Protection, hexproof, shroud and enchant restrictions on what may point at an object.

use super::matches_with_context;
use crate::state::GameState;
use crate::text_changes::RuleContext;
use baylee_core::ids::{ObjectId, PlayerId};

/// Protection (CR 702.16): does `object` have protection from a filter
/// that `source` matches? Checked for damage, targeting, and blocking.
#[must_use]
pub fn protected_from(state: &GameState, object: ObjectId, source: ObjectId) -> bool {
    let Some(src) = state.object(source) else {
        return false;
    };
    protected_from_object(state, object, src)
}

pub(super) fn protected_from_object(
    state: &GameState,
    object: ObjectId,
    src: &crate::object::GameObject,
) -> bool {
    let Some(obj) = state.object(object) else {
        return false;
    };
    let source = src.id;
    state.effects.iter().any(|fx| match fx.modifier {
        baylee_cards_dsl::Modifier::ProtectionFrom(f) => {
            crate::effects::applies_to(state, fx, obj)
                && matches_with_context(
                    f,
                    state,
                    src,
                    fx.controller,
                    RuleContext {
                        source: fx.source.unwrap_or(source),
                        text: state.effect_text(fx),
                        resolved_name: None,
                        reference: None,
                    },
                )
        }
        baylee_cards_dsl::Modifier::ProtectionFromExiledWithSourceTypes => {
            protection_from_exiled_types(state, fx, obj, src)
        }
        _ => false,
    })
}

/// Mirror Golem's dynamic protection: any card type found among the cards
/// exiled with the effect's source, with exact host incarnations preserved.
pub(crate) fn protection_from_exiled_types(
    state: &GameState,
    fx: &crate::effects::ContinuousEffect,
    protected: &crate::object::GameObject,
    source: &crate::object::GameObject,
) -> bool {
    if !crate::effects::applies_to(state, fx, protected) {
        return false;
    }
    let host = fx.source.unwrap_or(source.id);
    let context = RuleContext {
        source: host,
        text: state.effect_text(fx),
        resolved_name: None,
        reference: None,
    };
    super::linked_exile_cards(
        state,
        fx.controller,
        context,
        None,
        &baylee_cards_dsl::Filter::Any,
    )
    .into_iter()
    .filter_map(|id| state.object(id))
    .any(|exiled| {
        exiled
            .characteristics()
            .types
            .intersects(source.characteristics().types)
    })
}

/// Does a static ability on `object` say it can't be the target of `source`
/// ([`baylee_cards_dsl::Modifier::CantBeTargetedBy`], Thrun, Breaker of
/// Silence)? `source` is the spell, or the source of the ability, doing the
/// targeting, and the filter is asked of it with the effect's controller as
/// "you" — the same reading [`protected_from`] gives protection, whose
/// targeting half this is. `CantBeTargetedByAbilitiesFrom` (Artifact Ward)
/// asks only of an ability's source: a spell is on the stack as it chooses
/// its targets (CR 601.2a, 601.2c), and an ability's source is not the
/// object on the stack.
#[must_use]
pub fn untargetable_by_source(state: &GameState, object: ObjectId, source: ObjectId) -> bool {
    let Some(src) = state.object(source) else {
        return false;
    };
    untargetable_by_source_object(state, object, src)
}

pub(super) fn untargetable_by_source_object(
    state: &GameState,
    object: ObjectId,
    src: &crate::object::GameObject,
) -> bool {
    let Some(obj) = state.object(object) else {
        return false;
    };
    let source = src.id;
    let spell = src.zone == crate::zone::Zone::Stack;
    state.effects.iter().any(|fx| {
        let f = match fx.modifier {
            baylee_cards_dsl::Modifier::CantBeTargetedBy(f) => f,
            baylee_cards_dsl::Modifier::CantBeTargetedByAbilitiesFrom(f) if !spell => f,
            _ => return false,
        };
        crate::effects::applies_to(state, fx, obj)
            && matches_with_context(
                f,
                state,
                src,
                fx.controller,
                RuleContext {
                    source: fx.source.unwrap_or(source),
                    text: state.effect_text(fx),
                    resolved_name: None,
                    reference: None,
                },
            )
    })
}

/// A rule permission with a continuing set: new opponents' creatures are
/// covered too, and losing the resolving source does not end the permission.
fn ignores_opponents_hexproof(state: &GameState, you: PlayerId) -> bool {
    state.effects.iter().any(|effect| {
        effect.controller == you
            && effect.modifier == baylee_cards_dsl::Modifier::IgnoreOpponentsHexproof
    })
}

/// Hexproof (CR 702.11b) and shroud (CR 702.18a): does `object` refuse to
/// be targeted by a spell or ability `you` control?
///
/// Both keywords function only while the object is on the battlefield —
/// a card in hand printed with hexproof is targetable in the graveyard,
/// and every other zone. Player hexproof is a different thing entirely
/// (`Modifier::PlayerHexproof`, checked when a spell chooses a player).
#[must_use]
pub fn untargetable_by(state: &GameState, object: ObjectId, you: PlayerId) -> bool {
    let Some(obj) = state.object(object) else {
        return false;
    };
    if obj.zone != crate::zone::Zone::Battlefield {
        return false;
    }
    let keywords = obj.characteristics().keywords;
    // Shroud stops everyone, its own controller included.
    if keywords.contains(baylee_cards_dsl::KeywordSet::SHROUD) {
        return true;
    }
    // Hexproof stops opponents only (CR 702.11b), so a teammate may
    // target it — and in a game with no teams that is everyone else.
    keywords.contains(baylee_cards_dsl::KeywordSet::HEXPROOF)
        && state.is_opponent(obj.controller, you)
        && !(obj
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::CREATURE)
            && ignores_opponents_hexproof(state, you))
}

/// Whether rules-modifying effects permit this Aura to enchant the host.
/// Targeting restrictions and the Aura's own enchant filter are separate.
#[must_use]
pub fn permits_enchantment(state: &GameState, host: ObjectId, aura: ObjectId) -> bool {
    let Some(object) = state.object(host) else {
        return false;
    };
    !state.effects.iter().any(|fx| {
        (match fx.modifier {
            baylee_cards_dsl::Modifier::CantBeEnchanted => true,
            baylee_cards_dsl::Modifier::CantBeEnchantedExceptSource => fx.source != Some(aura),
            _ => false,
        }) && crate::effects::applies_to(state, fx, object)
    })
}
