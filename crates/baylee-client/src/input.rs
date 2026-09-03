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
//! Which key does what is the account's, not this module's: every binding
//! comes from `Keymap` and is resolved through `crate::keys`. The primary key
//! (`Enter` by default) is "the click", with a fixed precedence: the card
//! under the cursor, then the selected phase button, then confirm/pass —
//! which is why it is not the pass key: `Space` is `Confirm` and passes
//! whatever the cursor happens to be resting on.

use crate::Duel;
use crate::hud::{
    AbilityButton, ChoiceButton, HandCardVisual, MenuAction, MenuButton, OverlayKnob, PhaseButton,
    PileChip, PlayerTab, PreviewResize, PromptAction, PromptButton, RailButton, TrayCard,
    TrayClose, TrayTab,
};
use crate::keys::Fired;
use crate::settings::ClientSettings;
use crate::table::CardVisual;
use baylee_client_core::automation::AutoPilot;
use baylee_client_core::interaction::{Interaction, Prompt, SelectionOutcome};
use baylee_client_core::prefs::Action;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::PlayerAction;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

/// Every widget the zone browser puts on screen, as one system parameter.
///
/// Bundled rather than four more arguments because `pointer` already sits at
/// Bevy's parameter limit — and because they are one thing: the tray, its
/// tabs, its close button, and the pile chips that open it.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TrayWidgets<'w, 's> {
    cards: Query<'w, 's, &'static TrayCard>,
    tabs: Query<'w, 's, &'static TrayTab>,
    close: Query<'w, 's, &'static TrayClose>,
    chips: Query<'w, 's, &'static PileChip>,
}

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
///
/// Between those two there is now a third answer. A spell the engine has not
/// offered because the mana is not floating yet is not a card with nothing to
/// do — it is a card that wants two lands tapped first, which is what a player
/// at a table would do without thinking about it. [`crate::mana_for`] works
/// out which lands; the run in [`crate::ManaRun`] taps them and then casts.
pub fn activate_card(duel: &mut Duel, object: ObjectId) {
    if let Some(action) = duel.interaction.as_ref().and_then(|i| i.play_card(object)) {
        duel.submit(action);
        return;
    }
    if duel.reachable.contains(&object)
        && let Some(plan) = crate::mana_for(duel, object)
    {
        duel.last_error = None;
        duel.mana_run = Some(crate::ManaRun::new(plan, object));
        return;
    }
    // A permanent with something to do does it. One ability goes straight
    // through — a menu of one only ever wastes a tap — and several open the
    // chooser in the prompt bar, because "ability 2" is not a thing a player
    // should have to count out on a card.
    if let Some(options) = abilities_of(duel, object) {
        match options.len() {
            0 => {}
            1 => {
                duel.ability_menu = None;
                duel.submit(options[0].action.clone());
                return;
            }
            _ => {
                duel.ability_menu = Some(object);
                duel.ability_pick = 0;
                return;
            }
        }
    }
    duel.ability_menu = None;
    if let Some(i) = duel.interaction.as_mut() {
        i.toggle(object);
    }
}

/// What `object` is offering, if anything.
/// English deliberately: only the `action` on each option is read here —
/// what is drawn is [`crate::hud`]'s business, and this path picks an ability
/// by position or takes the only one there is.
fn abilities_of(duel: &Duel, object: ObjectId) -> Option<Vec<crate::abilities::AbilityOption>> {
    let view = duel.view.as_ref()?;
    let interaction = duel.interaction.as_ref()?;
    Some(crate::abilities::options(
        baylee_client_core::Lang::En,
        view,
        interaction,
        object,
    ))
}

