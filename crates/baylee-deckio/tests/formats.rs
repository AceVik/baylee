//! Every format, read and written, against real exports and against the
//! whole document model.

use baylee_core::preset::Finish;
use baylee_deckio::document::FINISHES;
use baylee_deckio::format::{self, detect};
use baylee_deckio::source::{Answer, Instruction, SourceId};
use baylee_deckio::{
    Card, Document, FormatId, Import, LossKind, MAX_DOCUMENT_BYTES, MAX_ROWS, ReadError, Zone,
    export, import,
};
use std::fmt::Write as _;

/// A real "Copy for Moxfield" export, byte for byte as Moxfield wrote it.
const MOXFIELD: &str = include_str!("fixtures/moxfield-export.txt");

fn read(format: FormatId, text: &str) -> Document {
    format::read(format, text)
        .unwrap_or_else(|e| panic!("{format:?}: {e}"))
        .document
}

fn imported(text: &str) -> (FormatId, Document) {
    match import(text).expect("imports") {
        Import::Read { format, read } => {
            assert!(read.skipped.is_empty(), "skipped {:?}", read.skipped);
            (format, read.document)
        }
        other => panic!("not a deck: {other:?}"),
    }
}

/// A document that uses every field, every zone and every finish.
fn everything() -> Document {
    let mut cards = Vec::new();
    for (at, finish) in FINISHES.into_iter().enumerate() {
        cards.push(Card {
            zone: [Zone::Main, Zone::Side, Zone::Maybe][at % 3],
            count: u32::try_from(at + 1).unwrap(),
            name: format!("Card {at}"),
            set: Some("M11".into()),
            collector_number: Some(format!("{}p", 100 + at)),
            lang: Some(["de", "ja", "en"][at % 3].into()),
            finish: Some(finish),
            scryfall_id: Some(format!("e3285e6a-8c1d-4c9f-9a3f-2f0a4d2f0a4{at}")),
            note: Some(format!("note {at}")),
        });
    }
    cards.insert(
        0,
        Card {
            zone: Zone::Commander,
            count: 1,
            name: "Fable of the Mirror-Breaker // Reflection of Kiki-Jiki".into(),
            set: Some("NEO".into()),
            collector_number: Some("141".into()),
            lang: None,
            finish: None,
            scryfall_id: None,
            note: None,
        },
    );
    Document {
        version: 1,
        name: Some("Alles drin".into()),
        format: Some("commander".into()),
        cards,
    }
}

#[test]
fn the_moxfield_sample_reads_every_row_into_its_zone() {
    let (format, doc) = imported(MOXFIELD);
    assert_eq!(format, FormatId::Moxfield);
    assert_eq!(doc.cards.len(), 129, "every row of the export");
    assert_eq!(doc.zone(Zone::Main).count(), 100);
    assert_eq!(doc.zone(Zone::Side).count(), 29);
    assert_eq!(doc.cards.iter().map(|c| c.count).sum::<u32>(), 129);

    let named = |name: &str| {
        doc.cards
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("{name} was not read"))
    };
    // Moxfield's `Front / Back` is Baylee's and Scryfall's `Front // Back`.
    for dfc in [
        "Archangel Avacyn // Avacyn, the Purifier",
        "Fable of the Mirror-Breaker // Reflection of Kiki-Jiki",
        "Huntmaster of the Fells // Ravager of the Fells",
        "Witch Enchanter // Witch-Blessed Meadow",
    ] {
        named(dfc);
    }
    let garden = named("Jetmir's Garden");
    assert_eq!(garden.zone, Zone::Side);
    assert_eq!(garden.finish, Some(Finish::Foil));
    assert_eq!(garden.set.as_deref(), Some("PSNC"));
    assert_eq!(garden.collector_number.as_deref(), Some("250s"));
    // Collector numbers as Scryfall spells them: a promo's letter, The
    // List's `SET-NUM`.
    assert_eq!(
        named("Atraxa, Grand Unifier").collector_number.as_deref(),
        Some("196p")
    );
    assert_eq!(
        named("Crop Rotation").collector_number.as_deref(),
        Some("DDR-7")
    );
    assert_eq!(named("Mawloc").set.as_deref(), Some("40K"));
    assert_eq!(
        doc.cards.iter().filter(|c| c.finish.is_some()).count(),
        1,
        "one foil in the whole export"
    );
}

