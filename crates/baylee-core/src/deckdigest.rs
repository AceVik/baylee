//! What the lobby's deck list says about a deck beyond its name (#254).
//!
//! A stored deck is rows of text ([`crate::deckrow`]). That is not what a
//! player picking between two decks asks, which is how many cards, in which
//! colours, led by whom. These types are that answer, as `GET /decks` sends
//! it and the lobby reads it. They are here, beside the rows they are read
//! from, because both ends name them. The reading itself needs the card
//! registry and is `baylee_cards::digest`.

use crate::preset::Finish;
use serde::{Deserialize, Serialize};

/// One deck, as the lobby's list describes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Digest {
    /// Cards in the deck, counting copies: `4 Llanowar Elves` is four.
    pub copies: u32,
    /// Cards in the sideboard, the same way.
    pub side_copies: u32,
    /// The deck's colour identity as `WUBRG` letters, empty for colourless.
    ///
    /// For a deck with commanders it is their combined colour identity
    /// (CR 903.4), both of a partner pair's together, which is what bounds
    /// every other card in it. For a deck without one it is every main-deck
    /// card's identity together: the colours the deck plays, which is what
    /// a player picking between two decks is looking for.
    pub identity: String,
    /// The commanders, in the deck's order, each with the picture the deck
    /// shows it with.
    pub leaders: Vec<Leader>,
}

/// A commander and the printing to picture it by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Leader {
    /// Its English name, as the deck stores it.
    pub name: String,
    /// The printing the deck's row names, else the one the registry
    /// references. Empty when the reader does not know the card.
    pub scryfall_id: String,
    /// The language of that printing.
    pub lang: String,
    /// The finish the row names; non-foil when it names none.
    pub finish: Finish,
    /// Whether that card has a second picture to turn to.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_back_image: bool,
}