/// Keyboard handling: every key comes from the account's keymap.
///
/// The handler asks *actions*, never keys. That is what makes rebinding work
/// at all, and it also removed the `if !shift` guards that used to be sprayed
/// through here — `W` and `⇧W` are two chords, and telling them apart is the
/// keymap's job, not this function's.
pub fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    mut duel: ResMut<Duel>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut rig: ResMut<crate::table::CameraRig>,
    mut settings: ResMut<crate::settings::ClientSettings>,
) {
    let fired = Fired::of(&keys, prefs.keymap());
    // Before the quiet check, and not after it: a letter typed into the type
    // filter is usually bound to no action at all, so `Fired` is empty for
    // exactly the keys the box cares about most.
    if subtype_keys(fired, &mut typed, &mut duel) {
        return;
    }
    if fired.quiet() {
        return;
    }
    look_around(fired, &mut duel, &mut rig, &mut settings, &mut prefs);
    // The ability menu owns the keyboard while it stands: a list of things to
    // do is not a background for the cursor to walk over.
    if ability_menu_keys(fired, &mut duel) {
        return;
    }
    if move_the_cursor(fired, &mut duel) {
        return;
    }
    if aim_and_declare(fired, &mut duel) {
        return;
    }
    if the_click(fired, &mut duel, &mut prefs) {
        return;
    }
    if duel.interaction.is_some() {
        answer_the_question(fired, &mut duel, &mut prefs);
    }
}

/// Camera, phase rail, fast-forward and display toggles — everything that
/// changes what the player sees rather than what the game hears.
fn look_around(
    fired: Fired,
    duel: &mut Duel,
    rig: &mut crate::table::CameraRig,
    settings: &mut crate::settings::ClientSettings,
    prefs: &mut crate::prefs::Prefs,
) {
    if fired.has(Action::FocusNextSeat) {
        focus_next_opponent(duel, rig);
    }
    if fired.has(Action::FocusHome) {
        navigate_home(duel, rig);
    }
    // The rail: move the highlight here, toggle it with the primary key.
    if fired.has(Action::RailUp) {
        prefs.rail_cursor().move_selection(-1);
    }
    if fired.has(Action::RailDown) {
        prefs.rail_cursor().move_selection(1);
    }
    if let Some((phase, turn)) = duel.view.as_ref().map(|v| (v.phase, v.turn)) {
        if fired.has(Action::NextPhase) {
            duel.autopilot = Some(AutoPilot::ToNextPhase { from: phase });
        }
        if fired.has(Action::NextTurn) {
            duel.autopilot = Some(AutoPilot::ToNextTurn { from_turn: turn });
        }
    }
    if fired.has(Action::ToggleOverlay) {
        duel.overlay_open = !duel.overlay_open;
    }
    if fired.has(Action::ToggleTextView) {
        // The modifier key shows the card face while held; this is the latch,
        // for players who read text rather than art. A preference, not a mode,
        // so it is remembered.
        settings.prefer_text_view = !settings.prefer_text_view;
        settings.save();
    }
    if fired.has(Action::ToggleBrowser) {
        // A latch rather than a held key, for the same reason the pile chips
        // are buttons: reading a graveyard is not a glance, and a held key is
        // not a gesture a phone has. The tab it was last left on is kept, so
        // a player checking their own yard twice does not re-pick it.
        if duel.browser.is_open() {
            duel.browser.close();
        } else {
            duel.browser.open();
        }
    }
}

/// The creature types still on screen, in the engine's order.
fn visible_types(duel: &Duel) -> Vec<crate::choices::ChoiceOption> {
    duel.interaction
        .as_ref()
        .map(Interaction::prompt)
        .and_then(|p| {
            crate::choices::options(
                &p,
                baylee_client_core::Lang::En,
                duel.statics.as_ref(),
                &duel.subtype_filter,
            )
        })
        .unwrap_or_default()
}

