//! Serial account work; the render thread never locks the credential connection.
use super::{
    Action, Confirmation, Hub, Lobby, LobbyEvent, LobbyState, Page, Read, Screen, Ui, Work, api,
    central,
};
use baylee_account::{Account, Error, TowerHttp, TowerPlay};
use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[cfg(test)]
mod tests;

#[derive(Default)]
pub(super) struct Worker {
    connection: Arc<Mutex<Option<TowerHttp>>>,
    inbox: Arc<Mutex<Option<Finished>>>,
}

struct Finished {
    work: Work,
    response: Option<api::Response>,
    failure: Option<Error>,
    seat: Option<TowerPlay>,
}

fn session(state: &mut LobbyState) -> Option<central::Handle> {
    let handle = state.central_handle();
    let unchanged = match (&handle, &state.echoes.session) {
        (Some(a), Some(b)) => a.same_session(b),
        (None, None) => true,
        _ => false,
    };
    if !unchanged {
        // A gateway's authenticated resume marker can precede the first Central
        // worker. Retain only that navigation intent; the new account must still
        // read/authorize the run. Never retain anything across an account departure.
        let first = state.echoes.session.is_none() && handle.is_some();
        let playing = first.then_some(state.echoes.playing).flatten();
        let returning = first.then_some(state.echoes.returning).flatten();
        // Departed jobs hold only their departed connection and mailbox.
        state.echoes = Ui {
            session: handle.clone(),
            playing,
            returning,
            opened: returning.is_some(),
            ..Ui::default()
        };
    }
    handle
}

fn ready(state: &LobbyState) -> bool {
    state.can_enter_tower()
        && !state.central_terms_pending()
        && state.terms_subject != super::super::front::terms::Subject::Central
}

/// Called only after the gateway reply passes the ordinary epoch check.
pub(in crate::lobby) fn seated(state: &mut LobbyState, event: &LobbyEvent) {
    match event {
        LobbyEvent::TowerSeated { seat, run } => {
            session(state);
            state.echoes.playing = Uuid::parse_str(&seat.game_id)
                .ok()
                .map(|game| (Uuid::from_bytes(*run), game));
        }
        LobbyEvent::Seated(_) | LobbyEvent::ChallengeSeated { .. } => {
            state.echoes.playing = None;
            state.echoes.returning = None;
        }
        _ => {}
    }
}

pub(in crate::lobby) fn ask(state: &mut LobbyState, action: Action) {
    session(state);
    if action == Action::Back {
        state.echoes.close();
        state.hub = Hub::Play;
        return;
    }
    if !ready(state) {
        return;
    }
    let ui = &mut state.echoes;
    if action == Action::Open {
        ui.opened = true;
        if !ui.model.busy() {
            ui.refresh();
        }
        return;
    }
    if ui.model.busy()
        || (ui.confirmation.is_some() && !matches!(action, Action::Accept | Action::Cancel))
    {
        return;
    }
    if navigate(ui, action) {
        return;
    }
    let id = Uuid::now_v7();
    let work = match action {
        Action::Overview if ui.choices.deck_unchanged(ui.model.run()) => {
            ui.page = Page::Overview;
            ui.refresh();
            None
        }
        Action::Refresh => {
            if ui.page == Page::Collection {
                ui.collection.refresh();
            } else {
                ui.refresh();
            }
            None
        }
        Action::Read(read) => read_page(ui, id, read),
        Action::Retry => ui.model.retry(id),
        Action::Play(host) if ui.choices.deck_unchanged(ui.model.run()) => {
            ui.model.reserve(id, host)
        }
        Action::Resume => ui.model.play(id),
        Action::Starter(cohort, starter) => {
            if !ui.model.uncertain()
                && ui.choices.deck_unchanged(ui.model.run())
                && ui
                    .model
                    .cohorts()
                    .iter()
                    .any(|c| c.id == cohort && usize::from(starter) < c.starters.len())
            {
                ui.confirmation = Some(Confirmation::Enrollment {
                    cohort,
                    starter,
                    consent: ui.model.consent(),
                });
            }
            None
        }
        Action::Accept => accept(ui, id),
        Action::Cancel => {
            ui.confirmation = None;
            None
        }
        Action::Listing(false) => ui.model.listing(id, false),
        Action::Listing(true) if !ui.model.uncertain() => {
            if let Some(board) = ui.model.ranking() {
                ui.confirmation = Some(Confirmation::Listing(board.listing.revision));
            }
            None
        }
        Action::Withdraw if !ui.model.uncertain() => {
            ui.confirmation = Some(Confirmation::Withdrawal);
            None
        }
        _ => super::choices::act(ui, action),
    };
    if let Some(work) = work {
        submit(state, work);
    }
}

