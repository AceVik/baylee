//! Serial account work; the render thread never locks the credential connection.
use super::{
    Action, Confirmation, Hub, Lobby, LobbyEvent, LobbyState, Page, Read, Screen, Ui, Work, api,
    central,
};
use baylee_account::{Account, Error, TowerHttp, TowerPlay};
use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

mod socket;
#[cfg(test)]
mod tests;

#[derive(Default)]
pub(super) struct Worker {
    connection: Arc<Mutex<Option<TowerHttp>>>,
    inbox: Arc<Mutex<Option<Finished>>>,
    socket: Mutex<socket::Live>,
}

/// Whether Central delegates a Tower: asked once per account, before the
/// Tower tile may stand (`Ui::tower_reached`).
#[derive(Default)]
pub(super) struct Reach {
    /// The question went out to the account's worker.
    asked: bool,
    /// What the worker found, waiting to be read.
    answer: Arc<Mutex<Option<bool>>>,
    /// The answer, once read.
    pub(super) known: Option<bool>,
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
        if state.lobby.close_scoped_builder() {
            state.build = crate::buildui::BuildUi::default();
            state.confirm_leave = false;
        }
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
    if action == Action::Page(Page::Deck)
        && state
            .echoes
            .model
            .run()
            .is_some_and(|run| run.cards.formation.is_some())
    {
        super::builder::open(state);
        return;
    }
    let ui = &mut state.echoes;
    if action == Action::Open {
        if !ui.opened {
            ui.sounds.push(crate::sound::tower::TowerSound::Open);
        }
        ui.opened = true;
        if !ui.model.busy() {
            ui.refresh();
        }
        return;
    }
    if ui.holding()
        || (ui.confirmation.is_some() && !matches!(action, Action::Accept | Action::Cancel))
        || ((ui.starter_preview.is_some() || ui.preview.is_some())
            && action != Action::ClosePreview)
    {
        return;
    }
    if navigate(ui, action) {
        ui.sounds.push(match action {
            Action::Select(_)
            | Action::Offer(_)
            | Action::InspectCard(_)
            | Action::InspectStarter(..) => crate::sound::tower::TowerSound::Select,
            _ => crate::sound::tower::TowerSound::Tab,
        });
        return;
    }
    let work = choose_work(ui, action);
    if let Some(work) = work {
        submit(state, work);
    } else if matches!(
        action,
        Action::Select(_) | Action::Offer(_) | Action::Reward(_) | Action::Material(_)
    ) {
        state
            .echoes
            .sounds
            .push(crate::sound::tower::TowerSound::Select);
    }
}

fn choose_work(ui: &mut Ui, action: Action) -> Option<Work> {
    let id = Uuid::now_v7();
    match action {
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
                ui.confirmation = Some(Confirmation::Enrol {
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
    }
}

fn accept(ui: &mut Ui, id: Uuid) -> Option<Work> {
    if ui.model.busy() {
        return None;
    }
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
        // The shown explanation must still be the one the authority asks for.
        Some(Confirmation::Enrol {
            cohort,
            starter,
            consent,
        }) if consent == ui.model.consent() => ui.model.start(id, cohort, starter, consent),
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
                ui.preview = None;
                ui.starter_preview = None;
                ui.refresh();
            }
        }
        Action::Page(page) => {
            ui.page = page;
            ui.preview = None;
            ui.starter_preview = None;
        }
        Action::Badge(kind) => ui.collection.selected = kind,
        Action::GoShop => {
            ui.page = Page::Shop;
            ui.preview = None;
            ui.starter_preview = None;
        }
        Action::InspectCard(card) => {
            ui.starter_preview = None;
            ui.preview = Some(card);
        }
        Action::InspectStarter(cohort, starter) => {
            ui.preview = None;
            ui.starter_preview = Some((cohort, starter));
        }
        Action::ClosePreview => {
            ui.preview = None;
            ui.starter_preview = None;
        }
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
        Action::ToggleSpareCards => ui.spare_cards_open = !ui.spare_cards_open,
        Action::ToggleRewardGold => ui.reward_gold_open = !ui.reward_gold_open,
        Action::ToggleRewardDeck => ui.reward_deck_open = !ui.reward_deck_open,
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
    if ui.model.busy() {
        match read {
            Read::Ranking(cohort, lane) if cohort.get_version_num() == 7 => {
                ui.page = Page::Ranking;
                ui.ranking = Some((cohort, lane));
                ui.preview = None;
                ui.starter_preview = None;
            }
            Read::Run(run)
                if run.get_version_num() == 7
                    && (!ui.model.uncertain() || ui.model.run().map(|r| r.id) == Some(run)) =>
            {
                ui.page = Page::Shop;
                ui.preview = None;
                ui.starter_preview = None;
            }
            _ => {}
        }
        ui.reads.push_front(read);
        return None;
    }
    let work = ui.model.read(id, read);
    if work.is_some()
        && let Read::Ranking(cohort, lane) = read
    {
        ui.page = Page::Ranking;
        ui.ranking = Some((cohort, lane));
        ui.preview = None;
        ui.starter_preview = None;
    }
    if work.is_some() && matches!(read, Read::Run(_)) {
        ui.page = Page::Shop;
        ui.preview = None;
        ui.starter_preview = None;
    }
    work
}

