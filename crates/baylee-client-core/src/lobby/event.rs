use super::{
    BlockedCard, DeckSummary, GameListing, KeptGuest, ListFor, SeatHandover, hosted, library,
    resume,
};

/// The outcome of a [`super::LobbyRequest`], handed back by the shell.
#[derive(Clone, PartialEq, Eq)]
pub enum LobbyEvent {
    /// Account library response.
    Library(library::Reply),
    /// The account now exists. It comes with no token, so a log-in follows.
    Registered,
    /// Signed in.
    LoggedIn {
        /// The account bearer token, for every later call.
        token: String,
        /// The account's username, as the gateway answered it; `None` from a
        /// gateway older than usernames (#269).
        username: Option<String>,
    },
    /// The same session with a new bearer, and when to renew that one: nothing
    /// else changes, and nothing watching the session starts again.
    BearerRenewed {
        /// The bearer for every later call.
        token: String,
        /// Seconds until it wants renewing.
        renew_in: u64,
    },
    /// A new guest, and its session (#269).
    GuestIn(KeptGuest),
    /// A session ended on the gateway ([`super::LobbyRequest::LogOut`]). Nothing
    /// follows: the lobby forgot it when it asked.
    LoggedOut,
    /// The account's decks.
    Decks(Vec<DeckSummary>),
    /// The playable card pool.
    Pool {
        /// Every card the engine can play.
        cards: Vec<crate::deckbuilder::PoolCard>,
        /// Whether the gateway could serve rules text with them.
        has_text: bool,
    },
    /// The cards blocked on the gateway (`GET /cards/blocked`), asked
    /// beside the pool; never offline.
    BlockedCards(Vec<BlockedCard>),
    /// What the gateway says about its cards beyond their text (`GET
    /// /pool/facts`), asked beside the pool; never offline.
    PoolFacts(std::sync::Arc<crate::cardquery::PoolFacts>),
    /// Every printing of one card.
    Printings {
        /// Which card was asked about.
        card: u32,
        /// Its printings, newest set first.
        printings: Vec<crate::deckbuilder::Printing>,
        /// Whether a catalog answered. `false` means the single printing
        /// below is this build's own reference, not the whole history.
        from_catalog: bool,
    },
    /// The lists this account may use.
    Banlists(Vec<baylee_cardtext::ListRow>),
    /// One list, and what it was asked for.
    Banlist(baylee_cardtext::ListBody, ListFor),
    /// The list of a table; `None` for a table that has none.
    RoomList(Option<baylee_cardtext::ListBody>),
    /// A list is gone, by its id.
    BanlistDeleted(String),
    /// A table's list turned a deck away, and why.
    DeckRefused(baylee_cardtext::DeckRefusal),
    /// One deck, with its rows, ready to edit.
    DeckLoaded {
        /// The deck's id.
        id: String,
        /// Its name.
        name: String,
        /// Its rows.
        cards: Vec<String>,
        /// Its sideboard rows.
        sideboard: Vec<String>,
        /// The cards that lead it, when it is a commander deck.
        commanders: Vec<String>,
        /// What it plays, as the store says.
        format: String,
    },
    /// A deck was saved. `deck_id` is the id `POST /decks` hands back for a
    /// *new* deck; an edit answers `204` and carries none, having had one.
    DeckSaved {
        /// The id the gateway filed it under, when this was a new deck.
        deck_id: Option<String>,
    },
    /// A deck was deleted.
    DeckDeleted,
    /// The account is gone (`DELETE /account` answered `204`), and every one
    /// of its sessions with it.
    AccountDeleted,
    /// A chair was given up, or a room closed.
    Left,
    /// The tables that are open — one page of them.
    Games(GameListing),
    /// Only this account's resumable chairs, in stable preference order.
    Seats(resume::Listing),
    /// A chair moved: somebody sat down, said they were ready, handed the
    /// room on. What changed is not carried, because the page being read is
    /// what has to be redrawn and only this client knows which page that is.
    Moved,
    /// A seat, and the ticket that proves it.
    Seated(SeatHandover),
    /// An already-running challenge seat, including its public task identity.
    /// This is presentation context, never permission to publish a score.
    ChallengeSeated {
        /// The normal authenticated seat grant.
        seat: SeatHandover,
        /// Public task identity used to return to its results.
        challenge: [u8; 16],
    },
    /// An owned gateway-hosted Tower seat; distinct from Central's task identity.
    TowerSeated {
        /// Normal authenticated seat capability.
        seat: SeatHandover,
        /// Owned expedition to reread on returning; never a result claim.
        run: [u8; 16],
    },
    /// The hosted models the gateway offers.
    HostedProfiles(Vec<hosted::HostedProfile>),
    /// The sign-in was right, and the password an admin's reset one
    /// (beta.8): a new one is chosen before anything else.
    PasswordChangeNeeded,
    /// The account is banned (beta.8): its sign-in or its session was
    /// refused with the admin's reason and the ban's end.
    Banned {
        /// The admin's words.
        reason: String,
        /// When it ends, as the gateway said it (RFC 3339, UTC); `None` for
        /// good.
        until: Option<String>,
    },
    /// The request failed, with something worth showing a player.
    Failed(String),
    /// The gateway did not answer at all: no connection, or a proxy saying
    /// that nothing stands behind it. Held until anything answers again.
    GatewayLost,
}
