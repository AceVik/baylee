//! "When you do, …": reflexive triggered abilities (CR 603.12).
//!
//! A reflexive ability is created by the resolution that caused its event,
//! and CR 603.12 has it check that event against what the resolution has
//! *already* done, never against what happens afterwards. That is the one
//! thing a `Trigger` cannot say. A trigger listens for an event whenever it
//! happens, so Brokers Hideout's "when you do", once written as a
//! leaves-the-battlefield trigger, fired on a bounce in response. The
//! Manticore example in the rule is exactly that case: its reflexive ability
//! "triggers only when you sacrifice another creature due to the original
//! triggered ability, and not if you sacrifice a creature for any other
//! reason".
//!
//! What this resolution has done is read off the journal, not off a field on
//! [`Resolution`]. `resolve_stack_top` records `StackObjectResolved` after
//! its CR 603.4 and 608.2b checks and before any effect runs, and
//! resolutions never interleave. So the entries after the latest marker are
//! this resolution's own. A question asked inside the action, such as Eden's
//! "you may sacrifice", splices its tail and resumes later. The journal is
//! unchanged by that, where a field would have to be carried through every
//! `Resolution` literal in the crate.

use baylee_cards_dsl::{Effect, ReflexiveEvent, TargetSpec};
use baylee_core::ids::AbilityRef;

use super::Resolution;
use crate::event::{Cause, GameEvent};
use crate::state::GameState;
use crate::trigger::PendingTrigger;
use crate::zone::Zone;

/// Creates the reflexive ability once for every time its event happened
/// earlier in this resolution (CR 603.12a), and not at all when it did not.
///
/// What it creates waits in [`GameState::reflexive`] until
/// `Engine::finish_resolution` hands it to the trigger queue. CR 603.3 puts it
/// on the stack the next time a player would receive priority, which is after
/// this resolution has finished.
pub(super) fn arm(
    state: &mut GameState,
    res: &Resolution,
    when: ReflexiveEvent,
    effects: &'static [Effect],
    target: Option<TargetSpec>,
) {
    let times = occurrences(state, res, when);
    if times == 0 {
        return;
    }
    // CR 603.7e: the source is the source of the ability that created it,
    // and the controller is whoever controlled that ability as it resolved.
    let timestamp = state.object(res.source).map_or(0, |o| o.timestamp);
    for _ in 0..times {
        state.reflexive.push(PendingTrigger {
            source: res.source,
            ability_index: AbilityRef::SYNTHETIC,
            abilities: None,
            controller: res.controller,
            timestamp,
            // Never the event's object. On the untargeted synthetic path the
            // stacker turns `event_object` into an implicit target, which is
            // how prowess pumps itself, and a reflexive body names its
            // target through `synthetic_target` or not at all.
            event_object: None,
            synthetic_effects: Some(effects),
            synthetic_target: target,
            once_per_turn: false,
            chosen_mode: None,
        });
    }
}

/// How many times `when` happened since this resolution began.
fn occurrences(state: &GameState, res: &Resolution, when: ReflexiveEvent) -> usize {
    // Only a resolution off the stack has a beginning to read back to. A
    // mana ability never goes on the stack (CR 605.3b), and its `on_stack`
    // is the source permanent. That permanent keeps the `ObjectId` of the
    // spell it resolved from, so without this check it could match that
    // spell's marker and count everything that has happened since.
    if state
        .object(res.on_stack)
        .is_none_or(|o| o.zone != Zone::Stack)
    {
        return 0;
    }
    let entries = state.journal.entries();
    let Some(start) = entries
        .iter()
        .rposition(|e| matches!(e.event, GameEvent::StackObjectResolved { .. }))
    else {
        return 0;
    };
    if !matches!(entries[start].event, GameEvent::StackObjectResolved { object } if object == res.on_stack)
    {
        return 0;
    }
    entries[start + 1..]
        .iter()
        .filter(|e| happened(when, res, &e.event))
        .count()
}

