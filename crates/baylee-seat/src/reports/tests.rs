use super::*;
use crate::lookup::fold;
use crate::narrator::tests::board;
use crate::narrator::{Decision, Offered};
use baylee_view::{Phase, Step};

pub(crate) mod log;

/// Lightning Bolt in this build's pool.
fn bolt() -> CardIndex {
    baylee_cards::decks::by_name("Lightning Bolt").expect("a pool card")
}

/// Lightning Bolt's text as the narrator tells it to a model.
fn bolt_text() -> String {
    crate::narrator::card_text(bolt(), 0).expect("its text")
}

/// The first line of Lightning Bolt's own text in this build, as the model
/// is told it: a quote the seat can find, never written out here.
pub(crate) fn bolt_quote() -> String {
    let oracle = baylee_cards::oracle::face(bolt(), 0).expect("its oracle");
    let line = oracle.lines().find(|l| !l.trim().is_empty()).unwrap();
    crate::narrator::strip_reminder(line)
}

/// What the test seat's model was told of a card's text: Lightning
/// Bolt's, and nothing else.
pub(crate) fn told(name: &str) -> Vec<String> {
    if fold(name) == "lightning bolt" {
        vec![bolt_text()]
    } else {
        Vec::new()
    }
}

/// A priority question of turn 4, the seat's first main phase, offering a
/// cast and a pass, answered with the cast.
pub(crate) fn moment() -> Moment {
    Moment {
        turn: 4,
        phase: Some(Phase::FirstMain),
        step: Some(Step::Main),
        pending: "Priority".into(),
        options: vec![
            Offered {
                id: "a1".into(),
                text: "Cast Lightning Bolt #50 (taps Mountain #23)".into(),
            },
            Offered {
                id: "p".into(),
                text: "Pass priority".into(),
            },
        ],
        answer: Some("a1 Cast Lightning Bolt #50 (taps Mountain #23)".into()),
        seq: 77,
        policy_chose: None,
        model_would: None,
    }
}

/// `report`, filed by the test seat at `question`, checked against
/// [`told`] at a priority question.
pub(crate) fn filed_with(report: Report, question: u64) -> Filed {
    let checked = Checked::new(&report, "Priority", told);
    Filed {
        source: Source::LlmSeat,
        game_id: "TEST-game".into(),
        seat: 1,
        question,
        model: "TEST-model".into(),
        profile: Some("collector".into()),
        report,
        checked,
        moment: moment(),
        lookups: 1,
        unix: 1_700_000_000,
        seen: Vec::new(),
        log_lines: 0,
    }
}

/// A report that quotes Lightning Bolt's text as the model was told it and
/// cites `rule`, saying `observed`.
pub(crate) fn report(observed: &str, rule: &str) -> Report {
    Report {
        kind: Kind::WrongOutcome,
        severity: Some(Severity::GameChanging),
        confidence: Some(Confidence::High),
        cards: vec![Card {
            name: "Lightning Bolt".into(),
            oracle_quote: Some(bolt_quote()),
        }],
        rules: vec![rule.into()],
        observed: observed.into(),
        expected: Some("It deals the damage its text says.".into()),
        evidence: None,
        references: Vec::new(),
    }
}

/// [`report`] citing rule 120.3, filed at `question`.
pub(crate) fn filed(observed: &str, question: u64) -> Filed {
    filed_with(report(observed, "120.3"), question)
}

/// The `n`th of many reports, each its own dedupe key (its own rule).
fn distinct(n: u64) -> Filed {
    filed_with(report(&format!("report {n}"), &format!("{}.1", 100 + n)), n)
}

/// A scratch directory of this test's own.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("baylee-seat-reports-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The keys of a JSON object, sorted.
pub(crate) fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

