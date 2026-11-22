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
    pub(crate) fn ranked_admit(
        &self,
        ticket: &crate::SignedToken,
    ) -> Result<(zeroize::Zeroizing<String>, bool, std::time::Duration), Error> {
        let mut admission = self
            .present("/auth/ranked/login", ticket)
            .map_err(|e| e.error)?;
        Ok((
            zeroize::Zeroizing::new(std::mem::take(&mut admission.token)),
            admission.terms_stale == Some(true),
            std::time::Duration::from_secs(admission.renew_in.unwrap_or(300).clamp(1, 1800)),
        ))
    }

    pub(crate) fn ranked_play(
        &self,
        bearer: &str,
        id: Id,
        deck: &RankedDeck,
    ) -> Result<Option<RankedSeat>, Error> {
        let body = central_api::encode(deck)?;
        self.ranked_exchange(bearer, id, &body)
    }

    pub(crate) fn ranked_reclaim(
        &self,
        bearer: &str,
        id: Id,
        game: Id,
    ) -> Result<Option<RankedSeat>, Error> {
        #[derive(Serialize)]
        struct Resume {
            game: Uuid,
        }
        let body = central_api::encode(&Resume {
            game: Uuid::from_bytes(game),
        })?;
        self.ranked_exchange(bearer, id, &body)
    }

    fn ranked_exchange(
        &self,
        bearer: &str,
        id: Id,
        body: &[u8],
    ) -> Result<Option<RankedSeat>, Error> {
        let id = Uuid::from_bytes(id);
        let response = self
            .agent
            .post(format!("{}/ranked/{id}/play", self.origin))
            .header("Authorization", &format!("Bearer {bearer}"))
            .header("Content-Type", "application/json")
            .send(body)
            .map_err(|_| Error::Unavailable)?;
        if response.status().as_u16() == 202 {
            return Ok(None);
        }
        Self::read_seat(response).map(Some)
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
