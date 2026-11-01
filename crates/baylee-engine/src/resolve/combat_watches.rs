//! Register a global, recurring combat event observer for the current turn.
use super::{GameState, Resolution, source_version};
use baylee_cards_dsl::Effect;

pub(super) fn start(state: &mut GameState, res: &Resolution, op: Effect) {
    let Effect::WheneverCombatEventThisTurn {
        event,
        filter,
        this_combat_only,
        targets,
        effects,
    } = op
    else {
        unreachable!("combat watch dispatcher");
    };
    if this_combat_only && state.turn.phase != crate::turn::Phase::Combat {
        return;
    }
    state.delayed.push(crate::state::DelayedTrigger {
        controller: res.controller,
        when: crate::state::DelayedWhen::TriggeredThisTurn {
            event,
            filter,
            this_combat_only,
            targets,
            context: res.rule_context(),
            after: state.journal.last_seq(),
        },
        action: crate::state::DelayedAction::Trigger {
            source: res.source,
            source_version: source_version(state, res)
                .or_else(|| state.object(res.source).map(|o| o.version))
                .unwrap_or(0),
            text: res.text,
            effects,
        },
    });
}
