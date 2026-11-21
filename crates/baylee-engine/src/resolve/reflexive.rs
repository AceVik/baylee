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

use baylee_cards_dsl::{
    Condition, Effect, ReflexiveEvent, ReflexiveWhen, TargetReq, TargetSpec, ThisWay,
    TriggerRemember,
};
use baylee_core::ids::{AbilityRef, DamageSourceRef};
use smallvec::{SmallVec, smallvec};

use super::Resolution;
use crate::event::{Cause, GameEvent};
use crate::state::{GameState, RememberedContext};
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
            text: res.text,
            source_version: None,
            related_objects: None,
            grantor: None,
            event_subject: None,
            event_ability: None,
            event_object_identity: None,
            counter_source_version: None,
            event_mana: None,
            event_mana_value: None,
            event_tap_snapshot: None,
            event_departure: None,
            event_amount: None,
            event_player: None,
            source: res.source,
            ability_index: AbilityRef::SYNTHETIC,
            abilities: None,
            controller: res.controller,
            timestamp,
            // Never the event's object, and no implicit target. On the
            // untargeted synthetic path the stacker puts `implicit_target`
            // first among the targets, which is how prowess pumps itself,
            // and a reflexive body names its target through
            // `synthetic_target` or not at all.
            event_object: None,
            implicit_target: None,
            synthetic_effects: Some(effects),
            synthetic_target: target.map(|spec| match spec {
                TargetSpec::ObjectOfEachOpponent(_) => {
                    baylee_cards_dsl::TargetReq::up_to(spec, u8::MAX)
                }
                _ => baylee_cards_dsl::TargetReq::one(spec),
            }),
            once_per_turn: false,
            chosen_mode: None,
            synthetic_condition: None,
            in_chosen_order: false,
        });
    }
}