/// Whether one journal entry is the event a reflexive ability waits for.
fn happened(when: ReflexiveEvent, res: &Resolution, event: &GameEvent) -> bool {
    match when {
        // A departure of the source by *effect*. A cost records
        // `Cause::Cost`, so a sacrifice paid in a payment window opened
        // during this resolution is not the one "you do" names. A
        // sacrifice that a replacement sends to exile still counts, because
        // the player still did it. Which effect it was is not read here.
        // `lints::every_reflexive_sits_where_it_can_trigger` makes the
        // sacrifice the op directly before the reflexive, and allows only
        // ops before it that cannot move the source.
        ReflexiveEvent::SacrificedThis => matches!(
            event,
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                cause: Cause::Effect,
                ..
            } if *object == res.source
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::object::ObjectKind;
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
    use smallvec::SmallVec;

    const BODY: &[Effect] = &[Effect::GainLife {
        amount: baylee_cards_dsl::Amount::Fixed(1),
    }];

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn state() -> GameState {
        GameState::from_preset(&preset(12, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game")
    }

    fn object(state: &mut GameState, kind: ObjectKind, loc: ZoneLocation) -> ObjectId {
        let name = state.names.intern("Probe");
        state.create_bare(me(), kind, name, loc)
    }

    fn resolution(source: ObjectId, on_stack: ObjectId) -> Resolution {
        Resolution {
            source,
            on_stack,
            controller: me(),
            effects: Vec::new(),
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
        }
    }

    fn sacrifice(state: &mut GameState, id: ObjectId, cause: Cause) {
        state
            .move_object(id, ZoneLocation::Graveyard(me()), ZonePosition::Top, cause)
            .expect("the permanent moves");
    }

    /// The event counted, and the three things around it that are not it.
    ///
    /// A departure of the source by effect, after this resolution's marker,
    /// creates one ability. A departure paid as a cost does not, because a
    /// payment window is not the action "you do" names. A departure recorded
    /// before the marker belongs to an earlier resolution. A marker for
    /// another stack object means the resolution in progress is not the one
    /// that marker opened.
    #[test]
    fn only_a_departure_by_effect_inside_this_resolution_is_counted() {
        let mut state = state();
        let land = object(&mut state, ObjectKind::Permanent, ZoneLocation::Battlefield);
        let ability = object(&mut state, ObjectKind::AbilityOnStack, ZoneLocation::Stack);
        let res = resolution(land, ability);

        state
            .journal
            .record(GameEvent::StackObjectResolved { object: ability });
        sacrifice(&mut state, land, Cause::Cost);
        arm(&mut state, &res, ReflexiveEvent::SacrificedThis, BODY, None);
        assert!(state.reflexive.is_empty(), "a cost is not the action");

        let land = object(&mut state, ObjectKind::Permanent, ZoneLocation::Battlefield);
        let res = resolution(land, ability);
        sacrifice(&mut state, land, Cause::Effect);
        state
            .journal
            .record(GameEvent::StackObjectResolved { object: ability });
        arm(&mut state, &res, ReflexiveEvent::SacrificedThis, BODY, None);
        assert!(
            state.reflexive.is_empty(),
            "before the marker is an earlier resolution"
        );

        let other = object(&mut state, ObjectKind::AbilityOnStack, ZoneLocation::Stack);
        let land = object(&mut state, ObjectKind::Permanent, ZoneLocation::Battlefield);
        let res = resolution(land, ability);
        state
            .journal
            .record(GameEvent::StackObjectResolved { object: other });
        sacrifice(&mut state, land, Cause::Effect);
        arm(&mut state, &res, ReflexiveEvent::SacrificedThis, BODY, None);
        assert!(
            state.reflexive.is_empty(),
            "the latest marker opened another resolution"
        );

        let land = object(&mut state, ObjectKind::Permanent, ZoneLocation::Battlefield);
        let res = resolution(land, ability);
        state
            .journal
            .record(GameEvent::StackObjectResolved { object: ability });
        sacrifice(&mut state, land, Cause::Effect);
        arm(&mut state, &res, ReflexiveEvent::SacrificedThis, BODY, None);
        assert_eq!(
            state.reflexive.len(),
            1,
            "the sacrifice itself creates one ability"
        );
        let t = &state.reflexive[0];
        assert_eq!(t.source, land, "CR 603.7e: the creating ability's source");
        assert_eq!(t.controller, me());
        assert_eq!(t.ability_index, AbilityRef::SYNTHETIC);
        assert_eq!(t.event_object, None, "never an implicit target");
        assert_eq!(t.synthetic_effects, Some(BODY));
    }

    /// A mana ability never goes on the stack (CR 605.3b), so it has no
    /// resolution to read back through, and its `on_stack` is the permanent.
    ///
    /// That permanent keeps the `ObjectId` of the spell it resolved from.
    /// When that spell's marker is still the latest one, the marker and
    /// `on_stack` agree, and without the stack check everything since the
    /// permanent arrived would count as this ability's doing.
    #[test]
    fn a_resolution_off_the_stack_counts_nothing() {
        let mut state = state();
        let creature = object(&mut state, ObjectKind::Permanent, ZoneLocation::Battlefield);
        state
            .journal
            .record(GameEvent::StackObjectResolved { object: creature });
        let mut res = resolution(creature, creature);
        res.mana_ability = true;
        sacrifice(&mut state, creature, Cause::Effect);
        arm(&mut state, &res, ReflexiveEvent::SacrificedThis, BODY, None);
        assert!(state.reflexive.is_empty());
    }
}
