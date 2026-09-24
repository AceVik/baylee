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
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct CardTextEntry {
    /// Asked for by card (`oracle_ids=`): the printing every face below is
    /// read from. Asked for by printing (`ids=`): the printing id that was
    /// asked for, which need not be that one — [`Self::lang`],
    /// [`Self::layout`] and the faces are still the served printing's.
    pub scryfall_id: String,
    /// The card: Scryfall's oracle id. Empty from a gateway that predates
    /// `oracle_ids=`.
    #[serde(default)]
    pub oracle_id: String,
    /// The language of the printing the names and type lines are read from.
    /// Whether a face's rules text is in it is [`FaceText::printed`].
    pub lang: String,
    /// That printing's Scryfall `layout`, which [`align`](crate::align)
    /// reads: a modal double-faced card's hint lines are removed only there.
    #[serde(default)]
    pub layout: String,
    /// Faces in printed order.
    pub faces: Vec<FaceText>,
}

/// Text for one face of a card, already resolved to a language.
///
/// The localized and English names are both carried: a client needs the
/// English name to recognise whether the object on the table is still the
/// card this text describes (a clone is not), and it cannot do that from a
/// translated name.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct FaceText {
    /// Name in the served language.
    pub name: String,
    /// English name of the same face.
    pub english_name: String,
    /// Type line in the served language.
    pub type_line: String,
    /// The face's whole rules text to draw: [`Self::printed`] where there
    /// is one, the English Oracle otherwise.
    pub oracle_text: String,
    /// Mana cost in Scryfall notation (language-independent).
    pub mana_cost: String,
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
        }
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
