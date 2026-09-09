//! Replacement rules that multiply what an effect does (CR 614): Doubling
//! Season, Elspeth Storm-Slayer and the cards written like them.
//!
//! They live in one module because they are read from six places across
//! three others — two token-creating effects, three counter-placing ones
//! and a planeswalker's starting loyalty — and the bug that collected them
//! here was one of those places reading its own rule
//! differently from the rest: a filter over the *affected controller* was
//! being asked of the resolving effect's own controller, which is a
//! question that answers yes for everybody, so my Doubling Season doubled
//! an opponent's tokens.
//!
//! [`put_counters`] is a door for the same reason [`crate::sba::destroy`]
//! is. A counter added beside it is a counter Doubling Season does not
//! see, and nothing says so until someone plays the pair: that is exactly
//! how "put a +1/+1 counter on each other Ally you control" — the shape
//! most of the pool writes — sat outside the rule while the single-target
//! shape beside it was inside.

use crate::eval;
use crate::event::GameEvent;
use crate::state::GameState;
use baylee_core::ids::{ObjectId, PlayerId};

/// How many times over an effect creating tokens under `recipient`'s
/// control actually creates them (CR 614.1).
///
/// `controller_filter` is a filter over the **affected controller**, not
/// over the effect doing the creating: "if one or more tokens would be
/// created under *your* control" is a statement about who ends up with them
/// and says nothing about whose spell put them there. So it is read against
/// the replacement's own source with the recipient as "you", which makes
/// `ControlledByYou` mean "the enchantment that recipient controls".
#[must_use]
pub fn token_multiplier(state: &GameState, recipient: PlayerId) -> u32 {
    let mut count = 1u32;
    for entry in &state.replacement_rules {
        if let baylee_cards_dsl::ReplacementRule::DoubleTokenCreation { controller_filter } =
            entry.rule
            && let Some(source_obj) = state.object(entry.source)
            && eval::matches(
                controller_filter,
                state,
                source_obj,
                recipient,
                entry.source,
            )
        {
            count *= 2;
        }
    }
    count
}

/// How many times over counters put on `target` are actually put on it
/// (CR 614.2).
///
/// The mirror of [`token_multiplier`] and read the other way round, because
/// this rule's filter is over the **object receiving them**: "a permanent
/// you control" is about the permanent, whoever's effect is placing them.
/// So the filter is matched against `target` with the *replacement's* own
/// controller as "you" — an opponent's spell putting a counter on my
/// creature is doubled by my Doubling Season, and mine putting one on
/// theirs is not.
#[must_use]
pub fn counter_multiplier(state: &GameState, target: ObjectId) -> u16 {
    let Some(target_obj) = state.object(target) else {
        return 1;
    };
    let mut count = 1u16;
    for entry in &state.replacement_rules {
        if let baylee_cards_dsl::ReplacementRule::DoubleCounterPlacement { object_filter } =
            entry.rule
            && eval::matches(
                object_filter,
                state,
                target_obj,
                entry.controller,
                entry.source,
            )
        {
            count = count.saturating_mul(2);
        }
    }
    count
}

/// Puts `n` counters of `kind` on `id`, after the replacements that
/// multiply them, and records the change.
///
/// Every effect that puts counters on a permanent goes through here. The
/// journal entry and the projection invalidation are part of the door and
/// not of the caller: a counter is a characteristic input (CR 613.4c), so
/// one placed without invalidating leaves the creature drawn at its old
/// size until something else asks for a pass.
pub fn put_counters(
    state: &mut GameState,
    id: ObjectId,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
) {
    let n = n.saturating_mul(counter_multiplier(state, id));
    if let Some(obj) = state.object_mut(id) {
        let old = obj.counters.get(kind);
        let new = obj.counters.add(kind, n);
        state.journal.record(GameEvent::CounterChanged {
            object: id,
            kind,
            old,
            new,
        });
    }
    state.invalidate_projections();
}
