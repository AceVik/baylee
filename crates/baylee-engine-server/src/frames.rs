//! The envelopes this process builds for the gateway.

use super::{Envelope, v1};

/// Wraps a player-facing envelope for the seat it belongs to.
#[must_use]
pub fn seat_frame(seat: u8, envelope: &Envelope) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
            seat: u32::from(seat),
            envelope: prost::Message::encode_to_vec(envelope).into(),
        })),
    }
}

/// The table is open (#256).
#[must_use]
pub fn curtain() -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::Curtain(v1::Curtain {})),
    }
}

/// An error a seat may be shown.
pub(crate) fn error(message: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::Error(v1::Error {
            code: 1,
            message: message.to_string(),
        })),
    }
}

/// A `GameEnded` that says why.
pub(crate) fn ended(game_id: &str, reason: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::GameEnded(v1::GameEnded {
            game_id: game_id.to_string(),
            winners: Vec::new(),
            reason: reason.to_string(),
        })),
    }
}
