//! Edge cases of each dialect: sections, prefixes, hostile input, and the
//! auto-detector's confusions between formats.

use baylee_deckio::format::{self, detect};
use baylee_deckio::{Document, FormatId, Import, ReadError, Zone, export, import};
use std::fmt::Write as _;

fn read(id: FormatId, text: &str) -> Result<baylee_deckio::Read, ReadError> {
    format::read(id, text)
}

fn names(doc: &Document, zone: Zone) -> Vec<String> {
    doc.zone(zone).map(|c| c.name.clone()).collect()
}

// --- Baylee text -----------------------------------------------------------

#[test]
fn baylee_header_fields_set_name_and_format_and_empty_ones_do_not() {
    let r = read(
        FormatId::Baylee,
        "# baylee deck export v1\n# name: Mono\n# format: commander\n1 Island\n",
    )
    .unwrap();
    assert_eq!(r.document.name.as_deref(), Some("Mono"));
    assert_eq!(r.document.format.as_deref(), Some("commander"));
    let r = read(FormatId::Baylee, "# name:\n# format:   \n1 Island\n").unwrap();
    assert_eq!(r.document.name, None);
    assert_eq!(r.document.format, None);
}

#[test]
fn baylee_zone_prefixes_are_case_insensitive_and_override_the_section() {
    let r = read(
        FormatId::Baylee,
        "cmd: 1 Atraxa, Praetors' Voice\nsb: 1 Karakas\nMB: 1 Sol Ring\n1 Island\n",
    )
    .unwrap();
    assert_eq!(names(&r.document, Zone::Commander).len(), 1);
    assert_eq!(names(&r.document, Zone::Side), ["Karakas"]);
    assert_eq!(names(&r.document, Zone::Maybe), ["Sol Ring"]);
    assert_eq!(names(&r.document, Zone::Main), ["Island"]);
}

#[test]
fn baylee_does_not_treat_a_card_colon_name_as_a_prefix() {
    // "Circle of Protection: Red" has a colon and is a card.
    let r = read(FormatId::Baylee, "1 Circle of Protection: Red\n").unwrap();
    assert_eq!(
        names(&r.document, Zone::Main),
        ["Circle of Protection: Red"]
    );
}

#[test]
fn baylee_reports_a_bad_line_with_its_number_and_keeps_the_rest() {
    let r = read(FormatId::Baylee, "1 Island\n\n!!! not a row\n2 Forest\n").unwrap();
    assert_eq!(r.document.cards.len(), 2);
    assert_eq!(r.skipped.len(), 1);
    assert_eq!(r.skipped[0].line, 3);
}

#[test]
fn text_with_no_row_at_all_names_the_first_line_it_could_not_read() {
    let err = read(FormatId::Baylee, "hello there\n").unwrap_err();
    assert!(
        matches!(err, ReadError::Unreadable(ref s) if s.line == 1),
        "{err}"
    );
    assert!(err.to_string().contains("hello there"));
}

#[test]
fn comments_and_blank_lines_alone_are_an_empty_deck() {
    let err = read(FormatId::Baylee, "# only\n\n# comments\n").unwrap_err();
    assert_eq!(err, ReadError::Empty);
}

#[test]
fn baylee_crlf_line_endings_read_like_lf() {
    let r = read(FormatId::Baylee, "1 Island\r\nSB: 1 Karakas\r\n").unwrap();
    assert_eq!(r.document.cards.len(), 2);
    assert!(r.skipped.is_empty());
}

#[test]
fn baylee_text_round_trips_through_its_own_writer() {
    let src = "# baylee deck export v1\n# name: X\nCMD: 1 Atraxa\n3 Island\nSB: 1 Karakas\nMB: 1 Sol Ring\n";
    let doc = read(FormatId::Baylee, src).unwrap().document;
    let out = export(FormatId::Baylee, &doc);
    assert!(out.losses.is_empty());
    let again = read(FormatId::Baylee, &out.text).unwrap().document;
    assert_eq!(again, doc);
}

#[test]
fn the_baylee_magic_line_wins_over_everything_else_in_detection() {
    // Even a text whose later lines look like JSON-ish or Moxfield-ish.
    let text = "# Baylee Deck Export V1\n1x Island\n1 A / B\n";
    assert_eq!(detect(text), Some(FormatId::Baylee));
}

