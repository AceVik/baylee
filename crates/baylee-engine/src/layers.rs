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

mod counts;
mod reductions;
use counts::{count_controlled, counts_by_attachment, pt_count};

use crate::effects::{ContinuousEffect, EffectFilter, EffectTable};
use crate::eval;
use crate::object::{Characteristics, GameObject};
use crate::state::GameState;
use baylee_cards_dsl::{Filter, KeywordSet, LAYERS, Layer, Modifier};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::{SubtypeSet, TypeSet};
use smallvec::SmallVec;
use std::sync::Arc;

mod index;
mod keyword_instances;
mod projection;
pub(crate) use keyword_instances::keyword_instances;
pub(crate) use projection::recompute_cow;

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
    /// Whether any effect in `ordered` uses a DSL filter rather than `ObjectIs`.
    has_global: bool,
    /// Exact-object ids when `index` is not built and `has_global` is false.
    named: SmallVec<[ObjectId; 8]>,
    /// Exact-object effects are visited only by the object they name.
    index: Option<index::EffectIndex>,
}

impl LayerPlan {
    /// Buckets and dependency-orders every registered effect.
    #[must_use]
    pub fn build(effects: &EffectTable) -> Self {
        Self::build_where(effects, |_| true)
    }

    pub(crate) fn before_entries(
        effects: &EffectTable,
        excluded: &std::collections::BTreeSet<ObjectId>,
    ) -> Self {
        Self::build_where(effects, |effect| {
            effect.origin != crate::effects::EffectOrigin::Static
                || effect
                    .source
                    .is_none_or(|source| !excluded.contains(&source))
        })
    }

