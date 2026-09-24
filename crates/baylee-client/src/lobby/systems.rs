//! Bevy systems: the mailbox pump, the seat watch, text entry and the
//! pointer.
//!
//! Nothing here decides anything; each system turns an input into a
//! [`LobbyEvent`] and hands it to [`baylee_client_core::lobby`].

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

// --------------------------------------------------------------- systems

/// Drains the mailbox, advances the lobby, and takes the seat it is granted.
#[allow(clippy::too_many_lines)] // request outcomes and seat handover share the mailbox
pub(super) fn poll(
    mut commands: Commands,
    mut state: ResMut<LobbyState>,
    mailbox: Res<Mailbox>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut opens: MessageWriter<DuelCommand>,
    // Absent in a headless test, which has no settings file to write to.
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    let replies = {
        let Ok(mut box_) = mailbox.0.lock() else {
            return;
        };
        if box_.is_empty() {
            return;
        }
        std::mem::take(&mut *box_)
    };
    for reply in replies {
        let reply = match reply {
            Reply::Remote(epoch, reply) if epoch == state.gateway_epoch => *reply,
            Reply::Remote(_, _) => continue,
            reply => reply,
        };
        if let Reply::Event(LobbyEvent::Printings {
            card,
            printings,
            from_catalog: false,
        }) = &reply
        {
            let oracle = state
                .lobby
                .builder()
                .pool()
                .iter()
                .find(|c| c.index == *card)
                .map(|c| c.oracle_id.as_str());
            if let Some(oracle) =
                oracle.filter(|id| !cfg!(test) && uuid::Uuid::parse_str(id).is_ok())
            {
                super::print_catalog::fetch(
                    *card,
                    oracle,
                    printings.clone(),
                    state.gateway_epoch,
                    &mailbox,
                );
                continue;
            }
        }
        let reply = match reply {
            Reply::PrintingCatalog(event) => Reply::Event(event),
            Reply::PoolLanguage(lang, event) if lang == state.lobby.lang() => Reply::Event(event),
            Reply::PoolLanguage(_, _) => continue,
            other => other,
        };
        match reply {
            Reply::Remote(_, _) | Reply::PrintingCatalog(_) | Reply::PoolLanguage(_, _) => {}
            Reply::Event(event) => {
                if let LobbyEvent::Games(listing) = &event
                    && !state.lobby.busy()
                    && state.lobby.awaiting().is_none()
                    && listing.games == state.lobby.games()
                    && listing.total == state.lobby.total()
                    && listing.offset == state.lobby.offset()
                {
                    continue;
                }

                // A sign-in that worked is the one moment this client knows
                // an address is a real one, so it is the only moment worth
                // writing it down. Read off the field rather than out of the
                // request, because the field is what the player typed and the
                // request is gone by now.
                let worked = matches!(event, LobbyEvent::LoggedIn { .. });
                let next = state.lobby.apply(event);
                if worked {
                    // The use is counted before anything is written, so
                    // that the address and the use go out in one save.
                    let gateway = state.gateway.clone();
                    state.uses.record(&gateway);
                    if let Some(settings) = settings.as_mut() {
                        settings.last_email = state.lobby.field(Field::Email).to_string();
                        keep_gateways(&state, settings);
                    }
                }
                dispatch(&mut state, &mailbox, next);
            }
            Reply::Registration { enabled, art_cache } => {
                state.lobby.set_registration_enabled(enabled);
                state.art_cache = art_cache;
            }
            Reply::Expired => {
                state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
                state.lobby.sign_out();
            }
            Reply::Gateway { url, probe } => {
                if state.gateway_answered(url, probe)
                    && let Some(settings) = settings.as_mut()
                {
                    keep_gateways(&state, settings);
                }
            }
        }
    }
    // Keys and standing orders belong to the account, so signing in is what
    // fetches them and signing out is what stops writing them back. Both are
    // idempotent, which is why this can simply follow the token every frame
    // the mailbox delivers something.
    match state.lobby.token() {
        // Account attachment changes transport bookkeeping, not visible preferences.
        // Their asynchronous arrival is marked changed by prefs::sync.
        Some(token) => prefs
            .bypass_change_detection()
            .attach(&state.gateway, token),
        None => prefs.bypass_change_detection().detach(),
    }
    let Screen::Seated(handover) = state.lobby.screen().clone() else {
        return;
    };
    if state.connected {
        return;
    }
    // An offline seat has no socket to dial and no ticket a gateway would
    // honour: the game is a preset the start button already built and
    // validated, and the host for it runs here. Everything after this point
    // — the duel, its views, its questions — is the same code either way,
    // which is the whole reason `DuelHost` exists.
    if handover.local {
        match state
            .offline
            .as_mut()
            .and_then(super::offline::Offline::take_started)
            .and_then(|preset| {
                // The one place an offline duel becomes a host, and therefore
                // the only place a hand-dealt board can be spliced in.
                // `host::house_duel` reads like the other half of this and is
                // not: nothing calls it, and the preset it builds is not this
                // one.
                #[allow(unused_mut)]
                let mut preset = preset;
                #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
                crate::host::deal_the_dev_board(&mut preset);
                // The seat names are drawn for the rest of the game, so they
                // are written in the player's language here and never again:
                // `GameStatic` is sent once, and a gateway's table names its
                // chairs after accounts, which have no language at all.
                let lang = settings
                    .as_ref()
                    .map_or(Lang::En, |s| baylee_client_core::Lang::of(&s.lang));
                let names = super::offline::seat_names(&preset, lang);
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                crate::host::LocalHost::new(&preset, PlayerId::new(0), &refs)
            }) {
            Some(host) => {
                state.connected = true;
                commands.insert_resource(InstalledHost(Box::new(host)));
                opens.write(DuelCommand::Open);
            }
            None => state
                .lobby
                .unseat_because(Phrase::NoOfflineDuel, &[] as &[&str]),
        }
        return;
    }
    let ticket = SeatTicket {
        gateway: state.gateway.clone(),
        game_id: handover.game_id,
        // A hint only; the table's opening payload says which chair this is.
        seat: PlayerId::new(u8::try_from(handover.seat).unwrap_or(0)),
        seat_token: handover.seat_token,
    };
    match NetworkHost::connect(ticket) {
        Ok(host) => {
            state.connected = true;
            commands.insert_resource(InstalledHost(Box::new(host)));
            opens.write(DuelCommand::Open);
        }
        Err(reason) => state
            .lobby
            .unseat_because(Phrase::CouldNotReachTable, &[&reason]),
    }
}

/// How often a table of ours that is open is checked for an opponent.
const WATCH_SECS: f32 = 2.0;

/// Re-reads the table list while we are holding a seat nobody can use yet
/// **and** nothing is pushing the list.
///
/// The lobby feed covers this now: an open table turns `"playing"` the moment
/// somebody joins it, and that is a lobby change like any other. What is left
/// here is the fallback for a gateway too old to have `/lobby/ws`, or a socket
/// that could not be opened — the wait is for another person, so ending it
/// two seconds late is far better than not ending it.
pub(super) fn watch(
    time: Res<Time>,
    mut since: Local<f32>,
    mut state: ResMut<LobbyState>,
    feed: Res<super::feed::Feed>,
    mailbox: Res<Mailbox>,
) {
    if feed.live() || state.lobby.awaiting().is_none() {
        *since = 0.0;
        return;
    }
    *since += time.delta_secs();
    if *since < WATCH_SECS {
        return;
    }
    *since = 0.0;
    let request = state.lobby.refresh();
    dispatch(&mut state, &mailbox, request);
}