// --- Moxfield --------------------------------------------------------------

#[test]
fn moxfield_reads_x_counts_and_arena_sections() {
    let r = read(
        FormatId::Moxfield,
        "About\nName Hello Deck\n\nDeck\n4x Island\n\nSideboard\n1 Karakas\n",
    )
    .unwrap();
    assert_eq!(r.document.name.as_deref(), Some("Hello Deck"));
    assert_eq!(r.document.zone(Zone::Main).next().unwrap().count, 4);
    assert_eq!(names(&r.document, Zone::Side), ["Karakas"]);
}

#[test]
fn moxfield_commander_section_ends_at_the_blank_line() {
    let r = read(
        FormatId::Moxfield,
        "COMMANDER:\n1 Atraxa, Praetors' Voice\n\n1 Island\n",
    )
    .unwrap();
    assert_eq!(names(&r.document, Zone::Commander).len(), 1);
    assert_eq!(names(&r.document, Zone::Main), ["Island"]);
}

#[test]
fn moxfield_sideboard_runs_to_the_end_across_blank_lines() {
    let r = read(FormatId::Moxfield, "SIDEBOARD:\n1 A\n\n1 B\n").unwrap();
    assert_eq!(names(&r.document, Zone::Side), ["A", "B"]);
}

#[test]
fn moxfield_header_words_are_forgiving_about_case_colon_and_slashes() {
    for header in [
        "SIDEBOARD:",
        "Sideboard",
        "// Sideboard",
        "side",
        "  sideboard  ",
    ] {
        let r = read(
            FormatId::Moxfield,
            &format!("1 Island\n{header}\n1 Karakas\n"),
        )
        .unwrap();
        assert_eq!(names(&r.document, Zone::Side), ["Karakas"], "{header:?}");
    }
}

#[test]
fn moxfield_only_the_first_single_slash_is_the_face_separator() {
    let r = read(FormatId::Moxfield, "1 Fire / Ice\n").unwrap();
    assert_eq!(names(&r.document, Zone::Main), ["Fire // Ice"]);
}

#[test]
fn baylee_text_keeps_a_single_slash_as_part_of_the_name() {
    // Only Moxfield's dialect rewrites " / "; the Baylee reader must not.
    let r = read(FormatId::Baylee, "1 Fire / Ice\n").unwrap();
    assert_eq!(names(&r.document, Zone::Main), ["Fire / Ice"]);
}

#[test]
fn moxfield_writer_puts_the_commander_first_and_the_sideboard_last() {
    let doc = read(FormatId::Baylee, "SB: 1 Karakas\n1 Island\nCMD: 1 Atraxa\n")
        .unwrap()
        .document;
    let out = export(FormatId::Moxfield, &doc).text;
    let at = |s: &str| out.find(s).unwrap_or_else(|| panic!("{s} in {out}"));
    assert!(at("COMMANDER:") < at("Island"));
    assert!(at("Island") < at("SIDEBOARD:"));
    assert!(at("SIDEBOARD:") < at("Karakas"));
}

#[test]
fn moxfield_writer_writes_a_double_faced_card_with_one_slash() {
    let doc = read(FormatId::Baylee, "1 Fire // Ice\n").unwrap().document;
    let out = export(FormatId::Moxfield, &doc).text;
    assert!(out.contains("Fire / Ice") && !out.contains("//"), "{out}");
}

// --- JSON ------------------------------------------------------------------

#[test]
fn json_round_trips_and_writes_a_trailing_newline() {
    let doc = read(FormatId::Baylee, "# name: J\n2 Island\nSB: 1 Karakas\n")
        .unwrap()
        .document;
    let out = export(FormatId::Json, &doc);
    assert!(out.text.ends_with('\n'));
    assert!(out.losses.is_empty());
    assert_eq!(read(FormatId::Json, &out.text).unwrap().document, doc);
}

#[test]
fn json_that_is_not_a_document_is_a_syntax_error_in_the_parsers_words() {
    for bad in [
        "{",
        "{\"version\":1}",
        "{\"version\":1,\"cards\":\"x\"}",
        "{\"cards\":[]}",
    ] {
        let err = read(FormatId::Json, bad).unwrap_err();
        assert!(matches!(err, ReadError::Syntax(_)), "{bad}: {err:?}");
    }
}

