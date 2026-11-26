//! A kept game's row: what its record cannot say, at [`VERSION`].
//!
//! Its own shape rather than the lobby's types serialised whole: what is
//! written here is read back by the next build, so a field moves here only
//! on purpose, with [`VERSION`] and the golden test (`tests.rs`) beside it.
//! Nothing that lives only while this process does is here (sockets, the
//! engine link, the outboxes, spectators); the preset and the execution
//! identity are the record's header.

use crate::lobby::{
    Delegate, HostedChair, Kept, LobbyGame, LobbySeat, LobbyState, RoomList, SeatKind,
};
use crate::store::Deck;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use baylee_core::preset::{GamePreset, RoomSetup};
use baylee_protocol::execution::{Identity, Variant};
use serde::{Deserialize, Serialize};

/// The shape of [`Row`]. A row of another version is not read: its game
/// ends at the restart, as every running game did before rows were kept.
pub(crate) const VERSION: i32 = 1;

/// One kept game.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Row {
    /// The account that opened the room.
    pub(crate) host: Option<String>,
    /// What the host called the table.
    pub(crate) name: String,
    /// The format the table says it plays.
    pub(crate) format: Option<String>,
    /// The list the decks passed, as the room held it.
    pub(crate) list: Option<List>,
    /// The custom starting state.
    pub(crate) setup: RoomSetup,
    /// Whether people with no chair may watch.
    pub(crate) allow_spectators: bool,
    /// The room's password, hashed.
    pub(crate) password_hash: Option<String>,
    /// When the room was opened (unix seconds).
    pub(crate) created_at: u64,
    /// When the game started (unix seconds).
    pub(crate) started_at: Option<u64>,
    /// The next arrival number.
    pub(crate) next_seq: u64,
    /// The finished game this one is a rematch of.
    pub(crate) parent: Option<String>,
    /// The chairs, in seat order.
    pub(crate) seats: Vec<Chair>,
}

/// A room's list as a row holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct List {
    /// The list's id in the store.
    pub(crate) id: uuid::Uuid,
    /// The list and its cards.
    pub(crate) body: baylee_cardtext::ListBody,
}

/// One chair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Chair {
    /// Whether it is a person's or the house's.
    pub(crate) kind: SeatKind,
    /// The house's difficulty.
    pub(crate) ai: Option<String>,
    /// Who sits in it.
    pub(crate) account: Option<String>,
    /// The SHA-256 of the token that opens it.
    pub(crate) token_hash: Option<String>,
    /// The session key it is bound to, in base64.
    pub(crate) session: Option<String>,
    /// Whether its engine said its participation ended.
    pub(crate) participation_ended: bool,
    /// The deck's name.
    pub(crate) deck_name: String,
    /// The deck.
    pub(crate) deck: Option<Deck>,
    /// The team.
    pub(crate) team: Option<u8>,
    /// Its arrival number.
    pub(crate) joined_seq: Option<u64>,
    /// The seat bridge sitting in it on its host's word.
    pub(crate) delegate: Option<Delegate>,
    /// The hosted model ordered for it.
    pub(crate) hosted: Option<HostedChair>,
}

/// Whether `game` is kept at all: a running casual game whose engine was
/// ordered. Ranked duels (replicated through Central), Tower runs and
/// challenges recover by their own rules, and keep them
/// (`docs/proposals/games-survive-restarts.md`).
pub(crate) fn keepable(game: &LobbyGame) -> bool {
    game.state == LobbyState::Playing
        && game.kept != Kept::Cut
        && game.ranked.is_none()
        && game.challenge.is_none()
        && game.challenge_task.is_none()
        && game.challenge_executor.is_none()
        && game.engine_job.is_none()
        && game.preset.is_some()
        && game
            .execution
            .as_ref()
            .is_some_and(|identity| identity.variant != Variant::Tower)
}

impl Row {
    /// `game` as its row, or `None` when it is not kept ([`keepable`]).
    pub(crate) fn of(game: &LobbyGame) -> Option<Self> {
        keepable(game).then(|| Self {
            host: game.host.clone(),
            name: game.name.clone(),
            format: game.format.clone(),
            list: game.list.as_ref().map(|list| List {
                id: list.id,
                body: (*list.body).clone(),
            }),
            setup: game.setup.clone(),
            allow_spectators: game.allow_spectators,
            password_hash: game.password_hash.clone(),
            created_at: game.created_at,
            started_at: game.started_at,
            next_seq: game.next_seq,
            parent: game.parent.clone(),
            seats: game.seats.iter().map(Chair::of).collect(),
        })
    }

    /// The game again, waiting for its engine: running, with the preset and
    /// the artifact its record's header names, to be resumed from its record
    /// ([`LobbyGame::restored`]).
    pub(crate) fn game(self, id: String, preset: GamePreset, execution: Identity) -> LobbyGame {
        let mut game = LobbyGame::blank(id, self.created_at);
        game.state = LobbyState::Playing;
        game.host = self.host;
        game.name = self.name;
        game.format = self.format;
        game.list = self.list.map(|list| RoomList::of(list.id, list.body));
        game.setup = self.setup;
        game.allow_spectators = self.allow_spectators;
        game.password_hash = self.password_hash;
        game.started_at = self.started_at;
        game.next_seq = self.next_seq;
        game.parent = self.parent;
        game.seats = self
            .seats
            .into_iter()
            .enumerate()
            .map(|(seat, chair)| chair.seat(seat))
            .collect();
        game.house_rules = preset.house_rules.clone();
        game.preset = Some(preset);
        game.execution = Some(execution);
        game.kept = Kept::Restored;
        game
    }

    /// The accounts the row names: in a chair, behind a seat bridge, or as
    /// the host.
    pub(crate) fn accounts(&self) -> Vec<String> {
        let mut accounts: Vec<String> = self
            .seats
            .iter()
            .flat_map(|chair| {
                chair
                    .account
                    .iter()
                    .chain(chair.delegate.iter().map(|d| &d.by))
                    .cloned()
            })
            .chain(self.host.iter().cloned())
            .collect();
        accounts.sort();
        accounts.dedup();
        accounts
    }
}

impl Chair {
    fn of(seat: &LobbySeat) -> Self {
        Self {
            kind: seat.kind,
            ai: seat.ai.clone(),
            account: seat.account_id.clone(),
            token_hash: seat.seat_token_hash.clone(),
            session: seat
                .central_session
                .as_deref()
                .map(|key| BASE64.encode(key)),
            participation_ended: seat.participation_ended,
            deck_name: seat.deck_name.clone(),
            deck: seat.deck.clone(),
            team: seat.team,
            joined_seq: seat.joined_seq,
            delegate: seat.delegate.clone(),
            hosted: seat.hosted.clone(),
        }
    }

    /// The chair at `seat`. A session key that is not base64 binds the chair
    /// to no session that can exist, so it opens for nobody until reclaimed.
    fn seat(self, seat: usize) -> LobbySeat {
        let mut chair = LobbySeat::open(seat);
        chair.kind = self.kind;
        chair.ai = self.ai;
        chair.account_id = self.account;
        chair.seat_token_hash = self.token_hash;
        chair.central_session = self.session.map(|key| {
            BASE64
                .decode(key)
                .unwrap_or_else(|_| b"unreadable".to_vec())
        });
        chair.participation_ended = self.participation_ended;
        chair.deck_name = self.deck_name;
        chair.deck = self.deck;
        chair.team = self.team;
        chair.joined_seq = self.joined_seq;
        chair.delegate = self.delegate;
        chair.hosted = self.hosted;
        chair
    }
}