/// Hands the sign-in form to the platform's own text input, where there is one.
///
/// Only the browser has one. Focusing a field there focuses a real `<input>`,
/// which is what raises a phone's keyboard and what makes autofill, paste and
/// an IME work at all; the value comes back whole rather than as keystrokes.
/// The keyboard is *not* raised on arrival — only when a field is tapped —
/// because a form that covers half the screen before anyone asked for it is
/// the thing every mobile web app gets wrong.
#[allow(clippy::too_many_lines)] // Platform input routing across lobby and editor fields.
pub(super) fn softkeys(
    mut keys: ResMut<SoftKeyboard>,
    mut state: ResMut<LobbyState>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut epoch: Local<u64>,
    mut build_epoch: Local<u64>,
) {
    if !SoftKeyboard::owns_typing() {
        return;
    }
    if state.lobby.library().page.is_some() || state.confirmation.is_some() {
        keys.close();
        drop(keys.drain());
        return;
    }
    // The builder counts its own placements, so it gets its own tally: one
    // shared counter would open the keyboard on the way between the screens.
    if matches!(state.lobby.screen(), Screen::Build) {
        let builder = state.lobby.builder();
        if builder.picker().is_some_and(|p| !p.set_open()) {
            keys.close();
            drop(keys.drain());
            *build_epoch = builder.focus_epoch();
            return;
        }
        if *build_epoch != builder.focus_epoch() {
            *build_epoch = builder.focus_epoch();
            keys.open(builder.focus().kind(), builder.focused_text());
            return;
        }
        for key in keys.drain() {
            match key {
                SoftKey::Text {
                    value,
                    cursor,
                    anchor,
                } => {
                    let field = state.lobby.builder().focus();
                    let changed = state.lobby.builder().focused_text() != value;
                    state
                        .lobby
                        .builder_mut()
                        .edit_buffer(field, |buf| buf.set(&value, cursor, anchor));
                    if changed && field == BuildField::Search {
                        state.completion = None;
                        state.completion_hidden = false;
                        scrolled.set(List::Pool, 0.0);
                    }
                }
                SoftKey::Caret { cursor, anchor } => {
                    let field = state.lobby.builder().focus();
                    state
                        .lobby
                        .builder_mut()
                        .edit_buffer(field, |buf| buf.place(cursor, anchor));
                }
                // Nothing to submit: a deck is saved from the bar, and
                // closing the keyboard is what "done" means here.
                SoftKey::Submit => {
                    if state.lobby.builder().focus() == BuildField::PickerSet {
                        state.lobby.builder_mut().picker_choose_set();
                    }
                    keys.close();
                }
                SoftKey::Dismiss => {
                    state.lobby.builder_mut().picker_close_sets();
                    keys.close();
                }
            }
        }
        return;
    }
    *build_epoch = state.lobby.builder().focus_epoch();
    // The table screen has two fields of its own — the search box and the
    // room password — so it types like the form does. Everything else has
    // none, and holding a keyboard open over it covers half a phone.
    if !matches!(state.lobby.screen(), Screen::SignIn { .. } | Screen::Table) {
        keys.close();
        *epoch = state.lobby.focus_epoch();
        return;
    }
    // A tap on a field is what opens it — including a tap on the field the
    // caret is already in, which is why this counts placements rather than
    // watching which field is focused.
    if *epoch != state.lobby.focus_epoch() {
        *epoch = state.lobby.focus_epoch();
        if state.lobby.typing_here() {
            let field = state.lobby.focus();
            keys.open(state.lobby.field_kind(field), state.lobby.field(field));
        } else {
            keys.close();
        }
        return;
    }
    if !state.lobby.typing_here() {
        keys.drain();
        return;
    }
    for key in keys.drain() {
        match key {
            // The element's caret comes with it: the browser is the authority
            // on where the next character goes, and this client draws that
            // caret rather than letting the invisible input draw it.
            SoftKey::Text {
                value,
                cursor,
                anchor,
            } => {
                let field = state.lobby.focus();
                state.lobby.set_field_at(field, &value, cursor, anchor);
            }
            SoftKey::Caret { cursor, anchor } => {
                let field = state.lobby.focus();
                state.lobby.set_caret(field, cursor, anchor);
            }
            SoftKey::Submit => {
                let request = if matches!(state.lobby.screen(), Screen::Table) {
                    // "Done" on the table screen means the search that was
                    // just typed; there is no form here to send.
                    state.lobby.search_again()
                } else {
                    state.lobby.submit()
                };
                dispatch(&mut state, &mailbox, request);
            }
            // Escape is "put the keyboard away", never "send the form": a
            // password field that signed you in on the key you pressed to
            // back out of it would be the worst possible reading.
            SoftKey::Dismiss => keys.close(),
        }
    }
}

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
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
    mut paste: Local<Option<super::editing::Paste>>,
) {
    if state.confirmation.is_some() {
        keys.clear();
        if codes.just_pressed(KeyCode::Escape) {
            state.confirmation = None;
        }
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
        } else if let Some(chord) = crate::keys::captured(&codes) {
            prefs.edit().keymap.bind(action, vec![chord]);
            state.settings = SettingsPane::Open;
        }
        return;
    }
    if state.settings.is_open() {
        // Nothing on the settings screen is typed into.
        keys.clear();
        return;
    }
    if state.lobby.library().page.is_some() {
        keys.clear();
        if codes.just_pressed(KeyCode::Escape) && !state.lobby.library().loading {
            state.lobby.close_library();
        }
        return;
    }
    if SoftKeyboard::owns_typing() {
        keys.clear();
        return;
    }
    if matches!(state.lobby.screen(), Screen::Build) {
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
        if super::editing::builder_keys(
            &mut keys,
            &codes,
            &mut state,
            &mut scrolled,
            clipboard.as_deref_mut(),
            &mut paste,
        ) {
            let request = state.lobby.save_deck();
            dispatch(&mut state, &mailbox, request);
        }
        return;
    }
    // The table screen has two fields of its own — the search box and the
    // room password — and before this it had no way to type into either.
    let table = matches!(state.lobby.screen(), Screen::Table);
    if !matches!(state.lobby.screen(), Screen::SignIn { .. }) && !table {
        keys.clear();
        return;
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
        state.as_mut(),
        &mut prefs,
        &mailbox,
        table,
    );
}

/// Chooses a saved gateway, which turns the front door to the account form.
///
/// One door for the pointer and the keyboard, so that choosing by Enter on a
/// row does everything a tap on it does.
fn choose_gateway(
    state: &mut LobbyState,
    prefs: &mut crate::prefs::Prefs,
    mailbox: &Mailbox,
    index: usize,
) {
    if state.select_gateway(index) {
        prefs.detach();
        http::probe_registration(state, mailbox);
        // Asked again: the answer in the list may be from before the gateway
        // was upgraded, and choosing it is the moment that answer is about to
        // matter.
        let url = state.gateway.clone();
        state.probes.insert(url.clone(), Probe::Asking);
        http::probe_gateway(url, mailbox);
    }
}