#[test]
fn json_with_an_empty_card_list_is_an_empty_deck() {
    let err = read(FormatId::Json, r#"{"version":1,"cards":[]}"#).unwrap_err();
    assert_eq!(err, ReadError::Empty);
}

#[test]
fn json_with_a_zero_count_or_blank_name_is_refused_as_a_card() {
    for card in [
        r#"{"zone":"Main","count":0,"name":"Island"}"#,
        r#"{"zone":"Main","count":1,"name":""}"#,
    ] {
        let text = format!(r#"{{"version":1,"cards":[{card}]}}"#);
        let err = format::read(FormatId::Json, &text);
        assert!(err.is_err(), "{card} was taken");
    }
}

#[test]
fn json_surrounded_by_whitespace_is_still_detected_as_json() {
    let text = "\n  \t{\"version\":1,\"cards\":[]}";
    assert_eq!(detect(text), Some(FormatId::Json));
}

// --- YAML ------------------------------------------------------------------

#[test]
fn yaml_round_trips_a_document() {
    let doc = read(FormatId::Baylee, "# name: Y\n2 Island\nMB: 1 Sol Ring\n")
        .unwrap()
        .document;
    let out = export(FormatId::Yaml, &doc);
    assert!(out.losses.is_empty());
    assert_eq!(read(FormatId::Yaml, &out.text).unwrap().document, doc);
}

#[test]
fn yaml_scalar_text_is_a_syntax_error_not_a_deck() {
    let err = read(FormatId::Yaml, "just a string\n").unwrap_err();
    assert!(matches!(err, ReadError::Syntax(_)), "{err:?}");
}

#[test]
fn yaml_is_detected_by_its_shape_and_a_plain_list_is_not() {
    assert_eq!(detect("---\nversion: 1\ncards: []\n"), Some(FormatId::Yaml));
    assert_eq!(
        detect("# c\n\nversion: 1\ncards: []\n"),
        Some(FormatId::Yaml)
    );
    assert_ne!(detect("1 Island\n4 Forest\n"), Some(FormatId::Yaml));
}

// --- Detection and bounds --------------------------------------------------

#[test]
fn nothing_but_whitespace_detects_as_nothing_and_imports_as_empty() {
    assert_eq!(detect(" \n\t\n"), None);
    assert_eq!(import("   \n").unwrap_err(), ReadError::Empty);
}

#[test]
fn a_plain_list_with_x_counts_is_moxfield_and_a_prefixed_one_is_baylee() {
    assert_eq!(detect("4x Island\n1 Karakas\n"), Some(FormatId::Moxfield));
    assert_eq!(detect("SB: 1 Karakas\n1 Island\n"), Some(FormatId::Baylee));
}

#[test]
fn a_double_slash_card_in_a_plain_list_is_not_pushed_to_moxfield() {
    // "A // B" has no single-slash face, so it is nobody's tell.
    assert_ne!(detect("1 Fire // Ice\n"), Some(FormatId::Moxfield));
}

#[test]
fn a_deck_made_of_one_line_over_the_row_limit_is_refused_not_truncated() {
    let mut text = String::new();
    for i in 0..1_200 {
        let _ = writeln!(text, "1 Card {i}");
    }
    let err = format::read(FormatId::Baylee, &text).unwrap_err();
    assert!(
        matches!(err, ReadError::TooManyRows(_)),
        "a deck over the cap must not be read short: {err:?}"
    );
}

#[test]
fn an_import_of_text_at_the_byte_cap_is_read_and_one_byte_over_is_refused() {
    let line = "1 Island\n";
    let mut ok = line.repeat(100);
    ok.push_str(&"# ".repeat(10));
    assert!(import(&ok).is_ok());
    let big = "#".repeat(baylee_deckio::MAX_DOCUMENT_BYTES + 1);
    assert!(matches!(
        import(&big).unwrap_err(),
        ReadError::TooLarge { bytes } if bytes == baylee_deckio::MAX_DOCUMENT_BYTES + 1
    ));
}

#[test]
fn an_import_of_a_deck_says_which_format_read_it() {
    match import("1 Island\n").unwrap() {
        Import::Read { format, read } => {
            assert!(matches!(format, FormatId::Baylee | FormatId::Moxfield));
            assert_eq!(read.document.cards.len(), 1);
        }
        other => panic!("{other:?}"),
    }
}
