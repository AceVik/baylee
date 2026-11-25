//! What the plain-data contracts refuse from a peer that means harm: engine
//! identities, seat agent frames, declared minds and tape lines.

use baylee_protocol::execution::{
    Identity, MAX_CAPABILITIES, MAX_METADATA_BYTES, SCHEMA, Variant, capabilities_from_metadata,
    valid_capabilities,
};
use baylee_protocol::mind::{MODEL_MAX, WORD_MAX, fault, shaped_like_a_key};
use baylee_protocol::v1::{SeatMind, seat_mind::Kind};
use baylee_protocol::{chair_redeem_path, chair_ticket_path, seathost, tape, watch_socket_path};

const REVISION: &str = "d102ca933e0ec8827d7579ec57cbe3f88d7349ad";

fn identity() -> Identity {
    Identity::native(Variant::StandardLoops, "1.0 (d102ca933e)", REVISION, 32)
}

fn changed(change: impl FnOnce(&mut Identity)) -> Identity {
    let mut identity = identity();
    change(&mut identity);
    identity
}

#[test]
fn an_identity_is_bounded_in_every_field() {
    type Field = fn(&mut Identity) -> &mut String;
    assert!(identity().valid());
    let fields: [(Field, usize); 4] = [
        (|i| &mut i.build, 160),
        (|i| &mut i.rules, 96),
        (|i| &mut i.cards, 96),
        (|i| &mut i.ai, 96),
    ];
    for (field, max) in fields {
        for (text, want) in [
            (String::new(), false),
            ("x".into(), true),
            ("x".repeat(max), true),
            ("x".repeat(max + 1), false),
            (" x".into(), false),
            ("x ".into(), false),
            ("a b~!".into(), true),
            ("a\tb".into(), false),
            ("a\0b".into(), false),
            ("a\u{7f}b".into(), false),
            ("ä".into(), false),
            ("a\u{202e}b".into(), false),
        ] {
            let identity = changed(|i| field(i).clone_from(&text));
            assert_eq!(identity.valid(), want, "{max}: {text:?}");
        }
    }
    for (features, want) in [
        ("0123456789abcdef".repeat(4), true),
        ("a".repeat(63), false),
        ("a".repeat(65), false),
        ("A".repeat(64), false),
        ("g".repeat(64), false),
        ("é".repeat(32), false),
    ] {
        assert_eq!(changed(|i| i.features.clone_from(&features)).valid(), want);
    }
}

#[test]
fn metadata_is_read_up_to_its_bound_and_never_past_it() {
    let mut bytes = identity().to_metadata();
    bytes.resize(MAX_METADATA_BYTES, b' ');
    assert_eq!(Identity::from_metadata(&bytes), Some(identity()));
    bytes.push(b' ');
    assert_eq!(Identity::from_metadata(&bytes), None);
    assert_eq!(Identity::from_metadata(b""), None);
}

#[test]
fn an_agent_is_taken_with_every_capability_it_names_or_with_none() {
    let distinct = |n: usize| -> Vec<Identity> {
        (0..n)
            .map(|at| changed(|i| i.ai = format!("house-{at}")))
            .collect()
    };
    let encoded =
        |list: &[Identity]| -> Vec<Vec<u8>> { list.iter().map(Identity::to_metadata).collect() };
    let full = distinct(MAX_CAPABILITIES);
    assert_eq!(
        capabilities_from_metadata(&encoded(&full)),
        Some(full.clone())
    );
    assert!(capabilities_from_metadata(&[]).is_none());
    assert!(!valid_capabilities(&distinct(MAX_CAPABILITIES + 1)));
    assert!(capabilities_from_metadata(&encoded(&distinct(MAX_CAPABILITIES + 1))).is_none());

    let mut one_bad = encoded(&full);
    one_bad[7] = b"{}".to_vec();
    assert!(capabilities_from_metadata(&one_bad).is_none());
    // A repeat far from its twin is a repeat too.
    let mut twice = full;
    twice[MAX_CAPABILITIES - 1] = twice[0].clone();
    assert!(capabilities_from_metadata(&encoded(&twice)).is_none());
}

