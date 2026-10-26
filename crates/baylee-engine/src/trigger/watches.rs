//! Triggers no object's scan finds: state triggers, storm and replicate, the delayed watches and the monarch's.

use super::{
    EventMana, PendingTrigger, ability_context, condition_matches, event_groups, freeze_trigger,
    triggered_parts,
};
use crate::eval;
use crate::event::GameEvent;
use crate::state::{CardLookup, GameState};
use crate::text_changes::TextChangeMap;
use crate::zone::Zone;
use baylee_cards_dsl::{PlayerRel, Trigger, TriggerZone};
use baylee_core::ids::{ObjectId, PlayerId};

mod observers;
mod recurring;

/// Storm and replicate retain the announced count in one instruction.
static CAST_COPIES: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::CopyThisSpell {
    amount: baylee_cards_dsl::Amount::EventAmount,
}];

/// The state triggers that trigger now (CR 603.8): every
/// [`Trigger::State`] on the battlefield whose condition holds, asked with
/// its source as `this` and the source's controller as "you".
///
/// `in_flight` answers whether the ability of `(source, index)` is already
/// waiting to go on the stack or is on it; one that is does not trigger
/// again until it has left the stack, and then only if its source is still
/// on the battlefield and the condition still holds, which asking again is.
#[must_use]
pub fn state_triggers(
    state: &GameState,
    lookup: &impl CardLookup,
    in_flight: impl Fn(ObjectId, u32) -> bool,
) -> Vec<PendingTrigger> {
    let mut triggers = Vec::new();
    for permanent in state.battlefield_seen() {
        let Some(obj) = state.object(permanent) else {
            continue;
        };
        let abilities = obj.abilities(lookup);
        if !abilities.iter().any(|ability| {
            triggered_parts(ability).is_some_and(|firing| {
                firing.zone == TriggerZone::Battlefield
                    && matches!(firing.trigger, Trigger::State(_))
            })
        }) {
            continue;
        }
        let list = obj.ability_list(lookup);
        for (index, ability) in list.abilities.iter().enumerate() {
            let Some(firing) = triggered_parts(ability) else {
                continue;
            };
            if firing.zone != TriggerZone::Battlefield {
                continue;
            }
            let Trigger::State(condition) = firing.trigger else {
                continue;
            };
            let context = ability_context(state, obj, &list, index);
            let index = index as u32;
            if in_flight(permanent, index)
                || !eval::condition_holds_with_context(state, obj.controller, context, **condition)
                || !condition_matches(state, firing.condition, obj.controller, context, None)
            {
                continue;
            }
            triggers.push(PendingTrigger {
                text: TextChangeMap::IDENTITY,
                source_version: Some(obj.version),
                related_objects: None,
                event_subject: None,
                event_ability: None,
                event_object_identity: None,
                counter_source_version: None,
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                event_amount: None,
                event_player: None,
                source: permanent,
                ability_index: index,
                abilities: Some(list.clone()),
                controller: obj.controller,
                timestamp: obj.timestamp,
                event_object: None,
                implicit_target: None,
                synthetic_effects: None,
                once_per_turn: firing.once_per_turn,
                synthetic_target: None,
                chosen_mode: None,
            });
        }
    }
    for trigger in &mut triggers {
        freeze_trigger(state, trigger);
    }
    triggers
}