#[test]
fn the_moxfield_sample_writes_back_byte_for_byte() {
    let doc = read(FormatId::Moxfield, MOXFIELD);
    let written = export(FormatId::Moxfield, &doc);
    assert_eq!(written.text, MOXFIELD);
    assert!(written.losses.is_empty(), "{:?}", written.losses);
}

#[test]
fn the_moxfield_sample_survives_every_format_in_turn() {
    let start = read(FormatId::Moxfield, MOXFIELD);
    let mut doc = start.clone();
    for format in [
        FormatId::Json,
        FormatId::Yaml,
        FormatId::Baylee,
        FormatId::Moxfield,
    ] {
        let text = export(format, &doc).text;
        let (detected, back) = imported(&text);
        assert_eq!(detected, format, "{text}");
        doc = back;
        assert_eq!(doc, start, "through {format:?}");
    }
}

#[test]
fn json_and_yaml_carry_every_field_and_every_finish() {
    let doc = everything();
    doc.validate().expect("the test document is a valid one");
    for format in [FormatId::Json, FormatId::Yaml] {
        let written = export(format, &doc);
        assert!(written.losses.is_empty());
        let (detected, back) = imported(&written.text);
        assert_eq!(detected, format);
        assert_eq!(back, doc, "{format:?}:\n{}", written.text);
    }
}

#[test]
fn json_writes_the_finish_in_its_own_words() {
    let text = export(FormatId::Json, &everything()).text;
    for word in [
        "\"nonfoil\"",
        "\"foil\"",
        "\"etched\"",
        "\"holographic\"",
        "\"glitter\"",
        "\"galaxy\"",
    ] {
        assert!(text.contains(word), "{word} in {text}");
    }
    assert!(text.contains("\"version\": 1"));
    assert!(text.contains("\"zone\": \"commander\""));
}

#[test]
fn the_spec_example_reads() {
    // `docs/deck-format.md` §"JSON format", with the zone words it names.
    let text = r#"{"version":1,"name":"Bolt","format":"constructed","cards":[
        {"zone":"main","count":4,"scryfall_id":null,"set":null,"collector_number":null,"name":"Lightning Bolt","lang":null,"finish":null},
        {"zone":"side","count":1,"name":"Karakas","set":"EMA","collector_number":"240","lang":"de","finish":"foil"},
        {"count":1,"name":"Forest"}]}"#;
    let (_, doc) = imported(text);
    assert_eq!(doc.cards.len(), 3);
    assert_eq!(doc.cards[1].finish, Some(Finish::Foil));
    assert_eq!(doc.cards[2].zone, Zone::Main, "zone defaults to main");
}

#[test]
fn baylee_text_says_everything_but_a_number_without_a_set() {
    let doc = everything();
    let written = export(FormatId::Baylee, &doc);
    assert!(written.losses.is_empty(), "{:?}", written.losses);
    assert!(
        written
            .text
            .starts_with("# baylee deck export v1\n# name: Alles drin\n")
    );
    assert!(
        written
            .text
            .contains("\nCMD: 1 Fable of the Mirror-Breaker // ")
    );
    assert!(written.text.contains("\nSB: "));
    assert!(written.text.contains("\nMB: "));
    let (detected, back) = imported(&written.text);
    assert_eq!(detected, FormatId::Baylee);
    // The text writes zone by zone; the document listed them interleaved.
    // And a row that named the default finish reads back as one that named
    // none, which is what `deckrow` means by it.
    let mut sorted = doc.clone();
    for card in &mut sorted.cards {
        if card.finish == Some(Finish::Normal) {
            card.finish = None;
        }
    }
    sorted.cards.sort_by_key(|c| match c.zone {
        Zone::Commander => 0,
        Zone::Main => 1,
        Zone::Side => 2,
        Zone::Maybe => 3,
    });
    assert_eq!(back, sorted);

    let mut unanchored = Document::default();
    unanchored.cards.push(Card {
        collector_number: Some("149".into()),
        ..card("Lightning Bolt")
    });
    let written = export(FormatId::Baylee, &unanchored);
    assert_eq!(written.losses[0].kind, LossKind::CollectorNumber);
}

