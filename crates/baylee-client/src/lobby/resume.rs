//! Restarting into an update, and coming back to where the player was
//! (`docs/client.md` §"Restarting into an update";
//! `baylee_client_core::resume` holds the file and the rules).
//!
//! **Leaving** ([`restart_on_request`]): "Restart now" writes where the
//! player is to the resume file (no secret), hands the file's nonce and the
//! session to the relaunch helper over its pipe, and quits; the update
//! installs on the way out.
//!
//! **Coming back** ([`take_resume`], then [`drive`]): a client started with
//! `--resume` reads the handoff off its stdin and the file (deleting it),
//! and if the two belong together and the file is fresh it walks the lobby
//! back: the house's game rebuilt from its record and checked against the
//! hash it had reached; or the gateway chosen, the session taken up again,
//! the chair asked for again (the gateway hands this account a new ticket
//! for its own chair, and the house held the chair meanwhile), the room,
//! the builder with its deck, the settings section. Then, at the table, how
//! it was being looked at ([`restore_the_look`]). Anything that does not
//! fit is a normal start with a short note.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use baylee_client_core::resume::{
    self, Account, Front, Game, Handoff, Place, RESUME_ARG, RESUME_FILE, ResumeState, TableLook,
};
use std::time::Duration;

/// How long [`drive`] waits for the lobby to settle before it gives up on
/// putting the screen back (a gateway that does not answer).
const SETTLE_SECS: f32 = 30.0;

/// How long the note at the table stands.
const NOTE_SECS: f32 = 8.0;

/// A restart being walked back, until it is done.
#[derive(Resource)]
pub(crate) struct Resuming {
    state: ResumeState,
    session: Option<String>,
    stage: Stage,
    /// Seconds spent waiting in [`Stage::Front`].
    waited: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Start,
    Front,
}

/// How the table was being looked at, applied once the table has a view.
#[derive(Resource)]
pub(crate) struct LookToRestore {
    look: TableLook,
    /// A hosted chair the house held: say so at the table.
    held: bool,
}

/// A line said over the table for a few seconds after a restart.
#[derive(Resource)]
pub(crate) struct TableNote {
    text: String,
    left: f32,
}

/// The note's panel.
#[derive(Component)]
pub(crate) struct TableNotePanel;

pub(super) fn install(app: &mut App) {
    app.add_message::<crate::update::UpdateRequest>()
        .add_systems(Startup, take_resume)
        // After the frame's Update: a walk that is not under way touches
        // nothing, and in Update its access alone reordered the front door's
        // unordered systems (the terms sheet's Esc focus test went red).
        .add_systems(Last, drive.run_if(resource_exists::<Resuming>))
        .add_systems(
            Update,
            (restore_the_look, show_the_note, restart_on_request),
        );
}

/// The wall clock in milliseconds, through `web_time` as everywhere here.
fn now_ms() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// At start: with `--resume`, the handoff and the file, taken or refused;
/// without it, a leftover file is removed and nothing else happens.
fn take_resume(mut commands: Commands, mut state: ResMut<LobbyState>) {
    let asked = std::env::args_os().any(|arg| arg == RESUME_ARG);
    let text = crate::settings::store::read_named(RESUME_FILE);
    if text.is_some() {
        crate::settings::store::remove_named(RESUME_FILE);
    }
    if !asked {
        return;
    }
    let handoff = baylee_update::relaunch::handoff_from_stdin(Duration::from_secs(2))
        .and_then(|bytes| Handoff::from_bytes(&bytes));
    let taken = text
        .ok_or(resume::Refused::Unreadable)
        .and_then(|text| resume::accept(&text, handoff.as_ref(), now_ms()));
    match taken {
        Ok(taken) => {
            info!("resume: picking up where the last client stopped");
            commands.insert_resource(Resuming {
                state: taken,
                session: handoff.and_then(|h| h.session),
                stage: Stage::Start,
                waited: 0.0,
            });
        }
        Err(why) => {
            info!("resume: not taken ({why:?})");
            state.lobby.tell(Phrase::ResumeFailed, &[]);
        }
    }
}