/// Storm (CR 702.40) and replicate (702.56a) trigger from casting, never copying.
/// The engine drains each mutation batch before advancing the turn. Walking
/// its cast events backwards recovers each event's earlier-spell count in
/// linear time, without rescanning the turn's entire journal for every spell.
pub(super) fn cast_copy_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    if !events
        .iter()
        .any(|entry| matches!(entry.event, GameEvent::SpellCast { .. }))
    {
        return;
    }
    let mut moves = std::collections::BTreeMap::<ObjectId, u32>::new();
    let mut before = state
        .per_turn
        .spells_cast
        .iter()
        .fold(0_u32, |n, count| n.saturating_add(*count));
    let mut collected = Vec::new();
    for entry in events.iter().rev() {
        if matches!(entry.event, GameEvent::TurnStarted { .. }) {
            break;
        }
        if let GameEvent::ZoneChanged { object, .. } = entry.event {
            *moves.entry(object).or_default() += 1;
            continue;
        }
        let GameEvent::SpellCast { object, player, .. } = entry.event else {
            continue;
        };
        before = before.saturating_sub(1);
        let Some(current) = state.object(object) else {
            continue;
        };
        let reference = baylee_core::ids::DamageSourceRef {
            object,
            version: current
                .version
                .wrapping_sub(moves.get(&object).copied().unwrap_or(0)),
        };
        let spell = state.source_object(reference).unwrap_or(current);
        let storm = spell
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::STORM);
        for copies in [
            storm.then_some(before),
            (spell.replicated > 0).then_some(u32::from(spell.replicated)),
        ]
        .into_iter()
        .flatten()
        {
            collected.push(PendingTrigger {
                text: TextChangeMap::IDENTITY,
                source_version: Some(spell.version),
                related_objects: None,
                event_subject: None,
                event_ability: None,
                event_object_identity: None,
                counter_source_version: None,
                event_amount: Some(copies),
                event_player: None,
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                source: object,
                ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                abilities: None,
                controller: player,
                timestamp: spell.timestamp,
                event_object: Some(object),
                implicit_target: Some(object),
                synthetic_effects: Some(CAST_COPIES),
                once_per_turn: false,
                synthetic_target: None,
                chosen_mode: None,
            });
        }
    }
    triggers.extend(collected.into_iter().rev());
}

/// "At the beginning of the monarch's end step, that player draws a card."
static MONARCH_DRAW: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::draw(1)];

/// "Whenever a creature deals combat damage to the monarch, its controller
/// becomes the monarch." The creature is the ability's event object, which
/// the synthetic path also puts first among its objects, and that is what
/// `ControllerOfTarget` reads.
static MONARCH_TAKEOVER: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::BecomeMonarch(
    PlayerRel::ControllerOfTarget,
)];