/// Generalized on-resolution reflexive trigger (`Effect::ReflexiveTrigger`,
/// CR 603.12, CR 603.12a, CR 603.4).
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // Direct unpack of Effect::ReflexiveTrigger fields across its four trigger-gating modes.
pub(super) fn arm_general(
    state: &mut GameState,
    res: &Resolution,
    when: ReflexiveWhen,
    once: bool,
    condition: Option<&'static Condition>,
    targets: Option<TargetReq>,
    remember: TriggerRemember,
    effects: &'static [Effect],
) {
    if state
        .object(res.on_stack)
        .is_none_or(|o| o.zone != Zone::Stack)
    {
        return;
    }

    let mut matched: SmallVec<[Option<RememberedContext>; 2]> = SmallVec::new();
    match when {
        ReflexiveWhen::Always => {
            matched.push(None);
        }
        ReflexiveWhen::Event(ev) => {
            for entry in super::this_way::entries(state, res) {
                if happened(state, ev, res, entry) {
                    let ctx = match entry.event {
                        GameEvent::ZoneChanged { object, .. } => match entry.departure.as_deref() {
                            Some(dep) => Some(super::delayed_general::context_from_departure(
                                state, object, dep,
                            )),
                            None => Some(super::delayed_general::context_from_object(
                                state, object, None,
                            )),
                        },
                        _ => None,
                    };
                    matched.push(ctx);
                }
            }
        }
        ReflexiveWhen::ThisWay(tw) => {
            let total = super::this_way::count(state, res, tw, None);
            if total == 0 {
                return;
            }
            if once {
                let mut first = first_this_way_object(state, res, tw);
                if let Some(ref mut ctx) = first {
                    ctx.amount = Some(total);
                } else {
                    first = Some(RememberedContext {
                        amount: Some(total),
                        ..RememberedContext::default()
                    });
                }
                matched.push(first);
            } else {
                let objects = all_this_way_objects(state, res, tw);
                if objects.is_empty() {
                    for _ in 0..total {
                        matched.push(Some(RememberedContext {
                            amount: Some(1),
                            ..RememberedContext::default()
                        }));
                    }
                } else {
                    for ctx in objects {
                        matched.push(Some(ctx));
                    }
                }
            }
        }
        ReflexiveWhen::Tapped(filter) => {
            let rule_ctx = res.rule_context();
            for entry in super::this_way::entries(state, res) {
                if let GameEvent::ObjectTapped {
                    object,
                    cause: Cause::Effect,
                } = entry.event
                    && state.object_or_departed(object).is_some_and(|card| {
                        crate::eval::matches_with_context(
                            filter,
                            state,
                            card,
                            res.controller,
                            rule_ctx,
                        )
                    })
                {
                    matched.push(Some(super::delayed_general::context_from_object(
                        state, object, None,
                    )));
                }
            }
        }
    }

    if matched.is_empty() {
        return;
    }
    if once && matched.len() > 1 {
        matched.truncate(1);
    }

    let final_contexts: SmallVec<[RememberedContext; 2]> = if remember == TriggerRemember::None {
        matched.into_iter().map(Option::unwrap_or_default).collect()
    } else {
        let explicit = super::delayed_general::resolve_remembered(state, res, remember);
        if explicit.is_empty()
            && matches!(
                remember,
                TriggerRemember::EachTarget
                    | TriggerRemember::EachCreatedToken
                    | TriggerRemember::EachExiledThisWay
                    | TriggerRemember::SacrificedObject
            )
        {
            return;
        }
        let rem_items: SmallVec<[RememberedContext; 2]> = if explicit.is_empty() {
            smallvec![RememberedContext::default()]
        } else {
            explicit
        };
        if once {
            rem_items.into_iter().take(1).collect()
        } else {
            let mut combined = SmallVec::new();
            for _ in 0..matched.len() {
                combined.extend(rem_items.iter().copied());
            }
            combined
        }
    };

    let source_version = super::subjects::source(state, res).map(|s| s.version);
    let attachment_lki = super::source_attachment_lki(state, res.on_stack);
    let timestamp = state.object(res.source).map_or(0, |o| o.timestamp);

    for rem in final_contexts {
        let event_ref = rem
            .object
            .zip(rem.identity.map(|(v, _)| v))
            .map(|(object, version)| DamageSourceRef { object, version });
        if !crate::eval::intervening_if_for_incarnation(
            state,
            condition.copied(),
            res.controller,
            res.source,
            source_version,
            attachment_lki,
            event_ref,
        ) {
            continue;
        }
        state.reflexive.push(PendingTrigger {
            text: res.text,
            source_version,
            related_objects: None,
            grantor: None,
            event_subject: rem.subject,
            event_ability: None,
            event_object_identity: rem.identity,
            counter_source_version: None,
            event_mana: None,
            event_mana_value: rem.mana_value,
            event_tap_snapshot: None,
            event_departure: rem.departure,
            event_amount: rem.amount,
            event_player: rem.player,
            source: res.source,
            ability_index: AbilityRef::SYNTHETIC,
            abilities: None,
            controller: res.controller,
            timestamp,
            event_object: rem.object,
            implicit_target: None,
            synthetic_effects: Some(effects),
            synthetic_target: targets,
            synthetic_condition: condition,
            once_per_turn: false,
            chosen_mode: None,
            in_chosen_order: false,
        });
    }
}

fn first_this_way_object(
    state: &GameState,
    res: &Resolution,
    tw: ThisWay,
) -> Option<RememberedContext> {
    all_this_way_objects(state, res, tw).into_iter().next()
}

fn all_this_way_objects(
    state: &GameState,
    res: &Resolution,
    tw: ThisWay,
) -> SmallVec<[RememberedContext; 2]> {
    let mut out = SmallVec::new();
    let rule_ctx = res.rule_context();
    for entry in super::this_way::entries(state, res) {
        if let GameEvent::Revealed { cards, .. } = &entry.event
            && let ThisWay::Revealed(filter) = tw
        {
            for &id in cards {
                if state.object(id).is_some_and(|c| {
                    crate::eval::matches_with_context(filter, state, c, res.controller, rule_ctx)
                }) {
                    out.push(super::delayed_general::context_from_object(state, id, None));
                }
            }
            continue;
        }
        if let Some(ctx) = this_way_entry_context(state, res, rule_ctx, entry, tw) {
            out.push(ctx);
        }
    }
    out
}

