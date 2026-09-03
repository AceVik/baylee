//! Bevy systems: the mailbox pump, the seat watch, text entry and the
//! pointer.
//!
//! Nothing here decides anything; each system turns an input into a
//! [`LobbyEvent`] and hands it to [`baylee_client_core::lobby`].

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

// --------------------------------------------------------------- systems

/// Drains the mailbox, advances the lobby, and takes the seat it is granted.
pub(super) fn poll(
    mut commands: Commands,
    mut state: ResMut<LobbyState>,
    mailbox: Res<Mailbox>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut opens: MessageWriter<DuelCommand>,
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
        match reply {
            Reply::Event(event) => {
                let next = state.lobby.apply(event);
                dispatch(&state, &mailbox, next);
            }
            Reply::Registration(enabled) => state.lobby.set_registration_enabled(enabled),
            Reply::Expired => state.lobby.sign_out(),
        }
    }
    // Keys and standing orders belong to the account, so signing in is what
    // fetches them and signing out is what stops writing them back. Both are
    // idempotent, which is why this can simply follow the token every frame
    // the mailbox delivers something.
    match state.lobby.token() {
        Some(token) => prefs.attach(&state.gateway, token),
        None => prefs.detach(),
    }
    let Screen::Seated(handover) = state.lobby.screen().clone() else {
        return;
    };
    if state.connected {
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
            .unseat(format!("could not reach the table: {reason}")),
    }
}

/// How often a table of ours that is open is checked for an opponent.
const WATCH_SECS: f32 = 2.0;

/// Re-reads the table list while we are holding a seat nobody can use yet.
///
/// The gateway has nothing to push here — the seat exists but the game does
/// not, so there is no socket to be told on. Two seconds is well under the
/// time it takes a person to notice, and it stops the moment the wait ends.
pub(super) fn watch(
    time: Res<Time>,
    mut since: Local<f32>,
    mut state: ResMut<LobbyState>,
    mailbox: Res<Mailbox>,
) {
    if state.lobby.awaiting().is_none() {
        *since = 0.0;
        return;
    }
    *since += time.delta_secs();
    if *since < WATCH_SECS {
        return;
    }
    *since = 0.0;
    let request = state.lobby.refresh();
    dispatch(&state, &mailbox, request);
}

/// Hands the sign-in form to the platform's own text input, where there is one.
///
/// Only the browser has one. Focusing a field there focuses a real `<input>`,
/// which is what raises a phone's keyboard and what makes autofill, paste and
/// an IME work at all; the value comes back whole rather than as keystrokes.
/// The keyboard is *not* raised on arrival — only when a field is tapped —
/// because a form that covers half the screen before anyone asked for it is
/// the thing every mobile web app gets wrong.
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
    // The builder counts its own placements, so it gets its own tally: one
    // shared counter would open the keyboard on the way between the screens.
    if matches!(state.lobby.screen(), Screen::Build) {
        let builder = state.lobby.builder();
        if *build_epoch != builder.focus_epoch() {
            *build_epoch = builder.focus_epoch();
            keys.open(builder.focus().kind(), builder.focused_text());
            return;
        }
        for key in keys.drain() {
            match key {
                SoftKey::Text(value) => {
                    let searching = state.lobby.builder().focus() == BuildField::Search;
                    state.lobby.builder_mut().set_focused(&value);
                    if searching {
                        scrolled.set(List::Pool, 0.0);
                    }
                }
                // Nothing to submit: a deck is saved from the bar, and
                // closing the keyboard is what "done" means here.
                SoftKey::Submit => keys.close(),
            }
        }
        return;
    }
    *build_epoch = state.lobby.builder().focus_epoch();
    if !matches!(state.lobby.screen(), Screen::SignIn { .. }) {
        keys.close();
        *epoch = state.lobby.focus_epoch();
        return;
    }
    // A tap on a field is what opens it — including a tap on the field the
    // caret is already in, which is why this counts placements rather than
    // watching which field is focused.
    if *epoch != state.lobby.focus_epoch() {
        *epoch = state.lobby.focus_epoch();
        let field = state.lobby.focus();
        keys.open(field.kind(), state.lobby.field(field));
        return;
    }
    for key in keys.drain() {
        match key {
            SoftKey::Text(value) => {
                let field = state.lobby.focus();
                state.lobby.set_field(field, &value);
            }
            SoftKey::Submit => {
                let request = state.lobby.submit();
                dispatch(&state, &mailbox, request);
            }
        }
    }
}

