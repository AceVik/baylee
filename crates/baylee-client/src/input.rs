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
use crate::hud::HandCardVisual;
use crate::table::CardVisual;
use baylee_client_core::interaction::Interaction;
use baylee_core::ids::ObjectId;
use bevy::prelude::*;

/// The one way a card becomes an action: play it when the engine offers
/// that, otherwise select it for the pending choice. Clicks and the
/// keyboard cursor both end here, so they can never disagree.
pub fn activate_card(duel: &mut Duel, object: ObjectId) {
    if let Some(action) = duel.interaction.as_ref().and_then(|i| i.play_card(object)) {
        duel.submit(action);
        return;
    }
    if let Some(i) = duel.interaction.as_mut() {
        i.toggle(object);
    }
}

/// Keyboard handling, following `docs/keyboard-map.md`.
pub fn keyboard(keys: Res<ButtonInput<KeyCode>>, mut duel: ResMut<Duel>) {
    // WASD moves the card cursor across the table grid (hand row at the
    // bottom, then the local board, then the opponents), E activates the
    // card under it — the same thing a click would do.
    let cursor_move = if keys.just_pressed(KeyCode::KeyA) {
        Some((0, -1))
    } else if keys.just_pressed(KeyCode::KeyD) {
        Some((0, 1))
    } else if keys.just_pressed(KeyCode::KeyW) {
        Some((1, 0))
    } else if keys.just_pressed(KeyCode::KeyS) {
        Some((-1, 0))
    } else {
        None
    };
    if let Some((d_row, d_col)) = cursor_move {
        move_cursor(&mut duel, d_row, d_col);
    }
    if keys.just_pressed(KeyCode::KeyE)
        && let Some(object) = duel.hovered
    {
        activate_card(&mut duel, object);
        return;
    }

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

/// The selectable cards as a row grid: hand at the bottom, then each
/// seat's lanes from the local seat outward. Row order matches the
/// visual layout, so W/S moves the way the eye expects.
fn cursor_grid(duel: &Duel) -> Vec<Vec<ObjectId>> {
    let Some(board) = duel.board.as_ref() else {
        return Vec::new();
    };
    let mut rows: Vec<Vec<ObjectId>> = Vec::new();
    let hand: Vec<ObjectId> = board.hand.iter().map(|c| c.id).collect();
    if !hand.is_empty() {
        rows.push(hand);
    }
    for pod in board
        .pods
        .iter()
        .filter(|p| p.is_local)
        .chain(board.pods.iter().filter(|p| !p.is_local))
    {
        for lane in &pod.lanes {
            let row: Vec<ObjectId> = lane.groups.iter().map(|g| g.representative).collect();
            if !row.is_empty() {
                rows.push(row);
            }
        }
    }
    rows
}

/// Moves the card cursor; wraps inside a row and clamps the column when
/// changing rows. With no cursor yet, starts at the first hand card.
fn move_cursor(duel: &mut Duel, d_row: i32, d_col: i32) {
    let grid = cursor_grid(duel);
    if grid.is_empty() {
        return;
    }
    let Some(current) = duel.hovered else {
        duel.hovered = Some(grid[0][0]);
        return;
    };
    let Some((mut row, mut col)) = grid.iter().enumerate().find_map(|(r, row)| {
        row.iter()
            .position(|&id| id == current)
            .map(|c| (r as i32, c as i32))
    }) else {
        duel.hovered = Some(grid[0][0]);
        return;
    };
    if d_col != 0 {
        let len = grid[row as usize].len() as i32;
        col = (col + d_col).rem_euclid(len);
    }
    if d_row != 0 {
        row = (row + d_row).rem_euclid(grid.len() as i32);
        col = col.min(grid[row as usize].len() as i32 - 1);
    }
    duel.hovered = Some(grid[row as usize][col as usize]);
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
    hand_cards: Query<&HandCardVisual>,
    duel: ResMut<Duel>,
) {
    handle_clicks(clicks, &cards, &hand_cards, duel);
}

fn handle_clicks(
    mut clicks: MessageReader<Pointer<Click>>,
    cards: &Query<&CardVisual>,
    hand_cards: &Query<&HandCardVisual>,
    mut duel: ResMut<Duel>,
) {
    for click in clicks.read() {
        let object = cards
            .get(click.entity)
            .map(|v| v.object)
            .ok()
            .or_else(|| hand_cards.get(click.entity).map(|h| h.object).ok());
        let Some(object) = object else {
            continue;
        };
        activate_card(&mut duel, object);
    }
}

/// Tracks the card under the pointer — the same cursor the WASD keys
/// move, so mouse and keyboard never fight over two highlights.
pub fn pointer_hover(
    mut overs: MessageReader<Pointer<Over>>,
    mut outs: MessageReader<Pointer<Out>>,
    cards: Query<&CardVisual>,
    hand_cards: Query<&HandCardVisual>,
    mut duel: ResMut<Duel>,
) {
    for over in overs.read() {
        if let Ok(v) = cards.get(over.entity) {
            duel.hovered = Some(v.object);
        } else if let Ok(h) = hand_cards.get(over.entity) {
            duel.hovered = Some(h.object);
        }
    }
    for out in outs.read() {
        if cards
            .get(out.entity)
            .is_ok_and(|v| duel.hovered == Some(v.object))
            || hand_cards
                .get(out.entity)
                .is_ok_and(|h| duel.hovered == Some(h.object))
        {
            duel.hovered = None;
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