/// The monarch's two inherent triggered abilities (CR 724.2).
///
/// No permanent has them, so no walk over permanents finds them: they are
/// read off the events directly. They have no source ([`ObjectId::NO_SOURCE`])
/// and are controlled by whoever was the monarch when they triggered, which
/// for the takeover is the player who is about to lose the title.
///
/// Delayed triggered abilities that watch an object (CR 603.7): earthbend's
/// "when that land dies or is put into exile, return it to the battlefield
/// tapped under your control" (CR 701.66a).
///
/// Each watch triggers on the first time its object leaves the battlefield
/// after the watch was created (CR 603.7a, 603.7b), and only if that was to
/// a graveyard or into exile: a land bounced to its owner's hand has left
/// and is a new object, and the watch is spent without triggering. The
/// scan that follows removes every watch whose object is gone
/// (`Engine::queue_new_triggers`), so none is read twice.
///
/// The event object is the card, which is what "return it" reads
/// (`TargetSpec::EventObject`); the source and controller are the ones the
/// watch was created with (CR 603.7d, 603.7e).
#[allow(clippy::too_many_lines)] // one flat read of each watch kind
pub(super) fn watch_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    static TAWNOSS_RETURN: &[baylee_cards_dsl::Effect] =
        &[baylee_cards_dsl::Effect::ReturnTawnossCoffinExile];
    for watch in &state.delayed {
        if let crate::state::DelayedWhen::Observed { .. } = watch.when {
            if watch.when.observation_fired(events, watch.controller)
                && let Some(trigger) = observers::trigger(state, watch)
            {
                triggers.push(trigger);
            }
            continue;
        }
        if let (
            crate::state::DelayedWhen::LeavesBattlefieldOrUntaps { card, after, .. },
            crate::state::DelayedAction::ReturnExiledGroup {
                source, objects, ..
            },
        ) = (watch.when, &watch.action)
        {
            let fired = events
                .iter()
                .filter(|entry| entry.seq > after)
                .any(|entry| {
                    matches!(
                        entry.event,
                        GameEvent::ZoneChanged {
                            object,
                            from: Zone::Battlefield,
                            ..
                        } | GameEvent::ObjectUntapped { object, .. }
                            if object == card
                    )
                });
            if fired {
                triggers.push(PendingTrigger {
                    text: TextChangeMap::IDENTITY,
                    source_version: Some(source.version),
                    event_ability: None,
                    event_object_identity: None,
                    related_objects: Some(std::sync::Arc::new(objects.clone())),
                    event_subject: None,
                    counter_source_version: None,
                    event_amount: None,
                    event_player: None,
                    event_mana: None,
                    event_mana_value: None,
                    event_departure: None,
                    source: source.object,
                    ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                    abilities: None,
                    controller: watch.controller,
                    timestamp: state.object(source.object).map_or(0, |o| o.timestamp),
                    event_object: None,
                    implicit_target: None,
                    synthetic_effects: Some(TAWNOSS_RETURN),
                    once_per_turn: false,
                    synthetic_target: None,
                    chosen_mode: None,
                });
            }
            continue;
        }
        // "When that land dies or is put into exile" watches two
        // destinations; "when this Aura leaves the battlefield" every one.
        let (card, after, to_graveyard_only, any_zone) = match watch.when {
            crate::state::DelayedWhen::DiesOrIsExiled { card, after, .. } => {
                (card, after, false, false)
            }
            crate::state::DelayedWhen::DiesThisTurn { card, after, .. } => {
                (card, after, true, false)
            }
            crate::state::DelayedWhen::LeavesBattlefield { card, after, .. }
            | crate::state::DelayedWhen::LeavesBattlefieldThisTurn { card, after, .. } => {
                (card, after, false, true)
            }
            _ => continue,
        };
        // A watch about another object ("that creature's controller
        // sacrifices it") carries that object as its event object, as the
        // incarnation it was (CR 603.7c, 400.7).
        let (source, source_version, effects, text, about) = match watch.action {
            crate::state::DelayedAction::Trigger {
                source,
                source_version,
                effects,
                text,
            } => (source, source_version, effects, text, None),
            crate::state::DelayedAction::TriggerAbout {
                source,
                source_version,
                effects,
                text,
                object,
                version,
                ..
            } => (
                source,
                source_version,
                effects,
                text,
                Some((object, version)),
            ),
            _ => continue,
        };
        let left = events
            .iter()
            .filter(|entry| entry.seq > after)
            .find_map(|entry| match entry.event {
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Battlefield,
                    to,
                    ..
                } if object == card => Some(to),
                _ => None,
            });
        let fires = if any_zone {
            left.is_some()
        } else if to_graveyard_only {
            matches!(left, Some(Zone::Graveyard))
        } else {
            matches!(left, Some(Zone::Graveyard | Zone::Exile))
        };
        if !fires {
            continue;
        }
        let (event_object, event_object_identity) = match about {
            Some((object, version)) => (
                Some(object),
                Some((
                    version,
                    state
                        .object(object)
                        .and_then(|o| o.characteristics().power)
                        .unwrap_or(0),
                )),
            ),
            // The card as it is right after it left: the incarnation the
            // trigger is about (CR 400.7), so a return reads that one and
            // not a later one.
            None => (
                Some(card),
                state
                    .object(card)
                    .map(|o| (o.version, o.characteristics().power.unwrap_or(0))),
            ),
        };
        triggers.push(PendingTrigger {
            text,
            source_version: Some(source_version),
            event_ability: None,
            event_object_identity,
            related_objects: None,
            event_subject: None,
            counter_source_version: None,
            event_amount: None,
            event_player: None,
            event_mana: None,
            event_mana_value: None,
            event_departure: None,
            source,
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            abilities: None,
            controller: watch.controller,
            timestamp: state.object(source).map_or(0, |o| o.timestamp),
            event_object,
            implicit_target: None,
            synthetic_effects: Some(effects),
            once_per_turn: false,
            synthetic_target: None,
            chosen_mode: None,
        });
    }
}

/// "Whenever that creature is dealt damage by [a source the filter matches]
/// this turn" (Glyph of Life, `DelayedWhen::DealtDamageThisTurn`): once for
/// each such damage event after the watch began, while the watched object is
/// still the one it was. The source is asked as it deals the damage — the
/// scan follows the event before anything else happens — with the watch's
/// controller as "you". "That much" is the event's amount.
fn synthetic_watch_trigger(
    state: &GameState,
    watch: &crate::state::DelayedTrigger,
) -> Option<PendingTrigger> {
    let crate::state::DelayedAction::Trigger {
        source,
        source_version,
        effects,
        text,
    } = watch.action
    else {
        return None;
    };
    Some(PendingTrigger {
        text,
        source_version: Some(source_version),
        event_ability: None,
        event_object_identity: None,
        related_objects: None,
        event_subject: None,
        counter_source_version: None,
        event_amount: None,
        event_player: None,
        event_mana: None,
        event_mana_value: None,
        event_departure: None,
        source,
        ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
        abilities: None,
        controller: watch.controller,
        timestamp: state.object(source).map_or(0, |o| o.timestamp),
        event_object: None,
        implicit_target: None,
        synthetic_effects: Some(effects),
        once_per_turn: false,
        synthetic_target: None,
        chosen_mode: None,
    })
}

