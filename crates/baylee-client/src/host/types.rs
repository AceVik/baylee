//! The renderer's host contract, independent of transport.
use super::{GameStatic, LogTail, Pending, PlayerAction, PlayerId, PlayerView};

/// Why a table ended without a rules result, rendered in the client's language.
#[derive(Clone, Debug)]
pub enum Closure {
    /// An administrator supplied this explanation.
    Admin(String),
    /// The local worker failed, exceeded its resources or lost its owner.
    Interrupted,
}

/// Something a host tells the client.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum HostMessage {
    /// The once-per-game payload: seats and the print table.
    Static(Box<GameStatic>),
    /// A fresh snapshot of the game, and the game log's lines that came
    /// with it (#262), when there were any.
    ///
    /// One message and not two because the book reads the lines against the
    /// view of their own frame, and a seat with many lines waiting is sent
    /// several frames repeating one view and `seq`, each with the next part
    /// of the log. Every frame's lines count, whatever becomes of its view.
    View(Box<PlayerView>, Option<LogTail>),
    /// A choice addressed to this seat.
    Choice(Box<Pending>),
    /// Something went wrong; the string is safe to show a player.
    Failed(String),
    /// The table is open: every human seat has prepared it and the shared
    /// entrance has finished. Nothing this seat sends is read before it, which is
    /// why the client holds its outbox until then. It never comes down again
    /// in the same game.
    Curtain,
    /// What an AI seat's mind said beside an answer: a debug engine sends it
    /// to every other seat, a release engine to none (`docs/protocol.md`
    /// §"An AI seat's reasoning").
    AiLog(#[serde(with = "encoded_ai_log")] baylee_protocol::v1::AiLog),
    /// How many watch this table (`docs/protocol.md` §"Spectators").
    Spectators(u32),
    /// An admin ended the game (`docs/protocol.md` §"Acting on a game"),
    /// and why: nobody won or lost. The last thing the table says.
    Closed(String),
    /// A local engine was interrupted for a technical reason. No game result
    /// is inferred; the renderer supplies its own localized explanation.
    Interrupted,
    /// A replay moved other than one step on (`docs/client.md`
    /// §"Replays"): what follows is the table at another moment, told from
    /// its first log line. The client forgets the view it holds and the log
    /// book, takes down the end, places the next board without a glide and
    /// plays no cue across it. Only a [`crate::replay::ReplayHost`] sends it.
    Jump,
    /// Human preparation progress and departure relative to receipt, clock corrected.
    Preparing {
        /// Prepared human seats.
        ready: u32,
        /// Expected human seats.
        total: u32,
        /// Seconds until the common portal departure; none also cancels it.
        starts_in: Option<f64>,
    },
}

/// Whether a host still has the connection it plays through.
///
/// A state rather than a message, because the banner that draws it is a
/// function of *now*: a message would have to be cleared by something, and
/// the only thing a client does on a dead socket is wait. It was routed
/// through `Duel::last_error` first, which clears in `submit` — a call a
/// disconnected player cannot make, so the words stayed on the screen after
/// the table came back.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LinkState {
    /// Central is confirming an input or recovering the ranked seat. The
    /// account worker obtains a fresh scoped ticket before play continues.
    RankedHold(baylee_client_core::Phrase),
    /// There is no network connection: a locally owned engine.
    Local,
    /// Connected, or opening for the first time.
    Up,
    /// The socket went away and this host is dialling again.
    Connecting,
    /// The socket went away and nothing is being done about it.
    Down,
    /// The table refused this client's protocol (#271). Final: a table
    /// that said no is never dialled again, and the player is told which
    /// side is behind.
    Refused {
        /// The protocol the table speaks.
        table: u32,
    },
    /// The gateway closed the seat's socket for good and said why, in this
    /// sentence: a ban, or a session that ended. Final as well: every dial
    /// would meet the same answer, and the player would read that the
    /// connection is coming back.
    Closed(baylee_client_core::i18n::Phrase),
}

