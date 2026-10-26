//! Determine every untap replacement before committing simultaneous untaps.
use crate::{
    choice::{Pending, YesNoPrompt},
    event::{Cause, GameEvent},
    state::GameState,
    zone::Zone,
};
use baylee_cards_dsl::{CounterKind, Modifier};
use baylee_core::ids::{DamageSourceRef, ObjectId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Removal {
    kind: CounterKind,
    count: u16,
}

#[derive(Clone, Debug, Hash)]
struct Event {
    source: DamageSourceRef,
    options: Vec<Removal>,
    selected: Option<Removal>,
}

/// Only permanents already determined to untap belong in this batch.
/// Restrictions and a choice to remain tapped precede replacement effects.
#[derive(Clone, Debug, Hash)]
pub(crate) struct UntapBatch {
    events: Vec<Event>,
}

impl UntapBatch {
    pub(crate) fn new(state: &GameState, objects: impl IntoIterator<Item = ObjectId>) -> Self {
        let events = objects
            .into_iter()
            .filter_map(|id| {
                let object = state.object(id)?;
                let mut options = Vec::new();
                for effect in state.effects.iter() {
                    let Modifier::RemoveCountersInsteadOfUntapping(kind) = effect.modifier else {
                        continue;
                    };
                    let count = object.counters.get(kind);
                    let removal = Removal { kind, count };
                    if count != 0
                        && crate::effects::applies_to(state, effect, object)
                        && !options.contains(&removal)
                    {
                        options.push(removal);
                    }
                }
                if object.counters.get(CounterKind::Stun) != 0 {
                    let removal = Removal {
                        kind: CounterKind::Stun,
                        count: 1,
                    };
                    if !options.contains(&removal) {
                        options.push(removal);
                    }
                }
                Some(Event {
                    source: DamageSourceRef {
                        object: id,
                        version: object.version,
                    },
                    selected: (options.len() == 1).then(|| options[0]),
                    options,
                })
            })
            .collect();
        Self { events }
    }

    pub(crate) fn pending(&self, state: &GameState) -> Option<Pending> {
        let event = self
            .events
            .iter()
            .find(|event| event.selected.is_none() && event.options.len() > 1)?;
        let object = state.object(event.source.object)?;
        let next = event.options[0];
        Some(Pending::YesNo {
            player: object.controller,
            prompt: YesNoPrompt::RemoveCountersInsteadOfUntapping {
                permanent: event.source.object,
                kind: next.kind,
                count: next.count,
            },
            source: None,
        })
    }

    pub(crate) fn answer(&mut self, choose: bool) {
        let event = self
            .events
            .iter_mut()
            .find(|event| event.selected.is_none() && event.options.len() > 1)
            .expect("offered untap replacement");
        if choose {
            event.selected = Some(event.options[0]);
        } else {
            event.options.remove(0);
            if event.options.len() == 1 {
                event.selected = Some(event.options[0]);
            }
        }
    }

    pub(crate) fn commit(self, state: &mut GameState) {
        for event in self.events {
            if !state.object(event.source.object).is_some_and(|object| {
                object.version == event.source.version && object.zone == Zone::Battlefield
            }) {
                continue;
            }
            if let Some(removal) = event.selected {
                crate::replacement::remove_counters(
                    state,
                    event.source.object,
                    removal.kind,
                    removal.count,
                );
            } else {
                debug_assert!(
                    event.options.is_empty(),
                    "all replacement decisions precede writes"
                );
                // Stun was included in this batch. Re-entering set_tapped would
                // apply an already determined replacement a second time.
                if state.restore_tapped(event.source.object, false) {
                    state.journal.record(GameEvent::ObjectUntapped {
                        object: event.source.object,
                        cause: Cause::TurnBased,
                    });
                }
            }
        }
    }
}
