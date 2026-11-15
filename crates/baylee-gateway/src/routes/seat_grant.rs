use crate::Serialize;

/// A chair handed to the caller: the game it is at, its number, and the
/// seat token that opens it.
///
/// @description The caller's chair: the game it is at, its seat, and the
/// seat token that proves it.
/// @openapi
#[derive(Serialize)]
pub(crate) struct SeatGrant {
    /// @description Optional public task UUID, as sixteen bytes. UI context for challenge resumption; never a scoring credential.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) challenge: Option<[u8; 16]>,
    // In key order: the bytes the `json!` it replaced wrote (serde_json sorts keys).
    /// @description The game the chair is at (for a rematch, the new room's
    /// id, not the one in the path).
    pub(crate) game_id: String,
    /// @description The chair's seat number, counted from 0.
    pub(crate) seat: usize,
    /// @description The chair's secret, sent as `Authorization: Bearer` to
    /// buy a seat socket's ticket (`POST /ws-ticket`) and to read the
    /// table's cosmetics. Handed out once; a new one
    /// (`POST /lobby/games/{id}/seat`) ends the old.
    pub(crate) seat_token: String,
}

impl SeatGrant {
    pub(crate) fn ordinary(game_id: String, seat: usize, seat_token: String) -> Self {
        Self {
            challenge: None,
            game_id,
            seat,
            seat_token,
        }
    }
}
