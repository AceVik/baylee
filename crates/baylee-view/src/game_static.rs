use baylee_core::ids::{PlayerId, PrintRef};
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------- prints

/// How a card is finished, which selects the art treatment a client renders.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum Finish {
    /// Ordinary print.
    #[default]
    Normal,
    /// Traditional foil.
    Foil,
    /// Etched foil.
    Etched,
    /// Cosmetic holographic treatment.
    Holographic,
    /// Cosmetic glitter treatment.
    Glitter,
    /// Cosmetic galaxy treatment.
    Galaxy,
}

/// One entry of the game's print table.
///
/// [`PrintRef`] indexes this table. The rules engine never reads it — it exists
/// purely so a client can fetch the right artwork, in the right language, with
/// the right finish.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PrintEntry {
    /// Scryfall printing id (the image cache key).
    pub scryfall_id: String,
    /// Two-letter language code of the printing, e.g. `EN`, `DE`.
    pub lang: String,
    /// Finish of this printing.
    pub finish: Finish,
}

// -------------------------------------------------------------------- static

/// Who occupies a seat. Sent once per game, not with every view.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SeatIdentity {
    /// Seat handle.
    pub player: PlayerId,
    /// Display name shown at the seat.
    pub display_name: String,
    /// Whether the seat is played by the house AI.
    pub is_ai: bool,
    /// Whether a player's chair is being held by the house because nobody is
    /// on the other end of it right now.
    ///
    /// Deliberately *not* the same field as `is_ai`: a chair the house is
    /// standing in for still belongs to the player who left it, and a seat
    /// that renamed itself to "the house AI" after a thirty-second hiccup
    /// would be telling the table something that is not true. The two are
    /// never both set.
    pub away: bool,
    /// Team, for multiplayer formats where seats are allied.
    pub team: Option<u8>,
}

/// The part of a game that never changes, sent once when a client attaches.
///
/// Splitting this out keeps every subsequent [`PlayerView`] small: the print
/// table alone would otherwise be re-sent on every single state change.
///
/// [`PlayerView`]: crate::PlayerView
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct GameStatic {
    /// View protocol version; a client refuses a mismatch.
    pub view_version: u32,
    /// Game handle.
    pub game_id: String,
    /// The seat this client occupies.
    pub your_seat: PlayerId,
    /// All seats in seat order.
    pub seats: Vec<SeatIdentity>,
    /// The print table indexed by [`PrintRef`].
    ///
    /// `None` where the viewing seat has not been shown that printing. The
    /// table is shared by the whole game and deduplicated per card, so sending
    /// all of it would hand every seat the union of both decklists — the one
    /// piece of hidden information with no game object to hide behind. A seat
    /// is entitled to its own deck's printings from the start and earns the
    /// rest by seeing the cards; a host re-sends this payload when it does.
    pub prints: Vec<Option<PrintEntry>>,
    /// How long a seat has to answer one question, in seconds, or `None` at a
    /// table with no decision clock.
    ///
    /// The **limit**, not a countdown: [`PlayerView::decision_remaining_ms`]
    /// is the number that moves, and this is the one it starts from. A player
    /// choosing a table is choosing a pace, and stating it once here is what
    /// lets a seat sheet say so without a per-question warning claiming a
    /// three-second table is nearly over.
    ///
    /// [`PlayerView::decision_remaining_ms`]: crate::PlayerView::decision_remaining_ms
    pub decision_secs: Option<u32>,
    /// How long a seat may be gone before the house answers for it, in
    /// seconds, or `None` at a table that waits forever.
    ///
    /// Known at join or never: a client is disconnected for exactly the
    /// window it would be counting, so no in-game payload can reach it while
    /// the number matters.
    ///
    /// Both of these are an `Option` rather than a bare number because the
    /// house rules spell *no limit* as zero, and zero on a screen reads as
    /// the opposite — a seat with no time at all. A gateway room cannot
    /// choose it (`clock::MIN_RECONNECT_SECS` is 10) but a local harness can,
    /// and this payload has to be honest about a table the gateway did not
    /// make.
    pub reconnect_secs: Option<u32>,
}

impl GameStatic {
    /// Resolves a print reference against the print table.
    #[must_use]
    pub fn print(&self, print: PrintRef) -> Option<&PrintEntry> {
        self.prints.get(print.get() as usize)?.as_ref()
    }

    /// The display name of a seat, or a stable fallback when the seat is
    /// unknown to this client.
    #[must_use]
    pub fn seat_name(&self, player: PlayerId) -> &str {
        self.seats
            .iter()
            .find(|s| s.player == player)
            .map_or("Unknown seat", |s| s.display_name.as_str())
    }
}