/// Typing into the creature-type filter, and walking what it leaves.
///
/// While the box is up the keymap is swallowed whole, because letters *are*
/// chords: `W` walks the cursor and `E` activates a card, and a player
/// spelling "Elemental" would otherwise play half their turn. Only the keys
/// that mean something to a list survive — the cursor walks the rows, Confirm
/// takes the highlighted one, Cancel empties the box.
///
/// Returns whether it consumed the frame.
fn subtype_keys(fired: Fired, typed: &mut MessageReader<KeyboardInput>, duel: &mut Duel) -> bool {
    if !matches!(
        duel.interaction.as_ref().map(Interaction::prompt),
        Some(Prompt::ChooseSubtype { .. })
    ) {
        return false;
    }
    let before = duel.subtype_filter.clone();
    for event in typed.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Character(s) => duel
                .subtype_filter
                .extend(s.chars().filter(|c| !c.is_control())),
            Key::Backspace => {
                duel.subtype_filter.pop();
            }
            _ => {}
        }
    }
    let rows = visible_types(duel);
    if duel.subtype_filter != before {
        // The highlight follows the list. A row that has just been filtered
        // away must not stay picked, or Confirm answers a type the player can
        // no longer see.
        if let Some(first) = rows.first().map(|row| row.index)
            && let Some(i) = duel.interaction.as_mut()
        {
            i.choose_index(first);
        }
        return true;
    }
    if fired.has(Action::Cancel) {
        duel.subtype_filter.clear();
        return true;
    }
    let step = i32::from(fired.has(Action::CursorDown)) - i32::from(fired.has(Action::CursorUp))
        + i32::from(fired.has(Action::CursorRight))
        - i32::from(fired.has(Action::CursorLeft));
    let picked = duel
        .interaction
        .as_ref()
        .and_then(Interaction::chosen_index);
    if step != 0 && !rows.is_empty() {
        let at = picked
            .and_then(|p| rows.iter().position(|row| row.index == p))
            .and_then(|p| i32::try_from(p).ok())
            .unwrap_or(0);
        let len = i32::try_from(rows.len()).unwrap_or(1);
        let next = usize::try_from((at + step).rem_euclid(len)).unwrap_or(0);
        if let Some(row) = rows.get(next)
            && let Some(i) = duel.interaction.as_mut()
        {
            i.choose_index(row.index);
        }
        return true;
    }
    if (fired.has(Action::Confirm) || fired.has(Action::Primary))
        // Only a row that is still on screen: the filter may have moved on
        // since the highlight was set.
        && picked.is_some_and(|p| rows.iter().any(|row| row.index == p))
        && let Some(action) = duel.interaction.as_ref().and_then(Interaction::confirm)
    {
        duel.submit(action);
    }
    true
}

/// The open ability chooser: the cursor keys walk it, the primary key or
/// confirm takes the entry, cancel puts it away. Returns whether it consumed
/// the frame.
///
/// The list is rebuilt from `LegalActions` here rather than trusted from the
/// frame it was drawn on — the same rule the pointer path follows, and for
/// the same reason: the engine may have withdrawn the ability since.
pub fn ability_menu_keys(fired: Fired, duel: &mut Duel) -> bool {
    let Some(object) = duel.ability_menu else {
        return false;
    };
    let Some(options) = abilities_of(duel, object).filter(|o| o.len() > 1) else {
        // Nothing left to choose: the menu is stale, and holding it open
        // would keep the keyboard hostage.
        duel.ability_menu = None;
        return false;
    };
    if fired.has(Action::Cancel) {
        duel.ability_menu = None;
        return true;
    }
    let step = i32::from(fired.has(Action::CursorDown)) - i32::from(fired.has(Action::CursorUp))
        + i32::from(fired.has(Action::CursorRight))
        - i32::from(fired.has(Action::CursorLeft));
    if step != 0 {
        let len = i32::try_from(options.len()).unwrap_or(1);
        let next = i32::try_from(duel.ability_pick).unwrap_or(0) + step;
        duel.ability_pick = usize::try_from(next.rem_euclid(len)).unwrap_or(0);
        return true;
    }
    if fired.has(Action::Primary) || fired.has(Action::Confirm) || fired.has(Action::ActivateCard) {
        let action = options.get(duel.ability_pick).map(|o| o.action.clone());
        duel.ability_menu = None;
        if let Some(action) = action {
            duel.submit(action);
        }
        return true;
    }
    false
}

/// The card cursor, and the key that acts on what it is over. Returns whether
/// it consumed the frame.
fn move_the_cursor(fired: Fired, duel: &mut Duel) -> bool {
    for (action, (d_row, d_col)) in [
        (Action::CursorUp, (1, 0)),
        (Action::CursorDown, (-1, 0)),
        (Action::CursorLeft, (0, -1)),
        (Action::CursorRight, (0, 1)),
    ] {
        if fired.has(action) {
            move_cursor(duel, d_row, d_col);
        }
    }
    if fired.has(Action::ActivateCard)
        && let Some(object) = duel.hovered
    {
        activate_card(duel, object);
        return true;
    }
    false
}

