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
//!
//! `Space` is "the click", with a fixed precedence: the card under the
//! cursor, then the selected phase button, then confirm/pass.

use crate::Duel;
use crate::hud::{
    HandCardVisual, MenuAction, MenuButton, OverlayKnob, PhaseButton, PlayerTab, PreviewResize,
    RailButton,
};
use crate::settings::ClientSettings;
use crate::table::CardVisual;
use baylee_client_core::automation::AutoPilot;
use baylee_client_core::interaction::Interaction;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::PlayerAction;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

/// Finds a component on the clicked entity or one of its ancestors —
/// a click on a button's icon or text belongs to the button.
fn find_in_lineage<'a, T: Component>(
    entity: Entity,
    query: &'a Query<&T>,
    parents: &Query<&ChildOf>,
) -> Option<&'a T> {
    let mut current = Some(entity);
    for _ in 0..6 {
        let e = current?;
        if let Ok(found) = query.get(e) {
            return Some(found);
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}

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
#[allow(clippy::too_many_lines)] // one key map, kept in one place on purpose
pub fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut duel: ResMut<Duel>,
    mut rig: ResMut<crate::table::CameraRig>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // ---- seat inspection: Shift+1..9 focuses the nth opponent ----------
    let digits = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    if shift {
        for (i, key) in digits.iter().enumerate() {
            if keys.just_pressed(*key) {
                focus_nth_opponent(&mut duel, &mut rig, i);
            }
        }
    }

    // ---- phase rail: Shift+W/S selects, Space toggles ------------------
    if shift && keys.just_pressed(KeyCode::KeyW) {
        duel.orders.move_selection(-1);
    }
    if shift && keys.just_pressed(KeyCode::KeyS) {
        duel.orders.move_selection(1);
    }

    // ---- TAB: fast-forward to the next phase ----------------------------
    if keys.just_pressed(KeyCode::Tab)
        && let Some(view) = duel.view.as_ref()
    {
        duel.autopilot = Some(AutoPilot::ToNextPhase { from: view.phase });
    }

    // ---- X: slide the own-board overlay down/up --------------------------
    if keys.just_pressed(KeyCode::KeyX) {
        duel.overlay_closed = !duel.overlay_closed;
    }

    // ---- WASD moves the card cursor, E activates it ---------------------
    if !shift {
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
    }
    if keys.just_pressed(KeyCode::KeyE)
        && let Some(object) = duel.hovered
    {
        activate_card(&mut duel, object);
        return;
    }

    // ---- Space is "the click": card, then phase toggle, then pass -------
    if keys.just_pressed(KeyCode::Space) {
        if let Some(object) = duel.hovered {
            activate_card(&mut duel, object);
            return;
        }
        if let Some((side, row)) = duel.orders.selected() {
            duel.orders.toggle(side, row);
            return;
        }
        if let Some(action) = duel.interaction.as_ref().and_then(Interaction::confirm) {
            duel.submit(action);
            return;
        }
    }

    if duel.interaction.is_none() {
        return;
    }

    // Confirm / pass priority (Enter never toggles anything else).
    if keys.just_pressed(KeyCode::Enter)
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

    // Cancel: an open preview first, then a selected phase button, then a
    // half-built selection.
    if keys.just_pressed(KeyCode::Escape) {
        if duel.hovered.is_some() {
            duel.hovered = None;
        } else if duel.orders.selected().is_some() {
            duel.orders.clear_selection();
        } else if let Some(i) = duel.interaction.as_mut() {
            i.cancel();
        }
    }
}

/// Drags on the preview's resize handle, and the resize shortcut
/// (Command/Alt + Shift + Up/Down). The size is persisted.
pub fn preview_resize(
    keys: Res<ButtonInput<KeyCode>>,
    mut downs: MessageReader<Pointer<Press>>,
    mut ups: MessageReader<Pointer<Release>>,
    resize: Query<&PreviewResize>,
    mut motions: MessageReader<MouseMotion>,
    mut duel: ResMut<Duel>,
    mut settings: ResMut<ClientSettings>,
) {
    for down in downs.read() {
        if resize.get(down.entity).is_ok() {
            duel.resize_drag = true;
        }
    }
    let mut ended = false;
    for _up in ups.read() {
        ended |= duel.resize_drag;
        duel.resize_drag = false;
    }
    if duel.resize_drag {
        let dx: f32 = motions.read().map(|m| m.delta.x).sum();
        if dx != 0.0 {
            settings.preview_scale = (settings.preview_scale + dx * 0.004).clamp(0.5, 1.75);
        }
    } else {
        motions.clear();
    }
    if ended {
        settings.save();
    }

    // Command/Alt + Shift + Up/Down resizes too.
    let meta = keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight)
        || keys.pressed(KeyCode::AltLeft)
        || keys.pressed(KeyCode::AltRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if meta && shift {
        if keys.just_pressed(KeyCode::ArrowUp) {
            settings.preview_scale = (settings.preview_scale + 0.05).clamp(0.5, 1.75);
            settings.save();
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            settings.preview_scale = (settings.preview_scale - 0.05).clamp(0.5, 1.75);
            settings.save();
        }
    }
}

