//! Simultaneous destruction choices share APNAP order across replacement kinds.
use crate::{choice::Pending, state::GameState};

/// Both queues only prune stale entries while offering their head. Comparing
/// those offers consumes neither event; every answer re-evaluates both queues.
pub(crate) fn destruction_choice(state: &mut GameState) -> Option<Pending> {
    let candidates = [
        super::damage_clearing_choice(state),
        super::shield_choice(state),
        super::regeneration_choice(state),
    ];
    let active = usize::from(state.turn.active.get());
    let seats = state.players.len();
    candidates.into_iter().flatten().min_by_key(|pending| {
        (usize::from(
            pending
                .asked()
                .expect("destruction choice has a player")
                .get(),
        ) + seats
            - active)
            % seats
    })
}
