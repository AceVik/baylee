//! Characteristic projection through the layer system (CR 613).
//!
//! `recompute` starts from an object's copiable base characteristics and
//! applies every matching continuous effect, layer by layer (1–7),
//! timestamp-ordered within a layer with dependency detection (CR 613.8).
//! The result lands in the object's cache, keyed by the effect generation —
//! the hot path is one integer compare.

use crate::effects::{ContinuousEffect, EffectFilter};
use crate::eval;
use crate::object::{Characteristics, GameObject};
use crate::state::GameState;
use baylee_cards_dsl::{Filter, KeywordSet, LAYERS, Modifier};
use baylee_core::generated::subtypes;
use baylee_core::ids::PlayerId;

/// Recomputes an object's characteristics from its base plus all matching
/// continuous effects.
#[must_use]
pub fn recompute(state: &GameState, obj: &GameObject) -> Characteristics {
    let mut c = obj.base.clone();
    let mut controller = obj.controller;
    for layer in LAYERS {
        let mut fxs: Vec<&ContinuousEffect> = state
            .effects
            .iter()
            .filter(|fx| fx.layer == layer && applies(state, fx, obj))
            .collect();
        sort_by_dependency(&mut fxs);
        for fx in fxs {
            apply(&mut c, &mut controller, fx, state, obj);
        }
    }
    // Layer 7d: counters (CR 613.4c).
    if c.types.contains(baylee_core::types::TypeSet::CREATURE) {
        let plus = obj.counters.get(crate::object::CounterKind::P1P1) as i16;
        let minus = obj.counters.get(crate::object::CounterKind::M1M1) as i16;
        if let Some(p) = &mut c.power {
            *p += plus - minus;
        }
        if let Some(t) = &mut c.toughness {
            *t += plus - minus;
        }
    }
    // Keyword counters (CR 122.1b): a lifelink counter grants lifelink.
    if obj.counters.get(crate::object::CounterKind::Lifelink) > 0 {
        c.keywords = c.keywords.union(KeywordSet::LIFELINK);
    }
    // Changeling (CR 702.73): every creature type.
    if c.keywords.contains(KeywordSet::CHANGELING) {
        for id in 0..subtypes::COUNT {
            let sid = baylee_core::ids::SubtypeId::new(id);
            if matches!(
                subtypes::kind(sid),
                baylee_core::types::SubtypeKind::Creature
            ) {
                c.subtypes.insert(sid);
            }
        }
    }
    let _ = controller; // control changes land in M2.S6
    c
}

fn applies(state: &GameState, fx: &ContinuousEffect, obj: &GameObject) -> bool {
    match &fx.filter {
        EffectFilter::ObjectIs(id) => *id == obj.id,
        EffectFilter::Dsl(filter) => eval::matches(
            filter,
            state,
            obj,
            fx.controller,
            fx.source.unwrap_or(obj.id),
        ),
    }
}

/// Dependency-aware ordering (CR 613.8): within a layer, an effect is
/// applied before effects that depend on it. Dependency (approximation):
/// B depends on A when A's modifier could change whether B's filter
/// matches. Cycles fall back to timestamp order.
fn sort_by_dependency(fxs: &mut Vec<&ContinuousEffect>) {
    fxs.sort_by(|a, b| {
        let a_first = depends_on(b, a);
        let b_first = depends_on(a, b);
        match (a_first, b_first) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.timestamp.cmp(&b.timestamp),
        }
    });
}

fn depends_on(dependent: &ContinuousEffect, depended: &ContinuousEffect) -> bool {
    let EffectFilter::Dsl(filter) = dependent.filter else {
        return false;
    };
    could_change_match(&depended.modifier, filter)
}

