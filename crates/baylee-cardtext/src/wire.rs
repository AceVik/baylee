//! What the gateway's `/catalog/text` answers, as one type for both ends.

use serde::{Deserialize, Serialize};

/// Every face of one card, in one language, read from one printing.
///
/// It used to be written twice — once in the catalog, once in the client,
/// which cannot link the catalog's ORM and Postgres driver and has to build
/// for wasm — with a test on each side pinning the JSON so the copies could
/// not drift. This crate links neither, so there is one copy and one pin.
///
/// The fields a gateway added later carry `#[serde(default)]`, so a client
/// reads an older gateway's answer, and an older client, which does not
/// refuse unknown fields, reads a newer one.
///
/// @description Every face of one card, in one language, read from one
/// printing.
/// @openapi
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct CardTextEntry {
    /// @description Asked for by card (`oracle_ids`): the Scryfall id of
    /// the printing the faces are read from. Asked for by printing (`ids`):
    /// the printing id that was asked for, which need not be that one.
    /// @internal
    /// Asked for by card (`oracle_ids=`): the printing every face below is
    /// read from. Asked for by printing (`ids=`): the printing id that was
    /// asked for, which need not be that one — [`Self::lang`],
    /// [`Self::layout`] and the faces are still the served printing's.
    pub scryfall_id: String,
    /// @description The card's Scryfall oracle id.
    /// @internal
    /// The card: Scryfall's oracle id. Empty from a gateway that predates
    /// `oracle_ids=`.
    #[serde(default)]
    pub oracle_id: String,
    /// @description The language code of the printing the names and type
    /// lines are read from: the one asked for, or `en` where the catalog
    /// has no printing in it. Whether a face's rules text is in it is that
    /// face's `printed`.
    /// @internal
    /// The language of the printing the names and type lines are read from.
    /// Whether a face's rules text is in it is [`FaceText::printed`].
    pub lang: String,
    /// @description That printing's Scryfall layout (`normal`,
    /// `modal_dfc`, …).
    /// @internal
    /// That printing's Scryfall `layout`, which [`align`](crate::align)
    /// reads: a modal double-faced card's hint lines are removed only there.
    #[serde(default)]
    pub layout: String,
    /// @description The card's faces, in printed order.
    /// @internal
    /// Faces in printed order.
    pub faces: Vec<FaceText>,
    /// @description Further printings of the card in the same language, newest
    /// first, each with a different text: where the served printing's line
    /// cannot stand for an ability, a client reads that line from the first of
    /// these that can. At most 6 of them and 4096 bytes of their text per
    /// card; absent when there are none.
    /// @internal
    /// Further printings in [`Self::lang`], for the lines the served one does
    /// not yield: newest first (the catalog's order), the served printing
    /// and repeated texts left out, untranslated faces `None`, at most
    /// [`MAX_CANDIDATES`] and [`MAX_CANDIDATE_BYTES`] of `printed` text.
    /// Empty under `en`, and when no printing translated anything. Not
    /// written when empty, so an entry without any is the JSON it always was.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<TextCandidate>,
}

/// At most this many [`TextCandidate`]s per card.
///
/// A chosen bound, not a measured one. In `fixtures/printings.json` Mind
/// Stone has five German wordings and seven Japanese ones, and the served
/// printing plus six candidates holds all seven; an older wording is only
/// read where every newer one fails, so a cut falls on the oldest.
pub const MAX_CANDIDATES: usize = 6;

/// At most this many bytes of `printed` text over all [`TextCandidate`]s of
/// one card.
///
/// `/catalog/text` faces the internet and reads up to 500 cards a request,
/// so this bounds an answer's growth to 2 MB, and a gateway's in-memory
/// language to 4 KiB per card. A chosen bound, not a measured one: a
/// candidate that would pass it is passed over, never cut.
pub const MAX_CANDIDATE_BYTES: usize = 4096;

/// Another printing of a card in the asked language, offered line by line.
///
/// @description Another printing of the card in the asked language: its
/// layout and each face's rules text exactly as printed (`null` where it is
/// not a translation).
/// @openapi
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct TextCandidate {
    /// @description The printing's Scryfall id.
    /// @internal
    /// Scryfall's printing id.
    pub scryfall_id: String,
    /// @description The printing's Scryfall layout.
    /// @internal
    /// Scryfall's `layout`, which [`align`](crate::align) reads.
    #[serde(default)]
    pub layout: String,
    /// @description Each face's rules text as this printing has it, in face
    /// order; `null` on a face it left untranslated.
    /// @internal
    /// Each face's printed text, as [`FaceText::printed`] is for the served
    /// printing: never English, `None` on a face it left untranslated.
    #[serde(default)]
    pub printed: Vec<Option<String>>,
}

