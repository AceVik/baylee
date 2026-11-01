use super::*;
use crate::narrator::Decision;
use crate::narrator::tests::board;

pub(crate) mod log;

fn filed(what: &str, question: u64) -> Filed {
    Filed {
        game_id: "TEST-game".into(),
        seat: 1,
        question,
        turn: 4,
        model: "TEST-model".into(),
        profile: Some("collector".into()),
        report: Report {
            what: what.into(),
            expected: Some("It deals 3 damage to any target.".into()),
            evidence: Some("[Lightning Bolt]'s Oracle text".into()),
            references: vec!["[Lightning Bolt]".into(), "702.19b".into()],
        },
        lookups: 1,
        unix: 1_700_000_000,
        seen: Vec::new(),
        log_lines: 0,
    }
}

/// A scratch directory of this test's own.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("baylee-seat-reports-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_report_is_an_object_or_a_plain_string_and_never_empty() {
    let full = Report::read(&json!({
        "what": "  Bolt dealt 2 damage. ",
        "expected": "3 damage",
        "evidence": "",
        "references": ["[Lightning Bolt]", " ", "702.19b"],
    }))
    .unwrap();
    assert_eq!(full.what, "Bolt dealt 2 damage.");
    assert_eq!(full.expected.as_deref(), Some("3 damage"));
    assert_eq!(full.evidence, None, "a blank field is no field");
    assert_eq!(full.references, ["[Lightning Bolt]", "702.19b"]);
    let plain = Report::read(&json!("The trigger never fired.")).unwrap();
    assert_eq!(plain.what, "The trigger never fired.");
    assert_eq!(plain.expected, None);
    // A strict schema's nulls, a lone reference, and nothing at all.
    let strict = Report::read(&json!({
        "what": "x", "expected": null, "evidence": null, "references": "702.19b"
    }))
    .unwrap();
    assert_eq!(
        (strict.expected, strict.references),
        (None, vec!["702.19b".to_string()])
    );
    for nothing in [
        json!(null),
        json!(""),
        json!({"what": " "}),
        json!({"expected": "y"}),
        json!(3),
    ] {
        assert_eq!(Report::read(&nothing), None, "{nothing}");
    }
    let long = Report::read(&json!("w".repeat(FIELD_CHARS + 50))).unwrap();
    assert_eq!(
        long.what.chars().count(),
        FIELD_CHARS + 1,
        "held, with an ellipsis"
    );
}

/// `report` and `lookup` are read in both answer shapes; a concession
/// stays the concession it was.
#[test]
fn a_decision_carries_a_report_and_a_lookup() {
    let input = json!({
        "ask": "q12", "pick": ["a1"],
        "report": {"what": "Wrong mana", "references": ["[Island]"]},
        "lookup": " Lightning Bolt ",
    });
    for decision in [
        Decision::from_decide(&input).unwrap(),
        Decision::from_json_answer(&input).unwrap(),
    ] {
        assert_eq!(decision.report.as_ref().unwrap().what, "Wrong mana");
        assert_eq!(decision.lookup.as_deref(), Some("Lightning Bolt"));
        assert_eq!(decision.pick, ["a1"]);
    }
    let plain = Decision::from_decide(&json!({"ask": "q3", "report": "No trigger"})).unwrap();
    assert_eq!(plain.report.unwrap().what, "No trigger");
    let none = Decision::from_decide(&json!({"ask": "q3", "report": null, "lookup": ""})).unwrap();
    assert_eq!((none.report, none.lookup), (None, None));
    let conceded = Decision::from_json_answer(&json!({
        "ask": "q9", "concede": "Lethal on board.", "report": "x", "lookup": "y"
    }))
    .unwrap();
    assert_eq!(conceded.concede.as_deref(), Some("Lethal on board."));
    assert_eq!((conceded.report, conceded.lookup), (None, None));
}

