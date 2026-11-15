//! Only the account's fresh Tower assignment selects this private gateway operation.
use super::{Error, GatewayHttp};
use uuid::Uuid;

/// The human capability returned by a hosted Tower execution. Never Debug-printed.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TowerSeat {
    /// Current execution transport identity, not the reservation UUID.
    pub game_id: Uuid,
    /// Human chair; the private Expert chair is never accepted here.
    pub seat: u32,
    /// Gateway-local seat capability, wiped on drop.
    #[serde(deserialize_with = "super::seat_secret")]
    pub seat_token: zeroize::Zeroizing<String>,
}

impl TowerSeat {
    fn validate(&self) -> Result<(), Error> {
        if self.seat != 0
            || self.game_id.get_version_num() != 7
            || self.game_id.get_variant() != uuid::Variant::RFC4122
            || self.seat_token.len() != 64
            || !self
                .seat_token
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}

impl GatewayHttp {
    pub(crate) fn tower_play(&self, bearer: &str, run: Uuid) -> Result<TowerSeat, Error> {
        if run.get_version_num() != 7 || run.get_variant() != uuid::Variant::RFC4122 {
            return Err(Error::Rejected);
        }
        let response = self
            .agent
            .post(format!("{}/tower/{run}/play", self.origin))
            .header("Content-Type", "application/json")
            .header("Authorization", &format!("Bearer {bearer}"))
            .send(b"{}".as_slice())
            .map_err(|_| Error::Unavailable)?;
        if let Some(error) = crate::http::status_error(response.status().as_u16()) {
            return Err(error);
        }
        let seat: TowerSeat = crate::http::decode_response(response)?;
        seat.validate()?;
        Ok(seat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tower_capability_accepts_only_canonical_human_execution_seats() {
        let valid =
            serde_json::json!({"game_id": Uuid::now_v7(), "seat": 0, "seat_token": "a".repeat(64)});
        assert!(
            serde_json::from_value::<TowerSeat>(valid.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for (field, value) in [
            ("game_id", serde_json::json!(Uuid::nil())),
            ("seat", serde_json::json!(1)),
            ("seat_token", serde_json::json!("A".repeat(64))),
            ("seat_token", serde_json::json!("a".repeat(63))),
        ] {
            let mut changed = valid.clone();
            changed[field] = value;
            assert!(
                serde_json::from_value::<TowerSeat>(changed)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        let mut extended = valid;
        extended["gateway"] = "https://untrusted.invalid".into();
        assert!(serde_json::from_value::<TowerSeat>(extended).is_err());
    }
}