/// A source of duel state.
///
/// Implementations must be non-blocking: [`DuelHost::poll`] is called once per
/// frame and may not wait on I/O. A networked implementation drains a channel
/// its transport task fills.
pub trait DuelHost: Send + Sync + 'static {
    /// Updated authoritative ranked clocks, including after a replacement
    /// view. The renderer drives their interpolation between Central reads.
    fn take_ranked_clocks(&mut self, _new_view: bool) -> Option<RankedClocks> {
        None
    }
    /// Position of the certified seat projection currently on the screen.
    /// Prevents queued UI input from being reinterpreted if a newer position
    /// arrives between an input event and the normal outbound flush.
    fn ranked_cursor(&self) -> Option<(u64, u64, [u8; 32])> {
        None
    }

    /// Everything that arrived since the last call.
    fn poll(&mut self) -> Vec<HostMessage>;

    /// Sends the local seat's answer.
    fn submit(&mut self, action: PlayerAction);

    /// Sends a seat-level setting (`ShareHand`, `ShareHandOnce`, `RequestHand`,
    /// `DeclineHand`), which neither answers a pending choice nor touches the
    /// engine's snapshot hash.
    fn seat_setting(&mut self, setting: baylee_view::SeatSetting) {
        let _ = setting;
    }

    /// Says this seat has drawn its table (#256), so the engine can open it
    /// once every seat has. Called after the table
    /// and its required assets have been prepared by the renderer.
    ///
    /// No default: a host that forgot it would hold the whole table behind
    /// the curtain for the engine's full wait, and a missing method is the
    /// only way the compiler can say so.
    fn ready(&mut self);

    /// The seat this client plays.
    fn seat(&self) -> PlayerId;

    /// Whether this client only watches (`docs/protocol.md`
    /// §"Spectators"): it holds no chair and answers nothing.
    fn watching(&self) -> bool {
        false
    }

    /// This seat's own decklist, one entry per card, where the host knows it.
    ///
    /// Empty by default: a networked seat is handed its deck by the gateway
    /// and never sees the list, only the cards as they are shown to it.
    fn own_deck(&self) -> Vec<baylee_core::ids::CardIndex> {
        Vec::new()
    }

    /// Whether the connection this host plays through is still there.
    ///
    /// Defaults to [`LinkState::Local`], which is the truthful answer for a host
    /// with no socket: local failures are explicit technical interruptions,
    /// so the network retry schedule never starts.
    fn link(&self) -> LinkState {
        LinkState::Local
    }

    /// Opens the connection again, resuming the seat where it left off.
    ///
    /// Only ever called on a [`LinkState::Down`] host, and deliberately not called
    /// by the host itself: one that redialled on its own would hammer a
    /// gateway that is down, and only the application knows whether a player
    /// is still sitting there. Defaults to doing nothing, for a host that has
    /// nothing to reopen.
    ///
    /// # Errors
    /// When the connection cannot be started at all — a malformed URL, or a
    /// transport that could not be created. Not when the dial *fails*: that
    /// answer arrives later, through [`DuelHost::link`].
    fn reconnect(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// How many dials since the connection was last up the table refused
    /// this seat's ticket on (`401`), for what the reconnect schedule says
    /// when it runs out (`reconnect::gave_up`). Defaults to none, for a host
    /// with nothing to dial.
    fn ticket_refusals(&self) -> u32 {
        0
    }

    /// The seat token this host plays with, for the report form's leak
    /// check (`report::keyring`) and for nothing else.
    ///
    /// Defaults to `None`, the truthful answer for a host with no gateway
    /// behind it: an in-process table has no token to leak.
    fn seat_token(&self) -> Option<&str> {
        None
    }

    /// The record of the game this client hosts itself, as written so far
    /// (`baylee_gamehost::record`), for a report about it to offer.
    ///
    /// Defaults to `None`, the truthful answer for a networked host: its
    /// game's record is the gateway's, which attaches it itself.
    fn local_record(&self) -> Option<&[u8]> {
        None
    }

    /// Where a restart picks this game up again, for the game this client
    /// hosts itself ([`ResumePoint`]); `None` for a networked host, whose
    /// chair the gateway hands back.
    fn resume_point(&self) -> Option<ResumePoint> {
        None
    }

    /// Whether local rules work is still writing the game record. A restart
    /// must wait for its completed checkpoint without blocking the renderer.
    fn busy(&self) -> bool {
        false
    }

    /// Where a replay stands, for the transport bar; `None` for a host
    /// playing a live game.
    fn replay(&self) -> Option<&crate::replay::ReplayStatus> {
        None
    }

    /// Moves a replay (the transport bar, its keys, dev-control). A live
    /// host has nothing to move and ignores it.
    fn control(&mut self, _ask: crate::replay::ReplayAsk) {}

    /// The gateway's id of the game this host shows, where it shows a
    /// hosted game without a seat in it (a replay's tape): the report form
    /// names it when there is no seated handover. `None` otherwise.
    fn game_id(&self) -> Option<&str> {
        None
    }
}

/// Public timeout information for a two-seat protected table.
pub struct RankedClocks {
    /// Remaining decision seconds per seat.
    pub decisions: [Option<u16>; 2],
    /// Remaining reconnection seconds per seat.
    pub reconnect: [Option<u16>; 2],
    /// Whether elapsed time is currently chargeable.
    pub running: bool,
}

/// What a restart needs to rebuild a game this client hosts
/// (`baylee_client_core::resume::Game::Local`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResumePoint {
    /// The record's file among the kept records.
    pub record: String,
    /// The local seat.
    pub seat: PlayerId,
    /// What each seat is called.
    pub names: Vec<String>,
    /// The engine's hash now, as the record writes it.
    pub hash: String,
}

/// Protobuf's debug note travels unchanged inside a private worker frame.
mod encoded_ai_log {
    use prost::Message as _;
    use serde::{Deserialize as _, Serialize as _};

    pub(super) fn serialize<S: serde::Serializer>(
        note: &baylee_protocol::v1::AiLog,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        note.encode_to_vec().serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<baylee_protocol::v1::AiLog, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        baylee_protocol::v1::AiLog::decode(bytes.as_slice()).map_err(serde::de::Error::custom)
    }
}
