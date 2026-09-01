//! The Scryfall side of the catalog: the bulk feed and the single-card API.
//!
//! Only the fields a client actually renders are modelled. Scryfall's card
//! object has well over a hundred; carrying them all would make every schema
//! change a Scryfall change, and none of the rest reaches a player.

use serde::{Deserialize, Serialize};

/// Base URL of the Scryfall API.
pub const API: &str = "https://api.scryfall.com";

/// One card printing as the catalog stores it.
///
/// A printing is language-specific: the German Forest and the English Forest
/// are two rows sharing an `oracle_id`. That is Scryfall's own model and it is
/// what makes "the same card in my language" a plain indexed lookup.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Card {
    /// Printing UUID (primary key).
    pub id: String,
    /// Rules identity, shared by every printing and language.
    pub oracle_id: Option<String>,
    /// Two-letter language code of this printing.
    #[serde(default = "english")]
    pub lang: String,
    /// Set code.
    #[serde(default)]
    pub set: String,
    /// Collector number within the set.
    #[serde(default)]
    pub collector_number: String,
    /// Rarity.
    pub rarity: Option<String>,
    /// Layout (`normal`, `modal_dfc`, `split`, …).
    pub layout: Option<String>,
    /// Release date (ISO-8601; sorts correctly as text).
    pub released_at: Option<String>,

    // ---- what distinguishes one printing from another ------------------
    /// Full set name, for a picker that shows more than a three-letter code.
    pub set_name: Option<String>,
    /// Illustrator of the front face.
    pub artist: Option<String>,
    /// Which finishes this printing was actually sold in — `nonfoil`, `foil`,
    /// `etched`. A deck row may only name a finish the printing *has*, so this
    /// is the one field that makes the picker's finish buttons truthful
    /// rather than decorative.
    #[serde(default)]
    pub finishes: Vec<String>,
    /// Frame treatments (`showcase`, `extendedart`, `etched`, …). What makes
    /// two printings from the same set look different at a glance.
    #[serde(default)]
    pub frame_effects: Vec<String>,
    /// `black`, `white`, `borderless`, `silver`, `gold`.
    pub border_color: Option<String>,
    /// Whether this printing is a promo.
    #[serde(default)]
    pub promo: bool,

    // ---- single-face fields (absent on multi-face layouts) --------------
    /// English name.
    #[serde(default)]
    pub name: String,
    /// Name as printed in this printing's language.
    pub printed_name: Option<String>,
    /// English type line.
    pub type_line: Option<String>,
    /// Type line as printed.
    pub printed_type_line: Option<String>,
    /// English rules text.
    pub oracle_text: Option<String>,
    /// Rules text as printed.
    pub printed_text: Option<String>,
    /// Mana cost in Scryfall notation.
    pub mana_cost: Option<String>,
    /// Power (a string: it may be `*` or `1+*`).
    pub power: Option<String>,
    /// Toughness.
    pub toughness: Option<String>,
    /// Starting loyalty.
    pub loyalty: Option<String>,

    /// Faces, for multi-face layouts.
    pub card_faces: Option<Vec<Face>>,
}

/// Default language when a record omits it.
fn english() -> String {
    "en".to_string()
}

/// One face of a multi-face card.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Face {
    /// English name of this face.
    #[serde(default)]
    pub name: String,
    /// Name as printed.
    pub printed_name: Option<String>,
    /// English type line.
    pub type_line: Option<String>,
    /// Type line as printed.
    pub printed_type_line: Option<String>,
    /// English rules text.
    pub oracle_text: Option<String>,
    /// Rules text as printed.
    pub printed_text: Option<String>,
    /// Mana cost of this face.
    pub mana_cost: Option<String>,
    /// Power.
    pub power: Option<String>,
    /// Toughness.
    pub toughness: Option<String>,
    /// Loyalty.
    pub loyalty: Option<String>,
}

