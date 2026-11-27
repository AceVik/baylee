//! Resolve private opponent names only on the participant's Central worker.
//! A detached worker can finish into its own mailbox but never a new match.
use super::{LobbyState, native::RankedSocket};
use crate::{
    Duel, DuelPhase, InstalledHost,
    lobby::{central::Handle, ranked_profile::valid_handle},
};
use baylee_account::{
    Error,
    ranked::{
        Id,
        replica::{SeatRequest, SeatResponse},
    },
};
use bevy::prelude::*;
use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};

type Seats = [Option<String>; 2];

#[derive(Default)]
pub(super) struct Reader {
    context: Option<(Handle, Id, Id)>,
    pending: Option<Receiver<Result<Seats, Error>>>,
    next: Option<Instant>,
}

impl Reader {
    fn owns_finished(&self, lobby: &LobbyState, duel: &Duel) -> bool {
        self.context.as_ref().is_some_and(|(previous, _, game)| {
            !duel.watching
                && duel.replay.is_none()
                && duel.statics.as_ref().is_some_and(|raw| {
                    uuid::Uuid::parse_str(&raw.game_id).is_ok_and(|id| id.as_bytes() == game)
                })
                && lobby
                    .central_handle()
                    .is_some_and(|current| current.is_active() && current.same_session(previous))
        })
    }

    fn bind(&mut self, handle: &Handle, socket: &RankedSocket) -> bool {
        if self.context.as_ref().is_some_and(|(owner, matched, game)| {
            owner.same_session(handle) && *matched == socket.match_id && *game == socket.game_id
        }) {
            return false;
        }
        self.context = Some((handle.clone(), socket.match_id, socket.game_id));
        self.pending = None;
        self.next = Some(Instant::now());
        true
    }

    fn start(&mut self, handle: &Handle, id: Id) {
        let (send, recv) = mpsc::sync_channel(1);
        if handle.submit(move |account| {
            let result = account
                .replica(SeatRequest::Participants { match_id: id })
                .and_then(|answer| match answer {
                    // TODO(ai-players client): the AI badges, in seat order.
                    SeatResponse::Participants {
                        match_id, seats, ..
                    } if match_id == id
                        && seats.iter().flatten().all(|name| valid_handle(name)) =>
                    {
                        Ok(seats)
                    }
                    _ => Err(Error::Rejected),
                });
            let _ = send.send(result);
        }) {
            self.pending = Some(recv);
            self.next = None;
        } else {
            self.next = Some(Instant::now() + Duration::from_secs(5));
        }
    }
}

pub(super) fn poll(
    mut reader: Local<Reader>,
    lobby: Res<LobbyState>,
    socket: Option<Res<RankedSocket>>,
    host: Option<Res<InstalledHost>>,
    phase: Option<Res<State<DuelPhase>>>,
    mut duel: Option<ResMut<Duel>>,
) {
    let Some(duel) = duel.as_mut() else {
        return;
    };
    let target = socket
        .as_deref()
        .filter(|s| {
            !duel.watching
                && duel.replay.is_none()
                && host
                    .as_ref()
                    .is_some_and(|h| !h.0.watching() && h.0.replay().is_none())
                && duel.statics.as_ref().is_some_and(|raw| {
                    uuid::Uuid::parse_str(&raw.game_id).is_ok_and(|id| id.as_bytes() == &s.game_id)
                })
        })
        .and_then(|s| {
            lobby
                .central_handle()
                .filter(Handle::is_active)
                .map(|h| (s, h))
        });
    let Some((socket, owner)) = target else {
        // The result screen releases the transport as soon as the duel ends.
        // Keep an already resolved roster for that player's own result screen.
        if socket.is_none()
            && phase.is_some_and(|phase| *phase.get() == DuelPhase::Finished)
            && reader.owns_finished(&lobby, duel)
        {
            return;
        }
        if reader.context.is_some() {
            *reader = Reader::default();
            duel.participant_labels(None);
        }
        return;
    };
    if reader.bind(&owner, socket) {
        duel.participant_labels(None);
    }
    if let Some(pending) = &reader.pending {
        let result = match pending.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(Error::Unavailable),
        };
        reader.pending = None;
        match result {
            Ok(seats) if seats.iter().any(Option::is_some) => {
                let game = uuid::Uuid::from_bytes(socket.game_id).to_string();
                duel.participant_labels(Some((game, seats)));
            }
            Err(Error::Busy | Error::Unavailable) => {
                reader.next = Some(Instant::now() + Duration::from_secs(10));
            }
            _ => {}
        }
    }
    if reader.next.is_some_and(|due| due <= Instant::now()) {
        reader.start(&owner, socket.match_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_late_private_name_reply_cannot_cross_account_or_ranked_game() {
        let (first, _jobs_a) = Handle::queued_for_test();
        let (second, _jobs_b) = Handle::queued_for_test();
        let mut socket = RankedSocket {
            generation: 1,
            look: None,
            match_id: *uuid::Uuid::now_v7().as_bytes(),
            game_id: *uuid::Uuid::now_v7().as_bytes(),
            protected: true,
        };
        let mut reader = Reader::default();
        assert!(reader.bind(&first, &socket));
        let (send, recv) = mpsc::sync_channel(1);
        reader.pending = Some(recv);
        assert!(!reader.bind(&first, &socket));
        assert!(reader.bind(&second, &socket));
        assert!(send.send(Ok([None, None])).is_err());
        let (send, recv) = mpsc::sync_channel(1);
        reader.pending = Some(recv);
        socket.match_id = *uuid::Uuid::now_v7().as_bytes();
        assert!(reader.bind(&second, &socket));
        assert!(send.send(Ok([None, None])).is_err());
    }
}
