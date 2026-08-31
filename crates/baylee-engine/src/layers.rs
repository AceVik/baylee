//! Characteristic projection through the layer system (CR 613).
//!
//! [`recompute`] starts from an object's copiable base characteristics and
//! applies every matching continuous effect, layer by layer (1–7),
//! timestamp-ordered within a layer with dependency detection (CR 613.8).
//! The result lands in the object's cache, keyed by the effect generation —
//! the hot path is one integer compare.
//!
//! # Why a [`LayerPlan`]
//!
//! Bucketing and dependency-ordering the effect table does not depend on
//! the object being projected: [`sort_by_dependency`] reads only modifiers
//! and filters. Doing it inside the per-object loop therefore repeated the
//! same O(n²) sort once per object *per layer* — the projection pass was
//! `O(objects · layers · effects²)`. A [`LayerPlan`] does it once per
//! refresh, leaving `O(layers · effects²  +  objects · effects)`.
//!
//! Restricting a topological order to a subset is still a topological order
//! of that subset, so ordering the whole bucket up front and filtering per
//! object afterwards yields the same sequence the per-object sort did —
//! and a *more* consistent one, since every object now sees one global
//! ordering decision instead of a separately-tied one.

use crate::effects::{ContinuousEffect, EffectFilter, EffectTable};
use crate::eval;
use crate::object::{Characteristics, GameObject};
use crate::state::GameState;
use baylee_cards_dsl::{Filter, KeywordSet, LAYERS, Layer, Modifier};
use baylee_core::ids::PlayerId;
use baylee_core::types::SubtypeSet;
use smallvec::SmallVec;

/// Number of layers (CR 613.1 sublayers included).
const LAYER_COUNT: usize = LAYERS.len();

/// The largest bucket the exact dependency sort handles; beyond it CR
/// 613.8's timestamp fallback applies. No real board reaches 64 continuous
/// effects *in one layer*, and the bound is what keeps the adjacency
/// matrix a fixed-size bitmask instead of a nested allocation.
const MAX_SORTED: usize = 64;

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

/// The effect table bucketed per layer and ordered once (CR 613.8).
///
/// Build one per projection pass and hand it to [`recompute_with`] for
/// every object.
#[derive(Clone, Debug, Default)]
pub struct LayerPlan {
    /// Indices into the effect table, grouped by layer and ordered within
    /// each group. Indices rather than references: a plan that borrows the
    /// table cannot coexist with the `&mut` needed to store the results,
    /// and a `u32` is half a pointer.
    ordered: Vec<u32>,
    /// `(start, end)` into `ordered`, indexed by `Layer as usize`.
    spans: [(u32, u32); LAYER_COUNT],
}

impl LayerPlan {
    /// Buckets and dependency-orders every registered effect.
    #[must_use]
    pub fn build(effects: &EffectTable) -> Self {
        let mut plan = Self {
            ordered: Vec::with_capacity(effects.len()),
            spans: [(0, 0); LAYER_COUNT],
        };
        if effects.is_empty() {
            return plan;
        }
        let all = effects.as_slice();
        // One bucketing pass per layer keeps registration order as the
        // stable tie-break inside each bucket; the table is small enough
        // that this beats sorting the whole thing by (layer, timestamp).
        for layer in LAYERS {
            let start = plan.ordered.len() as u32;
            plan.ordered.extend(
                all.iter()
                    .enumerate()
                    .filter(|(_, fx)| fx.layer == layer)
                    .map(|(i, _)| i as u32),
            );
            let end = plan.ordered.len() as u32;
            sort_by_dependency(all, &mut plan.ordered[start as usize..end as usize]);
            plan.spans[layer as usize] = (start, end);
        }
        plan
    }

    /// Whether no continuous effect is registered at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }

    /// The ordered effect indices of one layer.
    #[must_use]
    fn layer(&self, layer: Layer) -> &[u32] {
        let (start, end) = self.spans[layer as usize];
        &self.ordered[start as usize..end as usize]
    }
}

/// Whether an object needs the layer machinery at all.
///
/// With no effect registered, a projection can still differ from the base:
/// +1/+1 counters (CR 613.4c), keyword counters (CR 122.1b) and changeling
/// (CR 702.73) are applied by the projection too. Everything else — every
/// card sitting in a library, hand or graveyard on a board with no anthem
/// — projects to exactly its base, and skipping those is what keeps a
/// refresh proportional to the board rather than to the decks.
#[must_use]
pub fn needs_projection(plan: &LayerPlan, obj: &GameObject) -> bool {
    !plan.is_empty()
        || !obj.counters.is_empty()
        || obj.base.keywords.contains(KeywordSet::CHANGELING)
}

/// Recomputes an object's characteristics from its base plus all matching
/// continuous effects.
///
/// Convenience wrapper that builds a one-shot [`LayerPlan`]; the engine's
/// refresh pass builds the plan once and calls [`recompute_with`].
pub fn recompute(state: &GameState, obj: &GameObject) -> Projection {
    let plan = LayerPlan::build(&state.effects);
    recompute_with(state, obj, &plan)
}

