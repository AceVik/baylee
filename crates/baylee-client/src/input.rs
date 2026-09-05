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

use crate::hud::{
    AbilityButton, ChoiceButton, HandCardVisual, MenuAction, MenuButton, OverlayKnob, PhaseButton,
    PileChip, PlayerTab, PreviewResize, PromptAction, PromptButton, RailButton, TrayCard,
    TrayClose, TrayTab,
};
use crate::keys::Fired;
use crate::settings::ClientSettings;
use crate::table::CardVisual;
use crate::{Deed, Duel};
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
    // A second tap on the same card is the send. A tap on a different one is
    // a change of mind and not a confirmation, so it disarms and arms afresh.
    if duel.armed.as_ref().is_some_and(|a| a.object == object) {
        fire_armed(duel);
        return;
    }
    duel.armed = None;
    if duel
        .interaction
        .as_ref()
        .and_then(|i| i.play_card(object))
        .is_some()
    {
        arm(duel, object, Deed::Play);
        return;
    }
    if duel.reachable.contains(&object)
        && let Some(plan) = crate::mana_for(duel, object)
    {
        duel.last_error = None;
        arm(duel, object, Deed::Run(plan));
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
                arm_ability(duel, object, &options[0]);
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
    let answered = duel
        .interaction
        .as_mut()
        .is_some_and(|i| i.toggle(object) != SelectionOutcome::Rejected);
    if !answered {
        open_pile(duel, object);
    }
}

/// A tap that meant nothing else, on a card lying on top of a pile, opens
/// that pile.
///
/// Last of all the branches above, and that ordering is the rule rather than
/// an accident: while the engine is asking a player to choose a card out of
/// their graveyard, a tap on the top of it must *answer the question*, not
/// drop a panel over the board they are answering it from. Only a tap that
/// nothing else claimed is a request to look through the pile.
fn open_pile(duel: &mut Duel, object: ObjectId) {
    let Some(board) = duel.board.as_ref() else {
        return;
    };
    let opening = board.pods.iter().find_map(|pod| {
        pod.piles
            .iter()
            .find(|pile| pile.top == Some(object) && pile.is_browsable())
            .and_then(|pile| baylee_client_core::BrowseZone::of_pile(pile.kind, pod.player))
    });
    if let Some(zone) = opening {
        duel.browser.open_at(zone);
    }
}

/// Arms a deed. The prompt bar draws what is armed, and the way out of it.
fn arm(duel: &mut Duel, object: ObjectId, deed: Deed) {
    duel.armed = Some(crate::Armed { object, deed });
}

/// One ability: sent outright when it makes mana, armed otherwise.
///
/// The exception is narrow on purpose (`docs/design.md` §2.5). Floating mana
/// empties at end of step and a wrong colour is fixed by tapping another
/// source, so it is the one cheap mistake in the game; everything else on a
/// permanent — sacrificing it, paying life, a loyalty ability that can only
/// be used once a turn — is not.
fn arm_ability(duel: &mut Duel, object: ObjectId, option: &crate::abilities::AbilityOption) {
    if option.mana {
        duel.submit(option.action.clone());
    } else {
        arm(duel, object, Deed::Ability(option.action.clone()));
    }
}

/// Sends what is armed, or disarms when the engine no longer offers it.
///
/// Everything is resolved against the *current* `LegalActions` rather than
/// trusted from the tap that armed it — the rule the ability chooser and
/// [`crate::ManaRun`] both already follow. A deed that has gone stale leaves
/// nothing on the wire and says why.
pub fn fire_armed(duel: &mut Duel) {
    let Some(armed) = duel.armed.take() else {
        return;
    };
    match armed.deed {
        Deed::Play => {
            match duel
                .interaction
                .as_ref()
                .and_then(|i| i.play_card(armed.object))
            {
                Some(action) => duel.submit(action),
                None => duel.last_error = Some(STALE.to_string()),
            }
        }
        Deed::Ability(action) => {
            let still_offered = abilities_of(duel, armed.object)
                .is_some_and(|options| options.iter().any(|o| o.action == action));
            if still_offered {
                duel.submit(action);
            } else {
                duel.last_error = Some(STALE.to_string());
            }
        }
        // The plan itself is not re-planned: `ManaRun` re-checks every one of
        // its steps against what the engine is offering as it spends them, and
        // stops honestly if a land it counted on can no longer be tapped. What
        // *is* re-read is whether a run is still the right answer — between the
        // two taps this seat holds priority, so the one thing that can have
        // changed is its own manual land tap, after which the spell may be
        // castable outright and the run would float mana nobody asked for.
        Deed::Run(plan) => {
            if let Some(action) = duel
                .interaction
                .as_ref()
                .and_then(|i| i.play_card(armed.object))
            {
                duel.submit(action);
            } else if duel.reachable.contains(&armed.object) {
                duel.last_error = None;
                duel.mana_run = Some(crate::ManaRun::new(plan, armed.object));
            } else {
                duel.last_error = Some(STALE.to_string());
            }
        }
    }
}

