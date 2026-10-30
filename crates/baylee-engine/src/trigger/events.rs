//! What a journal entry tells a trigger: its damage, player and objects, the targets it adds, and where a source stood.

use super::watches::{combat_damage_by_in_batch, combat_damage_in_batch};
use crate::event::GameEvent;
use crate::state::GameState;
use baylee_cards_dsl::Trigger;
use baylee_core::ids::{ObjectId, PlayerId};

/// How much damage a damage event dealt, as `trigger` reads it: to a
/// player or permanent, or for [`Trigger::DealsDamage`] to anything.
///
/// Per entry, except where the trigger is about the player rather than the
/// source: [`Trigger::PlayerDealtDamage`] fires once for a step's combat
/// damage (CR 510.2, 603.2c), and "that many" is all of it, so its amount
/// is the batch's total to that player. A source's trigger
/// (`DealsCombatDamageToOpponent`) keeps its own share: two creatures are
/// two sources, each dealing its own damage. [`Trigger::DealsDamage`] is
/// the source's side of that: once for a step's combat damage, so "that
/// much" is all the combat damage the source dealt in the step, to
/// whatever it was dealt.
pub(super) fn event_damage_of(
    trigger: &Trigger,
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
) -> Option<u32> {
    match event {
        GameEvent::DamageDealt {
            source: Some(source),
            is_combat: true,
            ..
        } if matches!(
            trigger,
            Trigger::DealsDamage(_) | Trigger::DealsCombatDamage(_)
        ) =>
        {
            Some(combat_damage_by_in_batch(batch, *source))
        }
        GameEvent::DamageDealt { amount, .. } if matches!(trigger, Trigger::DealsDamage(_)) => {
            Some(*amount)
        }
        GameEvent::DamageDealt {
            target,
            is_combat: true,
            ..
        } if matches!(
            trigger,
            Trigger::PlayerDealtDamage(_) | Trigger::DealtDamage(_) | Trigger::DealtCombatDamage(_)
        ) =>
        {
            Some(combat_damage_in_batch(batch, *target))
        }
        GameEvent::DamageDealt { amount, .. } => Some(*amount),
        GameEvent::LifeChanged { old, new, .. } => Some(new.abs_diff(*old)),
        _ => None,
    }
}

/// The player a triggering event is about: "that player" or "they" of the
/// ability it triggers (`PlayerRel::EventPlayer`, CR 603.2).
///
/// The one who drew, cast the spell (CR 112.2: the player who put it on the
/// stack), played the land, cycled or discarded the card, whose life
/// changed, whose pool the mana went to, or who was dealt the damage. An
/// event that is no player's, or one about an object (whose controller is
/// `PlayerRel::ControllerOfEvent`), has none. Read here, as the trigger is
/// collected, because by the time the ability resolves the event is gone:
/// a draw leaves no object behind to ask.
pub(super) fn event_player_of(event: &GameEvent) -> Option<PlayerId> {
    match event {
        GameEvent::StepChanged { active: player, .. }
        | GameEvent::AbilityActivated { player, .. }
        | GameEvent::AbilityTriggered {
            controller: player, ..
        }
        | GameEvent::BecameTarget {
            controller: player, ..
        }
        | GameEvent::PlayerBecameTarget {
            controller: player, ..
        }
        | GameEvent::CardsDrawn { player, .. }
        | GameEvent::SpellCast { player, .. }
        | GameEvent::SpellCopied { player, .. }
        | GameEvent::LandPlayed { player, .. }
        | GameEvent::Cycled { player, .. }
        | GameEvent::Discarded { player, .. }
        | GameEvent::LifeChanged { player, .. }
        | GameEvent::ManaProduced { player, .. }
        | GameEvent::LibrarySearched { player }
        | GameEvent::DamageDealt {
            target: crate::event::DamageTarget::Player(player),
            ..
        } => Some(*player),
        _ => None,
    }
}

