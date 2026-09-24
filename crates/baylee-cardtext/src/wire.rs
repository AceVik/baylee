//! What the gateway's `/catalog/text` answers, as one type for both ends.

use serde::{Deserialize, Serialize};

/// Every face of one requested printing, in one language.
///
/// It used to be written twice — once in the catalog, once in the client,
/// which cannot link the catalog's ORM and Postgres driver and has to build
/// for wasm — with a test on each side pinning the JSON so the copies could
/// not drift. This crate links neither, so there is one copy and one pin.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct CardTextEntry {
    /// The printing id that was asked for.
    pub scryfall_id: String,
    /// The language actually served, after the fallback.
    pub lang: String,
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
    /// Rules text in the served language.
    pub oracle_text: String,
    /// Mana cost in Scryfall notation (language-independent).
    pub mana_cost: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The field names are the contract between a gateway and every client
    /// ever shipped against it; renaming one is a protocol change. The row
    /// is what the catalog serves for Mind Stone's German fic printing (read
    /// 2026-09-24): that printing has no German type line, so the English
    /// one stands in, field by field.
    #[test]
    fn the_json_is_the_wire_contract() {
        let entry = CardTextEntry {
            scryfall_id: "b50fd971-3dd1-4878-889f-81e38970408c".to_owned(),
            lang: "de".to_owned(),
            faces: vec![FaceText {
                name: "Gedankenstein".to_owned(),
                english_name: "Mind Stone".to_owned(),
                type_line: "Artifact".to_owned(),
                oracle_text:
                    "{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte."
                        .to_owned(),
                mana_cost: "{2}".to_owned(),
            }],
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert_eq!(
            json,
            r#"{"scryfall_id":"b50fd971-3dd1-4878-889f-81e38970408c","lang":"de","faces":[{"name":"Gedankenstein","english_name":"Mind Stone","type_line":"Artifact","oracle_text":"{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.","mana_cost":"{2}"}]}"#
        );
        assert_eq!(serde_json::from_str::<CardTextEntry>(&json).unwrap(), entry);
    }
}