#[test]
fn a_ranked_key_needs_a_clean_stamp_of_a_whole_revision() {
    assert!(identity().ranked_compatibility().is_some());
    let long = format!("{REVISION}{}", "0".repeat(24));
    let sha256 = Identity::native(Variant::StandardLoops, "1.0 (d102ca933e)", &long, 32);
    assert!(sha256.ranked_compatibility().is_some());

    for build in [
        "1.0 (d102ca933)".to_owned(),
        "1.0(d102ca933e)".into(),
        "1.0 (d102ca933e".into(),
        "1.0 d102ca933e)".into(),
        "1.0 ()".into(),
        format!("1.0 ({REVISION}0)"),
        format!("{} (d102ca933e)", "x".repeat(160)),
    ] {
        let other = changed(|i| i.build.clone_from(&build));
        assert_eq!(other.ranked_compatibility(), None, "{build}");
    }
    for revision in [
        REVISION[..39].to_owned(),
        format!("{REVISION}0"),
        REVISION.to_uppercase(),
        format!("{}g", &REVISION[..39]),
    ] {
        let other = Identity::native(Variant::StandardLoops, "1.0 (d102ca933e)", &revision, 32);
        assert_eq!(other.ranked_compatibility(), None, "{revision}");
    }
    for other in [
        changed(|i| i.cards = format!("cards@{REVISION}")),
        changed(|i| i.schema = SCHEMA + 1),
        changed(|i| i.protocol -= 1),
        changed(|i| i.variant = Variant::Tower),
    ] {
        assert_eq!(other.ranked_compatibility(), None);
    }
}

#[test]
fn each_variant_names_its_own_rules_features_and_house() {
    let of = |variant| Identity::native(variant, "b", "r", 5);
    let (plain, loops, tower) = (
        of(Variant::Standard),
        of(Variant::StandardLoops),
        of(Variant::Tower),
    );
    assert!(plain.valid() && loops.valid() && tower.valid());
    assert_ne!(plain.features, loops.features);
    assert_ne!(plain.features, tower.features);
    assert_ne!(loops.features, tower.features);
    assert_eq!(plain.rules, loops.rules);
    assert_ne!(plain.rules, tower.rules);
    assert!(plain.ai.starts_with("standard-") && tower.ai.starts_with("tower-"));
    assert!(tower.rules.starts_with("tower-"));
    assert!(plain.ai.ends_with("-5") && plain.cards == "registry@r");
}

#[test]
fn a_seat_agent_frame_is_one_whole_known_document() {
    for bad in [
        "",
        "null",
        "[]",
        "{}",
        r#"{"type":"nonsense"}"#,
        r#"{"type":"Heartbeat"}"#,
        r#"{"type":"stop_seat"}"#,
        r#"{"type":"stop_seat","order":7}"#,
        r#"{"type":"heartbeat"}{"type":"heartbeat"}"#,
        r#"{"type":"start_seat","order":"o","game_id":"g","seat":-1,"profile":"p","chair_ticket":"t","gateway_url":"u"}"#,
        r#"{"type":"start_seat","order":"o","game_id":"g","seat":4294967296,"profile":"p","chair_ticket":"t","gateway_url":"u"}"#,
    ] {
        assert!(seathost::Frame::parse(bad).is_err(), "{bad}");
    }
    // Nested far past what any frame holds: an error, not a blown stack.
    let deep = format!(
        r#"{{"type":"profile_write","request":"r","id":"i","admin":"a","definition":{{"label":"L","vendor":"V","profile":{}{}}}}}"#,
        "[".repeat(100_000),
        "]".repeat(100_000)
    );
    assert!(seathost::Frame::parse(&deep).is_err());
}

#[test]
fn a_seat_agent_frame_that_cannot_be_read_does_not_repeat_the_key_in_it() {
    let refused = seathost::Frame::parse(
        r#"{"type":"key_write","request":"r","id":"i","admin":7,"key":"sk-ant-very-secret"}"#,
    )
    .expect_err("a number for a name");
    assert!(!refused.contains("very-secret"), "{refused}");
}

#[test]
fn a_profile_label_is_visible_and_at_most_forty_characters() {
    for (label, want) in [
        ("ä".repeat(40), true),
        ("ä".repeat(41), false),
        ("x".into(), true),
        (String::new(), false),
        ("\u{3000}".into(), false),
        ("a\u{85}b".into(), false),
    ] {
        assert_eq!(seathost::valid_label(&label), want, "{label:?}");
    }
    assert!(seathost::valid_name(&"a".repeat(64)));
    for bad in ["a b", "a\\b", "a%2e", "ä", "a\0", "a:b"] {
        assert!(!seathost::valid_name(bad), "{bad:?}");
    }
}

#[test]
fn a_hosted_chair_is_named_by_at_most_twelve_plain_characters() {
    for (label, id, want) in [
        ("-_Opus_-", "x", "Opus"),
        ("a b", "x", "a-b"),
        ("\u{202e}evil\u{200b}", "x", "evil"),
        ("GPT/6..1", "x", "GPT61"),
        ("ABCDEFGHIJK-LM", "x", "ABCDEFGHIJK"),
        ("ABCDEFGHIJKLM", "x", "ABCDEFGHIJKL"),
        ("--- ---", "my.profile", "myprofile"),
        ("", "", "model"),
    ] {
        assert_eq!(seathost::chair_name(label, id), want, "{label:?}");
    }
}