/// Combat: where the next declaration points, and the answer that declares
/// nothing. Returns whether it consumed the frame.
fn aim_and_declare(fired: Fired, duel: &mut Duel) -> bool {
    let step = i32::from(fired.has(Action::CombatFocusNext))
        - i32::from(fired.has(Action::CombatFocusPrev));
    if step != 0 {
        cycle_combat_focus(duel, step);
    }
    if fired.has(Action::CombatNone) {
        declare_nothing(duel);
        return true;
    }
    false
}

/// The primary key, with its fixed precedence: the card under the cursor,
/// then the selected phase button, then confirm. Returns whether it consumed
/// the frame.
fn the_click(fired: Fired, duel: &mut Duel, prefs: &mut crate::prefs::Prefs) -> bool {
    if !fired.has(Action::Primary) {
        return false;
    }
    if let Some(object) = duel.hovered {
        activate_card(duel, object);
        return true;
    }
    if let Some((side, row)) = prefs.orders().selected() {
        prefs.edit().orders.toggle(side, row);
        return true;
    }
    if let Some(action) = duel.interaction.as_ref().and_then(Interaction::confirm) {
        duel.submit(action);
        return true;
    }
    false
}

/// Every straight answer to a pending choice.
///
/// Each goes through the interaction, which refuses it unless the engine
/// actually asked — so a key bound to "yes" does nothing at all during
/// combat, without this function knowing what combat is.
fn answer_the_question(fired: Fired, duel: &mut Duel, prefs: &mut crate::prefs::Prefs) {
    // Confirm / pass priority. Never toggles anything else, so it is the one
    // key that always means "I am done here".
    if fired.has(Action::Confirm)
        && let Some(action) = duel.interaction.as_ref().and_then(Interaction::confirm)
    {
        duel.submit(action);
        return;
    }
    for (action, answer) in [(Action::MulliganKeep, true), (Action::MulliganTake, false)] {
        if fired.has(action)
            && let Some(sent) = duel
                .interaction
                .as_ref()
                .and_then(|i| i.answer_mulligan(answer))
        {
            duel.submit(sent);
            return;
        }
    }
    for (action, answer) in [(Action::AnswerYes, true), (Action::AnswerNo, false)] {
        if fired.has(action)
            && let Some(sent) = duel
                .interaction
                .as_ref()
                .and_then(|i| i.answer_yes_no(answer))
        {
            duel.submit(sent);
            return;
        }
    }

    // Number choices step, and the interaction clamps the value to the
    // offered range — a player can hold a key without producing something the
    // engine would reject.
    let step = i32::from(fired.has(Action::NumberUp)) - i32::from(fired.has(Action::NumberDown));
    if step != 0
        && let Some(i) = duel.interaction.as_mut()
    {
        let next = if step > 0 {
            i.number().saturating_add(1)
        } else {
            i.number().saturating_sub(1)
        };
        i.set_number(next);
    }

    // Cancel: an open preview first, then a selected phase button, then a
    // half-built answer.
    if fired.has(Action::Cancel) {
        if duel.hovered.is_some() {
            duel.hovered = None;
        } else if prefs.orders().selected().is_some() {
            prefs.rail_cursor().clear_selection();
        } else if let Some(i) = duel.interaction.as_mut() {
            i.cancel();
        }
    }
}

/// Aims the next declaration at the next defender (or attacker).
fn cycle_combat_focus(duel: &mut Duel, delta: i32) {
    if let Some(i) = duel.interaction.as_mut() {
        i.cycle_focus(delta);
    }
}

/// Declares nothing and moves on — no attackers, or no blockers.
///
/// Routed through the interaction rather than sent as an empty action
/// directly, so an empty declaration is validated exactly like a full one and
/// the key does nothing at all outside combat.
fn declare_nothing(duel: &mut Duel) {
    let Some(i) = duel.interaction.as_mut() else {
        return;
    };
    if !i.is_combat() {
        return;
    }
    i.cancel();
    if let Some(action) = i.confirm() {
        duel.submit(action);
    }
}

