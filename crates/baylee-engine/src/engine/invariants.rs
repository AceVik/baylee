//! What a fuzzer may ask of a running engine between two answers.
//!
//! Compiled for this crate's tests and under the non-default `fuzz` feature,
//! never into a shipped build: `docs/verification-hooks.md` is the contract
//! the fuzzer builds against.

use super::Engine;
use crate::effects::EffectFilter;
use crate::layers::{self, LayerPlan};
use crate::object::GameObject;
use crate::state::{CardLookup, GameState};
use crate::zone::ZoneLocation;

impl<L: CardLookup> Engine<L> {
    /// Whether the cached layered projection equals a from-scratch recompute
    /// on the current state.
    ///
    /// The projection is cached behind one `u64` generation compare
    /// (`GameState::refresh_characteristics`), so a change that moves a
    /// characteristic input without invalidating leaves every reader looking
    /// at the old value while the generation says all is well. This asks the
    /// content rather than the number: every object a refresh is responsible
    /// for is projected again through the layers and compared, field for
    /// field, with what `GameObject::characteristics` and
    /// `GameObject::controller` answer now.
    ///
    /// Read-only and deterministic; it writes nothing, not even a cache.
    #[must_use]
    pub fn projection_is_fresh(&self) -> bool {
        projection_is_fresh(&self.state)
    }
}

/// [`Engine::projection_is_fresh`] on a bare state.
///
/// The objects checked are the ones `refresh_characteristics` projects:
/// the battlefield and the spells on the stack always; every object while an
/// effect reaches into another zone; and any other object that is holding a
/// cached projection, which only a cross-zone refresh leaves behind and the
/// next refresh has to clear (#120).
pub(crate) fn projection_is_fresh(state: &GameState) -> bool {
    let plan = LayerPlan::build(&state.effects);
    let cross_zone = state.effects.iter().any(|fx| {
        matches!(fx.filter, EffectFilter::Dsl(f) if crate::state::filter_reaches_other_zones(f))
    });
    let on_board = |id| {
        state.zones.list(ZoneLocation::Battlefield).contains(&id)
            || state.zones.stack_projectable().contains(&id)
    };
    state.arena.iter().all(|(id, obj)| {
        let refreshed = cross_zone || obj.cache.value().is_some() || on_board(id);
        !refreshed || agrees_with_a_recompute(state, obj, &plan)
    })
}

/// Whether one object's cached projection is what the layers make of it now.
fn agrees_with_a_recompute(state: &GameState, obj: &GameObject, plan: &LayerPlan) -> bool {
    if layers::needs_projection(plan, obj) {
        let fresh = layers::recompute_with(state, obj, plan);
        *obj.characteristics() == fresh.characteristics && obj.controller == fresh.controller
    } else {
        // What the refresh writes for an object nothing can reach: no cache,
        // so the base answers, and the base controller.
        *obj.characteristics() == *obj.base && obj.controller == obj.base_controller
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit::{
        Duel, basic_forest, keep_mulligans, on_battlefield, quiet_creature, reach_main_phase,
    };
    use baylee_cards_dsl::CounterKind;
    use baylee_core::ids::PlayerId;

    /// A settled board is fresh; a counter put on a creature past the door
    /// that invalidates (`replacement::record_counters`) leaves the cached
    /// body behind with the generation unmoved, and that is what the check
    /// exists to see; invalidating and refreshing makes it fresh again.
    ///
    /// The injection is the skipped invalidation itself rather than an
    /// effect registered and not yet refreshed, because the latter moves the
    /// generation and a check that only compared the `u64` would catch it
    /// too. This one can only be caught by reading the content.
    #[test]
    fn a_skipped_invalidation_is_a_stale_projection() {
        let p0 = PlayerId::new(0);
        let creature = quiet_creature();
        let mut engine = Duel::new(7, basic_forest())
            .battlefield(0, &[creature])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        assert!(engine.projection_is_fresh(), "a settled board is fresh");

        let id = on_battlefield(&engine, p0, creature).expect("the creature is out");
        let before = engine
            .state
            .object(id)
            .expect("exists")
            .characteristics()
            .clone();
        engine
            .state
            .object_mut(id)
            .expect("exists")
            .counters
            .add(CounterKind::P1P1, 1);
        assert_eq!(
            *engine.state.object(id).expect("exists").characteristics(),
            before,
            "the injection skipped the invalidation, so the cache still answers the old body"
        );
        assert!(
            !engine.projection_is_fresh(),
            "a counter nobody invalidated for is a stale projection"
        );

        engine.state.invalidate_projections();
        engine.state.refresh_characteristics();
        assert!(
            engine.projection_is_fresh(),
            "invalidating and refreshing settles it again"
        );
        assert_ne!(
            *engine.state.object(id).expect("exists").characteristics(),
            before,
            "and the refresh did read the counter"
        );
    }
}
