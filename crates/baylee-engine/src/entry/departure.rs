//! Recompute unpaid entry choices when a player leaves the game.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(crate) fn refresh_after_departure(state: &mut GameState) {
    let question = state.entries.question.take();
    let asking = state.entries.pending.front().map(|entry| entry.object);
    let mut pending = std::mem::take(&mut state.entries.pending);
    pending.retain(|entry| {
        !state.has_left(entry.controller)
            && state
                .object(entry.object)
                .is_some_and(|o| o.version == entry.version)
    });
    // No payment has happened while one of these questions is open. Rebuild
    // the exact payment rather than reducing its cardinality to the survivors.
    // Earlier, completed modifiers and replacement applications stay applied.
    if let Some(entry) = pending.front_mut()
        && Some(entry.object) == asking
    {
        match question {
            Some(Question::Copy { .. }) => entry.copy = CopyStatus::Pending,
            Some(
                Question::Number
                | Question::Shape { .. }
                | Question::AcceptSacrifice { .. }
                | Question::Sacrifice { .. }
                | Question::SacrificeCount { .. }
                | Question::Discard
                | Question::DiscardDestination { .. },
            ) => entry.next = entry.next.saturating_sub(1),
            _ => {}
        }
    }
    state.entries.pending = pending;
    state.entries.chosen_entry = None;
    // Appended sacrifices already belong to the replaced event, even if the
    // Equilibrium itself just left. `after_entry` skips departed payers.
}
