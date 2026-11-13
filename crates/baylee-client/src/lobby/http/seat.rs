//! Backward-compatible seat metadata from the authenticated gateway response.
use baylee_client_core::lobby::{LobbyEvent, SeatHandover};

pub(super) fn decode(body: &str) -> Option<LobbyEvent> {
    #[derive(serde::Deserialize)]
    struct Grant {
        #[serde(flatten)]
        seat: SeatHandover,
        #[serde(default)]
        challenge: Option<[u8; 16]>,
    }
    let grant: Grant = serde_json::from_str(body).ok()?;
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