/// Walks the lobby back to where the restart left it.
#[allow(clippy::too_many_arguments)] // one walk, through everything it touches
fn drive(
    mut commands: Commands,
    resuming: Option<ResMut<Resuming>>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mailbox: Res<Mailbox>,
    mut opens: MessageWriter<DuelCommand>,
    time: Res<Time>,
) {
    let Some(mut resuming) = resuming else {
        return;
    };
    match resuming.stage {
        Stage::Start => {
            let next = start(&mut commands, &resuming, &mut state, &mut prefs, &mailbox);
            match next {
                Started::Seated => {
                    state.connected = true;
                    opens.write(DuelCommand::Open);
                    commands.remove_resource::<Resuming>();
                }
                Started::Lobby => resuming.stage = Stage::Front,
                Started::Nothing => commands.remove_resource::<Resuming>(),
            }
        }
        Stage::Front => {
            resuming.waited += time.delta_secs();
            // A game being returned to takes the screen itself: the lobby's
            // own screen is not put back under it, and the walk ends once
            // the chair is the player's again.
            if matches!(resuming.state.game, Some(Game::Hosted { .. })) {
                if matches!(state.lobby.screen(), Screen::Seated(_))
                    || resuming.waited > SETTLE_SECS
                {
                    commands.remove_resource::<Resuming>();
                }
                return;
            }
            let settled = matches!(state.lobby.screen(), Screen::Table) && !state.lobby.busy();
            if resuming.waited > SETTLE_SECS {
                commands.remove_resource::<Resuming>();
            } else if settled {
                let front = resuming.state.front.clone();
                put_the_front_back(&mut state, &mailbox, &front);
                commands.remove_resource::<Resuming>();
            }
        }
    }
}

/// What the first step did.
enum Started {
    /// A table was installed from the record: the duel opens.
    Seated,
    /// The lobby is on its way back; the screen follows once it settles.
    Lobby,
    /// Nothing more to put back.
    Nothing,
}

fn start(
    commands: &mut Commands,
    resuming: &Resuming,
    state: &mut LobbyState,
    prefs: &mut ResMut<crate::prefs::Prefs>,
    mailbox: &Mailbox,
) -> Started {
    let wanted = &resuming.state;
    if let Some(Game::Local {
        record,
        seat,
        names,
        hash,
    }) = &wanted.game
    {
        match rebuild(record, *seat, names, hash) {
            Ok(host) => {
                state.offline.get_or_insert_with(offline::Offline::load);
                state
                    .lobby
                    .resume_offline_table(baylee_client_core::lobby::SeatHandover {
                        game_id: "offline".to_string(),
                        seat: u32::from(*seat),
                        seat_token: String::new(),
                        local: true,
                    });
                commands.insert_resource(InstalledHost(Box::new(host)));
                if let Some(look) = wanted.table.clone() {
                    commands.insert_resource(LookToRestore { look, held: false });
                }
                return Started::Seated;
            }
            Err(phrase) => {
                state.lobby.tell(phrase, &[]);
                state.offline.get_or_insert_with(offline::Offline::load);
                let request = state.lobby.play_offline();
                http::dispatch(state, mailbox, request);
                return Started::Lobby;
            }
        }
    }
    if wanted.front.offline {
        state.offline.get_or_insert_with(offline::Offline::load);
        let request = state.lobby.play_offline();
        http::dispatch(state, mailbox, request);
        return Started::Lobby;
    }
    let Some(gateway) = &wanted.gateway else {
        return Started::Nothing;
    };
    let Some(index) = state.gateways.iter().position(|g| g == gateway) else {
        state.lobby.tell(Phrase::ResumeFailed, &[]);
        return Started::Nothing;
    };
    keyboard::choose_gateway(state, prefs, mailbox, index);
    let request = match (&wanted.account, &resuming.session) {
        (Some(Account { guest: true, .. }), _) => state
            .guests
            .get(gateway)
            .cloned()
            .and_then(|kept| state.lobby.apply(LobbyEvent::GuestIn(kept))),
        (Some(Account { username, .. }), Some(token)) => state.lobby.apply(LobbyEvent::LoggedIn {
            token: token.clone(),
            username: username.clone(),
        }),
        // Nobody signed in, or no session came across: the sign-in face,
        // with the name filled in as on any start.
        _ => return Started::Nothing,
    };
    if request.is_none() {
        return Started::Nothing;
    }
    if let Some(Game::Hosted { game_id, .. }) = &wanted.game {
        state.lobby.resume_seat(game_id, true);
        if let Some(look) = wanted.table.clone() {
            commands.insert_resource(LookToRestore { look, held: true });
        }
    } else if let Some(room) = &wanted.front.room {
        state.lobby.resume_seat(room, false);
    }
    http::dispatch(state, mailbox, request);
    Started::Lobby
}

