//! Keys on a sheet: digits, rows, pages, the cursor, arranging, the browser's answers.

#[allow(clippy::wildcard_imports)] // the input module's shared vocabulary
use super::*;

/// The digits, while an ability or local cast sheet stands: each row is
/// selected by its printed number; `0` pages abilities. Returns whether it consumed the
/// frame.
///
/// Read straight off `KeyboardInput` as `Key::Character`, the way
/// `subtype_keys` and the number-entry path already read digits. The
/// alternative is nine new [`Action`]s, which is nine rows in every keymap
/// screen and a rebinding a player could make — for a key whose entire
/// meaning is the numeral printed beside the row.
///
/// A row with a cost **arms** on the first press and sends on the second,
/// which is [`abilitysheet::press`]'s whole answer; a mana ability and a
/// `{T}`-only ability send on the first, which is the same one-tap exemption
/// [`arm_ability`] applies everywhere else.
pub(super) fn sheet_digits(typed: &mut MessageReader<KeyboardInput>, duel: &mut Duel) -> bool {
    // Both halves of the guard are about *not reading the events*. The sheet
    // can stand open while the browser's filter box has the keyboard, and
    // `typed.read()` drains every key rather than the digits alone — so a
    // sheet that read here unconditionally would eat the letters a player is
    // typing into that box and open an ability with the digits.
    if (duel.ability_menu.is_none() && duel.cast_menu.is_none() && !duel.granted_menu.open)
        || duel.browser.is_typing()
    {
        return false;
    }
    let pressed: Vec<char> = typed
        .read()
        .filter(|event| event.state.is_pressed())
        .filter_map(|event| match &event.logical_key {
            Key::Character(s) => Some(s.chars()),
            _ => None,
        })
        .flatten()
        .filter(char::is_ascii_digit)
        .collect();
    if pressed.is_empty() {
        return false;
    }
    let mut took = false;
    for digit in pressed {
        if duel.granted_menu.open {
            let index = digit
                .to_digit(10)
                .and_then(|n| n.checked_sub(1))
                .map(|n| n as usize);
            let id = index
                .and_then(|index| crate::hud::granted_offers(duel).get(index))
                .map(|o| o.id);
            if let Some(id) = id {
                crate::hud::granted_click(duel, MenuAction::PickGranted(id));
            }
            return true;
        }
        if let Some(menu) = duel.cast_menu.as_ref() {
            let row = digit
                .to_digit(10)
                .and_then(|n| n.checked_sub(1))
                .map(|n| n as usize);
            if let Some(row) = row.filter(|row| *row < menu.modes.len()) {
                take_cast_row(duel, row);
            }
            // Choosing only arms. Further digits from this frame cannot
            // accidentally confirm the newly armed spell.
            return true;
        }
        // A digit may have sent something, which puts the sheet away, and
        // every digit after it would then be read against a permanent that is
        // no longer being asked about.
        if duel.ability_menu.is_none() {
            break;
        }
        took |= sheet_digit(duel, digit);
    }
    took
}

/// One digit, against the sheet as it stands: a row, or the pager. Returns
/// whether it named either.
///
/// Split out of [`sheet_digits`] because that one takes a `MessageReader` and
/// this is the half worth testing — arming, sending and paging are the
/// two-stage mechanic, and a test that had to build keyboard events to reach
/// them would be testing Bevy.
///
/// The list is rebuilt here for every digit, because the one before it may
/// have sent something: a list read once and used twice is the chooser bug
/// this whole path was written to avoid.
pub fn sheet_digit(duel: &mut Duel, digit: char) -> bool {
    let Some(object) = duel.ability_menu else {
        return false;
    };
    let Some(options) = abilities_of(duel, object).filter(|o| o.len() > 1) else {
        return false;
    };
    // The digits count the **written** rows and skip the pips a mana header
    // leads with, which is [`crate::abilities::Split::numbered`]: a pip on a
    // sheet carries no keycap, so a digit that reached one would send a row
    // nothing on screen had numbered.
    let (head, counted) = crate::abilities::Split::of(&options).numbered();
    let page = abilitysheet::clamp(counted, duel.ability_page);
    duel.ability_page = page;
    if digit == abilitysheet::PAGER {
        // The pager is drawn only where there is a second page, so a `0` on a
        // sheet of two rows is a key nothing on screen offered: it consumes
        // no frame and turns nothing.
        turn_the_page(duel);
        return abilitysheet::paged(counted);
    }
    let Some(at) = abilitysheet::option_of(counted, page, digit) else {
        return false;
    };
    take_sheet_row(duel, head + at);
    true
}

