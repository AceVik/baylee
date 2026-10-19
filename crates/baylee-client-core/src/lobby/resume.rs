//! Account-bound seat discovery, independent of the public lobby's pages.

use serde::{Deserialize, Serialize};

/// The gateway returns at most this many seats, playing games first.
pub const MAX_SEATS: usize = 32;
/// The compact response has no names, decks or credentials.
pub const MAX_BYTES: usize = 16 * 1024;

/// A chair the authenticated account may take back without joining again.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    /// The existing game, never a newly created one.
    pub game_id: String,
    /// The account's chair at that game.
    pub seat: u32,
    /// Whether its engine is already running.
    pub playing: bool,
}

/// `GET /lobby/seats`: only the authenticated account's resumable chairs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Listing {
    /// Stable order: playing first, newest first, game ID as tie-breaker.
    pub seats: Vec<Seat>,
}

impl Listing {
    /// Reads the bounded wire response and rejects invalid route identifiers,
    /// impossible seats and duplicate games before any reclaim is requested.
    #[must_use]
    pub fn parse(body: &str) -> Option<Self> {
        if body.len() > MAX_BYTES {
            return None;
        }
        let listing: Self = serde_json::from_str(body).ok()?;
        if listing.seats.len() > MAX_SEATS {
            return None;
        }
        for (at, seat) in listing.seats.iter().enumerate() {
            let id = uuid::Uuid::parse_str(&seat.game_id).ok()?;
            if seat.game_id != id.hyphenated().to_string()
                || seat.seat >= 8
                || listing.seats[..at]
                    .iter()
                    .any(|old| old.game_id == seat.game_id)
            {
                return None;
            }
        }
        Some(listing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_rejects_oversized_duplicate_and_path_injecting_rows() {
        let row = Seat {
            game_id: "019e7e31-0000-7000-8000-000000000001".into(),
            seat: 7,
            playing: true,
        };
        let encode = |seats| serde_json::to_string(&Listing { seats }).unwrap();
        assert!(Listing::parse(&encode(vec![row.clone()])).is_some());
        assert!(Listing::parse(&encode(vec![row.clone(); 2])).is_none());
        assert!(Listing::parse(&encode(vec![row.clone(); MAX_SEATS + 1])).is_none());
        for (game_id, seat) in [("../auth/logout", 0), (row.game_id.as_str(), 8)] {
            assert!(
                Listing::parse(&encode(vec![Seat {
                    game_id: game_id.into(),
                    seat,
                    playing: true,
                }]))
                .is_none()
            );
        }
        assert!(Listing::parse(&" ".repeat(MAX_BYTES + 1)).is_none());
        assert!(Listing::parse(r#"{"games":[]}"#).is_none());
    }
}
