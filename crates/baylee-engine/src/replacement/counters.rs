//! Counter placement, removal and replacement multipliers.

use super::{counter_effects, counter_limits, eval};
use crate::{event::GameEvent, state::GameState};
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};

/// How many times over counters put on `target` are actually put on it
/// (CR 614.16).
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
            #[cfg(test)]
            crate::ability_log::replaced(entry);
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
    let Some(by) = state.object(id).map(|o| o.controller) else {
        return;
    };
    put_counters_by(state, by, id, kind, n);
}

/// Resolving effects name the player placing the counters, independently of
/// the recipient's controller (CR 122.6a, Hapatra).
pub fn put_counters_by(
    state: &mut GameState,
    by: PlayerId,
    id: ObjectId,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
) {
    let n = n.saturating_mul(counter_multiplier(state, id));
    record_counters_by(state, by, id, kind, n);
}

/// The same door with the multiplying replacements left out: `n` counters
/// land, recorded and with the projections invalidated, and nothing gets to
/// say otherwise.
///
/// There is one caller, and CR 614.16 is why it is separate. A
/// counter-doubling replacement applies to what "the effect of a resolving
/// spell or ability" places and to what another replacement effect places —
/// and a **turn-based action** is neither of those. The lore counter a Saga
/// takes as its controller's precombat main phase begins is exactly that
/// (CR 505.4, CR 714.3b, both of which say it does not use the stack), so it
/// comes through here. The counter it takes as it *enters* is a replacement
/// effect (CR 614.1c) and goes through [`put_counters`] like everything else.
pub fn record_counters(
    state: &mut GameState,
    id: ObjectId,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
) {
    let Some(by) = state.object(id).map(|o| o.controller) else {
        return;
    };
    record_counters_by(state, by, id, kind, n);
}

/// Records an actual placement with its placing player; no effect-only multiplier.
pub fn record_counters_by(
    state: &mut GameState,
    by: PlayerId,
    id: ObjectId,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
) {
    let n = counter_limits::room(state, id, kind).map_or(n, |room| n.min(room));
    let Some(obj) = state.object_mut(id) else {
        return;
    };
    let (old, version) = (obj.counters.get(kind), obj.version);
    let new = obj.counters.add(kind, n);
    let placed = new > old;
    let first_this_turn = placed
        && state.per_turn.counter_placements.insert((
            DamageSourceRef {
                object: id,
                version,
            },
            kind,
        ));
    state.journal.record(GameEvent::CounterChanged {
        object: id,
        version,
        by: placed.then_some(by),
        first_this_turn,
        kind,
        old,
        new,
    });
    if placed {
        counter_effects::placed(state, id, kind);
    }
    state.invalidate_projections();
}

/// Takes `n` counters of `kind` off `id`, records the change, and answers how
/// many actually came off.
///
/// The sibling of [`record_counters`] and in this module for the reason that
/// one is: the journal entry and the projection invalidation are part of the
/// door. A +1/+1 counter spent on Walking Ballista's ping makes the creature
/// smaller, and a removal that skipped the invalidation would leave it drawn
/// at its old size and, worse, leave the state-based action that kills it at
/// nought toughness reading a cached projection.
///
/// It takes no multiplier, and that is the asymmetry rather than an omission:
/// CR 614.16 and the counter-doubling replacements are about counters being
/// **put** on a permanent, and Magic prints nothing that multiplies a
/// removal. A cost of one counter is a cost of one counter under any number
/// of Doubling Seasons.
///
/// It also saturates rather than refusing. Every caller has already checked
/// that enough are there — `can_afford` for a cost, the rules for everything
/// else — and a door that could half-fail would make the two answers
/// disagree at the one moment it matters.
pub fn remove_counters(
    state: &mut GameState,
    id: ObjectId,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
) -> u16 {
    let mut taken = 0;
    if let Some(obj) = state.object_mut(id) {
        let version = obj.version;
        let old = obj.counters.get(kind);
        taken = old.min(n);
        let new = old - taken;
        obj.counters.set(kind, new);
        state.journal.record(GameEvent::CounterChanged {
            object: id,
            version,
            by: None,
            first_this_turn: false,
            kind,
            old,
            new,
        });
    }
    state.invalidate_projections();
    crate::resolve::linked_counters::expire(state);
    if kind == baylee_cards_dsl::CounterKind::Hone
        && state
            .object(id)
            .is_none_or(|object| object.counters.get(kind) == 0)
    {
        state.effects.remove_where(|effect| {
            effect.origin == crate::effects::EffectOrigin::Counter(kind)
                && effect.source == Some(id)
        });
    }
    taken
}

/// CR 122.5: both halves must be possible, for the exact battlefield objects.
pub(crate) fn can_move_counter(
    state: &GameState,
    from: DamageSourceRef,
    to: DamageSourceRef,
    kind: baylee_cards_dsl::CounterKind,
) -> bool {
    if from.object == to.object || counter_limits::room(state, to.object, kind) == Some(0) {
        return false;
    }
    let current = |reference: DamageSourceRef| {
        state.object(reference.object).filter(|object| {
            object.version == reference.version
                && object.zone == crate::zone::Zone::Battlefield
                && !object.status.contains(crate::object::Status::PHASED_OUT)
        })
    };
    current(from).is_some_and(|object| object.counters.get(kind) > 0) && current(to).is_some()
}

/// Moving removes one counter; destination placement still applies replacements.
pub(crate) fn move_counter(
    state: &mut GameState,
    by: PlayerId,
    from: DamageSourceRef,
    to: DamageSourceRef,
    kind: baylee_cards_dsl::CounterKind,
) {
    if can_move_counter(state, from, to, kind) {
        remove_counters(state, from.object, kind, 1);
        put_counters_by(state, by, to.object, kind, 1);
    }
}