/// What the bar says when an armed deed no longer exists.
///
/// English here, like the mana run's own abort lines beside it: `last_error`
/// is one channel carrying the gateway's words as well as the client's, and
/// translating half of it would be worse than translating none.
const STALE: &str = "the engine no longer offers that";

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
    // Same reason, and the same place in the order: a digit is bound to no
    // action, so `Fired` is empty for exactly the keys a number choice wants.
    if number_keys(&mut typed, &mut duel) {
        return;
    }
    if fired.quiet() {
        return;
    }
    // The keyboard's half of the same rule as the pointer's: any bound key
    // forgets a half-pressed concession.
    duel.concede_armed = false;
    look_around(fired, &mut duel, &mut rig, &mut settings, &mut prefs);
    // An armed deed owns the keyboard first, and ahead of the ability menu:
    // arming is where the chooser *ends*, so a confirm key reaching the menu
    // instead would pick a second ability rather than send the first.
    if armed_keys(fired, &mut duel) {
        return;
    }
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
    // Here rather than beside the other answers, because a hold is the one
    // thing a seat says while it is *not* being asked: the engine takes a
    // `SetPriorityHold` from any seated player at any time, which is what
    // makes cancelling one possible at all. Both keys cancel a running hold
    // and only set one when none is; `Duel::hold_action` owns that rule.
    if (fired.has(Action::HoldForStack) || fired.has(Action::HoldForTurn))
        && let Some(action) = duel.hold_action(fired.has(Action::HoldForTurn))
    {
        duel.submit(action);
    }
}

