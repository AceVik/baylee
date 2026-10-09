//! The keyboard: the sign-in and table screens' text fields, the
//! builder's chords, and the arrow keys over the saved gateways.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// Types into the sign-in form from a keyboard the client itself reads.
///
/// Skipped entirely where [`SoftKeyboard`] owns the typing: the browser's
/// input has focus, so the canvas sees nothing anyway, and anything it did see
/// would be entered twice.
// Three screens' worth of chords in one function, each a flat `match` read top
// to bottom. Splitting it by screen would mean three copies of the modifier
// arithmetic above them, which is the thing that must not drift.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub(super) fn keyboard(
    mut keys: MessageReader<KeyboardInput>,
    codes: Res<ButtonInput<KeyCode>>,
    logical: Option<Res<ButtonInput<Key>>>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
    mut paste: Local<Option<super::editing::Paste>>,
    mut form_paste: Local<Option<super::editing::FormPaste>>,
    entrance: Res<super::entrance::Entrance>,
    journey: Option<Res<crate::arrival::Journey>>,
    desk: Option<Res<crate::report::ReportDesk>>,
    // The `?` overlay and the dev gallery stand over the lobby and type
    // into fields of their own.
    holds: Option<Res<crate::shellkit::KitHolds>>,
) {
    if journey.as_ref().is_some_and(|j| j.active())
        || entrance.active()
        || desk.is_some_and(|desk| desk.holds_keyboard())
        || holds.is_some_and(|h| h.0)
    {
        keys.clear();
        return;
    }
    // The terms sheet answers its own keys (`front::terms::terms_keys`);
    // nothing under it takes one, except the account deletion's
    // confirmation that its Decline stood over it (below).
    if state.terms.up() && state.lobby.deleting_account().is_none() {
        keys.clear();
        return;
    }
    // The About sheet: Esc closes it, and nothing under it takes a key.
    if state.about_open {
        keys.clear();
        if codes.just_pressed(KeyCode::Escape) {
            state.about_open = false;
        }
        return;
    }
    if state.confirmation.is_some() {
        keys.clear();
        if codes.just_pressed(KeyCode::Escape) {
            state.confirmation = None;
        }
        return;
    }
    // A header popover is a menu: Esc closes it, and nothing under it
    // takes the key (`KEYBOARD.md` §2.5).
    if state.header_menu.is_some() && codes.just_pressed(KeyCode::Escape) {
        keys.clear();
        state.header_menu = None;
        return;
    }
    // A rebinding in progress takes every key, including the ones that mean
    // something everywhere else — a player who wants `Esc` on some other
    // action has to be able to press it. Escape and backspace are the two
    // exceptions, and they are what makes the row escapable at all.
    if let Some(action) = state.settings.capturing() {
        keys.clear();
        if codes.just_pressed(KeyCode::Escape) {
            state.settings = SettingsPane::Open;
        } else if codes.any_just_pressed([KeyCode::Backspace, KeyCode::Delete]) {
            // Unbinding is a real answer: a pointer still reaches everything.
            prefs.edit().keymap.bind(action, vec![]);
            state.settings = SettingsPane::Open;
        } else if let Some(chord) = crate::keys::captured_for(action, &codes, logical.as_deref()) {
            prefs.edit().keymap.bind(action, vec![chord]);
            state.settings = SettingsPane::Open;
        }
        return;
    }
    // The account's deletion stands over the settings screen, and its
    // password box is the one thing there that is typed into: Enter sends,
    // Escape cancels.
    // A shortcut listening (Settings › Controls, `KEYBOARD.md` §5): the
    // next key is its chord — refused with the holder's name when another
    // live action has it, taken by the same key pressed again; Esc cancels.
    if let Some(action) = state.settings.capturing_shell() {
        for key in keys.read() {
            if !key.state.is_pressed() || key.repeat {
                continue;
            }
            if key.logical_key == Key::Escape {
                state.settings = SettingsPane::Open;
                state.settings_view.refused = None;
                break;
            }
            let press = crate::shellkit::keys::press_of(key, Some(&codes));
            let mac = crate::shellkit::keys::mac();
            let Some(chord) = baylee_client_core::shellkeys::ShellChord::captured(&press, mac)
            else {
                continue;
            };
            let again = state
                .settings_view
                .refused
                .as_ref()
                .is_some_and(|(a, c, _)| *a == action && *c == chord);
            let result = if again {
                prefs.edit().shell_keys.take(action, chord.clone())
            } else {
                // Asked of a copy, so a refusal writes nothing back.
                let mut map = prefs.all().shell_keys.clone();
                match map.bind(action, chord.clone()) {
                    Ok(()) => {
                        prefs.edit().shell_keys = map;
                        Ok(())
                    }
                    Err(why) => Err(why),
                }
            };
            match result {
                Ok(()) => {
                    state.settings_view.refused = None;
                    state.settings = SettingsPane::Open;
                }
                Err(why) => state.settings_view.refused = Some((action, chord, why)),
            }
            break;
        }
        keys.clear();
        return;
    }
    if state.lobby.deleting_account().is_some() {
        if !keys.is_empty() {
            text_field_keys(
                &mut keys,
                &codes,
                &mut state,
                &mut prefs,
                &mailbox,
                false,
                clipboard.as_deref_mut(),
                &mut form_paste,
            );
        }
        return;
    }
    if state.settings.is_open() {
        // A profile's sheet: Esc puts it away (in a box, Esc leaves the box
        // first).
        if codes.just_pressed(KeyCode::Escape)
            && state.settings_view.profile_sheet
            && !state.seat.typing()
        {
            keys.clear();
            state.settings_view.profile_sheet = false;
            return;
        }
        // Opened from the front door, no header stands over it: Esc is its
        // way back (signed in, the nav is).
        if codes.just_pressed(KeyCode::Escape)
            && state.lobby.token().is_none()
            && !state.lobby.offline()
            && !state.seat.typing()
        {
            keys.clear();
            state.settings = SettingsPane::Closed;
            return;
        }
        // The seat panel's boxes are the only ones on the settings screen.
        crate::seatpanel::keys(&mut keys, &codes, &mut state, clipboard.as_deref_mut());
        return;
    }
    // A deck's history, over Decks or the builder (`DESIGN` §C.3): its
    // arrows and its Esc ladder; nothing behind it hears a key.
    if state.library_open() && state.menu.is_none() && state.confirmation.is_none() {
        keys.clear();
        let request = super::history::keys(&codes, &mut state);
        dispatch(&mut state, &mailbox, request);
        return;
    }
    // The Decks and Play screens' menus and sheets (WP2, WP3): Esc closes
    // the innermost one and nothing behind it hears the key (`KEYBOARD.md`
    // §2.5); a room is never left by Esc.
    if matches!(state.lobby.screen(), Screen::Table) && shell_layer_keys(&codes, &mut state) {
        keys.clear();
        return;
    }
    // Before the platform's typing: the import and export dialogs take no
    // text, only chords, and answer them the same everywhere.
    if matches!(state.lobby.screen(), Screen::Build)
        && super::editing::transfer_keys(&codes, &mut state, &mut scrolled)
    {
        keys.clear();
        return;
    }
    if SoftKeyboard::owns_typing() {
        keys.clear();
        return;
    }
    if matches!(state.lobby.screen(), Screen::Build) || state.lobby.builder().picker().is_some() {
        // While a row of the filter builder holds the caret, every key goes
        // there and none of them into the deck builder's own boxes. The two
        // are editors of one string and only one may be typed into, which is
        // the rule `filterdialog`'s header states — and the rows have a real
        // caret where the search box, being a plain `String`, has none, so
        // they answer the whole chord set rather than this screen's three.
        if state
            .lobby
            .builder()
            .panel()
            .is_some_and(|it| it.typing().is_some())
        {
            let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
            let word = codes.any_pressed([
                KeyCode::AltLeft,
                KeyCode::AltRight,
                KeyCode::ControlLeft,
                KeyCode::ControlRight,
            ]);
            let line = codes.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
            let command = line || codes.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
            let reach = if line {
                Reach::Line
            } else if word {
                Reach::Word
            } else {
                Reach::Char
            };
            for key in keys.read() {
                if !key.state.is_pressed() {
                    continue;
                }
                let deck = state.lobby.builder_mut();
                match &key.logical_key {
                    Key::Backspace => deck.in_panel(|it| {
                        it.pop_typed();
                    }),
                    Key::Delete => deck.in_panel(FilterPanel::delete_typed_forward),
                    Key::ArrowLeft => {
                        deck.in_panel(|it| it.move_typing_caret(reach, Dir::Left, shift));
                    }
                    Key::ArrowRight => {
                        deck.in_panel(|it| it.move_typing_caret(reach, Dir::Right, shift));
                    }
                    Key::Home => {
                        deck.in_panel(|it| it.move_typing_caret(Reach::Line, Dir::Left, shift));
                    }
                    Key::End => {
                        deck.in_panel(|it| it.move_typing_caret(Reach::Line, Dir::Right, shift));
                    }
                    // Enter hands the row back and leaves the panel open,
                    // the same way it does in the zone browser.
                    Key::Enter | Key::Escape => deck.in_panel(FilterPanel::stop_typing),
                    Key::Character(text) if command => {
                        if text.eq_ignore_ascii_case("a") {
                            deck.in_panel(FilterPanel::select_all_typed);
                        }
                    }
                    _ => {
                        if let Some(text) = key.text.as_ref() {
                            deck.type_into_panel(text);
                        }
                    }
                }
                // A different filter is a different list; the row that was
                // halfway down it is not in this one.
                scrolled.set(List::Pool, 0.0);
            }
            return;
        }
        super::editing::builder_keys(
            &mut keys,
            &codes,
            &mut state,
            &mut scrolled,
            clipboard.as_deref_mut(),
            &mut paste,
        );
        return;
    }
    // The table screen has two fields of its own — the search box and the
    // room password — and before this it had no way to type into either.
    let table = matches!(state.lobby.screen(), Screen::Table);
    if !matches!(state.lobby.screen(), Screen::SignIn { .. }) && !table {
        keys.clear();
        return;
    }
    // A paste the clipboard answered after the frame that asked for it.
    if form_paste.is_some() {
        super::editing::land_form_paste(&mut state, &mut form_paste);
    }
    // Nothing was pressed, so nothing is touched. `ResMut` is what the lobby's
    // retained tree watches — change detection stands in for a revision
    // struct — and merely taking `&mut` out of one marks it changed, so a
    // handler that reached for the state on every quiet frame would rebuild
    // the whole screen sixty times a second.
    if keys.is_empty() {
        return;
    }
    text_field_keys(
        &mut keys,
        &codes,
        &mut state,
        &mut prefs,
        &mailbox,
        table,
        clipboard.as_deref_mut(),
        &mut form_paste,
    );
}

