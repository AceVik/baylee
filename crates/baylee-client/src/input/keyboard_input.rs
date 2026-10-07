//! The keyboard system and the keys of a question: numbers, damage, types, arming.

#[allow(clippy::wildcard_imports)] // the input module's shared vocabulary
use super::*;

/// Keyboard handling: every key comes from the account's keymap.
///
/// The handler asks *actions*, never keys. That is what makes rebinding work
/// at all, and it also removed the `if !shift` guards that used to be sprayed
/// through here — `W` and `⇧W` are two chords, and telling them apart is the
/// keymap's job, not this function's.
#[allow(clippy::too_many_arguments)] // the eighth is the report form's claim on the keys
pub fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    logical: Option<Res<ButtonInput<Key>>>,
    mut typed: MessageReader<KeyboardInput>,
    mut duel: ResMut<Duel>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut settings: ResMut<crate::settings::ClientSettings>,
    mut had_keyboard: Local<bool>,
    (desk, mut arrangements): (
        Option<Res<crate::report::ReportDesk>>,
        Option<ResMut<crate::arrangement::ArrangementFrame>>,
    ),
) {
    // The report form, when it is up, has every key (#309).
    if desk.is_some_and(|desk| desk.holds_keyboard()) {
        typed.clear();
        return;
    }
    let fired = Fired::of_layout(&keys, logical.as_deref(), prefs.keymap());
    // The arrangement menu, while it stands, has every key too (DESIGN-v8
    // §2.3): the table hears nothing until it is shut.
    if duel.arrangement_menu.is_some()
        && let Some(frame) = arrangements.as_deref_mut()
    {
        let digits: Vec<u32> = typed
            .read()
            .filter(|event| event.state.is_pressed())
            .filter_map(|event| match &event.logical_key {
                Key::Character(s) => s.chars().find_map(|c| c.to_digit(10)),
                _ => None,
            })
            .collect();
        crate::arrangement::keys(fired, &digits, &mut duel, &mut settings, frame);
        return;
    }
    // The keystroke that opened the panel is not a keystroke for the box.
    // `G` opens the sheet on a frame where nothing here reads the message
    // queue, so the character is still standing in it when the box takes the
    // keyboard a moment later — and `browser_keys` would type it the first
    // time it looks. Advancing this reader past whatever is pending on the
    // frame the box *gains* the keyboard is the whole guard, and it is the
    // right shape for the click path too: a key pressed before the box was
    // clicked belongs to the table it was pressed over.
    //
    // A bool rather than `typing_epoch`, which counts re-seedings of the
    // platform's own field as well as focus and would drop a character every
    // time `Esc` emptied the box.
    let typing = duel.browser.is_typing();
    if typing && !*had_keyboard {
        typed.clear();
    }
    *had_keyboard = typing;
    // Before the quiet check, and not after it: a letter typed into the type
    // filter is usually bound to no action at all, so `Fired` is empty for
    // exactly the keys the box cares about most.
    if subtype_keys(
        fired,
        &mut typed,
        &mut duel,
        baylee_client_core::Lang::of(&settings.lang),
    ) {
        return;
    }
    // Same reason, and the same place in the order: a digit is bound to no
    // action, so `Fired` is empty for exactly the keys a number choice wants.
    if number_keys(&mut typed, &mut duel) {
        return;
    }
    if duel.granted_menu.open && sheet_digits(&mut typed, &mut duel) {
        return;
    }
    if crate::hud::granted_keys(fired, &mut duel) {
        return;
    }
    if damage_keys(fired, &mut duel) {
        return;
    }
    // And once more for the ability sheet, whose rows are sent by the digit
    // drawn on each of them. Same place in the order and the same reason: a
    // digit is bound to no action, so `Fired` is empty for exactly these
    // keys.
    if sheet_digits(&mut typed, &mut duel) {
        return;
    }
    // And the same again for the browser's filter box, which holds the
    // keyboard from the moment the panel opens until the player hands it back
    // — see `browser_takes_the_keyboard`. Only while it holds it, though: the
    // sheet can stand open for a whole turn, and a released box that went on
    // swallowing every keystroke would be the end of playing with the
    // graveyard visible.
    if browser_keys(fired, &keys, &mut typed, &mut duel) {
        return;
    }
    // `P` opens the arrangement menu, `Shift+P` takes the next arrangement:
    // after every text field has had its keys, so a `p` typed into the
    // browser's filter is a letter.
    if let Some(frame) = arrangements.as_deref_mut()
        && crate::arrangement::keys(fired, &[], &mut duel, &mut settings, frame)
    {
        return;
    }
    if fired.quiet() {
        return;
    }
    // A table the follow switch is still moving drops the confirm key: the
    // card under the finger moved, and the press was meant for the one that
    // was there (DESIGN-v8 §1.1). Only the follow's moves — a switch or a
    // chip the player pressed is theirs and governs nothing here.
    if duel.follow_settling && fired.has(Action::Confirm) {
        return;
    }
    // And the same for a table tearing (the owner's of 07.10.2026): the
    // keys that act on a card under the cursor wait until it has docked.
    if duel.tear.is_some()
        && (fired.has(Action::Confirm)
            || fired.has(Action::Primary)
            || fired.has(Action::ActivateCard)
            || fired.has(Action::ActivateGroup))
    {
        return;
    }
    // The keyboard's half of the same rule as the pointer's: any bound key
    // forgets a half-pressed concession.
    duel.concede_armed = false;
    look_around(fired, &mut duel, &mut settings, &mut prefs);
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
    // And the cast chooser for the same reason, after it: the two never stand
    // together, so the order between them is arbitrary and the rule is not.
    if cast_menu_keys(fired, &mut duel) {
        return;
    }
    if arrange_keys(fired, &mut duel) {
        return;
    }
    if move_the_cursor(fired, &mut duel) {
        return;
    }
    if aim_and_declare(fired, &mut duel) {
        return;
    }
    // Ahead of the primary key and of the straight answers, which is the
    // whole of its precedence: while the dialog is the surface holding the
    // question it owns both of §6's keys, and `the_click` would otherwise
    // spend Enter on whatever card the pointer happens to be resting on
    // behind the sheet.
    if browser_answer_keys(fired, &mut duel) {
        return;
    }
    if the_click(fired, &mut duel, &mut prefs) {
        return;
    }
    if duel.interaction.is_some() {
        answer_the_question(fired, &mut duel, &mut prefs);
    } else {
        between_questions(fired, &mut duel);
    }
}

