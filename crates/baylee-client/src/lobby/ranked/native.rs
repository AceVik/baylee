//! A serial, account-bound queue worker and an independent game socket. The
//! home lobby, presence, chat and its gateway session remain live throughout.
use crate::{
    DuelCommand, DuelPhase, InstalledHost,
    lobby::{LobbyState, central},
    net::{NetworkHost, SeatTicket},
};
use baylee_account::{
    Account, Error, RankedHost, RankedPlay,
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

mod deck;
mod public;

enum Reply {
    Frozen(u64, Result<deck::Frozen, Error>),
    Queue(Request, Result<QueueState, Error>),
    Seat(Option<RankedHost>, Result<Option<RankedPlay>, Error>),
}

type Inbox = Arc<Mutex<Option<Reply>>>;

pub(crate) struct Worker {
    pub(crate) queue: QueueState,
    pub(crate) needs_deck: bool,
    pub(crate) busy: bool,
    pub(crate) error: Option<Error>,
    pub(crate) board: Option<Response>,
    pub(crate) board_busy: bool,
    pub(crate) board_error: Option<Error>,
    pub(crate) revision: u64,
    pub(crate) played: Option<Id>,
    session: Option<central::Handle>,
    inbox: Option<Inbox>,
    selecting: Option<(u64, Format, deck::Source)>,
    frozen: Option<Arc<deck::Frozen>>,
    host: Option<RankedHost>,
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
            revision: 0,
            played: None,
            session: None,
            inbox: None,
            selecting: None,
            frozen: None,
            host: None,
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
            self.inbox = None;
            self.selecting = None;
            self.queue = QueueState::Idle;
            self.intent = None;
            self.frozen = None;
            self.host = None;
            self.play_paused = false;
            self.recheck = false;
            self.needs_deck = false;
            self.busy = false;
            self.error = None;
            self.played = None;
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
        worker.next = Some(Instant::now());
        worker.busy = true;
        worker.changed();
    }
}

fn ready(state: &LobbyState) -> bool {
    state.central_ready()
        && !state.central_terms_pending()
        && state.terms_subject != crate::lobby::front::terms::Subject::Central
        && matches!(state.lobby.screen(), Screen::Table)
        && state.lobby.awaiting().is_none()
}

pub(crate) fn poll(
    mut state: ResMut<LobbyState>,
    mut commands: Commands,
    mut opening: MessageWriter<DuelCommand>,
    phase: Option<Res<State<DuelPhase>>>,
) {
    let state = state.bypass_change_detection();
    let handle = state.central_handle().filter(central::Handle::is_active);
    state.ranked_net.session(handle);
    public::poll(&mut state.ranked_net);
    if !state.has_central_account() || state.central_terms_pending() {
        return;
    }
    let can_play = ready(state) && phase.as_deref().is_none_or(|p| *p.get() == DuelPhase::Closed);
    let epoch = state.gateway_epoch;
    let worker = &mut state.ranked_net;
    let reply = worker
        .inbox
        .as_ref()
        .and_then(|inbox| inbox.try_lock().ok()?.take());
    if let Some(reply) = reply {
        worker.inbox = None;
        if let Some(play) = finish(worker, epoch, reply) {
            if !can_play {
                worker.played = None;
                worker.recheck = true;
                return;
            }
            // Only the seat capability is installed in the duel. The separate
            // admission and all its rights are dropped here.
            match NetworkHost::connect(ticket(play)) {
                Ok(host) => {
                    commands.insert_resource(InstalledHost(Box::new(host)));
                    opening.write(DuelCommand::Open);
                }
                Err(_) => {
                    worker.played = None;
                    worker.failed(Error::Unavailable);
                }
            }
            return;
        }
    }
    if worker.session.is_none()
        || worker.inbox.is_some()
        || worker.next.is_none_or(|at| at > Instant::now())
    {
        return;
    }
    dispatch(worker, can_play);
}

fn ticket(mut play: RankedPlay) -> SeatTicket {
    SeatTicket {
        gateway: play.assignment.origin,
        game_id: play.seat.game_id.to_string(),
        seat: PlayerId::new(play.assignment.seat),
        watch: false,
        seat_token: std::mem::take(&mut *play.seat.seat_token),
    }
}

fn dispatch(worker: &mut Worker, can_play: bool) {
    if let Some((epoch, format, source)) = worker.selecting {
        if worker.submit(move |account| Reply::Frozen(epoch, source.freeze(account, format))) {
            worker.selecting = None;
        }
        return;
    }
    if can_play && worker.intent.is_none()
        && !worker.needs_deck
        && !worker.play_paused
        && !worker.recheck
        && let QueueState::Assigned { key, assignment } = &worker.queue
        && worker.played != Some(assignment.match_id)
    {
        let key = *key;
        let frozen = worker.frozen.clone();
        let mut host = worker.host.take();
        worker.submit(move |account| {
            let snapshot = frozen.as_ref().map(|f| (f.digest, &f.deck));
            let play = account.play_ranked(key, snapshot, &mut host);
            Reply::Seat(host, play)
        });
        return;
    }
    let request = worker.intent.clone().unwrap_or(Request::Status);
    worker.submit(move |account| {
        let result = match account.ranked(request.clone()) {
            Ok(Response::Queue { state }) => Ok(state),
            Ok(_) => Err(Error::Rejected),
            Err(error) => Err(error),
        };
        Reply::Queue(request, result)
    });
}

fn finish(worker: &mut Worker, epoch: u64, reply: Reply) -> Option<RankedPlay> {
    let now = Instant::now();
    worker.next = Some(now + POLL);
    match reply {
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
        Reply::Seat(host, result) => {
            worker.host = host;
            if worker.intent.is_some() {
                worker.next = Some(now);
                return None;
            }
            match result {
                Ok(Some(play)) => {
                    worker.played = Some(play.assignment.match_id);
                    worker.error = None;
                    worker.changed();
                    return Some(play);
                }
                Ok(None) => {
                    worker.delay = POLL;
                }
                Err(error) => {
                    worker.recheck = true;
                    worker.play_paused =
                        !matches!(error, Error::Unavailable | Error::Busy | Error::Conflict);
                    worker.failed(error);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests;
