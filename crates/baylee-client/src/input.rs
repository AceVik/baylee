//! Input: keyboard first, pointer second.
//!
//! `docs/keyboard-map.md` makes a commitment — every choice the game can ask is
//! answerable without a pointer, and nothing requires drag-and-drop. That is
//! not only an accessibility promise; a competitive player passing priority
//! forty times a turn will not reach for a mouse each time.
//!
//! Both paths converge on the same place: they build a [`PlayerAction`] through
//! [`baylee_client_core::interaction::Interaction`], which refuses anything the
//! engine did not offer. No input handler decides legality by itself.

use crate::Duel;
use crate::table::CardVisual;
use baylee_client_core::interaction::Interaction;
use bevy::prelude::*;

/// Keyboard handling, following `docs/keyboard-map.md`.
pub fn keyboard(keys: Res<ButtonInput<KeyCode>>, mut duel: ResMut<Duel>) {
    if duel.interaction.is_none() {
        return;
    }

    // Confirm / pass priority.
    if (keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter))
        && let Some(action) = duel.interaction.as_ref().and_then(Interaction::confirm)
    {
        duel.submit(action);
        return;
    }

    // Mulligan: keep or take.
    if keys.just_pressed(KeyCode::KeyK)
        && let Some(action) = duel
            .interaction
            .as_ref()
            .and_then(|i| i.answer_mulligan(true))
    {
        duel.submit(action);
        return;
    }
    if keys.just_pressed(KeyCode::KeyB)
        && let Some(action) = duel
            .interaction
            .as_ref()
            .and_then(|i| i.answer_mulligan(false))
    {
        duel.submit(action);
        return;
    }

    // Yes / no.
    if keys.just_pressed(KeyCode::KeyY)
        && let Some(action) = duel
            .interaction
            .as_ref()
            .and_then(|i| i.answer_yes_no(true))
    {
        duel.submit(action);
        return;
    }
    if keys.just_pressed(KeyCode::KeyN)
        && let Some(action) = duel
            .interaction
            .as_ref()
            .and_then(|i| i.answer_yes_no(false))
    {
        duel.submit(action);
        return;
    }

    // Number choices: arrows step, and the value is clamped to the offered
    // range by the interaction, so a player can hold a key without producing
    // something the engine would reject.
    if (keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::ArrowRight))
        && let Some(i) = duel.interaction.as_mut()
    {
        let next = i.number().saturating_add(1);
        i.set_number(next);
    }
    if (keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::ArrowLeft))
        && let Some(i) = duel.interaction.as_mut()
    {
        let next = i.number().saturating_sub(1);
        i.set_number(next);
    }

    // Cancel clears a half-built selection without answering.
    if keys.just_pressed(KeyCode::Escape)
        && let Some(i) = duel.interaction.as_mut()
    {
        i.cancel();
    }

    // Cycle the focused opponent, so an eight-seat table is navigable without
    // hunting for a small pod with the pointer.
    if keys.just_pressed(KeyCode::Tab) {
        cycle_focus(&mut duel);
    }
}

/// Moves the inspection focus to the next opponent.
fn cycle_focus(duel: &mut Duel) {
    let Some(board) = duel.board.as_ref() else {
        return;
    };
    let opponents: Vec<_> = board
        .pods
        .iter()
        .filter(|p| !p.is_local)
        .map(|p| p.player)
        .collect();
    if opponents.is_empty() {
        return;
    }
    duel.focus = match duel.focus {
        None => Some(opponents[0]),
        Some(current) => {
            let index = opponents.iter().position(|p| *p == current);
            match index {
                Some(i) if i + 1 < opponents.len() => Some(opponents[i + 1]),
                // Cycling past the last opponent returns to your own board.
                _ => None,
            }
        }
    };
}

/// Pointer handling: clicking a card.
///
/// A click means "this object", and what that does depends entirely on the
/// pending choice: it selects a target, declares an attacker, or plays a card.
/// Resolving that here rather than in the renderer keeps one place where a
/// click becomes an action.
pub fn pointer(
    clicks: MessageReader<Pointer<Click>>,
    cards: Query<&CardVisual>,
    duel: ResMut<Duel>,
) {
    handle_clicks(clicks, &cards, duel);
}

fn handle_clicks(
    mut clicks: MessageReader<Pointer<Click>>,
    cards: &Query<&CardVisual>,
    mut duel: ResMut<Duel>,
) {
    for click in clicks.read() {
        let Ok(visual) = cards.get(click.entity) else {
            continue;
        };
        let object = visual.object;

        // While holding priority a click plays the card, when the engine said
        // it is playable.
        if let Some(action) = duel.interaction.as_ref().and_then(|i| i.play_card(object)) {
            duel.submit(action);
            continue;
        }

        // Otherwise it is a selection for the pending choice; the interaction
        // rejects anything that was not offered.
        if let Some(i) = duel.interaction.as_mut() {
            i.toggle(object);
        }
    }
}

#[cfg(test)]
mod tests {
    use baylee_client_core::interaction::{CombatCandidates, Interaction};
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_engine::choice::{LegalActions, Pending, PlayerAction};

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    #[test]
    fn confirming_a_priority_choice_passes() {
        let i = Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    can_pass: true,
                    lands: vec![obj(1)],
                    castable: vec![],
                    mana_abilities: vec![],
                    abilities: vec![],
                    suspendable: vec![],
                }),
            },
            PlayerId::new(0),
            &CombatCandidates::default(),
        );
        assert_eq!(i.confirm(), Some(PlayerAction::PassPriority));
        // And a click on the land plays it instead of passing.
        assert_eq!(
            i.play_card(obj(1)),
            Some(PlayerAction::PlayLand { card: obj(1) })
        );
    }

    #[test]
    fn a_click_on_something_the_engine_did_not_offer_does_nothing() {
        let mut i = Interaction::new(
            Pending::ChooseTargets {
                player: PlayerId::new(0),
                options: vec![obj(1)],
                min: 1,
                max: 1,
            },
            PlayerId::new(0),
            &CombatCandidates::default(),
        );
        i.toggle(obj(99));
        assert!(i.selected().is_empty());
        assert!(!i.can_confirm());
    }
}