/// Text for one face of a card, already resolved to a language.
///
/// The localized and English names are both carried: a client needs the
/// English name to recognise whether the object on the table is still the
/// card this text describes (a clone is not), and it cannot do that from a
/// translated name.
///
/// @description The text of one face of a card, in the served language.
/// @openapi
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct FaceText {
    /// @description The face's name in the served language.
    /// @internal
    /// Name in the served language.
    pub name: String,
    /// @description The same face's English name.
    /// @internal
    /// English name of the same face.
    pub english_name: String,
    /// @description The face's type line in the served language.
    /// @internal
    /// Type line in the served language.
    pub type_line: String,
    /// @description The face's whole rules text to draw: `printed` where
    /// there is one, the English Oracle text otherwise.
    /// @internal
    /// The face's whole rules text to draw: [`Self::printed`] where there
    /// is one, the English Oracle otherwise.
    pub oracle_text: String,
    /// @description The face's mana cost in Scryfall's notation (`{2}{G}`),
    /// the same in every language.
    /// @internal
    /// Mana cost in Scryfall notation (language-independent).
    pub mana_cost: String,
    /// @description The face's rules text in the asked language, exactly as
    /// that printing has it; `null` when it is not a translation: under
    /// `en`, and when no printing of the card translated this face. Never
    /// English.
    /// @internal
    /// The face's rules text in the asked language, exactly as Scryfall has
    /// it printed, from the printing [`pick`](crate::pick) chose — the input
    /// [`align`](crate::align) takes, as from any other source of printings.
    ///
    /// `None` when that is not a translation: under `en`, where the Oracle
    /// is drawn; when no printing of the card translated anything; and on a
    /// face the chosen printing left untranslated
    /// ([`untranslated`](crate::untranslated)). English is never carried
    /// here, so a `Some` is always the asked language.
    #[serde(default)]
    pub printed: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.";

    /// The row the catalog serves for Mind Stone's German fic printing (read
    /// 2026-09-24): that printing has no German type line, so the English
    /// one stands in, field by field.
    fn mind_stone() -> CardTextEntry {
        CardTextEntry {
            scryfall_id: "b50fd971-3dd1-4878-889f-81e38970408c".to_owned(),
            oracle_id: "c97361b5-af16-4a7b-af85-a429dbaf4ad2".to_owned(),
            lang: "de".to_owned(),
            layout: "normal".to_owned(),
            faces: vec![FaceText {
                name: "Gedankenstein".to_owned(),
                english_name: "Mind Stone".to_owned(),
                type_line: "Artifact".to_owned(),
                oracle_text: TEXT.to_owned(),
                mana_cost: "{2}".to_owned(),
                printed: Some(TEXT.to_owned()),
            }],
            candidates: Vec::new(),
        }
    }

    /// This type as a client from before `candidates` has it.
    #[derive(Deserialize, PartialEq, Debug)]
    struct Before {
        scryfall_id: String,
        #[serde(default)]
        oracle_id: String,
        lang: String,
        #[serde(default)]
        layout: String,
        faces: Vec<FaceText>,
    }

    /// The field candidates added is pinned too, and an entry that has some
    /// is still read by a client that predates them: serde ignores a field
    /// it does not know, which is what an older client's copy of this type
    /// is, here [`Before`].
    #[test]
    fn candidates_are_pinned_and_an_older_client_reads_past_them() {
        let mut entry = mind_stone();
        entry.candidates.push(TextCandidate {
            scryfall_id: "c14".to_owned(),
            layout: "normal".to_owned(),
            printed: vec![Some("{T}: Erhöhe um {1}.".to_owned())],
        });
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            json.ends_with(r#""candidates":[{"scryfall_id":"c14","layout":"normal","printed":["{T}: Erhöhe um {1}."]}]}"#),
            "{json}"
        );
        assert_eq!(serde_json::from_str::<CardTextEntry>(&json).unwrap(), entry);
        let old: Before = serde_json::from_str(&json).unwrap();
        assert_eq!(old.faces, mind_stone().faces);
        assert_eq!(old.scryfall_id, entry.scryfall_id);
        assert_eq!(
            (old.oracle_id.len(), old.lang.as_str(), old.layout.as_str()),
            (36, "de", "normal")
        );
    }

    /// The field names are the contract between a gateway and every client
    /// ever shipped against it; renaming one is a protocol change.
    #[test]
    fn the_json_is_the_wire_contract() {
        let entry = mind_stone();
        let json = serde_json::to_string(&entry).unwrap();
        assert_eq!(
            json,
            r#"{"scryfall_id":"b50fd971-3dd1-4878-889f-81e38970408c","oracle_id":"c97361b5-af16-4a7b-af85-a429dbaf4ad2","lang":"de","layout":"normal","faces":[{"name":"Gedankenstein","english_name":"Mind Stone","type_line":"Artifact","oracle_text":"{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.","mana_cost":"{2}","printed":"{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte."}]}"#
        );
        assert_eq!(serde_json::from_str::<CardTextEntry>(&json).unwrap(), entry);
    }

    /// A client reads a gateway that predates `oracle_ids=`: the fields it
    /// added are missing from that JSON, and read as empty.
    #[test]
    fn an_older_gateway_s_answer_still_reads() {
        let old = r#"{"scryfall_id":"b50fd971-3dd1-4878-889f-81e38970408c","lang":"de","faces":[{"name":"Gedankenstein","english_name":"Mind Stone","type_line":"Artifact","oracle_text":"{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.","mana_cost":"{2}"}]}"#;
        let entry: CardTextEntry = serde_json::from_str(old).unwrap();
        let mut want = mind_stone();
        want.oracle_id.clear();
        want.layout.clear();
        want.faces[0].printed = None;
        assert_eq!(entry, want);
    }
}
