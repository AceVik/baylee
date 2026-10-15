//! Simultaneous destruction choices share APNAP order across replacement kinds.
use crate::{choice::Pending, state::GameState};

/// Both queues only prune stale entries while offering their head. Comparing
/// those offers consumes neither event; every answer re-evaluates both queues.
pub(crate) fn destruction_choice(state: &mut GameState) -> Option<Pending> {
    let shield = super::shield_choice(state);
    let regeneration = super::regeneration_choice(state);
    match (shield, regeneration) {
        (Some(shield), Some(regeneration)) => {
            let active = usize::from(state.turn.active.get());
            let seats = state.players.len();
            let order = |pending: &Pending| {
                (usize::from(
                    pending
                        .asked()
                        .expect("destruction choice has a player")
                        .get(),
                ) + seats
                    - active)
                    % seats
            };
            if order(&regeneration) < order(&shield) {
                Some(regeneration)
            } else {
                Some(shield)
            }
        }
        (shield, regeneration) => shield.or(regeneration),
    }
}