/// Every field of a report is read, spelt as the model may spell it, and
/// held to its bound: names and rule numbers bare, quotes without their
/// quotation marks, the cards and the rules past eight dropped.
#[test]
fn a_report_reads_its_fields_and_holds_them_to_their_bounds() {
    let cards: Vec<Value> = (0..12)
        .map(|n| json!({"name": format!("[Card {n}]"), "oracle_quote": format!("“words {n}”")}))
        .collect();
    let full = Report::read(&json!({
        "kind": " Card-Text Mismatch ",
        "severity": "GAME CHANGING",
        "confidence": "medium",
        "cards": cards,
        "rules": ["CR 702.19b.", 601.2, " 117.3A ", "702.19b", "", 1, 2, 3, 4, 5, 6, 7],
        "observed": "  Bolt dealt 2 damage. ",
        "expected": "3 damage",
        "references": ["https://scryfall.com/card/x", " "],
    }))
    .unwrap();
    assert_eq!(full.kind, Kind::CardTextMismatch);
    assert_eq!(full.severity, Some(Severity::GameChanging));
    assert_eq!(full.confidence, Some(Confidence::Medium));
    assert_eq!(full.cards.len(), MOST_CARDS, "the cards past eight dropped");
    assert_eq!(full.cards[0].name, "Card 0", "brackets off");
    assert_eq!(full.cards[0].oracle_quote.as_deref(), Some("words 0"));
    assert_eq!(
        full.rules,
        ["702.19b", "601.2", "117.3a", "1", "2", "3", "4", "5"],
        "each once, at most eight, as written less CR and the full stop"
    );
    assert_eq!(full.observed, "Bolt dealt 2 damage.");
    assert_eq!(full.expected.as_deref(), Some("3 damage"));
    assert_eq!(full.references, ["https://scryfall.com/card/x"]);
    assert_eq!(full.evidence, None);
    // Too long: cut, a quote and a name without a mark, free text with one.
    let long = Report::read(&json!({
        "kind": "rules_error",
        "cards": [{"name": "n".repeat(500), "oracle_quote": format!("\"{}\"", "q".repeat(900))}],
        "observed": "o".repeat(FIELD_CHARS + 50),
    }))
    .unwrap();
    assert_eq!(long.cards[0].name.chars().count(), NAME_CHARS);
    let quote = long.cards[0].oracle_quote.as_deref().unwrap();
    assert_eq!(quote, "q".repeat(QUOTE_CHARS));
    assert_eq!(long.observed.chars().count(), FIELD_CHARS + 1);
    assert!(long.observed.ends_with('…'));
    // A lone card and a lone rule where lists were asked for; a card that
    // is only a name; control characters left out.
    let lone = Report::read(&json!({
        "cards": "Shock", "rules": "CR 120.3", "observed": "a\u{0007}b\nc"
    }))
    .unwrap();
    assert_eq!(
        lone.cards,
        [Card {
            name: "Shock".into(),
            oracle_quote: None
        }]
    );
    assert_eq!(lone.rules, ["120.3"]);
    assert_eq!(lone.observed, "ab\nc");
}

/// An enum value the reader does not know is read, never refused: a kind
/// as `other` (a model's `unverified` too: only the seat says that), a
/// severity or a confidence as not said.
#[test]
fn unknown_enum_values_are_other_or_unsaid() {
    let read = Report::read(&json!({
        "kind": "catastrophic_bug", "severity": "huge", "confidence": 0.9, "observed": "x"
    }))
    .unwrap();
    assert_eq!(
        (read.kind, read.severity, read.confidence),
        (Kind::Other, None, None)
    );
    let unverified = Report::read(&json!({"kind": "unverified", "observed": "x"})).unwrap();
    assert_eq!(unverified.kind, Kind::Other);
    for (written, kind) in Kind::MODEL.iter().map(|k| (k.as_str(), *k)) {
        let read = Report::read(&json!({"kind": written, "observed": "x"})).unwrap();
        assert_eq!(read.kind, kind, "{written}");
    }
}

/// An answer of the shape before 01.11.2026 is still read: `what` as
/// `observed`, its evidence and references kept; a plain string is what
/// it observed; and a report that says nothing is none.
#[test]
fn a_report_of_the_old_shape_is_still_read() {
    let old = Report::read(&json!({
        "what": "  Bolt dealt 2 damage. ",
        "expected": "3 damage",
        "evidence": "its text, looked up at q21",
        "references": "702.19b",
    }))
    .unwrap();
    assert_eq!(old.observed, "Bolt dealt 2 damage.");
    assert_eq!(old.kind, Kind::Other);
    assert_eq!(old.evidence.as_deref(), Some("its text, looked up at q21"));
    assert_eq!(old.references, ["702.19b"]);
    assert!(old.cards.is_empty() && old.rules.is_empty());
    let both = Report::read(&json!({"what": "old", "observed": "new"})).unwrap();
    assert_eq!(both.observed, "new", "observed before what");
    let plain = Report::read(&json!("The trigger never fired.")).unwrap();
    assert_eq!(plain.observed, "The trigger never fired.");
    assert_eq!(plain.expected, None);
    for nothing in [
        json!(null),
        json!(""),
        json!({"what": " "}),
        json!({"expected": "y"}),
        json!({"kind": "rules_error", "rules": ["120.3"], "observed": null}),
        json!(3),
    ] {
        assert_eq!(Report::read(&nothing), None, "{nothing}");
    }
}