/// Conservative dependency test: does `modifier` change anything `filter`
/// reads?
fn could_change_match(modifier: &Modifier, filter: &Filter) -> bool {
    match filter {
        Filter::HasType(_) | Filter::LacksType(_) | Filter::HasSubtype(_) => matches!(
            modifier,
            Modifier::AddType(_)
                | Modifier::RemoveType(_)
                | Modifier::AddSubtype(_)
                | Modifier::AllCreatureTypes
                | Modifier::AllBasicLandTypes
        ),
        Filter::HasColor(_) | Filter::IsColorless => {
            matches!(modifier, Modifier::AddColor(_) | Modifier::SetColor(_))
        }
        Filter::HasKeyword(_) => matches!(
            modifier,
            Modifier::AddKeyword(_) | Modifier::RemoveKeyword(_) | Modifier::LoseKeywords
        ),
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| could_change_match(modifier, f))
        }
        Filter::Not(f) => could_change_match(modifier, f),
        _ => false,
    }
}

fn apply(
    c: &mut Characteristics,
    controller: &mut PlayerId,
    fx: &ContinuousEffect,
    state: &GameState,
    _obj: &GameObject,
) {
    match &fx.modifier {
        Modifier::BecomeCopyOf(id) => {
            // Layer 1: copiable values of the target (its own projection
            // included, CR 707.2).
            if let Some(target) = state.object(*id) {
                *c = target.characteristics().clone();
            }
        }
        Modifier::ModifyPTPerCount { filter, p, t } => {
            let count = state
                .zones
                .list(crate::zone::ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        o.controller == fx.controller
                            && crate::eval::matches(filter, state, o, fx.controller, **id)
                    })
                })
                .count() as i16;
            if let Some(pow) = &mut c.power {
                *pow += count * p;
            }
            if let Some(tou) = &mut c.toughness {
                *tou += count * t;
            }
        }
        Modifier::AddType(t) => c.types = c.types.union(*t),
        Modifier::RemoveType(t) => c.types = c.types.difference(*t),
        Modifier::AddSubtype(s) => c.subtypes.insert(*s),
        Modifier::AllCreatureTypes => {
            for id in 0..subtypes::COUNT {
                let sid = baylee_core::ids::SubtypeId::new(id);
                if matches!(
                    subtypes::kind(sid),
                    baylee_core::types::SubtypeKind::Creature
                ) {
                    c.subtypes.insert(sid);
                }
            }
        }
        Modifier::AllBasicLandTypes => {
            for sid in [
                subtypes::land::FOREST,
                subtypes::land::ISLAND,
                subtypes::land::PLAINS,
                subtypes::land::SWAMP,
                subtypes::land::MOUNTAIN,
            ] {
                c.subtypes.insert(sid);
            }
        }
        Modifier::AddColor(col) => c.colors = c.colors.union(*col),
        Modifier::SetColor(col) => c.colors = *col,
        Modifier::AddKeyword(k) => c.keywords = c.keywords.union(*k),
        Modifier::RemoveKeyword(k) => c.keywords = c.keywords.difference(*k),
        Modifier::LoseKeywords => c.keywords = KeywordSet::EMPTY,
        // Handled by SBAs/legality checks, not by characteristics.
        Modifier::LegendRuleOff
        | Modifier::CantActivateArtifacts
        | Modifier::OpponentsCastAsSorcery
        | Modifier::PlayersCantLose
        | Modifier::CantLoseLife
        | Modifier::PreventDamageToIt
        | Modifier::PreventDamageFromIt
        | Modifier::OpponentsCantSearch
        | Modifier::NoMaxHandSize
        | Modifier::ProtectionFrom(_)
        | Modifier::GrantsFlashback
        | Modifier::PlayerHexproof
        | Modifier::GrantActivated { .. }
        | Modifier::SorceriesHaveFlash
        | Modifier::GrantTriggered { .. } => {}
        Modifier::ModifyPT(p, t) => {
            if let Some(power) = &mut c.power {
                *power += p;
            }
            if let Some(toughness) = &mut c.toughness {
                *toughness += t;
            }
        }
        Modifier::SetPT(p, t) => {
            if c.power.is_some() {
                c.power = Some(*p);
            }
            if c.toughness.is_some() {
                c.toughness = Some(*t);
            }
        }
        Modifier::SwitchPT => {
            if let (Some(p), Some(t)) = (c.power, c.toughness) {
                c.power = Some(t);
                c.toughness = Some(p);
            }
        }
    }
    let _ = controller;
}