fn accept(ui: &mut Ui, id: Uuid) -> Option<Work> {
    match ui.confirmation.take() {
        Some(Confirmation::Listing(revision))
            if ui
                .model
                .ranking()
                .is_some_and(|b| b.listing.revision == revision) =>
        {
            ui.model.listing(id, true)
        }
        Some(Confirmation::Withdrawal) => ui.model.withdraw(id),
        Some(Confirmation::Enrollment {
            cohort,
            starter,
            consent,
        }) => ui.model.start(id, cohort, starter, consent),
        Some(Confirmation::Choice {
            run,
            revision,
            command,
        }) if ui.model.run().is_some_and(|current| {
            current.id == run
                && current.revision == revision
                && ui.model.publication().is_some_and(|cohort| {
                    baylee_client_core::echoes::draft::allowed(current, cohort, &command)
                })
        }) =>
        {
            ui.model.choose(id, command)
        }
        _ => None,
    }
}

fn navigate(ui: &mut Ui, action: Action) -> bool {
    match action {
        Action::Page(Page::Ranking) => {
            let cohort = ui
                .model
                .run()
                .map(|r| r.cohort)
                .or_else(|| ui.model.cohorts().first().map(|c| c.id));
            if let Some(cohort) = cohort {
                ui.ranking = Some((cohort, api::ranking::Lane::Echoes));
                ui.page = Page::Ranking;
                ui.refresh();
            }
        }
        Action::Page(page) => ui.page = page,
        Action::Badge(kind) => ui.collection.selected = kind,
        Action::GoShop => ui.page = Page::Shop,
        Action::InspectCard(card) => ui.preview = Some(card),
        Action::ClosePreview => ui.preview = None,
        Action::DeckZone(zone) => {
            ui.deck.zone = zone;
            ui.deck.page = 0;
        }
        Action::DeckPage(page) => ui.deck.page = page,
        Action::Tiers(family) => ui.tiers = (ui.tiers != Some(family)).then_some(family),
        Action::InventoryPage(page) => ui.bag.page = page,
        Action::InventoryOrder(order) => {
            ui.bag.order = order;
            ui.bag.page = 0;
        }
        Action::InventoryGroups(grouped) => {
            ui.bag.grouped = grouped;
            ui.bag.page = 0;
        }
        _ => return false,
    }
    true
}

fn read_page(ui: &mut Ui, id: Uuid, read: Read) -> Option<Work> {
    if matches!(read, Read::Run(_)) && !ui.choices.deck_unchanged(ui.model.run()) {
        return None;
    }
    ui.reads.clear();
    ui.confirmation = None;
    let work = ui.model.read(id, read);
    if work.is_some()
        && let Read::Ranking(cohort, lane) = read
    {
        ui.page = Page::Ranking;
        ui.ranking = Some((cohort, lane));
    }
    if work.is_some() && matches!(read, Read::Run(_)) {
        ui.page = Page::Shop;
        ui.preview = None;
    }
    work
}

fn submit(state: &mut LobbyState, work: Work) {
    let retry = work.clone();
    let inbox = Arc::clone(&state.echoes.worker.inbox);
    let connection = Arc::clone(&state.echoes.worker.connection);
    if !state.echoes.session.as_ref().is_some_and(|handle| {
        handle.submit(move |account| {
            let mut finished = Finished {
                work,
                response: None,
                failure: None,
                seat: None,
            };
            finished.failure = execute(account, &connection, &mut finished).err();
            if let Ok(mut slot) = inbox.lock() {
                *slot = Some(finished);
            }
        })
    }) {
        state.echoes.model.receive(&retry, None, Some(Error::Busy));
        state.echoes.reads.clear();
    }
}

fn execute(
    account: &Account,
    connection: &Mutex<Option<TowerHttp>>,
    done: &mut Finished,
) -> Result<(), Error> {
    let mut connection = connection.lock().map_err(|_| Error::Unavailable)?;
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    if connection
        .as_ref()
        .is_none_or(|http| http.expires_at() <= now.saturating_add(60))
    {
        *connection = Some(account.tower()?);
    }
    let http = connection.as_ref().ok_or(Error::Unavailable)?;
    let result = (|| {
        if let Some(request) = done.work.request() {
            done.response = Some(http.exchange(request)?);
        }
        if let Some(run) = done.work.play() {
            done.seat = Some(account.play_tower(http, run)?);
        }
        Ok(())
    })();
    // Refresh only on the next explicit operation; never replay a choice here.
    if result == Err(Error::Rejected) {
        *connection = None;
    }
    result
}

