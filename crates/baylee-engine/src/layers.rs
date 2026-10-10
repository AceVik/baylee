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
//!
//! Conditional animation is the exception: whether removing creature
//! changes its condition depends on the current object. A plan containing
//! it reevaluates layer-4 dependencies during each projection (CR 613.8c).

use crate::effects::{ContinuousEffect, EffectFilter, EffectTable};
use crate::eval;
use crate::object::{Characteristics, GameObject};
use crate::state::GameState;
use baylee_cards_dsl::{Filter, KeywordSet, LAYERS, Layer, Modifier};
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::types::{SubtypeSet, TypeSet};
use smallvec::SmallVec;
use std::sync::Arc;

/// Number of layers (CR 613.1 sublayers included).
const LAYER_COUNT: usize = LAYERS.len();

/// The shared plan's fixed-size dependency graph limit; larger buckets
/// use its timestamp-only approximation. Conditional animation's dynamic
/// type-layer graph uses this inline capacity and grows for larger boards.
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
    /// Whether a layer counted other objects (a P/T as big as the lands
    /// you control) and so read their cached projections, which a refresh
    /// that has not reached them yet still holds from the last one. The
    /// refresh projects these again once every other object is done
    /// (`GameState::refresh_characteristics`).
    pub read_board: bool,
}

/// The effect table bucketed per layer, with object-independent ordering
/// prepared once (CR 613.8).
///
/// Build one per projection pass and hand it to [`recompute_with`] for
/// every object.
#[derive(Clone, Debug, Default)]
pub struct LayerPlan {
    /// Indices into the effect table, grouped by layer and ordered within
    /// each group except a type bucket with conditional animation, which
    /// is ordered during projection. Indices rather than references: a
    /// plan that borrows the table cannot coexist with the `&mut` needed
    /// to store the results, and a `u32` is half a pointer.
    ordered: Vec<u32>,
    /// `(start, end)` into `ordered`, indexed by `Layer as usize`.
    spans: [(u32, u32); LAYER_COUNT],
    /// A conditional animation needs actual, changing type-layer matches
    /// rather than the conservative order shared by other projections.
    conditional_animation: bool,
}