/// The row a press landed on: armed by the first press, sent by the second.
///
/// The digit and the click share it, because they are the same press — the
/// keycap is what the digit is drawn on, and a row whose click armed while
/// its digit sent would be two controls wearing one number.
pub(super) fn take_sheet_row(duel: &mut Duel, at: usize) {
    let Some(object) = duel.ability_menu else {
        return;
    };
    let Some(option) = abilities_of(duel, object).and_then(|o| o.get(at).cloned()) else {
        return;
    };
    duel.ability_pick = at;
    let armed = duel.armed.as_ref().is_some_and(|a| {
        a.object == object
            && match &a.deed {
                Deed::Ability(action) => *action == option.action,
                Deed::Run {
                    then: crate::RunEnd::Ability(index),
                    ..
                } => {
                    option.action
                        == PlayerAction::ActivateAbility {
                            source: object,
                            ability_index: *index,
                        }
                }
                _ => false,
            }
    });
    match abilitysheet::press(option.mana || option.tap_only, armed) {
        // Through `fire_armed` and not `duel.submit`, so the deed is
        // re-resolved against the current `LegalActions` exactly as the
        // confirm key and the second tap on the card already do.
        abilitysheet::Press::Fire => fire_armed(duel),
        abilitysheet::Press::Send | abilitysheet::Press::Arm => {
            if arm_ability(duel, object, &option) {
                duel.ability_menu = None;
            }
        }
    }
}

/// The tenth row: the next page of the ability sheet, wrapping at the end.
///
/// The cursor goes to the top of the new page rather than staying where it
/// was, because the page and the cursor are two ways of saying the same
/// thing — [`ability_menu_keys`] derives the page from the cursor when the
/// arrow keys walk off the end of one, and this is the same equality read the
/// other way round.
///
/// Wrapping rather than stopping: the pager is one key and there is no second
/// one for going back, so a player who overshoots gets there by pressing it
/// again.
pub(super) fn turn_the_page(duel: &mut Duel) {
    let Some(object) = duel.ability_menu else {
        return;
    };
    let Some(options) = abilities_of(duel, object) else {
        return;
    };
    let (head, counted) = crate::abilities::Split::of(&options).numbered();
    if !abilitysheet::paged(counted) {
        return;
    }
    let page = abilitysheet::clamp(counted, duel.ability_page);
    duel.ability_page = abilitysheet::turn(counted, page);
    duel.ability_pick = head + abilitysheet::rows(counted, duel.ability_page).start;
}

/// The card cursor, and the key that acts on what it is over. Returns whether
/// it consumed the frame.
pub(super) fn move_the_cursor(fired: Fired, duel: &mut Duel) -> bool {
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
    if fired.has(Action::ActivateGroup)
        && let Some(object) = duel.hovered
    {
        activate(duel, object, true);
        return true;
    }
    false
}

