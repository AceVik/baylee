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
    /// What kind of set that is — `core`, `expansion`, `commander`, `box`,
    /// `funny`, `memorabilia`, `token`, `alchemy`, …
    ///
    /// Stored because it is the one field that separates a card from a thing
    /// shaped like one, and it says so in Scryfall's own words rather than in
    /// a list of set codes this repo would have to keep. A Secret Lair is
    /// `box` and its cards are cards; Mystery Booster playtest cards and the
    /// Un-sets are `funny`; art cards and challenge-deck cards are
    /// `memorabilia`. Which of those the card corpus admits is decided where
    /// the corpus is built, not here.
    pub set_type: Option<String>,
    /// Collector number within the set.
    #[serde(default)]
    pub collector_number: String,
    /// Rarity.
    pub rarity: Option<String>,
    /// Layout (`normal`, `modal_dfc`, `split`, …).
    pub layout: Option<String>,
    /// Release date, ISO-8601. Stored as a `date`, so it is parsed on the
    /// way in rather than trusted to sort as text.
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
    /// Whether this printing exists only inside a client.
    #[serde(default)]
    pub digital: bool,
    /// Which games this printing was published in — `paper`, `arena`, `mtgo`.
    ///
    /// Stored beside `digital` rather than instead of it, because the two say
    /// different things and the useful question is about neither on its own.
    /// "Digital-only *card*" is not a property of a printing at all: it is
    /// *no printing of this card lists `paper`*, which only the whole set of
    /// a card's printings can answer.
    #[serde(default)]
    pub games: Vec<String>,
    /// Format to legality (`legal`, `banned`, `restricted`, `not_legal`).
    ///
    /// Only Vintage's is stored, and only one thing is asked of it: whether
    /// the card is in a format's pool **at all**. `not_legal` means no format
    /// has ever heard of it, which is what separates an Un-set's acorn cards
    /// from the tournament-legal cards printed in the same joke set — a line
    /// neither `set_type` nor the border colour draws, because Unfinity
    /// prints both, black-bordered, side by side. `banned` is a real card.
    #[serde(default)]
    pub legalities: std::collections::BTreeMap<String, String>,

    // ---- single-face fields (absent on multi-face layouts) --------------
    /// English name.
    #[serde(default)]
    pub name: String,
    /// Name as printed in this printing's language.
    pub printed_name: Option<String>,
    /// The just-for-fun name a printing carries instead of the card's own.
    ///
    /// Neither a translation nor a name in the rules sense: `name` stays the
    /// Oracle name and `oracle_id` is unchanged, and the physical card prints
    /// the real name in small type beside it. A Secret Lair Abrade is sold as
    /// "You're Gonna Need a Bigger Boat" and *is* Abrade. It belongs to the
    /// **printing** rather than the card — Command Tower has six of them — so
    /// it reaches search and never the rules.
    pub flavor_name: Option<String>,
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
    /// The oracle card this face is — present only where the printing
    /// itself has no id. See [`Card::oracle_identity`].
    pub oracle_id: Option<String>,
    /// Name as printed.
    pub printed_name: Option<String>,
    /// This face's just-for-fun name; see [`Card::flavor_name`].
    ///
    /// A face carries its own because the two halves of a reversible card can
    /// disagree: one printing of Birds of Paradise is "African Swallow" on the
    /// front and "European Swallow" on the back.
    pub flavor_name: Option<String>,
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
            // A flavor name sits on the faces of a reversible card and on the
            // card itself elsewhere, so a face that has none inherits the
            // card's rather than dropping it.
            let mut out: Vec<Face> = faces
                .iter()
                .map(|f| Face {
                    flavor_name: f.flavor_name.clone().or_else(|| self.flavor_name.clone()),
                    ..f.clone()
                })
                .collect();
            if self.faces_are_one_card() {
                // One row, because `card_search` is keyed on
                // `(oracle_id, face_index)`: a second row for a face that
                // exists once in the rules is the same card answering a
                // search twice, and it would sit beside the face 0 the
                // ordinary printings of that card already wrote.
                //
                // The flavor name is carried across rather than dropped with
                // the row. Of the six reversible printings that have one,
                // four put it on one face only (Kardur, Teferi's Ageless
                // Insight) or say the same thing twice (the three
                // Transformers Colossi); exactly one prints two different
                // names — Birds of Paradise SLD 1675, "African Swallow" and
                // "European Swallow" — and that one keeps the first, which is
                // the whole cost of the collapse.
                let flavor = out.iter().find_map(|f| f.flavor_name.clone());
                out.truncate(1);
                out[0].flavor_name = flavor;
            }
            return out;
        }
        vec![Face {
            name: self.name.clone(),
            // Spelled out rather than defaulted: a card with one face keeps
            // its identity at the top level, and `oracle_identity` reads it
            // from there. A face-level id here would be the reversible shape
            // with one side, which does not exist.
            oracle_id: None,
            printed_name: self.printed_name.clone(),
            flavor_name: self.flavor_name.clone(),
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

    /// Vintage's verdict on this card, `not_legal` when nothing says
    /// otherwise.
    ///
    /// Vintage because it is the widest constructed pool there is: a card no
    /// format admits is `not_legal` here too, and nothing else has to be
    /// asked. What it is *not* is a playability test — planes, schemes and
    /// Vanguard avatars are `not_legal` and are perfectly real cards, which
    /// is why the corpus only consults it inside a joke set.
    #[must_use]
    pub fn vintage_legality(&self) -> &str {
        self.legalities
            .get("vintage")
            .map_or("not_legal", String::as_str)
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

    /// The oracle card this printing is, wherever Scryfall put the id.
    ///
    /// Ordinarily it is a top-level field. A **reversible card** — one piece
    /// of cardboard with the same card printed on both sides, in two
    /// treatments — has none at all, and each face carries it instead. Asking
    /// only the top level dropped every one of them in silence: `select
    /// count(*) from cards where layout='reversible_card'` answered 0 against
    /// Scryfall's 82 English printings, which is how `Hallowed Fountain (ECL)
    /// 347` came to be a deck line the catalog could not resolve — five
    /// consecutive collector numbers missing from `ecl`, all five of them
    /// shocklands in the borderless treatment (#46).
    #[must_use]
    pub fn oracle_identity(&self) -> Option<&str> {
        self.oracle_id.as_deref().or_else(|| self.face_oracle_id())
    }

    /// The oracle id **every** face agrees on.
    ///
    /// Agreement is the whole test. Faces naming different oracle cards would
    /// be a shape nobody here has decided about, and answering `None` leaves
    /// such a record exactly as unstorable as it is today rather than picking
    /// one of the two ids and calling it the card.
    fn face_oracle_id(&self) -> Option<&str> {
        let faces = self.card_faces.as_ref()?;
        let first = faces.first()?.oracle_id.as_deref()?;
        faces
            .iter()
            .all(|f| f.oracle_id.as_deref() == Some(first))
            .then_some(first)
    }

    /// Whether this printing's faces are two pictures of **one face** rather
    /// than the two faces of one card.
    ///
    /// Two questions, and the second is not optional. A shared `oracle_id`
    /// says the faces belong to one card, which a reversible printing of an
    /// *adventure* also does: `tdm` 381 is one piece of cardboard carrying
    /// `Bloomvine Regent` on one side and its omen half `Claim Territory` on
    /// the other, both under one id and both real faces in the rules. So the
    /// **printed name** decides: same name on every face and it is one face
    /// photographed twice, different names and it is a card with two of them.
    ///
    /// Getting that wrong would have reached the ledger. `card_corpus` picks
    /// a card's first printing with `DISTINCT ON (oracle_id) ORDER BY
    /// released_at, …, collector_number`, and `collector_number` is text —
    /// `'378' < '51'` — so the borderless printing *is* the one it picks for
    /// Marang River Regent and Scavenger Regent. Collapsing those to one face
    /// would have renamed two ledger rows from `Marang River Regent // Coil
    /// and Catch` to `Marang River Regent`.
    ///
    /// Read off the shape and never off `layout`: a transforming card keeps
    /// its id at the top level, so it is untouched by this and keeps both
    /// rows whatever its faces are called.
    #[must_use]
    pub fn faces_are_one_card(&self) -> bool {
        if self.oracle_id.is_some() || self.face_oracle_id().is_none() {
            return false;
        }
        let Some(faces) = self.card_faces.as_ref() else {
            return false;
        };
        let Some(first) = faces.first() else {
            return false;
        };
        faces.iter().all(|f| f.name == first.name)
    }

    /// Whether the record is usable: without an id and an oracle id —
    /// wherever Scryfall put it, see [`Card::oracle_identity`] — it can be
    /// neither stored nor found again.
    ///
    /// Scryfall's bulk feed also contains tokens, art series and memorabilia,
    /// which have no oracle identity and no rules text worth caching.
    #[must_use]
    pub fn is_storable(&self) -> bool {
        !self.id.is_empty() && self.oracle_identity().is_some()
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

    /// A flavor name is the printing's, not the card's, and Scryfall puts it
    /// in two different places: on the card for an ordinary printing, on each
    /// face for a reversible one whose halves disagree. Both have to reach
    /// `faces()`, or the search loses the name a player is holding — and a
    /// reversible printing is **one** face, so the name on its second half has
    /// to be carried onto the row that is kept.
    #[test]
    fn a_flavor_name_reaches_the_face_from_wherever_scryfall_put_it() {
        let single: Card = serde_json::from_str(
            r#"{"id":"a","oracle_id":"b","lang":"en","set":"sld",
                "collector_number":"1","name":"Abrade",
                "flavor_name":"You're Gonna Need a Bigger Boat",
                "type_line":"Instant"}"#,
        )
        .expect("decodes");
        assert_eq!(
            single.faces()[0].flavor_name.as_deref(),
            Some("You're Gonna Need a Bigger Boat"),
            "a card-level flavor name has to reach the one face"
        );
        assert_eq!(single.name, "Abrade", "the Oracle name is untouched");

        // The real shape: no top-level `oracle_id`, one on each face, and the
        // same one. SLD 1675 is the single printing in the world whose two
        // halves print different names.
        let reversible: Card = serde_json::from_str(
            r#"{"id":"c","lang":"en","set":"sld",
                "collector_number":"1675","name":"Birds of Paradise // Birds of Paradise",
                "layout":"reversible_card",
                "card_faces":[
                  {"name":"Birds of Paradise","oracle_id":"d",
                   "flavor_name":"African Swallow"},
                  {"name":"Birds of Paradise","oracle_id":"d",
                   "flavor_name":"European Swallow"}]}"#,
        )
        .expect("decodes");
        let faces = reversible.faces();
        assert_eq!(faces.len(), 1, "one card, one row: {faces:?}");
        assert_eq!(faces[0].flavor_name.as_deref(), Some("African Swallow"));

        // The four that print a name on one half only keep it, which is the
        // case the collapse would lose if it read face 0 and stopped.
        let one_sided: Card = serde_json::from_str(
            r#"{"id":"g","lang":"en","set":"sld",
                "collector_number":"1807","name":"Kardur // Kardur",
                "layout":"reversible_card",
                "card_faces":[
                  {"name":"Kardur","oracle_id":"h"},
                  {"name":"Kardur","oracle_id":"h","flavor_name":"Chucky"}]}"#,
        )
        .expect("decodes");
        assert_eq!(
            one_sided.faces()[0].flavor_name.as_deref(),
            Some("Chucky"),
            "a name printed on the second half only was dropped with its row"
        );

        // A multi-face card whose faces carry none inherits the card's.
        let inherited: Card = serde_json::from_str(
            r#"{"id":"e","oracle_id":"f","lang":"en","set":"sld",
                "collector_number":"3","name":"A // B","layout":"split",
                "flavor_name":"Megatron",
                "card_faces":[{"name":"A"},{"name":"B"}]}"#,
        )
        .expect("decodes");
        assert!(
            inherited
                .faces()
                .iter()
                .all(|f| f.flavor_name.as_deref() == Some("Megatron")),
            "a face with no flavor name of its own takes the card's"
        );
    }

    /// A reversible card is one piece of cardboard with the same card on both
    /// sides, and its oracle id lives on the faces. Reading only the top level
    /// dropped all 82 English printings of the shape, which is a deck line
    /// that resolves to nothing rather than to the wrong card (#46).
    #[test]
    fn a_reversible_printing_is_stored_under_the_id_its_faces_carry() {
        let shock: Card = serde_json::from_str(
            r#"{"id":"cc0c1b0e-0000-4000-8000-000000000001","lang":"en",
                "set":"ecl","collector_number":"347",
                "name":"Hallowed Fountain // Hallowed Fountain",
                "layout":"reversible_card",
                "card_faces":[
                  {"name":"Hallowed Fountain",
                   "oracle_id":"f1750962-a87c-49f6-b731-02ae971ac6ea",
                   "type_line":"Land — Plains Island"},
                  {"name":"Hallowed Fountain",
                   "oracle_id":"f1750962-a87c-49f6-b731-02ae971ac6ea",
                   "type_line":"Land — Plains Island"}]}"#,
        )
        .expect("decodes");
        assert!(shock.is_storable(), "the printing was dropped");
        assert_eq!(
            shock.oracle_identity(),
            Some("f1750962-a87c-49f6-b731-02ae971ac6ea")
        );
        let faces = shock.faces();
        assert_eq!(faces.len(), 1, "two rows for one card: {faces:?}");
        assert_eq!(faces[0].type_line.as_deref(), Some("Land — Plains Island"));
    }

    /// The counter-test, and the reason agreement is the test rather than
    /// "the first face wins": faces naming two different oracle cards are a
    /// shape nobody has decided about, and it stays out rather than being
    /// stored as one of them.
    #[test]
    fn faces_that_name_two_different_cards_are_still_refused() {
        let odd: Card = serde_json::from_str(
            r#"{"id":"x","lang":"en","set":"zzz","collector_number":"1",
                "name":"A // B",
                "card_faces":[{"name":"A","oracle_id":"one"},
                              {"name":"B","oracle_id":"two"}]}"#,
        )
        .expect("decodes");
        assert_eq!(odd.oracle_identity(), None);
        assert!(!odd.is_storable());
        assert!(!odd.faces_are_one_card());

        // And a transforming card keeps both of its rows: its id is at the
        // top level, so its faces are two objects in the rules.
        let dfc: Card = serde_json::from_str(
            r#"{"id":"y","oracle_id":"z","lang":"en","set":"mid",
                "collector_number":"1","name":"Front // Back",
                "layout":"transform",
                "card_faces":[{"name":"Front"},{"name":"Back"}]}"#,
        )
        .expect("decodes");
        assert!(!dfc.faces_are_one_card());
        assert_eq!(dfc.faces().len(), 2);
    }

    /// The case a shared `oracle_id` alone gets wrong. `tdm` 381 is one piece
    /// of cardboard with `Bloomvine Regent` on one side and its omen half
    /// `Claim Territory` on the other: no top-level id, both faces carrying
    /// the same one, and **two** faces in the rules. Six printings have this
    /// shape and all six are in Tarkir: Dragonstorm.
    #[test]
    fn a_reversible_printing_of_a_card_with_two_faces_keeps_both() {
        let omen: Card = serde_json::from_str(
            r#"{"id":"n","lang":"en","set":"tdm","collector_number":"381",
                "name":"Bloomvine Regent // Claim Territory // Bloomvine Regent",
                "layout":"reversible_card",
                "card_faces":[
                  {"name":"Bloomvine Regent",
                   "oracle_id":"da1e019c-2ffb-412d-90d7-f2e5e5c44c4b",
                   "type_line":"Creature — Dragon"},
                  {"name":"Claim Territory",
                   "oracle_id":"da1e019c-2ffb-412d-90d7-f2e5e5c44c4b",
                   "type_line":"Sorcery — Omen"}]}"#,
        )
        .expect("decodes");
        assert!(omen.is_storable(), "the printing was dropped");
        assert!(
            !omen.faces_are_one_card(),
            "two different faces were read as one photographed twice"
        );
        let faces = omen.faces();
        assert_eq!(faces.len(), 2, "the omen half went missing: {faces:?}");
        assert_eq!(faces[1].name, "Claim Territory");
    }
}
