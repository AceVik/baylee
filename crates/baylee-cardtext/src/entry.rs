//! The entry a card is served as: which printing's text, face by face.
//!
//! Moved here from the catalog so that a client asking Scryfall itself
//! builds the entry the gateway would have served, from the same rule.

use crate::wire::{CardTextEntry, FaceText};

/// One printing of a card, with the fields its served text is made of.
///
/// Every field is Scryfall's, so the catalog fills this from its rows and a
/// client from Scryfall's own answer, and both are served by one
/// [`card_entry`].
#[derive(Clone, Debug, Default)]
pub struct TextPrinting {
    /// Scryfall's printing id.
    pub scryfall_id: String,
    /// The card's Oracle id.
    pub oracle_id: String,
    /// Scryfall's language code.
    pub lang: String,
    /// `YYYY-MM-DD`, or empty when Scryfall has no date for it.
    pub released_at: String,
    /// As printed: `263`, `263a`, `MOC-379`.
    pub collector_number: String,
    /// Scryfall's `layout`, or empty.
    pub layout: String,
    /// Its faces, in face order.
    pub faces: Vec<TextFace>,
}

/// One face of a [`TextPrinting`], as Scryfall writes it.
#[derive(Clone, Debug, Default)]
pub struct TextFace {
    /// The English name.
    pub name: String,
    /// The name as this printing prints it.
    pub printed_name: Option<String>,
    /// The English type line.
    pub type_line: Option<String>,
    /// The type line as this printing prints it.
    pub printed_type_line: Option<String>,
    /// The English Oracle text.
    pub oracle_text: Option<String>,
    /// The rules text as this printing prints it.
    pub printed_text: Option<String>,
    /// The mana cost, in Scryfall notation.
    pub mana_cost: Option<String>,
}

