//! Watching a game again (`.claude/b8/REPLAY-DESIGN.md` §6): the tape a
//! replay plays, its timeline, the transport that walks it, and the list of
//! games there are to watch.
//!
//! Everything here is pure: no clock (the transport is handed its ticks), no
//! disk, no socket. The renderer's `ReplayHost` owns a [`Tape`], a
//! [`Timeline`] and a [`Transport`] and turns the transport's [`Out`]s into
//! the messages a live host sends.
//!
//! ```text
//!   tape (gzip JSON Lines) ──Tape::read──┐
//!   a local record ──project──moments────┴─> Tape ──Timeline::of──> Timeline
//!                                                         │
//!   Command ──> Transport::apply ──> Out::{Show, Jump} ───┘ (the host sends frames)
//! ```

pub mod library;
pub mod tape;
pub mod timeline;
pub mod transport;
pub mod wire;
pub mod words;

pub use tape::{Frame, Tape, TapeMoment, TapeRefusal};
pub use timeline::Timeline;
pub use transport::{Command, Out, Speed, Transport};
pub use wire::{
    Check, Diverged, DivergedBy, Header, Source, StopLine, StopReason, TAPE_VERSION,
    TapePerspective, perspective_of, tape_perspective,
};

/// Whose table a replay draws (§3.3).
///
/// The client's mirror of `baylee_gamehost::replay::Perspective`, which this
/// crate cannot link, and of the tape header's `perspective`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Perspective {
    /// One seat, as a socket at it was told the game.
    Seat(baylee_core::ids::PlayerId),
    /// The spectators' table: no hand at all.
    Public,
    /// One seat with every other hand beside its own (games against the
    /// house only).
    AllHands(baylee_core::ids::PlayerId),
}

impl Perspective {
    /// The kind, as the tape's header and the replay route spell it.
    #[must_use]
    pub const fn wire(self) -> &'static str {
        match self {
            Self::Seat(_) => "seat",
            Self::Public => "public",
            Self::AllHands(_) => "all_hands",
        }
    }

    /// The seat the table is drawn from: the spectators' is seat 0, as the
    /// spectator socket's.
    #[must_use]
    pub const fn seat(self) -> baylee_core::ids::PlayerId {
        match self {
            Self::Seat(seat) | Self::AllHands(seat) => seat,
            Self::Public => baylee_core::ids::PlayerId::new(0),
        }
    }

    /// The same kind of perspective from `seat` (public stays public).
    #[must_use]
    pub const fn at(self, seat: baylee_core::ids::PlayerId) -> Self {
        match self {
            Self::Seat(_) => Self::Seat(seat),
            Self::Public => Self::Public,
            Self::AllHands(_) => Self::AllHands(seat),
        }
    }
}

/// What a check that fails does, as the replay route and
/// `baylee_gamehost::replay::Verify` spell it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Verify {
    /// Stop where the game would differ; everything before it stands.
    #[default]
    Stop,
    /// Go on, noting where: the player asked to play on.
    Note,
}

impl Verify {
    /// The route's spelling.
    #[must_use]
    pub const fn wire(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Note => "note",
        }
    }
}

/// Which perspectives a source offers (§3.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[allow(clippy::struct_excessive_bools)] // three independent offers, as the table in §3.3
pub struct Perspectives {
    /// The viewer's own seat.
    pub seat: bool,
    /// The spectators' table.
    pub public: bool,
    /// Every hand (a record its holder already holds whole).
    pub all_hands: bool,
}

impl Perspectives {
    /// A local record's, and a record file's: all three.
    pub const LOCAL: Self = Self {
        seat: true,
        public: true,
        all_hands: true,
    };

    /// Whether `perspective` is among them.
    #[must_use]
    pub const fn offers(self, perspective: Perspective) -> bool {
        match perspective {
            Perspective::Seat(_) => self.seat,
            Perspective::Public => self.public,
            Perspective::AllHands(_) => self.all_hands,
        }
    }

    /// The offered perspective after `now`, in the order the pill steps
    /// (seat, table, all hands), from `seat`; `now` when it is the only one.
    #[must_use]
    pub fn after(self, now: Perspective, seat: baylee_core::ids::PlayerId) -> Perspective {
        let order = [
            Perspective::Seat(seat),
            Perspective::Public,
            Perspective::AllHands(seat),
        ];
        let at = order
            .iter()
            .position(|p| std::mem::discriminant(p) == std::mem::discriminant(&now))
            .unwrap_or(0);
        (1..=order.len())
            .map(|k| order[(at + k) % order.len()])
            .find(|p| self.offers(*p))
            .unwrap_or(now)
    }
}

#[cfg(test)]
mod tests;
