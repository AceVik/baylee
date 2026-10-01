//! What the lobby's deck list says about a deck beyond its name (#254).
//!
//! A stored deck is rows of text ([`crate::deckrow`]). That is not what a
//! player picking between two decks asks, which is how many cards, in which
//! colours, led by whom. These types are that answer, as `GET /decks` sends
//! it and the lobby reads it. They are here, beside the rows they are read
//! from, because both ends name them. The reading itself needs the card
//! registry and is `baylee_cards::digest`.

use crate::preset::Finish;
use serde::{Deserialize, Serialize};

/// One deck, as the lobby's list describes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Digest {
    /// Cards in the deck, counting copies: `4 Llanowar Elves` is four.
    pub copies: u32,
    /// Cards in the sideboard, the same way.
    pub side_copies: u32,
    /// The deck's colour identity as `WUBRG` letters, empty for colourless.
    ///
    /// For a deck with commanders it is their combined colour identity
    /// (CR 903.4), both of a partner pair's together, which is what bounds
    /// every other card in it. For a deck without one it is every main-deck
    /// card's identity together: the colours the deck plays, which is what
    /// a player picking between two decks is looking for.
    pub identity: String,
    /// The commanders, in the deck's order, each with the picture the deck
    /// shows it with.
    pub leaders: Vec<Leader>,
}

/// A commander and the printing to picture it by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Leader {
    /// Its English name, as the deck stores it.
    pub name: String,
    /// The printing the deck's row names, else the one the registry
    /// references. Empty when the reader does not know the card.
    pub scryfall_id: String,
    /// The language of that printing.
    pub lang: String,
    /// The finish the row names; non-foil when it names none.
    pub finish: Finish,
    /// Whether that card has a second picture to turn to.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_back_image: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn leader(back: bool) -> Leader {
        Leader {
            name: "Atraxa".into(),
            scryfall_id: "abc".into(),
            lang: "en".into(),
            finish: Finish::Foil,
            has_back_image: back,
        }
    }

    #[test]
    fn a_leader_without_a_back_picture_omits_the_field() {
        let v = serde_json::to_value(leader(false)).unwrap();
        assert!(v.get("has_back_image").is_none(), "{v}");
        assert_eq!(v["finish"], "Foil");
    }

    #[test]
    fn a_leader_with_a_back_picture_says_so() {
        let v = serde_json::to_value(leader(true)).unwrap();
        assert_eq!(v["has_back_image"], true);
    }

    #[test]
    fn an_old_leader_without_the_field_reads_as_none() {
        let l: Leader = serde_json::from_value(
            json!({"name": "A", "scryfall_id": "", "lang": "en", "finish": "Normal"}),
        )
        .unwrap();
        assert!(!l.has_back_image);
    }

    #[test]
    fn a_digest_round_trips_with_its_leaders_in_order() {
        let d = Digest {
            copies: 100,
            side_copies: 3,
            identity: "WUBG".into(),
            leaders: vec![leader(true), Leader::default()],
        };
        let s = serde_json::to_string(&d).unwrap();
        let back: Digest = serde_json::from_str(&s).unwrap();
        assert_eq!(back, d);
        assert_eq!(back.leaders[0].name, "Atraxa");
    }

    #[test]
    fn the_wire_names_of_a_digest_are_stable() {
        let v = serde_json::to_value(Digest::default()).unwrap();
        let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["copies", "identity", "leaders", "side_copies"]);
    }

    #[test]
    fn a_digest_missing_a_required_field_is_refused() {
        let missing = json!({"copies": 1, "side_copies": 0, "identity": ""});
        assert!(serde_json::from_value::<Digest>(missing).is_err());
    }

    #[test]
    fn a_digest_with_a_wrong_typed_field_is_refused() {
        for bad in [
            json!({"copies": -1, "side_copies": 0, "identity": "", "leaders": []}),
            json!({"copies": "4", "side_copies": 0, "identity": "", "leaders": []}),
            json!({"copies": 1, "side_copies": 0, "identity": "", "leaders": {}}),
        ] {
            assert!(
                serde_json::from_value::<Digest>(bad.clone()).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn an_unknown_finish_is_refused() {
        let bad = json!({"name": "A", "scryfall_id": "", "lang": "en", "finish": "Sparkly"});
        assert!(serde_json::from_value::<Leader>(bad).is_err());
    }
}
