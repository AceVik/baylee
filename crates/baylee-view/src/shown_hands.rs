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
    /// Continuously show this seat's hand to exactly these teammates until the
    /// end of the game (or until withdrawn). The whole set, so that showing,
    /// withdrawing and accepting a request are one message, and sending the set
    /// already in force changes nothing. Omitting a teammate from `ShareHand`
    /// also clears any one-time snapshot previously shown to them.
    ShareHand(SeatSet),
    /// Ask a teammate to show this seat their hand.
    RequestHand(PlayerId),
    /// Turn down a teammate's request (and clear any snapshot shown to them).
    /// They may ask again from the next turn on.
    DeclineHand(PlayerId),
    /// Show a one-time snapshot of the cards currently in this seat's hand to
    /// these teammates. Those exact cards remain visible while they stay in
    /// this seat's hand, whereas cards drawn afterwards remain hidden unless
    /// shared again. Sending an empty set clears all one-time snapshots from
    /// this seat.
    ///
    /// Last, and it stays last: the wire names a variant by its place, so a
    /// variant put between the others would make an older client's
    /// `RequestHand` read as this.
    ShareHandOnce(SeatSet),
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

    /// The wire names a setting by its place. The settings an older client
    /// sends keep theirs, so a new one can only come after them.
    #[test]
    fn every_setting_keeps_its_place_on_the_wire() {
        let seat = PlayerId::new(1);
        let set: SeatSet = [seat].into_iter().collect();
        let places = [
            SeatSetting::ShareHand(set),
            SeatSetting::RequestHand(seat),
            SeatSetting::DeclineHand(seat),
            SeatSetting::ShareHandOnce(set),
        ]
        .map(|setting| baylee_core::wire::encode(&setting)[0]);
        assert_eq!(places, [0, 1, 2, 3]);
    }
}