/// `report` and `lookup` are read in both answer shapes; a concession
/// stays the concession it was.
#[test]
fn a_decision_carries_a_report_and_a_lookup() {
    let input = json!({
        "ask": "q12", "pick": ["a1"],
        "report": {"kind": "wrong_outcome", "observed": "Wrong mana", "cards": [{"name": "Island"}]},
        "lookup": " Lightning Bolt ",
    });
    for decision in [
        Decision::from_decide(&input).unwrap(),
        Decision::from_json_answer(&input).unwrap(),
    ] {
        let report = decision.report.as_ref().unwrap();
        assert_eq!(report.observed, "Wrong mana");
        assert_eq!(report.cards[0].name, "Island");
        assert_eq!(decision.lookup.as_deref(), Some("Lightning Bolt"));
        assert_eq!(decision.pick, ["a1"]);
    }
    let plain = Decision::from_decide(&json!({"ask": "q3", "report": "No trigger"})).unwrap();
    assert_eq!(plain.report.unwrap().observed, "No trigger");
    let none = Decision::from_decide(&json!({"ask": "q3", "report": null, "lookup": ""})).unwrap();
    assert_eq!((none.report, none.lookup), (None, None));
    let conceded = Decision::from_json_answer(&json!({
        "ask": "q9", "concede": "Lethal on board.", "report": "x", "lookup": "y"
    }))
    .unwrap();
    assert_eq!(conceded.concede.as_deref(), Some("Lethal on board."));
    assert_eq!((conceded.report, conceded.lookup), (None, None));
}

/// A quote matches only where it is words of the text the seat told the
/// model for that card, spacing aside: a word changed, a card it was not
/// told, or one quote of two that does not match, and it does not.
#[test]
fn quotes_are_checked_against_the_card_text_the_model_was_told() {
    let check = |cards: Vec<Card>| {
        let report = Report {
            cards,
            observed: "x".into(),
            ..Report::default()
        };
        Checked::new(&report, "Priority", told).oracle_quote_matches
    };
    let quote = bolt_quote();
    let card = |name: &str, quote: Option<String>| Card {
        name: name.into(),
        oracle_quote: quote,
    };
    let spaced = format!("\n  {}  ", quote.replace(' ', "  \n "));
    assert_eq!(
        check(vec![card("lightning bolt", Some(spaced))]),
        Some(true)
    );
    assert_eq!(
        check(vec![card("Lightning Bolt", Some(format!("{quote} twice")))]),
        Some(false),
        "a word that is not there"
    );
    assert_eq!(
        check(vec![card("Shock", Some(quote.clone()))]),
        Some(false),
        "Bolt's words, but not the card the model named"
    );
    assert_eq!(
        check(vec![
            card("Lightning Bolt", Some(quote.clone())),
            card("Lightning Bolt", Some("not its words".into())),
        ]),
        Some(false),
        "every quote must match"
    );
    assert_eq!(
        check(vec![
            card("Lightning Bolt", Some(quote.clone())),
            card("Shock", None),
        ]),
        Some(true),
        "a card named without a quote is not checked"
    );
    assert_eq!(check(vec![card("Lightning Bolt", None)]), None, "no quote");
    assert_eq!(check(Vec::new()), None);
}

/// A report that quotes no card's text and cites no rule number is
/// `unverified`, whatever kind the model gave it; either one keeps the
/// model's kind.
#[test]
fn a_report_with_neither_quote_nor_rule_is_unverified() {
    let kind = |cards: Vec<Card>, rules: Vec<String>| {
        let report = Report {
            kind: Kind::RulesError,
            cards,
            rules,
            observed: "x".into(),
            ..Report::default()
        };
        Checked::new(&report, "Priority", told).kind
    };
    let named = Card {
        name: "Lightning Bolt".into(),
        oracle_quote: None,
    };
    let quoted = Card {
        oracle_quote: Some(bolt_quote()),
        ..named.clone()
    };
    assert_eq!(kind(Vec::new(), Vec::new()), Kind::Unverified);
    assert_eq!(
        kind(vec![named], Vec::new()),
        Kind::Unverified,
        "a card's name is no quote"
    );
    assert_eq!(kind(vec![quoted], Vec::new()), Kind::RulesError);
    assert_eq!(kind(Vec::new(), vec!["601.2".into()]), Kind::RulesError);
    assert_eq!(
        kind(Vec::new(), vec!["not a rule".into()]),
        Kind::RulesError,
        "cited, if badly: rules_format_ok says so"
    );
    let old = Report::read(&json!({"what": "x", "evidence": "702.19b"})).unwrap();
    assert_eq!(
        Checked::new(&old, "Priority", told).kind,
        Kind::Unverified,
        "the old shape cites in free text, which the seat does not read"
    );
}