pub(super) fn damage_watch_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for watch in &state.delayed {
        let Some(base) = synthetic_watch_trigger(state, watch) else {
            continue;
        };
        match watch.when {
            crate::state::DelayedWhen::DealtDamageThisTurn {
                card,
                version,
                by,
                after,
            } => {
                if state.object(card).is_none_or(|o| o.version != version) {
                    continue;
                }
                for entry in events.iter().filter(|entry| entry.seq > after) {
                    let GameEvent::DamageDealt {
                        source: Some(damager),
                        target: crate::event::DamageTarget::Object(recipient),
                        amount,
                        ..
                    } = entry.event
                    else {
                        continue;
                    };
                    if recipient != card || amount == 0 {
                        continue;
                    }
                    let Some(dealer) = state.object_or_departed(damager) else {
                        continue;
                    };
                    if eval::matches(by, state, dealer, watch.controller, base.source) {
                        triggers.push(PendingTrigger {
                            event_amount: Some(amount),
                            event_object: Some(damager),
                            ..base.clone()
                        });
                    }
                }
            }
            crate::state::DelayedWhen::OneOrMoreDealCombatDamageToPlayerThisTurn {
                filter,
                after,
            } => {
                let mut damaged_players: smallvec::SmallVec<[PlayerId; 4]> =
                    smallvec::SmallVec::new();
                for entry in events.iter().filter(|entry| entry.seq > after) {
                    let GameEvent::DamageDealt {
                        source: Some(damager),
                        target: crate::event::DamageTarget::Player(damaged_player),
                        amount: 1..,
                        is_combat: true,
                    } = entry.event
                    else {
                        continue;
                    };
                    if !damaged_players.contains(&damaged_player)
                        && state.object_or_departed(damager).is_some_and(|dealer| {
                            eval::matches(filter, state, dealer, watch.controller, base.source)
                        })
                    {
                        damaged_players.push(damaged_player);
                    }
                }
                for damaged_player in damaged_players {
                    triggers.push(PendingTrigger {
                        event_player: Some(damaged_player),
                        ..base.clone()
                    });
                }
            }
            _ => {}
        }
    }
}

