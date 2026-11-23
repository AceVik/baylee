//! A serial, account-bound queue worker and an independent game socket. The
//! home lobby, presence, chat and its gateway session remain live throughout.
use crate::{
    DuelCommand, DuelPhase, InstalledHost,
    lobby::{LobbyState, central},
    net::{NetworkHost, SeatTicket, replica::Table},
};
use baylee_account::{
    Account, Error, RankedConsent, RankedHost, RankedPlay,
    ranked::{Format, Id, QueueState, Request, Response},
};
use baylee_client_core::lobby::Screen;
use baylee_core::ids::PlayerId;
use bevy::prelude::*;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

pub(in crate::lobby) mod consent;
mod deck;
mod host_documents;
mod preview;
pub(crate) use preview::{clear_preview, preview};
mod public;
mod replication;

enum Reply {
    Frozen(u64, Result<deck::Frozen, Error>),
    Queue(Request, Result<QueueState, Error>),
    Seat(Box<SeatReply>),
    Accepted(Result<(), Error>),
    Renewed(
        baylee_account::ranked::replica::Fence,
        Result<Instant, Error>,
    ),
    Authority(Request, Result<replication::Route, Error>),
    Submitted(Id, Result<replication::Submission, Error>),
}

struct SeatReply {
    host: Option<RankedHost>,
    play: Result<Option<RankedPlay>, Error>,
}

type Inbox = Arc<Mutex<Option<Reply>>>;

#[derive(Resource)]
pub(crate) struct RankedSocket {
    pub(crate) match_id: Id,
    pub(crate) game_id: Id,
    pub(crate) protected: bool,
}

pub(crate) fn close_game_transport(mut commands: Commands, live: Option<Res<RankedSocket>>) {
    if live.is_some() {
        commands.remove_resource::<InstalledHost>();
        commands.remove_resource::<RankedSocket>();
    }
}

pub(crate) fn clear_game_marker(mut commands: Commands, live: Option<Res<RankedSocket>>) {
    if live.is_some() {
        commands.remove_resource::<InstalledHost>();
        commands.remove_resource::<RankedSocket>();
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "queue admission, game transport, deck preview and public reads advance independently"
)]
pub(crate) struct Worker {
    pub(crate) queue: QueueState,
    pub(crate) needs_deck: bool,
    pub(crate) busy: bool,
    pub(crate) error: Option<Error>,
    pub(crate) board: Option<Response>,
    pub(crate) board_busy: bool,
    pub(crate) board_error: Option<Error>,
    pub(crate) preview: Option<(Format, Uuid, baylee_account::ranked::DeckPreview)>,
    pub(crate) preview_busy: bool,
    pub(crate) preview_error: Option<Error>,
    preview_jobs: preview::Jobs,
    pub(crate) revision: u64,
    pub(crate) played: Option<Id>,
    table: Table,
    session: Option<central::Handle>,
    inbox: Option<Inbox>,
    selecting: Option<(u64, Format, deck::Source)>,
    frozen: Option<Arc<deck::Frozen>>,
    host: Option<RankedHost>,
    consent: Option<RankedConsent>,
    documents: Option<Arc<baylee_account::RankedHostDocuments>>,
    displayed_documents: Option<Arc<baylee_account::RankedHostDocuments>>,
    host_choice: Option<bool>,
    accepting: bool,
    renew_at: Option<Instant>,
    intent: Option<Request>,
    next: Option<Instant>,
    delay: Duration,
    play_paused: bool,
    recheck: bool,
    read_wanted: Option<Request>,
    read_shown: Option<Request>,
    reading: Option<public::Inbox>,
    discard_read: bool,
}

impl Default for Worker {
    fn default() -> Self {
        Self {
            queue: QueueState::Idle,
            needs_deck: false,
            busy: false,
            error: None,
            board: None,
            board_busy: false,
            board_error: None,
            preview: None,
            preview_busy: false,
            preview_error: None,
            preview_jobs: preview::Jobs::default(),
            revision: 0,
            played: None,
            table: Table::default(),
            session: None,
            inbox: None,
            selecting: None,
            frozen: None,
            host: None,
            consent: None,
            documents: None,
            displayed_documents: None,
            host_choice: None,
            accepting: false,
            renew_at: None,
            intent: None,
            next: None,
            delay: POLL,
            play_paused: false,
            recheck: false,
            read_wanted: None,
            read_shown: None,
            reading: None,
            discard_read: false,
        }
    }
}