#[test]
fn a_profile_state_reads_as_the_word_it_writes() {
    use seathost::State;
    for state in [
        State::Available,
        State::Busy,
        State::Probing,
        State::Exhausted,
        State::NeedsLogin,
        State::Failing,
        State::Disabled,
    ] {
        assert_eq!(serde_json::to_value(state).unwrap(), state.word());
    }
}

fn model(change: impl FnOnce(&mut SeatMind)) -> Option<&'static str> {
    let mut mind = SeatMind {
        kind: Kind::LlmApi.into(),
        provider: "anthropic".into(),
        model: "claude".into(),
        ..SeatMind::default()
    };
    change(&mut mind);
    fault(&mind)
}

#[test]
fn a_declared_mind_is_bounded_in_characters_per_field() {
    assert_eq!(model(|_| {}), None);
    assert_eq!(model(|m| m.provider = "p".repeat(WORD_MAX)), None);
    assert!(model(|m| m.provider = "p".repeat(WORD_MAX + 1)).is_some());
    assert_eq!(model(|m| m.model = "m".repeat(MODEL_MAX)), None);
    assert!(model(|m| m.model = "m".repeat(MODEL_MAX + 1)).is_some());
    assert_eq!(model(|m| m.effort = "e".repeat(WORD_MAX)), None);
    assert!(model(|m| m.effort = "e".repeat(WORD_MAX + 1)).is_some());
    for bad in [
        "a b",
        "a\nb",
        "ä",
        "a\u{202e}",
        "<b>",
        "a\"b",
        "a;b",
        "http://x",
        "a://b",
    ] {
        assert!(model(|m| m.model = bad.into()).is_some(), "{bad:?}");
    }
    assert_eq!(
        model(|m| m.model = "org/model-1.0:latest@v1_x".into()),
        None
    );
    assert!(model(|m| m.kind = 99).is_some(), "a kind of a newer peer");
}

#[test]
fn a_key_is_known_by_its_marker_and_length_wherever_it_could_start() {
    let k = |n: usize| "a".repeat(n);
    for (text, want) in [
        (format!("sk-{}", k(16)), true),
        (format!("sk-{}", k(15)), false),
        (format!("KEY=sk-{}", k(16)), true),
        (format!("x9sk-{}", k(16)), true),
        (format!("desk-{}", k(16)), false),
        (format!("a.sk-{}", k(16)), false),
        (format!("ésk-{}", k(16)), true),
        // A word first and a key after it.
        (format!("desk-tools sk-{}", k(16)), true),
        (format!("sonnetsk-ant-{}", k(20)), true),
        (format!("tsk-ant-{}", k(19)), false),
        (format!("xsk-proj-{}", k(20)), true),
        (format!("github_pat_{}", k(20)), true),
        (format!("ghx_{}", k(20)), false),
        ("Bearer x".into(), true),
        ("Bearer ".into(), false),
        ("forbearer x".into(), false),
        ("x-api-key".into(), true),
        ("prefix-api-key".into(), false),
        (String::new(), false),
    ] {
        assert_eq!(shaped_like_a_key(&text), want, "{text}");
    }
    for letter in ["p", "o", "u", "s", "r"] {
        assert!(shaped_like_a_key(&format!("zgh{letter}_{}", k(20))));
        assert!(!shaped_like_a_key(&format!("zgh{letter}_{}", k(19))));
    }
}

#[test]
fn a_tape_line_that_is_not_one_known_document_is_refused() {
    for bad in [
        "",
        "null",
        "[]",
        "{}",
        r#"{"kind":"nothing"}"#,
        "\u{feff}{}",
        &"[".repeat(100_000),
    ] {
        assert!(
            tape::parse_line(bad).is_err(),
            "{}",
            &bad[..bad.len().min(40)]
        );
    }
}

#[test]
fn every_ticket_path_names_its_game_and_chair_and_no_secret() {
    assert_eq!(
        watch_socket_path("g1", "t1"),
        format!(
            "/games/g1/watch?ticket=t1&protocol={}",
            baylee_protocol::PROTOCOL_VERSION
        )
    );
    assert_eq!(
        chair_ticket_path("g1", 3),
        "/lobby/games/g1/chairs/3/ticket"
    );
    assert_eq!(
        chair_redeem_path("g1", 3),
        "/lobby/games/g1/chairs/3/redeem"
    );
}
