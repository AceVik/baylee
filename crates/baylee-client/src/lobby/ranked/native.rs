//! A serial, account-bound queue worker and an independent game socket. The
//! home lobby, presence, chat and its gateway session remain live throughout.
use crate::{
    DuelCommand, DuelPhase, InstalledHost,
    lobby::{LobbyState, central},
    net::{NetworkHost, SeatTicket, replica::Table},
};
use baylee_account::{
    Account, Error, RankedConsent, RankedHost, RankedPlay,
    ranked::{Format, Id, QueueState, Request, Response, ai::AiOpponents},
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
    Opponent(Id, Result<Opponent, Error>),
}

struct SeatReply {
    host: Option<RankedHost>,
    play: Result<Option<RankedPlay>, Error>,
}

type Inbox = Arc<Mutex<Option<Reply>>>;

#[derive(Resource)]
pub(crate) struct RankedSocket {
    pub(crate) generation: u64,
    pub(crate) look: Option<baylee_account::advent::Equipment>,
    pub(crate) match_id: Id,
    pub(crate) game_id: Id,
    pub(crate) protected: bool,
    /// This player's seat and, when the opponent is an AI, its badge, as
    /// the assignment said: the table wears it from its first frame, before
    /// Central's list of participants confirms it.
    pub(crate) seat: u8,
    pub(crate) opponent_ai: Option<baylee_account::ranked::ai::AiBadge>,
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
    frozen: Option<(Id, Arc<deck::Frozen>)>,
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
    /// Whom the next ticket this worker mints accepts, as the person chose.
    pub(crate) ai: AiOpponents,
    /// Whom the ticket this worker minted accepts, by its key: a retry sends
    /// the same, since Central refuses a key whose choice moved (Central
    /// `docs/ai-players.md` §3). A ticket found at launch has none, and
    /// nothing says its kind.
    ticket_ai: Option<(Id, AiOpponents)>,
    /// A search moving to another choice: Central has no way to change a
    /// ticket's, so it is cancelled and joined again under a new key while
    /// the desk keeps showing the search it was.
    switch: Option<Switch>,
    /// This clock minus Central's, in seconds, learnt from the join that
    /// minted the live ticket (its `queued_at` is Central's now then).
    skew: i64,
    /// When the person began this search, on Central's clock; kept across a
    /// switch, so the clock beside the search does not start again.
    since: Option<i64>,
    /// The opponent of the match the desk shows (found or settled), as
    /// Central's list of participants names it, and the match last asked.
    opponent: Option<Opponent>,
    asked_opponent: Option<Id>,
}

/// A match's opponent: its published handle and, for an AI player, its
/// badge, which stands beside the handle and never in its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Opponent {
    match_id: Id,
    pub(crate) name: Option<String>,
    pub(crate) ai: Option<baylee_account::ranked::ai::AiBadge>,
}

/// A search being moved to another choice ([`Worker::switch`]).
#[derive(Clone, Debug)]
struct Switch {
    /// The waiting search the desk shows meanwhile.
    shown: QueueState,
    /// The format and deck to search again with.
    format: Format,
    deck: String,
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
            ai: AiOpponents::None,
            ticket_ai: None,
            switch: None,
            skew: 0,
            since: None,
            opponent: None,
            asked_opponent: None,
        }
    }
}

/// The most this clock may stand off Central's before the difference is
/// taken for a slow reply rather than a clock, in seconds.
const MAX_SKEW: i64 = 6 * 3600;