pub(super) fn submit(state: &mut LobbyState, work: Work) {
    if work.request().is_some() {
        let result = state
            .echoes
            .worker
            .socket
            .lock()
            .map_err(|_| Error::Unavailable)
            .and_then(|mut live| {
                live.enqueue(
                    work.clone(),
                    state.echoes.session.as_ref(),
                    &state.echoes.worker.connection,
                )
            });
        if let Err(error) = result {
            state.echoes.model.receive(&work, None, Some(error));
            state.echoes.reads.clear();
        }
        return;
    }
    admit(
        state,
        Finished {
            work,
            response: None,
            failure: None,
            seat: None,
        },
    );
}

fn admit(state: &mut LobbyState, mut finished: Finished) {
    let work = finished.work.clone();
    let retry = work.clone();
    let inbox = Arc::clone(&state.echoes.worker.inbox);
    let connection = Arc::clone(&state.echoes.worker.connection);
    if !state.echoes.session.as_ref().is_some_and(|handle| {
        handle.submit(move |account| {
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

/// Asks Central once per account whether it delegates a Tower at all (owner,
/// 27.11.2026: Tower Echoes is not live for the open beta, and a tile that
/// opens onto an error must not stand). A Tower Central cannot issue a token
/// for, for want of one configured, answers `Unavailable`; any other answer
/// (a refusal for terms or restoration included) means one is there, and the
/// connection a yes brings is kept for the first visit. Only the answer
/// marks the lobby changed, so an idle lobby rebuilds nothing.
fn reach(state: &mut ResMut<LobbyState>) {
    if state.echoes.reach.known.is_some() {
        return;
    }
    let found = state
        .echoes
        .reach
        .answer
        .lock()
        .ok()
        .and_then(|mut slot| slot.take());
    if let Some(live) = found {
        state.echoes.reach.known = Some(live);
        return;
    }
    // Asked where the answer is drawn, the lobby's play screen with its
    // tiles, and nowhere else: the account worker has other work on the
    // Tower's own pages, and a lobby never shown needs no answer.
    let tiles = matches!(state.lobby.screen(), Screen::Table) && state.showing_play();
    if state.echoes.reach.asked || !tiles || !ready(state) {
        return;
    }
    let Some(handle) = state.echoes.session.clone() else {
        return;
    };
    let answer = Arc::clone(&state.echoes.reach.answer);
    let connection = Arc::clone(&state.echoes.worker.connection);
    let asked = handle.submit(move |account| {
        let live = match account.tower() {
            Ok(http) => {
                if let Ok(mut slot) = connection.lock()
                    && slot.is_none()
                {
                    *slot = Some(http);
                }
                true
            }
            Err(Error::Unavailable) => false,
            Err(_) => true,
        };
        if let Ok(mut slot) = answer.lock() {
            *slot = Some(live);
        }
    });
    state.bypass_change_detection().echoes.reach.asked = asked;
}

pub(super) fn tick(
    time: Res<Time>,
    mut state: ResMut<LobbyState>,
    mut scrolled: ResMut<super::Scrolled>,
    mut sounds: Option<MessageWriter<crate::sound::tower::TowerSound>>,
    mut clock: Local<f64>,
) {
    if !state.echoes.sounds.is_empty() {
        let drained = std::mem::take(&mut state.bypass_change_detection().echoes.sounds);
        if let Some(sounds) = sounds.as_mut() {
            for cue in drained {
                sounds.write(cue);
            }
        }
    }
    let changed = match (state.central_handle(), &state.echoes.session) {
        (Some(a), Some(b)) => !a.same_session(b),
        (None, None) => false,
        _ => true,
    };
    if changed {
        session(&mut state);
    }
    reach(&mut state);
    let showing = state.hub == Hub::Challenges
        && state.echoes.opened
        && state.echoes.page != Page::Collection
        && ready(&state);
    socket_updates(&mut state, showing);
    let reply = state
        .echoes
        .worker
        .inbox
        .lock()
        .ok()
        .and_then(|mut slot| slot.take());
    if let Some(reply) = reply
        && apply_reply(
            &mut state,
            &mut scrolled,
            sounds.as_mut(),
            &mut clock,
            time.elapsed_secs_f64(),
            reply,
        )
    {
        return;
    }
    if !state.echoes.model.busy() && super::builder::pending(&state) {
        super::builder::reconcile(&mut state, time.elapsed_secs_f64());
    }
    if !ready(&state) || state.echoes.model.busy() {
        return;
    }
    // Decide with a shared reference first: a mutable borrow alone rebuilds the page.
    if state.echoes.queued.is_some() {
        if let Some(work) = state.echoes.release() {
            submit(&mut state, work);
        }
        return;
    }
    if let Some(run) = state.echoes.returning {
        state.echoes.returning = None;
        state.hub = Hub::Challenges;
        ask(&mut state, Action::Read(Read::Run(run)));
        if !state.echoes.loaded {
            state.echoes.reads.push_back(Read::Cohorts(None));
        }
        return;
    }
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

fn apply_reply(
    state: &mut LobbyState,
    scrolled: &mut super::Scrolled,
    mut sounds: Option<&mut MessageWriter<crate::sound::tower::TowerSound>>,
    clock: &mut f64,
    now: f64,
    reply: Finished,
) -> bool {
    let previous = state.echoes.model.run().map(|run| (run.id, run.phase));
    let was_frozen = state.echoes.model.run().map(|run| run.shop.frozen);
    let command_kind = match reply.work.request() {
        Some(api::Request::Command { command, .. }) => Some(command.clone()),
        _ => None,
    };
    let withdrawn = matches!(reply.response, Some(api::Response::Withdrawn {}));
    let enrolled = matches!(reply.work.request(), Some(api::Request::Start { .. }))
        && matches!(reply.response, Some(api::Response::Run { .. }));
    let follows = matches!(reply.response, Some(api::Response::Run { .. }));
    let loaded = matches!(reply.response, Some(api::Response::Cohorts { .. }));
    let listed = matches!(reply.response, Some(api::Response::Listing { .. }));
    let resume = matches!(reply.response, Some(api::Response::Runs { .. }))
        && state.echoes.model.run().is_none();
    let failed = reply.failure.is_some();
    if !state
        .echoes
        .model
        .receive(&reply.work, reply.response, reply.failure)
    {
        return false;
    }
    let now_run = state.echoes.model.run().map(|run| (run.id, run.phase));
    if previous != now_run {
        scrolled.set(super::List::Table, 0.0);
        scrolled.set(super::List::EchoesInspector, 0.0);
        state.echoes.preview = None;
        state.echoes.starter_preview = None;
        state.echoes.spare_cards_open = false;
        state.echoes.reward_gold_open = false;
        state.echoes.reward_deck_open = false;
        if matches!(now_run, Some((_, api::Phase::Reward)))
            && let Some(sounds) = sounds.as_deref_mut()
        {
            sounds.write(crate::sound::tower::TowerSound::RewardReveal);
        }
    }
    if let Some(sounds) = sounds.as_deref_mut() {
        reply_sound(sounds, failed, enrolled, command_kind.as_ref(), was_frozen);
    }
    state.echoes.loaded |= loaded;
    if resume {
        resume_active(&mut state.echoes);
    }
    state.echoes.withdrawn |= withdrawn;
    if enrolled {
        state.echoes.withdrawn = false;
        state.echoes.page = Page::Shop;
        state.echoes.preview = None;
        state.echoes.starter_preview = None;
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
    if failed {
        state.echoes.reads.clear();
    }
    *clock = now;
    if let Some(seat) = reply.seat {
        if let Some(sounds) = sounds {
            sounds.write(crate::sound::tower::TowerSound::StageEnter);
        }
        enter(state, seat);
        return true;
    }
    false
}

fn reply_sound(
    sounds: &mut MessageWriter<crate::sound::tower::TowerSound>,
    failed: bool,
    enrolled: bool,
    command_kind: Option<&super::domain::v2::run::Command>,
    was_frozen: Option<bool>,
) {
    use super::domain::v2::run::Command as Cmd;
    use crate::sound::tower::TowerSound;
    if failed {
        sounds.write(TowerSound::Refused);
    } else if enrolled {
        sounds.write(TowerSound::DraftStart);
    } else if let Some(cmd) = command_kind {
        match cmd {
            Cmd::Refresh {} => {
                sounds.write(TowerSound::Reroll);
            }
            Cmd::Freeze { frozen } if was_frozen != Some(*frozen) => {
                sounds.write(if *frozen {
                    TowerSound::Freeze
                } else {
                    TowerSound::Thaw
                });
            }
            Cmd::Claim { .. } => {
                sounds.write(TowerSound::RewardClaim);
            }
            Cmd::SellCard { .. } => {
                sounds.write(TowerSound::Sell);
            }
            _ => {}
        }
    }
}

fn resume_active(ui: &mut Ui) {
    if matches!(ui.reads.front(), Some(Read::Run(_))) {
        return;
    }
    if let Some(run) = ui
        .model
        .runs()
        .iter()
        .find(|run| {
            matches!(
                run.phase,
                api::Phase::Shop
                    | api::Phase::Reward
                    | api::Phase::Reserved
                    | api::Phase::Playing
                    | api::Phase::Verifying
            )
        })
        .map(|run| run.id)
    {
        ui.reads.push_front(Read::Run(run));
        if !matches!(ui.page, Page::Collection | Page::Ranking) {
            ui.page = Page::Shop;
            ui.preview = None;
            ui.starter_preview = None;
        }
    }
}

fn socket_updates(state: &mut ResMut<'_, LobbyState>, showing: bool) {
    // Keep the ResMut wrapper: an idle socket must not mark the lobby changed.
    let updates = state.echoes.worker.socket.lock().ok().map(|mut live| {
        live.poll(
            showing,
            state.echoes.session.as_ref(),
            &state.echoes.worker.connection,
        )
    });
    if let Some(updates) = updates {
        for response in updates.pushed {
            if state.echoes.model.push_changes(&response) {
                let follows = matches!(response, api::Response::Run { .. });
                state.echoes.model.push(response);
                if follows {
                    state.echoes.follow_run();
                }
            }
        }
        if let Some(done) = updates.finished {
            if done.failure.is_none() && done.work.play().is_some() {
                admit(state, done);
            } else if let Ok(mut inbox) = state.echoes.worker.inbox.lock() {
                *inbox = Some(done);
            }
        }
        if updates.resync && !state.echoes.model.busy() && !state.echoes.model.uncertain() {
            state.echoes.refresh();
        }
    }
}

fn refresh_due(ui: &Ui, now: f64, clock: f64) -> bool {
    ui.reads.is_empty()
        && ui.confirmation.is_none()
        && !ui.model.uncertain()
        && ui.choices.deck_unchanged(ui.model.run())
        && !ui.loaded
        && ui.model.failure().is_none()
        && (clock == 0.0 || now - clock >= 5.0)
}

fn refresh_visible(ui: &mut Ui, now: f64, clock: &mut f64) {
    *clock = now;
    if !ui.loaded || ui.page == Page::Ranking {
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

pub(in crate::lobby) fn returned(mut state: ResMut<LobbyState>, duel: Option<Res<crate::Duel>>) {
    if matches!(state.lobby.screen(), Screen::Replay(_)) {
        return;
    }
    let Some((run, _)) = state.echoes.playing.take() else {
        return;
    };
    // A duel with a verdict is sealed and verified next: its run waits for
    // that result and has no table left to resume (F-TOWER-03).
    if crate::lobby::systems::ended_as(duel.as_deref()).is_some() {
        state.echoes.settling = Some(run);
    }
    state.echoes.opened = true;
    state.echoes.returning = Some(run);
    state.echoes.collection.refresh();
    state.echoes.page = Page::Shop;
    state.echoes.preview = None;
    state.echoes.starter_preview = None;
    state.hub = Hub::Challenges;
    if ready(&state) && !state.echoes.model.busy() {
        state.echoes.returning = None;
        ask(&mut state, Action::Read(Read::Run(run)));
        if !state.echoes.loaded {
            state.echoes.reads.push_back(Read::Cohorts(None));
        }
    }
}