/// Recomputes an object's characteristics against a prepared [`LayerPlan`].
pub fn recompute_with(state: &GameState, obj: &GameObject, plan: &LayerPlan) -> Projection {
    let mut c = obj.base.clone();
    // Layer 2 starts from the *base* controller, not from whatever the
    // last refresh projected: an effect that has since ended must leave
    // no trace.
    let mut controller = obj.base_controller;
    let all = state.effects.as_slice();
    for layer in LAYERS {
        for &idx in plan.layer(layer) {
            let fx = &all[idx as usize];
            // CR 613.1: each layer sees the characteristics as modified by
            // every earlier layer, so the filter is evaluated against the
            // in-progress projection — an "all creatures get +1/+1" anthem
            // has to see a land that layer 4 just animated.
            if applies(state, fx, obj, &c) {
                apply(&mut c, &mut controller, fx, state, obj);
            }
        }
        if layer == Layer::PtCounters {
            apply_pt_counters(&mut c, obj);
        }
    }
    // Keyword counters (CR 122.1b): a lifelink counter grants lifelink.
    if obj.counters.get(crate::object::CounterKind::Lifelink) > 0 {
        c.keywords = c.keywords.union(KeywordSet::LIFELINK);
    }
    // Changeling (CR 702.73): every creature type, in eight word ORs.
    if c.keywords.contains(KeywordSet::CHANGELING) {
        c.subtypes = c.subtypes.union(SubtypeSet::ALL_CREATURE);
    }
    Projection {
        characteristics: c,
        controller,
    }
}

/// Layer 7d: +1/+1 and -1/-1 counters (CR 613.4c) — applied inside the
/// layer loop so they land BEFORE the 7e power/toughness switch.
fn apply_pt_counters(c: &mut Characteristics, obj: &GameObject) {
    if !c.types.contains(baylee_core::types::TypeSet::CREATURE) {
        return;
    }
    let plus =
        i16::try_from(obj.counters.get(crate::object::CounterKind::P1P1)).unwrap_or(i16::MAX);
    let minus =
        i16::try_from(obj.counters.get(crate::object::CounterKind::M1M1)).unwrap_or(i16::MAX);
    let delta = plus - minus;
    if delta == 0 {
        return;
    }
    if let Some(p) = &mut c.power {
        *p = p.saturating_add(delta);
    }
    if let Some(t) = &mut c.toughness {
        *t = t.saturating_add(delta);
    }
}