/// The table screen's menus and sheets (WP2, WP3): `Esc` closes the
/// innermost open thing, one per press (`KEYBOARD.md` §2.5). Answers
/// whether the keys are the layer's (nothing under it hears them); the
/// Create-table sheet keeps its two fields typing.
fn shell_layer_keys(codes: &ButtonInput<KeyCode>, state: &mut ResMut<LobbyState>) -> bool {
    let escape = codes.just_pressed(KeyCode::Escape);
    if state.menu.is_some() {
        if escape {
            state.menu = None;
        }
        return true;
    }
    if state.chair_sheet.is_some() {
        if escape {
            state.chair_sheet = None;
        }
        return true;
    }
    if state.decks.preview.is_some() {
        if escape {
            state.decks.preview = None;
        }
        return true;
    }
    if matches!(
        state.lobby.library().page,
        Some(client_core::lobby::library::Page::History(_))
    ) {
        if escape && !state.lobby.library().loading {
            state.lobby.close_library();
        }
        return true;
    }
    if state.play.picker {
        if escape {
            state.play.picker = false;
        }
        return true;
    }
    if state.play.sheet.is_some() && escape {
        state.play.sheet = None;
        state.lobby.focus_on(Field::Search);
        return true;
    }
    false
}

/// Whether the focused field is one the table screen draws now: the
/// Create-table sheet's name and password only while it is up, a room's
/// starting-board search only in its open drawer, a locked row's password
/// only on the list. A field that is not drawn takes no keys.
pub(super) fn field_drawn(state: &LobbyState) -> bool {
    let sheet = state.play.sheet.is_some();
    let room = state.lobby.awaiting().is_some() && !state.room_away;
    match state.lobby.focus() {
        Field::RoomName => sheet,
        Field::RoomPassword => sheet || !room,
        Field::RoomBoard(seat) => room && state.room_setup_seat == Some(seat),
        Field::RoomCounter => room && state.room_setup_seat.is_some(),
        Field::Search | Field::DeckSearch => !room && !sheet,
        _ => true,
    }
}

