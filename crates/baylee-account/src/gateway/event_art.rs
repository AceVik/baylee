//! Account and guest participants use only their current home table's seat.
use super::GatewayHttp;
use crate::{
    Error,
    advent::{self, AssetMeta, Kind, SeatLooks},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Deserialize)]
struct Look {
    #[serde(default)]
    event_sleeve: Option<AssetMeta>,
    #[serde(default)]
    event_playmat: Option<AssetMeta>,
}

fn eligible(game: Uuid, seat_token: &str) -> Result<(), Error> {
    if game.get_version_num() != 7
        || game.get_variant() != uuid::Variant::RFC4122
        || seat_token.len() != 64
        || !seat_token
            .bytes()
            .all(|b| matches!(b,b'0'..=b'9'|b'a'..=b'f'))
    {
        return Err(Error::Rejected);
    }
    Ok(())
}

impl GatewayHttp {
    /// Read only event cosmetics visible to this seat at its ordinary table.
    ///
    /// # Errors
    /// Rejects an expired seat, malformed metadata and oversized responses.
    pub fn advent_table_looks(
        &self,
        game: Uuid,
        seat_token: &str,
    ) -> Result<[SeatLooks; 8], Error> {
        eligible(game, seat_token)?;
        let reply = self
            .agent
            .get(format!("{}/games/{game}/cosmetics", self.origin))
            .header(advent::framing::CAPABILITY_HEADER, "1")
            .header("Authorization", format!("Bearer {seat_token}"))
            .call()
            .map_err(|_| Error::Unavailable)?;
        let raw: BTreeMap<u8, Look> = crate::http::decode_response(reply)?;
        let mut seats = [SeatLooks::default(); 8];
        for (index, look) in raw {
            let dest = seats.get_mut(usize::from(index)).ok_or(Error::Rejected)?;
            for asset in [look.event_sleeve, look.event_playmat]
                .into_iter()
                .flatten()
            {
                asset.validate()?;
            }
            *dest = SeatLooks {
                sleeve: look.event_sleeve,
                playmat: look.event_playmat,
            };
        }
        Ok(seats)
    }

    /// Read the exact visible art through its home gateway with the current game seat.
    ///
    /// # Errors
    /// Refuses stale game/seat authorizations, changed metadata and unbounded ranges.
    pub fn advent_table_asset(
        &self,
        game: Uuid,
        seat_token: &str,
        seat: u8,
        kind: Kind,
        meta: &AssetMeta,
        active: impl Fn() -> bool,
    ) -> Result<Vec<u8>, Error> {
        #[derive(Serialize)]
        struct Page {
            seat: u8,
            kind: Kind,
            offset: u32,
        }
        eligible(game, seat_token)?;
        meta.validate()?;
        if seat >= 8 {
            return Err(Error::Rejected);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(meta.size as usize)
            .map_err(|_| Error::Busy)?;
        let url = format!("{}/games/{game}/event-art", self.origin);
        while bytes.len() < meta.size as usize {
            if !active() {
                return Err(Error::Conflict);
            }
            let offset = u32::try_from(bytes.len()).map_err(|_| Error::Rejected)?;
            let body = central_api::encode(&Page { seat, kind, offset })?;
            let reply = self
                .agent
                .post(&url)
                .header(advent::framing::CAPABILITY_HEADER, "1")
                .header("Authorization", format!("Bearer {seat_token}"))
                .header("Content-Type", "application/json")
                .send(body.as_slice())
                .map_err(|_| Error::Unavailable)?;
            let advent::Response::Asset { page } = crate::http::decode_response(reply)? else {
                return Err(Error::Rejected);
            };
            if page.asset != *meta {
                return Err(Error::Conflict);
            }
            if page.offset != offset
                || page.bytes.len() != (meta.size - offset).min(advent::PAGE_BYTES) as usize
            {
                return Err(Error::Rejected);
            }
            bytes.extend_from_slice(&page.bytes);
        }
        crate::check_advent_image(meta, &bytes)?;
        Ok(bytes)
    }
}