/// The house's game, rebuilt from its kept record and held to the hash the
/// restart wrote down.
fn rebuild(
    record: &str,
    seat: u8,
    names: &[String],
    hash: &str,
) -> Result<crate::host::LocalHost, Phrase> {
    let bytes = crate::records::read_kept(record).ok_or(Phrase::ResumeFailed)?;
    let host = crate::host::LocalHost::resume(bytes, record, PlayerId::new(seat), names).map_err(
        |why| {
            warn!("resume: the record did not rebuild the game: {why}");
            Phrase::ResumeGameDiffers
        },
    )?;
    if host.hash() != hash {
        warn!(
            "resume: the rebuilt game is at {} and the restart left it at {hash}",
            host.hash()
        );
        return Err(Phrase::ResumeGameDiffers);
    }
    Ok(host)
}

/// The hub's tab, the settings section and the builder's deck, as they were.
/// A game being returned to takes the screen itself.
fn put_the_front_back(state: &mut LobbyState, mailbox: &Mailbox, front: &Front) {
    if front.decks_tab {
        state.hub = Hub::Decks;
    }
    if let Some(section) = front.settings.and_then(resume::section_of) {
        state.settings = SettingsPane::Open;
        state.set_settings_section(section);
    }
    if front.place == Place::Builder {
        let request = match &front.editing {
            Some(Some(id)) => state
                .lobby
                .decks()
                .iter()
                .position(|deck| deck.id == *id)
                .and_then(|index| state.lobby.edit_deck(index)),
            Some(None) => state.lobby.build_deck(),
            None => None,
        };
        http::dispatch(state, mailbox, request);
    }
}

/// Once the table has a view: the arrangement, the visit, the drawer, the
/// fold and the zone browser as they were. Written once, and only what
/// differs, so a table at rest is not written to.
fn restore_the_look(
    mut commands: Commands,
    pending: Option<Res<LookToRestore>>,
    duel: Option<ResMut<crate::Duel>>,
    settings: Option<Res<crate::settings::ClientSettings>>,
) {
    let (Some(pending), Some(mut duel)) = (pending, duel) else {
        return;
    };
    let Some((seq, seat, seats, stood_in)) = duel.view.as_ref().map(|view| {
        let stood_in = view
            .seats
            .get(usize::from(view.seat.get()))
            .is_some_and(|own| own.house_answered == Some(baylee_view::HouseAnswer::StandIn));
        (view.seq, view.seat, view.seats.len(), stood_in)
    }) else {
        return;
    };
    let look = &pending.look;
    if look.arrangement.is_some() && duel.arrangement_game != look.arrangement {
        duel.arrangement_game = look.arrangement;
    }
    let visit = look
        .visiting
        .map(PlayerId::new)
        .filter(|v| *v != seat && usize::from(v.get()) < seats);
    if visit.is_some() && duel.visiting != visit {
        duel.visiting = visit;
    }
    if duel.hand_drawer.chosen_open != look.hand_drawer_open {
        duel.hand_drawer.chosen_open = look.hand_drawer_open;
    }
    if look.sheet_folded && !duel.decision_fold.is_folded(Some(seq)) {
        duel.decision_fold.toggle(Some(seq));
    }
    if look.browser_open && !duel.browser.is_open() {
        duel.browser.toggle_by_hand();
    }
    if pending.held {
        // Held, or (past the reconnect window) played by the house: the
        // note says which, as the seat's own view does.
        let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
        let phrase = if stood_in {
            Phrase::ResumeHousePlayed
        } else {
            Phrase::ResumeSeatHeld
        };
        commands.insert_resource(TableNote {
            text: phrase.text(lang).to_owned(),
            left: NOTE_SECS,
        });
    }
    commands.remove_resource::<LookToRestore>();
}

