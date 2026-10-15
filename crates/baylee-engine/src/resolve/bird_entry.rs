//! Delayed cast watches and the entry counters they grant to a spell.
use super::{GameState, Resolution};
use baylee_cards_dsl::{Effect, Filter};

pub(super) fn register(
    state: &mut GameState,
    res: &Resolution,
    filter: &'static Filter,
    effects: &'static [Effect],
) {
    state.delayed.push(crate::state::DelayedTrigger {
        controller: res.controller,
        when: crate::state::DelayedWhen::NextSpellCastThisTurn {
            filter,
            after: state.journal.last_seq(),
        },
        action: crate::state::DelayedAction::Trigger {
            source: res.source,
            source_version: super::source_version(state, res)
                .or_else(|| state.object(res.source).map(|object| object.version))
                .unwrap_or(0),
            effects,
            text: res.text,
        },
    });
}

pub(super) fn grant(state: &mut GameState, res: &Resolution, n: u32) {
    let Some(reference) = super::subjects::event(state, res) else {
        return;
    };
    if super::subjects::is_current(state, reference)
        && let Some(object) = state.object_mut(reference.object)
        && object.zone == crate::zone::Zone::Stack
    {
        object.riders.push(crate::object::Rider::EntryP1P1(n));
    }
}