const POLL: Duration = Duration::from_secs(3);
const MAX_DELAY: Duration = Duration::from_secs(30);

impl Worker {
    fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub(super) fn waiting_consent(&self) -> bool {
        self.table.waiting_consent()
    }

    fn failed(&mut self, error: Error) {
        if self.error != Some(error) || self.busy {
            self.error = Some(error);
            self.busy = false;
            self.changed();
        }
        self.next = Some(Instant::now() + self.delay);
        self.delay = (self.delay * 2).min(MAX_DELAY);
    }

    fn submit(&mut self, operation: impl FnOnce(&mut Account) -> Reply + Send + 'static) -> bool {
        let Some(handle) = self.session.as_ref() else {
            return false;
        };
        let inbox: Inbox = Arc::default();
        let writer = Arc::clone(&inbox);
        if handle.submit(move |account| {
            let reply = operation(account);
            if let Ok(mut slot) = writer.lock() {
                *slot = Some(reply);
            }
        }) {
            self.inbox = Some(inbox);
            true
        } else {
            self.failed(Error::Busy);
            false
        }
    }

    fn observe(&mut self, queue: QueueState) {
        if self.consent.as_ref().is_some_and(|c| !c.matches(&queue)) {
            self.consent = None;
            self.accepting = false;
            self.host = None;
        }
        if self.queue != queue {
            self.play_paused = false;
        }
        let old_needs = self.needs_deck;
        self.needs_deck = match &queue {
            QueueState::Waiting { .. } => self.frozen.is_none(),
            QueueState::Assigned { assignment, .. } => {
                assignment.game.is_none()
                    && self
                        .frozen
                        .as_ref()
                        .is_none_or(|f| f.digest != assignment.deck)
            }
            QueueState::Idle | QueueState::Finished { .. } => {
                self.frozen = None;
                self.host = None;
                false
            }
        };
        if self.busy
            || (self.error.is_some() && !self.play_paused)
            || self.queue != queue
            || self.needs_deck != old_needs
        {
            self.busy = false;
            if !self.play_paused {
                self.error = None;
            }
            self.queue = queue;
            self.changed();
        }
    }