/// Whether a search accepting `ai` takes the opponent a match found: a
/// person always, an AI as the choice allows (the local AIs are the free
/// kinds, house and net).
fn takes(ai: AiOpponents, opponent: Option<&baylee_account::ranked::ai::AiBadge>) -> bool {
    opponent.is_none_or(|badge| match ai {
        AiOpponents::None => false,
        AiOpponents::Local => badge.kind.free(),
        AiOpponents::Any => true,
    })
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

    /// The queue as the desk shows it: while a search moves to another
    /// choice, the search it was, so nothing on the desk blinks through Idle.
    pub(super) fn shown_queue(&self) -> &QueueState {
        self.switch.as_ref().map_or(&self.queue, |s| &s.shown)
    }

    /// Whether a search is moving to another choice.
    pub(super) fn switching(&self) -> bool {
        self.switch.is_some()
    }

    /// Central's clock now, in Unix seconds, as near as this one knows it.
    pub(super) fn central_now(&self) -> i64 {
        self.central_now_ms() / 1000
    }

    /// Central's clock now, in Unix milliseconds.
    pub(super) fn central_now_ms(&self) -> i64 {
        let local = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map_or(0, |since| {
                i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
            });
        local.saturating_sub(self.skew.saturating_mul(1000))
    }

    /// When Central may seat an AI in the live search; none while it moves
    /// to another choice, or when the search takes none.
    pub(super) fn fill_in_at(&self) -> Option<i64> {
        match (&self.switch, &self.queue) {
            (None, QueueState::Waiting { ai_fill_in_at, .. }) => *ai_fill_in_at,
            _ => None,
        }
    }

    /// Whom the live search accepts, when this worker sent it.
    pub(super) fn ticket_choice(&self) -> Option<AiOpponents> {
        match &self.queue {
            QueueState::Waiting { key, .. } | QueueState::Assigned { key, .. } => self
                .ticket_ai
                .filter(|(minted, _)| minted == key)
                .map(|(_, ai)| ai),
            QueueState::Idle | QueueState::Finished { .. } => None,
        }
    }

    /// When the person began searching, on Central's clock.
    pub(super) fn searching_since(&self) -> Option<i64> {
        self.since
    }

    /// The match the desk shows a found or settled game of, and this
    /// player's seat in it.
    fn shown_match(&self) -> Option<(Id, u8)> {
        match self.shown_queue() {
            QueueState::Assigned { assignment, .. } => Some((assignment.match_id, assignment.seat)),
            QueueState::Finished { match_id, seat, .. } => Some((*match_id, *seat)),
            QueueState::Idle | QueueState::Waiting { .. } => None,
        }
    }

    /// The shown match, while its opponent has not been asked for.
    fn unnamed_match(&self) -> Option<(Id, u8)> {
        self.shown_match()
            .filter(|(id, _)| self.asked_opponent != Some(*id))
    }

    /// The opponent of the match the desk shows, once Central named it.
    pub(super) fn opponent(&self) -> Option<&Opponent> {
        let (shown, _) = self.shown_match()?;
        self.opponent.as_ref().filter(|o| o.match_id == shown)
    }

    /// The rendering fixture names its match's opponent (`visual_fixture`).
    #[cfg(feature = "dev-control")]
    pub(super) fn name_for_fixture(
        &mut self,
        match_id: Id,
        name: &str,
        ai: Option<baylee_account::ranked::ai::AiBadge>,
    ) {
        self.opponent = Some(Opponent {
            match_id,
            name: Some(name.to_owned()),
            ai,
        });
        self.asked_opponent = Some(match_id);
        self.changed();
    }

    /// The rendering fixture's reply, as if Central had answered a join this
    /// worker minted to accept `ai` (`visual_fixture`).
    #[cfg(feature = "dev-control")]
    pub(super) fn show_for_fixture(&mut self, queue: QueueState, ai: AiOpponents) {
        self.ai = ai;
        if let QueueState::Waiting { key, queued_at, .. } = &queue {
            self.ticket_ai = Some((*key, ai));
            self.since.get_or_insert(*queued_at);
        } else {
            self.since = None;
        }
        self.queue = queue;
        self.changed();
    }

    /// Learns Central's clock from the answer to a join this worker minted:
    /// a new ticket's `queued_at` is Central's now as it answered.
    fn learn_clock(&mut self, request: &Request, queue: &QueueState) {
        let (Request::Join { key: sent, .. }, QueueState::Waiting { key, queued_at, .. }) =
            (request, queue)
        else {
            return;
        };
        if sent != key {
            return;
        }
        let local = i64::try_from(crate::lobby::parts::now_secs()).unwrap_or(i64::MAX);
        let skew = local.saturating_sub(*queued_at);
        self.skew = if skew.abs() <= MAX_SKEW { skew } else { 0 };
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

    fn keep_deck(&mut self, frozen: deck::Frozen) -> Result<(), Error> {
        let key = match &self.queue {
            QueueState::Assigned { assignment, .. } if assignment.deck != frozen.digest => {
                return Err(Error::Conflict);
            }
            QueueState::Waiting { key, .. } | QueueState::Assigned { key, .. } => *key,
            QueueState::Idle | QueueState::Finished { .. } => {
                let key = *Uuid::now_v7().as_bytes();
                self.intent = Some(Request::Join {
                    key,
                    format: frozen.deck.format,
                    deck: frozen.digest,
                });
                self.ticket_ai = Some((key, self.ai));
                key
            }
        };
        self.frozen = Some((key, Arc::new(frozen)));
        Ok(())
    }

    fn observe(&mut self, queue: QueueState) {
        // A switch ends where the new search stands; Idle is the way between
        // the two searches. A match that came first ends it too when the new
        // choice takes that opponent (a person always): the cancel still
        // waiting to go is dropped and the match is played. Against an AI
        // the person has just turned away the cancel goes on, voiding the
        // match, and the search joins again as they asked.
        if let Some(switch) = &self.switch {
            let done = match (&switch.shown, &queue) {
                (QueueState::Waiting { key: old, .. }, QueueState::Waiting { key, .. }) => {
                    old != key
                }
                (_, QueueState::Idle | QueueState::Finished { .. }) => false,
                // A move only ever starts from a waiting search.
                (_, QueueState::Waiting { .. }) => true,
                (_, QueueState::Assigned { assignment, .. }) => {
                    let taken = takes(self.ai, assignment.opponent_ai.as_ref());
                    if taken && matches!(self.intent, Some(Request::Cancel { .. })) {
                        self.intent = None;
                    }
                    taken
                }
            };
            if done {
                self.switch = None;
                self.changed();
            }
        }
        match &queue {
            QueueState::Waiting { queued_at, .. } => {
                self.since.get_or_insert(*queued_at);
            }
            _ if self.switch.is_some() => {}
            _ => self.since = None,
        }
        if self.consent.as_ref().is_some_and(|c| !c.matches(&queue)) {
            self.consent = None;
            self.accepting = false;
            self.host = None;
        }
        if self.queue != queue {
            self.play_paused = false;
        }
        if let QueueState::Waiting { key, .. } | QueueState::Assigned { key, .. } = &queue
            && self.frozen.as_ref().is_some_and(|(old, _)| old != key)
        {
            self.frozen = None;
        }
        let old_needs = self.needs_deck;
        self.needs_deck = match &queue {
            QueueState::Waiting { .. } => self.frozen.is_none(),
            QueueState::Assigned { assignment, .. } => {
                assignment.game.is_none()
                    && self
                        .frozen
                        .as_ref()
                        .is_none_or(|(_, f)| f.digest != assignment.deck)
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
            self.ticket_ai = None;
            self.switch = None;
            self.skew = 0;
            self.since = None;
            self.opponent = None;
            self.asked_opponent = None;
            self.delay = POLL;
            self.next = self.session.is_some().then(Instant::now);
            self.changed();
        }
    }
}

pub(crate) fn board(state: &mut LobbyState, request: Request) {
    public::request(&mut state.ranked_net, request);
}

/// [`board`], keeping the rows on screen until the new ones arrive.
pub(crate) fn board_keeping(state: &mut LobbyState, request: Request) {
    public::request_keeping(&mut state.ranked_net, request);
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

/// Searches with the deck `id` in `format`, accepting the AI opponents `ai`
/// if the search mints a new ticket (a deck restored to a waiting one keeps
/// what that ticket accepts).
pub(crate) fn join(state: &mut LobbyState, format: Format, id: &str, ai: AiOpponents) {
    let handle = state.central_handle();
    state.ranked_net.session(handle);
    if state.ranked_net.busy || state.ranked_net.intent.is_some() || !ready(state) {
        return;
    }
    state.ranked_net.ai = ai;
    if state.ranked_net.switch.is_none() {
        state.ranked_net.since = None;
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
    // Leaving the queue while a search moves to another choice leaves it
    // altogether: the join that would follow is dropped.
    if worker.switch.take().is_some() {
        if matches!(worker.intent, Some(Request::Join { .. })) {
            worker.intent = None;
        }
        worker.selecting = None;
        worker.busy = false;
        worker.since = None;
        worker.changed();
    }
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

/// Moves a waiting search to accept `ai` instead. Central keeps a ticket's
/// choice for its key, so the ticket is cancelled and the same deck joins
/// again under a new key; the desk shows the search throughout. `false`
/// when there is no waiting search this worker can move now.
pub(crate) fn switch(state: &mut LobbyState, ai: AiOpponents, deck: &str) -> bool {
    let handle = state.central_handle();
    let worker = &mut state.ranked_net;
    worker.session(handle);
    if worker.switch.is_some() {
        // Already on its way: the join that follows takes the latest choice.
        worker.ai = ai;
        return true;
    }
    let QueueState::Waiting { key, format, .. } = worker.queue.clone() else {
        return false;
    };
    if worker.busy || worker.intent.is_some() || worker.needs_deck {
        return false;
    }
    if worker.ticket_choice() == Some(ai) {
        return true;
    }
    worker.ai = ai;
    worker.switch = Some(Switch {
        shown: worker.queue.clone(),
        format,
        deck: deck.to_owned(),
    });
    worker.intent = Some(Request::Cancel { key });
    worker.next = Some(Instant::now());
    worker.changed();
    true
}

/// The join that finishes a switch, once its cancel has left the queue Idle.
fn rejoin(state: &mut LobbyState) {
    let worker = &state.ranked_net;
    let Some(switch) = worker.switch.as_ref() else {
        // A choice changed while the last move was already joining: the
        // search it found accepts the earlier one, so it moves once more.
        if !worker.busy
            && worker.intent.is_none()
            && !worker.needs_deck
            && matches!(worker.queue, QueueState::Waiting { .. })
            && worker.ticket_choice().is_some_and(|sent| sent != worker.ai)
            && let Some((deck, _)) = super::queue::selected(state)
        {
            let ai = worker.ai;
            switch(state, ai, &deck.to_string());
        }
        return;
    };
    if worker.busy
        || worker.inbox.is_some()
        || worker.intent.is_some()
        || worker.selecting.is_some()
        || !matches!(worker.queue, QueueState::Idle | QueueState::Finished { .. })
    {
        return;
    }
    let (format, deck, ai) = (switch.format, switch.deck.clone(), worker.ai);
    join(state, format, &deck, ai);
    // A join refused before it began (the lobby took a table meanwhile)
    // ends the switch, and the desk shows where the queue really stands.
    if !state.ranked_net.busy {
        state.ranked_net.switch = None;
        state.ranked_net.since = None;
        state.ranked_net.changed();
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
    super::inline::sync_format(lobby.bypass_change_detection());
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
    let desk_open = closed && state.hub == crate::lobby::Hub::Ranked;
    preview::poll(&mut state.ranked_net, desk_open);
    let live = socket.as_deref().filter(|_| {
        !closed
            && phase
                .as_deref()
                .is_some_and(|phase| *phase.get() != DuelPhase::Finished)
    });
    let can_play = ready(state) && closed;
    let epoch = state.gateway_epoch;
    let reply = state
        .ranked_net
        .inbox
        .as_ref()
        .and_then(|inbox| inbox.try_lock().ok()?.take());
    if let Some(reply) = reply {
        let notify =
            !can_play && live.is_none() && replication::new_assignment(&state.ranked_net, &reply);
        state.ranked_net.inbox = None;
        let play = finish(&mut state.ranked_net, epoch, reply);
        super::inline::sync_format(state);
        let worker = &mut state.ranked_net;
        if let Some(play) = play {
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
    if can_play {
        rejoin(state);
        super::inline::sync_format(state);
    }
    let worker = &mut state.ranked_net;
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
                replication::read(
                    account,
                    Request::Status,
                    AiOpponents::None,
                    Some(presence),
                    documents,
                ),
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
    // Who the found or settled game is against, asked once per match before
    // the seat is taken: the desk names the opponent by its handle, and an
    // AI by its handle with its badge beside it.
    if let Some((match_id, seat)) = worker.unnamed_match() {
        worker.asked_opponent = Some(match_id);
        worker.submit(move |account| {
            Reply::Opponent(match_id, replication::opponent(account, match_id, seat))
        });
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
            let snapshot = frozen.as_ref().map(|(_, f)| (f.digest, &f.deck));
            let play = account.play_ranked_replica(key, &authority, snapshot, &mut host);
            Reply::Seat(Box::new(SeatReply { host, play }))
        });
        return;
    }
    let request = worker.intent.clone().unwrap_or(Request::Status);
    let documents = worker.documents.clone();
    // A join sends what its key was minted with, retry after retry.
    let ai = match &request {
        Request::Join { key, .. } => worker
            .ticket_ai
            .filter(|(minted, _)| minted == key)
            .map_or(AiOpponents::None, |(_, ai)| ai),
        _ => AiOpponents::None,
    };
    worker.submit(move |account| {
        Reply::Authority(
            request.clone(),
            replication::read(account, request, ai, None, documents),
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
                    worker.switch = None;
                    worker.failed(Error::Conflict);
                    return None;
                }
                Err(error) => {
                    worker.switch = None;
                    worker.failed(error);
                    return None;
                }
            };
            if let Err(error) = worker.keep_deck(frozen) {
                worker.switch = None;
                worker.failed(error);
                return None;
            }
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
                worker.learn_clock(&request, &queue);
                worker.observe(queue);
                worker.delay = POLL;
                if worker.intent.is_some() || worker.selecting.is_some() {
                    worker.next = Some(now);
                } else if !active {
                    // Idle; while a search moves, `rejoin` joins again from
                    // the next frame's poll, with no read in between. A
                    // settled game not yet named asks who it was against.
                    worker.next = worker.unnamed_match().map(|_| now);
                } else if let Some(at) = worker.fill_in_at() {
                    // Fill-in happens inside this account's own poll, so
                    // near the time it asks on the second (Central
                    // `docs/ai-players.md` §3: every 2 to 3 seconds).
                    let left = at.saturating_sub(worker.central_now());
                    let wait = if left > 0 { left.min(3) } else { 2 };
                    worker.next = Some(now + Duration::from_secs(wait.unsigned_abs()));
                }
            }
            Err(error) => {
                if !matches!(error, Error::Unavailable | Error::Busy)
                    && worker.intent.as_ref() == Some(&request)
                {
                    worker.intent = None;
                    // A refusal ends a move between choices: the desk shows
                    // where the queue really stands, with the refusal.
                    worker.switch = None;
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
        Reply::Opponent(match_id, result) => heard_opponent(worker, now, match_id, result),
    }
    None
}

/// Central named the opponent of `match_id`, or could not.
fn heard_opponent(
    worker: &mut Worker,
    now: Instant,
    match_id: Id,
    result: Result<Opponent, Error>,
) {
    worker.next = Some(now);
    match result {
        Ok(opponent) => {
            if worker.opponent.as_ref() != Some(&opponent) {
                worker.opponent = Some(opponent);
                worker.changed();
            }
        }
        // Asked again on a later poll; a refusal is not asked again, and
        // the desk shows the badge without a name.
        Err(Error::Busy | Error::Unavailable) if worker.asked_opponent == Some(match_id) => {
            worker.asked_opponent = None;
            worker.next = Some(now + POLL);
        }
        Err(_) => {}
    }
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