/// Where card art comes from for the lobby as it stands, and the session the
/// gateway's mirror is shown: the mirror while signed in to a gateway that
/// has one, else Scryfall.
pub(super) fn art_source(state: &LobbyState) -> (Option<String>, Option<&str>) {
    let token = state.lobby.token();
    let mirror = (token.is_some() && state.art_cache)
        .then(|| client_core::images::gateway_art_base(&state.gateway));
    (mirror, token)
}

/// Keeps the art base and the mirror's session with the lobby's (#273).
///
/// The mirror serves only a session, so a client that is not signed in to its
/// gateway, offline play among them, takes art from Scryfall and shows the
/// mirror nothing. The one place either is set, so the two cannot disagree.
pub(super) fn art_follows_the_session(
    state: Res<LobbyState>,
    mut applied: Local<Option<(Option<String>, Option<String>)>>,
) {
    if !state.is_changed() {
        return;
    }
    let (mirror, token) = art_source(&state);
    let now = (mirror, token.map(str::to_string));
    if applied.as_ref() == Some(&now) {
        return;
    }
    match &now.0 {
        Some(mirror) => client_core::images::use_art_base(mirror.clone()),
        None => client_core::images::reset_art_base(),
    }
    #[cfg(not(target_arch = "wasm32"))]
    crate::artreader::use_session(now.1.as_deref());
    *applied = Some(now);
}

/// Writes the saved gateways and their uses back to the settings file.
fn keep_gateways(state: &LobbyState, settings: &mut crate::settings::ClientSettings) {
    settings.gateways.clone_from(&state.gateways);
    settings.gateway_uses.clone_from(&state.uses);
    settings.save();
}