#[allow(clippy::too_many_lines)] // Keep each journal event and its captured context in one flat match.
fn this_way_entry_context(
    state: &GameState,
    res: &Resolution,
    rule_ctx: crate::text_changes::RuleContext,
    entry: &crate::event::JournalEntry,
    tw: ThisWay,
) -> Option<RememberedContext> {
    match (&entry.event, tw) {
        (
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                to: Zone::Graveyard,
                ..
            },
            ThisWay::Died(filter),
        )
        | (
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                to: Zone::Exile,
                ..
            },
            ThisWay::ExiledFromBattlefield(filter),
        )
        | (
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                to: Zone::Hand,
                ..
            },
            ThisWay::ReturnedToHand(filter),
        ) => departed_battlefield_context(state, res, rule_ctx, entry, *object, filter),
        (
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                cause: Cause::Destroy,
                ..
            },
            ThisWay::Destroyed(filter),
        ) => entry.departure.as_deref().and_then(|dep| {
            let card = state.damage_source(*object, Some(dep.version))?;
            crate::eval::matches_projected_with_context(
                filter,
                state,
                card,
                card.characteristics(),
                res.controller,
                rule_ctx,
            )
            .then(|| super::delayed_general::context_from_departure(state, *object, dep))
        }),
        (GameEvent::Discarded { object, .. }, ThisWay::Discarded)
        | (
            GameEvent::ZoneChanged {
                object,
                from: Zone::Hand,
                to: Zone::Library,
                ..
            },
            ThisWay::PutIntoLibraryFromHand,
        ) => Some(super::delayed_general::context_from_object(
            state, *object, None,
        )),
        (GameEvent::Discarded { object, .. }, ThisWay::DiscardedMatching(filter))
        | (
            GameEvent::ZoneChanged {
                object,
                from: Zone::Library,
                to: Zone::Exile,
                ..
            },
            ThisWay::ExiledFromLibrary(filter),
        )
        | (
            GameEvent::ZoneChanged {
                object,
                from: Zone::Graveyard,
                to: Zone::Exile,
                ..
            },
            ThisWay::ExiledFromGraveyard(filter),
        )
        | (
            GameEvent::ZoneChanged {
                object,
                from: Zone::Library,
                to: Zone::Graveyard,
                ..
            },
            ThisWay::Milled(filter),
        ) => state.object(*object).and_then(|card| {
            crate::eval::matches_with_context(filter, state, card, res.controller, rule_ctx)
                .then(|| super::delayed_general::context_from_object(state, *object, None))
        }),
        (
            GameEvent::LifeChanged {
                player, old, new, ..
            },
            ThisWay::LifeLost,
        ) if new < old => Some(RememberedContext {
            player: Some(*player),
            amount: Some(old.abs_diff(*new)),
            ..RememberedContext::default()
        }),
        _ => None,
    }
}

fn departed_battlefield_context(
    state: &GameState,
    res: &Resolution,
    rule_ctx: crate::text_changes::RuleContext,
    entry: &crate::event::JournalEntry,
    object: baylee_core::ids::ObjectId,
    filter: &baylee_cards_dsl::Filter,
) -> Option<RememberedContext> {
    let card = state.object(object)?;
    let chars = state
        .last_known_characteristics(object)
        .unwrap_or_else(|| card.characteristics());
    crate::eval::matches_projected_with_context(
        filter,
        state,
        card,
        chars,
        res.controller,
        rule_ctx,
    )
    .then(|| match entry.departure.as_deref() {
        Some(dep) => super::delayed_general::context_from_departure(state, object, dep),
        None => super::delayed_general::context_from_object(state, object, None),
    })
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
        .filter(|e| happened(state, when, res, e))
        .count()
}