    fn build_where(effects: &EffectTable, include: impl Fn(&ContinuousEffect) -> bool) -> Self {
        let mut plan = Self {
            ordered: Vec::with_capacity(effects.len()),
            spans: [(0, 0); LAYER_COUNT],
            conditional_animation: effects
                .iter()
                .any(|fx| crate::artifact_animation::animates(fx.modifier)),
            has_global: false,
            named: SmallVec::new(),
            index: None,
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
                        include(fx)
                            && (fx.layer == layer
                                || crate::artifact_animation::continues_in(fx.modifier, layer))
                    })
                    .map(|(i, _)| i as u32),
            );
            let end = plan.ordered.len() as u32;
            if layer != Layer::Type || !plan.conditional_animation {
                sort_by_dependency(all, &mut plan.ordered[start as usize..end as usize], layer);
            }
            plan.spans[layer as usize] = (start, end);
        }
        plan.index = index::EffectIndex::build(effects, &plan.ordered);
        if plan.index.is_none() {
            for &at in &plan.ordered {
                match all[at as usize].filter {
                    EffectFilter::ObjectIs(id, _) => {
                        if !plan.named.contains(&id) {
                            plan.named.push(id);
                        }
                    }
                    EffectFilter::Dsl(_) | EffectFilter::DslAbout(..) => {
                        plan.has_global = true;
                        plan.named.clear();
                        break;
                    }
                }
            }
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

    fn candidates(&self, layer: Layer, id: ObjectId) -> index::Candidates<'_> {
        // Conditional type dependencies are recomputed against the whole
        // bucket by next_type_effect; keep its reference path unchanged.
        if let Some(index) = &self.index
            && !(layer == Layer::Type && self.conditional_animation)
        {
            let (start, end) = self.spans[layer as usize];
            let (global, local) = index.ranks(id, start, end);
            index::Candidates::Indexed {
                ordered: &self.ordered,
                global,
                local,
            }
        } else {
            index::Candidates::All(self.layer(layer).iter())
        }
    }

    /// The indexed candidates for one object, for non-characteristic rules too.
    pub(crate) fn effects_for<'a>(
        &'a self,
        effects: &'a EffectTable,
        layer: Layer,
        id: ObjectId,
    ) -> impl Iterator<Item = &'a crate::effects::ContinuousEffect> {
        self.candidates(layer, id)
            .map(move |index| &effects.as_slice()[index as usize])
    }

    fn may_affect(&self, id: ObjectId) -> bool {
        if let Some(index) = &self.index {
            index.may_affect(id)
        } else {
            self.has_global || self.named.contains(&id)
        }
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
    plan.may_affect(obj.id)
        || eval::live_context(state, obj.id).text != crate::text_changes::TextChangeMap::IDENTITY
        || !obj.counters.is_empty()
        || obj.base.keywords.contains(KeywordSet::CHANGELING)
        || obj.riders.contains(&crate::object::Rider::Dashed)
        // CR 202.3e: on the stack, X is the number chosen for it, so a
        // spell with X needs its projection even where no effect reaches it.
        || (obj.zone == crate::zone::Zone::Stack && obj.x_value > 0)
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
    let (characteristics, controller, read_board) = recompute_cow(state, obj, plan);
    Projection {
        characteristics: characteristics.into_owned(),
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
            let mut read = |effects: &[Effect]| {
                for effect in effects {
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
                            | ManaSource::ColorsAmong { .. }
                            | ManaSource::ExiledWithSource
                            | ManaSource::CircledColors
                            | ManaSource::LandColor { .. }
                            | ManaSource::Chosen => {}
                        }
                    }
                }
            };
            match ability {
                AbilityDef::Activated { effects, .. }
                | AbilityDef::ActivatedConditional { effects, .. } => read(effects),
                AbilityDef::ModalActivated { modes, .. } => {
                    for mode in *modes {
                        read(mode.effects);
                    }
                }
                _ => {}
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
fn apply_pt_counters(
    c: &mut std::borrow::Cow<'_, Characteristics>,
    obj: &GameObject,
    state: &GameState,
) {
    if !c.types.contains(baylee_core::types::TypeSet::CREATURE) {
        return;
    }
    let (mut dp, mut dt) = (0i16, 0i16);
    for (kind, n) in obj.counters.iter() {
        // Normal placements participate in timestamp order as Counter-origin
        // effects. Direct setup fixtures alone need this aggregate fallback.
        if state.effects.iter().any(|effect| {
            effect.origin == crate::effects::EffectOrigin::Counter(kind)
                && matches!(effect.filter, EffectFilter::ObjectIs(id, version) if id == obj.id && version == obj.version)
        }) {
            continue;
        }
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
    let c = c.to_mut();
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
    if matches!(fx.filter, EffectFilter::ObjectIs(..)) && !fx.filter.names(obj) {
        return false;
    }
    if !crate::effects::source_is_present(state, fx) {
        return false;
    }
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
                resolved_name: None,
                reference: None,
            },
        ),
        EffectFilter::DslAbout(filter, id, version) => eval::matches_projected_with_context(
            filter,
            state,
            obj,
            projected,
            fx.controller,
            crate::text_changes::RuleContext {
                source: fx.source.unwrap_or(obj.id),
                text: state.effect_text(fx),
                resolved_name: None,
                reference: crate::effects::still_the_object(state, *id, *version),
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
    if crate::artifact_animation::animates(dependent.modifier) {
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
    if !crate::artifact_animation::animates(dependent.modifier)
        && !crate::artifact_animation::animates(depended.modifier)
    {
        return depends_on(dependent, depended, Layer::Type);
    }
    let matches = |fx: &ContinuousEffect, c: &Characteristics| {
        applies(state, fx, obj, c)
            && (!crate::artifact_animation::animates(fx.modifier)
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
fn could_change_keywords(modifier: &Modifier) -> bool {
    matches!(
        modifier,
        Modifier::AddKeyword(_)
            | Modifier::RemoveKeyword(_)
            | Modifier::LoseKeywords
            | Modifier::LoseAllLandwalk
            | Modifier::CantHaveKeywords(_)
            | Modifier::LoseAllAbilities
            | Modifier::AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture
            | Modifier::SetLandType(_)
            | Modifier::SetLandTypeToChosen
            | Modifier::SetLandTypeToSecondChosen
            | Modifier::AddKeywordIfCountersAtLeast { .. }
            | Modifier::BecomeCopyOf(_)
    )
}

#[allow(clippy::too_many_lines)] // Exhaustive dependency table, one arm per filter family.
fn could_change_match(modifier: &Modifier, filter: &Filter, layer: Layer) -> bool {
    match filter {
        Filter::Named(_)
        | Filter::NamedBySource
        | Filter::NameOriginallyPrintedIn(_)
        | Filter::SameNameAsLinkedExile => {
            matches!(modifier, Modifier::BecomeCopyOf(_))
        }
        Filter::LowestManaValueOnBattlefield(f) => {
            could_change_match(modifier, f, layer) || matches!(modifier, Modifier::BecomeCopyOf(_))
        }
        // The last two read the object's subtypes as well; what differs is
        // where the subtype they are compared against comes from.
        // Another object's characteristics decide these, so any modifier
        // may move them.
        Filter::AttachedTo(_) | Filter::BlockedThisTurnBy(_) | Filter::SharesAColorWith(_) => true,
        Filter::OfSourceChosenColor => matches!(
            modifier,
            Modifier::AddColor(_) | Modifier::SetColor(_) | Modifier::BecomeCopyOf(_)
        ),
        Filter::HasType(_)
        | Filter::LacksType(_)
        | Filter::HasSubtype(_)
        | Filter::MatchesChosenTypeOfSource
        | Filter::SharesCreatureTypeWithOtherTarget
        | Filter::SharesTypeWithReference(_)
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
                    | Modifier::SetLandTypeToSecondChosen
                    | Modifier::AddChosenBasicLandType
                    | Modifier::BecomeType { .. }
                    | Modifier::AddTypeIfCountersAtLeast { .. }
                    | Modifier::CopyLastExiledCreaturePTAndSubtypes
                    | Modifier::BecomeCopyOf(_)
            ) || (layer == Layer::Type && crate::artifact_animation::animates(*modifier))
        }
        Filter::HasColor(_) | Filter::IsColorless | Filter::Monocolored => matches!(
            modifier,
            Modifier::AddColor(_) | Modifier::SetColor(_) | Modifier::BecomeCopyOf(_)
        ),
        Filter::HasKeyword(_) => could_change_keywords(modifier),
        Filter::ToughnessAtMost(_)
        | Filter::ToughnessAtLeast(_)
        | Filter::PowerAtLeast(_)
        | Filter::PowerAtMost(_)
        | Filter::PowerAtMostSourcePower
        | Filter::PowerLessThanSourcePower
        | Filter::ToughnessLessThanSourcePower => {
            matches!(
                modifier,
                Modifier::HoneEquipment
                    | Modifier::ModifyPT(..)
                    | Modifier::SetPT(..)
                    | Modifier::SetPower(_)
                    | Modifier::SetToughness(_)
                    | Modifier::SetPTToCount(_)
                    | Modifier::CopyLastExiledCreaturePTAndSubtypes
                    | Modifier::SwitchPT
                    | Modifier::CharacteristicPT { .. }
                    | Modifier::CharacteristicPTSeparate { .. }
                    | Modifier::ModifyPTPerCount { .. }
                    | Modifier::ModifyPTByAmount { .. }
                    | Modifier::ModifyPTByCount(_)
                    | Modifier::ModifyPTHalfCount(_)
                    | Modifier::ReducePTByCount { .. }
                    | Modifier::ModifyPTPerGraveyardCard { .. }
                    | Modifier::BecomeCopyOf(_)
            ) || (layer == Layer::PtSet && crate::artifact_animation::animates(*modifier))
        }
        // Layer 2 moves a permanent from one side of the table to the
        // other, which is the whole of what these read: a change of
        // control also restarts how long it has been controlled (CR 302.6).
        Filter::ControlledByYou
        | Filter::ControlledByOpponent
        | Filter::ControlledByActivePlayer
        | Filter::ControlledByDefendingPlayer
        | Filter::NotControlledByReferenceController
        | Filter::ControlledSinceTurnBegan => matches!(modifier, Modifier::GainControl),
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| could_change_match(modifier, f, layer))
        }
        Filter::Not(f) => could_change_match(modifier, f, layer),
        // Nothing in the modifier vocabulary changes any of these. Ownership
        // and tokenhood are fixed for as long as the object exists (CR
        // 108.3, CR 111.1); tapped, attacking, attachment and zone are game
        // state rather than a characteristic a continuous effect writes; mana value comes off the printed
        // cost, which only copying rewrites — and a copy effect's own filter
        // names an object rather than reading one of these. A *name* is the
        // same case as mana value: layer 3 (CR 613.1c) writes one and this
        // vocabulary has no such modifier, `BecomeCopyOf` being layer 1 and
        // applied before any of this. The day a text-changing modifier
        // exists, `Named` is what leaves this list.
        Filter::HasSupertype(_) => matches!(
            modifier,
            Modifier::AddSupertype(_) | Modifier::RemoveSupertype(_)
        ),
        Filter::HasRampage | Filter::HasCumulativeUpkeep | Filter::HasManaAbility => {
            matches!(layer, Layer::Ability | Layer::Type)
        }
        Filter::ActivatedAbilityFrom(filter) => could_change_match(modifier, filter, layer),
        Filter::TargetsPermanentYouControl => matches!(layer, Layer::Control),
        Filter::Any
        | Filter::IsCommander
        | Filter::HasAdventure
        | Filter::This
        | Filter::Another
        | Filter::IsToken
        | Filter::WithSingleTarget
        | Filter::OwnedBySourceOwner
        | Filter::OwnedByYou
        | Filter::Tapped
        | Filter::Untapped
        | Filter::AttackingYou
        | Filter::Attacking
        | Filter::BandedWithSource
        | Filter::NotTargetedByAnotherNamed(_)
        | Filter::Blocking
        | Filter::Blocked
        | Filter::Unblocked
        | Filter::EnteredThisTurn
        | Filter::PutIntoGraveyardThisTurn
        | Filter::AttackedThisTurn
        | Filter::CountersAtLeast(_, _)
        | Filter::HasAnyCounter
        | Filter::Enchanted
        | Filter::Equipped
        | Filter::CmcEqualsCountersOnSource(_)
        | Filter::CmcEqualsYourUnspentMana
        | Filter::BlockingSource
        | Filter::SpellTargetingAffected
        | Filter::BlockedBySource
        | Filter::Reference
        | Filter::BlockingReference
        | Filter::BlockedByReferenceThisTurn
        | Filter::BlockedReferenceThisTurn
        | Filter::DealtDamageToSourceThisTurn
        | Filter::DealtDamageBySourceThisTurn
        | Filter::HasCounter(_)
        | Filter::AttachedToBySource
        | Filter::AttachedToSource
        | Filter::IsAttached
        | Filter::CmcAtMost(_)
        | Filter::CmcAtMostX
        | Filter::CmcExactlyX
        | Filter::CmcAtMostColorsSpent
        | Filter::CmcAtLeast(_)
        | Filter::InZone(_)
        | Filter::Kicked
        | Filter::CastFromHand
        | Filter::HasWatermark
        | Filter::SharesWatermarkWithSource
        | Filter::BlockedByReference
        | Filter::DealtDamageThisTurn
        | Filter::BlockedThisTurn
        | Filter::AttackedDuringControllersLastTurn
        | Filter::TwiceCmcExactlyX
        | Filter::CouldHaveAttackedThisTurn
        | Filter::ControllerControls(_)
        | Filter::TappedToPayForSourceThisTurn
        | Filter::UnCard => false,
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
        resolved_name: None,
        reference: None,
    };
    match &context.text.modifier(fx.modifier) {
        Modifier::AnimateNoncreatureArtifact
        | Modifier::AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture => {
            if layer == Layer::Type {
                c.types = c.types.union(TypeSet::ARTIFACT).union(TypeSet::CREATURE);
            } else if layer == Layer::Ability {
                c.keywords = KeywordSet::EMPTY;
                c.abilities_lost = Some(Characteristics::lost_at(fx.timestamp));
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
        Modifier::CharacteristicPTSeparate { power, toughness } => {
            let p = pt_count(state, obj, c, fx.controller, *power, context, read_board);
            let t = pt_count(state, obj, c, fx.controller, *toughness, context, read_board);
            c.power = Some(p);
            c.toughness = Some(t);
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
        Modifier::ModifyPTByAmount { power, toughness } => {
            *read_board = true;
            let signed = |amount: &baylee_cards_dsl::Amount| {
                let n = crate::eval::amount_with_context(amount, state, fx.controller, context, None);
                let n = i16::try_from(n).unwrap_or(i16::MAX);
                if amount.is_negative() { -n } else { n }
            };
            if let Some(p) = &mut c.power { *p = p.saturating_add(signed(power)); }
            if let Some(t) = &mut c.toughness { *t = t.saturating_add(signed(toughness)); }
        }
        Modifier::ModifyPTByCount(count) => {
            let n = pt_count(state, obj, c, fx.controller, *count, context, read_board);
            if let Some(power) = &mut c.power { *power = power.saturating_add(n); }
            if let Some(toughness) = &mut c.toughness { *toughness = toughness.saturating_add(n); }
        }
        Modifier::ReducePTByCount { count, minimum_toughness } => {
            let count = pt_count(state, obj, c, fx.controller, *count, context, read_board);
            reductions::reduce_pt(c, count, *minimum_toughness);
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
        Modifier::AddSupertype(types) => c.supertypes = c.supertypes.union(*types),
        Modifier::RemoveSupertype(types) => c.supertypes = c.supertypes.difference(*types),
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
        Modifier::SetLandTypeToSecondChosen => {
            if let Some(subtype) = fx.source.and_then(|source| state.object(source))
                .and_then(|source| source.riders.iter().find_map(|rider| match rider {
                    crate::object::Rider::SecondChosenSubtype(subtype) => Some(*subtype),
                    _ => None,
                }))
            {
                set_land_type(c, subtype);
            }
        }
        Modifier::AddChosenBasicLandType => {
            if let Some(subtype) = fx.source.and_then(|id| state.object(id))
                .and_then(|source| source.chosen_subtype)
            {
                c.subtypes.insert(subtype);
            }
        }
        Modifier::AddColor(col) => c.colors = c.colors.union(*col),
        Modifier::SetColor(col) => c.colors = *col,
        Modifier::AddKeyword(k) => c.keywords = c.keywords.union(*k),
        Modifier::RemoveKeyword(k) => c.keywords = c.keywords.difference(*k),
        Modifier::LoseKeywords => c.keywords = KeywordSet::EMPTY,
        Modifier::CantHaveKeywords(bits) => c.keywords = c.keywords.difference(*bits),
        Modifier::LoseAllLandwalk => c.keywords = c.keywords.difference(
            KeywordSet::PLAINSWALK.union(KeywordSet::ISLANDWALK)
                .union(KeywordSet::SWAMPWALK).union(KeywordSet::MOUNTAINWALK)
                .union(KeywordSet::FORESTWALK)),
        Modifier::LoseAllAbilities => {
            c.keywords = KeywordSet::EMPTY;
            c.abilities_lost = Some(Characteristics::lost_at(fx.timestamp));
        }
        // Handled by SBAs/legality checks, not by characteristics.
        Modifier::SpellsCostLess(_)
        | Modifier::LoseAllBandsWithOther
        | Modifier::BandsWithOther(_)
        | Modifier::SpellsCostMore(_)
        | Modifier::AbilitiesCostMore(_)
        | Modifier::ManaAbilitiesCostLife(_)
        | Modifier::ReplaceTappedMana { .. }
        | Modifier::AbilitiesCostLess { .. }
        | Modifier::EquipCostsLess(_)
        | Modifier::CantActivateNonMana
        | Modifier::RegenerateInsteadOfDestroy
        | Modifier::CantBeRegenerated
        | Modifier::LegendRuleOff
        | Modifier::PlayLandsFromGraveyard
        | Modifier::CastPermanentSpellsFromGraveyard
        | Modifier::CastSpellsFromGraveyard
        | Modifier::PermanentOfEachTypeFromGraveyard
        | Modifier::PlayLandsFromLibraryTop
        | Modifier::RevealAllLibraryTops
        | Modifier::RevealAllHands
        | Modifier::RevealYourHand
        | Modifier::RevealLibraryTop
        | Modifier::ExtraLandDrops(_)
        | Modifier::CantActivateArtifacts
        | Modifier::CantActivate
        | Modifier::ChosenNameCantActivate
        | Modifier::OpponentsCastAsSorcery
        | Modifier::OpponentsCantCast(_)
        | Modifier::PlayersCantCast { .. }
        | Modifier::PlayersCantPlay(_)
        | Modifier::CantBeEnchantedExceptSource
        | Modifier::CantBeTargetedBy(_)
        | Modifier::CantBecomeEnchanted
        | Modifier::OtherPlayersCantGainControl
        | Modifier::CantBeTargetedByWallOnly
        | Modifier::CantBeTargetedByAbilitiesFrom(_)
        | Modifier::DrawLimitPerTurn { .. }
        | Modifier::PlayersCantLose
        | Modifier::CantLoseLife { .. }
        | Modifier::NoLossForZeroLife { .. }
        | Modifier::LifeGainDrawsInstead { .. }
        | Modifier::CantBeAttackedExceptBy { .. }
        // Damage results and counter placements consume these replacements;
        // they do not alter a projected characteristic.
        | Modifier::ReplacePlayerDamage { .. }
        | Modifier::DamageLifeLossFloor { .. }
        | Modifier::AdditionalCounterPlacement { .. }
        | Modifier::NextDestructionClearsDamage
        | Modifier::PreventDamageToIt
        | Modifier::PreventDamageFromIt
            | Modifier::PreventAllDamageFromIt
            | Modifier::PreventDamageToYouFrom(_)
        | Modifier::PreventCombatDamageFrom(_)
        | Modifier::PreventDamageFrom(_)
        | Modifier::CombatDamageCantBePrevented
        | Modifier::CantBlock(_)
        | Modifier::CantBeBlockedBy(_)
        | Modifier::CantBeBlockedByPlayer(_)
        | Modifier::CantAttack
        | Modifier::AttackWithoutTappingWhileSourceUntapped { .. }
        | Modifier::CantAttackUnlessDefenderControls(_)
        | Modifier::CantAttackPlayerIdleLastTurn
        | Modifier::LandwalkMatching(_)
        | Modifier::LandwalkOfSubtype { .. }
        | Modifier::AttacksEachCombat
        | Modifier::MaxAttackersPerCombat(_)
        | Modifier::MaxBlockersPerCombat(_)
        | Modifier::CantBeEnchanted
        | Modifier::CanBlockAdditional(_)
        | Modifier::CanBlockAnyNumber
        | Modifier::MustBeBlockedByAllAble
        | Modifier::MustBeBlockedByAllAbleMatching(_)
        | Modifier::AssignsNoCombatDamage
        | Modifier::BlocksEachAttackerIfAble
        | Modifier::RedirectDamageToYou(_)
        | Modifier::CountersPreventDamage(_)
        | Modifier::OpponentsSearchTop(_)
        | Modifier::OpponentsCantSearch
        | Modifier::NoMaxHandSize
            | Modifier::ChosenPlayerMaxHandSize(_)
        | Modifier::ProtectionFrom(_)
        | Modifier::GrantsFlashback
        | Modifier::GrantsEscape(_)
        | Modifier::GraveyardInstantSorceryOnce
        | Modifier::GraveyardVampireOnce
        | Modifier::NextSpellUncounterable
        | Modifier::OneSpellManaAnyType
        | Modifier::NextDrawWish
        | Modifier::NextDrawFromTop { .. }
        | Modifier::CreatureAbilityManaAnyColor
        | Modifier::BlockableWithoutLandwalk(_)
        | Modifier::BlockableWithoutAnyLandwalk
        | Modifier::IgnoreOpponentsHexproof
        | Modifier::CounterMaximum { .. }
        | Modifier::CrewSaddlePowerBonus(_)
        | Modifier::BoastTwice
        | Modifier::LifeTotalCantChange { .. }
        | Modifier::PlayerProtectionFromEverything { .. }
        | Modifier::PlayerProtectionFromOpponents { .. }
        | Modifier::DoubleCountersPlacedByYou
        | Modifier::CantAttackYou
        | Modifier::AttackTaxLife { .. }
        | Modifier::SpellCastLimitPerTurn { .. }
        | Modifier::PlayerHexproof { .. }
        | Modifier::GrantActivated { .. }
        | Modifier::GrantActivatedDefinition(_)
        | Modifier::GrantActivatedFromLinkedExile(_)
        | Modifier::GrantStatic { .. }
        | Modifier::SpellsHaveFlash(_)
        | Modifier::SorceriesHaveFlash
        | Modifier::GrantTriggered { .. }
        | Modifier::GrantTriggeredUpToOne { .. }
        | Modifier::ManaIsAnyColor
        | Modifier::SpendManaAs { .. }
        | Modifier::SearchTakeover
        // CR 613.11: a rule, so there is no characteristic to write. The
        // untap step reads it (`progress::untap_step`).
        | Modifier::SkipUntapStep { .. }
        | Modifier::SkipDrawStep { .. }
        | Modifier::UntapAtMost { .. }
        | Modifier::AttacksDespiteDefender
        | Modifier::AttacksAsThoughHaste
        | Modifier::AttacksAloneOnly
        | Modifier::RemoveCountersInsteadOfUntapping(_)
        | Modifier::DoesNotUntap
        | Modifier::MayChooseNotToUntap
        | Modifier::MayRedirectDamageToYou
        | Modifier::CantAttackUnlessPayMana(_)
        | Modifier::CantAttackUnlessYouSacrifice { .. }
        | Modifier::DamageToItCantBePreventedOrRedirected
        | Modifier::CantEnterBattlefield(_)
        | Modifier::SpellsCostMoreMana(_)
        | Modifier::CantAttackIfDefenderControls(_)
        // A replacement, read where a card would reach a graveyard
        // (`replacement::graveyard_destination`).
        | Modifier::ExileInsteadOfYourGraveyard
        | Modifier::ExileInsteadOfLeavingBattlefield => {}
        Modifier::HoneEquipment => {
            *read_board = true;
            if let Some(equipment) = fx.source.and_then(|id| state.object(id))
                && equipment.zone == crate::zone::Zone::Battlefield
                && !equipment.status.contains(crate::object::Status::PHASED_OUT)
                && equipment.characteristics().subtypes.contains(baylee_core::generated::subtypes::artifact::EQUIPMENT)
                && let Some(power) = &mut c.power
            {
                let n = equipment.counters.get(crate::object::CounterKind::Hone);
                *power = power.saturating_add(i16::try_from(n).unwrap_or(i16::MAX));
            }
        }
        Modifier::ModifyPT(p, t) => {
            let count = if let crate::effects::EffectOrigin::Counter(kind) = fx.origin {
                i16::try_from(obj.counters.get(kind)).unwrap_or(i16::MAX)
            } else { 1 };
            if let Some(power) = &mut c.power {
                *power = power.saturating_add(p.saturating_mul(count));
            }
            if let Some(toughness) = &mut c.toughness {
                *toughness = toughness.saturating_add(t.saturating_mul(count));
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
        Modifier::SetToughness(t) => {
            if c.types.contains(baylee_core::types::TypeSet::CREATURE) {
                c.toughness = Some(*t);
            }
        }
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
        Modifier::CopyLastExiledCreaturePTAndSubtypes => {
            *read_board = true;
            if let Some(exiled) = crate::eval::linked_exile_cards(
                state,
                fx.controller,
                context,
                None,
                &Filter::CREATURE,
            )
            .into_iter()
            .filter_map(|id| state.object(id))
            .max_by_key(|o| o.timestamp)
            {
                let projected;
                let chars = if state.off_battlefield_pt_cda(exiled).is_some() {
                    projected = recompute(state, exiled);
                    &projected.characteristics
                } else {
                    exiled.characteristics()
                };
                if layer == Layer::Type {
                    c.subtypes = c
                        .subtypes
                        .difference(SubtypeSet::ALL_CREATURE)
                        .union(
                            chars
                                .subtypes
                                .difference(SubtypeSet::ALL.difference(SubtypeSet::ALL_CREATURE)),
                        );
                    c.subtypes
                        .insert(baylee_core::generated::subtypes::creature::SHAPESHIFTER);
                } else if layer == Layer::PtSet && c.types.contains(TypeSet::CREATURE) {
                    c.power = chars.power;
                    c.toughness = chars.toughness;
                }
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

fn prohibited_keywords(modifier: &Modifier) -> KeywordSet {
    match modifier {
        Modifier::CantHaveKeywords(bits) => *bits,
        _ => KeywordSet::EMPTY,
    }
}

#[cfg(test)]
mod tests;