/// Typing a number rather than stepping to it.
///
/// Stepping from 0 to 9 is nine presses, and X is routinely somebody's whole
/// hand of lands. So a digit types: it appends to what stands, and falls back
/// to the digit alone when appending would leave the offered range — which is
/// what a player means by typing `7` when the value already reads `12` and the
/// maximum is 9. Backspace takes a digit off, and the interaction clamps
/// whatever comes out, so nothing typed here is expressible outside the range
/// the engine offered.
fn number_keys(typed: &mut MessageReader<KeyboardInput>, duel: &mut Duel) -> bool {
    if !matches!(
        duel.interaction.as_ref().map(Interaction::prompt),
        Some(Prompt::ChooseNumber { .. })
    ) {
        return false;
    }
    let mut touched = false;
    for event in typed.read() {
        if !event.state.is_pressed() {
            continue;
        }
        let Some(i) = duel.interaction.as_mut() else {
            continue;
        };
        match &event.logical_key {
            Key::Character(s) => {
                for digit in s.chars().filter_map(|c| c.to_digit(10)) {
                    let appended = i.number().saturating_mul(10).saturating_add(digit);
                    // `set_number` clamps, so "did it fit" is asked by
                    // comparing what came back with what went in.
                    if i.set_number(appended) != appended {
                        i.set_number(digit);
                    }
                    touched = true;
                }
            }
            Key::Backspace => {
                let shorter = i.number() / 10;
                i.set_number(shorter);
                touched = true;
            }
            _ => {}
        }
    }
    touched
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

/// An armed deed: the confirm keys send it, cancel disarms. Returns whether
/// it consumed the frame.
///
/// Cancel is listed first in `docs/keyboard-map.md`'s Escape order for a
/// reason — an armed deed is the cheapest thing in the client to undo,
/// because it is the only one with nothing on the wire yet.
pub fn armed_keys(fired: Fired, duel: &mut Duel) -> bool {
    if duel.armed.is_none() {
        return false;
    }
    if fired.has(Action::Cancel) {
        duel.armed = None;
        return true;
    }
    if fired.has(Action::Primary) || fired.has(Action::Confirm) || fired.has(Action::ActivateCard) {
        fire_armed(duel);
        return true;
    }
    false
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
        let option = options.get(duel.ability_pick).cloned();
        duel.ability_menu = None;
        if let Some(option) = option {
            arm_ability(duel, object, &option);
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
    if step != 0 {
        step_number(duel, step);
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

/// Moves a number choice by one, in whichever direction.
///
/// The one door for both arms of the stepper and both keys, so a click and a
/// key cannot come to disagree about what "up" is. The interaction clamps, so
/// holding a key stops at the boundary rather than producing something the
/// engine would reject.
fn step_number(duel: &mut Duel, delta: i32) {
    if let Some(i) = duel.interaction.as_mut() {
        let next = if delta > 0 {
            i.number().saturating_add(1)
        } else {
            i.number().saturating_sub(1)
        };
        i.set_number(next);
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
    let picked = duel
        .ability_menu
        .and_then(|object| Some((object, abilities_of(duel, object)?)))
        .and_then(|(object, options)| Some((object, options.get(index).cloned()?)));
    duel.ability_menu = None;
    if let Some((object, option)) = picked {
        arm_ability(duel, object, &option);
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
/// The two buttons in the top-right menu.
///
/// `was_armed` is the concession's state *before* this click, taken once at
/// the top of the loop: every click disarms, so the second press only counts
/// when nothing happened in between.
fn menu_click(duel: &mut Duel, action: MenuAction, was_armed: bool) {
    match action {
        // Two presses, because there is no undo behind this one.
        MenuAction::Concede => {
            if was_armed {
                duel.submit(PlayerAction::Concede);
            } else {
                duel.concede_armed = true;
            }
        }
        // Re-checked and not merely drawn greyed: a button drawn a frame ago
        // must not send what the engine has since withdrawn, which is the same
        // rule the ability chooser follows. Draw offers still need mutual
        // agreement — a protocol item.
        MenuAction::OfferDraw => {
            if duel.can_offer_draw() {
                duel.submit(PlayerAction::OfferDraw);
            }
        }
        // Through the same door as the keys, and re-checked for the same
        // reason: the button is only drawn while a hold is running, and
        // `hold_action` reads the *current* view rather than the one that was
        // drawn — so a hold the engine has already expired cannot be
        // "cancelled" into a new one by a stale button.
        MenuAction::ReleaseHold => {
            if duel.priority_held()
                && let Some(action) = duel.hold_action(false)
            {
                duel.submit(action);
            }
        }
        // The same door the keys use, so the two ways of confirming cannot
        // drift; `fire_armed` re-resolves against the current `LegalActions`.
        MenuAction::SendArmed => fire_armed(duel),
        MenuAction::CancelArmed => duel.armed = None,
    }
}

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
        // Every click disarms the concession, and the concede branch below
        // reads what it *was*. Taken here rather than in that branch so the
        // rule is one line and cannot be forgotten by a widget added later:
        // a half-pressed concession survives exactly nothing.
        let was_armed = std::mem::take(&mut duel.concede_armed);
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
            menu_click(&mut duel, button.action, was_armed);
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
                // Stepping changes nothing the engine can hear either: the
                // number is not an answer until Confirm sends it.
                PromptAction::Step(delta) => {
                    step_number(&mut duel, delta);
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
    mut source: Local<HoverSource>,
    mut last: Local<Option<ObjectId>>,
    cards: Query<&CardVisual>,
    hand_cards: Query<&HandCardVisual>,
    parents: Query<&ChildOf>,
    mut duel: ResMut<Duel>,
) {
    // `hovered` has four writers and only one of them is this system. A hover
    // this system did not write knows nothing about where it came from, so it
    // is `Elsewhere` — which is the permissive answer, and has to be: the
    // keyboard cursor walking off a permanent and onto a hand card would
    // otherwise meet a stale `Table` source and be cleared on the next frame.
    // That is the stall of "The pointer only speaks when it moves" all over
    // again, through a different door.
    if duel.hovered != *last {
        *source = HoverSource::Elsewhere;
    }

    // A hovered card can leave without ever firing an `Out`, and playing the
    // card under the pointer is the ordinary way into that: the hand bar is
    // rebuilt, the node the pointer was over is despawned, and a despawned
    // entity reports nothing. So the hover is also held against the kind of
    // entity that reported it. A land played from the hand is no longer *a
    // hand card* whatever the battlefield now draws under the same id, which
    // is why the source matters and a bare "does this object still exist"
    // would not have cleared it.
    //
    // Checked before the grace window, because the pointer has not moved —
    // that is the whole point.
    if let Some(object) = duel.hovered {
        let alive = match *source {
            HoverSource::Hand => hand_cards.iter().any(|h| h.object == object),
            HoverSource::Table => cards.iter().any(|v| v.object == object),
            // Somebody else's write — the keyboard cursor, most often, and
            // the union is that cursor's own invariant rather than a
            // weakening of the two above. The two kinds of hover are valid
            // for different reasons: a *pointer* hover holds while the
            // pointer is over the entity that reported it, and a *keyboard*
            // cursor holds while its object is anywhere in `cursor_grid`,
            // which spans the hand and every pod's lanes. An `ObjectId`
            // survives a zone change — nothing bumps a generation and
            // nothing frees an arena slot — so a card played off the cursor
            // is still in the grid, one row down, and `move_cursor` keeps
            // navigating from it. Clearing it there would not fix a ghost;
            // it would drop the player's cursor and send the next arrow key
            // back to the first card in hand.
            // `move_cursor` heals a stale one on its own, so only a card that
            // has left both places is cleared.
            HoverSource::Elsewhere => {
                hand_cards.iter().any(|h| h.object == object)
                    || cards.iter().any(|v| v.object == object)
            }
        };
        if !alive {
            duel.hovered = None;
            *source = HoverSource::Elsewhere;
        }
    }

    if moves.read().next().is_some() {
        *grace = 3;
    }
    // Not an early return, because the last line has to run on every path:
    // a `*last` left behind would make this system read its own write as
    // somebody else's on the very next frame.
    if *grace == 0 {
        // Drain, so a later real move does not act on a backlog of events
        // the cards generated by sliding around.
        overs.clear();
        outs.clear();
    } else {
        *grace -= 1;
        for over in overs.read() {
            if let Some(v) = find_in_lineage(over.entity, &cards, &parents) {
                duel.hovered = Some(v.object);
                *source = HoverSource::Table;
            } else if let Some(h) = find_in_lineage(over.entity, &hand_cards, &parents) {
                duel.hovered = Some(h.object);
                *source = HoverSource::Hand;
            }
        }
        for out in outs.read() {
            let is_current = find_in_lineage(out.entity, &cards, &parents)
                .is_some_and(|v| duel.hovered == Some(v.object))
                || find_in_lineage(out.entity, &hand_cards, &parents)
                    .is_some_and(|h| duel.hovered == Some(h.object));
            if is_current {
                duel.hovered = None;
                *source = HoverSource::Elsewhere;
            }
        }
    }

    *last = duel.hovered;
}

/// Which kind of entity reported the hover the client is currently drawing.
///
/// The pointer is the authority on *what* is hovered and the events are the
/// authority on when it stops — except that a card can be despawned out from
/// under a still pointer, which fires no event at all. This is what makes
/// that case answerable: the hover survives only while something of the same
/// kind still draws the object.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum HoverSource {
    /// A hand card, a command-zone card, or a card in the own-board overlay.
    Hand,
    /// A permanent on the table.
    Table,
    /// The keyboard cursor, or nothing at all.
    #[default]
    Elsewhere,
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

    // The arrows are `NumberUp`/`NumberDown` in the standard keymap, and this
    // function reads `KeyCode` directly rather than through it — so while a
    // number is being chosen the same press was both raising X and panning the
    // table under it. The choice wins; the mouse and the touch gestures below
    // are untouched, because neither of them is bound to anything.
    let stepping = matches!(
        duel.interaction.as_ref().map(Interaction::prompt),
        Some(Prompt::ChooseNumber { .. })
    );

    // ---- keyboard: pan / zoom / rotate ----------------------------------
    let pan_step = rig.distance * 0.02;
    if stepping {
        // nothing: the arrows belong to the number
    } else if shift {
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

/// Returns the camera to the view behind the local seat that takes in the
/// whole table.
///
/// The rig is set back to its default rather than to that framing, because
/// the framing depends on the window and on the layout this call is about to
/// change: `table::frame_table` recognises the default as "nobody aimed this"
/// and puts the table back in frame on the next tick.
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

        // Twice, because playing a land is irreversible and therefore
        // two-stage: the first press arms what the cursor is over and the
        // second sends it. `reset_all` between them, or the key is still
        // held and never fires again.
        for _ in 0..2 {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::Enter);
            app.update();
        }

        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [PlayerAction::PlayLand { card: obj(3) }],
            "the land under the cursor is what the primary key plays"
        );
    }

    /// Builds the app the two menu-button tests share: the real `pointer`
    /// system, and a click helper that goes through the real message.
    fn menu_app(
        duel: crate::Duel,
    ) -> (bevy::app::App, bevy::prelude::Entity, bevy::prelude::Entity) {
        use crate::hud::MenuAction;
        use bevy::prelude::*;

        let mut app = App::new();
        app.init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Click>>()
            .insert_resource(duel)
            .add_systems(Update, super::pointer);
        let draw = app
            .world_mut()
            .spawn(crate::hud::MenuButton {
                action: MenuAction::OfferDraw,
            })
            .id();
        let concede = app
            .world_mut()
            .spawn(crate::hud::MenuButton {
                action: MenuAction::Concede,
            })
            .id();
        (app, draw, concede)
    }

    /// One click on one entity, as the picking backend would report it.
    fn click(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::picking::events::{Click, Pointer};
        use bevy::picking::pointer::{Location, PointerId};
        use bevy::prelude::*;
        use bevy::window::{PrimaryWindow, WindowRef};

        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap_or_else(|_| {
                app.world_mut()
                    .spawn((Window::default(), PrimaryWindow))
                    .id()
            });
        let camera = app.world_mut().spawn_empty().id();
        let target = WindowRef::Entity(window)
            .normalize(Some(window))
            .expect("a window is a render target");
        let location = Location {
            target: NormalizedRenderTarget::Window(target),
            position: Vec2::ZERO,
        };
        let event = Click {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
            duration: std::time::Duration::from_millis(10),
            count: 1,
        };
        app.world_mut()
            .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
        app.update();
    }

    /// Playing the card under the pointer must take its preview with it.
    ///
    /// The hand bar is rebuilt whole on every board change, so the node the
    /// pointer was over is *despawned* — and Bevy fires no `Out` for an
    /// entity that no longer exists. The card preview therefore stayed open
    /// over the middle of the table until the player happened to hover
    /// something else, which is how a screenshot of a live game found it.
    ///
    /// The second half is the part a bare "does this object still exist"
    /// check would fail: a land goes on playing under the same `ObjectId`,
    /// now as a permanent, so the hover is only stale because it came from
    /// the *hand*.
    #[test]
    fn a_hand_card_that_is_played_takes_its_hover_with_it() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
            .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
            .add_message::<bevy::window::CursorMoved>()
            .insert_resource(crate::Duel::default())
            .add_systems(Update, super::pointer_hover);

        let card = app
            .world_mut()
            .spawn(crate::hud::HandCardVisual { object: obj(3) })
            .id();
        hover(&mut app, card);
        assert_eq!(
            app.world().resource::<crate::Duel>().hovered,
            Some(obj(3)),
            "the pointer over a hand card is a hover"
        );

        // The land is played: the hand bar is rebuilt without it, and the
        // same object arrives on the table. No `Out` is fired, and the
        // pointer does not move.
        app.world_mut().entity_mut(card).despawn();
        app.world_mut().spawn(crate::table::CardVisual {
            object: obj(3),
            count: 1,
        });
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().hovered,
            None,
            "a hand card that left the hand is not still hovered"
        );
    }

    /// The cure must not bring back the disease.
    ///
    /// `Duel::hovered` has four writers and `pointer_hover` is only one of
    /// them: the keyboard cursor writes it too, and the cursor walking off a
    /// permanent and onto a hand card is exactly that. Held against the
    /// *pointer's* last source, that write would be checked against the table,
    /// not found there and cleared on the next frame — the stall of "the
    /// pointer only speaks when it moves" back through a different door. So a
    /// hover this system did not write is nobody's kind in particular.
    #[test]
    fn a_hover_this_system_did_not_write_is_left_alone() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
            .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
            .add_message::<bevy::window::CursorMoved>()
            .insert_resource(crate::Duel::default())
            .add_systems(Update, super::pointer_hover);

        let permanent = app
            .world_mut()
            .spawn(crate::table::CardVisual {
                object: obj(7),
                count: 1,
            })
            .id();
        app.world_mut()
            .spawn(crate::hud::HandCardVisual { object: obj(3) });
        hover(&mut app, permanent);
        assert_eq!(app.world().resource::<crate::Duel>().hovered, Some(obj(7)));

        // What `move_cursor` does when the keyboard walks onto the hand: it
        // writes the hover directly, and the pointer has not moved.
        app.world_mut().resource_mut::<crate::Duel>().hovered = Some(obj(3));
        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().hovered,
            Some(obj(3)),
            "the keyboard cursor survives a pointer that is resting elsewhere"
        );
    }

    /// The other half of `a_hand_card_that_is_played_takes_its_hover_with_it`.
    ///
    /// The same event — the card under the hover is played, its hand node is
    /// despawned and a permanent appears under the same `ObjectId` — and the
    /// answer is the opposite one, because the two hovers are valid for
    /// different reasons. The pointer's is over an entity that no longer
    /// exists, so it goes. The keyboard's is a position in `cursor_grid`,
    /// the card is still in that grid one row down, and taking it away would
    /// send the player's next arrow key back to the start of their hand.
    ///
    /// Written as a test rather than left to the comment because the union in
    /// the `Elsewhere` arm reads like the permissive fallback of the other
    /// two, and the next person to tighten it will have this fail.
    #[test]
    fn the_keyboard_cursor_follows_a_card_it_played_onto_the_table() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
            .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
            .add_message::<bevy::window::CursorMoved>()
            .insert_resource(crate::Duel::default())
            .add_systems(Update, super::pointer_hover);

        // A land in hand, with the keyboard cursor on it: written straight to
        // the resource, which is what `move_cursor` does.
        let in_hand = app
            .world_mut()
            .spawn(crate::hud::HandCardVisual { object: obj(5) })
            .id();
        app.world_mut().resource_mut::<crate::Duel>().hovered = Some(obj(5));
        app.update();

        // It is played. The hand bar is rebuilt without it and the same
        // object is now a permanent.
        app.world_mut().entity_mut(in_hand).despawn();
        app.world_mut().spawn(crate::table::CardVisual {
            object: obj(5),
            count: 1,
        });
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().hovered,
            Some(obj(5)),
            "the cursor should follow the card it just played, not reset"
        );
    }

    /// One pointer entering one entity, as the picking backend would report
    /// it. The cursor move is what opens the system's grace window: a still
    /// pointer is deliberately silent.
    fn hover(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::picking::events::{Over, Pointer};
        use bevy::picking::pointer::{Location, PointerId};
        use bevy::prelude::*;
        use bevy::window::{PrimaryWindow, WindowRef};

        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap_or_else(|_| {
                app.world_mut()
                    .spawn((Window::default(), PrimaryWindow))
                    .id()
            });
        let camera = app.world_mut().spawn_empty().id();
        let target = WindowRef::Entity(window)
            .normalize(Some(window))
            .expect("a window is a render target");
        let location = Location {
            target: NormalizedRenderTarget::Window(target),
            position: Vec2::ZERO,
        };
        app.world_mut().write_message(bevy::window::CursorMoved {
            window,
            position: Vec2::ZERO,
            delta: None,
        });
        app.world_mut().write_message(Pointer::new(
            PointerId::Mouse,
            location,
            Over {
                hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
            },
            entity,
        ));
        app.update();
    }

    /// The engine refuses a draw offer outside the offerer's own priority, so
    /// the button used to be a live button whose usual answer was an error.
    #[test]
    fn a_draw_is_only_offered_from_this_seats_own_priority() {
        use bevy::prelude::*;

        // A choice that is not priority: the offer is not sent.
        let (mut app, draw, _) = menu_app(crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::YesNo {
                    player: PlayerId::new(0),
                    prompt: baylee_engine::choice::YesNoPrompt::Generic,
                    source: None,
                },
                PlayerId::new(0),
            )),
            ..Default::default()
        });
        click(&mut app, draw);
        assert!(
            app.world().resource::<crate::Duel>().outbox().is_empty(),
            "a draw was offered without priority, which the engine refuses"
        );

        // And with priority it goes.
        let (mut app, draw, _) = menu_app(crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::Priority {
                    player: PlayerId::new(0),
                    legal: Box::new(LegalActions {
                        can_pass: true,
                        lands: vec![],
                        castable: vec![],
                        mana_abilities: vec![],
                        abilities: vec![],
                        suspendable: vec![],
                    }),
                },
                PlayerId::new(0),
            )),
            ..Default::default()
        });
        click(&mut app, draw);
        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [PlayerAction::OfferDraw]
        );
    }

    /// One misclick used to end a ranked game.
    #[test]
    fn conceding_takes_two_presses_and_anything_else_forgets_the_first() {
        let (mut app, draw, concede) = menu_app(crate::Duel::default());

        click(&mut app, concede);
        assert!(
            app.world().resource::<crate::Duel>().outbox().is_empty(),
            "one press conceded the game"
        );
        assert!(app.world().resource::<crate::Duel>().concede_armed);

        // Anything else in between and the first press is forgotten.
        click(&mut app, draw);
        assert!(!app.world().resource::<crate::Duel>().concede_armed);
        click(&mut app, concede);
        assert!(app.world().resource::<crate::Duel>().outbox().is_empty());

        // Twice in a row, and it goes.
        click(&mut app, concede);
        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [PlayerAction::Concede]
        );
    }

    /// A number choice with a range, ready to be typed at.
    fn number_duel(max: u32) -> crate::Duel {
        crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::ChooseNumber {
                    player: PlayerId::new(0),
                    min: 0,
                    max,
                },
                PlayerId::new(0),
            )),
            ..Default::default()
        }
    }

    /// Stepping from 0 to 12 is twelve presses, and X is routinely somebody's
    /// whole hand of lands.
    #[test]
    fn a_number_can_be_typed_rather_than_stepped_to() {
        use bevy::input::ButtonInput;
        use bevy::input::keyboard::{Key, KeyboardInput};
        use bevy::prelude::*;

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_message::<KeyboardInput>()
            .insert_resource(number_duel(20))
            .add_systems(Update, super::keyboard);
        let window = app.world_mut().spawn_empty().id();

        let type_digit = |app: &mut App, c: char| {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::Digit0,
                logical_key: Key::Character(c.to_string().into()),
                state: bevy::input::ButtonState::Pressed,
                text: Some(c.to_string().into()),
                repeat: false,
                window,
            });
            app.update();
        };

        type_digit(&mut app, '1');
        type_digit(&mut app, '2');
        let number = |app: &App| {
            app.world()
                .resource::<crate::Duel>()
                .interaction
                .as_ref()
                .expect("the choice stands")
                .number()
        };
        assert_eq!(number(&app), 12, "a second digit appends");

        // …and a digit that would leave the range is the whole answer instead,
        // which is what a player means by typing 7 at a maximum of 20.
        type_digit(&mut app, '7');
        assert_eq!(number(&app), 7);

        // Backspace takes one off.
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Backspace,
            logical_key: Key::Backspace,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
        assert_eq!(number(&app), 0);
    }

    /// The arrows are bound to `NumberUp`/`NumberDown`, and `camera_controls`
    /// reads `KeyCode` directly rather than through the keymap — so the same
    /// press was raising X and panning the table out from under it.
    #[test]
    fn the_arrows_do_not_pan_the_table_while_a_number_is_being_chosen() {
        use bevy::input::ButtonInput;
        use bevy::input::mouse::{MouseMotion, MouseWheel};
        use bevy::prelude::*;

        let run = |duel: crate::Duel| {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<ButtonInput<MouseButton>>()
                .init_resource::<crate::table::CameraRig>()
                .add_message::<MouseMotion>()
                .add_message::<MouseWheel>()
                .add_message::<bevy::input::gestures::PanGesture>()
                .add_message::<bevy::input::gestures::PinchGesture>()
                .add_message::<bevy::input::gestures::RotationGesture>()
                .insert_resource(duel)
                .add_systems(Update, super::camera_controls);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ArrowUp);
            app.update();
            app.world().resource::<crate::table::CameraRig>().target
        };

        assert_eq!(
            run(number_duel(20)),
            crate::table::CameraRig::default().target,
            "the arrow panned the table while it was choosing a number"
        );
        assert_ne!(
            run(crate::Duel::default()),
            crate::table::CameraRig::default().target,
            "and with no number pending the arrow still pans"
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

    /// A hold is the one statement a seat makes while it is *not* being asked,
    /// which is also what makes it dangerous: the prompt bar is empty because
    /// the seat is not being asked, and an empty prompt bar is what an idle
    /// turn looks like too. So the key that sets a hold has to be the key that
    /// takes it back, and it has to work from a view alone.
    #[test]
    fn the_hold_keys_stop_the_questions_and_take_it_back() {
        use baylee_engine::choice::PriorityHold;
        use bevy::input::ButtonInput;
        use bevy::input::keyboard::KeyboardInput;
        use bevy::prelude::*;

        use crate::host::{DuelHost, HostMessage, LocalHost};
        let mut host = LocalHost::new(
            &crate::host::tests::duel_preset(),
            PlayerId::new(0),
            &["You", "AI"],
        )
        .expect("host");
        // A real view rather than a hand-built one: `hold_action` reads the
        // turn number and the stack depth off it, and a view assembled by the
        // test would only ever agree with the test.
        let view = host
            .poll()
            .into_iter()
            .find_map(|m| match m {
                HostMessage::View(v) => Some(*v),
                _ => None,
            })
            .expect("a view");
        let turn = view.turn;
        let depth = u16::try_from(view.stack.len()).expect("an opening stack fits");

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_message::<KeyboardInput>()
            .insert_resource(crate::Duel {
                view: Some(view),
                ..Default::default()
            })
            .add_systems(Update, super::keyboard);

        // `reset_all` and not `clear`: a key still held is not pressed again.
        let press = |app: &mut App, key: KeyCode| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);
            app.update();
        };
        let held = |app: &mut App, held: bool| {
            app.world_mut()
                .resource_mut::<crate::Duel>()
                .view
                .as_mut()
                .expect("the view is still there")
                .priority_held = held;
        };

        press(&mut app, KeyCode::F6);
        press(&mut app, KeyCode::F7);
        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [
                PlayerAction::SetPriorityHold(PriorityHold::UntilStackEmpty { depth }),
                PlayerAction::SetPriorityHold(PriorityHold::UntilEndOfTurn { turn }),
            ],
            "the two hold keys must say two different things"
        );

        // The engine took it; the view says so. Now either key is the way out,
        // because a player who has stopped being asked should not have to
        // remember which one they pressed.
        held(&mut app, true);
        press(&mut app, KeyCode::F7);
        held(&mut app, true);
        press(&mut app, KeyCode::F6);
        assert_eq!(
            app.world().resource::<crate::Duel>().outbox()[2..],
            [
                PlayerAction::SetPriorityHold(PriorityHold::Always),
                PlayerAction::SetPriorityHold(PriorityHold::Always),
            ],
            "a running hold must be cancelled by either key, never replaced"
        );
    }

    /// The same way out, for a player who never finds a function key.
    #[test]
    fn the_prompt_bar_can_take_a_hold_back_too() {
        use baylee_engine::choice::PriorityHold;

        use crate::host::{DuelHost, HostMessage, LocalHost};
        use crate::hud::MenuAction;
        let mut host = LocalHost::new(
            &crate::host::tests::duel_preset(),
            PlayerId::new(0),
            &["You", "AI"],
        )
        .expect("host");
        let mut view = host
            .poll()
            .into_iter()
            .find_map(|m| match m {
                HostMessage::View(v) => Some(*v),
                _ => None,
            })
            .expect("a view");
        view.priority_held = true;
        let mut duel = crate::Duel {
            view: Some(view),
            ..Default::default()
        };

        super::menu_click(&mut duel, MenuAction::ReleaseHold, false);
        assert_eq!(
            duel.outbox(),
            [PlayerAction::SetPriorityHold(PriorityHold::Always)]
        );

        // And with nothing to release it sends nothing, rather than setting a
        // hold from the button that exists to cancel one.
        duel.view.as_mut().expect("the view").priority_held = false;
        let mut fresh = crate::Duel {
            view: duel.view.clone(),
            ..Default::default()
        };
        super::menu_click(&mut fresh, MenuAction::ReleaseHold, false);
        assert!(fresh.outbox().is_empty());
    }

    /// A duel driven to seat 0's first main phase, out of a real `LocalHost`.
    ///
    /// Every one of the arming tests needs a `LegalActions` that actually
    /// offers something, and a hand-built one offers whatever the test wanted
    /// it to. This plays the opening the way the client does — keep the hand,
    /// pass until the engine offers a land — and stops at the window where a
    /// player has a decision to make.
    fn duel_in_main_phase() -> (crate::Duel, crate::host::LocalHost) {
        use crate::host::{DuelHost, HostMessage, LocalHost};
        use baylee_engine::choice::Pending;

        let seat = PlayerId::new(0);
        let mut host = LocalHost::new(&crate::host::tests::duel_preset(), seat, &["You", "AI"])
            .expect("the preset makes a game");
        let mut duel = crate::Duel::default();
        for _ in 0..64 {
            for message in host.poll() {
                match message {
                    HostMessage::Static(s) => duel.statics = Some(*s),
                    HostMessage::View(v) => duel.view = Some(*v),
                    HostMessage::Choice(p) => {
                        duel.interaction = Some(Interaction::new(*p, seat));
                    }
                    HostMessage::Failed(why) => panic!("the host refused: {why}"),
                }
            }
            match duel.interaction.as_ref().map(Interaction::pending) {
                Some(Pending::Mulligan { .. }) => host.submit(PlayerAction::MulliganKeep),
                Some(Pending::Priority { legal, .. }) if !legal.lands.is_empty() => {
                    return (duel, host);
                }
                Some(Pending::Priority { .. }) => host.submit(PlayerAction::PassPriority),
                _ => break,
            }
        }
        panic!("the game never offered seat 0 a land to play");
    }

    /// The whole of arm-then-act: the first tap says nothing, the second
    /// sends, and cancel leaves the wire empty.
    ///
    /// There is no undo in the engine and there should not be one, so the
    /// client owes a player the chance to take a tap back before it becomes a
    /// game action. Tested at this level and not on `Interaction`, because
    /// what is being claimed is about *taps*: an assertion that the second
    /// call to a resolver returns an action would pass just as well if the
    /// first one had already sent it.
    #[test]
    fn a_tap_arms_a_card_and_a_second_tap_plays_it() {
        let (mut duel, _host) = duel_in_main_phase();
        let land = duel
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
            .and_then(|l| l.lands.first().copied())
            .expect("the window offers a land");

        super::activate_card(&mut duel, land);
        assert!(
            duel.outbox().is_empty(),
            "the first tap put a card on the wire"
        );
        assert_eq!(
            duel.armed,
            Some(crate::Armed {
                object: land,
                deed: crate::Deed::Play
            })
        );

        super::activate_card(&mut duel, land);
        assert_eq!(duel.outbox(), [PlayerAction::PlayLand { card: land }]);
        assert!(duel.armed.is_none(), "firing left the deed armed");
    }

    /// Cancel is the whole point of arming: it has to leave nothing behind.
    #[test]
    fn cancel_disarms_with_nothing_on_the_wire() {
        use crate::keys::Fired;
        use baylee_client_core::prefs::{Action, Chord, Keymap};
        use bevy::prelude::KeyCode;

        let (mut duel, _host) = duel_in_main_phase();
        let land = duel
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
            .and_then(|l| l.lands.first().copied())
            .expect("the window offers a land");

        super::activate_card(&mut duel, land);
        assert!(duel.armed.is_some());

        let mut keymap = Keymap::standard();
        keymap.bind(Action::Cancel, vec![Chord::key("Escape")]);
        let mut keys = bevy::input::ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Escape);
        assert!(super::armed_keys(Fired::of(&keys, &keymap), &mut duel));
        assert!(duel.armed.is_none(), "cancel left the deed armed");
        assert!(duel.outbox().is_empty(), "cancel sent something");
    }

    /// The exception, and the reason it is one: floating mana is the cheap
    /// mistake in this game, so tapping a land stays a single tap.
    #[test]
    fn a_mana_ability_still_goes_through_on_one_tap() {
        use crate::host::DuelHost;
        let (mut duel, mut host) = duel_in_main_phase();
        let land = duel
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
            .and_then(|l| l.lands.first().copied())
            .expect("the window offers a land");

        // Put the land in play first — it is the only mana source this deck
        // has, and a land in hand makes no mana.
        super::activate_card(&mut duel, land);
        super::activate_card(&mut duel, land);
        // `outbox` is private to `Duel` but declared in the crate root, so a
        // child module may drain it — which is what `flush_outbox` does in
        // the running client.
        for action in std::mem::take(&mut duel.outbox) {
            host.submit(action);
        }
        for message in host.poll() {
            match message {
                crate::host::HostMessage::View(v) => duel.view = Some(*v),
                crate::host::HostMessage::Choice(p) => {
                    duel.interaction = Some(Interaction::new(*p, PlayerId::new(0)));
                }
                _ => {}
            }
        }
        let source = duel
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
            .and_then(|l| l.mana_abilities.first().copied())
            .expect("the land that was just played can be tapped");

        super::activate_card(&mut duel, source);
        assert!(
            duel.armed.is_none(),
            "tapping for mana asked for a confirmation"
        );
        assert_eq!(
            duel.outbox(),
            [PlayerAction::ActivateManaAbility { source }]
        );
    }
}