/// Chooses a saved gateway, which turns the front door to the account form.
///
/// One door for the pointer and the keyboard, so that choosing by Enter on a
/// row does everything a tap on it does.
pub(super) fn choose_gateway(
    state: &mut LobbyState,
    prefs: &mut ResMut<crate::prefs::Prefs>,
    mailbox: &Mailbox,
    index: usize,
) {
    if state.select_gateway(index) {
        // Bookkeeping, not a visible preference: `poll` follows the token
        // the same way (and a detach without an account is a no-op).
        prefs.bypass_change_detection().detach();
        http::probe_registration(state, mailbox);
        // Asked again: the answer in the list may be from before the gateway
        // was upgraded, and choosing it is the moment that answer is about to
        // matter.
        let url = state.gateway.clone();
        state.probes.insert(url.clone(), Probe::Asking);
        http::probe_gateway(url, mailbox);
    }
}

/// The sign-in and table screens' own text fields.
///
/// Its own function because it is the whole of what a browser's `<input>`
/// would answer for free, written out for a canvas that has none: a caret
/// that moves by character, word and line, a selection that shift extends,
/// Delete as well as Backspace, and select-all.
/// ⌘V in a sign-in field: a closed-beta key (#317) is pasted far more often
/// than typed. A clipboard that answers later lands its text in the same
/// field through `land_form_paste`, if the caret is still there.
fn paste_into_form(
    state: &mut LobbyState,
    clipboard: &mut bevy::clipboard::Clipboard,
    paste: &mut Option<super::editing::FormPaste>,
) {
    let mut read = clipboard.fetch_text();
    match read.poll_result() {
        // `insert` drops the line breaks a copied key brings along.
        Some(Ok(text)) => state.lobby.insert(&text),
        Some(Err(_)) => {}
        None => {
            *paste = Some(super::editing::FormPaste {
                field: state.lobby.focus(),
                epoch: state.lobby.focus_epoch(),
                read,
            });
        }
    }
}

