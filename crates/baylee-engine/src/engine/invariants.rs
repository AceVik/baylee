//! What a fuzzer may ask of a running engine between two answers.
//!
//! Compiled for this crate's tests and under the non-default `fuzz` feature,
//! never into a shipped build: `docs/verification-hooks.md` is the contract
//! the fuzzer builds against.

use super::Engine;
use crate::state::{CardLookup, GameState};

impl<L: CardLookup> Engine<L> {
    /// Whether the cached layered projection is what a refresh would make of
    /// the current state.
    ///
    /// The projection is cached behind one `u64` generation compare
    /// (`GameState::refresh_characteristics`), so a change that moves a
    /// characteristic's input without invalidating leaves every reader
    /// looking at the old value while the generation says all is well. This
    /// asks the content rather than the number: a copy of the state is
    /// invalidated and refreshed, and every object's characteristics and
    /// controller are compared, field for field, with what
    /// `GameObject::characteristics` and `GameObject::controller` answer
    /// now.
    ///
    /// Read-only and deterministic; the engine's own state is not written,
    /// not even a cache. It costs a clone of the state and a full refresh.
    #[must_use]
    pub fn projection_is_fresh(&self) -> bool {
        projection_is_fresh(&self.state)
    }
}

/// [`Engine::projection_is_fresh`] on a bare state.
///
/// The refresh is the engine's own, run on a copy, rather than a second
/// statement here of which objects it projects and how: that set has grown
/// (the stack's spells, every object under a cross-zone effect, the cached
/// leftovers of one, the cards defining their own power and toughness in
/// every zone), and a copy of it would drift from it in silence, reporting
/// either a stale object the refresh never looks at or nothing about one it
/// does.
pub(crate) fn projection_is_fresh(state: &GameState) -> bool {
    let mut settled = state.clone();
    settled.invalidate_projections();
    settled.refresh_characteristics();
    state.arena.iter().all(|(id, obj)| {
        settled.object(id).is_some_and(|fresh| {
            fresh.characteristics() == obj.characteristics() && fresh.controller == obj.controller
        })
    })
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
        assert!(
            !engine.projection_is_fresh(),
            "a refresh that is due and has not run is no fresher: the cache still answers the \
             old body, and the generation says so"
        );
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
