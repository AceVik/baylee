//! Prospective event matching for exact-incarnation, one-shot observers.

use super::DelayedWhen;
use crate::event::{GameEvent, JournalEntry};
use crate::zone::Zone;
use baylee_cards_dsl::DelayedObservation;
use baylee_core::ids::PlayerId;

impl DelayedWhen {
    pub(crate) fn observation_fired(self, events: &[JournalEntry], controller: PlayerId) -> bool {
        let Self::Observed { watch, when, after } = self else {
            return false;
        };
        for entry in events.iter().filter(|entry| entry.seq > after) {
            match entry.event {
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Battlefield,
                    ..
                } if object == watch.object => {
                    // The first departure ends the watch even when it does
                    // not satisfy its condition. Never follow a later entry.
                    return entry.departure.as_ref().is_some_and(|departure| {
                        departure.version == watch.version
                            && match when {
                                DelayedObservation::LeavesBattlefieldOrUntaps => true,
                                DelayedObservation::YouLoseControl => {
                                    departure.controller == controller
                                }
                            }
                    });
                }
                GameEvent::ObjectUntapped { object, .. }
                    if object == watch.object
                        && when == DelayedObservation::LeavesBattlefieldOrUntaps =>
                {
                    return true;
                }
                GameEvent::ControllerChanged { object, old, new }
                    if object == watch.object
                        && old == controller
                        && new != controller
                        && when == DelayedObservation::YouLoseControl =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Departure;
    use baylee_core::ids::{DamageSourceRef, ObjectId};

    fn watch(when: DelayedObservation) -> DelayedWhen {
        DelayedWhen::Observed {
            watch: DamageSourceRef {
                object: ObjectId::NO_SOURCE,
                version: 7,
            },
            when,
            after: 10,
        }
    }

    fn change(seq: u64, old: u8, new: u8) -> JournalEntry {
        JournalEntry {
            seq,
            event: GameEvent::ControllerChanged {
                object: ObjectId::NO_SOURCE,
                old: PlayerId::new(old),
                new: PlayerId::new(new),
            },
            departure: None,
        }
    }

    fn leave(version: u32, controller: u8) -> JournalEntry {
        JournalEntry {
            seq: 12,
            event: GameEvent::ZoneChanged {
                object: ObjectId::NO_SOURCE,
                from: Zone::Battlefield,
                to: Zone::Hand,
                cause: crate::event::Cause::Effect,
                place: None,
            },
            departure: Some(Box::new(Departure {
                sacrificed: false,
                return_keywords: [0, 0],
                version,
                power: 0,
                controller: PlayerId::new(controller),
                toughness: 0,
                attachments: Vec::new(),
            })),
        }
    }

    #[test]
    fn delayed_control_observer_ignores_prior_events_and_other_players_control_loss() {
        let observation = watch(DelayedObservation::YouLoseControl);
        let you = PlayerId::new(0);
        assert!(!observation.observation_fired(&[change(10, 0, 1)], you));
        assert!(!observation.observation_fired(&[change(11, 1, 0)], you));
        assert!(observation.observation_fired(&[change(11, 1, 0), change(12, 0, 1)], you));
        assert!(observation.observation_fired(&[leave(7, 0)], you));
        assert!(!observation.observation_fired(&[leave(7, 1)], you));
        assert!(!observation.observation_fired(&[leave(9, 0)], you));
        assert!(!observation.observation_fired(&[leave(7, 1), change(13, 0, 1)], you));
    }

    #[test]
    fn delayed_leave_or_untap_observer_ignores_control_change() {
        let observation = watch(DelayedObservation::LeavesBattlefieldOrUntaps);
        let you = PlayerId::new(0);
        assert!(!observation.observation_fired(&[change(11, 0, 1)], you));
        assert!(observation.observation_fired(&[leave(7, 1)], you));
        assert!(!observation.observation_fired(&[leave(8, 1)], you));
        let mut untap = JournalEntry {
            seq: 10,
            event: GameEvent::ObjectUntapped {
                object: ObjectId::NO_SOURCE,
                cause: crate::event::Cause::Effect,
            },
            departure: None,
        };
        assert!(!observation.observation_fired(&[untap.clone()], you));
        untap.seq = 11;
        assert!(observation.observation_fired(&[untap], you));
    }
}