    fn session(&mut self, handle: Option<central::Handle>) {
        let same = match (&self.session, &handle) {
            (Some(a), Some(b)) => a.same_session(b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            // Drop the inbox as well as credentials; an old worker still
            // finishing a request can only publish to its own detached slot.
            self.session = handle;
            self.table.unavailable();
            self.table = Table::default();
            self.inbox = None;
            self.selecting = None;
            self.queue = QueueState::Idle;
            self.intent = None;
            self.frozen = None;
            self.host = None;
            self.consent = None;
            self.accepting = false;
            self.play_paused = false;
            self.documents = None;
            self.displayed_documents = None;
            self.host_choice = None;
            self.recheck = false;
            self.needs_deck = false;
            self.busy = false;
            self.error = None;
            self.played = None;
            self.renew_at = None;
            self.preview = None;
            self.preview_busy = false;
            self.preview_error = None;
            self.preview_jobs = preview::Jobs::default();
            self.delay = POLL;
            self.next = self.session.is_some().then(Instant::now);
            self.changed();
        }
    }
}

pub(crate) fn board(state: &mut LobbyState, request: Request) {
    public::request(&mut state.ranked_net, request);
}

pub(crate) fn refresh(state: &mut LobbyState) {
    let handle = state.central_handle();
    let worker = &mut state.ranked_net;
    worker.session(handle);
    worker.play_paused = false;
    worker.next = Some(Instant::now());
}

/// The player explicitly returned to their already-running ranked table.
pub(crate) fn resume(state: &mut LobbyState) {
    if let QueueState::Assigned { assignment, .. } = &state.ranked_net.queue
        && state.ranked_net.played == Some(assignment.match_id)
    {
        state.ranked_net.played = None;
        refresh(state);
    }
}

pub(crate) fn join(state: &mut LobbyState, format: Format, id: &str) {
    let handle = state.central_handle();
    state.ranked_net.session(handle);
    if state.ranked_net.busy || state.ranked_net.intent.is_some() || !ready(state) {
        return;
    }
    let source = match deck::Source::selected(state, id, format) {
        Ok(source) => source,
        Err(error) => {
            state.ranked_net.failed(error);
            return;
        }
    };
    let epoch = state.gateway_epoch;
    let worker = &mut state.ranked_net;
    worker.busy = true;
    worker.error = None;
    worker.changed();
    worker.selecting = Some((epoch, format, source));
    worker.next = Some(Instant::now());
}

pub(crate) fn cancel(state: &mut LobbyState) {
    let handle = state.central_handle();
    let worker = &mut state.ranked_net;
    worker.session(handle);
    if let QueueState::Waiting { key, .. } | QueueState::Assigned { key, .. } = worker.queue {
        worker.intent = Some(Request::Cancel { key });
        worker.consent = None;
        worker.accepting = false;
        worker.host = None;
        worker.next = Some(Instant::now());
        worker.busy = true;
        worker.changed();
    }
}

fn ready(state: &LobbyState) -> bool {
    state.has_central_account()
        && !state.central_terms_pending()
        && !state.terms.up()
        && state.terms_subject != crate::lobby::front::terms::Subject::Central
        && matches!(state.lobby.screen(), Screen::Table)
        && state.lobby.awaiting().is_none()
}

pub(crate) fn poll(
    mut lobby: ResMut<LobbyState>,
    mut commands: Commands,
    mut opening: MessageWriter<DuelCommand>,
    phase: Option<Res<State<DuelPhase>>>,
    mut scrolled: Option<ResMut<crate::lobby::Scrolled>>,
    socket: Option<Res<RankedSocket>>,
) {
    let handle = lobby
        .central_handle()
        .filter(|h| h.is_active() && lobby.has_central_account());
    lobby.bypass_change_detection().ranked_net.session(handle);
    let closed = phase
        .as_deref()
        .is_none_or(|p| *p.get() == DuelPhase::Closed);
    consent::sync(&mut lobby, scrolled.as_mut(), closed);
    let state = lobby.bypass_change_detection();
    public::poll(&mut state.ranked_net);
    if !state.has_central_account() || state.central_terms_pending() {
        if socket.is_some() && !closed {
            opening.write(DuelCommand::Close);
        }
        return;
    }
    let desk_open = closed && super::inline::preview_visible(state);
    preview::poll(&mut state.ranked_net, desk_open);
    let live = socket.as_deref().filter(|_| {
        !closed
            && phase
                .as_deref()
                .is_some_and(|phase| *phase.get() != DuelPhase::Finished)
    });
    let can_play = ready(state) && closed;
    let epoch = state.gateway_epoch;
    let worker = &mut state.ranked_net;
    let reply = worker
        .inbox
        .as_ref()
        .and_then(|inbox| inbox.try_lock().ok()?.take());
    if let Some(reply) = reply {
        let notify = !can_play && live.is_none() && replication::new_assignment(worker, &reply);
        worker.inbox = None;
        if let Some(play) = finish(worker, epoch, reply) {
            let taking_over = live.is_some_and(|live| live.match_id == play.assignment.match_id);
            if !can_play && !taking_over {
                worker.played = None;
                worker.recheck = true;
                return;
            }
            // Only the seat capability is installed in the duel. The separate
            // admission and all its rights are dropped here.
            if let Ok((host, marker)) =
                replication::connect(worker, play, live.map(|live| live.protected))
            {
                commands.insert_resource(InstalledHost(Box::new(host)));
                commands.insert_resource(marker);
                if !taking_over {
                    opening.write(DuelCommand::Open);
                }
            } else {
                worker.played = None;
                worker.failed(Error::Unavailable);
            }
            return;
        }
        if live.is_some() && (worker.consent.is_some() || worker.documents.is_some()) {
            worker.played = None;
            opening.write(DuelCommand::Close);
            let lang = lobby.lobby.lang();
            lobby
                .lobby
                .note_said(baylee_client_core::Phrase::RankedNewHostTerms.text(lang));
            return;
        }
        if live.is_some() && matches!(worker.queue, QueueState::Idle | QueueState::Finished { .. })
        {
            if matches!(worker.queue, QueueState::Idle) {
                let lang = lobby.lobby.lang();
                lobby
                    .lobby
                    .note_said(baylee_client_core::Phrase::RankedSeatReleased.text(lang));
            }
            opening.write(DuelCommand::Close);
            return;
        }
        if notify {
            let lang = lobby.lobby.lang();
            lobby
                .lobby
                .note_said(baylee_client_core::Phrase::RankedOpponentWaiting.text(lang));
            return;
        }
    }
    if worker.session.is_none()
        || worker.inbox.is_some()
        || (worker.next.is_none_or(|at| at > Instant::now())
            && !worker.table.has_submission(Instant::now())
            && !worker.table.presence_due(Instant::now()))
    {
        return;
    }
    dispatch(worker, can_play, socket.is_some());
}

fn ticket(mut play: RankedPlay) -> SeatTicket {
    SeatTicket {
        gateway: play.relay_origin,
        game_id: play.seat.game_id.to_string(),
        seat: PlayerId::new(play.assignment.seat),
        watch: false,
        seat_token: std::mem::take(&mut *play.seat.seat_token),
    }
}

fn dispatch(worker: &mut Worker, can_play: bool, playing: bool) {
    if let Some(presence) = worker.table.take_presence(Instant::now()) {
        let documents = worker.documents.clone();
        worker.submit(move |account| {
            Reply::Authority(
                Request::Status,
                replication::read(account, Request::Status, Some(presence), documents),
            )
        });
        return;
    }
    if replication::send_pending(worker) {
        return;
    }
    if host_documents::answer(worker) {
        return;
    }
    if playing
        && worker.renew_at.is_some_and(|t| t <= Instant::now())
        && let QueueState::Assigned { key, assignment } = &worker.queue
        && worker.played == Some(assignment.match_id)
        && assignment.game.is_some()
        && let Some(authority) = worker.table.expected(assignment)
    {
        let key = *key;
        let assignment = assignment.clone();
        worker.submit(move |account| {
            Reply::Renewed(
                authority.fence,
                account.renew_ranked_replica(key, &assignment, authority.fence),
            )
        });
        return;
    }
    if worker.accepting
        && let Some(consent) = worker.consent.clone()
    {
        if worker.submit(move |account| Reply::Accepted(consent.accept(account))) {
            worker.accepting = false;
        }
        return;
    }
    if let Some((epoch, format, source)) = worker.selecting {
        if worker.submit(move |account| Reply::Frozen(epoch, source.freeze(account, format))) {
            worker.selecting = None;
        }
        return;
    }
    let reticket = playing && worker.table.needs_ticket();
    if (can_play || reticket)
        && worker.intent.is_none()
        && !worker.needs_deck
        && !worker.play_paused
        && worker.consent.is_none()
        && worker.documents.is_none()
        && !worker.recheck
        && let QueueState::Assigned { key, assignment } = &worker.queue
        && (worker.played != Some(assignment.match_id) || reticket)
        && let Some(authority) = worker.table.expected(assignment)
    {
        let key = *key;
        let frozen = worker.frozen.clone();
        let mut host = worker.host.take();
        worker.submit(move |account| {
            let snapshot = frozen.as_ref().map(|f| (f.digest, &f.deck));
            let play = account.play_ranked_replica(key, &authority, snapshot, &mut host);
            Reply::Seat(Box::new(SeatReply { host, play }))
        });
        return;
    }
    let request = worker.intent.clone().unwrap_or(Request::Status);
    let documents = worker.documents.clone();
    worker.submit(move |account| {
        Reply::Authority(
            request.clone(),
            replication::read(account, request, None, documents),
        )
    });
}

fn finish(worker: &mut Worker, epoch: u64, reply: Reply) -> Option<RankedPlay> {
    let now = Instant::now();
    worker.next = Some(now + POLL);
    match reply {
        Reply::Submitted(id, result) => {
            replication::answered(&worker.table, id, result, now);
            worker.next = Some(now);
        }
        Reply::Authority(request, result) => {
            let queue = result.map(|route| replication::observe(worker, route));
            if queue.is_err() {
                worker.table.unavailable();
            }
            return finish(worker, epoch, Reply::Queue(request, queue));
        }
        Reply::Frozen(source_epoch, result) => {
            let frozen = match result {
                Ok(frozen) if epoch == source_epoch => frozen,
                Ok(_) => {
                    worker.failed(Error::Conflict);
                    return None;
                }
                Err(error) => {
                    worker.failed(error);
                    return None;
                }
            };
            if let QueueState::Assigned { assignment, .. } = &worker.queue
                && assignment.deck != frozen.digest
            {
                worker.failed(Error::Conflict);
                return None;
            }
            if matches!(worker.queue, QueueState::Idle | QueueState::Finished { .. }) {
                worker.intent = Some(Request::Join {
                    key: *Uuid::now_v7().as_bytes(),
                    format: frozen.deck.format,
                    deck: frozen.digest,
                });
            }
            worker.frozen = Some(Arc::new(frozen));
            worker.busy = worker.intent.is_some();
            worker.needs_deck = false;
            worker.play_paused = false;
            worker.next = Some(now);
            worker.changed();
        }
        Reply::Queue(request, result) => match result {
            Ok(queue) => {
                worker.recheck = false;
                if worker.intent.as_ref() == Some(&request) {
                    worker.intent = None;
                }
                let active = matches!(
                    queue,
                    QueueState::Waiting { .. } | QueueState::Assigned { .. }
                );
                worker.observe(queue);
                worker.delay = POLL;
                if worker.intent.is_some() || worker.selecting.is_some() {
                    worker.next = Some(now);
                } else if !active {
                    worker.next = None;
                }
            }
            Err(error) => {
                if !matches!(error, Error::Unavailable | Error::Busy)
                    && worker.intent.as_ref() == Some(&request)
                {
                    worker.intent = None;
                }
                worker.failed(error);
            }
        },
        Reply::Renewed(fence, result) => {
            if matches!(&worker.queue, QueueState::Assigned{assignment,..} if worker.table.expected(assignment).is_some_and(|s| s.fence == fence))
            {
                match result {
                    Ok(renew_at) => worker.renew_at = Some(renew_at),
                    Err(error) => {
                        worker.failed(error);
                        worker.renew_at = Some(now + worker.delay);
                    }
                }
            }
        }
        Reply::Accepted(result) => {
            worker.consent = None;
            worker.documents = None;
            worker.host = None;
            worker.recheck = true;
            match result {
                Ok(()) => worker.next = Some(now),
                Err(error) => worker.failed(error),
            }
        }
        Reply::Seat(seat) => return finish_seat(worker, now, *seat),
    }
    None
}

fn finish_seat(worker: &mut Worker, now: Instant, seat: SeatReply) -> Option<RankedPlay> {
    worker.host = seat.host;
    if worker.intent.is_some() {
        worker.next = Some(now);
        return None;
    }
    match seat.play {
        Ok(Some(play)) => {
            worker.renew_at = Some(play.renew_at);
            worker.played = Some(play.assignment.match_id);
            worker.error = None;
            worker.changed();
            Some(play)
        }
        Ok(None) => {
            worker.delay = POLL;
            worker.consent = worker.host.as_ref().and_then(RankedHost::consent).cloned();
            None
        }
        Err(error) => {
            worker.recheck = true;
            worker.play_paused =
                !matches!(error, Error::Unavailable | Error::Busy | Error::Conflict);
            worker.failed(error);
            None
        }
    }
}

#[cfg(test)]
mod tests;
