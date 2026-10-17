//! Spectators (`docs/protocol.md` §"Spectators"): the public table, told to
//! nobody in particular.
//!
//! Every spectator of a game is shown the same thing, so the host keeps one
//! cursor and one print table for all of them and the engine sends one
//! stream, which the gateway hands to every watching socket. A spectator is
//! no seat: it is never asked, never on a clock, never waited for, and has
//! nothing it could answer with ([`Session::act`] takes a seat).

use super::{Session, choice_envelope, frames_of};
use crate::view::SPECTATOR;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{LogTail, PlayerView, SpectatorView};

/// What the host has told the spectators of one game, together.
#[derive(Debug, Default)]
pub(crate) struct Gallery {
    /// How many spectator sockets the gateway says are open.
    pub(crate) count: u32,
    /// The log lines already sent to the spectators' stream.
    pub(crate) told: usize,
    /// The printings the spectators have been shown, by view or log. Starts
    /// empty: a spectator has no deck, so it earns every entry by seeing the
    /// card in public, and a decklist never leaks through the print table.
    pub(crate) revealed: Vec<bool>,
}

impl Session {
    /// How many spectators the gateway says are watching.
    #[must_use]
    pub const fn spectators(&self) -> u32 {
        self.gallery.count
    }

    /// The gateway's count of open spectator sockets. Moves nothing in the
    /// game; the engine tells the seats.
    pub fn set_spectators(&mut self, count: u32) {
        self.gallery.count = count;
    }

    /// The public view as it stands, built for no seat.
    #[must_use]
    pub fn spectator_view(&self) -> SpectatorView {
        crate::view::spectator_view(
            self.engine.state(),
            self.seq,
            self.awaiting_seat(),
            self.deciding(),
            self.engine.library_reveal_blocked(),
            &self.house_answered,
        )
    }

    /// The opening payload of a spectator's stream: the roster and the
    /// printings spectators have earned, never a seat's deck. `your_seat` is
    /// the seat the table is drawn from, seat 0; a spectator holds none.
    #[must_use]
    pub fn spectator_game_static(&self) -> Envelope {
        self.public_game_static(PlayerId::new(0))
    }

    /// [`Session::spectator_game_static`] drawn from `seat`: the roster and
    /// the printings earned in public, never a deck. A replay's view of a
    /// seat whose hidden cards it may not show (`project.rs`).
    pub(super) fn public_game_static(&self, seat: PlayerId) -> Envelope {
        let mut statics = self.game_static(seat);
        statics.prints = crate::view::game_static(
            String::new(),
            seat,
            Vec::new(),
            &self.prints,
            &self.gallery.revealed,
            &self.house_rules,
        )
        .prints;
        Envelope {
            msg: Some(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
                game_id: self.game_id.clone(),
                view_version: baylee_view::VIEW_VERSION,
                static_json: serde_json::to_vec(&statics).unwrap_or_default(),
            })),
        }
    }

    /// The spectators' next frames: the public view and the log lines not
    /// sent yet, as nobody in particular may know them, after a fresh opening
    /// payload when these show a printing for the first time. The game's end,
    /// which everyone is told, follows it.
    pub fn spectator_envelopes(&mut self) -> Vec<Envelope> {
        let view = self.spectator_view();
        let lines = self.log.len();
        let from = self.gallery.told.min(lines);
        self.gallery.told = lines;
        self.log.seal(lines);
        let tail = LogTail {
            from: u32::try_from(from).unwrap_or(u32::MAX),
            entries: self.log.told(SPECTATOR, from, lines),
        };
        let mut grew = false;
        for print in view.prints().chain(tail.prints()) {
            if let Some(slot) = self.gallery.revealed.get_mut(print.get() as usize)
                && !*slot
            {
                *slot = true;
                grew = true;
            }
        }
        let mut out = Vec::new();
        if grew {
            out.push(self.spectator_game_static());
        }
        out.extend(frames_of(self.seq, super::view_json(&view), tail));
        let pending = self.engine.pending();
        if matches!(pending, Pending::GameOver(_)) {
            out.push(choice_envelope(self.seq, pending));
        }
        out
    }

    /// The public table drawn from `seat` as a `PlayerView`, with the
    /// spectators' log: a replay's view of a seat whose hidden cards it may
    /// not show (`project.rs`), `view` built from [`Session::spectator_view`]
    /// (`SpectatorView::into_player_view`) plus the hands it may show. Shares
    /// the spectators' cursor and print table, so a projection uses one or
    /// the other stream, never both.
    pub(super) fn public_envelopes_for(
        &mut self,
        seat: PlayerId,
        view: &PlayerView,
    ) -> Vec<Envelope> {
        let lines = self.log.len();
        let from = self.gallery.told.min(lines);
        self.gallery.told = lines;
        self.log.seal(lines);
        let tail = LogTail {
            from: u32::try_from(from).unwrap_or(u32::MAX),
            entries: self.log.told(SPECTATOR, from, lines),
        };
        let mut grew = false;
        for print in view.prints().chain(tail.prints()) {
            if let Some(slot) = self.gallery.revealed.get_mut(print.get() as usize)
                && !*slot
            {
                *slot = true;
                grew = true;
            }
        }
        let mut out = Vec::new();
        if grew {
            out.push(self.public_game_static(seat));
        }
        out.extend(super::state_frames(self.seq, view, tail));
        out
    }

    /// Adds the spectators' frames to a pump's, addressed to [`SPECTATOR`],
    /// while anybody watches: the one place a moving game tells them.
    pub(super) fn tell_spectators(&mut self, out: &mut Vec<(PlayerId, Envelope)>) {
        if self.gallery.count > 0 {
            let frames = self.spectator_envelopes();
            out.extend(frames.into_iter().map(|env| (SPECTATOR, env)));
        }
    }

    /// Everything a spectator needs from scratch: the opening payload, the
    /// view and the whole log. Every spectator shares the stream, so the
    /// others are sent the log again; a client appends by `LogTail::from`.
    pub fn spectator_snapshot(&mut self) -> Vec<Envelope> {
        self.gallery.told = 0;
        let mut out = Vec::new();
        let rest = self.spectator_envelopes();
        // The payload first, and once: the walk above may already have
        // grown the print table and put one at its head.
        if !matches!(
            rest.first().and_then(|e| e.msg.as_ref()),
            Some(v1::envelope::Msg::GameStatic(_))
        ) {
            out.push(self.spectator_game_static());
        }
        out.extend(rest);
        out
    }
}