#[cfg(test)]
mod refusal_tests {
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_engine::choice::{LegalActions, Pending, PlayerAction};

    fn priority() -> Pending {
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                ..LegalActions::default()
            }),
        }
    }

    /// A refusal stands until this seat tries something else.
    ///
    /// It cannot be cleared when the next question arrives, which is the
    /// obvious place: the acting seat is re-sent its own question every time
    /// anybody at the table says anything, so the line would have been wiped
    /// a frame or two after it appeared — and the click that earned it is the
    /// click the player is waiting to understand.
    #[test]
    fn a_refusal_outlives_the_question_it_answered() {
        let mut duel = crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                priority(),
                PlayerId::new(0),
            )),
            last_error: Some("illegal action for your seat".to_string()),
            ..crate::Duel::default()
        };

        // The same question again — an opponent said something. This goes
        // through the arm that installs it, because assigning the field would
        // only be testing that writing one field leaves another alone.
        duel.receive_choice(priority());
        assert_eq!(
            duel.last_error.as_deref(),
            Some("illegal action for your seat"),
            "a re-sent question is not this player doing anything"
        );

        // This player tries again: the old refusal is about the old attempt.
        duel.submit(PlayerAction::PassPriority);
        assert!(duel.last_error.is_none());
        assert_eq!(duel.outbox(), &[PlayerAction::PassPriority]);
    }

    /// An armed deed the engine has withdrawn says so, and says it without
    /// sending anything.
    #[test]
    fn a_stale_deed_reports_instead_of_firing() {
        let mut duel = crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                priority(),
                PlayerId::new(0),
            )),
            armed: Some(crate::Armed {
                object: ObjectId::new(3, 0),
                deed: crate::Deed::Play,
            }),
            ..crate::Duel::default()
        };
        super::fire_armed(&mut duel);
        assert!(duel.outbox().is_empty(), "nothing goes on the wire");
        assert_eq!(duel.last_error.as_deref(), Some(super::STALE));
    }
}
