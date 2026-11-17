//! The entry a card is served as: which printing's text, face by face.
//!
//! Moved here from the catalog so that a client asking Scryfall itself
//! builds the entry the gateway would have served, from the same rule.

use crate::wire::{CardTextEntry, FaceText, TextCandidate};

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
/// first of those that translated the most of it (`fullest`) — so a caller
/// that gathers printings anywhere else sorts them this way first.
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
    let served = picked.or_else(|| fullest(&local)).unwrap_or(reference);
    // An English row's `printed_*` is a promo's spelling (Secret Lair's
    // `IMP'S MSCHF`), never the card's.
    let localized = served.lang != "en";
    let faces = served
        .faces
        .iter()
        .enumerate()
        .map(|(index, f)| {
            let oracle_text = oracle_of(&oracle, index, f);
            let printed = picked.and(translated(&oracle, index, f));
            let printed = printed.as_ref();
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
    let candidates = picked.map_or_else(Vec::new, |served| further(&oracle, served, &local));
    Some(CardTextEntry {
        scryfall_id: served.scryfall_id.clone(),
        oracle_id: served.oracle_id.clone(),
        lang: served.lang.clone(),
        layout: served.layout.clone(),
        faces,
        candidates,
    })
}

/// The Oracle text face `index` is placed against: the reference printing's,
/// else the face's own.
fn oracle_of<'a>(oracle: &[&'a str], index: usize, face: &'a TextFace) -> &'a str {
    oracle
        .get(index)
        .copied()
        .or(face.oracle_text.as_deref())
        .unwrap_or_default()
}

/// The face's printed text where it is a translation ([`crate::untranslated`]).
fn translated(oracle: &[&str], index: usize, face: &TextFace) -> Option<String> {
    face.printed_text
        .clone()
        .filter(|printed| !crate::untranslated(oracle_of(oracle, index, face), Some(printed)))
}

/// The printings in the language other than `served`, as
/// [`CardTextEntry::candidates`]: in `local`'s order, which is newest first;
/// one that translated nothing, or whose text the served printing or an
/// earlier candidate already carries, left out; cut at
/// [`MAX_CANDIDATES`](crate::MAX_CANDIDATES) and, a whole printing at a time,
/// at [`MAX_CANDIDATE_BYTES`](crate::MAX_CANDIDATE_BYTES).
///
/// A printing too long for what is left of the bytes is passed over rather
/// than ending the list, so one wordy promo does not cost the card every
/// older wording behind it.
fn further(oracle: &[&str], served: &TextPrinting, local: &[&TextPrinting]) -> Vec<TextCandidate> {
    let texts = |p: &TextPrinting| -> Vec<Option<String>> {
        p.faces
            .iter()
            .enumerate()
            .map(|(index, face)| translated(oracle, index, face))
            .collect()
    };
    let mut seen = vec![texts(served)];
    let mut bytes = 0;
    let mut out = Vec::new();
    for printing in local {
        if out.len() == crate::MAX_CANDIDATES {
            break;
        }
        if printing.scryfall_id == served.scryfall_id {
            continue;
        }
        let printed = texts(printing);
        if printed.iter().all(Option::is_none) || seen.contains(&printed) {
            continue;
        }
        let size: usize = printed.iter().flatten().map(String::len).sum();
        if bytes + size > crate::MAX_CANDIDATE_BYTES {
            continue;
        }
        bytes += size;
        seen.push(printed.clone());
        out.push(TextCandidate {
            scryfall_id: printing.scryfall_id.clone(),
            layout: printing.layout.clone(),
            printed,
        });
    }
    out
}