/// The sign-in and table screens' own text fields.
///
/// Its own function because it is the whole of what a browser's `<input>`
/// would answer for free, written out for a canvas that has none: a caret
/// that moves by character, word and line, a selection that shift extends,
/// Delete as well as Backspace, and select-all.
fn text_field_keys(
    keys: &mut MessageReader<KeyboardInput>,
    codes: &ButtonInput<KeyCode>,
    state: &mut LobbyState,
    prefs: &mut crate::prefs::Prefs,
    mailbox: &Mailbox,
    table: bool,
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
        // Tab always moves the caret; everything else needs it to be in a
        // field this screen is drawing.
        if !matches!(key.logical_key, Key::Tab) && !state.lobby.typing_here() {
            continue;
        }
        match &key.logical_key {
            Key::Backspace => state.lobby.backspace(),
            Key::Delete => state.lobby.delete_forward(),
            Key::ArrowLeft => state.lobby.move_caret(reach, Dir::Left, shift),
            Key::ArrowRight => state.lobby.move_caret(reach, Dir::Right, shift),
            Key::Home => state.lobby.move_caret(Reach::Line, Dir::Left, shift),
            Key::End => state.lobby.move_caret(Reach::Line, Dir::Right, shift),
            Key::Tab => state
                .lobby
                .cycle_focus(if shift { Tab::Back } else { Tab::Next }),
            // Escape shuts the gear menu, and otherwise goes back from the
            // account form: the same as the Back button beside its title.
            Key::Escape if !table && state.front_menu => state.front_menu = false,
            Key::Escape if !table && state.lobby.gateway_chosen() => state.leave_gateway(),
            Key::Escape if !table && state.gateway_cursor.is_some() => state.gateway_cursor = None,
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
                }
            }
            // Everything else is text or nothing. `type_char` drops the
            // control characters Tab and Enter also produce.
            _ => {
                if let Some(text) = key.text.as_ref() {
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

/// Turns a click on a lobby control into an intent.
#[allow(clippy::too_many_arguments)] // two pointer streams, then the usual
#[allow(clippy::too_many_lines)] // one flat match, read top to bottom
pub(super) fn clicks(
    mut pointer: MessageReader<Pointer<Click>>,
    mut ends: MessageReader<Pointer<DragEnd>>,
    mut scrolled: ResMut<Scrolled>,
    presses: Query<&Press>,
    // The filter builder's own buttons carry the model's vocabulary rather
    // than a `Press`, and one query reads all of them — the same one the zone
    // browser reads. `crate::filterui` puts a `FilterAct` on every button it
    // draws, so a control added to the builder is wired by being drawn; the
    // alternative was a `Press` variant per button, in two places, kept in
    // step by hand.
    acts: Query<&crate::filterui::FilterAct>,
    dones: Query<&crate::filterui::FilterDone>,
    parents: Query<&ChildOf>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mailbox: Res<Mailbox>,
    // Absent in a headless test, which has no settings file to write to.
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
    motion: Res<super::front::FrontMotion>,
) {
    // A panel on its way out or in answers nothing: what is under the
    // pointer is half of a form that is going, or not yet there.
    if motion.moving() {
        pointer.clear();
        ends.clear();
        return;
    }
    // A release always fires a click, drag or no drag, so a swipe down the
    // card list would add whichever card it started on. The scroll it already
    // performed is what the gesture meant.
    let swiped = ends.read().any(|end| end.distance.length() > DRAG_SLOP);
    if swiped {
        pointer.clear();
        return;
    }
    for click in pointer.read() {
        // The builder's buttons first: its rows sit inside the deck builder's
        // own panel, so a `Press` above them would otherwise swallow a click
        // meant for a row.
        if let Some(act) = crate::input::find_in_lineage(click.entity, &acts, &parents) {
            let act = act.0;
            state.lobby.builder_mut().filter_act(act);
            // *Done* is both: the act hands the row's caret back and the
            // marker beside it shuts the panel.
            if crate::input::find_in_lineage(click.entity, &dones, &parents).is_some() {
                state.lobby.builder_mut().close_panel();
            }
            continue;
        }
        let Some(press) = in_lineage(click.entity, &presses, &parents) else {
            if !crate::buildui::autocomplete::suggestions(&state).is_empty() {
                state.completion_hidden = true;
                state.completion = None;
            }
            continue;
        };
        if !matches!(
            press,
            Press::CompleteSearch(_) | Press::FocusBuild(BuildField::Search)
        ) && !crate::buildui::autocomplete::suggestions(&state).is_empty()
        {
            state.completion_hidden = true;
            state.completion = None;
        }
        if state.confirmation.is_some()
            && !matches!(press, Press::ConfirmDestructive | Press::CancelDestructive)
        {
            continue;
        }
        // Anything pressed but the menu's own controls closes the gear menu,
        // the veil around it included.
        if state.front_menu
            && !matches!(
                *press,
                Press::FrontMenu | Press::PickLang(_) | Press::PickerNothing
            )
        {
            state.front_menu = false;
        }
        // Any other control answers the question the back button asked.
        if *press != Press::CloseBuilder {
            state.confirm_leave = false;
        }
        // A filter that changes what is in the list puts it back at the top:
        // finding yourself halfway down a fresh search is disorienting, and
        // the row you were reading is not in it any more anyway.
        if matches!(
            *press,
            Press::ToggleColor(_)
                | Press::SetKind(_)
                | Press::SetCmc(_)
                | Press::TogglePlayable
                | Press::CycleSort
                | Press::ClearFilters
        ) {
            scrolled.set(List::Pool, 0.0);
        }
        // Any click that is not the rebinding chip itself calls off a
        // rebinding in progress. Leaving it armed would mean the next key
        // pressed anywhere lands on whichever row was last tapped.
        if state.settings.is_open() && !matches!(*press, Press::Rebind(_)) {
            state.settings = SettingsPane::Open;
        }
        match *press {
            Press::Hub(hub) => {
                state.hub = hub;
                scrolled.set(List::Table, 0.0);
            }
            Press::RoomSize(more) => {
                state.room_chairs = if more {
                    state.room_chairs.saturating_add(1).min(MAX_CHAIRS)
                } else {
                    state.room_chairs.saturating_sub(1).max(MIN_CHAIRS)
                };
            }
            Press::AddGateway => {
                if let Some(url) = state.check_gateway() {
                    http::probe_gateway(url, &mailbox);
                }
            }
            Press::SelectGateway(index) => choose_gateway(&mut state, &mut prefs, &mailbox, index),
            Press::ForgetGateway(index) => {
                if let Some(url) = state.gateways.get(index) {
                    state.confirmation = Some(confirm::Destructive::ForgetGateway(url.clone()));
                }
            }
            Press::LeaveGateway => state.leave_gateway(),
            Press::FrontMenu => state.front_menu = !state.front_menu,
            Press::BrowseHouse | Press::BrowseHistory | Press::RetryLibrary => {
                scrolled.set(List::Library, 0.0);
                let history = *press == Press::BrowseHistory
                    || (*press == Press::RetryLibrary
                        && matches!(
                            state.lobby.library().page,
                            Some(client_core::lobby::library::Page::History(_))
                        ));
                let request = if history {
                    if let Some(client_core::lobby::library::Page::History(id)) =
                        state.lobby.library().page.clone()
                    {
                        state.lobby.browse_deck_history(&id)
                    } else {
                        state.lobby.browse_history()
                    }
                } else {
                    state.lobby.browse_house()
                };
                dispatch(&mut state, &mailbox, request);
            }
            Press::DeckHistory(index) => {
                if let Some(id) = state.lobby.decks().get(index).map(|d| d.id.clone()) {
                    let request = state.lobby.browse_deck_history(&id);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::CloseLibrary => state.lobby.close_library(),
            Press::PreviewHouse(index) => {
                let choice = state
                    .lobby
                    .library()
                    .house
                    .get(index)
                    .map(|d| (d.id.clone(), d.version));
                if let Some((id, version)) = choice {
                    let request = state.lobby.preview_version(&id, version);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::PreviewVersion(version) => {
                if let Some(client_core::lobby::library::Page::History(id)) =
                    state.lobby.library().page.clone()
                {
                    let request = state.lobby.preview_version(&id, version);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::CopyHouse(index) => {
                let request = state.lobby.copy_house(index);
                dispatch(&mut state, &mailbox, request);
            }
            Press::RestoreVersion => {
                let request = state.lobby.restore_preview();
                dispatch(&mut state, &mailbox, request);
            }
            Press::OpenSettings => state.settings = SettingsPane::Open,
            Press::CloseSettings => state.settings = SettingsPane::Closed,
            Press::Rebind(action) => {
                // Tapping the armed row again disarms it, so the chip is its
                // own cancel and there is no way to get stuck waiting.
                state.settings = if state.settings.capturing() == Some(action) {
                    SettingsPane::Open
                } else {
                    SettingsPane::Rebinding(action)
                };
            }
            Press::ResetBinding(action) => prefs.edit().keymap.reset(action),
            Press::ResetAllBindings => {
                prefs.edit().keymap = baylee_client_core::prefs::Keymap::standard();
            }
            Press::ResetAbilityOrders => prefs.edit().ability_orders.clear(),
            Press::ForgetAbility(ability) => prefs
                .edit()
                .ability_orders
                .retain(|order| order.ability != ability),
            Press::ToggleAuto(rule) => {
                let mut edit = prefs.edit();
                rule.toggle(&mut edit.auto);
            }
            Press::ToggleMotion => {
                let mut edit = prefs.edit();
                edit.reduce_motion = !edit.reduce_motion;
            }
            Press::PickSky(mode) => prefs.edit().sky = mode,
            Press::PickSound(level) => prefs.edit().sound = level,
            Press::PickAtmosphere(air) => prefs.edit().atmosphere = air,
            Press::PickLang(lang) => {
                state.lobby.set_lang(lang);
                // One setting, two readers: the interface draws itself in
                // this language and the catalog is asked for card text in
                // it. Remembered at once, because the settings screen has
                // no way out but a click and a language that reverted on
                // the next launch would read as a button that did nothing.
                state.lang = lang.code().to_string();
                if state.lobby.builder().loaded() {
                    dispatch(&mut state, &mailbox, Some(LobbyRequest::LoadPool));
                }
                if let Some(settings) = settings.as_mut() {
                    settings.lang = lang.code().to_string();
                    settings.save();
                }
            }
            Press::ToggleRail(side, row) => prefs.edit().orders.toggle(side, row),
            Press::SetRail(preset) => prefs.edit().orders.set_to(preset),
            Press::Focus(field) => state.lobby.focus_on(field),
            Press::Reveal(field) => state.lobby.toggle_reveal(field),
            Press::ToggleRegistering => state.lobby.toggle_registering(),
            Press::Submit => {
                let request = state.lobby.submit();
                dispatch(&mut state, &mailbox, request);
            }
            // Offline has no account to forget, so the same button is what
            // leaves offline play — and the performer has to go with it, or
            // the sign-in form's own requests would still be answered out of
            // the local deck file.
            Press::SignOut => {
                state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
                prefs.detach();
                scrolled.set(List::Table, 0.0);
                state.offline = None;
                state.lobby.sign_out();
            }
            Press::Refresh => {
                let request = state.lobby.refresh();
                dispatch(&mut state, &mailbox, request);
            }
            Press::Search => {
                let request = state.lobby.search_again();
                dispatch(&mut state, &mailbox, request);
            }
            Press::Page(forwards) => {
                let request = state.lobby.page(forwards);
                dispatch(&mut state, &mailbox, request);
            }
            Press::SelectDeck(index) => state.lobby.select_deck(index),
            Press::Host(mode) => {
                let request = state.lobby.host(mode);
                dispatch(&mut state, &mailbox, request);
            }
            Press::Join(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.join(&game);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::OpenRoom(chairs) => {
                let request = state.lobby.open_room(GameMode::Open, chairs, String::new());
                dispatch(&mut state, &mailbox, request);
            }
            Press::JoinSeat(index, seat) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.join_seat(&game, Some(seat));
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::LeaveTable(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.leave_table(&game);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::Ready(index, ready) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.set_ready(&game, ready);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            // The same press as the button on the veil, from the other side:
            // this player went back to the lobby and their chair at the next
            // table is waiting there for them.
            Press::Rematch(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.rematch(&game);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::StartRoom(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.start_room(&game);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::HandOver(index, seat) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.hand_over(&game, seat);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::SeatKind(index, seat, kind) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.set_seat(&game, seat, Some(kind), None);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::SeatAi(index, seat, profile) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request =
                        state
                            .lobby
                            .set_seat(&game, seat, None, Some(profile.to_string()));
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::SeatTeam(index, seat, team) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.seat_team(&game, seat, team);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::SeatDeck(index, seat) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.seat_deck(&game, seat);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            // Not a duel any more. Offline is the lobby with a different
            // performer behind it, so this opens the *table screen* with the
            // player's own decks in it and a room to arrange. What the button
            // skips is still the sign-in; what it no longer skips is choosing
            // who you are playing and with what.
            Press::PlayOffline => {
                state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
                prefs.detach();
                scrolled.set(List::Table, 0.0);
                // Whatever is already here is kept. `Press::SignOut` is the only
                // thing that clears it, so a player coming back to offline
                // play finds the decks and the room they left.
                state
                    .offline
                    .get_or_insert_with(super::offline::Offline::load);
                let request = state.lobby.play_offline();
                dispatch(&mut state, &mailbox, request);
            }
            // Game-over actions are handled by `leave_clicks`. An empty
            // part of the artwork dialog dismisses its set autocomplete.
            Press::Leave | Press::PlayAgain => {}
            Press::PickerNothing => state.lobby.builder_mut().picker_close_sets(),
            Press::NewDeck => {
                state.commander_pick = None;
                state.pane = Pane::Deck;
                let request = state.lobby.build_deck();
                dispatch(&mut state, &mailbox, request);
            }
            Press::EditDeck(index) => {
                state.commander_pick = None;
                state.pane = Pane::Deck;
                let request = state.lobby.edit_deck(index);
                dispatch(&mut state, &mailbox, request);
            }
            Press::DeleteDeck(index) => {
                if let Some(deck) = state.lobby.decks().get(index) {
                    state.confirmation = Some(confirm::Destructive::Delete(deck.id.clone()));
                }
            }
            Press::ConfirmDestructive => {
                let forgetting = matches!(
                    state.confirmation,
                    Some(confirm::Destructive::ForgetGateway(_))
                );
                let request = confirm::accept(&mut state);
                if forgetting && let Some(settings) = settings.as_mut() {
                    keep_gateways(&state, settings);
                }
                dispatch(&mut state, &mailbox, request);
            }
            Press::CancelDestructive => state.confirmation = None,
            Press::CloseBuilder => {
                if state.lobby.builder().dirty() && !state.confirm_leave {
                    state.confirm_leave = true;
                    state.lobby.tell_refusal(Phrase::UnsavedChanges, &[]);
                } else {
                    state.confirm_leave = false;
                    state.hub = Hub::Decks;
                    let request = state.lobby.close_builder();
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::SaveDeck => {
                let request = state.lobby.save_deck();
                dispatch(&mut state, &mailbox, request);
            }
            Press::FocusBuild(field) => {
                state.completion_hidden = false;
                state.completion = None;
                let deck = state.lobby.builder_mut();
                // A tap in the search box shuts the builder and takes the
                // caret, which is the way back out of it — the same rule the
                // zone browser's box follows.
                if field == BuildField::Search {
                    deck.close_panel();
                }
                deck.focus_on(field);
            }
            Press::PickRowPrint(at) => {
                scrolled.set(List::PickerPanel, 0.0);
                let zone = state.lobby.builder().zone();
                let request = state.lobby.builder_mut().open_row_picker(at, zone);
                dispatch(&mut state, &mailbox, request);
            }
            Press::PickPrint(slot) => {
                scrolled.set(List::PickerPanel, 0.0);
                let zone = state.lobby.builder().zone();
                let request = state.lobby.builder_mut().open_picker(slot, zone);
                dispatch(&mut state, &mailbox, request);
            }
            Press::PickerStep(by) => {
                state.lobby.builder_mut().picker_close_sets();
                state.lobby.builder_mut().picker_step(by);
            }
            Press::PickerGo(at) => state.lobby.builder_mut().picker_go(at),
            Press::PickerLang(which) => {
                // The list the index came from is the one being read here, so
                // a stale index simply selects nothing rather than panicking.
                let lang = which.and_then(|i| {
                    state
                        .lobby
                        .builder()
                        .picker()
                        .and_then(|p| p.langs().get(i).cloned())
                });
                state.lobby.builder_mut().picker_set_lang(lang.as_deref());
            }
            Press::PickerRefresh => {
                let request = state.lobby.builder_mut().refresh_printings();
                let card = state
                    .lobby
                    .builder()
                    .picker()
                    .and_then(|p| state.lobby.builder().card(p.slot()))
                    .cloned();
                if request.is_some()
                    && !cfg!(test)
                    && let Some(card) = card.filter(|c| uuid::Uuid::parse_str(&c.oracle_id).is_ok())
                {
                    let fallback = state
                        .lobby
                        .builder()
                        .picker()
                        .map(|p| p.all_printings().to_vec())
                        .unwrap_or_default();
                    super::print_catalog::fetch(
                        card.index,
                        &card.oracle_id,
                        fallback,
                        state.gateway_epoch,
                        &mailbox,
                    );
                } else {
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::PickerForceFinish => state.lobby.builder_mut().picker_force_finish(),
            Press::PickerSet(at) => state.lobby.builder_mut().picker_set_set(at),
            Press::PickerFinish(finish) => state.lobby.builder_mut().picker_set_finish(finish),
            Press::PickerConfirm => {
                if !state.lobby.builder_mut().picker_confirm() {
                    state.lobby.tell_refusal(Phrase::NoRoomForCopy, &[]);
                }
            }
            Press::PickerClose => state.lobby.builder_mut().close_picker(),
            Press::AddRow(at) => {
                let zone = state.lobby.builder().zone();
                if let Some(entry) = state.lobby.builder().entries(zone).get(at).cloned()
                    && !state
                        .lobby
                        .builder_mut()
                        .add_print(entry.slot, zone, entry.print)
                {
                    state.lobby.tell_refusal(Phrase::NoRoomForCopy, &[]);
                }
            }
            Press::RemoveRow(at) => {
                let zone = state.lobby.builder().zone();
                state.lobby.builder_mut().remove_at(at, zone);
            }
            Press::MoveRow(at) => {
                let from = state.lobby.builder().zone();
                let to = match from {
                    Zone::Main => Zone::Side,
                    Zone::Side => Zone::Main,
                };
                state.lobby.builder_mut().move_entry(at, from, to);
            }
            Press::RemoveCardFrom(slot, zone) => {
                state.lobby.builder_mut().remove(slot, zone);
            }
            Press::AddCardTo(slot, zone) => {
                state.lobby.builder_mut().add(slot, zone);
            }
            Press::ChooseCommander(partner) => {
                state.commander_pick = Some(partner);
                state.pane = Pane::Cards;
                state.lobby.builder_mut().clear_filters();
                state.lobby.builder_mut().set_text("is:commander");
                state.lobby.builder_mut().focus_on(BuildField::Search);
                scrolled.set(List::Pool, 0.0);
            }
            Press::CancelCommanderPick => {
                state.commander_pick = None;
                state.lobby.builder_mut().set_text("");
            }
            Press::SetCommander(slot) | Press::AddPartner(slot) => {
                let accepted = if matches!(press, Press::AddPartner(_)) {
                    state.lobby.builder_mut().add_partner(slot)
                } else {
                    state.lobby.builder_mut().set_commander(slot)
                };
                if accepted {
                    state.commander_pick = None;
                    state.pane = Pane::Deck;
                    state.lobby.builder_mut().set_text("");
                }
            }
            Press::RemoveCommander(slot) => state.lobby.builder_mut().remove_commander(slot),
            Press::CompleteSearch(slot) => {
                crate::buildui::autocomplete::choose(&mut state, slot);
                scrolled.set(List::Pool, 0.0);
            }
            Press::ToggleDeckActions => state.deck_actions_open = !state.deck_actions_open,
            Press::ToggleStatistics => state.stats_open = !state.stats_open,
            Press::ClearCommander => state.lobby.builder_mut().clear_commander(),
            Press::SetZone(zone) => state.lobby.builder_mut().set_zone(zone),
            Press::ToggleColor(color) => state.lobby.builder_mut().toggle_color(color),
            Press::SetKind(kind) => {
                let builder = state.lobby.builder_mut();
                // A second tap on the open chip is how it is closed again;
                // without it a filter can only be dropped from "Clear".
                let same = builder.kind() == kind;
                builder.set_kind(if same { None } else { kind });
            }
            Press::SetCmc(cmc) => state.lobby.builder_mut().set_cmc(Some(cmc)),
            Press::TogglePlayable => state.lobby.builder_mut().toggle_playable_only(),
            Press::CycleSort => state.lobby.builder_mut().cycle_sort(),
            Press::ClearFilters => state.lobby.builder_mut().clear_filters(),
            Press::ClearDeck => {
                state.confirmation = Some(confirm::Destructive::Clear(
                    state.lobby.builder().editing().map(str::to_owned),
                ));
            }
            Press::ShowPane(pane) => state.pane = pane,
            Press::Inspect(slot) => state.lobby.builder_mut().inspect(slot),
            Press::CloseCard => state.lobby.builder_mut().stop_inspecting(),
            Press::ToggleFilters => state.filters_open = !state.filters_open,
            Press::ToggleFilterPanel => state.lobby.builder_mut().toggle_panel(),
        }
    }
}

/// How far a pointer has to travel before the gesture is a scroll rather than
/// a tap. Below it a shaky finger would still add a card; above it, a swipe
/// down a list would.
const DRAG_SLOP: f32 = 8.0;

/// What one line of wheel travel moves a list, in logical pixels.
const WHEEL_LINE: f32 = 32.0;

/// A list that scrolls its own contents, and which one it is.
///
/// `Overflow::scroll_y` only *clips*: Bevy moves the content when
/// [`ScrollPosition`] changes and nothing changes it on its own. Without this
/// system a sixty-row result list would simply end at the bottom of the panel
/// with no way to reach the rest.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Scrollable(pub(crate) List);

/// The lists that remember where they were left.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum List {
    /// The searchable card pool.
    Pool,
    /// The deck being built.
    Deck,
    /// The tables and decks on the lobby screen.
    Table,
    /// The saved gateways on the front door.
    Gateways,
    Library,
    PickerSets,
    PickerPanel,
}

/// Where each list was left, across rebuilds of the node tree.
///
/// Deliberately not part of [`LobbyState`]: the tree is rebuilt whenever that
/// changes, so keeping the offsets there would rebuild sixty rows on every
/// notch of the wheel. Kept apart, adding a card rebuilds the list *and*
/// leaves it where the player was reading — which is the only reason they
/// scrolled there.
#[derive(Resource, Default)]
pub(crate) struct Scrolled {
    pool: f32,
    deck: f32,
    table: f32,
    gateways: f32,
    library: f32,
    picker_panel: f32,
}

impl Scrolled {
    pub(crate) fn get(&self, list: List) -> f32 {
        match list {
            List::Pool => self.pool,
            List::Deck => self.deck,
            List::Table => self.table,
            List::Gateways => self.gateways,
            List::Library => self.library,
            List::PickerSets => 0.0,
            List::PickerPanel => self.picker_panel,
        }
    }

    pub(super) fn set(&mut self, list: List, at: f32) {
        match list {
            List::Pool => self.pool = at,
            List::Deck => self.deck = at,
            List::Table => self.table = at,
            List::Gateways => self.gateways = at,
            List::Library => self.library = at,
            List::PickerSets => {}
            List::PickerPanel => self.picker_panel = at,
        }
    }
}

/// Turns a wheel or a swipe into scrolling on the list under the pointer.
pub(super) fn scrolls(
    mut wheels: MessageReader<Pointer<Scroll>>,
    mut drags: MessageReader<Pointer<Drag>>,
    parents: Query<&ChildOf>,
    mut lists: Query<(&mut ScrollPosition, &ComputedNode, &Scrollable)>,
    mut memory: ResMut<Scrolled>,
) {
    for wheel in wheels.read() {
        let travel = match wheel.unit {
            MouseScrollUnit::Line => wheel.y * WHEEL_LINE,
            MouseScrollUnit::Pixel => wheel.y,
        };
        // A wheel pushed away from the reader moves the content up, which is
        // an *increase* in the scroll offset.
        scroll_lineage(wheel.entity, -travel, &parents, &mut lists, &mut memory);
    }
    for drag in drags.read() {
        // A finger drags the content itself, so it goes the other way again.
        scroll_lineage(
            drag.entity,
            -drag.delta.y,
            &parents,
            &mut lists,
            &mut memory,
        );
    }
}

/// Scrolls the nearest list at or above an entity, so a gesture over a row
/// scrolls the list the row is in.
fn scroll_lineage(
    entity: Entity,
    by: f32,
    parents: &Query<&ChildOf>,
    lists: &mut Query<(&mut ScrollPosition, &ComputedNode, &Scrollable)>,
    memory: &mut Scrolled,
) {
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Ok((mut position, computed, which)) = lists.get_mut(e) {
            position.y = crate::hud::scrolled(
                position.y,
                by,
                computed.size().y,
                computed.content_size().y,
                computed.inverse_scale_factor(),
            );
            memory.set(which.0, position.y);
            return;
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
}

/// Leaves a finished game and comes back here.
pub(super) fn leave_clicks(
    mut pointer: MessageReader<Pointer<Click>>,
    presses: Query<&Press>,
    parents: Query<&ChildOf>,
    mut state: ResMut<LobbyState>,
    mut closes: MessageWriter<DuelCommand>,
) {
    for click in pointer.read() {
        if let Some(way) = in_lineage(click.entity, &presses, &parents) {
            take_the_way_out(*way, &mut state, &mut closes);
        }
    }
}

/// The way off the end screen for somebody with no pointer.
///
/// `DuelSet::Input` stops at `DuelPhase::Playing`, so on the end screen every
/// key did nothing at all and a keyboard-only player had no way back to the
/// lobby — the one screen in the client with no exit. It lives here and not in
/// `input.rs` for the reason [`crate::hud::finish`] gives about the sheet
/// itself: the verdict is the duel's to say and the way out is the shell's,
/// and a `DuelPlugin` embedded in something with no lobby behind it has
/// nowhere to go.
///
/// It reads the buttons that are **actually drawn** rather than a list of its
/// own, so a key can never take a way out the sheet does not offer.
/// `Confirm` and `Primary` — Space and Enter by default — press the lead
/// answer, which is what the brass button on the sheet is; `Cancel` always
/// leaves, because the way out is the thing nobody may be stuck without.
pub(super) fn leave_keys(
    keys: Res<ButtonInput<KeyCode>>,
    prefs: Res<crate::prefs::Prefs>,
    exits: Query<&Press, With<super::ui::DuelExit>>,
    mut state: ResMut<LobbyState>,
    mut closes: MessageWriter<DuelCommand>,
) {
    use baylee_client_core::prefs::Action;
    let fired = crate::keys::Fired::of(&keys, prefs.keymap());
    if fired.quiet() {
        return;
    }
    let mut leave = false;
    let mut again = false;
    for press in &exits {
        match press {
            Press::Leave => leave = true,
            Press::PlayAgain => again = true,
            _ => {}
        }
    }
    // `ui::spawn_leave_button` puts *play again* first where there is one, and
    // the first answer on a slip is the lead — so the lead is the rematch when
    // the sheet has one and the way back otherwise.
    let lead = if again {
        Press::PlayAgain
    } else if leave {
        Press::Leave
    } else {
        return;
    };
    let way = if fired.has(Action::Cancel) && leave {
        Press::Leave
    } else if fired.has(Action::Confirm) || fired.has(Action::Primary) {
        lead
    } else {
        return;
    };
    take_the_way_out(way, &mut state, &mut closes);
}

/// Takes one of the end screen's exits, whatever asked for it.
///
/// Both buttons close the table; the difference is what is waiting on the
/// other side of it. A rematch is *recorded* rather than sent, because
/// [`came_back`] tears the seat down on the way out and would clear a request
/// already in flight — see `Lobby::want_rematch`.
fn take_the_way_out(way: Press, state: &mut LobbyState, closes: &mut MessageWriter<DuelCommand>) {
    match way {
        Press::Leave => {}
        Press::PlayAgain => {
            let played = match state.lobby.screen() {
                Screen::Seated(handover) => Some(handover.game_id.clone()),
                _ => None,
            };
            if let Some(game_id) = played {
                state.lobby.want_rematch(game_id);
            }
        }
        _ => return,
    }
    closes.write(DuelCommand::Close);
}

/// The lobby is on screen again: forget the seat and re-read the tables.
pub(super) fn came_back(
    mut commands: Commands,
    mut state: ResMut<LobbyState>,
    mailbox: Res<Mailbox>,
    duel: Option<Res<crate::Duel>>,
) {
    // Drops the socket (or the in-process engine) with it: a stale host would
    // keep a dead table's messages queued behind the next game's.
    commands.remove_resource::<InstalledHost>();
    state.connected = false;
    if !matches!(state.lobby.screen(), Screen::Seated(_)) {
        return;
    }
    // The verdict rather than the bare fact of an ending (#155). The duel
    // still has it: `Duel::default()` is written on `DuelCommand::Open` and
    // not on `Close`, so the `Pending::GameOver` that drew the end screen is
    // still in place while this runs, and stays until the next game opens.
    //
    // `Option<Res<_>>` and not `Res<_>`, which is the difference between a
    // fallback and a silently dead system: every test in `tests::end_screen`
    // installs `LobbyPlugin` without `DuelPlugin` and so has no `Duel` at
    // all, and Bevy answers a missing plain `Res` by skipping the system with
    // a warning. That would not fail those tests — it would hollow them out,
    // since what they assert on is what this function does. An embedder with
    // its own duel is the same case in production.
    match ended_as(duel.as_deref()) {
        Some((result, seat, team, own)) => {
            state
                .lobby
                .stand_up_after(&result, seat, team, own.as_ref());
        }
        None => state.lobby.stand_up(Phrase::GameEnded, &[]),
    }
    // The host that has just been dropped *was* the offline table, so this is
    // where it stops existing. Before the refresh below rather than after, so
    // the listing that comes back is the one without it — a row still saying
    // "playing" is a table the lobby would try to reclaim a chair at.
    if let Some(offline) = state.offline.as_mut() {
        offline.close_table();
    }
    // The order matters: unseating first is what lets the request survive,
    // and what stops `poll` re-dialling the game that just ended before the
    // new ticket arrives. A player who pressed *play again* is not shown the
    // table list on the way — the answer puts them straight back in a seat.
    let request = state.lobby.take_rematch().or_else(|| state.lobby.refresh());
    dispatch(&mut state, &mailbox, request);
}

/// How the game ended, for whoever is about to say so.
///
/// All three values or none: a `GameResult` with no roster behind it cannot
/// be worded, because `verdict` needs the seat to know whether "won" means
/// this player. That is the same refusal `hud::finish::spawn_finish` makes
/// for the same reason — a game that never really started says nothing
/// rather than telling a player who never sat down that they lost.
fn ended_as(
    duel: Option<&crate::Duel>,
) -> Option<(
    GameResult,
    PlayerId,
    Option<u8>,
    Option<baylee_view::SeatView>,
)> {
    let duel = duel?;
    let result = *duel.ending()?;
    let statics = duel.statics.as_ref()?;
    let own = duel
        .view
        .as_ref()
        .and_then(|view| view.seat(statics.your_seat))
        .cloned();
    Some((result, statics.your_seat, duel.my_team(), own))
}

/// A component whose click means something.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Press {
    Hub(Hub),
    RoomSize(bool),
    AddGateway,
    SelectGateway(usize),
    /// Asks, in the confirm dialog, whether a saved gateway leaves the list.
    ForgetGateway(usize),
    /// Back from the account form to the gateway form.
    LeaveGateway,
    /// Opens or closes the front door's gear menu.
    FrontMenu,
    BrowseHouse,
    BrowseHistory,
    DeckHistory(usize),
    CloseLibrary,
    RetryLibrary,
    PreviewHouse(usize),
    PreviewVersion(i32),
    CopyHouse(usize),
    RestoreVersion,
    /// Put the caret in this field.
    Focus(Field),
    /// Show a masked field in the clear, or cover it again.
    Reveal(Field),
    /// Swap the form between log-in and sign-up.
    ToggleRegistering,
    /// Send the sign-in form.
    Submit,
    /// Play the house AI in this process, no account needed.
    PlayOffline,
    /// Forget the account.
    SignOut,
    /// Re-read decks and tables.
    Refresh,
    /// Read the table list again for whatever the search box says.
    Search,
    /// Step one page through the table list. `true` is forwards.
    Page(bool),
    /// Pick a deck by its index in the list.
    SelectDeck(usize),
    /// Open a new table.
    Host(GameMode),
    /// Open a table with a chosen number of chairs.
    OpenRoom(usize),
    /// Sit down at a listed table by its index.
    Join(usize),
    /// Sit down in a named chair of a listed table.
    JoinSeat(usize, u32),
    /// Give up a chair. The room outlives it.
    LeaveTable(usize),
    /// Say whether this player is ready at a listed table.
    Ready(usize, bool),
    /// Start a room this account hosts.
    StartRoom(usize),
    /// Hand the room to the player in a chair.
    HandOver(usize, u32),
    /// Make a chair a person's or the AI's.
    SeatKind(usize, u32, SeatKind),
    /// Set an AI chair's difficulty.
    SeatAi(usize, u32, &'static str),
    /// Put the selected deck in a chair.
    SeatDeck(usize, u32),
    /// Move a chair onto a side. `0` puts it back on its own.
    SeatTeam(usize, u32, u8),
    /// Leave a finished game.
    Leave,
    /// Play that game again. Beside [`Press::Leave`], because those are the
    /// only two things left to do with a table that is over.
    PlayAgain,
    /// Take the chair kept for this player at a listed rematch room.
    Rematch(usize),
    /// Open the settings screen.
    OpenSettings,
    /// Leave it.
    CloseSettings,
    /// Wait for a key and bind it to this action.
    Rebind(baylee_client_core::prefs::Action),
    /// Put one action back to its default key.
    ResetBinding(baylee_client_core::prefs::Action),
    /// Put every key back.
    ResetAllBindings,
    /// Flip one automation switch.
    ToggleAuto(baylee_client_core::prefs::AutoRule),
    /// Return every card ability to manual responses.
    ResetAbilityOrders,
    /// Forget one ability's policy.
    ForgetAbility(baylee_core::ids::AbilityRef),
    /// Stop the table moving, or let it move again.
    ToggleMotion,
    /// Speak this language from now on.
    PickLang(Lang),
    /// Put a sky behind the table, or let the clock choose one.
    PickSky(baylee_client_core::sky::SkyMode),
    /// Turn the table up, down, or off.
    PickSound(baylee_client_core::cue::Loudness),
    /// Put weather in the air over the table, or take it away.
    PickAtmosphere(baylee_client_core::atmosphere::Atmosphere),
    /// Turn one step of the phase rail red or green.
    ToggleRail(
        baylee_client_core::automation::RailSide,
        baylee_client_core::automation::RailRow,
    ),
    /// Put the whole rail to a preset.
    SetRail(baylee_client_core::automation::RailPreset),
    /// Open the builder on a new deck.
    NewDeck,
    /// Open the builder on a saved deck, by its index in the list.
    EditDeck(usize),
    /// Throw a saved deck away, by its index in the list.
    DeleteDeck(usize),
    /// Leave the builder for the tables.
    CloseBuilder,
    /// Save whatever the builder holds.
    SaveDeck,
    /// Put the caret in one of the builder's boxes.
    FocusBuild(BuildField),
    /// Build into the main deck or the sideboard.
    SetZone(Zone),
    /// Turn one colour of the identity filter on or off.
    ToggleColor(char),
    /// Show only one card type, or all of them again.
    SetKind(Option<&'static str>),
    /// Show only one mana value, or all of them again. Doubles as the click
    /// target on a curve bar.
    SetCmc(u32),
    /// Hide the cards the engine does not play properly, or stop hiding them.
    TogglePlayable,
    /// Change what the results are sorted by.
    CycleSort,
    /// Drop every filter at once.
    ClearFilters,
    /// Empty both zones.
    ClearDeck,
    ConfirmDestructive,
    CancelDestructive,
    /// Show the pool or the deck, on a screen with room for one.
    ShowPane(Pane),
    /// Read a card in full, by its slot in the pool.
    Inspect(usize),
    /// Open the printing picker on a pool card, by its slot.
    PickPrint(usize),
    PickRowPrint(usize),
    /// Move the picker's carousel.
    PickerStep(i32),
    /// Jump the carousel to one printing, by its place in the visible list.
    PickerGo(usize),
    /// Limit the carousel to one language, by its place in the picker's list,
    /// or `None` for all of them. An index rather than the code itself
    /// because a `Press` is `Copy` and a language code is a `String`.
    PickerLang(Option<usize>),
    /// Choose a finish for the printing the carousel is on.
    PickerFinish(Finish),
    PickerRefresh,
    PickerForceFinish,
    PickerSet(Option<usize>),
    /// Add the picked printing to the deck.
    PickerConfirm,
    /// Put the picker away, adding nothing.
    PickerClose,
    /// Nothing. Carried by the picker's own panel so a tap inside it is
    /// not also a tap on the shade behind it, which would close it.
    PickerNothing,
    /// Take one copy out of a named row of the deck list.
    RemoveRow(usize),
    AddRow(usize),
    /// Move one copy of a named row to the other list — deck to sideboard,
    /// or back. The row keeps the printing it was chosen with.
    MoveRow(usize),
    /// Add one copy of a pool card to a named list, whichever one is open.
    AddCardTo(usize, Zone),
    RemoveCardFrom(usize, Zone),
    /// Make a pool card the deck's commander.
    SetCommander(usize),
    ChooseCommander(bool),
    CancelCommanderPick,
    AddPartner(usize),
    RemoveCommander(usize),
    ToggleStatistics,
    ToggleDeckActions,
    CompleteSearch(usize),
    /// Take the commander mark off, leaving the card in the deck.
    ClearCommander,
    /// Put it away again.
    CloseCard,
    /// Show or hide the filter chips on a narrow screen.
    ToggleFilters,
    /// Open the filter-string builder on what the search box holds, or shut
    /// it. The cogwheel inside the box.
    ToggleFilterPanel,
}

/// The nearest [`Press`] at or above an entity, so a click on a button's
/// label counts as a click on the button.
fn in_lineage<'a>(
    entity: Entity,
    presses: &'a Query<&Press>,
    parents: &Query<&ChildOf>,
) -> Option<&'a Press> {
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Ok(found) = presses.get(e) {
            return Some(found);
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}

/// Raises the loading veil while the lobby is waiting on the network.
///
/// Two waits, and the second is the one that needs saying. A request in
/// flight is usually a blink. Taking a seat is not: the gateway orders an
/// engine and the socket may wait up to thirty seconds for it to attach, and
/// a screen that says nothing for thirty seconds is a screen a player will
/// click again.
pub(super) fn waiting(state: Res<LobbyState>, mut loading: ResMut<crate::loading::Loading>) {
    let lang = state.lobby.lang();
    match state.lobby.screen() {
        Screen::Seated(_) => loading.show(Phrase::VeilTakingSeat.text(lang)),
        // Offline the wait is this process building a deck, a room or an
        // engine, and a veil claiming a conversation with a gateway would
        // be naming a machine that was never dialled.
        _ if state.lobby.busy() && state.lobby.offline() => {
            loading.show(Phrase::VeilWorking.text(lang));
        }
        _ if state.lobby.busy() => loading.show(Phrase::VeilTalking.text(lang)),
        _ => loading.clear(),
    }
}
