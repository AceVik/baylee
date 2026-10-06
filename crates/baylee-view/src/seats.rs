use crate::objects::CardIdentity;
use baylee_core::ids::{ObjectId, PlayerId};
use serde::{Deserialize, Serialize};

// --------------------------------------------------------------------- seats

/// Mana floating in a seat's pool.
///
/// Public information: everyone at a real table can see what you have
/// floating, and the seat that has to decide whether to tap another land
/// needs it — which is why it lives here and not only in the engine.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
pub struct ManaPoolView {
    /// Current permissions for spending this seat's mana as other types.
    /// Counts below retain the mana's actual colors (CR 609.4b).
    #[serde(default)]
    pub spending: baylee_core::mana::ManaSpending,
    /// White mana.
    pub white: u32,
    /// Blue mana.
    pub blue: u32,
    /// Black mana.
    pub black: u32,
    /// Red mana.
    pub red: u32,
    /// Green mana.
    pub green: u32,
    /// Colorless mana.
    pub colorless: u32,
    /// Mana that may only be spent on certain spells (Cavern of Souls), by
    /// colour — indexed the way [`baylee_core::mana::ManaColor::index`]
    /// indexes, so `restricted[ManaColor::White.index()]` is restricted white
    /// mana.
    ///
    /// This was a single uncoloured total until `VIEW_VERSION` 14, and that
    /// made the pool unreadable exactly where a player has to read it: naming
    /// white off a Cavern of Souls put an uncoloured number on the screen, so
    /// there was no way to tell whether the choice had taken — or which of two
    /// taps had produced what. *What* the restriction permits stays an engine
    /// question, answered when the payment is attempted; which colour is under
    /// it is a fact the player chose a moment ago and is owed back.
    pub restricted: [u64; 6],
}

impl ManaPoolView {
    /// Everything in the pool, restricted mana included.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.white as u64
            + self.blue as u64
            + self.black as u64
            + self.red as u64
            + self.green as u64
            + self.colorless as u64
            + self.restricted_total()
    }

    /// Just the restricted mana, whatever colour it is under.
    #[must_use]
    pub const fn restricted_total(&self) -> u64 {
        let mut sum = 0;
        let mut i = 0;
        while i < self.restricted.len() {
            sum += self.restricted[i];
            i += 1;
        }
        sum
    }

    /// Whether nothing is floating.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// The public line of one seat: life, counters, and zone sizes.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SeatView {
    /// Seat handle.
    pub player: PlayerId,
    /// Life total.
    pub life: i32,
    /// Poison counters.
    pub poison: u16,
    /// Energy counters.
    pub energy: u16,
    /// Cards in hand. Contents are only in [`PlayerView::hand`], and only for
    /// the viewing seat.
    ///
    /// [`PlayerView::hand`]: crate::PlayerView::hand
    pub hand_count: u32,
    /// An active public effect removes this seat's maximum hand size.
    /// False for older hosts that do not publish this optional field.
    #[serde(default)]
    pub no_max_hand_size: bool,
    /// Cards left in the library.
    pub library_count: u32,
    /// Cards in the graveyard, so a client can show the count without
    /// rendering the pile.
    pub graveyard_count: u32,
    /// Why the seat lost the game, or `None` while it is still in it.
    ///
    /// Public: a seat going out is announced at a real table, and so is why.
    pub loss: Option<LossCause>,
    /// Who answered this seat's most recent decision, when it was not the
    /// seat itself: the decision clock, or the house standing in for a
    /// player whose socket is gone. `None` when the seat answered its own
    /// last decision.
    ///
    /// It clears only on the seat's own next answer over a socket.
    /// Reconnecting does not clear it, because the last decision is still
    /// the house's until the player makes one; neither does an automation
    /// setting, which is not an answer. An AI chair never carries it: the
    /// roster already says the house plays that chair. A seat that has lost
    /// is asked nothing more, so the value it had then stays, which is what
    /// tells a loss to the clock from a loss the player played out.
    pub house_answered: Option<HouseAnswer>,
    /// Mana floating in this seat's pool.
    pub mana_pool: ManaPoolView,
    /// This seat's commanders (CR 903.3), in the order they were designated.
    ///
    /// Empty in every format but Commander, which is what a client keys the
    /// command zone's panel on.
    pub commanders: Vec<CommanderView>,
    /// Combat damage this seat has *taken*, per commander that dealt it
    /// (CR 903.10a): twenty-one from one of them and the seat loses whatever
    /// its life total says, so this is a second life total and is shown like
    /// one.
    ///
    /// Public, and public to everyone — the tally is a shared count at a real
    /// table. The `source` names a commander in some seat's
    /// [`SeatView::commanders`], which is what a client resolves it through;
    /// seats with no damage from a given commander simply have no entry.
    pub commander_damage: Vec<CommanderDamage>,
}

impl SeatView {
    /// Whether the seat has lost the game.
    #[must_use]
    pub const fn has_lost(&self) -> bool {
        self.loss.is_some()
    }
}

/// Why a seat lost the game (CR 104.3).
///
/// A mirror of the engine's reason rather than the engine's type, so renaming
/// an engine variant cannot silently change the protocol.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LossCause {
    /// Life total reached 0 or less (CR 104.3b).
    Life,
    /// Drew from an empty library (CR 104.3c).
    EmptyDraw,
    /// Ten or more poison counters (CR 104.3d).
    Poison,
    /// Twenty-one combat damage from one commander (CR 903.10a).
    CommanderDamage,
    /// Conceded (CR 104.3a).
    Conceded,
    /// An effect said the seat loses (CR 104.3e), such as a pact left
    /// unpaid.
    Effect,
}

/// Who answered a seat's decision in its place.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum HouseAnswer {
    /// The seat's socket was there, and its decision clock ran out.
    Clock,
    /// The seat had no socket, and the house was standing in for the player.
    StandIn,
}

/// One of a seat's commanders, and what casting it has cost so far.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CommanderView {
    /// The commander's object handle.
    ///
    /// Stable across zone changes: an [`ObjectId`] survives the moves that
    /// make a card a new object (CR 400.7), which is what lets a damage tally
    /// and a cast count follow one commander through dying, going home and
    /// coming back down.
    pub object: ObjectId,
    /// Its card.
    ///
    /// Known to every seat, in every zone — including its owner's hand or
    /// library, which is where CR 903.9b's replacement leaves it when
    /// declined. A commander is designated openly (CR 903.3), so this is not
    /// the hidden-zone leak it would be for any other card: the seat learned
    /// this identity from the command zone before the first turn.
    pub card: Option<CardIdentity>,
    /// Its name, for a client with no printing for it yet.
    pub name: String,
    /// Times it has been cast from the command zone.
    ///
    /// CR 903.8's tax in full: `{2}` more generic for each. Per *commander*
    /// and not per seat — a seat with partners pays each one's tax
    /// separately, and a single number could not say so.
    pub casts: u32,
}

/// Commander damage one seat has taken from one commander (CR 903.10a).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CommanderDamage {
    /// The commander that dealt it.
    pub source: ObjectId,
    /// How much, over the whole game. Twenty-one is lethal.
    pub amount: u16,
}

impl SeatView {
    /// Whether the seat is in danger from an empty library — clients warn at
    /// the point where drawing is imminent rather than after the loss.
    #[must_use]
    pub const fn is_decking_out(&self) -> bool {
        self.library_count <= 3
    }
}