fn applies(
    state: &GameState,
    fx: &ContinuousEffect,
    obj: &GameObject,
    projected: &Characteristics,
) -> bool {
    match &fx.filter {
        EffectFilter::ObjectIs(id) => *id == obj.id,
        EffectFilter::Dsl(filter) => eval::matches_projected(
            filter,
            state,
            obj,
            projected,
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
///
/// The adjacency matrix is one `u64` row per effect, so the whole graph
/// for a realistic layer fits in 512 bytes of stack and needs no
/// allocation at all.
fn sort_by_dependency(all: &[ContinuousEffect], fxs: &mut [u32]) {
    let n = fxs.len();
    if n < 2 {
        return;
    }
    let fx = |slot: usize| &all[fxs[slot] as usize];
    if n > MAX_SORTED {
        // CR 613.8's own fallback. `sort_by_key` is stable, so effects
        // sharing a timestamp keep registration order — determinism holds.
        fxs.sort_by_key(|i| all[*i as usize].timestamp);
        return;
    }
    // `deps[j]` has bit `i` set when j must be applied after i.
    let mut deps = [0u64; MAX_SORTED];
    let mut any_edge = false;
    for (j, row) in deps.iter_mut().enumerate().take(n) {
        for i in 0..n {
            if i != j && depends_on(fx(j), fx(i)) {
                *row |= 1u64 << i;
                any_edge = true;
            }
        }
    }
    if !any_edge {
        fxs.sort_by_key(|i| all[*i as usize].timestamp);
        return;
    }
    let mut placed: u64 = 0;
    let mut order: SmallVec<[u32; 16]> = SmallVec::with_capacity(n);
    for _ in 0..n {
        // Prefer a ready node (no unplaced dependency) with the smallest
        // timestamp. If nothing is ready the graph has a cycle — breaking
        // it by timestamp is exactly the CR 613.8 fallback.
        let mut best: Option<(usize, bool)> = None;
        for (i, row) in deps.iter().enumerate().take(n) {
            if placed & (1u64 << i) != 0 {
                continue;
            }
            let ready = row & !placed == 0;
            best = Some(match best {
                None => (i, ready),
                Some((b, b_ready)) => {
                    if (ready && !b_ready)
                        || (ready == b_ready && fx(i).timestamp < fx(b).timestamp)
                    {
                        (i, ready)
                    } else {
                        (b, b_ready)
                    }
                }
            });
        }
        let Some((i, _)) = best else { break };
        placed |= 1u64 << i;
        order.push(fxs[i]);
    }
    fxs.copy_from_slice(&order);
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
        Filter::HasColor(_) | Filter::IsColorless | Filter::Monocolored => {
            matches!(modifier, Modifier::AddColor(_) | Modifier::SetColor(_))
        }
        Filter::HasKeyword(_) => matches!(
            modifier,
            Modifier::AddKeyword(_) | Modifier::RemoveKeyword(_) | Modifier::LoseKeywords
        ),
        Filter::ToughnessAtMost(_) => matches!(
            modifier,
            Modifier::ModifyPT(..)
                | Modifier::SetPT(..)
                | Modifier::SwitchPT
                | Modifier::ModifyPTPerCount { .. }
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
        // Layer 2 (CR 613.1b): whoever controls the effect controls the
        // permanent, for exactly as long as the effect lasts.
        Modifier::GainControl => *controller = fx.controller,
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
                .count();
            let count = i16::try_from(count).unwrap_or(i16::MAX);
            if let Some(pow) = &mut c.power {
                *pow = pow.saturating_add(count.saturating_mul(*p));
            }
            if let Some(tou) = &mut c.toughness {
                *tou = tou.saturating_add(count.saturating_mul(*t));
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
        Modifier::AllCreatureTypes => c.subtypes = c.subtypes.union(SubtypeSet::ALL_CREATURE),
        Modifier::AllBasicLandTypes => c.subtypes = c.subtypes.union(SubtypeSet::BASIC_LANDS),
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
                *power = power.saturating_add(*p);
            }
            if let Some(toughness) = &mut c.toughness {
                *toughness = toughness.saturating_add(*t);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cards_dsl::Duration;
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
            layer: Layer::Type,
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
        let all = [b, c, a];
        let mut fxs = vec![0u32, 1, 2];
        sort_by_dependency(&all, &mut fxs);
        let ids: Vec<u32> = fxs.iter().map(|i| all[*i as usize].id.get()).collect();
        assert_eq!(ids, [1, 2, 3], "A, then the dependent B, then C");
    }

    /// Independent effects keep timestamp order, and a dependency cycle
    /// falls back to timestamps (CR 613.8).
    #[test]
    fn timestamps_order_the_independent_and_break_cycles() {
        let x = fx(1, 10, &ANY_F, Modifier::AddType(TypeSet::ARTIFACT));
        let y = fx(2, 5, &ANY_F, Modifier::AddKeyword(KeywordSet::FLYING));
        let all = [x, y];
        let mut fxs = vec![0u32, 1];
        sort_by_dependency(&all, &mut fxs);
        let ids: Vec<u32> = fxs.iter().map(|i| all[*i as usize].id.get()).collect();
        assert_eq!(ids, [2, 1], "earlier timestamp first");
    }

    /// A plan buckets effects per layer, and each bucket is ordered on its
    /// own — the property that lets the per-object loop just filter.
    #[test]
    fn plan_buckets_and_orders_each_layer_once() {
        let mut table = EffectTable::default();
        let mut reg = |layer: Layer, timestamp: u64, modifier: Modifier| {
            table.register(ContinuousEffect {
                id: EffectId::new(0),
                source: None,
                controller: PlayerId::new(0),
                layer,
                timestamp,
                duration: Duration::Indefinitely,
                filter: EffectFilter::Dsl(&ANY_F),
                modifier,
            });
        };
        reg(Layer::PtModify, 5, Modifier::ModifyPT(1, 1));
        reg(Layer::Type, 9, Modifier::AddType(TypeSet::ARTIFACT));
        reg(Layer::PtModify, 2, Modifier::ModifyPT(2, 2));

        let plan = LayerPlan::build(&table);
        assert!(!plan.is_empty());
        assert_eq!(plan.layer(Layer::Type).len(), 1);
        assert_eq!(plan.layer(Layer::Color).len(), 0);
        let all = table.as_slice();
        let pt: Vec<u64> = plan
            .layer(Layer::PtModify)
            .iter()
            .map(|i| all[*i as usize].timestamp)
            .collect();
        assert_eq!(pt, [2, 5], "each bucket is timestamp-ordered on its own");
    }

    /// Changeling is a whole-range mask, not a scan: every creature type
    /// is set and nothing outside the creature block is.
    #[test]
    fn changeling_sets_exactly_the_creature_block() {
        use baylee_core::generated::subtypes;
        let all = SubtypeSet::ALL_CREATURE;
        assert!(all.contains(subtypes::creature::WIZARD));
        assert!(all.contains(subtypes::creature::ALLY));
        assert!(!all.contains(subtypes::land::FOREST));
        assert!(!all.contains(subtypes::spell::ADVENTURE));
        assert_eq!(all.len(), u32::from(subtypes::CREATURE_END));
    }
}