/// One card's entry, from its printings in `lang` and in English.
///
/// `printings` are one card's, **in the catalog's order**: newest first
/// (an empty date last), then the shortest collector number, then the
/// collector number, then the printing id. The order is read twice — the
/// English printing that supplies the Oracle is the first English one, and a
/// card whose every printing in `lang` is untranslated is named from the
/// first of those — so a caller that gathers printings anywhere else sorts
/// them this way first.
///
/// `None` only for a card with no printing at all in the language or in
/// English.
#[must_use]
pub fn card_entry(lang: &str, printings: &[TextPrinting]) -> Option<CardTextEntry> {
    let english = printings.iter().find(|p| p.lang == "en");
    let local: Vec<&TextPrinting> = printings
        .iter()
        .filter(|p| lang != "en" && p.lang == lang)
        .collect();
    // Scryfall carries the Oracle on every row, whatever its language, so a
    // card never printed in English still has one.
    let reference = english.or_else(|| local.first().copied())?;
    let oracle: Vec<&str> = reference
        .faces
        .iter()
        .map(|f| f.oracle_text.as_deref().unwrap_or_default())
        .collect();
    let candidates: Vec<crate::Printing> = local
        .iter()
        .map(|p| crate::Printing {
            scryfall_id: p.scryfall_id.clone(),
            released_at: p.released_at.clone(),
            collector_number: p.collector_number.clone(),
            layout: p.layout.clone(),
            printed: p.faces.iter().map(|f| f.printed_text.clone()).collect(),
        })
        .collect();
    let picked = crate::pick(lang, &oracle, &candidates).and_then(|layer| {
        local
            .iter()
            .copied()
            .find(|p| p.scryfall_id == layer.printing.scryfall_id)
    });
    let served = picked
        .or_else(|| local.first().copied())
        .unwrap_or(reference);
    // An English row's `printed_*` is a promo's spelling (Secret Lair's
    // `IMP'S MSCHF`), never the card's.
    let localized = served.lang != "en";
    let faces = served
        .faces
        .iter()
        .enumerate()
        .map(|(index, f)| {
            let oracle_text = oracle
                .get(index)
                .copied()
                .or(f.oracle_text.as_deref())
                .unwrap_or_default();
            let printed = picked
                .and(f.printed_text.as_ref())
                .filter(|printed| !crate::untranslated(oracle_text, Some(printed.as_str())));
            let own = |printed: &Option<String>| printed.as_ref().filter(|_| localized).cloned();
            FaceText {
                name: own(&f.printed_name).unwrap_or_else(|| f.name.clone()),
                english_name: f.name.clone(),
                type_line: own(&f.printed_type_line)
                    .or_else(|| f.type_line.clone())
                    .unwrap_or_default(),
                oracle_text: printed.cloned().unwrap_or_else(|| oracle_text.to_owned()),
                mana_cost: f.mana_cost.clone().unwrap_or_default(),
                printed: printed.cloned(),
            }
        })
        .collect();
    Some(CardTextEntry {
        scryfall_id: served.scryfall_id.clone(),
        oracle_id: served.oracle_id.clone(),
        lang: served.lang.clone(),
        layout: served.layout.clone(),
        faces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORACLE: &str = "{T}: Add {C}.\n{1}, {T}, Sacrifice this artifact: Draw a card.";
    const GERMAN: &str = "{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.";

    fn face(name: &str, printed_name: Option<&str>, printed: Option<&str>) -> TextFace {
        TextFace {
            name: name.to_owned(),
            printed_name: printed_name.map(str::to_owned),
            type_line: Some("Artifact".to_owned()),
            printed_type_line: printed_name.map(|_| "Artefakt".to_owned()),
            oracle_text: Some(ORACLE.to_owned()),
            printed_text: printed.map(str::to_owned),
            mana_cost: Some("{2}".to_owned()),
        }
    }

    fn printing(id: &str, lang: &str, date: &str, faces: Vec<TextFace>) -> TextPrinting {
        TextPrinting {
            scryfall_id: id.to_owned(),
            oracle_id: "card".to_owned(),
            lang: lang.to_owned(),
            released_at: date.to_owned(),
            collector_number: "1".to_owned(),
            layout: "normal".to_owned(),
            faces,
        }
    }

    /// What a client leans on, held on every entry the tests below build: a
    /// `printed` is always the asked language and never English, and an
    /// entry that fell back to English names carries none.
    fn served(lang: &str, printings: &[TextPrinting]) -> CardTextEntry {
        let entry = card_entry(lang, printings).expect("an entry");
        for face in &entry.faces {
            if face.printed.is_some() {
                assert_ne!(lang, "en", "English is never a printed layer");
                assert_eq!(entry.lang, lang, "a printed layer is the asked language");
                assert_eq!(
                    face.printed.as_ref(),
                    Some(&face.oracle_text),
                    "the drawn text is the printed layer"
                );
            }
        }
        if entry.lang != lang {
            assert!(entry.faces.iter().all(|f| f.printed.is_none()));
        }
        entry
    }

    #[test]
    fn a_card_is_served_from_the_printing_pick_chooses() {
        let printings = [
            printing(
                "english",
                "en",
                "2026-01-01",
                vec![face("Mind Stone", None, None)],
            ),
            printing(
                "null",
                "de",
                "2025-06-13",
                vec![face("Mind Stone", Some("Gedankenstein"), None)],
            ),
            printing(
                "german",
                "de",
                "2007-07-13",
                vec![face("Mind Stone", Some("Gedankenstein"), Some(GERMAN))],
            ),
        ];
        let entry = served("de", &printings);
        assert_eq!(
            (entry.scryfall_id.as_str(), entry.lang.as_str()),
            ("german", "de")
        );
        assert_eq!(entry.faces[0].printed.as_deref(), Some(GERMAN));
        assert_eq!(entry.faces[0].name, "Gedankenstein");
        assert_eq!(entry.faces[0].english_name, "Mind Stone");
        assert_eq!(entry.faces[0].type_line, "Artefakt");
    }

    /// Both branches of "untranslated": English words under `de` and no
    /// text at all name the card in German and draw the Oracle.
    #[test]
    fn english_under_another_language_is_never_served_as_it() {
        let english_words = "{T}: Add {C}.\n{1}, {T}, sacrifice this artifact: draw a card.";
        for printed in [Some(english_words), None] {
            let printings = [printing(
                "untranslated",
                "de",
                "2025-06-13",
                vec![face("Mind Stone", Some("Gedankenstein"), printed)],
            )];
            let entry = served("de", &printings);
            assert_eq!(entry.lang, "de");
            assert_eq!(entry.faces[0].name, "Gedankenstein");
            assert_eq!(entry.faces[0].printed, None);
            assert_eq!(entry.faces[0].oracle_text, ORACLE);
        }
    }

    #[test]
    fn a_card_never_printed_in_the_language_is_served_in_english() {
        let printings = [printing(
            "english",
            "en",
            "2026-01-01",
            vec![face("Mind Stone", None, None)],
        )];
        let entry = served("de", &printings);
        assert_eq!(
            (entry.scryfall_id.as_str(), entry.lang.as_str()),
            ("english", "en")
        );
        assert_eq!(entry.faces[0].printed, None);
        assert_eq!(entry.faces[0].oracle_text, ORACLE);
    }

    /// Under `en` the Oracle is drawn, and the Oracle's name: a Secret Lair
    /// row carries `printed_*` spellings that are the promo's, not the
    /// card's.
    #[test]
    fn english_is_the_oracle_and_the_oracle_s_name() {
        let printings = [printing(
            "sld",
            "en",
            "2026-09-02",
            vec![face("Mind Stone", Some("MIND STN"), Some("TAP: ADD C"))],
        )];
        let entry = served("en", &printings);
        assert_eq!(entry.faces[0].name, "Mind Stone");
        assert_eq!(entry.faces[0].type_line, "Artifact");
        assert_eq!(entry.faces[0].printed, None);
        assert_eq!(entry.faces[0].oracle_text, ORACLE);
    }

    /// A double-faced card is drawn from one printing: the one whose every
    /// face is translated wins over a newer one with an untranslated back,
    /// and with only the newer one, its back falls to the Oracle alone.
    #[test]
    fn every_face_of_an_entry_comes_from_one_printing() {
        let half = printing(
            "half",
            "de",
            "2025-01-01",
            vec![
                face("Front", Some("Vorne"), Some(GERMAN)),
                face("Back", Some("Hinten"), None),
            ],
        );
        let whole = printing(
            "whole",
            "de",
            "2020-01-01",
            vec![
                face("Front", Some("Vorne"), Some(GERMAN)),
                face("Back", Some("Hinten"), Some(GERMAN)),
            ],
        );
        let entry = served("de", &[half.clone(), whole]);
        assert_eq!(entry.scryfall_id, "whole");
        assert!(entry.faces.iter().all(|f| f.printed.is_some()));

        let entry = served("de", &[half]);
        assert_eq!(entry.scryfall_id, "half");
        assert_eq!(entry.faces[0].printed.as_deref(), Some(GERMAN));
        assert_eq!(entry.faces[1].printed, None);
        assert_eq!(entry.faces[1].oracle_text, ORACLE);
        assert_eq!(entry.faces[1].name, "Hinten");
    }
}