fn card(name: &str) -> Card {
    Card {
        zone: Zone::Main,
        count: 1,
        name: name.into(),
        set: None,
        collector_number: None,
        lang: None,
        finish: None,
        scryfall_id: None,
        note: None,
    }
}

#[test]
fn moxfield_names_what_it_cannot_write_and_claims_no_foil_it_was_not_given() {
    let doc = everything();
    let written = export(FormatId::Moxfield, &doc);
    let kinds: Vec<(LossKind, usize)> = written.losses.iter().map(|l| (l.kind, l.rows)).collect();
    assert_eq!(
        kinds,
        [
            // The maybeboard's two rows are not written at all, so only
            // the other four count against the fields.
            (LossKind::Name, 1),
            (LossKind::Lang, 4),
            (LossKind::ScryfallId, 4),
            (LossKind::Note, 4),
            (LossKind::Finish, 2),
            (LossKind::Maybeboard, 2),
        ]
    );
    // Holographic, glitter and galaxy are written plain, never as `*F*`.
    for (at, finish) in FINISHES.into_iter().enumerate() {
        let line = written
            .text
            .lines()
            .find(|l| l.contains(&format!("Card {at} ")))
            .map(str::to_string);
        let expected = match finish {
            Finish::Foil => Some("*F*"),
            Finish::Etched => Some("*E*"),
            _ => None,
        };
        if let Some(line) = line {
            match expected {
                Some(marker) => assert!(line.ends_with(marker), "{line}"),
                None => assert!(!line.contains('*'), "{line}"),
            }
        }
    }
    assert!(
        written.text.starts_with(
            "COMMANDER:\n1 Fable of the Mirror-Breaker / Reflection of Kiki-Jiki (NEO) 141\n\n"
        ),
        "{}",
        written.text
    );
    assert!(written.text.contains("\nSIDEBOARD:\n"));
    // And back: the commander in its zone, the faces in Baylee's spelling.
    let (_, back) = imported(&written.text);
    assert_eq!(back.cards[0].zone, Zone::Commander);
    assert_eq!(
        back.cards[0].name,
        "Fable of the Mirror-Breaker // Reflection of Kiki-Jiki"
    );
}

#[test]
fn a_plain_list_is_read_by_whichever_format_it_points_to() {
    assert_eq!(
        detect("4 Lightning Bolt\n20 Mountain\n"),
        Some(FormatId::Baylee)
    );
    assert_eq!(
        detect("1x Aesi, Tyrant of Gyre Strait (DSC)\n1x Archdruid's Charm\n"),
        Some(FormatId::Moxfield)
    );
    assert_eq!(detect("1 A\n\nSIDEBOARD:\n1 B\n"), Some(FormatId::Moxfield));
    assert_eq!(detect("1 A\nSB: 1 B\n"), Some(FormatId::Baylee));
    assert_eq!(detect("  {\"version\": 1}"), Some(FormatId::Json));
    assert_eq!(detect("version: 1\ncards: []\n"), Some(FormatId::Yaml));
    assert_eq!(detect("---\nversion: 1\n"), Some(FormatId::Yaml));
    assert_eq!(
        detect("Commander\n1 Atraxa\n\nDeck\n1 Sol Ring"),
        Some(FormatId::Moxfield)
    );
    assert_eq!(detect("hello there"), None);
}

#[test]
fn the_other_sites_x_counts_read() {
    let (format, doc) = imported(
        "1x Aesi, Tyrant of Gyre Strait (DSC)\n1x Archdruid's Charm\n1x Arid Mesa (SPG)\n",
    );
    assert_eq!(format, FormatId::Moxfield);
    assert_eq!(doc.cards.len(), 3);
    assert_eq!(doc.cards[2].set.as_deref(), Some("SPG"));
}

