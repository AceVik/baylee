//! The effect table: registered continuous effects and their lifetimes.
//!
//! Effects are first-class objects with a source, a layer, a duration, and
//! a filter. Removal is structural: effects with
//! [`Duration::WhileSourceOnBattlefield`] are dropped when their source
//! leaves the battlefield, so anthems remove themselves — card code never
//! has to remember to clean up. The `generation` counter drives the
//! characteristic-projection cache: one integer compare per cache hit.

use baylee_cards_dsl::{Duration, Filter, Layer, Modifier};
use baylee_core::ids::{EffectId, ObjectId, PlayerId};

/// Which objects a continuous effect applies to.
#[derive(Clone, Copy, Debug)]
pub enum EffectFilter {
    /// A declarative DSL filter.
    Dsl(&'static Filter),
    /// Exactly one object (created effects like Giant Growth).
    ObjectIs(ObjectId),
}

/// A registered continuous effect.
#[derive(Clone, Debug)]
pub struct ContinuousEffect {
    /// Effect handle.
    pub id: EffectId,
    /// The permanent/spell/emblem that created this effect.
    pub source: Option<ObjectId>,
    /// The player who controls the effect (for "you"/"opponent" filters).
    pub controller: PlayerId,
    /// The layer it applies in.
    pub layer: Layer,
    /// Registration timestamp (effects ordering within a layer).
    pub timestamp: u64,
    /// Its lifetime.
    pub duration: Duration,
    /// Which objects are affected.
    pub filter: EffectFilter,
    /// What it changes.
    pub modifier: Modifier,
}

/// All currently registered continuous effects.
#[derive(Clone, Debug, Default)]
pub struct EffectTable {
    effects: Vec<ContinuousEffect>,
    next_id: u32,
    /// Bumped on every add/remove — the projection cache key.
    pub generation: u64,
}

impl EffectTable {
    /// Registers an effect; bumps the generation.
    pub fn register(&mut self, mut fx: ContinuousEffect) -> EffectId {
        let id = EffectId::new(self.next_id);
        self.next_id += 1;
        fx.id = id;
        self.effects.push(fx);
        self.generation += 1;
        id
    }

    /// Removes effects matching a predicate; bumps the generation if any.
    pub fn remove_where(&mut self, pred: impl Fn(&ContinuousEffect) -> bool) {
        let before = self.effects.len();
        self.effects.retain(|fx| !pred(fx));
        if self.effects.len() != before {
            self.generation += 1;
        }
    }

    /// All active effects (registration order).
    pub fn iter(&self) -> impl Iterator<Item = &ContinuousEffect> {
        self.effects.iter()
    }

    /// All active effects as a slice, so callers can address them by index.
    ///
    /// `layers::LayerPlan` orders indices rather than references: a plan
    /// holding borrows of this table could not coexist with the `&mut
    /// GameState` that stores the projection results.
    #[must_use]
    pub fn as_slice(&self) -> &[ContinuousEffect] {
        &self.effects
    }

    /// Whether any effect is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Whether an effect from `source` with `ability_index` is registered
    /// (static-ability sync).
    #[must_use]
    pub fn has_source_ability(&self, source: ObjectId, modifier: Modifier) -> bool {
        self.effects
            .iter()
            .any(|fx| fx.source == Some(source) && fx.modifier == modifier)
    }

    /// Number of registered effects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.effects.len()
    }
}