/// The room's optional editors are shell state. Tab only visits fields the
/// room currently draws; the other forms keep the core's normal field ring.
fn cycle_form_focus(state: &mut LobbyState, direction: Tab) {
    // The Create-table sheet: its name and its password, the only two
    // fields it draws (offline, the name alone).
    if state.play.sheet.is_some() {
        let next = if state.lobby.focus() == Field::RoomName && !state.lobby.offline() {
            Field::RoomPassword
        } else {
            Field::RoomName
        };
        state.lobby.focus_on(next);
        state.lobby.select_all();
        return;
    }
    let lobby = &mut state.lobby;
    if lobby.screen() != &Screen::Table
        || lobby.awaiting().is_none()
        || lobby.deleting_account().is_some()
    {
        lobby.cycle_focus(direction);
        return;
    }
    let Some(room) = lobby
        .games()
        .iter()
        .find(|g| lobby.awaiting().is_some_and(|h| h.game_id == g.id))
        .filter(|g| g.yours)
    else {
        return;
    };
    // The room draws no name or password box (Edit rules opens the sheet
    // for them, WP2): only an open drawer's board search and counter.
    let mut fields = Vec::new();
    if let Some(seat) = state
        .room_setup_seat
        .filter(|s| usize::from(*s) < room.seats.len())
    {
        fields.push(Field::RoomBoard(seat));
        if state.room_card_edit.is_some_and(|(s, _)| s == seat) {
            fields.push(Field::RoomCounter);
        }
    }
    if fields.is_empty() {
        return;
    }
    let at = fields
        .iter()
        .position(|f| *f == lobby.focus())
        .unwrap_or(fields.len() - 1);
    let next = match direction {
        Tab::Next => (at + 1) % fields.len(),
        Tab::Back => (at + fields.len() - 1) % fields.len(),
    };
    lobby.focus_on(fields[next]);
    lobby.select_all();
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the clipboard rides along; one flat match
fn text_field_keys(
    keys: &mut MessageReader<KeyboardInput>,
    codes: &ButtonInput<KeyCode>,
    state: &mut ResMut<LobbyState>,
    prefs: &mut ResMut<crate::prefs::Prefs>,
    mailbox: &Mailbox,
    table: bool,
    mut clipboard: Option<&mut bevy::clipboard::Clipboard>,
    paste: &mut Option<super::editing::FormPaste>,
) {
    // The three modifiers a text field reads, and they are read once for the
    // whole batch because a key event carries no modifier state of its own.
    // Which one means what is the platform's convention and not a preference:
    // shift extends a selection everywhere, and the two reaches past a single
    // character are ⌥/Ctrl for a word and ⌘/Home-End for the line.
    let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let word = codes.any_pressed([
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    let line = codes.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    // ⌘A and Ctrl+A. The same chord has to keep its "a" out of the field,
    // which is why it is answered before the text arm below ever sees it.
    let command = line || codes.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let reach = if line {
        Reach::Line
    } else if word {
        Reach::Word
    } else {
        Reach::Char
    };
    for key in keys.read() {
        if !key.state.is_pressed() {
            continue;
        }
        // The front door's Esc goes back a face whatever has the focus
        // (`KEYBOARD.md` §7.2): the create and guest faces to sign in, sign
        // in to the gateways.
        if !table && key.logical_key == Key::Escape && state.lobby.deleting_account().is_none() {
            if state.front_menu {
                state.front_menu = false;
            } else if !state.lobby.back_to_sign_in() {
                if state.lobby.gateway_chosen() {
                    state.leave_gateway();
                } else if state.gateway_cursor.is_some() {
                    state.gateway_cursor = None;
                }
            }
            continue;
        }
        // Tab is the kit's focus walker on the front door and on the table
        // screen (Play, Decks, the room and their sheets; `front::keys`,
        // `focusing`); only the account deletion's password box, a lobby
        // field with no stop, still moves the caret with it.
        if key.logical_key == Key::Tab {
            if state.lobby.deleting_account().is_some() {
                cycle_form_focus(state, if shift { Tab::Back } else { Tab::Next });
            }
            continue;
        }
        // Everything else needs the caret in a field this screen is drawing.
        if !state.lobby.typing_here() || (table && !field_drawn(state)) {
            continue;
        }
        match &key.logical_key {
            Key::Backspace => state.lobby.backspace(),
            Key::Delete => state.lobby.delete_forward(),
            // A caret that moves changes only the field it stands in, and
            // `ui::retrace_runs` redraws that field's runs in place: marking
            // the whole state changed for it rebuilt every node on the
            // screen per arrow key (§10 #3 of the shell design).
            Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End => {
                let (reach, dir) = match &key.logical_key {
                    Key::ArrowLeft => (reach, Dir::Left),
                    Key::ArrowRight => (reach, Dir::Right),
                    Key::Home => (Reach::Line, Dir::Left),
                    _ => (Reach::Line, Dir::Right),
                };
                state
                    .bypass_change_detection()
                    .lobby
                    .move_caret(reach, dir, shift);
            }
            Key::Escape if state.lobby.deleting_account().is_some() => {
                state.lobby.cancel_account_deletion();
            }
            Key::Enter if state.lobby.deleting_account().is_some() => {
                let request = state.lobby.submit();
                dispatch(state, mailbox, request);
            }
            // Up and down walk the saved gateways, which the one-line address
            // field has no use for.
            Key::ArrowUp | Key::ArrowDown if !table && !state.lobby.gateway_chosen() => {
                state.move_gateway_cursor(matches!(key.logical_key, Key::ArrowDown));
            }
            // In the gateway form Enter chooses the row the arrows are on,
            // and otherwise is the Save button beside the address: that face
            // has no sign-in form to submit.
            Key::Enter if !table && !state.lobby.gateway_chosen() => {
                if let Some(index) = state.gateway_cursor {
                    choose_gateway(state, prefs, mailbox, index);
                } else if let Some(url) = state.check_gateway() {
                    http::probe_gateway(url, mailbox);
                }
            }
            Key::Enter if table && matches!(state.lobby.focus(), Field::RoomBoard(_)) => {
                if let Field::RoomBoard(seat) = state.lobby.focus()
                    && let Some(slot) = state.lobby.room_matches(seat).first().copied()
                {
                    state.lobby.room_add_card(seat, slot);
                }
            }
            Key::Escape if table && matches!(state.lobby.focus(), Field::RoomBoard(_)) => {
                let focus = state.lobby.focus();
                state.lobby.set_field(focus, "");
            }
            // A search box with text: Esc clears it (HTML's search input).
            Key::Escape
                if table
                    && matches!(state.lobby.focus(), Field::Search | Field::DeckSearch)
                    && !state.lobby.field(state.lobby.focus()).is_empty() =>
            {
                let focus = state.lobby.focus();
                state.lobby.set_field(focus, "");
                if focus == Field::Search {
                    let request = state.lobby.search_again();
                    dispatch(state, mailbox, request);
                }
            }
            // The Create-table sheet's default button (`KEYBOARD.md` §7.4).
            Key::Enter if table && state.play.sheet.is_some() => {
                if let Some((draft, editing)) = state.play.sheet.take() {
                    let request = if editing {
                        state.lobby.apply_table(&draft)
                    } else {
                        state.lobby.open_table(&draft)
                    };
                    if request.is_none() {
                        state.play.sheet = Some((draft, editing));
                    } else {
                        state.lobby.focus_on(Field::Search);
                    }
                    dispatch(state, mailbox, request);
                }
            }
            // The shelf's search filters as it is typed; Enter has nothing
            // more to ask.
            Key::Enter if table && state.lobby.focus() == Field::DeckSearch => {}
            Key::Enter => {
                let request = if table {
                    state.lobby.search_again()
                } else {
                    state.lobby.submit()
                };
                dispatch(state, mailbox, request);
            }
            Key::Character(text) if command => {
                if text.eq_ignore_ascii_case("a") {
                    state.lobby.select_all();
                } else if text.eq_ignore_ascii_case("v")
                    && let Some(cb) = clipboard.as_deref_mut()
                {
                    paste_into_form(state, cb, paste);
                }
            }
            // Everything else is text or nothing. `type_char` drops the
            // control characters Tab and Enter also produce.
            _ => {
                if let Some(text) = key.text.as_ref() {
                    // Caps Lock, inferred: a letter whose case disagrees
                    // with Shift (the platform reports the key, not the lock).
                    if let Some(letter) = text.chars().find(|c| c.is_alphabetic())
                        && letter.is_uppercase() != letter.is_lowercase()
                    {
                        let locked = letter.is_uppercase() != shift;
                        if state.caps_lock != locked {
                            state.caps_lock = locked;
                        }
                    }
                    // Typing is about the address, not the rows.
                    if state.gateway_cursor.is_some() {
                        state.gateway_cursor = None;
                    }
                    for ch in text.chars() {
                        state.lobby.type_char(ch);
                    }
                }
            }
        }
    }
}