/// "Whenever you attack this turn" (Dalkovan Encampment,
/// `DelayedWhen::YouAttackThisTurn`): once for each declaration of attackers
/// after the watch began that holds at least one creature of the watch's
/// controller. One declaration is one action, and the scan reads each
/// action's events once, so a scan triggers each watch at most once; a
/// creature put onto the battlefield attacking journals no `BecameAttacker`
/// (CR 508.3a).
pub(super) fn attack_watch_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for watch in &state.delayed {
        let Some(base) = synthetic_watch_trigger(state, watch) else {
            continue;
        };
        recurring::collect(state, events, watch, &base, triggers);
        match watch.when {
            crate::state::DelayedWhen::YouAttackThisTurn { after } => {
                let attacked = events
                    .iter()
                    .filter(|entry| entry.seq > after)
                    .any(|entry| {
                        matches!(entry.event, GameEvent::BecameAttacker { object, .. }
                        if state.object(object).is_some_and(|o| o.controller == watch.controller))
                    });
                if attacked {
                    triggers.push(PendingTrigger {
                        event_player: Some(watch.controller),
                        ..base
                    });
                }
            }
            crate::state::DelayedWhen::CreatureAttacksYouOrYourPlaneswalkerUntilYourNextTurn {
                after,
            } => {
                for entry in events.iter().filter(|entry| entry.seq > after) {
                    let GameEvent::BecameAttacker { object, defending } = entry.event else {
                        continue;
                    };
                    let attacks_you_or_yours = match defending {
                        baylee_core::ids::Defender::Player(p) => p == watch.controller,
                        baylee_core::ids::Defender::Planeswalker(pw) => state
                            .object(pw)
                            .is_some_and(|o| o.controller == watch.controller),
                        baylee_core::ids::Defender::Battle(_) => false,
                    };
                    if attacks_you_or_yours {
                        triggers.push(PendingTrigger {
                            event_object_identity: state.event_object_identity(object, entry.seq),
                            event_object: Some(object),
                            implicit_target: Some(object),
                            ..base.clone()
                        });
                    }
                }
            }
            crate::state::DelayedWhen::TappedForManaThisTurn { by, filter, after } => {
                let context = crate::text_changes::RuleContext {
                    source: base.source,
                    text: base.text,
                    resolved_name: None,
                    reference: None,
                };
                for entry in events.iter().filter(|entry| entry.seq > after) {
                    let GameEvent::ManaProduced {
                        player,
                        source: Some(tapped),
                        ..
                    } = entry.event
                    else {
                        continue;
                    };
                    if eval::players(by, state, watch.controller)
                        .is_some_and(|seats| seats.contains(&player))
                        && first_mana_of_a_tap(&entry.event, state.journal.entries(), tapped)
                        && state.object(tapped).is_some_and(|o| {
                            eval::matches_with_context(filter, state, o, watch.controller, context)
                        })
                    {
                        triggers.push(PendingTrigger {
                            event_player: Some(player),
                            event_object: Some(tapped),
                            event_mana: produced_mana(&entry.event, events),
                            ..base.clone()
                        });
                    }
                }
            }
            crate::state::DelayedWhen::AttacksAndIsntBlockedThisTurn {
                card,
                version,
                after,
            } => {
                if state.object(card).is_none_or(|o| o.version != version) {
                    continue;
                }
                for entry in events.iter().filter(|entry| entry.seq > after) {
                    if matches!(entry.event, GameEvent::AttackerUnblocked { object } if object == card)
                    {
                        triggers.push(PendingTrigger {
                            event_object_identity: state.event_object_identity(card, entry.seq),
                            event_object: Some(card),
                            ..base.clone()
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// Whether a `ManaProduced` event is the first mana of an activation that
/// tapped `object` as its cost: "tap [a permanent] for mana" is activating a
/// mana ability of it with {T} in the cost (CR 106.12), and the trigger
/// fires as that ability resolves and produces mana (CR 106.12a).
///
/// Read off the journal, where every mana ability with {T} writes the same
/// pair — the tap under `Cause::Cost`, then the mana, the intrinsic door
/// (`casting::add_intrinsic_mana`) and the resolved ability (`resolve::mana`)
/// alike. The nearest earlier entry about `object` decides: its own tap
/// means this is the tap's first mana, and its own earlier mana means this
/// is the second colour of one activation ("Add {R}{G}"), which is still
/// one tap. A tap to attack is `Cause::TurnBased` and a creature tapped for
/// convoke or crew makes no mana, so neither reaches here.
pub(super) fn first_mana_of_a_tap(
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    object: ObjectId,
) -> bool {
    let Some(at) = event_groups::index_of_event(event, batch) else {
        return false;
    };
    batch[..at]
        .iter()
        .rev()
        .find_map(|entry| match entry.event {
            GameEvent::AbilityActivated {
                source,
                tap_cost,
                mana_ability,
                ..
            } if source.object == object => Some(tap_cost && mana_ability),
            GameEvent::ObjectTapped {
                object: tapped,
                cause: crate::event::Cause::Cost,
            } if tapped == object => Some(true),
            GameEvent::ManaProduced {
                source: Some(made), ..
            } if made == object => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

/// Capture all types made by this activation, stopping at the next tap of
/// the same source. Bonus mana from another permanent is not its production.
pub(super) fn produced_mana(
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
) -> Option<EventMana> {
    let GameEvent::ManaProduced {
        player,
        source: Some(source),
        ..
    } = event
    else {
        return None;
    };
    let at = event_groups::index_of_event(event, batch)?;
    let mut types = 0;
    for entry in &batch[at..] {
        match entry.event {
            GameEvent::ObjectTapped { object, .. } if object == *source => break,
            GameEvent::ManaProduced {
                source: Some(object),
                player: who,
                color,
                amount,
            } if object == *source && who == *player && amount > 0 => {
                types |= 1 << (color as u8);
            }
            _ => {}
        }
    }
    Some(EventMana {
        player: *player,
        types,
    })
}

/// Whether a combat `DamageDealt` is the first of its combat damage step to
/// reach `dealt` — a permanent or a player. All combat damage in a step is
/// dealt at once (CR 510.2), so however many creatures dealt it, `dealt`
/// was dealt damage in one event, and "whenever this creature is dealt
/// damage" or "whenever you're dealt damage" triggers once (CR 603.2c).
/// One step's damage is one batch: nothing is scanned between its events,
/// and first strike's damage is another step.
pub(super) fn first_combat_damage_to(
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    dealt: crate::event::DamageTarget,
) -> bool {
    let Some(at) = event_groups::index_of_event(event, batch) else {
        return false;
    };
    !batch[..at].iter().any(|entry| {
        matches!(
            entry.event,
            GameEvent::DamageDealt {
                target,
                amount,
                is_combat: true,
                ..
            } if target == dealt && amount > 0
        )
    })
}

/// All the combat damage a batch dealt to `recipient`: the one event
/// [`first_combat_damage_to`] fires on, as an amount (CR 510.2).
pub(super) fn combat_damage_in_batch(
    batch: &[crate::event::JournalEntry],
    recipient: crate::event::DamageTarget,
) -> u32 {
    batch
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::DamageDealt {
                target,
                amount,
                is_combat: true,
                ..
            } if target == recipient => Some(amount),
            _ => None,
        })
        .fold(0u32, u32::saturating_add)
}

/// All the combat damage `source` dealt in a batch, to every recipient
/// (CR 510.2): the one event [`Trigger::DealsDamage`] fires on.
pub(super) fn combat_damage_by_in_batch(
    batch: &[crate::event::JournalEntry],
    source: ObjectId,
) -> u32 {
    batch
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::DamageDealt {
                source: Some(from),
                amount,
                is_combat: true,
                ..
            } if from == source => Some(amount),
            _ => None,
        })
        .fold(0u32, u32::saturating_add)
}

/// Whether `event` is the first combat damage `source` dealt in `batch`.
pub(super) fn first_combat_damage_by(
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    source: ObjectId,
) -> bool {
    let Some(at) = event_groups::index_of_event(event, batch) else {
        return false;
    };
    !batch[..at].iter().any(|entry| {
        matches!(
            entry.event,
            GameEvent::DamageDealt {
                source: Some(from),
                amount,
                is_combat: true,
                ..
            } if from == source && amount > 0
        )
    })
}

/// The monarch is read once for the whole batch. A batch is what happened
/// between two scans, and nothing that makes a player the monarch shares
/// one with a step beginning or with combat damage: a resolution is scanned
/// before the next step begins, and damage is scanned before anything
/// resolves.
///
/// Timestamp `0` puts them ahead of the monarch's other triggers from the
/// same batch in the queue, so they go on the stack first and resolve last.
/// The rules let the controller choose that order (CR 603.3b), which this
/// engine does not ask yet for any trigger.
pub(super) fn monarch_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    let Some(monarch) = state.monarch else {
        return;
    };
    let inherent = |effects, event_object| PendingTrigger {
        text: TextChangeMap::IDENTITY,
        source_version: None,
        related_objects: None,
        event_subject: None,
        event_ability: None,
        event_object_identity: None,
        counter_source_version: None,
        event_mana: None,
        event_mana_value: None,
        event_departure: None,
        event_amount: None,
        event_player: None,
        source: ObjectId::NO_SOURCE,
        ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
        abilities: None,
        controller: monarch,
        timestamp: 0,
        event_object,
        implicit_target: event_object,
        synthetic_effects: Some(effects),
        once_per_turn: false,
        synthetic_target: None,
        chosen_mode: None,
    };
    for entry in events {
        match &entry.event {
            GameEvent::StepChanged {
                step: crate::turn::Step::End,
                ..
            } if state.turn.active == monarch => {
                triggers.push(inherent(MONARCH_DRAW, None));
            }
            GameEvent::DamageDealt {
                source: Some(creature),
                target: crate::event::DamageTarget::Player(player),
                is_combat: true,
                ..
            } if *player == monarch
                && state.object_or_departed(*creature).is_some_and(|o| {
                    o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
                }) =>
            {
                triggers.push(inherent(MONARCH_TAKEOVER, Some(*creature)));
            }
            _ => {}
        }
    }
}
