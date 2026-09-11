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

/// Whether a continuous effect carrying this modifier fixes the set of
/// objects it applies to at the moment it begins (CR 611.2c).
///
/// The rule draws its line at what the effect *does*: one that modifies
/// characteristics or changes control affects the objects that were there
/// when it began and no others, so "all creatures get -2/-2 until end of
/// turn" leaves a creature that arrives afterwards alone. One that does
/// neither — a prevention shield, a rule about what players may do — keeps
/// applying to whatever comes along, which is why this cannot be answered
/// by the effect's `Layer`: the layer says where an effect is applied and
/// every variant here names one, including the modifiers that change no
/// characteristic at all.
///
/// Exhaustive deliberately, the way [`crate::layers`]'s dependency table is:
/// a `_ => false` arm would answer "keeps matching for ever" for every
/// modifier added from here on, which is the direction that is silent —
/// the effect simply goes on catching permanents nobody cast it at.
#[must_use]
pub fn locks_its_set(modifier: &Modifier) -> bool {
    match modifier {
        // Characteristics: types, colors, abilities, P/T — and control,
        // which the rule names beside them.
        Modifier::AddType(_)
        | Modifier::RemoveType(_)
        | Modifier::AddSubtype(_)
        | Modifier::AllCreatureTypes
        | Modifier::AllBasicLandTypes
        | Modifier::AddColor(_)
        | Modifier::SetColor(_)
        | Modifier::AddKeyword(_)
        | Modifier::RemoveKeyword(_)
        | Modifier::LoseKeywords
        | Modifier::ProtectionFrom(_)
        | Modifier::BecomeCopyOf(_)
        | Modifier::GrantsFlashback
        | Modifier::GainControl
        | Modifier::AddTypeIfCountersAtLeast { .. }
        | Modifier::AddKeywordIfCountersAtLeast { .. }
        | Modifier::GrantActivated { .. }
        | Modifier::GrantTriggered { .. }
        | Modifier::ModifyPTPerCount { .. }
        | Modifier::ModifyPT(..)
        | Modifier::SetPT(..)
        | Modifier::SwitchPT => true,
        // Neither: a shield that prevents damage, and the rules a player
        // plays under. Teferi's `SorceriesHaveFlash` is the clearest of
        // them — it is about its controller's spells, and a set of objects
        // fixed at resolution would mean the cards in hand at that moment.
        Modifier::LegendRuleOff
        | Modifier::CantActivateArtifacts
        | Modifier::OpponentsCastAsSorcery
        | Modifier::PlayersCantLose
        | Modifier::CantLoseLife
        | Modifier::PreventDamageToIt
        | Modifier::PreventDamageFromIt
        | Modifier::OpponentsCantSearch
        | Modifier::NoMaxHandSize
        | Modifier::PlayerHexproof
        | Modifier::SorceriesHaveFlash
        | Modifier::ManaIsAnyColor
        | Modifier::SearchTakeover => false,
    }
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

/// The activated ability a continuous effect grants `source`, if any.
///
/// One function with three readers, and that is the point of it existing.
/// [`crate::Engine::legal_actions`] offers this ability under the synthetic
/// index `choice::GRANTED_ABILITY`, `start_granted` runs it when the answer
/// comes back, and the view projects what it makes so a client can plan mana
/// through it. Written out three times, an offer and a projection that
/// disagreed would be a land the planner counts on and the engine refuses.
///
/// Registration order is slot order, and it has to be: the offer numbers the
/// grants it finds and the activation decodes that number back, so the two
/// walks must agree on what "the second one" means. The effect table is
/// append-only within a game, which is what makes the order stable.
pub fn granted_activated(
    state: &crate::state::GameState,
    source: ObjectId,
) -> impl Iterator<Item = GrantedAbility> {
    let obj = state.object(source);
    state.effects.iter().filter_map(move |fx| {
        let obj = obj?;
        let Modifier::GrantActivated {
            cost,
            effects,
            mana_ability,
        } = &fx.modifier
        else {
            return None;
        };
        let applies = match &fx.filter {
            EffectFilter::ObjectIs(id) => *id == source,
            EffectFilter::Dsl(filter) => crate::eval::matches(
                filter,
                state,
                obj,
                fx.controller,
                fx.source.unwrap_or(source),
            ),
        };
        applies.then_some(GrantedAbility {
            cost: *cost,
            effects,
            mana_ability: *mana_ability,
        })
    })
}

/// One granted activated ability, as the engine and the view both read it.
#[derive(Clone, Copy, Debug)]
pub struct GrantedAbility {
    /// What activating it costs.
    pub cost: baylee_cards_dsl::cost::Cost,
    /// What it does.
    pub effects: &'static [baylee_cards_dsl::effect::Effect],
    /// Whether it is a mana ability (CR 605.1) and so uses no stack.
    pub mana_ability: bool,
}