/// A rule number is three digits, then a full stop and up to three, then
/// a letter; `rules_format_ok` says whether every one cited is.
#[test]
fn rule_numbers_are_checked_for_their_form() {
    for good in ["601", "601.2", "702.19b", "117.3a", "100.1", "903.10a"] {
        assert!(rule_number(good), "{good}");
    }
    for bad in [
        "",
        "60",
        "6012",
        "060.1",
        "601.",
        "601.2bc",
        "601.1234",
        "702.B",
        "rule 601",
        "601.2 b",
        "７０２.１",
        "§601.2",
    ] {
        assert!(!rule_number(bad), "{bad}");
    }
    let ok = |rules: &[&str]| {
        let report = Report {
            rules: rules.iter().map(ToString::to_string).collect(),
            observed: "x".into(),
            ..Report::default()
        };
        Checked::new(&report, "Priority", told).rules_format_ok
    };
    assert_eq!(ok(&["702.19b", "601.2"]), Some(true));
    assert_eq!(ok(&["702.19b", "the stack rule"]), Some(false));
    assert_eq!(ok(&[]), None, "none cited");
}

/// Two reports are the same where their kind, their cards' names (case,
/// spacing and order aside), their rule numbers (order aside) and the
/// question's kind are: what they say in words does not count.
#[test]
fn the_dedupe_key_folds_order_case_and_spacing() {
    let key = |kind, cards: &[&str], rules: &[&str], pending| {
        let rules: Vec<String> = rules.iter().map(ToString::to_string).collect();
        dedupe_key(kind, cards, &rules, pending)
    };
    let one = key(
        Kind::RulesError,
        &["Lightning Bolt", "Shock"],
        &["601.2", "702.19b"],
        "Priority",
    );
    assert_eq!(one.len(), 64);
    assert!(
        one.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    assert_eq!(
        one,
        key(
            Kind::RulesError,
            &["shock", " LIGHTNING   bolt", "Shock"],
            &["702.19b", "601.2"],
            "Priority"
        )
    );
    for other in [
        key(
            Kind::WrongOutcome,
            &["Lightning Bolt", "Shock"],
            &["601.2", "702.19b"],
            "Priority",
        ),
        key(
            Kind::RulesError,
            &["Lightning Bolt"],
            &["601.2", "702.19b"],
            "Priority",
        ),
        key(
            Kind::RulesError,
            &["Lightning Bolt", "Shock"],
            &["601.2"],
            "Priority",
        ),
        key(
            Kind::RulesError,
            &["Lightning Bolt", "Shock"],
            &["601.2", "702.19b"],
            "ChooseTargets",
        ),
    ] {
        assert_ne!(one, other);
    }
    let a = filed("Bolt dealt 2.", 12);
    let b = filed("Something else entirely.", 13);
    assert_eq!(a.checked.dedupe_key, b.checked.dedupe_key, "words aside");
}

/// The reports file is the account's alone whatever the umask: the file
/// `0600`, every directory made for it `0700`, one an older build left
/// readable by others narrowed at the next line, and a directory that was
/// there already left as it was (`docs/privacy.md`).
#[cfg(unix)]
#[test]
fn the_reports_file_is_readable_by_its_account_alone() {
    use std::os::unix::fs::PermissionsExt as _;
    let mode =
        |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    let root = scratch("private");
    let dir = root.join("made").join("for-it");
    append(&dir, &filed("Bolt dealt 2.", 12)).unwrap();
    assert_eq!(mode(&dir.join(FILE)), 0o600, "the file");
    assert_eq!(mode(&dir), 0o700, "the directory made for it");
    assert_eq!(mode(&root.join("made")), 0o700, "and its parent");

    // A file and a directory that were there already, open to all.
    let shared = root.join("shared");
    std::fs::create_dir(&shared).unwrap();
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(shared.join(FILE), "").unwrap();
    std::fs::set_permissions(shared.join(FILE), std::fs::Permissions::from_mode(0o644)).unwrap();
    append(&shared, &filed("Bolt dealt 2.", 12)).unwrap();
    assert_eq!(mode(&shared.join(FILE)), 0o600, "an older file is narrowed");
    assert_eq!(mode(&shared), 0o755, "the operator's directory is theirs");
    let text = std::fs::read_to_string(shared.join(FILE)).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_book_files_each_dedupe_key_once_and_at_most_its_most() {
    let dir = scratch("book");
    let mut book = Book::default();
    assert!(book.file(filed("Bolt dealt 2.", 12), Some(&dir)));
    assert!(
        !book.file(filed("It dealt two, not three.", 13), Some(&dir)),
        "the same key"
    );
    for n in 0..40 {
        book.file(distinct(20 + n), Some(&dir));
    }
    let taken = book.take();
    assert_eq!(taken.len(), MOST);
    assert!(book.take().is_empty(), "taken once");
    assert!(!book.file(distinct(99), Some(&dir)), "the game's are filed");
    let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
    let lines: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), MOST);
    let first = &lines[0];
    assert_eq!(
        keys(first),
        [
            "cards",
            "confidence",
            "decision",
            "dedupe_key",
            "expected",
            "game_id",
            "kind",
            "lookups",
            "model",
            "observed",
            "oracle_quote_matches",
            "profile",
            "question",
            "references",
            "rules",
            "rules_format_ok",
            "seat",
            "severity",
            "source",
            "turn",
            "unix",
        ]
    );
    assert_eq!(first["game_id"], "TEST-game");
    assert_eq!(first["source"], "llm-seat");
    assert_eq!(first["question"], 12);
    assert_eq!(first["model"], "TEST-model");
    assert_eq!(first["kind"], "wrong_outcome");
    assert_eq!(first["oracle_quote_matches"], true);
    assert!(first["decision"].get("log_line").is_none(), "no log here");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The cards a report names are the ones the seat's own view shows it.
#[test]
fn a_report_names_the_cards_the_seat_was_shown() {
    let (view, _) = board();
    let report = Report {
        observed: "[Lightning Bolt] could not target the angel.".into(),
        cards: vec![Card {
            name: "Serra Angel".into(),
            oracle_quote: None,
        }],
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

/// The question a report was made at, as the seat showed it: its options
/// as the menu listed them, and the answer read as the seat reads it, or
/// why it could not be, or none for an answer that only looked a card up.
#[test]
fn a_report_s_question_is_the_one_the_seat_showed() {
    use crate::narrator::Narrator;
    use crate::narrator::tests::{priority, request};
    let (view, log) = board();
    let request = request(view.clone(), priority(&view), log);
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let answer = |pick: &[&str], lookup: Option<&str>| {
        let decision = Decision {
            ask: Some("q12".into()),
            pick: pick.iter().map(ToString::to_string).collect(),
            lookup: lookup.map(ToString::to_string),
            ..Decision::default()
        };
        Moment::of(&request, &wake.menu, &decision)
    };
    let taken = answer(&["a1"], None);
    assert_eq!(taken.turn, 7);
    assert_eq!(taken.phase, Some(view.phase));
    assert_eq!(taken.step, Some(view.step));
    assert_eq!(taken.pending, "Priority");
    assert_eq!(taken.seq, view.seq);
    assert_eq!(taken.options, wake.menu.offered());
    let a1 = &wake.menu.offered()[0];
    assert_eq!(
        taken.answer.as_deref(),
        Some(format!("{} {}", a1.id, a1.text).as_str())
    );
    let refused = answer(&["a99"], None).answer.unwrap();
    assert!(
        refused.starts_with("not taken: «a99» is not an option"),
        "{refused}"
    );
    assert_eq!(answer(&[], Some("Shock")).answer, None, "only a lookup");
    let pass = wake.menu.offered().iter().find(|o| o.id == "p").unwrap();
    assert_eq!(
        answer(&["p"], Some("Shock")).answer,
        Some(format!("p {}", pass.text)),
        "a lookup beside a choice: the choice"
    );
}

/// Each report is a bug report of its own, so triage takes one at a time:
/// the seat's checks flat in `client.report`, the model's words beside
/// them, the cards, the rules and the question bounded, and the game
/// named; the seat attaches no record of its own.
#[test]
fn each_report_is_a_bug_report_of_its_own_with_its_triage_fields_flat() {
    let mut second = filed_with(report("The trigger never fired.", "603.2"), 15);
    second.report.severity = None;
    let reports = vec![
        filed("Bolt dealt 2.", 12),
        filed("It dealt two.", 14),
        second,
    ];
    let bodies = submissions(&reports, None);
    assert_eq!(bodies.len(), 2, "each dedupe key once");
    let body = &bodies[0];
    assert_eq!(keys(body), ["client", "game_id", "kind", "text"]);
    assert_eq!(body["kind"], "bug");
    assert_eq!(body["game_id"], "TEST-game");
    let client = &body["client"];
    assert_eq!(
        keys(client),
        [
            "game_id",
            "model",
            "profile",
            "refs",
            "report",
            "seat",
            "source",
            "untrusted"
        ],
        "no log was told, none is sent, and so no tail of it"
    );
    assert_eq!(client["source"], "llm-seat");
    assert_eq!(client["game_id"], "TEST-game");
    assert_eq!(client["model"], "TEST-model");
    let report = &client["report"];
    assert_eq!(report["kind"], "wrong_outcome");
    assert_eq!(report["severity"], "game_changing");
    assert_eq!(report["confidence"], "high");
    assert_eq!(report["oracle_quote_matches"], true);
    assert_eq!(report["rules_format_ok"], true);
    assert_eq!(report["dedupe_key"], json!(reports[0].checked.dedupe_key));
    assert_eq!(
        report["cards"],
        json!([{"name": "Lightning Bolt", "oracle_quote": bolt_quote()}])
    );
    assert_eq!(report["rules"], json!(["120.3"]));
    assert_eq!(report["observed"], "Bolt dealt 2.");
    let decision = &report["decision"];
    assert_eq!(
        keys(decision),
        [
            "answer", "options", "pending", "phase", "seq", "step", "turn"
        ]
    );
    assert_eq!(decision["turn"], 4);
    assert_eq!(decision["phase"], "FirstMain");
    assert_eq!(decision["step"], "Main");
    assert_eq!(decision["pending"], "Priority");
    assert_eq!(decision["seq"], 77);
    assert_eq!(decision["options"][0]["id"], "a1");
    assert_eq!(
        decision["answer"],
        "a1 Cast Lightning Bolt #50 (taps Mountain #23)"
    );
    assert!(
        decision.get("policy_chose").is_none() && decision.get("model_would").is_none(),
        "only a trainer's"
    );
    assert_eq!(
        bodies[1]["client"]["report"]["observed"],
        "The trigger never fired."
    );
    assert_eq!(bodies[1]["client"]["report"]["severity"], Value::Null);
    for body in &bodies {
        for absent in ["local_record", "record"] {
            assert!(body.get(absent).is_none(), "{absent}");
        }
    }
    assert!(submissions(&[], None).is_empty(), "no reports, no report");
}

/// A report's text says whose words it is, what the seat checked, and
/// names its card in brackets, a reference to this build's pool's card
/// where it stands.
#[test]
fn a_report_s_text_says_whose_it_is_and_names_its_cards() {
    let body = submissions(&[filed("Bolt dealt 2.", 12)], None).remove(0);
    let client = &body["client"];
    let text = body["text"].as_str().unwrap();
    assert!(
        text.starts_with("Untrusted: a report written by a language model while it played P2 (model TEST-model, profile collector)."),
        "{text}"
    );
    assert!(
        text.contains("q12, turn 4: wrong_outcome, game_changing, confidence high"),
        "{text}"
    );
    assert!(text.contains("   Rules: 120.3"), "{text}");
    assert!(
        text.contains("every quote is words of the card text the model was told: yes"),
        "{text}"
    );
    assert!(!text.contains("1. q12"), "one report, no number");
    // The card named in brackets is a reference to this build's pool's
    // card, where it stands in the text.
    let cards = client["refs"]["cards"].as_array().unwrap();
    assert!(!cards.is_empty());
    let chars: Vec<char> = text.chars().collect();
    for card in cards {
        assert_eq!(card["card"], bolt().get());
        let at = card["at"].as_array().unwrap();
        let (start, end) = (
            usize::try_from(at[0].as_u64().unwrap()).unwrap(),
            usize::try_from(at[1].as_u64().unwrap()).unwrap(),
        );
        assert_eq!(
            chars[start..end].iter().collect::<String>(),
            "[Lightning Bolt]"
        );
    }
}

/// The text names the seat as its log and its model do (`P2` for the
/// view's seat 1), and every mind that filed a report, in order: after a
/// fallback took over, a chair's one bug report has the first's in
/// `client.model` and each report names its own; a report of its own
/// names the mind that filed it.
#[test]
fn a_bug_report_names_the_seat_as_its_log_does_and_every_mind() {
    let mut reports = vec![
        filed("Bolt dealt 2.", 12),
        filed_with(report("No trigger.", "603.2"), 14),
    ];
    reports[1].model = "TEST-fallback".into();
    reports[1].profile = Some("backup".into());
    reports.push(filed_with(report("Wrong mana.", "106.1"), 15));
    let body = bundle(&reports, None).unwrap();
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
    assert_eq!(client["reports"][0]["model"], "TEST-model");
    assert_eq!(client["reports"][1]["model"], "TEST-fallback");
    assert_eq!(client["reports"][1]["profile"], "backup");
    assert_eq!(client["seat"], 1, "the view's number, as each report's");
    let own = &submissions(&reports, None)[1];
    assert!(
        own["text"].as_str().unwrap().starts_with(
            "Untrusted: a report written by a language model while it played P2 (model \
             TEST-fallback, profile backup)."
        ),
        "{}",
        own["text"]
    );
    assert_eq!(own["client"]["model"], "TEST-fallback");
}

/// Where a route takes one bug report a game (a chair's), the game's
/// reports go in one, each with its own fields; one report alone goes as
/// it goes anywhere.
#[test]
fn a_chair_s_reports_go_in_one_each_with_its_fields() {
    let reports = vec![
        filed("Bolt dealt 2.", 12),
        filed("It dealt two.", 14),
        filed_with(report("The trigger never fired.", "603.2"), 15),
    ];
    let body = bundle(&reports, None).unwrap();
    let client = &body["client"];
    assert!(client.get("report").is_none());
    assert!(client.get("reports_omitted").is_none(), "all fit");
    let items = client["reports"].as_array().unwrap();
    assert_eq!(items.len(), 2, "each dedupe key once");
    for (item, filed) in items.iter().zip([&reports[0], &reports[2]]) {
        let mut own = filed.value(None);
        own["model"] = json!(filed.model);
        own["profile"] = json!(filed.profile);
        assert_eq!(item, &own, "the report, and the mind that filed it");
        assert_eq!(item["kind"], "wrong_outcome");
        assert!(item["dedupe_key"].is_string());
    }
    let text = body["text"].as_str().unwrap();
    assert!(text.starts_with("Untrusted: reports written"), "{text}");
    assert!(text.contains("\n1. q12, turn 4: "), "{text}");
    assert!(text.contains("\n2. q15, turn 4: "), "{text}");
    let one = [filed("Bolt dealt 2.", 12)];
    assert_eq!(bundle(&one, None), submissions(&one, None).pop());
    assert_eq!(bundle(&[], None), None);
}

/// A report filed by a model that watches the trained AI says so, and its
/// question says what the policy chose and what the model would have.
#[test]
fn a_trainer_s_report_says_so_and_what_each_chose() {
    let mut trainer = filed("The policy was offered an illegal cast.", 12);
    trainer.source = Source::LlmTrainer;
    trainer.moment.policy_chose = Some("a1 Cast Lightning Bolt #50".into());
    trainer.moment.model_would = Some("p Pass priority".into());
    let body = &submissions(&[trainer.clone()], None)[0];
    let client = &body["client"];
    assert_eq!(client["source"], "llm-trainer");
    assert!(
        client["untrusted"]
            .as_str()
            .unwrap()
            .contains("watched the trained AI play")
    );
    let decision = &client["report"]["decision"];
    assert_eq!(decision["policy_chose"], "a1 Cast Lightning Bolt #50");
    assert_eq!(decision["model_would"], "p Pass priority");
    assert!(
        body["text"]
            .as_str()
            .unwrap()
            .contains("while it watched the trained AI play P2 (model TEST-model")
    );
    let mut seat = trainer;
    seat.source = Source::LlmSeat;
    let decision = &submissions(&[seat], None)[0]["client"]["report"]["decision"];
    assert!(
        decision.get("policy_chose").is_none(),
        "a seat's report never says it"
    );
}

/// The longest reports there may be, many options and characters of four
/// bytes: each bug report's text within its bound, its question's options
/// cut to forty, and `client` within the gateway's bound; a bundle that
/// would pass it leaves its last reports out and says how many.
#[test]
fn bug_reports_hold_to_their_bounds() {
    let wide = |n: usize, c: char| std::iter::repeat_n(c, n).collect::<String>();
    let heavy = |n: u64| {
        let mut f = distinct(n);
        let r = &mut f.report;
        r.observed = format!("{n} {}", wide(FIELD_CHARS, '😀'));
        r.expected = Some(wide(FIELD_CHARS, '😀'));
        r.evidence = Some(wide(FIELD_CHARS, '😀'));
        r.references = vec![wide(REFERENCE_CHARS, '😀'); MOST_REFERENCES];
        r.cards = (0..MOST_CARDS)
            .map(|c| Card {
                name: format!("{c}{}", wide(NAME_CHARS - 1, '😀')),
                oracle_quote: Some(wide(QUOTE_CHARS, '😀')),
            })
            .collect();
        f.moment.options = (0..60)
            .map(|o| Offered {
                id: format!("a{o}"),
                text: wide(500, '😀'),
            })
            .collect();
        f.moment.answer = Some(wide(1_000, '😀'));
        f
    };
    let reports: Vec<Filed> = (0..MOST as u64).map(heavy).collect();
    let weight = |value: &Value| serde_json::to_vec(value).unwrap().len();
    let bodies = submissions(&reports, None);
    assert_eq!(bodies.len(), MOST);
    for body in &bodies {
        let text = body["text"].as_str().unwrap();
        assert!(
            text.chars().count() <= TEXT_CHARS,
            "{}",
            text.chars().count()
        );
        assert!(weight(&body["client"]) <= CLIENT_BYTES);
        let decision = &body["client"]["report"]["decision"];
        let options = decision["options"].as_array().unwrap();
        assert_eq!(options.len(), MOST_OPTIONS);
        assert_eq!(decision["options_omitted"], 60 - MOST_OPTIONS);
        assert_eq!(
            options[0]["text"].as_str().unwrap().chars().count(),
            OPTION_CHARS
        );
        assert_eq!(
            decision["answer"].as_str().unwrap().chars().count(),
            ANSWER_CHARS
        );
    }
    let body = bundle(&reports, None).unwrap();
    let client = &body["client"];
    assert!(weight(client) <= CLIENT_BYTES, "{}", weight(client));
    let sent = client["reports"].as_array().unwrap().len();
    let left = usize::try_from(client["reports_omitted"].as_u64().unwrap()).unwrap();
    assert!(sent > 1 && left > 0, "{sent} sent, {left} left out");
    assert_eq!(sent + left, MOST);
    let text = body["text"].as_str().unwrap();
    assert!(text.chars().count() <= TEXT_CHARS);
    assert!(text.contains("more reports are in client.reports"), "said");
}

/// A refusal repeats the model's words ("not taken: board «…» is not
/// full"), so an answer may be as long as the model made it. The text holds
/// the question and the answer to the bounds `decision` keeps them to, so
/// one long answer never pushes its report out of the text, alone or in a
/// chair's bundle.
#[test]
fn a_long_answer_is_cut_in_the_text_as_in_the_decision() {
    let wide = |n: usize, c: char| std::iter::repeat_n(c, n).collect::<String>();
    let long = |f: &mut Filed| {
        f.moment.pending = wide(500, 'P');
        f.moment.answer = Some(format!(
            "not taken: board «{}» is not full",
            wide(30_000, 'x')
        ));
    };
    let mut one = filed("Bolt dealt 2.", 12);
    long(&mut one);
    let line = format!(
        "   The question: {}; the answer: {}\n",
        wide(ID_CHARS, 'P'),
        cut_to(one.moment.answer.as_deref().unwrap(), ANSWER_CHARS)
    );
    let body = submissions(std::slice::from_ref(&one), None).remove(0);
    let text = body["text"].as_str().unwrap();
    assert!(text.contains("Observed: Bolt dealt 2."), "{text}");
    assert!(text.contains(&line), "{text}");
    assert!(!text.contains("Shortened"), "{text}");
    assert_eq!(
        body["client"]["report"]["decision"]["answer"].as_str(),
        Some(line.trim_end().rsplit("the answer: ").next().unwrap()),
        "the text's answer is the decision's"
    );

    // A key of its own, or the bundle files it once.
    let mut two = filed_with(report("Something else entirely.", "601.2"), 13);
    long(&mut two);
    let body = bundle(&[one, two], None).unwrap();
    let text = body["text"].as_str().unwrap();
    assert!(text.contains("1. q12") && text.contains("2. q13"), "{text}");
    assert_eq!(text.matches(line.as_str()).count(), 2, "{text}");
    assert!(!text.contains("Shortened"), "{text}");
}

/// `text`'s first `most` characters.
fn cut_to(text: &str, most: usize) -> String {
    text.chars().take(most).collect()
}
