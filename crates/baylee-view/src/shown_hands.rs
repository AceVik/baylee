use crate::objects::HandObject;
use baylee_core::ids::{PlayerId, SeatSet};
use serde::{Deserialize, Serialize};

// --------------------------------------------------------------- shown hands

/// A hand visible through a public rule or explicit teammate sharing (#265).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SharedHand {
    /// Whose hand it is.
    pub player: PlayerId,
    /// Visible to everyone through a rule, rather than voluntary sharing.
    /// Agents retain these hands when they discard teammate-only disclosures.
    #[serde(default)]
    pub public: bool,
    /// The cards, in the owner's hand order, as the owner's own view has
    /// them.
    pub cards: Vec<HandObject>,
}

/// Something a seat says about itself that is not a move in the game
/// (#265).
///
/// It travels in its own envelope (`SeatSettingMsg`), never as a
/// `PlayerAction`: the engine journals every action, its automation settings
/// included, and a setting touches neither the journal nor the snapshot
/// hash. A replay has nobody to show a hand to.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SeatSetting {
    /// Show this seat's hand to exactly these teammates. The whole set, so
    /// that showing, withdrawing and accepting a request are one message,
    /// and sending the set already in force changes nothing.
    ShareHand(SeatSet),
    /// Ask a teammate to show this seat their hand.
    RequestHand(PlayerId),
    /// Turn down a teammate's request. They may ask again from the next
    /// turn on.
    DeclineHand(PlayerId),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn older_sharing_frames_never_become_public_reveals() {
        let hand = SharedHand {
            player: PlayerId::new(1),
            public: true,
            cards: vec![],
        };
        let mut wire = serde_json::to_value(hand).unwrap();
        wire.as_object_mut().unwrap().remove("public");
        let decoded: SharedHand = serde_json::from_value(wire).unwrap();
        assert!(!decoded.public);
    }
}
