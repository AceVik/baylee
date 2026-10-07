//! The pointer: a click on a lobby control turned into an intent.

use super::keyboard::choose_gateway;
use super::press::in_lineage;
use super::systems::keep_gateways;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// Signs out: the session ends on the gateway as well as here, and a guest's
/// with the guest, which this device then keeps no longer (#269).
///
/// Offline has no account to forget, so the same button is what leaves
/// offline play — and the performer has to go with it, or the sign-in form's
/// own requests would still be answered out of the local deck file.
fn sign_out(
    state: &mut LobbyState,
    prefs: &mut crate::prefs::Prefs,
    scrolled: &mut Scrolled,
    mailbox: &Mailbox,
    settings: &mut Option<ResMut<crate::settings::ClientSettings>>,
) {
    let guest = state.lobby.guest();
    state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
    prefs.detach();
    scrolled.set(List::Table, 0.0);
    state.offline = None;
    let ending = state.lobby.sign_out();
    if guest {
        let gateway = state.gateway.clone();
        state.guests.remove(&gateway);
        if let Some(settings) = settings.as_mut() {
            keep_gateways(state, settings);
        }
    }
    dispatch(state, mailbox, ending);
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
    entrance: Res<super::entrance::Entrance>,
    journey: Option<Res<crate::arrival::Journey>>,
) {
    // A panel on its way out or in answers nothing: what is under the
    // pointer is half of a form that is going, or not yet there.
    if journey.as_ref().is_some_and(|j| j.active()) || motion.moving() || entrance.active() {
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
        //
        // Every write in this preamble is guarded: taking `&mut` out of the
        // state marks it changed, and a changed state rebuilds the whole tree
        // (§10 #1 of the shell design) — so a press that changes nothing
        // must not write anything either (`a_press_that_changes_nothing_marks_nothing`).
        if *press != Press::CloseBuilder && state.confirm_leave {
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
        if state.settings.capturing().is_some() && !matches!(*press, Press::Rebind(_)) {
            state.settings = SettingsPane::Open;
        }
        // And anything but the seat panel's own controls takes the caret out
        // of its box.
        if !matches!(*press, Press::Seat(_) | Press::SeatKey(_)) && state.seat.typing() {
            state.seat.blur();
        }
        match *press {
            Press::Hub(hub) => {
                if state.hub != hub {
                    state.hub = hub;
                    scrolled.set(List::Table, 0.0);
                }
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
            Press::OpenSettings if !state.settings.is_open() => {
                state.settings = SettingsPane::Open;
            }
            Press::CloseSettings if state.settings.is_open() => {
                state.settings = SettingsPane::Closed;
            }
            Press::Seat(act) => state.seat.act(act),
            Press::SeatKey(key) => state.seat.key_press(key),
            Press::AskToDeleteAccount => state.lobby.ask_to_delete_account(),
            Press::CancelAccountDeletion => state.lobby.cancel_account_deletion(),
            Press::ConfirmAccountDeletion => {
                let request = state.lobby.delete_account();
                dispatch(&mut state, &mailbox, request);
            }
            Press::OpenSource => super::source::open(&state),
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
            // A choice already made is no edit: `edit` schedules a write-back
            // to the gateway as well as marking the preferences changed.
            Press::PickSky(mode) if prefs.all().sky != mode => prefs.edit().sky = mode,
            Press::PickSound(level) if prefs.all().sound != level => prefs.edit().sound = level,
            Press::PickAtmosphere(air) if prefs.all().atmosphere != air => {
                prefs.edit().atmosphere = air;
            }
            Press::PickLang(lang) if lang == state.lobby.lang() => {}
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
            // A guest is asked first: signed out, it is gone (#269).
            Press::SignOut if state.lobby.guest() => {
                state.confirmation = Some(confirm::Destructive::SignOutGuest);
            }
            Press::SignOut => {
                sign_out(
                    &mut state,
                    &mut prefs,
                    &mut scrolled,
                    &mailbox,
                    &mut settings,
                );
            }
            Press::PlayAsGuest => {
                let request = state.lobby.play_as_guest();
                // A kept guest is back at once, with no answer to wait for:
                // that is the use of the gateway.
                if state.lobby.guest() {
                    let gateway = state.gateway.clone();
                    state.uses.record(&gateway);
                    if let Some(settings) = settings.as_mut() {
                        keep_gateways(&state, settings);
                    }
                }
                dispatch(&mut state, &mailbox, request);
            }
            Press::Refresh => {
                let request = state.lobby.refresh();
                dispatch(&mut state, &mailbox, request);
            }
            Press::Search => {
                let request = state.lobby.search_again();
                dispatch(&mut state, &mailbox, request);
            }
            Press::ClearSearch => {
                state.lobby.set_field(Field::Search, "");
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
            Press::RoomCardAdd(seat, slot, printing) => {
                let count = state
                    .lobby
                    .room_draft()
                    .and_then(|d| d.setup.seats.get(usize::from(seat)))
                    .map_or(0, |s| s.permanents.len());
                state.lobby.room_add_card(seat, slot);
                if printing {
                    let request = state.lobby.room_pick_print(seat, count);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::RoomCardRemove(seat, at) => {
                state.room_card_edit = None;
                state.lobby.room_remove_card(seat, at);
            }
            Press::RoomCardEdit(seat, at) => {
                state.room_card_edit = if state.room_card_edit == Some((seat, at)) {
                    None
                } else {
                    Some((seat, at))
                };
            }
            Press::RoomCardPrint(seat, at) => {
                let request = state.lobby.room_pick_print(seat, at);
                dispatch(&mut state, &mailbox, request);
            }
            Press::RoomCounterAdd(seat, at) => state.lobby.room_add_counter(seat, at),
            Press::RoomCounterStep(seat, at, counter, delta) => {
                state.lobby.room_counter_step(seat, at, counter, delta);
            }
            Press::RoomDeckPicker(seat) => {
                state.room_deck_seat = (state.room_deck_seat != Some(seat)).then_some(seat);
            }
            Press::RoomSetup(seat) => {
                state.room_setup_seat = (state.room_setup_seat != Some(seat)).then_some(seat);
                state.room_card_edit = None;
                // Collapsing or switching editors must not leave a hidden
                // card search (or counter field) receiving keyboard input.
                if matches!(
                    state.lobby.focus(),
                    Field::RoomBoard(_) | Field::RoomCounter
                ) {
                    state.lobby.focus_on(Field::RoomName);
                }
            }
            Press::RoomAdjust(change) => state.lobby.adjust_room(change),
            Press::SaveRoom(remove_password) => {
                let request = state.lobby.save_room(remove_password);
                dispatch(&mut state, &mailbox, request);
            }
            Press::RoomDeck(index, seat, deck) => {
                state.lobby.select_deck(deck);
                state.room_deck_seat = None;
                if let Some(game) = state.lobby.games().get(index).map(|g| g.id.clone()) {
                    let request = state.lobby.seat_deck(&game, seat);
                    dispatch(&mut state, &mailbox, request);
                }
            }
            Press::OpenRoom(chairs) => {
                state.room_setup_seat = None;
                state.room_card_edit = None;
                state.room_deck_seat = None;
                let request = state.lobby.open_room(GameMode::Open, chairs, String::new());
                dispatch(&mut state, &mailbox, request);
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
            Press::RoomLlm(index, seat, press) => {
                let found = state.lobby.games().get(index).map(|g| {
                    let phase = if g.state == "waiting" {
                        crate::tableseats::Phase::Waiting
                    } else {
                        crate::tableseats::Phase::Playing
                    };
                    let house = g
                        .seats
                        .iter()
                        .any(|s| s.seat == seat && s.kind == SeatKind::Ai);
                    (g.id.clone(), phase, house)
                });
                if let Some((game, phase, house)) = found {
                    let open_it = state.llm.press(seat, press, phase, "steady");
                    // The house's chair is the gateway's: open it, and the
                    // bridge takes it once the room lists it open.
                    if open_it && house {
                        let request =
                            state
                                .lobby
                                .set_seat(&game, seat, Some(SeatKind::Human), None);
                        dispatch(&mut state, &mailbox, request);
                    }
                }
            }
            Press::SeatKind(index, seat, kind) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    // Open or the house's: no language model of ours here.
                    state.llm.unplan(seat);
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
            // Game-over actions are handled by `leave_clicks`; a press on
            // the transfer dialog's panel only keeps it from reaching the
            // shade behind. An empty part of the artwork dialog dismisses
            // its set autocomplete.
            Press::Leave
            | Press::PlayAgain
            | Press::TransferNothing
            | Press::OpenSettings
            | Press::CloseSettings
            | Press::PickSky(_)
            | Press::PickSound(_)
            | Press::PickAtmosphere(_) => {}
            Press::PickerNothing => state.lobby.builder_mut().picker_close_sets(),
            Press::NewDeck => {
                state.commander_pick = None;
                state.pane = Pane::Deck;
                let request = state.lobby.build_deck();
                dispatch(&mut state, &mailbox, request);
            }
            Press::ImportDeck => {
                state.commander_pick = None;
                state.pane = Pane::Deck;
                let request = state.lobby.build_deck();
                dispatch(&mut state, &mailbox, request);
                if matches!(state.lobby.screen(), Screen::Build) {
                    scrolled.set(List::Transfer, 0.0);
                    state.lobby.builder_mut().open_import();
                }
            }
            Press::OpenImport => {
                scrolled.set(List::Transfer, 0.0);
                state.lobby.builder_mut().open_import();
            }
            Press::OpenExport => {
                scrolled.set(List::Transfer, 0.0);
                state.lobby.builder_mut().open_export();
            }
            Press::TransferClose => state.lobby.builder_mut().close_transfer(),
            Press::ImportPaste => state
                .transfer_asks
                .push(crate::buildui::transfer::Ask::Paste),
            Press::ImportClear => state.lobby.builder_mut().import_clear(),
            Press::ImportTake => {
                let lang = state.lobby.lang();
                state.lobby.builder_mut().import_confirm(lang);
                scrolled.set(List::Deck, 0.0);
            }
            Press::ExportFormat(format) => state.lobby.builder_mut().export_choose(format),
            Press::ExportCopy => state
                .transfer_asks
                .push(crate::buildui::transfer::Ask::Copy),
            Press::ExportSave => state
                .transfer_asks
                .push(crate::buildui::transfer::Ask::Save),
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
            Press::ConfirmDestructive
                if matches!(state.confirmation, Some(confirm::Destructive::SignOutGuest)) =>
            {
                state.confirmation = None;
                sign_out(
                    &mut state,
                    &mut prefs,
                    &mut scrolled,
                    &mailbox,
                    &mut settings,
                );
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
            Press::PickCommanderPrint(slot) => {
                let Some(at) = state.lobby.builder().commander_row(slot) else {
                    continue;
                };
                scrolled.set(List::PickerPanel, 0.0);
                let request = state.lobby.builder_mut().open_row_picker(at, Zone::Main);
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
                if !state.lobby.room_confirm_print() && !state.lobby.builder_mut().picker_confirm()
                {
                    state.lobby.tell_refusal(Phrase::NoRoomForCopy, &[]);
                }
            }
            Press::PickerClose => state.lobby.room_close_print(),
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
