//! Portable private deck submission to a single assigned game host.
use super::{Error, GatewayHttp};
use crate::ranked::{Format, Id};
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

impl GatewayHttp {
    pub(crate) fn ranked_play(
        &self,
        bearer: &str,
        id: Id,
        deck: &RankedDeck,
    ) -> Result<Option<RankedSeat>, Error> {
        let id = Uuid::from_bytes(id);
        let body = central_api::encode(deck)?;
        let response = self
            .agent
            .post(format!("{}/ranked/{id}/play", self.origin))
            .header("Authorization", &format!("Bearer {bearer}"))
            .header("Content-Type", "application/json")
            .send(body.as_slice())
            .map_err(|_| Error::Unavailable)?;
        if response.status().as_u16() == 202 {
            return Ok(None);
        }
        Self::read_seat(response).map(Some)
    }

    pub(crate) fn ranked_reclaim(&self, bearer: &str, game: Id) -> Result<RankedSeat, Error> {
        let game = Uuid::from_bytes(game);
        let response = self
            .agent
            .post(format!("{}/lobby/games/{game}/seat", self.origin))
            .header("Authorization", &format!("Bearer {bearer}"))
            .header("Content-Type", "application/json")
            .send(b"{}".as_slice())
            .map_err(|_| Error::Unavailable)?;
        Self::read_seat(response)
    }

    fn read_seat(response: ureq::http::Response<ureq::Body>) -> Result<RankedSeat, Error> {
        let seat: RankedSeat = crate::http::decode_response(response)?;
        if seat.seat > 1
            || seat.game_id.get_version_num() != 7
            || seat.game_id.get_variant() != uuid::Variant::RFC4122
            || seat.seat_token.len() != 64
            || !seat
                .seat_token
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::Rejected);
        }
        Ok(seat)
    }
}
