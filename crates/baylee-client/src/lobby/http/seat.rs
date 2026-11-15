//! Backward-compatible seat metadata from the authenticated gateway response.
use baylee_client_core::lobby::{LobbyEvent, SeatHandover};

pub(super) fn decode(body: &str) -> Option<LobbyEvent> {
    #[derive(serde::Deserialize)]
    struct Grant {
        #[serde(flatten)]
        seat: SeatHandover,
        #[serde(default)]
        challenge: Option<[u8; 16]>,
        #[serde(default)]
        tower: Option<[u8; 16]>,
    }
    let grant: Grant = serde_json::from_str(body).ok()?;
    if let Some(run) = grant.tower {
        let id = uuid::Uuid::from_bytes(run);
        let game = uuid::Uuid::parse_str(&grant.seat.game_id).ok()?;
        if grant.challenge.is_some()
            || id.get_version_num() != 7
            || id.get_variant() != uuid::Variant::RFC4122
            || game.get_version_num() != 7
            || game.get_variant() != uuid::Variant::RFC4122
            || game.to_string() != grant.seat.game_id
            || grant.seat.seat != 0
            || grant.seat.local
        {
            return None;
        }
        return Some(LobbyEvent::TowerSeated {
            seat: grant.seat,
            run,
        });
    }
    Some(match grant.challenge {
        Some(challenge) => {
            let id = uuid::Uuid::from_bytes(challenge);
            if id.get_version_num() != 7 || id.get_variant() != uuid::Variant::RFC4122 {
                return None;
            }
            LobbyEvent::ChallengeSeated {
                seat: grant.seat,
                challenge,
            }
        }
        None => LobbyEvent::Seated(grant.seat),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tower_context_is_distinct_bounded_and_only_for_its_human_seat() {
        let run = *uuid::Uuid::now_v7().as_bytes();
        let mut grant = serde_json::json!({ "tower":run, "game_id":uuid::Uuid::now_v7().to_string(), "seat":0, "seat_token":"synthetic-only" });
        let event = decode(&grant.to_string()).unwrap();
        assert!(matches!(event, LobbyEvent::TowerSeated { run: id, .. } if id == run));
        assert_eq!(format!("{event:?}"), "TowerSeated { .. }");
        grant["seat"] = 1.into();
        assert!(decode(&grant.to_string()).is_none());
        grant["seat"] = 0.into();
        grant["challenge"] = serde_json::json!(run);
        assert!(decode(&grant.to_string()).is_none());
        grant.as_object_mut().unwrap().remove("challenge");
        grant["tower"] = serde_json::to_value([0_u8; 16]).unwrap();
        assert!(decode(&grant.to_string()).is_none());
        grant.as_object_mut().unwrap().remove("tower");
        assert!(matches!(
            decode(&grant.to_string()),
            Some(LobbyEvent::Seated(_))
        ));
    }
}