/// Drags on the preview's resize handle, and the resize shortcut
/// (Command/Alt + Shift + Up/Down). The size is persisted.
#[allow(clippy::too_many_arguments)] // events + queries + state, all needed
pub fn preview_resize(
    keys: Res<ButtonInput<KeyCode>>,
    mut downs: MessageReader<Pointer<Press>>,
    mut ups: MessageReader<Pointer<Release>>,
    resize: Query<&PreviewResize>,
    parents: Query<&ChildOf>,
    mut motions: MessageReader<MouseMotion>,
    mut duel: ResMut<Duel>,
    mut settings: ResMut<ClientSettings>,
) {
    for down in downs.read() {
        if find_in_lineage(down.entity, &resize, &parents).is_some() {
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

/// Frames the next opponent's board, wrapping back to your own.
///
/// One key that walks the table rather than a numbered key per chair: a
/// four-seat game has three opponents, and a binding screen listing nine
/// "focus seat N" rows would be listing six that can never fire.
fn focus_next_opponent(duel: &mut Duel, rig: &mut crate::table::CameraRig) {
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
    let next = match duel.focus {
        None => Some(opponents[0]),
        Some(current) => opponents
            .iter()
            .position(|p| *p == current)
            .and_then(|i| opponents.get(i + 1).copied()),
    };
    match next {
        Some(player) => navigate_to_player(duel, rig, player),
        // Past the last opponent is home again, so the key never dead-ends.
        None => navigate_home(duel, rig),
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

/// Sends the ability one row of the ability chooser stands for.
///
/// Rebuilt from `LegalActions` here rather than trusted from the bar: a
/// chooser drawn a frame ago must not be able to send an ability the engine
/// has since stopped offering.
fn pick_ability(duel: &mut Duel, index: usize) {
    let action = duel
        .ability_menu
        .and_then(|object| abilities_of(duel, object))
        .and_then(|options| options.get(index).cloned())
        .map(|option| option.action);
    duel.ability_menu = None;
    if let Some(action) = action {
        duel.submit(action);
    }
}

/// Answers an indexed choice: a colour, a seat, one of several ways to cast.
///
/// It answers on the click that picks it -- there is no second "OK", because
/// there is nothing to combine. The rows are rebuilt from the *current*
/// prompt first, so a button drawn before the engine moved on answers nothing
/// rather than the wrong thing.
fn pick_choice(duel: &mut Duel, index: usize) {
    let offered = duel
        .interaction
        .as_ref()
        .map(baylee_client_core::Interaction::prompt)
        // The language is irrelevant here and deliberately not plumbed: only
        // the *shape* of the answer is read back -- whether this prompt is an
        // indexed choice at all, and how many rows it has. The labels are the
        // renderer's business.
        .and_then(|p| {
            crate::choices::options(
                &p,
                baylee_client_core::Lang::En,
                duel.statics.as_ref(),
                &duel.subtype_filter,
            )
        })
        // Not `index < rows.len()`: a filtered list's rows carry the
        // engine's own indices, and most of them are not on screen.
        .is_some_and(|rows| rows.iter().any(|row| row.index == index));
    if !offered {
        return;
    }
    let action = duel
        .interaction
        .as_mut()
        .and_then(|i| i.choose_index(index).then(|| i.confirm())?);
    if let Some(action) = action {
        duel.submit(action);
    }
}

/// A click on the zone browser or on one of the chips that opens it.
///
/// Its own function rather than four more arms in [`pointer`]: they are one
/// widget, and the browser is meant to be a second *place* to click a card,
/// not a second way to answer a choice — which is why a tray card goes
/// through the same [`activate_card`] a card on the table does.
///
/// Returns whether the click belonged to the browser.
fn browser_click(
    duel: &mut Duel,
    entity: Entity,
    tray: &TrayWidgets,
    parents: &Query<&ChildOf>,
) -> bool {
    if let Some(card) = find_in_lineage(entity, &tray.cards, parents) {
        activate_card(duel, card.object);
        return true;
    }
    // A tab inside the open tray switches zone; a chip outside it opens and
    // closes the whole panel. Two different jobs, so two components.
    if let Some(tab) = find_in_lineage(entity, &tray.tabs, parents) {
        duel.browser.show(tab.zone);
        return true;
    }
    if find_in_lineage(entity, &tray.close, parents).is_some() {
        duel.browser.close();
        return true;
    }
    if let Some(chip) = find_in_lineage(entity, &tray.chips, parents) {
        let zone = chip.zone;
        // A second click on the pile already showing puts it away, so a chip
        // is a toggle rather than a one-way door.
        if duel.browser.is_open() && duel.browser.tab() == zone {
            duel.browser.close();
        } else {
            duel.browser.open();
            duel.browser.show(zone);
        }
        return true;
    }
    false
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
    prompt_buttons: Query<&PromptButton>,
    ability_buttons: Query<&AbilityButton>,
    choice_buttons: Query<&ChoiceButton>,
    tray: TrayWidgets,
    parents: Query<&ChildOf>,
    mut duel: ResMut<Duel>,
    mut prefs: ResMut<crate::prefs::Prefs>,
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
            // A seat is a legal target of what is being cast ("any target",
            // CR 115.4), so the tab is how a player points at a face. It only
            // stops being a camera control while that is true.
            if let Some(i) = duel.interaction.as_mut()
                && i.toggle_player(tab.player) != SelectionOutcome::Rejected
            {
                continue;
            }
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
            prefs.edit().orders.toggle(button.side, button.row);
            continue;
        }
        if find_in_lineage(e, &knobs, &parents).is_some() {
            duel.overlay_open = !duel.overlay_open;
            continue;
        }
        if let Some(button) = find_in_lineage(e, &menu_buttons, &parents) {
            match button.action {
                MenuAction::Concede => duel.submit(PlayerAction::Concede),
                // Draw offers need mutual agreement — a protocol item.
                MenuAction::OfferDraw => duel.submit(PlayerAction::OfferDraw),
            }
            continue;
        }
        if let Some(button) = find_in_lineage(e, &ability_buttons, &parents) {
            pick_ability(&mut duel, button.index);
            continue;
        }
        if let Some(button) = find_in_lineage(e, &choice_buttons, &parents) {
            pick_choice(&mut duel, button.index);
            continue;
        }
        if browser_click(&mut duel, e, &tray, &parents) {
            continue;
        }
        if let Some(button) = find_in_lineage(e, &prompt_buttons, &parents) {
            let action = match button.action {
                PromptAction::Yes => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_yes_no(true)),
                PromptAction::No => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_yes_no(false)),
                PromptAction::Keep => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_mulligan(true)),
                PromptAction::Mulligan => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_mulligan(false)),
                PromptAction::Confirm => duel.interaction.as_ref().and_then(Interaction::confirm),
                // Aiming changes nothing the engine can hear; it moves the
                // focus the next declaration will use.
                PromptAction::AimNext => {
                    cycle_combat_focus(&mut duel, 1);
                    None
                }
                PromptAction::DeclareNothing => {
                    declare_nothing(&mut duel);
                    None
                }
            };
            if let Some(action) = action {
                duel.submit(action);
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
/// Like clicks, hover resolves through the entity's ancestors: the
/// card's image covers the whole card, and the event lands on it first.
///
/// # Why a still pointer says nothing
///
/// `Pointer<Over>` fires when the *card* moves under the pointer just as
/// readily as when the pointer moves over the card, and on this table the
/// cards are always moving: a repacked lane, a tap, a hover lift, a permanent
/// arriving. A pointer resting anywhere near the board therefore re-pinned
/// `hovered` every few frames and the keyboard cursor could not walk at all —
/// twelve presses moved it once, measured at a live table, which broke the
/// keymap's promise that every choice is answerable without a pointer. So the
/// pointer only speaks when it has actually moved. The grace of a few frames
/// covers the gap between a `CursorMoved` and the picking pass that follows
/// it, so a genuine mouse move is never swallowed.
#[allow(clippy::too_many_arguments)] // three picking queries plus the two event streams
pub fn pointer_hover(
    mut overs: MessageReader<Pointer<Over>>,
    mut outs: MessageReader<Pointer<Out>>,
    mut moves: MessageReader<bevy::window::CursorMoved>,
    mut grace: Local<u8>,
    cards: Query<&CardVisual>,
    hand_cards: Query<&HandCardVisual>,
    parents: Query<&ChildOf>,
    mut duel: ResMut<Duel>,
) {
    if moves.read().next().is_some() {
        *grace = 3;
    }
    if *grace == 0 {
        // Drain, so a later real move does not act on a backlog of events
        // the cards generated by sliding around.
        overs.clear();
        outs.clear();
        return;
    }
    *grace -= 1;
    for over in overs.read() {
        if let Some(v) = find_in_lineage(over.entity, &cards, &parents) {
            duel.hovered = Some(v.object);
        } else if let Some(h) = find_in_lineage(over.entity, &hand_cards, &parents) {
            duel.hovered = Some(h.object);
        }
    }
    for out in outs.read() {
        let is_current = find_in_lineage(out.entity, &cards, &parents)
            .is_some_and(|v| duel.hovered == Some(v.object))
            || find_in_lineage(out.entity, &hand_cards, &parents)
                .is_some_and(|h| duel.hovered == Some(h.object));
        if is_current {
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
    if duel.overlay_open {
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
    use baylee_client_core::interaction::Interaction;
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
                player_options: vec![],
                min: 1,
                max: 1,
            },
            PlayerId::new(0),
        );
        i.toggle(obj(99));
        assert!(i.selected().is_empty());
        assert!(!i.can_confirm());
    }

    /// The whole keyboard path, and not just the decision underneath it.
    ///
    /// `confirming_a_priority_choice_passes` asks the `Interaction` directly,
    /// which a live game showed is not enough: a land the engine had offered,
    /// sitting under the cursor, was played by no key and no click, and every
    /// unit test kept passing. So this one presses a `KeyCode` at the real
    /// system and reads the outbox — nothing hand-built in between.
    #[test]
    fn the_primary_key_plays_the_land_under_the_cursor() {
        use baylee_client_core::prefs::Action;
        use bevy::input::ButtonInput;
        use bevy::input::keyboard::KeyboardInput;
        use bevy::prelude::*;

        let duel = crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::Priority {
                    player: PlayerId::new(0),
                    legal: Box::new(LegalActions {
                        can_pass: true,
                        lands: vec![obj(3)],
                        castable: vec![],
                        mana_abilities: vec![],
                        abilities: vec![],
                        suspendable: vec![],
                    }),
                },
                PlayerId::new(0),
            )),
            hovered: Some(obj(3)),
            ..Default::default()
        };

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_message::<KeyboardInput>()
            .insert_resource(duel)
            .add_systems(Update, super::keyboard);

        // A precondition, so that a keymap this test cannot see is never the
        // reason it passes: it would pass by pressing nothing at all.
        {
            let prefs = app.world().resource::<crate::prefs::Prefs>();
            let mut probe = ButtonInput::<KeyCode>::default();
            probe.press(KeyCode::Enter);
            assert!(
                crate::keys::Fired::of(&probe, prefs.keymap()).has(Action::Primary),
                "enter is the primary key in the standard map"
            );
        }

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [PlayerAction::PlayLand { card: obj(3) }],
            "the land under the cursor is what the primary key plays"
        );
    }

    /// The browser had a pointer route and no keyboard one, which is exactly
    /// the promise `docs/keyboard-map.md` makes and the reason the action was
    /// added rather than the chip being the only way in.
    #[test]
    fn the_browser_key_opens_the_tray_and_shuts_it_again() {
        use bevy::input::ButtonInput;
        use bevy::input::keyboard::KeyboardInput;
        use bevy::prelude::*;

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_message::<KeyboardInput>()
            .init_resource::<crate::Duel>()
            .add_systems(Update, super::keyboard);

        // `reset_all` and not `clear`: a key that is still held is not pressed
        // again, and the second press would fire nothing at all.
        let press = |app: &mut App, key: KeyCode| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);
            app.update();
        };

        assert!(!app.world().resource::<crate::Duel>().browser.is_open());
        press(&mut app, KeyCode::KeyG);
        assert!(
            app.world().resource::<crate::Duel>().browser.is_open(),
            "the browser key did not open the tray"
        );
        press(&mut app, KeyCode::KeyG);
        assert!(
            !app.world().resource::<crate::Duel>().browser.is_open(),
            "it is a latch, so the same key shuts it"
        );
    }
}
