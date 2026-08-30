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

/// The result of a projection: characteristics plus the controller a
/// layer-2 effect would assign. No control modifier exists yet, so the
/// controller currently always equals the object's own — carrying it
/// keeps layer 2 from being silently dropped when the modifier arrives.
#[must_use]
#[derive(Clone, Debug)]
pub struct Projection {
    /// Projected characteristics (layers 1–7 applied).
    pub characteristics: Characteristics,
    /// Projected controller (layer 2).
    pub controller: PlayerId,
}

/// Recomputes an object's characteristics from its base plus all matching
/// continuous effects.
pub fn recompute(state: &GameState, obj: &GameObject) -> Projection {
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
        if layer == baylee_cards_dsl::Layer::PtCounters {
            // Layer 7d: +1/+1 and -1/-1 counters (CR 613.4c) — inside the
            // layer loop so they apply BEFORE the 7e power/toughness
            // switch, not after it.
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
    Projection {
        characteristics: c,
        controller,
    }
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
/// matches.
///
/// This is a real topological sort (Kahn): dependency is not transitive,
/// so a pairwise comparator cannot express it and `sort_by` on a
/// non-total order is unspecified behavior (newer Rust may panic).
/// Ties — and genuine cycles — fall back to timestamp order (CR 613.8).
fn sort_by_dependency(fxs: &mut Vec<&ContinuousEffect>) {
    let n = fxs.len();
    if n < 2 {
        return;
    }
    // Edge i → j: i must be applied before j (j depends on i).
    let mut indegree = vec![0usize; n];
    let mut edges = vec![Vec::new(); n];
    for i in 0..n {
        for j in 0..n {
            if i != j && depends_on(fxs[j], fxs[i]) {
                edges[i].push(j);
                indegree[j] += 1;
            }
        }
    }
    let mut done = vec![false; n];
    let mut order: Vec<&ContinuousEffect> = Vec::with_capacity(n);
    for _ in 0..n {
        // Prefer a ready node (indegree 0) with the smallest timestamp.
        // If nothing is ready the dependency graph has a cycle — break
        // it by timestamp, which is exactly the CR 613.8 fallback.
        let mut best: Option<usize> = None;
        for i in 0..n {
            if done[i] {
                continue;
            }
            best = Some(match best {
                None => i,
                Some(b) => {
                    let ready_i = indegree[i] == 0;
                    let ready_b = indegree[b] == 0;
                    if (ready_i && !ready_b)
                        || (ready_i == ready_b && fxs[i].timestamp < fxs[b].timestamp)
                    {
                        i
                    } else {
                        b
                    }
                }
            });
        }
        let Some(i) = best else { break };
        done[i] = true;
        order.push(fxs[i]);
        for &j in &edges[i] {
            indegree[j] = indegree[j].saturating_sub(1);
        }
    }
    *fxs = order;
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

#[allow(clippy::too_many_lines)] // the modifier vocabulary is one flat table
fn apply(
    c: &mut Characteristics,
    controller: &mut PlayerId,
    fx: &ContinuousEffect,
    state: &GameState,
    obj: &GameObject,
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
        Modifier::AddTypeIfCountersAtLeast {
            kind,
            at_least,
            types,
        } => {
            if obj.counters.get(*kind) >= u16::from(*at_least) {
                c.types = c.types.union(*types);
            }
        }
        Modifier::AddKeywordIfCountersAtLeast {
            kind,
            at_least,
            keywords,
        } => {
            if obj.counters.get(*kind) >= u16::from(*at_least) {
                c.keywords = c.keywords.union(*keywords);
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
        | Modifier::GrantTriggered { .. }
        | Modifier::ManaIsAnyColor
        | Modifier::SearchTakeover => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectFilter;
    use baylee_cards_dsl::{Duration, Filter};
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::ids::EffectId;
    use baylee_core::types::TypeSet;

    static ANY_F: Filter = Filter::Any;
    static READS_TYPE: Filter = Filter::HasType(TypeSet::ARTIFACT);
    static READS_KW: Filter = Filter::HasKeyword(KeywordSet::FLYING);

    fn fx(
        id: u32,
        timestamp: u64,
        filter: &'static Filter,
        modifier: Modifier,
    ) -> ContinuousEffect {
        ContinuousEffect {
            id: EffectId::new(id),
            source: None,
            controller: PlayerId::new(0),
            layer: baylee_cards_dsl::Layer::Type,
            timestamp,
            duration: Duration::Indefinitely,
            filter: EffectFilter::Dsl(filter),
            modifier,
        }
    }

    /// B depends on A (A adds types, B's filter reads types), C depends
    /// on B (B adds a keyword, C's filter reads keywords) — while the
    /// timestamps (B=1, C=2, A=3) contradict the dependency chain.
    /// Only a topological order yields A, B, C; a pairwise comparator
    /// sees b<c, c<a, a<b and its output is unspecified (and newer Rust
    /// may panic on the broken total order).
    #[test]
    fn dependency_order_is_topological_not_pairwise() {
        let a = fx(1, 3, &ANY_F, Modifier::AddType(TypeSet::ARTIFACT));
        let b = fx(2, 1, &READS_TYPE, Modifier::AddKeyword(KeywordSet::FLYING));
        let c = fx(
            3,
            2,
            &READS_KW,
            Modifier::AddColor(ColorSet::from_slice(&[Color::Red])),
        );
        let mut fxs = vec![&b, &c, &a];
        sort_by_dependency(&mut fxs);
        let ids: Vec<u32> = fxs.iter().map(|f| f.id.get()).collect();
        assert_eq!(ids, [1, 2, 3], "A, then the dependent B, then C");
    }

    /// Independent effects keep timestamp order, and a dependency cycle
    /// falls back to timestamps (CR 613.8).
    #[test]
    fn timestamps_order_the_independent_and_break_cycles() {
        let x = fx(1, 10, &ANY_F, Modifier::AddType(TypeSet::ARTIFACT));
        let y = fx(2, 5, &ANY_F, Modifier::AddKeyword(KeywordSet::FLYING));
        let mut fxs = vec![&x, &y];
        sort_by_dependency(&mut fxs);
        let ids: Vec<u32> = fxs.iter().map(|f| f.id.get()).collect();
        assert_eq!(ids, [2, 1], "earlier timestamp first");
    }
}