pub(super) fn tick(
    time: Res<Time>,
    mut state: ResMut<LobbyState>,
    mut scrolled: ResMut<super::Scrolled>,
    mut clock: Local<f64>,
) {
    let changed = match (state.central_handle(), &state.echoes.session) {
        (Some(a), Some(b)) => !a.same_session(b),
        (None, None) => false,
        _ => true,
    };
    if changed {
        session(&mut state);
    }
    let reply = state
        .echoes
        .worker
        .inbox
        .lock()
        .ok()
        .and_then(|mut slot| slot.take());
    if let Some(reply) = reply {
        let previous = state.echoes.model.run().map(|run| (run.id, run.phase));
        let withdrawn = matches!(reply.response, Some(api::Response::Withdrawn {}));
        let enrolled = matches!(reply.work.request(), Some(api::Request::Start { .. }))
            && matches!(reply.response, Some(api::Response::Run { .. }));
        let follows = matches!(reply.response, Some(api::Response::Run { .. }));
        let loaded = matches!(reply.response, Some(api::Response::Cohorts { .. }));
        let listed = matches!(reply.response, Some(api::Response::Listing { .. }));
        if state
            .echoes
            .model
            .receive(&reply.work, reply.response, reply.failure)
        {
            if previous != state.echoes.model.run().map(|run| (run.id, run.phase)) {
                scrolled.set(super::List::Table, 0.0);
                scrolled.set(super::List::EchoesInspector, 0.0);
                state.echoes.preview = None;
            }
            state.echoes.loaded |= loaded;
            state.echoes.withdrawn |= withdrawn;
            if enrolled {
                state.echoes.withdrawn = false;
                state.echoes.page = Page::Shop;
                state.echoes.preview = None;
            }
            if let Some(run) = state.echoes.model.run().cloned() {
                state.echoes.choices.bind(&run);
            }
            if follows {
                state.echoes.follow_run();
            }
            if listed {
                state.echoes.refresh();
            }
            if reply.failure.is_some() {
                state.echoes.reads.clear();
            }
            *clock = time.elapsed_secs_f64();
            if let Some(seat) = reply.seat {
                enter(&mut state, seat);
                return;
            }
        }
    }
    if !ready(&state) || state.echoes.model.busy() {
        return;
    }
    if let Some(run) = state.echoes.returning {
        state.echoes.returning = None;
        state.hub = Hub::Challenges;
        ask(&mut state, Action::Read(Read::Run(run)));
        return;
    }
    let showing = state.hub == Hub::Challenges
        && state.echoes.opened
        && state.echoes.page != Page::Collection;
    if !showing {
        return;
    }
    let now = time.elapsed_secs_f64();
    // Even a no-op mutable borrow marks LobbyState changed and rebuilds the
    // entire page. Decide with a shared reference before requesting a refresh.
    if refresh_due(&state.echoes, now, *clock) {
        refresh_visible(&mut state.echoes, now, &mut clock);
    }
    if !state.echoes.reads.is_empty() {
        let read = state
            .echoes
            .reads
            .pop_front()
            .expect("observed nonempty queue");
        if let Some(work) = state.echoes.model.read(Uuid::now_v7(), read) {
            submit(&mut state, work);
        }
    }
}

fn refresh_due(ui: &Ui, now: f64, clock: f64) -> bool {
    ui.reads.is_empty()
        && ui.confirmation.is_none()
        && !ui.model.uncertain()
        && ui.choices.deck_unchanged(ui.model.run())
        && ((!ui.loaded && ui.model.failure().is_none()) || now - clock >= 15.0)
}

fn refresh_visible(ui: &mut Ui, now: f64, clock: &mut f64) {
    *clock = now;
    if ui.page == Page::Ranking {
        ui.refresh();
    } else if let Some(run) = ui.model.run() {
        let id = run.id;
        ui.reads.push_back(Read::Run(id));
    } else {
        ui.refresh();
    }
}

fn enter(state: &mut LobbyState, mut play: TowerPlay) {
    state.echoes.playing = Some((play.run, play.seat.game_id));
    state.echoes.reads.clear();
    state.challenges.playing = None;
    let lang = state.lobby.lang();
    state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
    state.gateway = play.host.origin;
    state.gateway_selected = true;
    state.art_cache = false;
    state.offline = None;
    state.local_decks = None;
    state.connected = false;
    state.room_away = false;
    state.lobby = Lobby::new();
    state.lobby.set_lang(lang);
    state.lobby.set_gateway_ready(true);
    let _ = state.lobby.apply(LobbyEvent::LoggedIn {
        token: std::mem::take(&mut play.admission.token),
        username: play.admission.username.take(),
    });
    state.lobby.set_central_bound(true);
    if let Some(renew_in) = play.admission.renew_in {
        let token = state.lobby.token().unwrap_or_default().to_owned();
        let _ = state
            .lobby
            .apply(LobbyEvent::BearerRenewed { token, renew_in });
    }
    state
        .lobby
        .enter_challenge(baylee_client_core::lobby::SeatHandover {
            game_id: play.seat.game_id.to_string(),
            seat: play.seat.seat,
            seat_token: std::mem::take(&mut *play.seat.seat_token),
            local: false,
        });
}

pub(in crate::lobby) fn returned(mut state: ResMut<LobbyState>) {
    if matches!(state.lobby.screen(), Screen::Replay(_)) {
        return;
    }
    let Some((run, _)) = state.echoes.playing.take() else {
        return;
    };
    state.echoes.opened = true;
    state.echoes.returning = Some(run);
    state.echoes.collection.refresh();
    state.echoes.page = Page::Shop;
    state.echoes.preview = None;
    state.hub = Hub::Challenges;
    if ready(&state) && !state.echoes.model.busy() {
        state.echoes.returning = None;
        ask(&mut state, Action::Read(Read::Run(run)));
    }
}
