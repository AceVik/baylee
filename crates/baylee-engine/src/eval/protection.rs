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

pub(crate) fn protected_from_object(
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
                        resolved_subtype: None,
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
        resolved_subtype: None,
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
    let is_spell = src.zone == crate::zone::Zone::Stack
        && src.kind != crate::object::ObjectKind::AbilityOnStack;
    untargetable_by_source_with(state, object, src.controller, src, is_spell)
}

/// Targeting prohibitions and conditional hexproof (CR 702.11d) checked against
/// both the chooser (`you`) and the spell or ability source (`src`, `is_spell`).
pub(super) fn untargetable_by_source_with(
    state: &GameState,
    object: ObjectId,
    you: PlayerId,
    src: &crate::object::GameObject,
    is_spell: bool,
) -> bool {
    use baylee_cards_dsl::Modifier;
    let Some(obj) = state.object(object) else {
        return false;
    };
    let source = src.id;
    let on_battlefield = obj.zone == crate::zone::Zone::Battlefield;
    let cond_hexproof_active = on_battlefield
        && state.is_opponent(obj.controller, you)
        && !cant_have_hexproof(state, obj)
        && !ignores_hexproof(state, obj, you);
    state.effects.iter().any(|fx| {
        let ctx = || RuleContext {
            source: fx.source.unwrap_or(source),
            text: state.effect_text(fx),
            resolved_name: None,
            resolved_subtype: None,
            reference: None,
        };
        match fx.modifier {
            Modifier::CantBeTargetedBy(f) => {
                crate::effects::applies_to(state, fx, obj)
                    && matches_with_context(f, state, src, fx.controller, ctx())
            }
            Modifier::CantBeTargetedByAbilitiesFrom(f) => {
                !is_spell
                    && crate::effects::applies_to(state, fx, obj)
                    && matches_with_context(f, state, src, fx.controller, ctx())
            }
            Modifier::HexproofFrom(f) => {
                cond_hexproof_active
                    && crate::effects::applies_to(state, fx, obj)
                    && matches_with_context(f, state, src, fx.controller, ctx())
            }
            Modifier::HexproofFromOwnColors => {
                cond_hexproof_active
                    && crate::effects::applies_to(state, fx, obj)
                    && src
                        .characteristics()
                        .colors
                        .intersects(obj.characteristics().colors)
            }
            Modifier::HexproofFromAbilities => {
                cond_hexproof_active && !is_spell && crate::effects::applies_to(state, fx, obj)
            }
            _ => false,
        }
    })
}

/// Whether a continuous effect removes hexproof from `obj` (`CR 702.11f`), which
/// suppresses both printed hexproof and conditional "hexproof from [quality]".
fn cant_have_hexproof(state: &GameState, obj: &crate::object::GameObject) -> bool {
    state.effects.iter().any(|fx| {
        matches!(
            fx.modifier,
            baylee_cards_dsl::Modifier::CantHaveKeywords(bits)
                if bits.contains(baylee_cards_dsl::KeywordSet::HEXPROOF)
        ) && crate::effects::applies_to(state, fx, obj)
    })
}

/// As-though permissions leave the keyword intact. Filtered permissions apply
/// to every targeting player; the older permission belongs to its controller.
fn ignores_hexproof(state: &GameState, object: &crate::object::GameObject, you: PlayerId) -> bool {
    use baylee_cards_dsl::Modifier;
    state.effects.iter().any(|effect| match effect.modifier {
        Modifier::IgnoreOpponentsHexproof => {
            effect.controller == you
                && object
                    .characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
                && crate::effects::source_is_present(state, effect)
        }
        Modifier::IgnoreOpponentsHexproofMatching {
            permanent_filter: filter,
            ..
        } => {
            effect.controller == you
                && crate::effects::source_is_present(state, effect)
                && matches_with_context(
                    filter,
                    state,
                    object,
                    effect.controller,
                    RuleContext {
                        source: effect.source.unwrap_or(object.id),
                        text: state.effect_text(effect),
                        resolved_name: None,
                        resolved_subtype: None,
                        reference: None,
                    },
                )
        }
        Modifier::CanBeTargetedAsThoughNoHexproof => {
            crate::effects::applies_to(state, effect, object)
        }
        _ => false,
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
        && !ignores_hexproof(state, obj, you)
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

/// Whether `player` has protection from a quality or player that `source` (such
/// as an Aura enchanting or attempting to enchant `player`) satisfies
/// (CR 702.16c, CR 702.16j, CR 702.16k).
#[must_use]
pub fn player_protected_from(state: &GameState, player: PlayerId, source: ObjectId) -> bool {
    let Some(src) = state.object(source) else {
        return false;
    };
    player_protected_from_object(state, player, src)
}

pub(crate) fn player_protected_from_object(
    state: &GameState,
    player: PlayerId,
    src: &crate::object::GameObject,
) -> bool {
    let source = src.id;
    let affected = |fx: &crate::effects::ContinuousEffect, who: baylee_cards_dsl::PlayerRel| {
        if let Some(seats) = fx.filter.seats() {
            seats.contains(player)
        } else {
            super::players_for_source(who, state, fx.controller, fx.source)
                .is_some_and(|seats| seats.contains(&player))
        }
    };
    state.effects.iter().any(|fx| {
        if !crate::effects::source_is_present(state, fx) {
            return false;
        }
        match fx.modifier {
            baylee_cards_dsl::Modifier::PlayerProtectionFromEverything { who } => affected(fx, who),
            baylee_cards_dsl::Modifier::PlayerProtectionFromOpponents { who } => {
                affected(fx, who) && state.is_opponent(player, src.controller)
            }
            baylee_cards_dsl::Modifier::PlayerProtectionFrom { who, from } => {
                affected(fx, who)
                    && matches_with_context(
                        from,
                        state,
                        src,
                        fx.controller,
                        RuleContext {
                            source: fx.source.unwrap_or(source),
                            text: state.effect_text(fx),
                            resolved_name: None,
                            resolved_subtype: fx.filter.chosen_subtype(),
                            reference: None,
                        },
                    )
            }
            _ => false,
        }
    })
}
