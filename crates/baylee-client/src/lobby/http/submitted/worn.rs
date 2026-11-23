//! Confirm a frozen account deck's cosmetics only for its actual ordinary seat.
use crate::{
    DuelPhase,
    lobby::{Expect, LobbyState, Reply, advent_table::HomeSeat, central, http},
};
use baylee_account::advent::{self, Equipment, table};
use baylee_client_core::lobby::{LobbyEvent, LobbyRequest, Screen, SeatHandover};
use bevy::prelude::*;
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone)]
enum Destination {
    Create,
    Join(String, Option<u32>),
    Change(String, u32),
}

impl Destination {
    fn accepts(&self, seat: &SeatHandover) -> bool {
        match self {
            Self::Create => true,
            Self::Join(game, chair) => {
                seat.game_id == *game && chair.is_none_or(|n| n == seat.seat)
            }
            Self::Change(game, chair) => seat.game_id == *game && seat.seat == *chair,
        }
    }
}

#[derive(Clone)]
pub(in crate::lobby) struct Selection {
    account: central::Handle,
    // None comes exclusively from an authenticated reclaim, never from a deck edit.
    look: Option<Equipment>,
    dest: Destination,
}

fn held(state: &LobbyState) -> Option<&SeatHandover> {
    match state.lobby.screen() {
        Screen::Seated(h) => Some(h),
        _ => state.lobby.awaiting(),
    }
}

fn same_account(state: &LobbyState, account: &central::Handle) -> bool {
    account.is_active()
        && state
            .central
            .as_ref()
            .and_then(|c| c.handle.as_ref())
            .is_some_and(|current| current.same_session(account))
}

impl Selection {
    pub(super) fn new(
        account: central::Handle,
        look: Equipment,
        request: &LobbyRequest,
    ) -> Option<Self> {
        let dest = match request {
            LobbyRequest::CreateGame { .. } => Destination::Create,
            LobbyRequest::JoinGame { game_id, seat, .. } => {
                Destination::Join(game_id.clone(), *seat)
            }
            LobbyRequest::SetSeat { game_id, seat, .. } => {
                Destination::Change(game_id.clone(), *seat)
            }
            _ => return None,
        };
        Some(Self {
            account,
            look: Some(look),
            dest,
        })
    }

    pub(in crate::lobby::http) fn recover(
        state: &LobbyState,
        request: &LobbyRequest,
    ) -> Option<Self> {
        let LobbyRequest::TakeSeat { game_id } = request else {
            return None;
        };
        (state.has_central_account() && state.lobby.token().is_some()).then_some(())?;
        Some(Self {
            account: state.central_handle()?,
            look: None,
            dest: Destination::Join(game_id.clone(), None),
        })
    }

    pub(in crate::lobby) fn expects(&self) -> Expect {
        match self.dest {
            Destination::Change(..) => Expect::Moved,
            _ => Expect::Seat,
        }
    }

    pub(in crate::lobby) fn admit(self, state: &mut LobbyState, reply: &Reply) -> bool {
        if !same_account(state, &self.account) {
            return false;
        }
        let seat = match (&self.dest, reply) {
            (Destination::Change(..), Reply::Event(LobbyEvent::Moved)) => held(state),
            (Destination::Join(..) | Destination::Create, Reply::Event(LobbyEvent::Seated(h))) => {
                Some(h)
            }
            _ => None,
        };
        if let Some(h) = seat.filter(|h| self.dest.accepts(h))
            && let Some((game, chair)) = ordinary(h)
        {
            // A room may hand the same ticket back while its first start is pending.
            // Keep that locally frozen revision until it has actually been presented.
            if self.look.is_none()
                && state.casual_worn.as_ref().is_some_and(|old| {
                    old.game == game
                        && old.chair == chair
                        && old.epoch == state.gateway_epoch
                        && old.origin == state.gateway
                        && old.account.same_session(&self.account)
                })
            {
                return true;
            }
            state.casual_worn = Some(Arc::new(Binding {
                account: self.account,
                origin: state.gateway.clone(),
                epoch: state.gateway_epoch,
                game,
                game_text: h.game_id.clone(),
                chair,
                look: self.look,
            }));
        }
        true
    }
}

fn ordinary(h: &SeatHandover) -> Option<(Uuid, u8)> {
    let game = Uuid::try_parse(&h.game_id).ok()?;
    let chair = u8::try_from(h.seat).ok()?;
    (!h.local
        && chair < 8
        && game.get_version_num() == 7
        && game.get_variant() == uuid::Variant::RFC4122
        && http::is_bearer(&h.seat_token))
    .then_some((game, chair))
}

pub(in crate::lobby) struct Binding {
    account: central::Handle,
    origin: String,
    epoch: u64,
    game: Uuid,
    game_text: String,
    chair: u8,
    look: Option<Equipment>,
}

enum Step {
    Suspended,
    Ready(Instant),
    Grant {
        receive: mpsc::Receiver<Result<Option<table::Proof>, baylee_account::Error>>,
        // A queued Central operation observes its cancellation when the seat is dropped.
        _lifetime: Arc<()>,
    },
    Confirm {
        receive: mpsc::Receiver<bool>,
        wearing: bool,
    },
    Done,
}