/// While an arrangement holds a card, the cursor keys move it — one place
/// along its pile, or to the end of the pile above or below — and reach
/// nothing else.
///
/// A held card is the one moment those keys have a better meaning than
/// walking the table behind the sheet: the focus keys already walk the
/// cards, and the tick already takes one up and puts it down in front of
/// another, so what a keyboard was missing is the small move a pointer
/// makes by tapping the neighbour. With nothing held they are the table's
/// again. Returns whether it consumed the frame.
pub(super) fn arrange_keys(fired: Fired, duel: &mut Duel) -> bool {
    if !duel.browser.answers_here(duel.interaction.as_ref()) {
        return false;
    }
    let Some(i) = duel.interaction.as_mut() else {
        return false;
    };
    if i.arrangement().and_then(Arrangement::held).is_none() {
        return false;
    }
    let mut consumed = false;
    for (action, nudge) in [
        (Action::CursorLeft, Nudge::Earlier),
        (Action::CursorRight, Nudge::Later),
        (Action::CursorUp, Nudge::PrevRow),
        (Action::CursorDown, Nudge::NextRow),
    ] {
        if fired.has(action) {
            i.nudge(nudge);
            consumed = true;
        }
    }
    consumed
}

/// Combat: where the next declaration points, and the answer that declares
/// nothing. Returns whether it consumed the frame.
pub(super) fn aim_and_declare(fired: Fired, duel: &mut Duel) -> bool {
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

/// The dialog's own keyboard, for the two keys that meant something else.
///
/// `docs/redesign-proposal.md` §6 is the spec: "Tab moves through rows, Space
/// toggles, Enter confirms". [`Action::CombatFocusNext`] was already the first
/// of those — its own doc says walking a `Mode::Objects` offer is the same
/// gesture as aiming at a blocker — and the other two both belonged to
/// something else.
///
/// [`Action::Confirm`] means "I am done here" *and* "pass priority", which is
/// right in every window but two, and this is the first of them: a
/// `ChooseCards { min: 0 }` arrives while a player is passing priority through
/// a stack of triggers, and the next press in that rhythm answered it with
/// "nothing found" — silently, with no trace and no undo on the wire. A Solemn
/// Simulacrum's search for a basic land was thrown away that way twice, and
/// the land count never moved. The second window is the combat declaration,
/// which is not a sheet and so is guarded where the key is read instead; see
/// [`committed_answer`].
///
/// [`Action::Primary`] is Enter, and it did reach confirm — but only as
/// [`the_click`]'s *third* branch, behind the card under the pointer. A card
/// under the pointer is the ordinary state of a table with a dialog standing
/// over it, so the key §6 gives the dialog was the one key the dialog was
/// least likely to get: the pointer resting on the permanent whose ability
/// asked the question is enough to take it.
///
/// So while the dialog is the surface holding the question, both keys belong
/// to it. The confirm key ticks the focused row instead of sending — a stray
/// press then does something the player can see and take back — and the
/// primary key sends. Neither reaches the table behind the sheet, which is
/// the whole point: `answers_here` is only true when the answer is *not* on
/// the table, because [`baylee_client_core::browser::Browser::follow`] opens
/// the sheet for a choice exactly when nothing on the table can answer it.
///
/// Returns whether it consumed the frame.
pub(super) fn browser_answer_keys(fired: Fired, duel: &mut Duel) -> bool {
    if !duel.browser.answers_here(duel.interaction.as_ref()) {
        return false;
    }
    if fired.has(Action::Primary) {
        // Consumes the frame even when the answer is not complete — a `min`
        // not reached yet, so `confirm` says no. The alternative is Enter
        // falling through to the hovered card, which is the defect this
        // branch exists to close, and it would fire on exactly the presses a
        // player makes while still building the answer.
        if let Some(action) = committed_answer(duel) {
            duel.submit(action);
        }
        return true;
    }
    if !fired.has(Action::Confirm) {
        return false;
    }
    // Consumes the frame even when the focus stands on nothing selectable:
    // the point is that this key does not reach `confirm` while the dialog
    // is up, and a `Rejected` that fell through would reach it.
    if let Some(i) = duel.interaction.as_mut() {
        i.toggle_focused();
    }
    true
}