#[test]
fn a_book_files_each_what_once_and_at_most_its_most() {
    let dir = scratch("book");
    let mut book = Book::default();
    assert!(book.file(filed("Bolt dealt 2.", 12), Some(&dir)));
    assert!(
        !book.file(filed("  bolt   DEALT 2. ", 13), Some(&dir)),
        "the same what"
    );
    for n in 0..40 {
        book.file(filed(&format!("report {n}"), 20 + n), Some(&dir));
    }
    let taken = book.take();
    assert_eq!(taken.len(), MOST);
    assert!(book.take().is_empty(), "taken once");
    assert!(
        !book.file(filed("one more", 99), Some(&dir)),
        "the game's are filed"
    );
    let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
    let lines: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), MOST);
    let first = &lines[0];
    let mut keys: Vec<&str> = first
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "evidence",
            "expected",
            "game_id",
            "lookups",
            "model",
            "profile",
            "question",
            "references",
            "seat",
            "turn",
            "unix",
            "what"
        ]
    );
    assert_eq!(first["game_id"], "TEST-game");
    assert_eq!(first["question"], 12);
    assert_eq!(first["model"], "TEST-model");
    assert_eq!(first["lookups"], 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The cards a report names are the ones the seat's own view shows it.
#[test]
fn a_report_names_the_cards_the_seat_was_shown() {
    let (view, _) = board();
    let report = Report {
        what: "[Lightning Bolt] could not target [Serra Angel].".into(),
        references: vec!["[Black Lotus]".into()],
        ..Report::default()
    };
    let seen = seen_in(&report, &view);
    let names: Vec<&str> = seen.iter().map(|card| card.english.as_str()).collect();
    assert!(names.contains(&"Lightning Bolt"), "{names:?}");
    assert!(names.contains(&"Serra Angel"), "{names:?}");
    assert!(!names.contains(&"Black Lotus"), "{names:?}");
    let bolt = seen.iter().find(|c| c.english == "Lightning Bolt").unwrap();
    assert_eq!(bolt.object, Some(crate::narrator::tests::id(50)));
}

#[test]
fn a_game_s_reports_are_one_bug_report_marked_as_a_model_s() {
    let mut reports = vec![filed("Bolt dealt 2.", 12), filed("bolt dealt 2.", 14)];
    reports[1].report.references.clear();
    reports.push(filed("The trigger never fired.", 15));
    let body = submission(&reports, None).unwrap();
    assert_eq!(body["kind"], "bug");
    assert_eq!(body["game_id"], "TEST-game");
    assert!(body.get("local_record").is_none());
    let client = &body["client"];
    assert!(
        client.get("log").is_none() && client.get("log_omitted").is_none(),
        "no log was told, none is sent"
    );
    assert!(client["reports"][0].get("log_line").is_none());
    assert_eq!(client["source"], "llm-seat");
    assert_eq!(client["model"], "TEST-model");
    assert_eq!(client["profile"], "collector");
    assert_eq!(client["seat"], 1);
    let filed = client["reports"].as_array().unwrap();
    assert_eq!(filed.len(), 2, "each what once");
    assert_eq!(filed[0]["question"], 12);
    let text = body["text"].as_str().unwrap();
    assert!(
        text.starts_with("Untrusted: reports written by a language model"),
        "{text}"
    );
    assert!(text.contains("1. q12, turn 4: Bolt dealt 2."), "{text}");
    assert!(
        text.contains("References: [Lightning Bolt], 702.19b"),
        "{text}"
    );
    assert!(
        text.contains("2. q15, turn 4: The trigger never fired."),
        "{text}"
    );
    // The card named in brackets is a reference to this build's pool's
    // card, where it stands in the text.
    let cards = client["refs"]["cards"].as_array().unwrap();
    assert!(!cards.is_empty());
    let bolt = baylee_cards::decks::by_name("Lightning Bolt").unwrap();
    let chars: Vec<char> = text.chars().collect();
    for card in cards {
        assert_eq!(card["card"], bolt.get());
        assert_eq!(card["text"], "Lightning Bolt");
        let at = card["at"].as_array().unwrap();
        let (start, end) = (
            usize::try_from(at[0].as_u64().unwrap()).unwrap(),
            usize::try_from(at[1].as_u64().unwrap()).unwrap(),
        );
        assert_eq!(
            chars[start..end].iter().collect::<String>(),
            "[Lightning Bolt]"
        );
        assert!(card.get("object").is_none(), "not from a view");
    }
    assert_eq!(submission(&[], None), None, "no reports, no report");
}

/// The text names the seat as its log and its model do (`P2` for the
/// view's seat 1), and every mind that filed a report, in order: after a
/// fallback took over, `client.model` is the first's, and each report
/// names its own.
#[test]
fn a_bug_report_names_the_seat_as_its_log_does_and_every_mind() {
    let mut reports = vec![filed("Bolt dealt 2.", 12), filed("No trigger.", 14)];
    reports[1].model = "TEST-fallback".into();
    reports[1].profile = Some("backup".into());
    reports.push(filed("Wrong mana.", 15));
    let body = submission(&reports, None).unwrap();
    let text = body["text"].as_str().unwrap();
    assert!(
        text.starts_with(
            "Untrusted: reports written by a language model while it played P2 (model \
             TEST-model, profile collector; then model TEST-fallback, profile backup)."
        ),
        "{text}"
    );
    let client = &body["client"];
    assert_eq!(client["model"], "TEST-model");
    assert_eq!(client["reports"][1]["model"], "TEST-fallback");
    assert_eq!(client["reports"][1]["profile"], "backup");
    assert_eq!(client["seat"], 1, "the view's number, as each report's");
}

#[test]
fn a_bug_report_s_text_holds_to_its_bound() {
    let long = "x".repeat(FIELD_CHARS);
    let reports: Vec<Filed> = (0..MOST)
        .map(|n| {
            let mut f = filed(&format!("{n} {long}"), n as u64);
            f.report.expected = Some(long.clone());
            f.report.evidence = Some(long.clone());
            f
        })
        .collect();
    let body = submission(&reports, None).unwrap();
    let text = body["text"].as_str().unwrap();
    assert!(
        text.chars().count() <= TEXT_CHARS,
        "{}",
        text.chars().count()
    );
    assert!(text.contains("more reports are in client.reports"), "said");
    assert_eq!(
        body["client"]["reports"].as_array().unwrap().len(),
        MOST,
        "all kept whole"
    );
}