/// `Esc` between two questions, innermost first: a reveal another seat
/// made, then a visit.
fn between_questions(fired: Fired, duel: &mut Duel) {
    if fired.has(Action::Cancel) && duel.reveals.current().is_some() {
        // A reveal stands between two questions as well, and is put away
        // first, as it is in front of one (`answer_the_question`).
        duel.reveals.dismiss();
    } else if fired.has(Action::Cancel) && duel.visiting.is_some() {
        // Between two questions there is nothing to take back: `Esc` is
        // "back" to the home shot (DESIGN-v7 §2.4).
        navigate_home(duel);
    }
}

/// Camera, phase rail, fast-forward and display toggles — everything that
/// changes what the player sees rather than what the game hears.
pub(super) fn look_around(
    fired: Fired,
    duel: &mut Duel,
    settings: &mut crate::settings::ClientSettings,
    prefs: &mut crate::prefs::Prefs,
) {
    if fired.has(Action::FocusNextSeat) {
        visit_next_seat(duel, true);
    }
    if fired.has(Action::FocusPrevSeat) {
        visit_next_seat(duel, false);
    }
    if fired.has(Action::FocusHome) {
        navigate_home(duel);
    }
    if fired.has(Action::HandDrawer) {
        duel.toggle_hand_drawer();
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
    if fired.has(Action::ToggleTextView) {
        // The modifier key shows the card face while held; this is the latch,
        // for players who read text rather than art. A preference, not a mode,
        // so it is remembered.
        settings.prefer_text_view = !settings.prefer_text_view;
        settings.save();
    }
    if fired.has(Action::ToggleBrowser) {
        // A latch rather than a held key, for the same reason a tap on a pile
        // opens one: reading a graveyard is not a glance, and a held key is
        // not a gesture a phone has. The tab it was last left on is kept, so
        // a player checking their own yard twice does not re-pick it.
        duel.browser.toggle_by_hand();
    }
    if fired.has(Action::ToggleLog) {
        // A latch for the browser's reason: a log is read, not glanced at.
        duel.log_open = !duel.log_open;
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
///
/// The doc block was above `browser_softkeys` for as long as it existed —
/// spliced onto the next item's own, so `cargo doc` printed it over the soft
/// keyboard's. Same family as the `still_gliding` splice: the anchor is the
/// closing brace before a block, never the `///` after it.
fn number_keys(typed: &mut MessageReader<KeyboardInput>, duel: &mut Duel) -> bool {
    if !duel
        .interaction
        .as_ref()
        .is_some_and(Interaction::edits_number)
    {
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

/// Explicit resolving dialogs own cursor navigation; numeric editors own digits.
pub(super) fn damage_keys(fired: Fired, duel: &mut Duel) -> bool {
    let Some(i) = duel.interaction.as_mut().filter(|i| {
        matches!(
            i.decision_id(),
            Some(
                baylee_client_core::interaction::DecisionId::Damage(_)
                    | baylee_client_core::interaction::DecisionId::Source(_)
                    | baylee_client_core::interaction::DecisionId::Mana(_)
                    | baylee_client_core::interaction::DecisionId::Text { .. }
            )
        )
    }) else {
        return false;
    };
    let count = match i.prompt() {
        Prompt::TextReplacement { .. } => 10,
        Prompt::ChooseManaAbility { options, .. } => options.len(),
        Prompt::ChooseDamageSource { options } => options.len(),
        Prompt::ChooseDamageEffect { options, .. } => options.len(),
        Prompt::AllocatePrevention { damage, .. } => damage.len(),
        _ => 0,
    };
    let step = i32::from(fired.has(Action::CursorDown)) - i32::from(fired.has(Action::CursorUp));
    if step == 0 || count == 0 {
        return false;
    }
    let at = i.chosen_index().unwrap_or(0);
    let next = if step > 0 {
        (at + 1) % count
    } else {
        at.checked_sub(1).unwrap_or(count - 1)
    };
    i.choose_index(next);
    duel.target_page = next / crate::choices::DAMAGE_PAGE_SIZE;
    true
}

/// The same localized, sorted creature-type rows the renderer shows.
fn visible_types(duel: &Duel, lang: baylee_client_core::Lang) -> Vec<crate::choices::ChoiceOption> {
    duel.interaction
        .as_ref()
        .map(Interaction::prompt)
        .and_then(|p| {
            // Filtering must use the renderer's language too: a German
            // prefix must select the same rows with keyboard and pointer.
            crate::choices::options(
                &p,
                lang,
                duel.statics.as_ref(),
                &duel.subtype_filter,
                crate::choices::FaceNames::default(),
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
fn subtype_keys(
    fired: Fired,
    typed: &mut MessageReader<KeyboardInput>,
    duel: &mut Duel,
    lang: baylee_client_core::Lang,
) -> bool {
    if !matches!(
        duel.interaction.as_ref().map(Interaction::prompt),
        Some(Prompt::ChooseSubtype { .. } | Prompt::ChooseCardName)
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
    let rows = visible_types(duel, lang);
    if duel.subtype_filter != before {
        // The highlight follows the list. A row that has just been filtered
        // away must not stay picked, or Confirm answers a type the player can
        // no longer see.
        if let Some(first) = rows.first().map(|row| row.index)
            && let Some(i) = duel.interaction.as_mut()
        {
            crate::choices::pick(i, first);
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
    let picked = duel.interaction.as_ref().and_then(crate::choices::picked);
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
            crate::choices::pick(i, row.index);
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

/// Takes back what is armed, and the chosen way with it.
///
/// The two are one decision: [`take_cast_row`] answers this client's question
/// and arms the run in the same press, so an `Esc` that left the answer
/// behind would take back the taps and keep the choice. It is not merely
/// untidy — the answer is spent by the *engine's* `ChooseCastMode`, and that
/// question is still reachable by another route (the free alternative the
/// engine offers with an empty pool needs no run at all), so a forgotten one
/// would be applied to a cast the player made some other way.
pub fn disarm(duel: &mut Duel) {
    let Some(armed) = duel.armed.take() else {
        return;
    };
    if duel
        .cast_answer
        .is_some_and(|(card, _)| card == armed.object)
    {
        duel.cast_answer = None;
    }
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
        disarm(duel);
        return true;
    }
    if fired.has(Action::Primary) || fired.has(Action::Confirm) || fired.has(Action::ActivateCard) {
        fire_armed(duel);
        return true;
    }
    false
}

/// The open ability sheet: the cursor keys walk it, the primary key or
/// confirm takes the row, cancel puts the sheet away. Returns whether it
/// consumed the frame.
///
/// The list is rebuilt from `LegalActions` here rather than trusted from the
/// frame it was drawn on — the same rule the pointer path follows, and for
/// the same reason: the engine may have withdrawn the ability since.
///
/// The cursor is two-dimensional here and on no other sheet, because this one
/// has two directions in it: a centred **row** of mana pips above a column of
/// written rows. Up and down walk everything there is, as they always have;
/// left and right are the pip strip and jump to it from wherever the cursor
/// stands. [`abilitysheet::step_down`] and [`abilitysheet::step_along`] are
/// the arithmetic and carry the reasons.
///
/// The digits are not here. They are read as *characters* by
/// [`sheet_digits`], the way the subtype filter and a number entry are,
/// because a digit is bound to no [`Action`] and nine new ones would be nine
/// rows in every player's keymap for a key whose whole meaning is the number
/// printed on it.
pub fn ability_menu_keys(fired: Fired, duel: &mut Duel) -> bool {
    let Some(object) = duel.ability_menu else {
        return false;
    };
    let Some(options) = abilities_of(duel, object).filter(|o| o.len() > 1) else {
        // Nothing left to choose: the sheet is stale, and holding it open
        // would keep the keyboard hostage.
        duel.ability_menu = None;
        return false;
    };
    let split = crate::abilities::Split::of(&options);
    // The list can shrink under a page that was valid when it was turned to.
    duel.ability_page = abilitysheet::clamp(split.numbered().1, duel.ability_page);
    if fired.has(Action::Cancel) {
        // One step back, not all the way out. A sub-bubble was opened by a
        // press on a row of a sheet that is still the answer to the click,
        // and there is no undo in this client anywhere else either — `Esc`
        // takes back the last thing a player said, which here is "that tap".
        if duel.asking_tap().is_some() {
            duel.ability_tap = None;
            duel.ability_pick = 0;
            duel.ability_page = 0;
        } else {
            duel.ability_menu = None;
        }
        return true;
    }
    let down = i32::from(fired.has(Action::CursorDown)) - i32::from(fired.has(Action::CursorUp));
    let along =
        i32::from(fired.has(Action::CursorRight)) - i32::from(fired.has(Action::CursorLeft));
    if down != 0 || along != 0 {
        // A frame carrying both is answered by the strip, because that is the
        // gesture that names a destination rather than a direction.
        duel.ability_pick = if along == 0 {
            abilitysheet::step_down(options.len(), duel.ability_pick, down)
        } else {
            abilitysheet::step_along(options.len(), split.pips, duel.ability_pick, along)
        };
        // The cursor walks the whole list and the sheet shows nine rows of
        // it, so walking off the end of a page turns it. Deriving the page
        // from the cursor rather than moving them separately is what stops
        // the highlight from being on a row that is not drawn — and the pips
        // are drawn on every page, so a cursor on one leaves the page where
        // it was rather than sending it back to the first.
        duel.ability_page = if duel.ability_pick < split.pips && split.rows > 0 {
            duel.ability_page
        } else {
            split.page_of(duel.ability_pick)
        };
        return true;
    }
    if fired.has(Action::Primary) || fired.has(Action::Confirm) || fired.has(Action::ActivateCard) {
        if let Some(option) = options.get(duel.ability_pick).cloned()
            && arm_ability(duel, object, &option)
        {
            duel.ability_menu = None;
        }
        return true;
    }
    false
}
