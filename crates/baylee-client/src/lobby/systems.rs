//! Bevy systems: the mailbox pump, the seat watch, text entry and the
//! pointer.
//!
//! Nothing here decides anything; each system turns an input into a
//! [`LobbyEvent`] and hands it to [`baylee_client_core::lobby`].

use super::press::in_lineage;
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
                // a name is a real one, so it is the only moment worth
                // writing it down. Read off the field rather than out of the
                // request: after the sign-in the field holds the username the
                // gateway answered, which is the name to offer next time even
                // when an address was typed (#269).
                let worked = matches!(event, LobbyEvent::LoggedIn { .. });
                // A new guest is kept for this gateway, the only way back to
                // it (#269), and is a use of the gateway like a sign-in.
                let guest_in = match &event {
                    LobbyEvent::GuestIn(kept) => Some(kept.clone()),
                    _ => None,
                };
                let next = state.lobby.apply(event);
                if let Some(kept) = guest_in {
                    let gateway = state.gateway.clone();
                    state.uses.record(&gateway);
                    state.guests.insert(gateway, kept);
                    if let Some(settings) = settings.as_mut() {
                        keep_gateways(&state, settings);
                    }
                }
                if worked {
                    // The use is counted before anything is written, so
                    // that the name and the use go out in one save.
                    let gateway = state.gateway.clone();
                    state.uses.record(&gateway);
                    if let Some(settings) = settings.as_mut() {
                        settings.last_username = state.lobby.field(Field::Username).to_string();
                        keep_gateways(&state, settings);
                    }
                }
                dispatch(&mut state, &mailbox, next);
            }
            Reply::Registration {
                registration,
                art_cache,
                guests,
            } => {
                state.lobby.set_registration(registration);
                state.lobby.set_guests_enabled(guests);
                state.art_cache = art_cache;
            }
            Reply::Expired => {
                state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
                // A guest the gateway no longer knows is gone for good, and
                // so is this device's way back to it.
                let guest = state.lobby.guest();
                state.lobby.session_ended();
                if guest {
                    let gateway = state.gateway.clone();
                    state.guests.remove(&gateway);
                    if let Some(settings) = settings.as_mut() {
                        keep_gateways(&state, settings);
                    }
                }
            }
            // Written only when they change: the pill reads them, and a
            // write is a rebuild (§10 #1).
            Reply::Me(me) => {
                if state.lobby.me() != Some(&me) {
                    state.lobby.set_me(me);
                }
            }
            Reply::Stats(stats) => {
                if state.lobby.stats() != Some(&stats) {
                    state.lobby.set_stats(stats);
                }
            }
            Reply::Terms(reply) => {
                super::front::terms::receive(reply, &mut state, &mailbox, &mut settings);
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
#[allow(clippy::too_many_lines, clippy::too_many_arguments)] // Platform input routing across lobby and editor fields.
pub(super) fn softkeys(
    mut keys: ResMut<SoftKeyboard>,
    mut state: ResMut<LobbyState>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut epoch: Local<u64>,
    mut build_epoch: Local<u64>,
    entrance: Res<super::entrance::Entrance>,
    journey: Option<Res<crate::arrival::Journey>>,
) {
    if !SoftKeyboard::owns_typing() {
        return;
    }
    if journey.as_ref().is_some_and(|j| j.active())
        || entrance.active()
        || state.lobby.library().page.is_some()
        || state.confirmation.is_some()
    {
        keys.close();
        drop(keys.drain());
        return;
    }
    // The builder counts its own placements, so it gets its own tally: one
    // shared counter would open the keyboard on the way between the screens.
    if matches!(state.lobby.screen(), Screen::Build) || state.lobby.builder().picker().is_some() {
        let builder = state.lobby.builder();
        // The import and export dialogs have no field; the phone keyboard
        // stays down while one stands over the builder.
        if builder.picker().is_some_and(|p| !p.set_open()) || builder.transfer().is_some() {
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
                if let Field::RoomBoard(seat) = state.lobby.focus() {
                    if let Some(slot) = state.lobby.room_matches(seat).first().copied() {
                        state.lobby.room_add_card(seat, slot);
                    }
                    keys.close();
                    continue;
                }
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

/// Where card art comes from for the lobby as it stands, and the session the
/// gateway's mirror is shown: the mirror while signed in to a gateway that
/// has one, else Scryfall.
pub(super) fn art_source(state: &LobbyState) -> (Option<String>, Option<&str>) {
    let token = state.lobby.token();
    let mirror = (token.is_some() && state.art_cache)
        .then(|| client_core::images::gateway_art_base(&state.gateway));
    (mirror, token)
}

/// The gateway card text is asked of, and the session it is asked with:
/// the one signed in to, else none (#270).
pub(super) fn text_source(state: &LobbyState) -> Option<crate::cardtext::SignedIn> {
    state.lobby.token().map(|token| crate::cardtext::SignedIn {
        base: state.gateway.clone(),
        token: token.to_string(),
    })
}

/// Keeps the gateway card text is asked of with the lobby's session (#270).
///
/// `/catalog/text` serves only a session, as the art mirror does, so a
/// client that is not signed in to its gateway, offline play among them,
/// takes text from Scryfall and shows the gateway nothing. It reads the
/// lobby as [`art_follows_the_session`] does, so the two cannot disagree.
pub(super) fn text_follows_the_session(
    state: Res<LobbyState>,
    mut text: ResMut<crate::cardtext::TextGateway>,
) {
    if state.is_changed() {
        text.set_if_neq(crate::cardtext::TextGateway(text_source(&state)));
    }
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

/// Writes the saved gateways, their uses and the guests kept at them back
/// to the settings file.
pub(super) fn keep_gateways(state: &LobbyState, settings: &mut crate::settings::ClientSettings) {
    settings.gateways.clone_from(&state.gateways);
    settings.gateway_uses.clone_from(&state.uses);
    settings.guests.clone_from(&state.guests);
    settings.save();
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
    exits: Query<&Press, With<super::DuelExit>>,
    mut state: ResMut<LobbyState>,
    mut closes: MessageWriter<DuelCommand>,
    desk: Option<Res<crate::report::ReportDesk>>,
) {
    use baylee_client_core::prefs::Action;
    // The report form opens over the end screen too (F8, the corner
    // button): while it is up, `Esc` shuts the form and `Enter` is a line
    // break in its text, and neither is an answer to this sheet.
    if desk.is_some_and(|desk| desk.holds_keyboard()) {
        return;
    }
    let fired = crate::keys::Fired::of(&keys, prefs.keymap());
    if fired.quiet() {
        return;
    }
    let mut leave = false;
    let mut again = false;
    for press in &exits {
        match press {
            Press::End(EndPress::Leave) => leave = true,
            Press::End(EndPress::PlayAgain) => again = true,
            _ => {}
        }
    }
    // `ui::spawn_leave_button` puts *play again* first where there is one, and
    // the first answer on a slip is the lead — so the lead is the rematch when
    // the sheet has one and the way back otherwise.
    let lead = if again {
        Press::End(EndPress::PlayAgain)
    } else if leave {
        Press::End(EndPress::Leave)
    } else {
        return;
    };
    let way = if fired.has(Action::Cancel) && leave {
        Press::End(EndPress::Leave)
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
        Press::End(EndPress::Leave) => {}
        Press::End(EndPress::PlayAgain) => {
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

/// Raises the loading veil while the lobby is waiting on the network.
///
/// Two waits, and the second is the one that needs saying. A request in
/// flight is usually a blink. Taking a seat is not: the gateway orders an
/// engine and the socket may wait up to thirty seconds for it to attach, and
/// a screen that says nothing for thirty seconds is a screen a player will
/// click again.
pub(super) fn waiting(state: Res<LobbyState>, mut loading: ResMut<crate::loading::Loading>) {
    let lang = state.lobby.lang();
    let want = match state.lobby.screen() {
        Screen::Seated(_) => Some(Phrase::VeilTakingSeat),
        // Re-reading lists already on screen raises no veil (§10 #7): the
        // list keeps standing and is replaced when the answer lands.
        _ if state.lobby.refreshing() => None,
        // Offline the wait is this process building a deck, a room or an
        // engine, and a veil claiming a conversation with a gateway would
        // be naming a machine that was never dialled.
        _ if state.lobby.busy() && state.lobby.offline() => Some(Phrase::VeilWorking),
        _ if state.lobby.busy() => Some(Phrase::VeilTalking),
        _ => None,
    }
    .map(|phrase| phrase.text(lang));
    // Written only when it differs: this runs every frame.
    if loading.what() != want {
        match want {
            Some(what) => loading.show(what),
            None => loading.clear(),
        }
    }
}