impl LayerPlan {
    /// Buckets and dependency-orders every registered effect.
    #[must_use]
    pub fn build(effects: &EffectTable) -> Self {
        let mut plan = Self {
            ordered: Vec::with_capacity(effects.len()),
            spans: [(0, 0); LAYER_COUNT],
            conditional_animation: effects
                .iter()
                .any(|fx| matches!(fx.modifier, Modifier::AnimateNoncreatureArtifact)),
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
                    .filter(|(_, fx)| {
                        fx.layer == layer
                            || (layer == Layer::PtSet
                                && matches!(fx.modifier, Modifier::AnimateNoncreatureArtifact))
                    })
                    .map(|(i, _)| i as u32),
            );
            let end = plan.ordered.len() as u32;
            if layer != Layer::Type || !plan.conditional_animation {
                sort_by_dependency(all, &mut plan.ordered[start as usize..end as usize], layer);
            }
            plan.spans[layer as usize] = (start, end);
        }
        plan
    }

    /// Whether no continuous effect is registered at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }

    /// How many control-changing effects there are (CR 613.1b): each can
    /// hand one more controller to a static ability that reads it, so this
    /// bounds how often a refresh has to walk the board again.
    #[must_use]
    pub fn control_effects(&self) -> usize {
        self.layer(Layer::Control).len()
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
pub fn needs_projection(state: &GameState, plan: &LayerPlan, obj: &GameObject) -> bool {
    !plan.is_empty()
        || eval::live_context(state, obj.id).text != crate::text_changes::TextChangeMap::IDENTITY
        || !obj.counters.is_empty()
        || obj.base.keywords.contains(KeywordSet::CHANGELING)
        || obj.riders.contains(&crate::object::Rider::Dashed)
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
    let mut c = (*obj.base).clone();
    // "As long as this permanent's dash cost was paid, it has haste"
    // (CR 702.109a): an ability of the permanent itself, so it is there
    // before any effect applies and an effect that removes abilities
    // (layer 6) removes it too.
    if obj.riders.contains(&crate::object::Rider::Dashed) {
        c.keywords = c.keywords.union(KeywordSet::HASTE);
    }
    // Layer 2 starts from the *base* controller, not from whatever the
    // last refresh projected: an effect that has since ended must leave
    // no trace.
    let mut controller = obj.base_controller;
    let mut read_board = false;
    // CR 613.6: an effect that began in layer 4 continues on the same
    // objects in layer 7b. This belongs to this projection, never the game
    // state: the next refresh starts from the copiable values again.
    let mut animations: SmallVec<[u32; 4]> = SmallVec::new();
    let all = state.effects.as_slice();
    for layer in LAYERS {
        if layer == Layer::Text {
            // Text changes affect the type line and printed/copied keywords
            // here, before type-setting effects and later ability grants.
            let text = eval::live_context(state, obj.id).text;
            let old_types = c.subtypes;
            c.subtypes = text.land_types(c.subtypes);
            if c.subtypes != old_types {
                update_intrinsic_mana_summary(state, obj, &mut c, old_types);
            }
            c.keywords = text.keywords(c.keywords);
        }
        let dynamic_types = layer == Layer::Type && plan.conditional_animation;
        let mut remaining: SmallVec<[u32; 16]> = if dynamic_types {
            plan.layer(layer).into()
        } else {
            SmallVec::new()
        };
        for &ordered_idx in plan.layer(layer) {
            let idx = if dynamic_types {
                next_type_effect(state, obj, &c, all, &mut remaining)
            } else {
                ordered_idx
            };
            let fx = &all[idx as usize];
            // CR 613.1: each layer sees the characteristics as modified by
            // every earlier layer, so the filter is evaluated against the
            // in-progress projection — an "all creatures get +1/+1" anthem
            // has to see a land that layer 4 just animated.
            let animation = matches!(fx.modifier, Modifier::AnimateNoncreatureArtifact);
            let applies_now = if animation && layer == Layer::PtSet {
                animations.contains(&idx)
            } else {
                applies(state, fx, obj, &c) && (!animation || !c.types.contains(TypeSet::CREATURE))
            };
            if applies_now {
                if animation && layer == Layer::Type {
                    animations.push(idx);
                }
                #[cfg(test)]
                crate::ability_log::static_applied(fx);
                apply(
                    &mut c,
                    &mut controller,
                    fx,
                    state,
                    obj,
                    &mut read_board,
                    layer,
                );
            }
        }
        // CR 604.3: a characteristic-defining P/T works in every zone. On
        // the battlefield it is a registered static and applied above; off
        // it, nothing is registered, so the card applies its own here.
        if layer == Layer::PtCda
            && let Some(Modifier::CharacteristicPT {
                count,
                toughness_plus,
            }) = state.off_battlefield_pt_cda(obj)
        {
            let n = pt_count(
                state,
                obj,
                &c,
                controller,
                count,
                eval::live_context(state, obj.id),
                &mut read_board,
            );
            c.power = Some(n);
            c.toughness = Some(n.saturating_add(i16::from(toughness_plus)));
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
        read_board,
    }
}

/// Replace only the type-derived portion of the production summary. A printed
/// mana symbol can name the same color and is unaffected by CR 612 text changes.
fn update_intrinsic_mana_summary(
    state: &GameState,
    obj: &GameObject,
    current: &mut Characteristics,
    old_types: baylee_core::types::SubtypeSet,
) {
    use baylee_cards_dsl::{AbilityDef, Effect, ManaSource};
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::generated::subtypes::land;
    use baylee_core::mana::ManaColor;
    let mut printed = ColorSet::EMPTY;
    let mut keep = |color| {
        let color = match color {
            ManaColor::White => Color::White,
            ManaColor::Blue => Color::Blue,
            ManaColor::Black => Color::Black,
            ManaColor::Red => Color::Red,
            ManaColor::Green => Color::Green,
            ManaColor::Colorless => return,
        };
        printed = printed.union(ColorSet::of(color));
    };
    if let Some(list) = state.printed_ability_list(obj.id) {
        for ability in list
            .abilities
            .iter()
            .filter(|ability| ability.is_mana_ability())
        {
            let (AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. }) = ability
            else {
                continue;
            };
            for effect in *effects {
                if let Effect::AddMana { source, .. } = effect {
                    match source {
                        ManaSource::Fixed(color) => keep(*color),
                        ManaSource::Choice(colors) | ManaSource::ChosenOr(colors) => {
                            for color in *colors {
                                keep(*color);
                            }
                        }
                        ManaSource::CommanderIdentity => {
                            for color in ManaColor::ALL {
                                keep(color);
                            }
                        }
                        ManaSource::IntrinsicBasicLandTypes
                        | ManaSource::LandColor { .. }
                        | ManaSource::Chosen => {}
                    }
                }
            }
        }
    }
    for subtype in [
        land::PLAINS,
        land::ISLAND,
        land::SWAMP,
        land::MOUNTAIN,
        land::FOREST,
    ] {
        let color = basic_land_color(subtype);
        if old_types.contains(subtype) {
            current.produced_colors = current
                .produced_colors
                .difference(color.difference(printed));
        }
        if current.subtypes.contains(subtype) {
            current.produced_colors = current.produced_colors.union(color);
        }
    }
}

/// Layer 7c: counters that modify power and toughness (CR 613.4c) —
/// applied inside the layer loop so they land BEFORE the 7d switch.
///
/// Every counter the permanent wears is asked, rather than the two the pool
/// used to print: CR 122.1a gives a +X/+Y counter its own arithmetic, and
/// Wall of Roots' -0/-1 subtracts a toughness and no power. Power and
/// toughness are therefore summed apart — one shared delta is right only
/// while every counter is symmetric, which was true of +1/+1 and -1/-1 and
/// is true of nothing else.
fn apply_pt_counters(c: &mut Characteristics, obj: &GameObject) {
    if !c.types.contains(baylee_core::types::TypeSet::CREATURE) {
        return;
    }
    let (mut dp, mut dt) = (0i16, 0i16);
    for (kind, n) in obj.counters.iter() {
        let Some((per_power, per_toughness)) = kind.power_toughness() else {
            continue;
        };
        let n = i16::try_from(n).unwrap_or(i16::MAX);
        dp = dp.saturating_add(per_power.saturating_mul(n));
        dt = dt.saturating_add(per_toughness.saturating_mul(n));
    }
    if dp == 0 && dt == 0 {
        return;
    }
    if let Some(p) = &mut c.power {
        *p = p.saturating_add(dp);
    }
    if let Some(t) = &mut c.toughness {
        *t = t.saturating_add(dt);
    }
}

/// The largest chain of copies [`copiable_values`] will walk.
///
/// A cycle is unreachable in *this pool*, not in the rules: every copy
/// effect here is an as-it-enters one and so names something that was
/// already on the battlefield, which makes the chain as old as the game and
/// unable to bend back on itself. A card that turns an existing permanent
/// into a copy of another (Cytoshape) can be pointed both ways, and the
/// rules answer that with copiable-values arithmetic where this recursion
/// would spin — so the bound is what stops a future card from hanging the
/// projection, and the bound is why it is not an assertion.
const MAX_COPY_DEPTH: u8 = 8;

/// The copiable values of an object (CR 707.2).
///
/// Not its projection. A copy takes the printed values as modified by other
/// **copy** effects and by nothing else: an anthem, a +1/+1 counter and a
/// control-change all live in layers past 1 and none of them come across.
/// So this reads `base` and the effect table and **never a cache** — which
/// is what makes the answer the same whichever object was projected first,
/// and is the whole reason it is not `target.characteristics()`.
///
/// The [`Arc`] is the common path: nothing is copying most objects, so the
/// answer is `base` and costs a refcount. `None` means the object is gone,
/// which every caller already had to answer for — a copy effect whose
/// target has left the battlefield changes nothing.
///
/// Temporary copies carry a snapshot including their copy exceptions.
/// The original object changing or leaving cannot alter that snapshot.
///
/// One caller that stayed on `base`: `CopyTargetSpell` copies a spell, and
/// nothing registers a copy effect on a stack object, so the two answers
/// cannot differ there. It is the same class as the three permanent sites
/// and would want this function the day one does.
#[must_use]
pub fn copiable_values(state: &GameState, id: ObjectId) -> Option<Arc<Characteristics>> {
    let values = copiable_values_at(state, id, 0)?;
    // CR 202.3b and 712.8e: a copy of a nonmodal double-faced card's back
    // face has mana value 0. The face's own mana value is its front face's,
    // and that is the one thing about the face a copy does not take.
    if values.front_mana_value.is_some_and(|value| value != 0) {
        let mut copied = (*values).clone();
        copied.front_mana_value = Some(0);
        return Some(Arc::new(copied));
    }
    Some(values)
}

fn copiable_values_at(state: &GameState, id: ObjectId, depth: u8) -> Option<Arc<Characteristics>> {
    let obj = state.object(id)?;
    if depth >= MAX_COPY_DEPTH {
        return Some(obj.base.clone());
    }
    let mut out: Option<Arc<Characteristics>> = None;
    // Timestamp order, taken rather than sorted: every `register` stamps
    // with `next_timestamp` and the table is only ever appended to and
    // retained, so its own order is that order. Layer 1 also needs no
    // dependency sort — CR 613.8 orders an effect against effects whose
    // filters it could change, and there is no layer before this one.
    for fx in state.effects.as_slice() {
        if fx.layer != Layer::Copy {
            continue;
        }
        let so_far = out.as_deref().unwrap_or(&obj.base);
        if !applies(state, fx, obj, so_far) {
            continue;
        }
        if let Modifier::BecomeCopyOf(target) = fx.modifier {
            out = state
                .copy_snapshots
                .iter()
                .find(|(id, _)| *id == fx.id)
                .map(|(_, values)| values.clone())
                .or_else(|| copiable_values_at(state, target, depth + 1))
                .or(out);
        }
    }
    Some(out.unwrap_or_else(|| obj.base.clone()))
}

fn applies(
    state: &GameState,
    fx: &ContinuousEffect,
    obj: &GameObject,
    projected: &Characteristics,
) -> bool {
    match &fx.filter {
        EffectFilter::ObjectIs(..) => fx.filter.names(obj),
        EffectFilter::Dsl(filter) => eval::matches_projected_with_context(
            filter,
            state,
            obj,
            projected,
            fx.controller,
            crate::text_changes::RuleContext {
                source: fx.source.unwrap_or(obj.id),
                text: state.effect_text(fx),
            },
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
fn sort_by_dependency(all: &[ContinuousEffect], fxs: &mut [u32], layer: Layer) {
    let n = fxs.len();
    if n < 2 {
        return;
    }
    let fx = |slot: usize| &all[fxs[slot] as usize];
    if n > MAX_SORTED {
        // The shared plan's bounded fallback. `sort_by_key` is stable, so effects
        // sharing a timestamp keep registration order — determinism holds.
        fxs.sort_by_key(|i| all[*i as usize].timestamp);
        return;
    }
    // `deps[j]` has bit `i` set when j must be applied after i.
    let mut deps = [0u64; MAX_SORTED];
    let mut any_edge = false;
    for (j, row) in deps.iter_mut().enumerate().take(n) {
        for i in 0..n {
            if i != j && depends_on(fx(j), fx(i), layer) {
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

fn depends_on(dependent: &ContinuousEffect, depended: &ContinuousEffect, layer: Layer) -> bool {
    if matches!(dependent.modifier, Modifier::AnimateNoncreatureArtifact) {
        // Layer 4 uses `depends_on_in_type_layer` against the projection.
        // In 7b this effect's affected objects are fixed by layer 4
        // (CR 613.6), so its filter and condition create no dependency.
        return false;
    }
    let EffectFilter::Dsl(filter) = dependent.filter else {
        return false;
    };
    could_change_match(&depended.modifier, filter, layer)
}

/// CR 613.8c: after each type effect, reconsider which remaining effects
/// must wait. The normal shared plan remains sufficient on boards without
/// conditional animation. Equal timestamps keep registration order.
fn next_type_effect(
    state: &GameState,
    obj: &GameObject,
    projected: &Characteristics,
    all: &[ContinuousEffect],
    remaining: &mut SmallVec<[u32; 16]>,
) -> u32 {
    let n = remaining.len();
    let words = n.div_ceil(u64::BITS as usize);
    // One word per row up to 64 effects, entirely on the stack. Larger
    // boards use additional words rather than dropping dependencies.
    let mut deps: SmallVec<[u64; MAX_SORTED]> = smallvec::smallvec![0; n * words];
    for (slot, &index) in remaining.iter().enumerate() {
        for (other_slot, &other) in remaining.iter().enumerate() {
            if slot != other_slot
                && depends_on_in_type_layer(
                    &all[index as usize],
                    &all[other as usize],
                    state,
                    obj,
                    projected,
                )
            {
                deps[slot * words + other_slot / 64] |= 1u64 << (other_slot % 64);
            }
        }
    }
    // CR 613.8b ignores dependencies *within* a loop even if some
    // unrelated effect is ready. Waiting until the whole graph has no
    // ready node would wrongly let that unrelated effect jump ahead.
    let mut reachable = deps.clone();
    for via in 0..n {
        for slot in 0..n {
            if reachable[slot * words + via / 64] & (1u64 << (via % 64)) != 0 {
                for word in 0..words {
                    let through = reachable[via * words + word];
                    reachable[slot * words + word] |= through;
                }
            }
        }
    }
    for slot in 0..n {
        for other in 0..n {
            if reachable[slot * words + other / 64] & (1u64 << (other % 64)) != 0
                && reachable[other * words + slot / 64] & (1u64 << (slot % 64)) != 0
            {
                deps[slot * words + other / 64] &= !(1u64 << (other % 64));
            }
        }
    }
    let slot = remaining
        .iter()
        .enumerate()
        .filter(|(slot, _)| {
            deps[slot * words..(slot + 1) * words]
                .iter()
                .all(|row| *row == 0)
        })
        .min_by_key(|(_, index)| (all[**index as usize].timestamp, **index))
        .map_or(0, |(slot, _)| slot);
    remaining.remove(slot)
}

/// Dependency involving conditional animation reads what the effect
/// actually changes on this object. Removing creature from a noncreature
/// changes nothing; treating that no-op as a dependency would wrongly
/// reorder a later Swift Reconfiguration before Animate Artifact.
fn depends_on_in_type_layer(
    dependent: &ContinuousEffect,
    depended: &ContinuousEffect,
    state: &GameState,
    obj: &GameObject,
    projected: &Characteristics,
) -> bool {
    if !matches!(dependent.modifier, Modifier::AnimateNoncreatureArtifact)
        && !matches!(depended.modifier, Modifier::AnimateNoncreatureArtifact)
    {
        return depends_on(dependent, depended, Layer::Type);
    }
    let matches = |fx: &ContinuousEffect, c: &Characteristics| {
        applies(state, fx, obj, c)
            && (!matches!(fx.modifier, Modifier::AnimateNoncreatureArtifact)
                || !c.types.contains(TypeSet::CREATURE))
    };
    if !matches(depended, projected) {
        return false;
    }
    let mut after = projected.clone();
    let mut controller = obj.controller;
    let mut read_board = false;
    apply(
        &mut after,
        &mut controller,
        depended,
        state,
        obj,
        &mut read_board,
        Layer::Type,
    );
    matches(dependent, projected) != matches(dependent, &after)
}

/// Conservative dependency test: does `modifier` change anything `filter`
/// reads?
///
/// Exhaustive over [`Filter`] deliberately. It used to end in `_ => false`,
/// which is the quiet direction: a filter the table did not name was
/// declared independent of every modifier, CR 613.8 never reordered the
/// pair, and the later effect read a characteristic the earlier one was
/// about to change. Four filters were sitting in that arm — the two that
/// read an object's subtypes without writing one down, and both control
/// predicates against `GainControl` — and the compiler had nothing to say.
/// A filter added from here on has to answer.
fn could_change_match(modifier: &Modifier, filter: &Filter, layer: Layer) -> bool {
    match filter {
        // The last two read the object's subtypes as well; what differs is
        // where the subtype they are compared against comes from.
        Filter::HasType(_)
        | Filter::LacksType(_)
        | Filter::HasSubtype(_)
        | Filter::MatchesChosenTypeOfSource
        | Filter::SharesSubtypeWithCommander => {
            matches!(
                modifier,
                Modifier::AddType(_)
                    | Modifier::RemoveType(_)
                    | Modifier::AddSubtype(_)
                    | Modifier::AllCreatureTypes
                    | Modifier::ReplaceCreatureTypes(_)
                    | Modifier::AllBasicLandTypes
                    | Modifier::SetLandType(_)
                    | Modifier::SetLandTypeToChosen
                    | Modifier::BecomeType { .. }
                    | Modifier::AddTypeIfCountersAtLeast { .. }
                    | Modifier::BecomeCopyOf(_)
            ) || (layer == Layer::Type && matches!(modifier, Modifier::AnimateNoncreatureArtifact))
        }
        Filter::HasColor(_) | Filter::IsColorless | Filter::Monocolored => matches!(
            modifier,
            Modifier::AddColor(_) | Modifier::SetColor(_) | Modifier::BecomeCopyOf(_)
        ),
        Filter::HasKeyword(_) => matches!(
            modifier,
            Modifier::AddKeyword(_)
                | Modifier::RemoveKeyword(_)
                | Modifier::LoseKeywords
                | Modifier::LoseAllAbilities
                | Modifier::SetLandType(_)
                | Modifier::SetLandTypeToChosen
                | Modifier::AddKeywordIfCountersAtLeast { .. }
                | Modifier::BecomeCopyOf(_)
        ),
        Filter::ToughnessAtMost(_)
        | Filter::ToughnessAtLeast(_)
        | Filter::PowerAtLeast(_)
        | Filter::PowerAtMost(_)
        | Filter::PowerLessThanSourcePower
        | Filter::ToughnessLessThanSourcePower => {
            matches!(
                modifier,
                Modifier::ModifyPT(..)
                    | Modifier::SetPT(..)
                    | Modifier::SetPower(_)
                    | Modifier::SetPTToCount(_)
                    | Modifier::SwitchPT
                    | Modifier::CharacteristicPT { .. }
                    | Modifier::ModifyPTPerCount { .. }
                    | Modifier::ModifyPTHalfCount(_)
                    | Modifier::ModifyPTPerGraveyardCard { .. }
                    | Modifier::BecomeCopyOf(_)
            ) || (layer == Layer::PtSet && matches!(modifier, Modifier::AnimateNoncreatureArtifact))
        }
        // Layer 2 moves a permanent from one side of the table to the
        // other, which is the whole of what these read: a change of
        // control also restarts how long it has been controlled (CR 302.6).
        Filter::ControlledByYou
        | Filter::ControlledByOpponent
        | Filter::ControlledByActivePlayer
        | Filter::ControlledByDefendingPlayer
        | Filter::ControlledSinceTurnBegan => matches!(modifier, Modifier::GainControl),
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| could_change_match(modifier, f, layer))
        }
        Filter::Not(f) => could_change_match(modifier, f, layer),
        // Nothing in the modifier vocabulary changes any of these. Ownership
        // and tokenhood are fixed for as long as the object exists (CR
        // 108.3, CR 111.1); tapped, attacking, attachment and zone are game
        // state rather than a characteristic a continuous effect writes; no
        // modifier grants a supertype; and mana value comes off the printed
        // cost, which only copying rewrites — and a copy effect's own filter
        // names an object rather than reading one of these. A *name* is the
        // same case as mana value: layer 3 (CR 613.1c) writes one and this
        // vocabulary has no such modifier, `BecomeCopyOf` being layer 1 and
        // applied before any of this. The day a text-changing modifier
        // exists, `Named` is what leaves this list.
        Filter::HasManaAbility => matches!(layer, Layer::Ability | Layer::Type),
        Filter::Any
        | Filter::Named(_)
        | Filter::This
        | Filter::Another
        | Filter::HasSupertype(_)
        | Filter::IsToken
        | Filter::WithSingleTarget
        | Filter::OwnedByYou
        | Filter::Tapped
        | Filter::Untapped
        | Filter::Attacking
        | Filter::BandedWithSource
        | Filter::NotTargetedByAnotherNamed(_)
        | Filter::Blocking
        | Filter::Unblocked
        | Filter::EnteredThisTurn
        | Filter::PutIntoGraveyardThisTurn
        | Filter::AttackedThisTurn
        | Filter::HasCounter(_)
        | Filter::AttachedToBySource
        | Filter::AttachedToSource
        | Filter::IsAttached
        | Filter::CmcAtMost(_)
        | Filter::CmcAtMostX
        | Filter::CmcExactlyX
        | Filter::CmcAtMostColorsSpent
        | Filter::CmcAtLeast(_)
        | Filter::InZone(_) => false,
    }
}

/// The permanents `controller` controls that match `filter` — every
/// permanent that does, with no controller — as a count for a P/T that grows
/// with the board. `you` is who "you" is to the filter.
///
/// CR 613.1: every earlier layer is already applied, and for the object
/// being projected that result lives in `c` and not yet in its cache —
/// `recompute_with` walks one object through all the layers, so its cached
/// characteristics are the *previous* projection until this one is written
/// back. Ashaya, Soul of the Wild is the card that reads the difference: it
/// makes your nontoken creatures into lands at layer 4 and is then as big as
/// the lands you control at 7a, so it has to count **itself**, and a count
/// off the cache left it one short for exactly one refresh.
///
/// Every other object is read from its cache, and within one walk of the
/// refresh that is its finished projection only if the walk has already
/// reached it: an Elf further down the list is still what the last refresh
/// made of it, an Elf and not yet a Forest. So a count sets `read_board`,
/// and the refresh projects the counting objects again once every other
/// object is done. That is CR 613.1 (the layers in their order), not a
/// dependency: CR 613.8a asks for two effects in the same layer or
/// sublayer, and a count in layer 7 and the type change it reads in layer 4
/// are not. Phased-out permanents are not there to count (CR 702.26b).
/// CR 305.7: a land's subtype set to the basic land type `subtype`. Its old
/// land types go and the new one comes, and it loses every ability its rules
/// text gives it — the keywords here, the rest through `rules_text_lost`,
/// which `GameObject::abilities` reads. Layer 4 comes before layer 6, so a
/// keyword another effect grants still lands, whatever its timestamp. What
/// the land makes is now its new type's mana alone.
fn set_land_type(c: &mut Characteristics, subtype: baylee_core::ids::SubtypeId) {
    c.subtypes = c
        .subtypes
        .difference(baylee_core::generated::subtypes::ALL_LAND_TYPES);
    c.subtypes.insert(subtype);
    c.keywords = KeywordSet::EMPTY;
    c.rules_text_lost = true;
    c.produced_colors = basic_land_color(subtype);
    c.produced_colorless = false;
    c.produced_chosen = false;
}

/// The colour of mana a basic land type's ability makes (CR 305.6); none for
/// any other subtype.
fn basic_land_color(subtype: baylee_core::ids::SubtypeId) -> baylee_core::color::ColorSet {
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::generated::subtypes::land;
    [
        (land::PLAINS, Color::White),
        (land::ISLAND, Color::Blue),
        (land::SWAMP, Color::Black),
        (land::MOUNTAIN, Color::Red),
        (land::FOREST, Color::Green),
    ]
    .into_iter()
    .find(|(basic, _)| *basic == subtype)
    .map_or(ColorSet::EMPTY, |(_, color)| ColorSet::of(color))
}

/// Whether a count's filter picks its objects by what they are attached to
/// (`Filter::AttachedToSource`, alone or in an `And`), which no control
/// clause scopes.
fn counts_by_attachment(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::Filter;
    match filter {
        Filter::AttachedToSource => true,
        Filter::And(parts) => parts.iter().any(counts_by_attachment),
        _ => false,
    }
}

fn count_controlled(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    controller: Option<PlayerId>,
    you: PlayerId,
    filter: &baylee_cards_dsl::Filter,
    context: crate::text_changes::RuleContext,
) -> usize {
    state
        .battlefield_seen()
        .filter_map(|id| state.object(id))
        .filter(|o| {
            controller.is_none_or(|p| o.controller == p)
                && if o.id == obj.id {
                    crate::eval::matches_projected_with_context(filter, state, o, c, you, context)
                } else {
                    crate::eval::matches_with_context(filter, state, o, you, context)
                }
        })
        .count()
}

/// The number a [`PtCount`](baylee_cards_dsl::PtCount) names, for
/// `CharacteristicPT` (7a) and `SetPTToCount` (7b) alike; `you` is whose
/// permanents `YouControl` counts. Either count reads other objects, so
/// both set `read_board`.
fn pt_count(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    you: PlayerId,
    count: baylee_cards_dsl::PtCount,
    context: crate::text_changes::RuleContext,
    read_board: &mut bool,
) -> i16 {
    *read_board = true;
    let n = match count {
        baylee_cards_dsl::PtCount::YouControl(filter) => {
            count_controlled(state, obj, c, Some(you), you, filter, context)
        }
        baylee_cards_dsl::PtCount::OnBattlefield(filter) => {
            count_controlled(state, obj, c, None, you, filter, context)
        }
        baylee_cards_dsl::PtCount::DefendingPlayerControls(filter) => {
            match defending_player_of(state, obj.id) {
                Some(defending) => {
                    count_controlled(state, obj, c, Some(defending), you, filter, context)
                }
                None => 0,
            }
        }
        baylee_cards_dsl::PtCount::CardTypesInAllGraveyards => card_types_in_all_graveyards(state),
        baylee_cards_dsl::PtCount::ExiledWithThis => cards_exiled_with(state, obj),
    };
    i16::try_from(n).unwrap_or(i16::MAX)
}

/// The defending player for `attacker` (CR 508.5's first sentence): the
/// player it attacks, or the controller of the planeswalker it attacks —
/// the one it was declared attacking, even after that planeswalker has left
/// (CR 506.4c keeps the creature attacking), so its last controller then:
/// in a two-player game the defending player stays the nonactive player
/// for the whole combat phase (CR 506.2).
///
/// `None` while it is not attacking. CR 508.5's second sentence (a creature
/// removed from combat still refers to the player it was attacking) is not
/// modelled, because combat keeps no record of an attacker it removed; the
/// one card counting this way, Gaea's Liege, reads it only while it is
/// attacking, and its other sentence applies once it is not.
fn defending_player_of(state: &GameState, attacker: ObjectId) -> Option<PlayerId> {
    let info = state
        .combat
        .attackers()
        .iter()
        .find(|info| info.creature == attacker)?;
    match info.defending {
        Defender::Player(player) => Some(player),
        Defender::Planeswalker(walker) => state.last_known_controller(walker),
    }
}

/// The number of card types (CR 205.2a) among cards in all graveyards.
///
/// Read off each card's own characteristics: a card in a graveyard is
/// what it prints (CR 400.7 left every effect on it behind), and a
/// double-faced card is its front face there (CR 712.8a). Cards only
/// ([`GameObject::is_card`]): a token or a copy that died lies in the
/// graveyard until state-based actions remove it (CR 704.5d, 704.5e), and
/// the engine projects before it checks them.
fn card_types_in_all_graveyards(state: &GameState) -> usize {
    use baylee_core::types::TypeSet;
    const CARD_TYPES: [TypeSet; 9] = [
        TypeSet::ARTIFACT,
        TypeSet::BATTLE,
        TypeSet::CREATURE,
        TypeSet::ENCHANTMENT,
        TypeSet::INSTANT,
        TypeSet::KINDRED,
        TypeSet::LAND,
        TypeSet::PLANESWALKER,
        TypeSet::SORCERY,
    ];
    let mut seen = TypeSet::EMPTY;
    for player in 0..state.players.len() {
        let seat = PlayerId::new(u8::try_from(player).unwrap_or(u8::MAX));
        for id in state.zones.list(crate::zone::ZoneLocation::Graveyard(seat)) {
            if let Some(o) = state.object(*id).filter(|o| o.is_card()) {
                seen = seen.union(o.characteristics().types);
            }
        }
    }
    CARD_TYPES.iter().filter(|t| seen.contains(**t)).count()
}

/// The cards in exile that were exiled with `host` as it is now (CR 406.6):
/// a [`crate::object::Rider::ExiledWith`] naming its id and its version, so
/// a host that left and came back counts none of them (CR 400.7). Cards
/// only, as the Hearse says ([`GameObject::is_card`]).
fn cards_exiled_with(state: &GameState, host: &GameObject) -> usize {
    let mark = crate::object::Rider::ExiledWith {
        host: host.id,
        version: crate::object::Rider::version_of(host),
    };
    (0..state.players.len())
        .map(|seat| PlayerId::new(u8::try_from(seat).unwrap_or(u8::MAX)))
        .flat_map(|seat| state.zones.list(crate::zone::ZoneLocation::Exile(seat)))
        .filter(|id| {
            state
                .object(**id)
                .is_some_and(|o| o.is_card() && o.riders.contains(&mark))
        })
        .count()
}

#[allow(clippy::too_many_lines)] // the modifier vocabulary is one flat table
fn apply(
    c: &mut Characteristics,
    controller: &mut PlayerId,
    fx: &ContinuousEffect,
    state: &GameState,
    obj: &GameObject,
    read_board: &mut bool,
    layer: Layer,
) {
    let context = crate::text_changes::RuleContext {
        source: fx.source.unwrap_or(obj.id),
        text: state.effect_text(fx),
    };
    match &context.text.modifier(fx.modifier) {
        Modifier::AnimateNoncreatureArtifact => {
            if layer == Layer::Type {
                c.types = c.types.union(TypeSet::ARTIFACT).union(TypeSet::CREATURE);
            } else if layer == Layer::PtSet && c.types.contains(TypeSet::CREATURE) {
                // Read the in-progress values, including layer-1 copies,
                // rather than this object's cache from the last refresh.
                let value = i16::try_from(c.mana_value()).unwrap_or(i16::MAX);
                c.power = Some(value);
                c.toughness = Some(value);
            }
        }
        // Layer 2 (CR 613.1b): whoever controls the effect controls the
        // permanent, for exactly as long as the effect lasts. Never a player
        // who has left the game (CR 800.4b): their effects end as they leave
        // (`sba::eliminate_player`), and this is the door for any that did
        // not.
        Modifier::GainControl => {
            if !state.has_left(fx.controller) {
                *controller = fx.controller;
            }
        }
        Modifier::BecomeCopyOf(id) => {
            // Layer 1: the target's copiable values (CR 707.2), which are
            // its base as other copy effects have rewritten it and nothing
            // else. It used to be `target.characteristics()` — the whole
            // projection — so a Mirror copying a creature that was holding
            // a +1/+1 counter came down a 2/2.
            if let Some(values) = state.copy_snapshots.iter().find(|(id, _)| *id == fx.id)
                .map(|(_, values)| values.clone())
                .or_else(|| copiable_values(state, *id)) {
                *c = (*values).clone();
            }
        }
        Modifier::CharacteristicPT {
            count,
            toughness_plus,
        } => {
            // CR 604.3 and 613.4a: it defines the number, whatever the card
            // printed as its `*`, before anything in 7b–7e reads it.
            let n = pt_count(state, obj, c, fx.controller, *count, context, read_board);
            c.power = Some(n);
            c.toughness = Some(n.saturating_add(i16::from(*toughness_plus)));
        }
        Modifier::ModifyPTPerCount { filter, p, t } => {
            *read_board = true;
            // "For each Aura attached to it" counts by attachment, not by
            // control (CR 303.4e: an Aura's controller is separate from the
            // enchanted object's), so such a filter counts every permanent.
            let whose = (!counts_by_attachment(filter)).then_some(fx.controller);
            let count = count_controlled(state, obj, c, whose, fx.controller, filter, context);
            let count = i16::try_from(count).unwrap_or(i16::MAX);
            if let Some(pow) = &mut c.power {
                *pow = pow.saturating_add(count.saturating_mul(*p));
            }
            if let Some(tou) = &mut c.toughness {
                *tou = tou.saturating_add(count.saturating_mul(*t));
            }
        }
        Modifier::ModifyPTHalfCount(count) => {
            // Half the count, rounded down for power and up for toughness.
            let n = pt_count(state, obj, c, fx.controller, *count, context, read_board).max(0);
            if let Some(pow) = &mut c.power {
                *pow = pow.saturating_add(n / 2);
            }
            if let Some(tou) = &mut c.toughness {
                *tou = tou.saturating_add(n - n / 2);
            }
        }
        Modifier::ModifyPTPerGraveyardCard { filter, p, t } => {
            let count = state
                .zones
                .list(crate::zone::ZoneLocation::Graveyard(fx.controller))
                .iter()
                .filter_map(|id| state.object(*id))
                // "Card" in the graveyard, as `card_types_in_all_graveyards`.
                .filter(|o| o.is_card())
                .filter(|o| crate::eval::matches_with_context(filter, state, o, fx.controller, context))
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
        Modifier::ReplaceCreatureTypes(s) => {
            c.subtypes = c.subtypes.difference(SubtypeSet::ALL_CREATURE);
            c.subtypes.insert(*s);
        }
        Modifier::BecomeType { types, subtype } => {
            let kept = c.types.intersection(baylee_core::types::TypeSet::INSTANT.union(baylee_core::types::TypeSet::SORCERY));
            c.types = types.union(kept);
            c.subtypes = SubtypeSet::EMPTY;
            c.subtypes.insert(*subtype);
        }
        Modifier::AllBasicLandTypes => c.subtypes = c.subtypes.union(SubtypeSet::BASIC_LANDS),
        // CR 305.7: the old land types go and the new one comes, and the
        // land loses every ability its rules text gives it — the keywords
        // here, the rest through `rules_text_lost`, which
        // `GameObject::abilities` reads. Layer 4 comes before layer 6, so a
        // keyword another effect grants still lands, whatever its
        // timestamp. What the land makes is now its new type's mana alone.
        Modifier::SetLandType(s) => set_land_type(c, *s),
        Modifier::SetLandTypeToChosen => {
            if let Some(s) = fx
                .source
                .and_then(|source| state.object(source))
                .and_then(|source| source.chosen_subtype)
            {
                set_land_type(c, s);
            }
        }
        Modifier::AddColor(col) => c.colors = c.colors.union(*col),
        Modifier::SetColor(col) => c.colors = *col,
        Modifier::AddKeyword(k) => c.keywords = c.keywords.union(*k),
        Modifier::RemoveKeyword(k) => c.keywords = c.keywords.difference(*k),
        Modifier::LoseKeywords => c.keywords = KeywordSet::EMPTY,
        Modifier::LoseAllAbilities => {
            c.keywords = KeywordSet::EMPTY;
            c.abilities_lost = Some(Characteristics::lost_at(fx.timestamp));
        }
        // Handled by SBAs/legality checks, not by characteristics.
        Modifier::SpellsCostMore(_)
        | Modifier::AbilitiesCostMore(_)
        | Modifier::LegendRuleOff
        | Modifier::PlayLandsFromGraveyard
        | Modifier::CastPermanentSpellsFromGraveyard
        | Modifier::CastSpellsFromGraveyard
        | Modifier::PermanentOfEachTypeFromGraveyard
        | Modifier::PlayLandsFromLibraryTop
        | Modifier::RevealLibraryTop
        | Modifier::ExtraLandDrops(_)
        | Modifier::CantActivateArtifacts
        | Modifier::ChosenNameCantActivate
        | Modifier::OpponentsCastAsSorcery
        | Modifier::OpponentsCantCast(_)
        | Modifier::CantBeEnchantedExceptSource
        | Modifier::CantBeTargetedBy(_)
        | Modifier::CantBeTargetedByAbilitiesFrom(_)
        | Modifier::DrawLimitPerTurn { .. }
        | Modifier::PlayersCantLose
        | Modifier::CantLoseLife { .. }
        | Modifier::NoLossForZeroLife { .. }
        | Modifier::LifeGainDrawsInstead { .. }
        | Modifier::CantBeAttackedExceptBy { .. }
        | Modifier::PreventDamageToIt
        | Modifier::PreventDamageFromIt
        | Modifier::PreventDamageFrom(_)
        | Modifier::CombatDamageCantBePrevented
        | Modifier::CantBeBlockedBy(_)
        | Modifier::CantAttackUnlessDefenderControls(_)
        | Modifier::LandwalkMatching(_)
        | Modifier::AttacksEachCombat
        | Modifier::CanBlockAdditional(_)
        | Modifier::CanBlockAnyNumber
        | Modifier::MustBeBlockedByAllAble
        | Modifier::BlocksEachAttackerIfAble
        | Modifier::RedirectDamageToYou(_)
        | Modifier::CountersPreventDamage(_)
        | Modifier::OpponentsCantSearch
        | Modifier::NoMaxHandSize
        | Modifier::ProtectionFrom(_)
        | Modifier::GrantsFlashback
        | Modifier::PlayerHexproof
        | Modifier::GrantActivated { .. }
        | Modifier::GrantStatic { .. }
        | Modifier::SorceriesHaveFlash
        | Modifier::GrantTriggered { .. }
        | Modifier::ManaIsAnyColor
        | Modifier::SpendManaAs { .. }
        | Modifier::SearchTakeover
        // CR 613.11: a rule, so there is no characteristic to write. The
        // untap step reads it (`progress::untap_step`).
        | Modifier::SkipUntapStep { .. }
        | Modifier::UntapAtMost { .. }
        | Modifier::AttacksDespiteDefender
        | Modifier::AttacksAsThoughHaste
        | Modifier::DoesNotUntap
        | Modifier::MayChooseNotToUntap
        // A replacement, read where a card would reach a graveyard
        // (`replacement::graveyard_destination`).
        | Modifier::ExileInsteadOfYourGraveyard => {}
        Modifier::ModifyPT(p, t) => {
            if let Some(power) = &mut c.power {
                *power = power.saturating_add(*p);
            }
            if let Some(toughness) = &mut c.toughness {
                *toughness = toughness.saturating_add(*t);
            }
        }
        // CR 613.4b, layer 7b: a *setting* effect gives the permanent that
        // power and toughness outright. The test is whether it is a creature
        // *now* — layer 4 has already run — and not whether it was printed
        // with a P/T box: an animated land has none, and guarding on the
        // printed value left Treetop Village a creature with no toughness,
        // which the state-based actions then put in the graveyard the
        // instant its own ability resolved (CR 704.5f). Anything that is
        // still not a creature has no P/T to set (CR 208.3).
        Modifier::SetPT(p, t) => {
            if c.types.contains(baylee_core::types::TypeSet::CREATURE) {
                c.power = Some(*p);
                c.toughness = Some(*t);
            }
        }
        // CR 613.4b: "has base power N" sets power alone, toughness as the
        // layers before left it.
        Modifier::SetPower(p) => {
            if c.types.contains(baylee_core::types::TypeSet::CREATURE) {
                c.power = Some(*p);
            }
        }
        // CR 613.4b: the granted sentence sets power and toughness to the
        // count, outright like `SetPT` and on a creature only. "You" is the
        // object's own controller as layer 2 left it, because the ability
        // is the object's: a land Druid Class animated, stolen, counts its
        // new controller's lands.
        Modifier::SetPTToCount(count) => {
            if c.types.contains(baylee_core::types::TypeSet::CREATURE) {
                let n = pt_count(state, obj, c, *controller, *count, context, read_board);
                c.power = Some(n);
                c.toughness = Some(n);
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
            origin: crate::effects::EffectOrigin::Resolution,
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
        sort_by_dependency(&all, &mut fxs, Layer::Type);
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
        sort_by_dependency(&all, &mut fxs, Layer::Type);
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
                origin: crate::effects::EffectOrigin::Resolution,
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

    /// Changeling is a whole-kind mask, not a scan: every creature type is
    /// set and nothing of another kind is.
    ///
    /// The count is asked of `kind()` rather than of a range end, because ids
    /// are appended now (#43) and a creature type printed tomorrow will sit
    /// past every other kind's block. A mask holding one id too many would
    /// make a changeling a Saga.
    #[test]
    fn changeling_sets_exactly_the_creature_types() {
        use baylee_core::generated::subtypes;
        use baylee_core::types::SubtypeKind;
        let all = SubtypeSet::ALL_CREATURE;
        assert!(all.contains(subtypes::creature::WIZARD));
        assert!(all.contains(subtypes::creature::ALLY));
        assert!(!all.contains(subtypes::land::FOREST));
        assert!(!all.contains(subtypes::spell::ADVENTURE));
        let creatures = (0..subtypes::COUNT)
            .map(baylee_core::ids::SubtypeId::new)
            .filter(|id| subtypes::kind(*id) == Some(SubtypeKind::Creature))
            .count();
        assert_eq!(all.len() as usize, creatures);
        assert!(
            (0..subtypes::COUNT)
                .map(baylee_core::ids::SubtypeId::new)
                .all(|id| all.contains(id) == (subtypes::kind(id) == Some(SubtypeKind::Creature))),
            "the mask and kind() are the same list read two ways"
        );
    }

    // --- Projection fixtures ------------------------------------------

    struct RegistryLookup;
    impl crate::state::CardLookup for RegistryLookup {
        fn card(
            &self,
            index: baylee_core::ids::CardIndex,
        ) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn fresh() -> GameState {
        use baylee_core::preset::{
            AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
            SeatController, SeatSpec,
        };
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let entry = DeckEntry {
            card: forest,
            print: baylee_core::ids::PrintRef::new(0),
        };
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: (0..60).map(|_| entry).collect(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 4,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    /// A permanent of exactly these types, with this printed body.
    fn permanent(
        state: &mut GameState,
        label: &str,
        types: TypeSet,
        body: Option<(i16, i16)>,
    ) -> ObjectId {
        let name = state.names.intern(label);
        let id = state.create_bare(
            me(),
            crate::object::ObjectKind::Permanent,
            name,
            crate::zone::ZoneLocation::Battlefield,
        );
        let base = state.object_mut(id).expect("just created").base_mut();
        base.types = types;
        base.power = body.map(|(p, _)| p);
        base.toughness = body.map(|(_, t)| t);
        id
    }

    fn register(state: &mut GameState, timestamp: u64, modifier: Modifier) {
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: None,
            controller: me(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: Duration::Indefinitely,
            filter: EffectFilter::Dsl(&ANY_F),
            modifier,
        });
    }

    fn body(state: &GameState, id: ObjectId) -> (Option<i16>, Option<i16>) {
        let obj = state.object(id).expect("in play");
        let c = recompute(state, obj).characteristics;
        (c.power, c.toughness)
    }

    fn artifact_with_cost(state: &mut GameState, cost: &str) -> ObjectId {
        let artifact = permanent(state, "Artifact", TypeSet::ARTIFACT, None);
        state
            .object_mut(artifact)
            .expect("in play")
            .base_mut()
            .mana_cost = baylee_core::mana::ManaCost::parse(cost);
        artifact
    }

    /// CR 613.6: the filter can stop matching after layer 4 without
    /// stopping the same effect's layer-7b portion. A fresh projection
    /// makes the decision afresh instead of reading the previous result.
    #[test]
    fn conditional_animation_continues_after_its_filter_stops_matching() {
        let mut state = fresh();
        let artifact = artifact_with_cost(&mut state, "{2}{U}");
        let animation = fx(
            1,
            1,
            &Filter::NONCREATURE,
            Modifier::AnimateNoncreatureArtifact,
        );
        state.effects.register(animation);
        register(&mut state, 2, Modifier::LoseAllAbilities);

        let plan = LayerPlan::build(&state.effects);
        assert_eq!(plan.layer(Layer::Type), plan.layer(Layer::PtSet));
        for _ in 0..2 {
            state.invalidate_projections();
            state.refresh_characteristics();
            let c = state.object(artifact).expect("in play").characteristics();
            assert_eq!(c.types, TypeSet::ARTIFACT.union(TypeSet::CREATURE));
            assert_eq!((c.power, c.toughness), (Some(3), Some(3)));
            assert!(c.abilities_lost.is_some());
        }
    }

    /// An independent animation applies first even with a later timestamp,
    /// because it changes whether the noncreature condition holds (CR 613.8a).
    #[test]
    fn conditional_animation_waits_for_other_animations_and_resumes_when_they_end() {
        for (conditional_timestamp, other_timestamp) in [(1, 2), (2, 1)] {
            let mut state = fresh();
            let artifact = artifact_with_cost(&mut state, "{5}");
            register(
                &mut state,
                conditional_timestamp,
                Modifier::AnimateNoncreatureArtifact,
            );
            register(
                &mut state,
                other_timestamp,
                Modifier::AddType(TypeSet::CREATURE),
            );
            register(&mut state, other_timestamp, Modifier::SetPT(2, 4));
            assert_eq!(body(&state, artifact), (Some(2), Some(4)));

            state.effects.remove_where(|effect| {
                matches!(effect.modifier, Modifier::AddType(_) | Modifier::SetPT(..))
            });
            assert_eq!(body(&state, artifact), (Some(5), Some(5)));
        }
    }

    /// Two conditional animations form a dependency loop, so the earlier
    /// one starts. The skipped later one's P/T portion must stay skipped,
    /// allowing a setting effect between their timestamps to win in 7b.
    #[test]
    fn only_the_conditional_animation_that_started_can_set_power_and_toughness() {
        let mut state = fresh();
        let artifact = artifact_with_cost(&mut state, "{5}");
        register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);
        register(&mut state, 2, Modifier::SetPT(2, 4));
        register(&mut state, 3, Modifier::AnimateNoncreatureArtifact);
        assert_eq!(body(&state, artifact), (Some(2), Some(4)));
        state.effects.remove_where(|effect| effect.timestamp == 1);
        assert_eq!(body(&state, artifact), (Some(5), Some(5)));
    }

    /// Setting P/T precedes modifiers and counters, but competes with
    /// other settings by its own timestamp (CR 613.4b, 613.7).
    #[test]
    fn conditional_animation_uses_its_timestamp_in_layer_seven_b() {
        for (animation_timestamp, set_timestamp, expected) in [(1, 2, (5, 8)), (2, 1, (7, 9))] {
            let mut state = fresh();
            let artifact = artifact_with_cost(&mut state, "{5}");
            register(
                &mut state,
                animation_timestamp,
                Modifier::AnimateNoncreatureArtifact,
            );
            register(&mut state, set_timestamp, Modifier::SetPT(3, 4));
            register(&mut state, 0, Modifier::ModifyPT(1, 2));
            state.object_mut(artifact).expect("in play").counters.add(
                crate::object::CounterKind::Plus {
                    power: 1,
                    toughness: 2,
                },
                1,
            );
            assert_eq!(body(&state, artifact), (Some(expected.0), Some(expected.1)));
        }
    }

    /// Mana value is read from this projection, including copied values,
    /// without requiring another projection pass through a stale cache.
    #[test]
    fn conditional_animation_uses_the_copy_layers_mana_value() {
        let mut state = fresh();
        let artifact = artifact_with_cost(&mut state, "{3}");
        let original = artifact_with_cost(&mut state, "{7}");
        state.refresh_characteristics();
        let mut copy = fx(1, 2, &ANY_F, Modifier::BecomeCopyOf(original));
        copy.layer = Layer::Copy;
        copy.filter = EffectFilter::object(&state, artifact);
        state.effects.register(copy);
        register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);

        let projected = recompute(&state, state.object(artifact).expect("in play"));
        assert_eq!(projected.characteristics.mana_value(), 7);
        assert_eq!(projected.characteristics.power, Some(7));
        assert_eq!(projected.characteristics.toughness, Some(7));
        assert!(
            !projected.read_board,
            "this object's mana value needs no board read"
        );
    }

    /// CR 202.3b: the native back face's mana value belongs to the front,
    /// even when its castable disturb cost differs. CR 202.3e makes X zero
    /// off the stack.
    #[test]
    fn conditional_animation_uses_front_face_mana_value_and_zero_for_x() {
        let mut state = fresh();
        let transformed = artifact_with_cost(&mut state, "{2}");
        state
            .object_mut(transformed)
            .expect("in play")
            .base_mut()
            .front_mana_value = Some(6);
        let variable = artifact_with_cost(&mut state, "{X}{3}");
        let zero = artifact_with_cost(&mut state, "");
        register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);
        assert_eq!(body(&state, transformed), (Some(6), Some(6)));
        assert_eq!(body(&state, variable), (Some(3), Some(3)));
        assert_eq!(body(&state, zero), (Some(0), Some(0)));
    }

    /// Removing creature from something that is not a creature changes
    /// no applicability. On a printed creature it enables the animation,
    /// so that animation waits for it regardless of their timestamps.
    #[test]
    fn conditional_animation_depends_on_removal_only_when_it_changes_creature_membership() {
        for was_creature in [false, true] {
            for (animation_timestamp, removal_timestamp) in [(1, 2), (2, 1)] {
                let mut state = fresh();
                let artifact = artifact_with_cost(&mut state, "{5}");
                if was_creature {
                    let base = state.object_mut(artifact).expect("in play").base_mut();
                    base.types = base.types.union(TypeSet::CREATURE);
                    base.power = Some(2);
                    base.toughness = Some(4);
                }
                register(
                    &mut state,
                    animation_timestamp,
                    Modifier::AnimateNoncreatureArtifact,
                );
                register(
                    &mut state,
                    removal_timestamp,
                    Modifier::RemoveType(TypeSet::CREATURE),
                );
                let c = recompute(&state, state.object(artifact).expect("in play")).characteristics;
                let animated = was_creature || removal_timestamp < animation_timestamp;
                assert_eq!(c.types.contains(TypeSet::CREATURE), animated);
                assert_eq!(c.power, animated.then_some(5));
                assert_eq!(c.toughness, animated.then_some(5));
            }
        }
    }

    /// Adding artifact to an existing artifact does not change what the
    /// other animation applies to: there is no reverse dependency loop.
    #[test]
    fn conditional_animation_waits_for_artifact_filtered_animation_without_a_false_cycle() {
        let mut state = fresh();
        let artifact = artifact_with_cost(&mut state, "{5}");
        register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);
        state.effects.register(fx(
            1,
            2,
            &Filter::ARTIFACT,
            Modifier::AddType(TypeSet::CREATURE),
        ));
        register(&mut state, 0, Modifier::SetPT(2, 4));
        assert_eq!(body(&state, artifact), (Some(2), Some(4)));
    }

    /// Adding unrelated type effects cannot make a real dependency fall
    /// back to timestamps. The competing animation is in a second bitset
    /// word, after the 64-effect budget of the ordinary shared-plan sort.
    #[test]
    fn conditional_animation_keeps_dependencies_on_large_boards() {
        let mut state = fresh();
        let artifact = artifact_with_cost(&mut state, "{5}");
        register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);
        for timestamp in 2..66 {
            register(&mut state, timestamp, Modifier::AddType(TypeSet::ARTIFACT));
        }
        register(&mut state, 66, Modifier::AddType(TypeSet::CREATURE));
        register(&mut state, 0, Modifier::SetPT(2, 4));
        assert_eq!(
            LayerPlan::build(&state.effects).layer(Layer::Type).len(),
            66
        );
        assert_eq!(body(&state, artifact), (Some(2), Some(4)));
    }

    /// Ignore the two animations' loop before choosing among *all* ready
    /// effects. The earlier animation starts, removal then takes creature
    /// away, and the later animation starts too and wins the P/T timestamp.
    #[test]
    fn conditional_animation_loop_does_not_let_an_unrelated_ready_effect_jump_ahead() {
        let mut state = fresh();
        let artifact = artifact_with_cost(&mut state, "{5}");
        register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);
        register(&mut state, 2, Modifier::RemoveType(TypeSet::CREATURE));
        register(&mut state, 2, Modifier::SetPT(2, 4));
        register(&mut state, 3, Modifier::AnimateNoncreatureArtifact);
        assert_eq!(body(&state, artifact), (Some(5), Some(5)));
    }

    /// Named effects on different objects are independent. Animating the
    /// other artifact must not postpone the older animation until after
    /// the creature-removing effect on this one.
    #[test]
    fn conditional_animation_dependencies_respect_named_object_filters() {
        let mut state = fresh();
        let artifact = artifact_with_cost(&mut state, "{5}");
        let other = artifact_with_cost(&mut state, "{3}");
        let mut animation = fx(1, 1, &ANY_F, Modifier::AnimateNoncreatureArtifact);
        animation.filter = EffectFilter::object(&state, artifact);
        state.effects.register(animation);
        let mut removal = fx(2, 3, &ANY_F, Modifier::RemoveType(TypeSet::CREATURE));
        removal.filter = EffectFilter::object(&state, artifact);
        state.effects.register(removal);
        let mut other_animation = fx(3, 4, &ANY_F, Modifier::AddType(TypeSet::CREATURE));
        other_animation.filter = EffectFilter::object(&state, other);
        state.effects.register(other_animation);
        assert_eq!(body(&state, artifact), (None, None));
        assert!(
            recompute(&state, state.object(other).expect("in play"))
                .characteristics
                .types
                .contains(TypeSet::CREATURE)
        );
    }

    /// CR 205.1b: "becomes a Golem artifact creature" replaces the creature
    /// types it had and keeps every other type and subtype.
    #[test]
    fn a_new_creature_type_replaces_the_old_ones_and_keeps_the_rest() {
        use baylee_core::generated::subtypes::{artifact, creature};
        let mut state = fresh();
        let statue = permanent(
            &mut state,
            "Statue",
            TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            Some((1, 1)),
        );
        {
            let base = state.object_mut(statue).expect("in play").base_mut();
            base.subtypes.insert(creature::HUMAN);
            base.subtypes.insert(artifact::EQUIPMENT);
        }
        register(
            &mut state,
            1,
            Modifier::ReplaceCreatureTypes(creature::GOLEM),
        );
        let obj = state.object(statue).expect("in play");
        let c = recompute(&state, obj).characteristics;
        assert!(c.subtypes.contains(creature::GOLEM));
        assert!(
            !c.subtypes.contains(creature::HUMAN),
            "the old creature type goes"
        );
        assert!(
            c.subtypes.contains(artifact::EQUIPMENT),
            "an artifact type stays"
        );
        assert_eq!(c.types, TypeSet::ARTIFACT.union(TypeSet::CREATURE));
    }

    /// "A creature died" asks what the permanent was on the battlefield
    /// (CR 700.4): a land an effect made a creature dies as a creature,
    /// though the card in the graveyard is only a land.
    #[test]
    fn a_land_that_was_a_creature_as_it_died_is_a_creature_that_died() {
        let mut state = fresh();
        let land = permanent(&mut state, "Animated", TypeSet::LAND, None);
        register(&mut state, 1, Modifier::AddType(TypeSet::CREATURE));
        state.refresh_characteristics();
        assert!(
            state
                .object(land)
                .expect("in play")
                .characteristics()
                .types
                .contains(TypeSet::CREATURE),
            "the effect made it a creature"
        );
        let before = state.per_turn.creatures_died;
        state
            .move_object(
                land,
                crate::zone::ZoneLocation::Graveyard(me()),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("it moves");
        assert_eq!(state.per_turn.creatures_died, before + 1);
    }

    /// CR 613.4: within layer 7 the sublayers run in order, and the order is
    /// the answer — 7b sets, 7c modifies (effects and counters both), 7d
    /// switches. The timestamps here **contradict** it: the anthem is older
    /// than the setting effect, so an implementation that ordered layer 7 by
    /// timestamp alone would add first and then throw the sum away.
    ///
    /// Every stage is asymmetric, so each one is visible in the answer
    /// rather than hidden by a square body.
    #[test]
    fn the_sublayers_of_layer_seven_run_in_the_order_the_rules_give_them() {
        let mut state = fresh();
        let creature = permanent(&mut state, "Bear", TypeSet::CREATURE, Some((1, 1)));

        register(&mut state, 1, Modifier::ModifyPT(1, 0));
        assert_eq!(body(&state, creature), (Some(2), Some(1)), "7c alone");

        register(&mut state, 5, Modifier::SetPT(4, 2));
        assert_eq!(
            body(&state, creature),
            (Some(5), Some(2)),
            "7b first although it is the later effect, then 7c on top of it"
        );

        {
            let counters = &mut state.object_mut(creature).expect("in play").counters;
            counters.add(
                crate::object::CounterKind::Plus {
                    power: 1,
                    toughness: 1,
                },
                1,
            );
            // Asymmetric on purpose. A square change would commute with the
            // switch below, and then this test would pass against a
            // projection that applied the counters *after* it.
            counters.add(
                crate::object::CounterKind::Minus {
                    power: 0,
                    toughness: 1,
                },
                1,
            );
        }
        assert_eq!(
            body(&state, creature),
            (Some(6), Some(2)),
            "and a counter modifies in the same sublayer, after the effects"
        );

        register(&mut state, 9, Modifier::SwitchPT);
        assert_eq!(
            body(&state, creature),
            (Some(2), Some(6)),
            "the switch reads what every earlier sublayer left (CR 613.4d)"
        );
    }

    /// "The number of creatures named Plague Rats on the battlefield": every
    /// controller's, where "you control" counts one side of the table.
    #[test]
    fn a_count_on_the_battlefield_is_everybodys() {
        static RATS: baylee_cards_dsl::Filter = baylee_cards_dsl::Filter::Named("Plague Rats");
        let mut state = fresh();
        let mine = permanent(&mut state, "Plague Rats", TypeSet::CREATURE, Some((0, 0)));
        let _second = permanent(&mut state, "Plague Rats", TypeSet::CREATURE, Some((0, 0)));
        let theirs = permanent(&mut state, "Plague Rats", TypeSet::CREATURE, Some((0, 0)));
        state.object_mut(theirs).expect("in play").controller = PlayerId::new(1);
        let _bear = permanent(&mut state, "Bear", TypeSet::CREATURE, Some((2, 2)));

        register(
            &mut state,
            1,
            Modifier::SetPTToCount(baylee_cards_dsl::PtCount::OnBattlefield(&RATS)),
        );
        assert_eq!(body(&state, mine), (Some(3), Some(3)));
        assert_eq!(body(&state, theirs), (Some(3), Some(3)));

        let mut state = fresh();
        let mine = permanent(&mut state, "Plague Rats", TypeSet::CREATURE, Some((0, 0)));
        let theirs = permanent(&mut state, "Plague Rats", TypeSet::CREATURE, Some((0, 0)));
        state.object_mut(theirs).expect("in play").controller = PlayerId::new(1);
        register(
            &mut state,
            1,
            Modifier::SetPTToCount(baylee_cards_dsl::PtCount::YouControl(&RATS)),
        );
        assert_eq!(body(&state, mine), (Some(1), Some(1)), "the control");
    }

    /// A setting effect asks whether the permanent is a creature **now**,
    /// after layer 4, and not whether it was printed with a P/T box.
    ///
    /// This is the Treetop Village shape. Guarding on the printed value left
    /// an animated land a creature with no toughness, which the state-based
    /// actions put in the graveyard the instant its own ability resolved
    /// (CR 704.5f) — a card that killed itself by working.
    #[test]
    fn a_land_that_layer_four_animated_can_be_given_a_body() {
        let mut state = fresh();
        let land = permanent(&mut state, "Treetop Village", TypeSet::LAND, None);

        register(&mut state, 1, Modifier::SetPT(3, 3));
        assert_eq!(
            body(&state, land),
            (None, None),
            "nothing that is not a creature has a P/T to set (CR 208.3)"
        );

        register(&mut state, 2, Modifier::AddType(TypeSet::CREATURE));
        assert_eq!(
            body(&state, land),
            (Some(3), Some(3)),
            "layer 4 runs first, so layer 7b finds a creature"
        );
    }

    /// Counters are read off the permanent rather than off the effect table,
    /// so they reach a creature that no effect matches at all — and they
    /// reach nothing that is not a creature, because a P/T is a creature's
    /// (CR 208.3). Each counter kind carries its own arithmetic (CR 122.1a),
    /// which is what lets a −0/−1 and a +1/+1 sit on one permanent without
    /// either being a special case.
    #[test]
    fn a_counter_changes_a_body_only_where_there_is_one() {
        use crate::object::CounterKind;
        let mut state = fresh();
        let creature = permanent(&mut state, "Wall of Roots", TypeSet::CREATURE, Some((0, 5)));
        let rock = permanent(&mut state, "Sol Ring", TypeSet::ARTIFACT, None);

        for id in [creature, rock] {
            let counters = &mut state.object_mut(id).expect("in play").counters;
            counters.add(
                CounterKind::Plus {
                    power: 2,
                    toughness: 2,
                },
                1,
            );
            counters.add(
                CounterKind::Minus {
                    power: 0,
                    toughness: 1,
                },
                3,
            );
        }

        assert_eq!(
            body(&state, creature),
            (Some(2), Some(4)),
            "+2/+2 and three −0/−1: power and toughness are summed apart"
        );
        assert_eq!(
            body(&state, rock),
            (None, None),
            "an artifact with a +1/+1 counter on it is still not a creature"
        );
    }

    /// A projection is skipped entirely when nothing could change the
    /// answer, which is what keeps the pass off 2716 objects in an ordinary
    /// game. Counters and changeling are the two reasons that have nothing
    /// to do with the effect table, and both were found the hard way: a
    /// permanent wearing a counter needs the pass even on an empty board.
    #[test]
    fn nothing_is_projected_that_no_effect_and_no_counter_reaches() {
        use crate::object::CounterKind;
        let mut state = fresh();
        let bear = permanent(&mut state, "Bear", TypeSet::CREATURE, Some((2, 2)));

        let plan = LayerPlan::build(&state.effects);
        assert!(plan.is_empty());
        assert!(
            !needs_projection(&state, &plan, state.object(bear).expect("in play")),
            "an empty table over a bare creature is nothing to compute"
        );

        state.object_mut(bear).expect("in play").counters.add(
            CounterKind::Plus {
                power: 1,
                toughness: 1,
            },
            1,
        );
        assert!(
            needs_projection(&state, &plan, state.object(bear).expect("in play")),
            "a counter is a reason of its own — the table says nothing about it"
        );

        let mut state = fresh();
        let bear = permanent(&mut state, "Bear", TypeSet::CREATURE, Some((2, 2)));
        state.object_mut(bear).expect("in play").base_mut().keywords = KeywordSet::CHANGELING;
        assert!(
            needs_projection(
                &state,
                &LayerPlan::build(&state.effects),
                state.object(bear).expect("in play")
            ),
            "and so is changeling, which is a mask rather than an effect"
        );
    }

    #[test]
    fn text_changes_type_line_before_type_effects_and_preserves_copiable_values() {
        use crate::text_changes::TextReplacement;
        use baylee_cards_dsl::TextWordKind;
        use baylee_core::generated::subtypes::land;
        let mut state = fresh();
        let id = permanent(&mut state, "Forest", TypeSet::LAND, None);
        let original_name = state.object(id).unwrap().base.name;
        state.object_mut(id).unwrap().base_mut().subtypes = SubtypeSet::from_slice(&[land::FOREST]);
        let identity = baylee_core::ids::DamageSourceRef {
            object: id,
            version: state.object(id).unwrap().version,
        };
        assert!(state.text_changes.replace(
            identity,
            TextReplacement {
                kind: TextWordKind::BasicLandType,
                from: 4,
                to: 1
            }
        ));
        state.invalidate_projections();
        state.refresh_characteristics();
        let changed = state.object(id).unwrap().characteristics();
        assert_eq!(changed.subtypes, SubtypeSet::from_slice(&[land::ISLAND]));
        assert_eq!(changed.name, original_name);
        assert_eq!(
            crate::casting::intrinsic_mana_colors(&state, id),
            vec![baylee_core::mana::ManaColor::Blue]
        );
        assert_eq!(
            copiable_values(&state, id).unwrap().subtypes,
            SubtypeSet::from_slice(&[land::FOREST])
        );
        register(&mut state, 4, Modifier::SetLandType(land::SWAMP));
        state.refresh_characteristics();
        assert_eq!(
            state.object(id).unwrap().characteristics().subtypes,
            SubtypeSet::from_slice(&[land::SWAMP])
        );
        assert_eq!(
            crate::casting::intrinsic_mana_colors(&state, id),
            vec![baylee_core::mana::ManaColor::Black]
        );
    }

    #[test]
    fn printed_landwalk_changes_while_later_granted_landwalk_does_not() {
        use crate::text_changes::TextReplacement;
        let mut state = fresh();
        let id = permanent(&mut state, "Walker", TypeSet::CREATURE, Some((2, 2)));
        state.object_mut(id).unwrap().base_mut().keywords = KeywordSet::FORESTWALK;
        let identity = baylee_core::ids::DamageSourceRef {
            object: id,
            version: state.object(id).unwrap().version,
        };
        assert!(state.text_changes.replace(
            identity,
            TextReplacement {
                kind: baylee_cards_dsl::TextWordKind::BasicLandType,
                from: 4,
                to: 1
            }
        ));
        register(&mut state, 4, Modifier::AddKeyword(KeywordSet::FORESTWALK));
        state.refresh_characteristics();
        let keywords = state.object(id).unwrap().characteristics().keywords;
        assert!(keywords.contains(KeywordSet::ISLANDWALK));
        assert!(keywords.contains(KeywordSet::FORESTWALK));
    }

    #[test]
    fn text_filter_snapshot_uses_the_ability_words_after_source_changes_again() {
        use crate::text_changes::{RuleContext, TextReplacement};
        use baylee_core::color::{Color, ColorSet};
        static FILTER: Filter = Filter::And(&[
            Filter::CREATURE,
            Filter::HasColor(ColorSet::of(Color::Black)),
        ]);
        let mut state = fresh();
        let source = permanent(&mut state, "Text source", TypeSet::CREATURE, Some((2, 2)));
        let target = permanent(&mut state, "Blue target", TypeSet::CREATURE, Some((2, 2)));
        state.object_mut(target).unwrap().base_mut().colors = ColorSet::of(Color::Blue);
        let identity = baylee_core::ids::DamageSourceRef {
            object: source,
            version: state.object(source).unwrap().version,
        };
        assert!(state.text_changes.replace(
            identity,
            TextReplacement {
                kind: baylee_cards_dsl::TextWordKind::Color,
                from: 2,
                to: 1
            }
        ));
        let captured = RuleContext {
            source,
            text: state.text_changes.get(identity),
        };
        assert!(state.text_changes.replace(
            identity,
            TextReplacement {
                kind: baylee_cards_dsl::TextWordKind::Color,
                from: 1,
                to: 4
            }
        ));
        let object = state.object(target).unwrap();
        assert!(eval::matches_with_context(
            &FILTER,
            &state,
            object,
            me(),
            captured
        ));
        assert!(!eval::matches(&FILTER, &state, object, me(), source));
    }
    #[test]
    fn text_changes_do_not_rewrite_a_dynamically_chosen_land_type() {
        use crate::text_changes::{TextOrigin, TextReplacement};
        use baylee_core::generated::subtypes::land;
        let mut state = fresh();
        let source = permanent(&mut state, "Choice source", TypeSet::ARTIFACT, None);
        let target = permanent(&mut state, "Chosen land", TypeSet::LAND, None);
        state.object_mut(source).unwrap().chosen_subtype = Some(land::FOREST);
        let identity = state.source_identity(source).unwrap();
        assert!(state.text_changes.replace(
            identity,
            TextReplacement {
                kind: baylee_cards_dsl::TextWordKind::BasicLandType,
                from: 4,
                to: 1,
            }
        ));
        let effect = state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(source),
            controller: me(),
            origin: crate::effects::EffectOrigin::Static,
            layer: Layer::Type,
            timestamp: 1,
            duration: Duration::WhileSourceOnBattlefield,
            filter: EffectFilter::object(&state, target),
            modifier: Modifier::SetLandTypeToChosen,
        });
        state
            .effect_text_overrides
            .push((effect, TextOrigin::Live(identity)));
        state.refresh_characteristics();
        assert_eq!(
            state.object(target).unwrap().characteristics().subtypes,
            SubtypeSet::from_slice(&[land::FOREST]),
            "the chosen value is not the word Forest in the ability's text"
        );
        assert_eq!(
            crate::casting::intrinsic_mana_colors(&state, target),
            vec![baylee_core::mana::ManaColor::Green]
        );
    }
}
