//! Instructions performed as permanents untap, without a priority window.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup> Engine<L> {
    pub(super) fn return_during_untap(&mut self) {
        let active = self.state.turn.active;
        // Partition once: several activations are independent instructions,
        // while an ordinary untap allocates nothing when no return is due.
        let mut due = Vec::new();
        self.state.delayed.retain(|delayed| {
            let matches = delayed.controller == active
                && delayed.when == crate::state::DelayedWhen::NextUntap;
            if matches {
                if let crate::state::DelayedAction::ReturnToHandDuringUntap { card, version } =
                    delayed.action
                {
                    due.push((card, version));
                } else {
                    debug_assert!(false, "non-untap action scheduled during untap");
                }
            }
            !matches
        });
        for (card, version) in due {
            let Some(owner) = self
                .state
                .object(card)
                .filter(|object| {
                    object.version == version
                        && object.zone == Zone::Battlefield
                        && !object.status.contains(Status::PHASED_OUT)
                })
                .map(|object| object.owner)
            else {
                continue;
            };
            let _ = self.state.move_object(
                card,
                ZoneLocation::Hand(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
    }
}
