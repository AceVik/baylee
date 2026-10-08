//! Taking back a cast before it is complete (`PlayerAction::CancelCast`).
//!
//! A cast begun from priority keeps the game as it stood when it began
//! ([`CastStart`]). Cancelled, or left short in its payment window, it is put
//! back whole, as CR 732.1 reverses an action that is not completed: "the
//! entire action is reversed and any payments already made are canceled",
//! and the player "may also reverse any legal mana abilities that player
//! activated while making" it. The reversal is taken for the player, so a
//! land tapped for the cast is untapped, the mana it made is gone, mana that
//! floated before the cast is in the pool again, and what a mana ability did
//! beside its mana is undone with it: a reversed painland's damage is not
//! dealt, and "no abilities trigger ... as a result of an undone action", so
//! City of Brass's trigger is not put on the stack. The one thing CR 732.1
//! forbids reversing is a library touched ("moved cards to a library, moved
//! cards from a library to any zone other than the stack, caused a library
//! to be shuffled, or caused cards from a library to be revealed"); a cast
//! whose mana did that is still taken back, and its mana abilities stand.
//! The player keeps priority (CR 732.2).

use super::{CardLookup, Engine, EngineError, GameEvent, ObjectId, Pending, PlayerId};
use crate::zone::Zone;

/// The game as a cast from priority began (CR 601.2), kept to put back if
/// the cast is cancelled: the state, and the engine's own trigger
/// bookkeeping beside it. Shared, because a wizard is cloned to probe its
/// prices.
#[derive(Clone)]
pub(crate) struct CastStart {
    game: std::sync::Arc<crate::state::Checkpoint>,
    triggers: usize,
    trigger_scan_seq: u64,
    entry_scan_seq: u64,
}

impl std::fmt::Debug for CastStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CastStart")
            .field("journal", &self.game.journal_len())
            .field("triggers", &self.triggers)
            .finish_non_exhaustive()
    }
}

impl<L: CardLookup> Engine<L> {
    /// Keeps the game as a cast from priority begins.
    pub(super) fn cast_start(&mut self) -> CastStart {
        CastStart {
            game: std::sync::Arc::new(self.state.checkpoint()),
            triggers: self.trigger_queue.len(),
            trigger_scan_seq: self.trigger_scan_seq,
            entry_scan_seq: self.entry_scan_seq,
        }
    }

    /// The card `player` is casting and may cancel right now
    /// (`PlayerAction::CancelCast`): a cast it began from priority, asked one
    /// of its own questions or standing in its payment window. `None` while a
    /// mana ability inside the window asks its colour (answer that first),
    /// and for every cast an effect makes.
    #[must_use]
    pub fn cancellable_cast(&self, player: PlayerId) -> Option<ObjectId> {
        if self.pending.asked() != Some(player) || self.resolution.is_some() {
            return None;
        }
        if let Some(wizard) = &self.cast_wizard {
            return (wizard.player == player && wizard.started.is_some()).then_some(wizard.card);
        }
        match &self.mana_window {
            Some(super::PaymentWindow {
                player: payer,
                suspended: super::PaymentContinuation::Miracle { wizard, .. },
            }) if *payer == player
                && wizard.started.is_some()
                && matches!(self.pending, Pending::Priority { .. }) =>
            {
                Some(wizard.card)
            }
            _ => None,
        }
    }

    /// Takes back the cast `player` is making (`PlayerAction::CancelCast`).
    pub(super) fn cancel_cast(&mut self, player: PlayerId) -> Result<(), EngineError> {
        if self.cancellable_cast(player).is_none() {
            return Err(EngineError::IllegalAction("no cast to cancel"));
        }
        let wizard = match self.cast_wizard.take() {
            Some(wizard) => wizard,
            None => match self.mana_window.take().map(|w| w.suspended) {
                Some(super::PaymentContinuation::Miracle { wizard, .. }) => *wizard,
                _ => unreachable!("cancellable_cast found the cast in one of the two"),
            },
        };
        let card = wizard.card;
        if !self.rewind_cast(&wizard)
            && let Some(object) = self.state.object_mut(card)
        {
            object.x_value = 0;
        }
        self.state
            .journal
            .record(GameEvent::CastCancelled { player, card });
        self.after_action(player);
        Ok(())
    }

    /// Puts the game back as `wizard`'s cast began, when it began from
    /// priority and nothing since touched a library (CR 732.1). Returns
    /// whether it did.
    pub(super) fn rewind_cast(&mut self, wizard: &super::cast_wizard::CastWizard) -> bool {
        let Some(start) = wizard.started.as_ref() else {
            return false;
        };
        let since = self
            .state
            .journal
            .entries()
            .get(start.game.journal_len()..)
            .unwrap_or_default();
        let touched_a_library = since.iter().any(|entry| {
            matches!(
                entry.event,
                GameEvent::Revealed { .. }
                    | GameEvent::Shuffled { .. }
                    | GameEvent::CardsDrawn { .. }
                    | GameEvent::ZoneChanged {
                        from: Zone::Library,
                        ..
                    }
                    | GameEvent::ZoneChanged {
                        to: Zone::Library,
                        ..
                    }
            )
        });
        if touched_a_library {
            return false;
        }
        self.state.roll_back_to(&start.game);
        self.trigger_queue.truncate(start.triggers);
        self.trigger_scan_seq = start.trigger_scan_seq;
        self.entry_scan_seq = start.entry_scan_seq;
        true
    }
}
