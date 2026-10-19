//! Serialize socket ownership and engine messages with the seat's lobby lock.
//!
//! Admission has awaits, so reserving a generation and announcing it are separate:
//! an older attempt must not finish late and replace its successor. The previous
//! owner remains responsible for detaching until a successor actually attaches.

use super::{LobbyGame, Shared, lobby, to_engine, v1};

/// One admission attempt, also used to identify the resulting socket.
pub(super) struct Ownership {
    /// Unique across chair resets as well as reconnects (no counter ABA).
    generation: uuid::Uuid,
    /// Capability snapshot, checked again after every asynchronous authorization.
    hash: String,
}

impl Ownership {
    /// Reserve before waiting for the engine or Central. Caller holds the lobby lock.
    pub(super) fn reserve(game: &mut LobbyGame, seat: usize, hash: &str) -> Option<Self> {
        let chair = game.seats.get_mut(seat)?;
        if chair.participation_ended || chair.seat_token_hash.as_deref() != Some(hash) {
            return None;
        }
        let generation = uuid::Uuid::now_v7();
        chair.socket_pending = Some(generation);
        Some(Self {
            generation,
            hash: hash.into(),
        })
    }

    /// Commit ownership and its engine announcement in the same critical section.
    pub(super) fn attach(&self, game: &mut LobbyGame, seat: usize) -> Option<lobby::EngineLink> {
        let chair = game.seats.get_mut(seat)?;
        if chair.participation_ended
            || chair.socket_pending != Some(self.generation)
            || chair.seat_token_hash.as_deref() != Some(self.hash.as_str())
        {
            return None;
        }
        let link = game.engine.as_ref()?;
        let attached = v1::envelope::Msg::SeatAttached(v1::SeatAttached {
            seat: seat as u32,
            resync: false,
        });
        if !to_engine(link, attached) {
            return None;
        }
        chair.socket_owner = Some(self.generation);
        Some(link.clone())
    }

    /// Current socket and current capability; a replacement invalidates both directions.
    pub(super) fn active(&self, game: &LobbyGame, seat: usize) -> bool {
        game.seats.get(seat).is_some_and(|chair| {
            chair.socket_owner == Some(self.generation)
                && chair.seat_token_hash.as_deref() == Some(self.hash.as_str())
        })
    }

    /// Check and enqueue atomically with respect to replacement, including resyncs.
    pub(super) fn send(
        &self,
        game: &LobbyGame,
        seat: usize,
        link: &lobby::EngineLink,
        msg: v1::envelope::Msg,
    ) -> bool {
        self.active(game, seat)
            && game
                .seats
                .get(seat)
                .is_some_and(|chair| !chair.participation_ended)
            && to_engine(link, msg)
    }

    /// A rotated capability can detach until its successor attaches, but never after.
    pub(super) fn detach(&self, game: &mut LobbyGame, seat: usize, link: &lobby::EngineLink) {
        let Some(chair) = game.seats.get_mut(seat) else {
            return;
        };
        if chair.socket_owner != Some(self.generation) {
            return;
        }
        chair.socket_owner = None;
        to_engine(
            link,
            v1::envelope::Msg::SeatDetached(v1::SeatDetached {
                seat: seat as u32,
                left: false,
            }),
        );
    }
}

/// An attached socket's ownership, released even when its task is cancelled.
pub(super) struct Connection {
    /// Gateway state containing the shared serialization lock.
    pub(super) state: Shared,
    /// Game whose seat this connection owns.
    pub(super) game: String,
    /// Seat within that game.
    pub(super) seat: usize,
    /// Admission generation and frozen capability.
    pub(super) ownership: Ownership,
    /// Engine link selected when the socket attached.
    pub(super) link: lobby::EngineLink,
}

impl Connection {
    /// Whether this socket may still receive its seat's private frames.
    pub(super) fn active(&self) -> bool {
        self.state
            .lobby
            .lock()
            .games
            .get(&self.game)
            .is_some_and(|game| self.ownership.active(game, self.seat))
    }

    /// Ownership check and synchronous enqueue share one lock; no network I/O inside.
    pub(super) fn send(&self, msg: v1::envelope::Msg) -> bool {
        self.state
            .lobby
            .lock()
            .games
            .get(&self.game)
            .is_some_and(|game| self.ownership.send(game, self.seat, &self.link, msg))
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Some(game) = self.state.lobby.lock().games.get_mut(&self.game) {
            self.ownership.detach(game, self.seat, &self.link);
        }
    }
}

#[cfg(test)]
mod tests;