/// Types into the sign-in form from a keyboard the client itself reads.
///
/// Skipped entirely where [`SoftKeyboard`] owns the typing: the browser's
/// input has focus, so the canvas sees nothing anyway, and anything it did see
/// would be entered twice.
pub(super) fn keyboard(
    mut keys: MessageReader<KeyboardInput>,
    codes: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
) {
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
    if SoftKeyboard::owns_typing() {
        keys.clear();
        return;
    }
    if matches!(state.lobby.screen(), Screen::Build) {
        for key in keys.read() {
            if !key.state.is_pressed() {
                continue;
            }
            let builder = state.lobby.builder_mut();
            let searching = builder.focus() == BuildField::Search;
            let mut narrowed = false;
            match &key.logical_key {
                Key::Backspace => {
                    builder.backspace_focused();
                    narrowed = searching;
                }
                Key::Tab => builder.cycle_focus(),
                // Enter in the search box adds the first result: the fastest
                // way to type a deck is name, return, name, return.
                Key::Enter => {
                    if builder.focus() == BuildField::Search {
                        let first = builder.results().first().copied();
                        let zone = builder.zone();
                        if let Some(slot) = first {
                            builder.add(slot, zone);
                        }
                    }
                }
                _ => {
                    if let Some(text) = key.text.as_ref() {
                        for ch in text.chars() {
                            builder.type_focused(ch);
                            narrowed = searching;
                        }
                    }
                }
            }
            // A different search is a different list; the row that was
            // halfway down it is not in this one.
            if narrowed {
                scrolled.set(List::Pool, 0.0);
            }
        }
        return;
    }
    if !matches!(state.lobby.screen(), Screen::SignIn { .. }) {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if !key.state.is_pressed() {
            continue;
        }
        match &key.logical_key {
            Key::Backspace => state.lobby.backspace(),
            Key::Tab => state.lobby.cycle_focus(),
            Key::Enter => {
                let request = state.lobby.submit();
                dispatch(&state, &mailbox, request);
            }
            // Everything else is text or nothing. `type_char` drops the
            // control characters Tab and Enter also produce.
            _ => {
                if let Some(text) = key.text.as_ref() {
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
    parents: Query<&ChildOf>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mailbox: Res<Mailbox>,
    mut commands: Commands,
    mut opens: MessageWriter<DuelCommand>,
) {
    // A release always fires a click, drag or no drag, so a swipe down the
    // card list would add whichever card it started on. The scroll it already
    // performed is what the gesture meant.
    let swiped = ends.read().any(|end| end.distance.length() > DRAG_SLOP);
    if swiped {
        pointer.clear();
        return;
    }
    for click in pointer.read() {
        let Some(press) = in_lineage(click.entity, &presses, &parents) else {
            continue;
        };
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
            Press::ToggleAuto(rule) => {
                let mut edit = prefs.edit();
                rule.toggle(&mut edit.auto);
            }
            Press::ToggleMotion => {
                let mut edit = prefs.edit();
                edit.reduce_motion = !edit.reduce_motion;
            }
            Press::ToggleRail(side, row) => prefs.edit().orders.toggle(side, row),
            Press::Focus(field) => state.lobby.focus_on(field),
            Press::ToggleRegistering => state.lobby.toggle_registering(),
            Press::Submit => {
                let request = state.lobby.submit();
                dispatch(&state, &mailbox, request);
            }
            Press::SignOut => state.lobby.sign_out(),
            Press::Refresh => {
                let request = state.lobby.refresh();
                dispatch(&state, &mailbox, request);
            }
            Press::StarterDeck => {
                let rows = starter_rows();
                let request = state.lobby.create_deck(STARTER, rows);
                dispatch(&state, &mailbox, request);
            }
            Press::SelectDeck(index) => state.lobby.select_deck(index),
            Press::Host(mode) => {
                let request = state.lobby.host(mode);
                dispatch(&state, &mailbox, request);
            }
            Press::Join(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.join(&game);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::OpenRoom(chairs) => {
                let request = state.lobby.open_room(GameMode::Open, chairs, String::new());
                dispatch(&state, &mailbox, request);
            }
            Press::JoinSeat(index, seat) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.join_seat(&game, Some(seat));
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::LeaveTable(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.leave_table(&game);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::Ready(index, ready) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.set_ready(&game, ready);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::StartRoom(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.start_room(&game);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::HandOver(index, seat) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.hand_over(&game, seat);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::SeatKind(index, seat, kind) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.set_seat(&game, seat, Some(kind), None);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::SeatAi(index, seat, profile) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request =
                        state
                            .lobby
                            .set_seat(&game, seat, None, Some(profile.to_string()));
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::SeatDeck(index, seat) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.seat_deck(&game, seat);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::PlayOffline => match crate::host::house_duel() {
                Some(host) => {
                    state.connected = true;
                    commands.insert_resource(InstalledHost(Box::new(host)));
                    opens.write(DuelCommand::Open);
                }
                None => state.lobby.say("could not start the offline duel"),
            },
            // `Leave` is only ever spawned on the finished screen, and
            // `PickerNothing` exists to stop a tap inside the picker
            // reaching the shade behind it. Neither does anything here.
            Press::Leave | Press::PickerNothing => {}
            Press::NewDeck => {
                let request = state.lobby.build_deck();
                dispatch(&state, &mailbox, request);
            }
            Press::EditDeck(index) => {
                state.pane = Pane::Deck;
                let request = state.lobby.edit_deck(index);
                dispatch(&state, &mailbox, request);
            }
            Press::DeleteDeck(index) => {
                let request = state.lobby.delete_deck(index);
                dispatch(&state, &mailbox, request);
            }
            Press::CloseBuilder => {
                if state.lobby.builder().dirty() && !state.confirm_leave {
                    state.confirm_leave = true;
                    state.lobby.say("unsaved changes — press again to leave");
                } else {
                    state.confirm_leave = false;
                    let request = state.lobby.close_builder();
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::SaveDeck => {
                let request = state.lobby.save_deck();
                dispatch(&state, &mailbox, request);
            }
            Press::FocusBuild(field) => state.lobby.builder_mut().focus_on(field),
            Press::AddCard(slot) => {
                let zone = state.lobby.builder().zone();
                if !state.lobby.builder_mut().add(slot, zone) {
                    state.lobby.say("no room for another copy of that");
                }
            }
            Press::PickPrint(slot) => {
                let zone = state.lobby.builder().zone();
                let request = state.lobby.builder_mut().open_picker(slot, zone);
                dispatch(&state, &mailbox, request);
            }
            Press::PickerStep(by) => state.lobby.builder_mut().picker_step(by),
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
            Press::PickerFinish(finish) => state.lobby.builder_mut().picker_set_finish(finish),
            Press::PickerConfirm => {
                if !state.lobby.builder_mut().picker_confirm() {
                    state.lobby.say("no room for another copy of that");
                }
            }
            Press::PickerClose => state.lobby.builder_mut().close_picker(),
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
            Press::AddCardTo(slot, zone) => {
                state.lobby.builder_mut().add(slot, zone);
            }
            Press::SetCommander(slot) => {
                state.lobby.builder_mut().set_commander(slot);
            }
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
            Press::ClearDeck => state.lobby.builder_mut().clear_deck(),
            Press::ShowPane(pane) => state.pane = pane,
            Press::Inspect(slot) => state.lobby.builder_mut().inspect(slot),
            Press::CloseCard => state.lobby.builder_mut().stop_inspecting(),
            Press::ToggleFilters => state.filters_open = !state.filters_open,
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
pub(super) struct Scrollable(pub(super) List);

/// The lists that remember where they were left.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum List {
    /// The searchable card pool.
    Pool,
    /// The deck being built.
    Deck,
    /// The tables and decks on the lobby screen.
    Table,
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
}

impl Scrolled {
    pub(crate) fn get(&self, list: List) -> f32 {
        match list {
            List::Pool => self.pool,
            List::Deck => self.deck,
            List::Table => self.table,
        }
    }

    pub(super) fn set(&mut self, list: List, at: f32) {
        match list {
            List::Pool => self.pool = at,
            List::Deck => self.deck = at,
            List::Table => self.table = at,
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
    for _ in 0..8 {
        let Some(e) = current else {
            return;
        };
        if let Ok((mut position, computed, which)) = lists.get_mut(e) {
            position.y = scrolled(
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

/// Where a list ends up after a gesture.
///
/// Bevy clamps what it *draws* but leaves [`ScrollPosition`] alone, so an
/// offset past the end would have to be unwound before the list moved again —
/// a swipe that ran off the bottom would then need the same distance back
/// before anything happened. The two sizes are physical pixels and the offset
/// is logical, which is what `scale` (a `ComputedNode`'s inverse scale factor)
/// converts between.
pub(super) fn scrolled(from: f32, by: f32, view: f32, content: f32, scale: f32) -> f32 {
    let room = (content - view).max(0.0) * scale;
    (from + by).clamp(0.0, room)
}

/// Leaves a finished game and comes back here.
pub(super) fn leave_clicks(
    mut pointer: MessageReader<Pointer<Click>>,
    presses: Query<&Press>,
    parents: Query<&ChildOf>,
    mut closes: MessageWriter<DuelCommand>,
) {
    for click in pointer.read() {
        if let Some(Press::Leave) = in_lineage(click.entity, &presses, &parents) {
            closes.write(DuelCommand::Close);
        }
    }
}

/// The lobby is on screen again: forget the seat and re-read the tables.
pub(super) fn came_back(
    mut commands: Commands,
    mut state: ResMut<LobbyState>,
    mailbox: Res<Mailbox>,
) {
    // Drops the socket (or the in-process engine) with it: a stale host would
    // keep a dead table's messages queued behind the next game's.
    commands.remove_resource::<InstalledHost>();
    state.connected = false;
    if !matches!(state.lobby.screen(), Screen::Seated(_)) {
        return;
    }
    state.lobby.unseat("the game ended");
    let request = state.lobby.refresh();
    dispatch(&state, &mailbox, request);
}

/// A component whose click means something.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Press {
    /// Put the caret in this field.
    Focus(Field),
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
    /// Save the starter deck.
    StarterDeck,
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
    /// Leave a finished game.
    Leave,
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
    /// Stop the table moving, or let it move again.
    ToggleMotion,
    /// Turn one step of the phase rail red or green.
    ToggleRail(
        baylee_client_core::automation::RailSide,
        baylee_client_core::automation::RailRow,
    ),
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
    /// Add one copy of a pool card, by its slot, to the open zone.
    AddCard(usize),
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
    /// Show the pool or the deck, on a screen with room for one.
    ShowPane(Pane),
    /// Read a card in full, by its slot in the pool.
    Inspect(usize),
    /// Open the printing picker on a pool card, by its slot.
    PickPrint(usize),
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
    /// Add the picked printing to the deck.
    PickerConfirm,
    /// Put the picker away, adding nothing.
    PickerClose,
    /// Nothing. Carried by the picker's own panel so a tap inside it is
    /// not also a tap on the shade behind it, which would close it.
    PickerNothing,
    /// Take one copy out of a named row of the deck list.
    RemoveRow(usize),
    /// Move one copy of a named row to the other list — deck to sideboard,
    /// or back. The row keeps the printing it was chosen with.
    MoveRow(usize),
    /// Add one copy of a pool card to a named list, whichever one is open.
    AddCardTo(usize, Zone),
    /// Make a pool card the deck's commander.
    SetCommander(usize),
    /// Take the commander mark off, leaving the card in the deck.
    ClearCommander,
    /// Put it away again.
    CloseCard,
    /// Show or hide the filter chips on a narrow screen.
    ToggleFilters,
}

/// The nearest [`Press`] at or above an entity, so a click on a button's
/// label counts as a click on the button.
fn in_lineage<'a>(
    entity: Entity,
    presses: &'a Query<&Press>,
    parents: &Query<&ChildOf>,
) -> Option<&'a Press> {
    let mut current = Some(entity);
    for _ in 0..6 {
        let e = current?;
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
    match state.lobby.screen() {
        Screen::Seated(_) => loading.show("Taking your seat"),
        _ if state.lobby.busy() => loading.show("Talking to the gateway"),
        _ => loading.clear(),
    }
}