/// Focuses the nth opponent's board (or toggles back to your own).
fn focus_nth_opponent(duel: &mut Duel, rig: &mut crate::table::CameraRig, index: usize) {
    let Some(board) = duel.board.as_ref() else {
        return;
    };
    let opponents: Vec<_> = board
        .pods
        .iter()
        .filter(|p| !p.is_local)
        .map(|p| p.player)
        .collect();
    let Some(&player) = opponents.get(index) else {
        return;
    };
    if duel.focus == Some(player) {
        navigate_home(duel, rig);
    } else {
        navigate_to_player(duel, rig, player);
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

/// Pointer handling: clicking a card, a player tab, or a rail button.
///
/// A click means "this object", and what that does depends entirely on the
/// pending choice: it selects a target, declares an attacker, or plays a card.
/// Resolving that here rather than in the renderer keeps one place where a
/// click becomes an action.
#[allow(clippy::too_many_arguments)] // one query per clickable widget kind
pub fn pointer(
    mut clicks: MessageReader<Pointer<Click>>,
    cards: Query<&CardVisual>,
    hand_cards: Query<&HandCardVisual>,
    tabs: Query<&PlayerTab>,
    phase_buttons: Query<&PhaseButton>,
    rail_buttons: Query<&RailButton>,
    menu_buttons: Query<&MenuButton>,
    knobs: Query<&OverlayKnob>,
    parents: Query<&ChildOf>,
    mut duel: ResMut<Duel>,
    mut rig: ResMut<crate::table::CameraRig>,
) {
    for click in clicks.read() {
        let e = click.entity;
        if let Some(object) = find_in_lineage(e, &cards, &parents)
            .map(|v| v.object)
            .or_else(|| find_in_lineage(e, &hand_cards, &parents).map(|h| h.object))
        {
            activate_card(&mut duel, object);
            continue;
        }
        if let Some(tab) = find_in_lineage(e, &tabs, &parents) {
            // Your own tab (or the already-focused one) brings the camera
            // home; any other opponent's tab frames their pod.
            if duel.seat() == Some(tab.player) || duel.focus == Some(tab.player) {
                navigate_home(&mut duel, &mut rig);
            } else {
                navigate_to_player(&mut duel, &mut rig, tab.player);
            }
            continue;
        }
        if let Some(button) = find_in_lineage(e, &phase_buttons, &parents) {
            duel.orders.toggle(button.side, button.row);
            continue;
        }
        if find_in_lineage(e, &knobs, &parents).is_some() {
            duel.overlay_closed = !duel.overlay_closed;
            continue;
        }
        if let Some(button) = find_in_lineage(e, &menu_buttons, &parents) {
            match button.action {
                MenuAction::Concede => duel.submit(PlayerAction::Concede),
                // Draw offers need mutual agreement — a protocol item.
                MenuAction::OfferDraw => {}
            }
            continue;
        }
        if let Some(button) = find_in_lineage(e, &rail_buttons, &parents) {
            let Some(view) = duel.view.as_ref() else {
                continue;
            };
            duel.autopilot = Some(match button {
                RailButton::NextPhase => AutoPilot::ToNextPhase { from: view.phase },
                RailButton::EndTurn => AutoPilot::ToNextTurn {
                    from_turn: view.turn,
                },
            });
            continue;
        }
        // A click on nothing interactive closes the card preview.
        duel.hovered = None;
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

/// The battlefield canvas camera: arrows pan, Shift+Up/Down zooms,
/// Shift+Left/Right rotates, left-drag pans, right-drag rotates, the
/// wheel zooms (over the hand bar it scrolls the hand instead), and the
/// touch gestures do what fingers do (pan/pinch/rotate).
#[allow(clippy::too_many_arguments)]
pub fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut motions: MessageReader<bevy::input::mouse::MouseMotion>,
    mut wheels: MessageReader<MouseWheel>,
    mut pans: MessageReader<bevy::input::gestures::PanGesture>,
    mut pinches: MessageReader<bevy::input::gestures::PinchGesture>,
    mut rotates: MessageReader<bevy::input::gestures::RotationGesture>,
    windows: Query<&Window>,
    mut duel: ResMut<Duel>,
    mut rig: ResMut<crate::table::CameraRig>,
) {
    // The canvas is not navigable while the own-board battlefield covers
    // it (the "battlefield bar" is in front).
    if !duel.overlay_closed {
        return;
    }
    let meta = keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight)
        || keys.pressed(KeyCode::AltLeft)
        || keys.pressed(KeyCode::AltRight);
    let shift = (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)) && !meta;

    // Screen-relative directions on the table plane.
    let right = Vec2::new(rig.yaw.cos(), -rig.yaw.sin());
    let forward = Vec2::new(-rig.yaw.sin(), -rig.yaw.cos());

    // ---- keyboard: pan / zoom / rotate ----------------------------------
    let pan_step = rig.distance * 0.02;
    if shift {
        if keys.pressed(KeyCode::ArrowUp) {
            rig.distance = (rig.distance * 0.985).max(crate::table::CameraRig::MIN_DISTANCE);
        }
        if keys.pressed(KeyCode::ArrowDown) {
            rig.distance = (rig.distance * 1.015).min(crate::table::CameraRig::MAX_DISTANCE);
        }
        if keys.pressed(KeyCode::ArrowLeft) {
            rig.yaw += 0.015;
        }
        if keys.pressed(KeyCode::ArrowRight) {
            rig.yaw -= 0.015;
        }
    } else {
        if keys.pressed(KeyCode::ArrowLeft) {
            rig.target -= right * pan_step;
        }
        if keys.pressed(KeyCode::ArrowRight) {
            rig.target += right * pan_step;
        }
        if keys.pressed(KeyCode::ArrowUp) {
            rig.target += forward * pan_step;
        }
        if keys.pressed(KeyCode::ArrowDown) {
            rig.target -= forward * pan_step;
        }
    }

    // ---- mouse: left-drag pans, right-drag rotates -----------------------
    let (mut dx, mut dy) = (0.0, 0.0);
    for motion in motions.read() {
        dx += motion.delta.x;
        dy += motion.delta.y;
    }
    let drag_scale = rig.distance / 600.0;
    if buttons.pressed(MouseButton::Left) && !duel.resize_drag {
        rig.target -= right * dx * drag_scale;
        rig.target -= forward * dy * drag_scale;
    }
    if buttons.pressed(MouseButton::Right) {
        rig.yaw -= dx * 0.004;
    }

    // ---- wheel: zoom, unless the pointer is over the hand bar ------------
    let over_hand = windows.single().ok().and_then(|w| {
        w.cursor_position()
            .map(|p| p.y > w.height() - (crate::hud::HAND_CARD_H + 20.0))
    }) == Some(true);
    for wheel in wheels.read() {
        if over_hand {
            duel.hand_scroll = (duel.hand_scroll - wheel.y * 60.0).max(0.0);
        } else {
            rig.distance = (rig.distance * (1.0 - wheel.y * 0.08)).clamp(
                crate::table::CameraRig::MIN_DISTANCE,
                crate::table::CameraRig::MAX_DISTANCE,
            );
        }
    }

    // ---- touch gestures ---------------------------------------------------
    for pan in pans.read() {
        rig.target -= right * pan.0.x * drag_scale;
        rig.target -= forward * pan.0.y * drag_scale;
    }
    for pinch in pinches.read() {
        rig.distance = (rig.distance / (1.0 + pinch.0 * 0.5)).clamp(
            crate::table::CameraRig::MIN_DISTANCE,
            crate::table::CameraRig::MAX_DISTANCE,
        );
    }
    for rotate in rotates.read() {
        rig.yaw += rotate.0;
    }
}

/// Navigates the camera to a seat's pod, framing it in the free canvas
/// area (clear of the own-board overlay), cards upright. Also marks the
/// seat as the layout's focus so its pod is enlarged.
pub fn navigate_to_player(
    duel: &mut Duel,
    rig: &mut crate::table::CameraRig,
    player: baylee_core::ids::PlayerId,
) {
    let Some(slot) = duel.layout.as_ref().and_then(|l| l.slot(player).copied()) else {
        return;
    };
    let world = Vec2::new(slot.center.x, -slot.center.y);
    *rig = crate::table::CameraRig::framing(&slot, world);
    duel.focus = Some(player);
    crate::rebuild_board(duel);
}

/// Returns the camera to the default view behind the local seat.
pub fn navigate_home(duel: &mut Duel, rig: &mut crate::table::CameraRig) {
    *rig = crate::table::CameraRig::default();
    duel.focus = None;
    crate::rebuild_board(duel);
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
