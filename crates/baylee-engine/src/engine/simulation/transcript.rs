//! A fork's journal: the real one, less what the forking seat was not
//! shown.
//!
//! The journal cannot be cut short. Rules read back into it — the spells
//! cast this turn (`resolve/half_damage.rs`), damage dealt this game since a
//! source arrived (`resolve/life.rs`), an event's object by its sequence
//! number (`damage_history.rs`) — and the driver's cursors are sequence
//! numbers into it. So every entry stays where it is, and what names a
//! hidden card is rewritten: a dealt card's old id and a move between two
//! zones the seat cannot see name [`ObjectId::NO_SOURCE`], an object that
//! no longer exists anywhere; the seed the game was set up from is zeroed;
//! a developer command loses its text. What remains is what the seat saw
//! happen.

use super::ForkRefusal;
use crate::event::{DamageTarget, GameEvent, Journal};
use crate::state::GameState;
use crate::zone::Zone;
use baylee_core::ids::{Defender, ObjectId, PlayerId};

/// Whether the journal can be shown to `seat` once rewritten.
///
/// A standing answer another seat's policy gave (`GameEvent::AutoAnswered`)
/// is that seat's setting, told to it alone. There is no entry to put in
/// its place that says nothing, so a journal holding one is refused.
///
/// # Errors
/// [`ForkRefusal::PrivatePolicy`].
pub(super) fn check(journal: &Journal, seat: PlayerId) -> Result<(), ForkRefusal> {
    if journal.entries().iter().any(
        |entry| matches!(entry.event, GameEvent::AutoAnswered { player, .. } if player != seat),
    ) {
        return Err(ForkRefusal::PrivatePolicy);
    }
    Ok(())
}

/// Rewrites a fork's copy of `source`'s journal for `seat`; `retired` is
/// the sorted ids of the cards the fork dealt again.
pub(super) fn withhold(
    journal: &mut Journal,
    source: &GameState,
    seat: PlayerId,
    retired: &[ObjectId],
) {
    let unseen = |zone: Zone, object: ObjectId| match zone {
        Zone::Library | Zone::OutsideGame => true,
        // Cards go to their owner's hand; an object the game no longer
        // holds is treated as somebody else's.
        Zone::Hand => source.object(object).is_none_or(|o| o.owner != seat),
        Zone::Battlefield | Zone::Graveyard | Zone::Exile | Zone::Stack | Zone::Command => false,
    };
    journal.rewrite(|entry| {
        match &mut entry.event {
            GameEvent::GameStarted { seed, .. } => *seed = 0,
            GameEvent::DevCommandApplied { command, .. } => command.clear(),
            GameEvent::ZoneChanged {
                object, from, to, ..
            } if unseen(*from, *object) && unseen(*to, *object) => {
                *object = ObjectId::NO_SOURCE;
            }
            _ => {}
        }
        let mut forget = |id: &mut ObjectId| {
            if retired.binary_search(id).is_ok() {
                *id = ObjectId::NO_SOURCE;
            }
        };
        for_each_object(&mut entry.event, &mut forget);
        if let Some(departure) = &mut entry.departure {
            departure.attachments.iter_mut().for_each(&mut forget);
        }
    });
}

/// Every object an event names, mutably.
#[allow(clippy::too_many_lines)] // one arm per event: the list is the guard
fn for_each_object(event: &mut GameEvent, f: &mut impl FnMut(&mut ObjectId)) {
    match event {
        GameEvent::AbilityActivated {
            ability, source, ..
        } => {
            if let Some(ability) = ability {
                f(&mut ability.object);
            }
            f(&mut source.object);
        }
        GameEvent::BecameTarget { object, target, .. } => {
            f(object);
            f(target);
        }
        GameEvent::Revealed { cards, .. } | GameEvent::CardsKept { cards, .. } => {
            cards.iter_mut().for_each(f);
        }
        GameEvent::BattleDefeated { source, .. } | GameEvent::Connived { object: source, .. } => {
            f(&mut source.object);
        }
        GameEvent::DamageDealt { source, target, .. } => {
            if let Some(source) = source {
                f(source);
            }
            if let DamageTarget::Object(object) = target {
                f(object);
            }
        }
        GameEvent::ManaProduced { source, .. } => {
            if let Some(source) = source {
                f(source);
            }
        }
        GameEvent::AbilityTriggered { object, source, .. } => {
            f(object);
            f(source);
        }
        GameEvent::BecameAttacker {
            object, defending, ..
        } => {
            f(object);
            if let Defender::Planeswalker(defender) | Defender::Battle(defender) = defending {
                f(defender);
            }
        }
        GameEvent::Banded { object, with } => {
            f(object);
            f(with);
        }
        GameEvent::BecameBlocker { object, attacker } => {
            f(object);
            f(attacker);
        }
        GameEvent::AutoAnswered { object, .. } => {
            if let Some(object) = object {
                f(object);
            }
        }
        GameEvent::PlayerBecameTarget { object, .. }
        | GameEvent::ZoneChanged { object, .. }
        | GameEvent::CounterChanged { object, .. }
        | GameEvent::ObjectTapped { object, .. }
        | GameEvent::ObjectUntapped { object, .. }
        | GameEvent::LandPlayed { object, .. }
        | GameEvent::SpellCast { object, .. }
        | GameEvent::SpellCopied { object, .. }
        | GameEvent::StackObjectResolved { object }
        | GameEvent::StackObjectDidNotResolve { object }
        | GameEvent::SpellCountered { object }
        | GameEvent::Discarded { object, .. }
        | GameEvent::Cycled { object, .. }
        | GameEvent::AttackerUnblocked { object }
        | GameEvent::ControllerChanged { object, .. }
        | GameEvent::PhaseChanged { object, .. }
        | GameEvent::CastCancelled { card: object, .. }
        | GameEvent::TurnedFaceUp { object }
        | GameEvent::Transformed { object, .. }
        | GameEvent::DoorUnlocked { object, .. }
        | GameEvent::AttackerBlocked { object }
        | GameEvent::CreatureMutated { object } => f(object),
        GameEvent::GameStarted { .. }
        | GameEvent::CoinFlipped { .. }
        | GameEvent::Shuffled { .. }
        | GameEvent::PlayerCountersChanged { .. }
        | GameEvent::LifeChanged { .. }
        | GameEvent::SpeedTriggered { .. }
        | GameEvent::TurnStarted { .. }
        | GameEvent::StepChanged { .. }
        | GameEvent::DiceRolled { .. }
        | GameEvent::CardsDrawn { .. }
        | GameEvent::LibrarySearched { .. }
        | GameEvent::PlayerLost { .. }
        | GameEvent::DayNightChanged { .. }
        | GameEvent::BecameMonarch { .. }
        | GameEvent::LoopDetected { .. }
        | GameEvent::GameWon { .. }
        | GameEvent::DevCommandApplied { .. } => {}
    }
}