#[derive(Default)]
pub(in crate::lobby) struct Renewal(Option<Registration>);

struct Registration {
    binding: Arc<Binding>,
    step: Step,
    confirmed: bool,
}

const RETRY: Duration = Duration::from_secs(30);
const RENEW: Duration = Duration::from_secs(300);

impl Binding {
    fn request(&self, confirmed: bool) -> table::Request {
        let origin = self.origin.trim_end_matches('/').to_owned();
        let game = *self.game.as_bytes();
        let seat = self.chair;
        match self.look.filter(|_| !confirmed) {
            Some(look) => table::Request::Begin {
                origin,
                game,
                seat,
                look,
            },
            None => table::Request::Resume { origin, game, seat },
        }
    }

    fn begin(&self, now: Instant, confirmed: bool) -> Step {
        let request = self.request(confirmed);
        let (send, receive) = mpsc::sync_channel(1);
        let lifetime = Arc::new(());
        let still_seated = Arc::downgrade(&lifetime);
        let session = self.account.clone();
        if self.account.submit(move |account| {
            if still_seated.upgrade().is_none() || !session.is_active() {
                return;
            }
            let proof = match account.advent(advent::Request::Table { request }) {
                Ok(advent::Response::Table {
                    response: table::Response::Grant { proof },
                }) => Ok(Some(proof)),
                Ok(advent::Response::Table {
                    response: table::Response::NoLook,
                }) => Ok(None),
                Err(error) => Err(error),
                _ => Err(baylee_account::Error::Rejected),
            };
            let _ = send.send(proof);
        }) {
            Step::Grant {
                receive,
                _lifetime: lifetime,
            }
        } else {
            Step::Ready(now + RETRY)
        }
    }

    fn confirm(&self, token: &str, proof: Option<table::Proof>) -> Step {
        let wearing = proof.is_some();
        let url = format!(
            "{}/games/{}/event-look",
            self.origin.trim_end_matches('/'),
            self.game
        );
        let request = http::bearer(
            http::json_post(&url, &serde_json::json!({"proof":proof})),
            Some(token),
        );
        let (send, receive) = mpsc::sync_channel(1);
        crate::transport::fetch_limited(request, 4096, move |response| {
            let _ = send.send(response.is_ok_and(|r| r.ok));
        });
        Step::Confirm { receive, wearing }
    }
}

/// Runs during the duel as well as in the lobby. A stale Central result is
/// dropped before a proof can be sent to a different gateway or seat.
pub(in crate::lobby) fn renew(
    mut state: ResMut<LobbyState>,
    phase: Res<State<DuelPhase>>,
    home: Option<Res<HomeSeat>>,
    mut renewal: Local<Renewal>,
) {
    let Some(binding) = state.casual_worn.as_ref() else {
        renewal.0 = None;
        return;
    };
    let Some(seat) = held(&state).filter(|h| {
        !h.local
            && h.game_id == binding.game_text
            && h.seat == u32::from(binding.chair)
            && state.gateway_epoch == binding.epoch
            && state.gateway == binding.origin
            && state.has_central_account()
            && same_account(&state, &binding.account)
    }) else {
        state.bypass_change_detection().casual_worn = None;
        renewal.0 = None;
        return;
    };
    if !matches!(phase.get(), DuelPhase::Opening | DuelPhase::Playing)
        || !home.is_some_and(|h| {
            h.matches(
                &binding.origin,
                binding.epoch,
                binding.game,
                &seat.seat_token,
            )
        })
    {
        if let Some(r) = &mut renewal.0 {
            // Forget any outstanding proof. A fresh chair must ask again. Keep the
            // knowledge that a confirmed look now belongs to Central across a reconnect.
            r.step = Step::Suspended;
        }
        return;
    }
    let now = Instant::now();
    if renewal
        .0
        .as_ref()
        .is_none_or(|r| !Arc::ptr_eq(&r.binding, binding))
    {
        renewal.0 = Some(Registration {
            binding: binding.clone(),
            step: Step::Ready(now),
            confirmed: false,
        });
    }
    let Some(r) = &mut renewal.0 else {
        return;
    };
    r.step = match &r.step {
        Step::Suspended => binding.begin(now, r.confirmed),
        Step::Ready(next) if now >= *next => binding.begin(now, r.confirmed),
        Step::Grant { receive, .. } => match receive.try_recv() {
            Ok(Ok(proof)) => binding.confirm(&seat.seat_token, proof),
            Err(mpsc::TryRecvError::Empty) => return,
            _ => Step::Ready(now + RETRY),
        },
        Step::Confirm { receive, wearing } => match receive.try_recv() {
            Ok(true) if *wearing => {
                r.confirmed = true;
                Step::Ready(now + RENEW)
            }
            Ok(true) => Step::Done,
            Err(mpsc::TryRecvError::Empty) => return,
            _ => Step::Ready(now + RETRY),
        },
        Step::Ready(_) | Step::Done => return,
    };
}

#[cfg(test)]
mod tests;