#[test]
fn a_moxfield_link_is_an_instruction_not_a_fetch() {
    match import("https://moxfield.com/decks/7dKEA5LPD0aX18EZPw_HYg").expect("a link") {
        Import::Source(found) => {
            assert_eq!(found.source, SourceId::Moxfield);
            assert_eq!(
                found.answer,
                Answer::Instruction(Instruction::MoxfieldExport)
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        import("https://example.org/decks/1"),
        Ok(Import::UnknownLink("example.org".into()))
    );
}

#[test]
fn a_line_that_is_no_row_is_reported_not_dropped() {
    let read = format::read(FormatId::Baylee, "4 Lightning Bolt\nLightning Bolt\n").expect("read");
    assert_eq!(read.document.cards.len(), 1);
    assert_eq!(read.skipped.len(), 1);
    assert_eq!(read.skipped[0].line, 2);
    assert_eq!(read.skipped[0].text, "Lightning Bolt");
    // Nothing readable at all is an error that names the first line.
    assert!(matches!(
        format::read(FormatId::Baylee, "Lightning Bolt\nShock\n"),
        Err(ReadError::Unreadable(_))
    ));
}

#[test]
fn a_stranger_cannot_make_the_reader_do_unbounded_work() {
    let huge = "1 Forest\n".repeat(MAX_DOCUMENT_BYTES / 9 + 1);
    assert!(matches!(import(&huge), Err(ReadError::TooLarge { .. })));

    let many = "1 Forest\n".repeat(MAX_ROWS + 1);
    assert!(many.len() < MAX_DOCUMENT_BYTES);
    for format in [FormatId::Baylee, FormatId::Moxfield] {
        assert!(matches!(
            format::read(format, &many),
            Err(ReadError::TooManyRows(_))
        ));
    }
    let doc = Document {
        cards: vec![card("Forest"); MAX_ROWS + 1],
        ..Document::default()
    };
    let json = export(FormatId::Json, &doc).text;
    // Written pretty, a thousand rows is past the byte limit before the row
    // limit is reached; compact, the row limit is what refuses it.
    let compact = serde_json::to_string(&doc).unwrap();
    assert!(import(&json).is_err());
    assert_eq!(import(&compact), Err(ReadError::TooManyRows(MAX_ROWS + 1)));
}

#[test]
fn a_document_of_another_version_is_refused_by_name() {
    assert_eq!(
        import(r#"{"version":2,"cards":[{"count":1,"name":"Forest"}]}"#),
        Err(ReadError::Version(2))
    );
    assert_eq!(
        import("version: 2\ncards:\n  - count: 1\n    name: Forest\n"),
        Err(ReadError::Version(2))
    );
}

#[test]
fn a_broken_document_says_what_its_parser_said() {
    assert!(matches!(
        import("{\"version\":1,"),
        Err(ReadError::Syntax(_))
    ));
    assert!(matches!(
        import(r#"{"version":1,"cards":[{"count":1,"name":"Forest","finish":"shiny"}]}"#),
        Err(ReadError::Syntax(message)) if message.contains("shiny")
    ));
    assert!(matches!(
        import(r#"{"version":1,"cards":[{"count":0,"name":"Forest"}]}"#),
        Err(ReadError::Card { at: 0, .. })
    ));
    assert_eq!(import("   \n"), Err(ReadError::Empty));
}

#[test]
fn yaml_aliases_cannot_multiply_a_small_file_into_a_large_deck() {
    // The classic expansion: each level names the one below it ten times.
    let mut text = String::from("version: 1\na0: &a0 {count: 1, name: Forest}\n");
    for level in 1..8 {
        let below = level - 1;
        let refs = vec![format!("*a{below}"); 10].join(", ");
        let _ = writeln!(text, "a{level}: &a{level} [{refs}]");
    }
    text.push_str("cards: *a7\n");
    assert!(text.len() < 2_000);
    // Refused by the parser's budget while expanding, not by a type error
    // after building ten million nodes.
    assert!(
        matches!(import(&text), Err(ReadError::Syntax(m)) if m.contains("budget")),
        "ten million rows from 2 KB"
    );
    // The flat form, too: one card named two thousand times.
    let flat = format!(
        "version: 1\nc: &c {{count: 1, name: Forest}}\ncards: [{}]\n",
        vec!["*c"; 2000].join(", ")
    );
    assert!(matches!(import(&flat), Err(ReadError::Syntax(m)) if m.contains("budget")));
}
