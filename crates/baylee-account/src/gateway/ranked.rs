//! Portable private deck submission to a single assigned game host.
use super::Error;
use crate::ranked::Format;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Immutable source-deck rows, including the zones actually played. Private to
/// the player and the gateway assigned to run the game.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RankedDeck {
    /// Independent ranked queue.
    pub format: Format,
    /// Main-deck lines in the normal deck export notation.
    pub cards: Vec<String>,
    /// Registered sideboard, kept separate for the game.
    pub sideboard: Vec<String>,
    /// Command-zone names, in their printed order.
    pub commanders: Vec<String>,
}

/// A gateway-local ranked seat capability. Intentionally has no Debug implementation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankedSeat {
    /// The engine game selected by Central.
    pub game_id: Uuid,
    /// The account's assigned seat in this duel.
    pub seat: u32,
    /// Transport credential, wiped on drop.
    #[serde(deserialize_with = "super::seat_secret")]
    pub seat_token: zeroize::Zeroizing<String>,
}

impl RankedSeat {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.seat > 1
            || self.game_id.get_version_num() != 7
            || self.game_id.get_variant() != uuid::Variant::RFC4122
            || self.seat_token.len() != 64
            || !self
                .seat_token
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}