/// Of printings none of which translated the rules text, the one that
/// translated the most of the rest: names, type lines and texts, counted
/// over every face. Ties go to the first, which in the order [`card_entry`]
/// reads is the newest.
///
/// Not simply the newest. A promo that prints only the name `Wald` would
/// put `Basic Land — Forest` on a German card beside an older printing's
/// `Basisland — Wald`; the deck builder picked by this count before the
/// entry did (#239).
fn fullest<'p>(local: &[&'p TextPrinting]) -> Option<&'p TextPrinting> {
    let translated = |printing: &TextPrinting| {
        printing
            .faces
            .iter()
            .flat_map(|f| [&f.printed_name, &f.printed_type_line, &f.printed_text])
            .filter(|text| text.as_deref().is_some_and(|t| !t.is_empty()))
            .count()
    };
    local.iter().copied().reduce(|best, p| {
        if translated(p) > translated(best) {
            p
        } else {
            best
        }
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
        // Candidates only beside a translated served printing, in the asked
        // language, within both caps, and none repeating another's text.
        if lang == "en" || entry.faces.iter().all(|f| f.printed.is_none()) {
            assert!(entry.candidates.is_empty(), "nothing to offer beside");
        }
        assert!(entry.candidates.len() <= crate::MAX_CANDIDATES);
        let bytes: usize = entry
            .candidates
            .iter()
            .flat_map(|c| c.printed.iter().flatten())
            .map(String::len)
            .sum();
        assert!(bytes <= crate::MAX_CANDIDATE_BYTES);
        let served_text: Vec<Option<String>> =
            entry.faces.iter().map(|f| f.printed.clone()).collect();
        for (n, candidate) in entry.candidates.iter().enumerate() {
            assert_ne!(candidate.scryfall_id, entry.scryfall_id);
            assert_ne!(candidate.printed, served_text);
            assert!(candidate.printed.iter().any(Option::is_some));
            assert!(
                entry.candidates[..n]
                    .iter()
                    .all(|c| c.printed != candidate.printed)
            );
            let printing = printings
                .iter()
                .find(|p| p.scryfall_id == candidate.scryfall_id)
                .expect("a candidate is one of the card's printings");
            assert_eq!(printing.lang, lang, "a candidate is the asked language");
        }
        entry
    }

    /// Mind Stone's German wordings, newest first, as `de` printings.
    fn wordings(texts: &[(&str, &str)]) -> Vec<TextPrinting> {
        texts
            .iter()
            .enumerate()
            .map(|(n, (id, text))| {
                printing(
                    id,
                    "de",
                    &format!("20{:02}-01-01", 25 - n),
                    vec![face("Mind Stone", Some("Gedankenstein"), Some(text))],
                )
            })
            .collect()
    }

    /// Every other German wording is offered beside the served one, newest
    /// first; a repeated wording and an untranslated printing are not.
    #[test]
    fn the_other_wordings_are_offered_newest_first_each_once() {
        const C14: &str = "{T}: Erhöhe deinen Manavorrat um {1}.\n{1}, {T}, opfere den Gedankenstein: Ziehe eine Karte.";
        const C15: &str = "{T}: Erhöhe deinen Manavorrat um {1}.{1}, {T}, opfere den Gedankenstein: Ziehe eine Karte.";
        let mut printings = wordings(&[
            ("c15", C15),
            ("fic", GERMAN),
            ("again", GERMAN),
            ("english", ORACLE),
            ("c14", C14),
            ("10e", C14),
        ]);
        printings.push(printing(
            "null",
            "de",
            "1990-01-01",
            vec![face("Mind Stone", Some("Gedankenstein"), None)],
        ));
        let entry = served("de", &printings);
        assert_eq!(
            entry.scryfall_id, "fic",
            "the newest that lines up is served"
        );
        let offered: Vec<&str> = entry
            .candidates
            .iter()
            .map(|c| c.scryfall_id.as_str())
            .collect();
        assert_eq!(offered, ["c15", "c14"]);
        assert_eq!(entry.candidates[1].printed, [Some(C14.to_owned())]);
        assert_eq!(entry.candidates[1].layout, "normal");
    }

    /// Both caps, each by its own branch: the count ends the list, and a
    /// printing too long for the bytes left is passed over for a shorter one
    /// behind it.
    #[test]
    fn the_candidates_are_capped_by_count_and_by_bytes() {
        let many: Vec<String> = (0..10)
            .map(|n| format!("{{T}}: Erzeuge {{C}}. ({n})\n{{1}}, {{T}}, opfere: Ziehe."))
            .collect();
        let rows: Vec<(String, &str)> = many
            .iter()
            .enumerate()
            .map(|(n, text)| (format!("p{n}"), text.as_str()))
            .collect();
        let rows: Vec<(&str, &str)> = rows.iter().map(|(id, t)| (id.as_str(), *t)).collect();
        let entry = served("de", &wordings(&rows));
        assert_eq!(entry.candidates.len(), crate::MAX_CANDIDATES);
        assert_eq!(entry.candidates[0].scryfall_id, "p1");

        let long = format!(
            "{{T}}: Erzeuge {{C}}.\n{}",
            "x".repeat(crate::MAX_CANDIDATE_BYTES)
        );
        let entry = served(
            "de",
            &wordings(&[("fic", GERMAN), ("long", &long), ("short", &many[0])]),
        );
        let offered: Vec<&str> = entry
            .candidates
            .iter()
            .map(|c| c.scryfall_id.as_str())
            .collect();
        assert_eq!(offered, ["short"]);
    }

    /// Nothing is offered under English, nor where nothing was translated.
    #[test]
    fn nothing_is_offered_where_nothing_is_translated() {
        let printings = [
            printing(
                "en1",
                "en",
                "2026-01-01",
                vec![face("Mind Stone", None, Some("TAP: ADD C"))],
            ),
            printing(
                "en2",
                "en",
                "2020-01-01",
                vec![face("Mind Stone", None, Some("T: add C"))],
            ),
        ];
        assert!(served("en", &printings).candidates.is_empty());
        let null = [printing(
            "null",
            "de",
            "2026-01-01",
            vec![face("Mind Stone", Some("Gedankenstein"), None)],
        )];
        assert!(served("de", &null).candidates.is_empty());
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

    /// A basic land's face, whose whole Oracle text is its reminder.
    fn basic(english: &str, local: &str, oracle: &str, printed: Option<&str>) -> TextFace {
        TextFace {
            name: english.to_owned(),
            printed_name: Some(local.to_owned()),
            type_line: Some(format!("Basic Land — {english}")),
            printed_type_line: Some(format!("Standardland — {local}")),
            oracle_text: Some(oracle.to_owned()),
            printed_text: printed.map(str::to_owned),
            mana_cost: None,
        }
    }

    /// Island's full-art printings (NEO 285 and its kind) print a bare `U`
    /// where the reminder goes. NEO is newer than the German reminder, and
    /// its `U` was served as the card's rules text. It translates nothing, so
    /// the reminder speaks for the card; with only the symbol to choose
    /// from, the card is named and typed in German and read in English.
    #[test]
    fn a_full_art_basic_s_mana_symbol_is_not_its_rules_text() {
        const ISLAND: &str = "({T}: Add {U}.)";
        let full_art = printing(
            "neo",
            "de",
            "2022-02-18",
            vec![basic("Island", "Insel", ISLAND, Some("U"))],
        );
        let reminder = printing(
            "m20",
            "de",
            "2019-07-12",
            vec![basic(
                "Island",
                "Insel",
                ISLAND,
                Some("({T}: Erzeuge {U}.)"),
            )],
        );

        let entry = served("de", &[full_art.clone(), reminder]);
        assert_eq!(entry.scryfall_id, "m20");
        assert_eq!(
            entry.faces[0].printed.as_deref(),
            Some("({T}: Erzeuge {U}.)")
        );

        let entry = served("de", &[full_art]);
        assert_eq!(
            (entry.scryfall_id.as_str(), entry.lang.as_str()),
            ("neo", "de")
        );
        assert_eq!(entry.faces[0].name, "Insel");
        assert_eq!(entry.faces[0].type_line, "Standardland — Insel");
        assert_eq!(entry.faces[0].printed, None);
        assert_eq!(entry.faces[0].oracle_text, ISLAND);
    }

    /// With no printing that translates the rules text, the card is still
    /// named and typed in the language, by the printing that translated the
    /// most of it rather than the newest. A promo Forest that prints only
    /// `Wald` would otherwise put `Basic Land — Forest` on a German card
    /// beside an older printing's `Basisland — Wald`. This is the lobby's
    /// `translated_types_win_over_a_name_only_promo`, whose rule this is.
    #[test]
    fn with_no_text_translated_the_most_complete_printing_names_the_card() {
        const FOREST: &str = "({T}: Add {G}.)";
        let mut name_only = basic("Forest", "Wald", FOREST, None);
        name_only.printed_type_line = None;
        let promo = printing("promo", "de", "2024-01-01", vec![name_only]);
        let older = printing(
            "older",
            "de",
            "2019-07-12",
            vec![basic("Forest", "Wald", FOREST, None)],
        );
        let entry = served("de", &[promo, older.clone()]);
        assert_eq!(entry.scryfall_id, "older");
        assert_eq!(entry.faces[0].type_line, "Standardland — Wald");
        assert_eq!(entry.faces[0].printed, None);

        let newer = printing(
            "newer",
            "de",
            "2023-01-01",
            vec![basic("Forest", "Wald", FOREST, None)],
        );
        assert_eq!(
            served("de", &[newer, older]).scryfall_id,
            "newer",
            "and between two that translated as much, the newest, as before"
        );
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