/// The object a trigger's event is about, as the triggered ability reads it
/// ([`TargetSpec::EventObject`](baylee_cards_dsl::TargetSpec)).
///
/// The event decides, except where one event names two objects and the
/// trigger says which it means. A blocker's declaration names the blocker
/// and the creature it blocks; "whenever this creature blocks or becomes
/// blocked by a non-Wall creature, destroy that creature" is about the one
/// that is not the source, whichever side of the block that is.
pub(super) fn event_object_for(
    trigger: &Trigger,
    event: &GameEvent,
    source: ObjectId,
    state: &GameState,
) -> Option<ObjectId> {
    match (trigger, event) {
        // The other creature of the pair: the one that is not the creature
        // this source is attached to.
        (
            Trigger::AttachedBlocksOrBecomesBlockedBy(_),
            GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            },
        ) => {
            let host = state.object(source)?.attached_to?;
            Some(if *blocker == host {
                *attacker
            } else {
                *blocker
            })
        }
        (
            Trigger::BecomesBlockedBy(_) | Trigger::BecomesBlocked,
            GameEvent::BecameBlocker {
                object: blocker, ..
            },
        ) => Some(*blocker),
        (
            Trigger::DealsCombatDamageToCreature(_)
            | Trigger::DealtDamage(_)
            | Trigger::DealtCombatDamage(_),
            GameEvent::DamageDealt {
                target: crate::event::DamageTarget::Object(recipient),
                ..
            },
        ) => Some(*recipient),
        (
            Trigger::DealsDamageToPlayer { .. }
            | Trigger::OneOrMoreDealCombatDamageToPlayer(_)
            | Trigger::DealsCombatDamageToYou(_)
            | Trigger::DealsCombatDamage(_)
            | Trigger::DealsCombatDamageToPlayerOrPlaneswalker(_),
            GameEvent::DamageDealt {
                source: damage_source,
                ..
            },
        ) => *damage_source,
        (Trigger::BlocksCreature(_), GameEvent::BecameBlocker { attacker, .. }) => Some(*attacker),
        (
            Trigger::BlocksOrBecomesBlockedBy(_) | Trigger::BlocksOrBecomesBlockedByOneOrMore(_),
            GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            },
        ) => Some(if *blocker == source {
            *attacker
        } else {
            *blocker
        }),
        _ => event_object_of(event),
    }
}

/// The creature a pair trigger about an attached creature is about
/// (`Trigger::AttachedBlocksOrBecomesBlockedBy`), as the object it is now:
/// the one of the pair this source is attached to.
pub(super) fn event_subject_for(
    trigger: &Trigger,
    event: &GameEvent,
    source: ObjectId,
    state: &GameState,
) -> Option<baylee_core::ids::DamageSourceRef> {
    let (
        Trigger::AttachedBlocksOrBecomesBlockedBy(_),
        GameEvent::BecameBlocker {
            object: blocker,
            attacker,
        },
    ) = (trigger, event)
    else {
        return None;
    };
    let host = state.object(source)?.attached_to?;
    if host != *blocker && host != *attacker {
        return None;
    }
    let version = state.object(host)?.version;
    Some(baylee_core::ids::DamageSourceRef {
        object: host,
        version,
    })
}

/// The object an event is about, if any.
pub(super) fn event_object_of(event: &GameEvent) -> Option<ObjectId> {
    match event {
        GameEvent::AbilityActivated { source, .. } => Some(source.object),
        GameEvent::ZoneChanged { object, .. }
        | GameEvent::SpellCast { object, .. }
        | GameEvent::SpellCopied { object, .. }
        | GameEvent::Discarded { object, .. }
        | GameEvent::CounterChanged { object, .. }
        | GameEvent::AbilityTriggered { object, .. }
        | GameEvent::BecameTarget { object, .. }
        | GameEvent::PlayerBecameTarget { object, .. }
        | GameEvent::BecameAttacker { object, .. }
        | GameEvent::BecameBlocker { object, .. }
        | GameEvent::AttackerBlocked { object }
        | GameEvent::CreatureMutated { object }
        | GameEvent::AttackerUnblocked { object }
        // The permanent that changed tap state: "that land's controller"
        // of Psychic Venom and "that permanent's controller" of Mesmeric Orb.
        | GameEvent::ObjectTapped { object, .. }
        | GameEvent::ObjectUntapped { object, .. }
        // The permanent tapped for mana (CR 106.12a): "its controller" and
        // "that player" of Gauntlet of Might and Manabarbs.
        | GameEvent::ManaProduced {
            source: Some(object),
            ..
        } => Some(*object),
        _ => None,
    }
}