/// The note over the table: spawned once, gone after [`NOTE_SECS`] or when
/// the table closes.
fn show_the_note(
    mut commands: Commands,
    note: Option<ResMut<TableNote>>,
    panels: Query<Entity, With<TableNotePanel>>,
    fonts: Option<Res<crate::hud::UiFonts>>,
    time: Res<Time>,
) {
    let Some(mut note) = note else {
        for panel in &panels {
            commands.entity(panel).despawn();
        }
        return;
    };
    note.bypass_change_detection().left -= time.delta_secs();
    if note.left <= 0.0 {
        commands.remove_resource::<TableNote>();
        return;
    }
    if !panels.is_empty() {
        return;
    }
    let Some(fonts) = fonts else {
        return;
    };
    commands.spawn((
        TableNotePanel,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(64.0),
            left: Val::Percent(50.0),
            max_width: Val::Px(420.0),
            padding: UiRect::all(Val::Px(12.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
        BackgroundColor(crate::hud::palette::PANEL_LIT),
        BorderColor::all(crate::hud::palette::MUTED.with_alpha(0.35)),
        GlobalZIndex(40),
        Pickable::IGNORE,
        children![(
            Text::new(note.text.clone()),
            crate::hud::tf(&fonts, 16.0),
            TextColor(crate::hud::palette::INK),
            Pickable::IGNORE,
        )],
    ));
}

/// "Restart now": where the player is goes to the resume file, the nonce
/// and the session to the helper's pipe, and the client quits. A refusal
/// is said on the notice, and the file is taken back.
#[allow(clippy::too_many_arguments)] // one snapshot, of everything the player sees
fn restart_on_request(
    mut requests: MessageReader<crate::update::UpdateRequest>,
    state: Res<LobbyState>,
    duel: Option<Res<crate::Duel>>,
    host: Option<Res<InstalledHost>>,
    phase: Option<Res<State<DuelPhase>>>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    place: Option<ResMut<crate::update::UpdatePlace>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !requests
        .read()
        .any(|r| *r == crate::update::UpdateRequest::RestartNow)
    {
        return;
    }
    let mut random = [0u8; 16];
    let _ = getrandom::fill(&mut random);
    let at_table = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let snapshot = snapshot(
        &state,
        duel.as_deref().filter(|_| at_table),
        host.as_deref().filter(|_| at_table),
        resume::nonce(random),
    );
    let text = serde_json::to_string_pretty(&snapshot).unwrap_or_default();
    crate::settings::store::write_named(RESUME_FILE, &text);
    let handoff = Handoff {
        nonce: snapshot.nonce.clone(),
        session: state.lobby.token().map(str::to_string),
    };
    match crate::update::native::restart(&handoff.to_bytes(), true) {
        Ok(()) => {
            exit.write(AppExit::Success);
        }
        Err(err) => {
            warn!("updates: could not restart: {err}");
            crate::settings::store::remove_named(RESUME_FILE);
            let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
            if let Some(mut place) = place {
                place.failed = Some(Phrase::UpdateRestartFailed.fill(lang, &[&err.to_string()]));
            }
        }
    }
}

/// Where the player is, as the resume file holds it.
fn snapshot(
    state: &LobbyState,
    duel: Option<&crate::Duel>,
    host: Option<&InstalledHost>,
    nonce: String,
) -> ResumeState {
    let lobby = &state.lobby;
    let offline = lobby.offline();
    let place = match lobby.screen() {
        Screen::SignIn { .. } if !state.gateway_selected => Place::Gateways,
        Screen::SignIn { .. } => Place::SignIn,
        Screen::Table => Place::Hub,
        Screen::Build => Place::Builder,
        Screen::Seated(_) => Place::Table,
    };
    let over = duel
        .and_then(|d| d.interaction.as_ref())
        .is_some_and(|i| matches!(i.pending(), baylee_engine::choice::Pending::GameOver(_)));
    let game = host.filter(|_| !over).and_then(|host| {
        host.0
            .resume_point()
            .map(|point| Game::Local {
                record: point.record,
                seat: point.seat.get(),
                names: point.names,
                hash: point.hash,
            })
            .or_else(|| match lobby.screen() {
                Screen::Seated(handover) if !handover.local && !handover.watching() => {
                    Some(Game::Hosted {
                        game_id: handover.game_id.clone(),
                        seat: handover.seat,
                    })
                }
                _ => None,
            })
    });
    let table = duel
        .filter(|_| game.is_some())
        .and_then(|duel| duel.view.as_ref().map(|view| (duel, view)))
        .map(|(duel, view)| TableLook {
            arrangement: duel.arrangement_game,
            visiting: duel.visiting.map(PlayerId::get),
            hand_drawer_open: duel.hand_drawer.chosen_open,
            sheet_folded: duel.decision_fold.is_folded(Some(view.seq)),
            browser_open: duel.browser.is_open(),
        });
    ResumeState {
        shape: resume::SHAPE,
        nonce,
        written_at_ms: now_ms(),
        gateway: (!offline && state.gateway_selected).then(|| state.gateway.clone()),
        account: lobby.token().filter(|_| !offline).map(|_| Account {
            username: (!lobby.guest())
                .then(|| lobby.field(Field::Username).trim().to_string())
                .filter(|name| !name.is_empty()),
            guest: lobby.guest(),
        }),
        front: Front {
            place,
            decks_tab: state.hub == Hub::Decks,
            settings: state
                .settings
                .is_open()
                .then(|| resume::section_number(state.settings_section())),
            editing: (place == Place::Builder)
                .then(|| lobby.builder().editing().map(str::to_string)),
            room: lobby.awaiting().map(|seat| seat.game_id.clone()),
            offline,
        },
        game,
        table,
    }
}