/// Whether one journal entry is the event a reflexive ability waits for.
fn happened(
    state: &GameState,
    when: ReflexiveEvent,
    res: &Resolution,
    entry: &crate::event::JournalEntry,
) -> bool {
    let event = &entry.event;
    match when {
        // A sacrifice by effect of a permanent the filter matched as it
        // left (CR 608.2h): the departure says it was a sacrifice, and its
        // controller then was this resolution's.
        ReflexiveEvent::Sacrificed(filter) => {
            let GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                cause: Cause::Effect,
                ..
            } = *event
            else {
                return false;
            };
            entry
                .departure
                .as_ref()
                .is_some_and(|d| d.sacrificed && d.controller == res.controller)
                && state.object_or_departed(object).is_some_and(|card| {
                    let chars = state
                        .last_known_characteristics(object)
                        .unwrap_or_else(|| card.characteristics());
                    crate::eval::matches_projected_with_context(
                        filter,
                        state,
                        card,
                        chars,
                        res.controller,
                        res.rule_context(),
                    )
                })
        }
        ReflexiveEvent::SacrificedGrantor => {
            let Some(grantor) = super::granted_source(state, res.on_stack) else {
                return false;
            };
            let GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                cause: Cause::Effect,
                ..
            } = *event
            else {
                return false;
            };
            object == grantor.object
                && entry.departure.as_ref().is_some_and(|departure| {
                    // The ZoneChanged entry stores the incarnation immediately
                    // before departure. The object itself is already its next
                    // incarnation in the graveyard (CR 400.7), so reading its
                    // current version would reject every actual sacrifice.
                    departure.version == grantor.version
                        && departure.sacrificed
                        && departure.controller == res.controller
                })
        }
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
        // The source exiled by effect, from wherever it was: a dies
        // trigger's source is the card in the graveyard, and
        // `Effect::ExileSource` moves it from there.
        // `lints::every_reflexive_sits_where_it_can_trigger` makes that
        // exile the op directly before the reflexive.
        ReflexiveEvent::Exiled(filter) => matches!(event,
            GameEvent::ZoneChanged { object, to: Zone::Exile, cause: Cause::Effect, .. }
                if res.targets.contains(object) && state.object(*object).is_some_and(|card| {
                    crate::eval::matches(filter, state, card, res.controller, res.source)
                })
        ),
        ReflexiveEvent::ExiledThis => matches!(
            event,
            GameEvent::ZoneChanged {
                object,
                to: Zone::Exile,
                cause: Cause::Effect,
                ..
            } if *object == res.source
        ),
        ReflexiveEvent::ExertedThis => matches!(
            event,
            GameEvent::PermanentExerted {
                object,
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
            subject: crate::resolve::SubjectContext::default(),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
            loot: None,
            draw_watch: None,
            this_way_from: None,
            coin_stake: None,
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
        assert_eq!(t.implicit_target, None, "never an implicit target");
        assert_eq!(t.event_object, None, "and no event object to name");
        assert_eq!(t.synthetic_effects, Some(BODY));
    }

    /// "You may exile it. When you do, …" (The Balrog of Moria): the
    /// source exiled by effect out of a graveyard counts, and the three
    /// moves beside it do not — the source going to a graveyard instead,
    /// another card exiled, and the source exiled as a cost.
    #[test]
    fn only_the_source_exiled_by_effect_is_exiled_this() {
        let exile = |state: &mut GameState, id: ObjectId, cause: Cause| {
            state
                .move_object(id, ZoneLocation::Exile(me()), ZonePosition::Top, cause)
                .expect("the card moves");
        };
        let mut state = state();
        let ability = object(&mut state, ObjectKind::AbilityOnStack, ZoneLocation::Stack);
        let run = |state: &mut GameState, act: &dyn Fn(&mut GameState, ObjectId)| {
            let card = object(state, ObjectKind::Card, ZoneLocation::Graveyard(me()));
            let res = resolution(card, ability);
            state
                .journal
                .record(GameEvent::StackObjectResolved { object: ability });
            act(state, card);
            let before = state.reflexive.len();
            arm(state, &res, ReflexiveEvent::ExiledThis, BODY, None);
            state.reflexive.len() - before
        };
        assert_eq!(run(&mut state, &|s, card| exile(s, card, Cause::Effect)), 1);
        assert_eq!(
            run(&mut state, &|s, card| {
                s.move_object(
                    card,
                    ZoneLocation::Hand(me()),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .expect("the card moves");
            }),
            0,
            "not an exile"
        );
        assert_eq!(
            run(&mut state, &|s, _| {
                let other = object(s, ObjectKind::Card, ZoneLocation::Graveyard(me()));
                exile(s, other, Cause::Effect);
            }),
            0,
            "another card"
        );
        assert_eq!(
            run(&mut state, &|s, card| exile(s, card, Cause::Cost)),
            0,
            "a cost is not the action"
        );
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