/// The stack object that has just acquired this target, and its controller.
/// Retargeting names only newly acquired targets; retaining one never retriggers ward.
pub(super) fn targeting(
    event: &GameEvent,
    state: &GameState,
    target: ObjectId,
) -> Option<(ObjectId, PlayerId)> {
    match *event {
        GameEvent::BecameTarget {
            object,
            target: acquired,
            controller,
        } => (acquired == target).then_some((object, controller)),
        GameEvent::SpellCast {
            object,
            player: controller,
            ..
        }
        | GameEvent::AbilityTriggered {
            object, controller, ..
        } => state
            .object(object)
            .filter(|_| state.targets_current_object(object, target))
            .map(|_| (object, controller)),
        _ => None,
    }
}

pub(super) struct ObjectMove {
    seq: u64,
    battlefield_departure_version: Option<u32>,
}

pub(super) fn object_moves(
    events: &[crate::event::JournalEntry],
    batch_has_zone_changes: bool,
    id: ObjectId,
) -> smallvec::SmallVec<[ObjectMove; 2]> {
    let mut moves = smallvec::SmallVec::new();
    if !batch_has_zone_changes {
        return moves;
    }
    for entry in events {
        if let GameEvent::ZoneChanged { object, from, .. } = entry.event
            && object == id
        {
            moves.push(ObjectMove {
                seq: entry.seq,
                battlefield_departure_version: (from == crate::zone::Zone::Battlefield)
                    .then(|| entry.departure.as_ref().map(|d| d.version))
                    .flatten(),
            });
        }
    }
    moves
}

/// The source and the event object are different identities. A death's event
/// object is the card in its destination (CR 400.7e); the ability's source is
/// the permanent immediately before departure (CR 113.7a, 603.10a).
pub(super) fn source_at_event<'a>(
    state: &'a GameState,
    current: &'a crate::object::GameObject,
    events: &[crate::event::JournalEntry],
    event: &crate::event::JournalEntry,
    look_back: bool,
) -> &'a crate::object::GameObject {
    let moves = object_moves(events, true, current.id);
    source_at_event_with_moves(state, current, &moves, event.seq, look_back)
}

pub(super) fn source_at_event_with_moves<'a>(
    state: &'a GameState,
    current: &'a crate::object::GameObject,
    moves: &[ObjectMove],
    event_seq: u64,
    look_back: bool,
) -> &'a crate::object::GameObject {
    if moves.is_empty() && (!look_back || current.zone == crate::zone::Zone::Battlefield) {
        return current;
    }
    let subsequent_moves = moves.iter().filter(|m| m.seq > event_seq).count();
    let at_event = if subsequent_moves == 0 {
        current
    } else {
        let version = current
            .version
            .wrapping_sub(u32::try_from(subsequent_moves).expect("zone changes fit version"));
        state
            .source_object(baylee_core::ids::DamageSourceRef {
                object: current.id,
                version,
            })
            .unwrap_or(current)
    };
    if !look_back || at_event.zone == crate::zone::Zone::Battlefield {
        return at_event;
    }
    // Simultaneous departures have separate journal rows. The source may
    // precede another creature whose death it observes; use its own departure,
    // never the victim's version or an unrelated later incarnation.
    moves
        .iter()
        .rev()
        .filter(|m| m.seq <= event_seq)
        .find_map(|m| {
            m.battlefield_departure_version.and_then(|version| {
                state.source_object(baylee_core::ids::DamageSourceRef {
                    object: current.id,
                    version,
                })
            })
        })
        .unwrap_or(at_event)
}