impl Card {
    /// The card's faces, normalised so single-face and multi-face cards are
    /// handled by one code path.
    #[must_use]
    pub fn faces(&self) -> Vec<Face> {
        if let Some(faces) = &self.card_faces
            && !faces.is_empty()
        {
            return faces.clone();
        }
        vec![Face {
            name: self.name.clone(),
            printed_name: self.printed_name.clone(),
            type_line: self.type_line.clone(),
            printed_type_line: self.printed_type_line.clone(),
            oracle_text: self.oracle_text.clone(),
            printed_text: self.printed_text.clone(),
            mana_cost: self.mana_cost.clone(),
            power: self.power.clone(),
            toughness: self.toughness.clone(),
            loyalty: self.loyalty.clone(),
        }]
    }

    /// The finishes this printing was sold in, never empty.
    ///
    /// Scryfall omits the field on some older records rather than writing
    /// `["nonfoil"]`, and an empty list here would reach the picker as a card
    /// that cannot be added in any finish at all. Every printing exists in at
    /// least the plain one, so that is the floor.
    #[must_use]
    pub fn finish_list(&self) -> Vec<String> {
        if self.finishes.is_empty() {
            return vec!["nonfoil".to_string()];
        }
        self.finishes.clone()
    }

    /// Whether the record is usable: without an id and an oracle id it can be
    /// neither stored nor found again.
    ///
    /// Scryfall's bulk feed also contains tokens, art series and memorabilia,
    /// which have no oracle identity and no rules text worth caching.
    #[must_use]
    pub fn is_storable(&self) -> bool {
        !self.id.is_empty() && self.oracle_id.is_some()
    }
}

/// An entry in Scryfall's bulk-data catalog.
#[derive(Debug, Clone, Deserialize)]
pub struct BulkEntry {
    /// Which feed (`default_cards`, `all_cards`, …).
    #[serde(rename = "type")]
    pub kind: String,
    /// Gzipped JSONL download.
    pub jsonl_download_uri: String,
    /// Compressed size in bytes, for progress reporting.
    #[serde(default)]
    pub compressed_size: u64,
}

/// The bulk-data catalog response.
#[derive(Debug, Clone, Deserialize)]
pub struct BulkList {
    /// The available feeds.
    pub data: Vec<BulkEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_face_card_still_reports_one_face() {
        let card = Card {
            id: "x".into(),
            name: "Brainstorm".into(),
            mana_cost: Some("{U}".into()),
            oracle_text: Some("Draw three cards…".into()),
            ..Card::default()
        };
        let faces = card.faces();
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].name, "Brainstorm");
        assert_eq!(faces[0].mana_cost.as_deref(), Some("{U}"));
    }

    #[test]
    fn a_multi_face_card_reports_its_own_faces() {
        let card = Card {
            id: "y".into(),
            name: "A // B".into(),
            card_faces: Some(vec![
                Face {
                    name: "A".into(),
                    ..Face::default()
                },
                Face {
                    name: "B".into(),
                    ..Face::default()
                },
            ]),
            ..Card::default()
        };
        assert_eq!(card.faces().len(), 2);
    }

    /// The bulk feed carries a lot that is not a playable card; storing it
    /// would bloat the search index with entries nothing can ever reference.
    #[test]
    fn records_without_an_oracle_identity_are_skipped() {
        let token = Card {
            id: "z".into(),
            oracle_id: None,
            ..Card::default()
        };
        assert!(!token.is_storable());
    }

    /// The reduced model has to survive a real Scryfall record, including the
    /// hundred fields it does not mention.
    #[test]
    fn a_localized_record_decodes_with_its_printed_fields() {
        let json = r#"{
            "id": "abc", "oracle_id": "def", "lang": "de",
            "set": "m21", "collector_number": "12", "rarity": "common",
            "layout": "normal", "released_at": "2020-07-03",
            "name": "Forest", "printed_name": "Wald",
            "type_line": "Basic Land — Forest",
            "printed_type_line": "Basisland — Wald",
            "oracle_text": "({T}: Add {G}.)",
            "printed_text": "({T}: Erzeuge {G}.)",
            "some_field_we_do_not_model": 42
        }"#;
        let card: Card = serde_json::from_str(json).expect("decodes");
        assert_eq!(card.lang, "de");
        assert_eq!(card.printed_name.as_deref(), Some("Wald"));
        assert!(card.is_storable());
    }
}
