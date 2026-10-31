use super::*;
use std::path::PathBuf;

pub(crate) struct TestDir(pub PathBuf);
impl TestDir {
    pub fn new(label: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "baylee-llm-collector4-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn codex_identity() -> baylee_gamehost::record::Mind {
    baylee_gamehost::record::Mind {
        kind: baylee_gamehost::record::MindKind::LlmCli,
        provider: "codex".into(),
        model: "gpt-6.1-terra".into(),
        effort: "low".into(),
        level: String::new(),
    }
}

pub fn write_identity_record(path: &Path, mind: &baylee_gamehost::record::Mind) {
    let rows = [
        json!({"kind":"header", "preset":{"seed":777,"seats":[{},{}]}}),
        json!({"kind":"declared_mind", "seat":0, "mind":mind, "at":0}),
        json!({"kind":"declared_mind", "seat":1, "mind":mind, "at":0}),
        json!({"kind":"input", "n":0, "seat":0, "by":"seat", "action":"PassPriority", "at":0}),
        json!({"kind":"end", "n":1, "winners":[0], "at":0}),
    ];
    let text = rows.iter().fold(String::new(), |mut text, row| {
        writeln!(text, "{row}").unwrap();
        text
    });
    std::fs::write(path, text).unwrap();
}

#[test]
fn actual_engine_record_preserves_format_and_enforces_life_and_mulligans() {
    use baylee_core::{ids::PlayerId, preset::SeatController};
    use baylee_engine::choice::PlayerAction;
    use baylee_gamehost::Session;
    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/acceptance-decks.txt"
    ));
    let a = baylee_cards::decks::load_acceptance(text, "Allytifact").unwrap();
    let b = baylee_cards::decks::load_acceptance(text, "Victory").unwrap();
    let dir = TestDir::new("actual-room-setup");
    let path = dir.path().join("record.jsonl");
    for declared in ["Commander", "Constructed", "Highlander"] {
        let setup = crate::schedule::collector_room_setup([declared, declared]).unwrap();
        let mut preset = baylee_cards::decks::preset_for(3, &a, &b);
        let actual_format = serde_json::to_value(preset.format).unwrap();
        baylee_cards::decks::apply_room_setup(&mut preset, &setup).unwrap();
        for seat in &mut preset.seats {
            seat.controller = SeatController::Open;
        }
        let mut session = Session::new_recorded(&preset, "setup-test").unwrap();
        session
            .act(PlayerId::new(0), PlayerAction::Concede)
            .unwrap();
        std::fs::write(&path, session.take_record()).unwrap();
        let actual = audit_record_setup(&path, &setup).unwrap();
        assert_eq!(actual["actual_record_format"], actual_format);
        assert_eq!(
            actual["seat_starting_life"],
            json!([setup.starting_life, setup.starting_life])
        );
        let opposite = crate::schedule::collector_room_setup(if declared == "Commander" {
            ["Constructed", "Constructed"]
        } else {
            ["Commander", "Commander"]
        })
        .unwrap();
        assert!(audit_record_setup(&path, &opposite).is_err());
        let mut wrong_mulligans = setup.clone();
        wrong_mulligans.free_mulligans = u8::from(setup.free_mulligans == 0);
        assert!(audit_record_setup(&path, &wrong_mulligans).is_err());
    }
}

#[test]
fn exact_codex_identity_passes_but_provider_model_effort_and_kind_spoofs_fail() {
    let dir = TestDir::new("declared-identity");
    let path = dir.path().join("record.jsonl");
    let mind = codex_identity();
    let expected = [vec![mind.clone()], vec![mind.clone()]];
    write_identity_record(&path, &mind);
    assert!(audit_record(&path, &expected).is_ok());
    for field in ["provider", "model", "effort", "kind", "level"] {
        let mut spoof = mind.clone();
        match field {
            "provider" => spoof.provider = "agy".into(),
            "model" => spoof.model = "gpt-6.1-sol".into(),
            "effort" => spoof.effort = "high".into(),
            "kind" => spoof.kind = baylee_gamehost::record::MindKind::LlmApi,
            _ => spoof.level = "steady".into(),
        }
        write_identity_record(&path, &spoof);
        assert!(audit_record(&path, &expected).is_err(), "{field}");
        // The completed lifecycle and the failed admission remain independent.
        assert!(completed_record_end(&path).is_ok());
    }
}

/// A record whose seat 1 declares `codex_identity()` and whose seat 0
/// declares each of `seat0` in turn, an input of its own after each.
fn write_chain_record(path: &Path, seat0: &[&baylee_gamehost::record::Mind]) {
    let mut rows = vec![
        json!({"kind":"header", "preset":{"seed":777,"seats":[{},{}]}}),
        json!({"kind":"declared_mind", "seat":1, "mind":codex_identity(), "at":0}),
    ];
    for (n, mind) in seat0.iter().enumerate() {
        rows.push(json!({"kind":"declared_mind", "seat":0, "mind":mind, "at":0}));
        rows.push(
            json!({"kind":"input", "n":n, "seat":0, "by":"seat", "action":"PassPriority", "at":0}),
        );
    }
    rows.push(json!({"kind":"end", "n":seat0.len(), "winners":[0], "at":0}));
    let text = rows.iter().fold(String::new(), |mut text, row| {
        writeln!(text, "{row}").unwrap();
        text
    });
    std::fs::write(path, text).unwrap();
}

/// A seat declares its profile's mind first, then fallbacks of its
/// requested chain in their order (one passed over where it could not take
/// over), each declared again on a new socket; the record's declarations
/// are listed once each, with where they stand and the first input after
/// them; the house may take the seat last, once the chain is used up. A
/// fallback out of order, a mind handed back to, one not requested, a
/// declaration after the house, or a first declaration that is not the
/// profile's own fails the audit.
#[test]
fn declarations_follow_each_seat_s_requested_chain() {
    use baylee_gamehost::record::{Mind, MindKind};
    let dir = TestDir::new("declared-chain");
    let path = dir.path().join("record.jsonl");
    let api = |model: &str| Mind {
        kind: MindKind::LlmApi,
        provider: "openai".into(),
        model: model.into(),
        effort: "low".into(),
        level: String::new(),
    };
    let primary = codex_identity();
    let (fb1, fb2) = (api("gpt-6.1-terra"), api("deepseek-chat"));
    let expected = [
        vec![primary.clone(), fb1.clone(), fb2.clone()],
        vec![codex_identity()],
    ];
    let house = Mind {
        kind: MindKind::House,
        provider: String::new(),
        model: String::new(),
        effort: String::new(),
        level: "sharp".into(),
    };
    let place = |mind: &Mind, index: usize, n: usize| json!({"mind": mind, "chain_index": index, "first_input_n": n});
    for (seat0, declared) in [
        (vec![&primary], vec![place(&primary, 0, 0)]),
        (
            vec![&primary, &fb1, &fb2],
            vec![place(&primary, 0, 0), place(&fb1, 1, 1), place(&fb2, 2, 2)],
        ),
        (
            vec![&primary, &primary, &fb2, &fb2],
            vec![place(&primary, 0, 0), place(&fb2, 2, 2)],
        ),
        (
            vec![&primary, &fb1, &fb2, &house],
            vec![
                place(&primary, 0, 0),
                place(&fb1, 1, 1),
                place(&fb2, 2, 2),
                json!({"mind": &house, "chain_index": null, "house": true, "first_input_n": 3}),
            ],
        ),
    ] {
        write_chain_record(&path, &seat0);
        let proof = audit_record(&path, &expected).unwrap();
        assert_eq!(proof.declared[0], declared);
        assert_eq!(
            proof.declared[1],
            [json!({"mind": codex_identity(), "chain_index": 0, "first_input_n": null})],
            "seat 1 made no input"
        );
    }
    let other = api("gpt-6.1-sol");
    for seat0 in [
        vec![&fb1],
        vec![&primary, &fb2, &fb1],
        vec![&primary, &fb1, &primary],
        vec![&primary, &other],
        vec![&house],
        vec![&primary, &house, &fb1],
    ] {
        write_chain_record(&path, &seat0);
        assert!(audit_record(&path, &expected).is_err(), "{seat0:?}");
    }
    // A seat without fallbacks takes nothing but its own mind.
    write_chain_record(&path, &[&primary, &fb1]);
    assert!(audit_record(&path, &[vec![primary.clone()], vec![codex_identity()]]).is_err());
}

/// What the transcript says of the fallbacks is kept as it said it: each
/// that took over and each that could not, in order, and the closing
/// `fell_back` line; a seat without any says so with empty lists.
#[test]
fn a_transcript_s_fallbacks_are_kept_in_order() {
    let rows = [
        json!({"event":"answered","question":1,"by":"mind","action":"PassPriority"}),
        json!({"fallback_refused": {"profile":"fb1","why":"the day's cap of $3.00 is spent"}}),
        json!({"fallback": {"profile":"fb2","turn":3,"cause":"limited","why":"429"}}),
        json!({"fallback": {"profile":"fb3","turn":5,"cause":"failed","why":"502"}}),
        json!({"summary": {}}),
        json!({"fell_back": [
            {"label":"fb2","turn":3,"answered":4},
            {"label":"fb3","turn":5,"answered":9},
        ]}),
    ];
    assert_eq!(
        transcript_fallbacks(&rows),
        json!({
            "took_over": [
                {"profile":"fb2","turn":3,"cause":"limited","why":"429"},
                {"profile":"fb3","turn":5,"cause":"failed","why":"502"},
            ],
            "refused": [{"profile":"fb1","why":"the day's cap of $3.00 is spent"}],
            "played": [
                {"label":"fb2","turn":3,"answered":4},
                {"label":"fb3","turn":5,"answered":9},
            ],
        })
    );
    assert_eq!(
        transcript_fallbacks(&rows[..1]),
        json!({"took_over": [], "refused": [], "played": []})
    );
}

#[test]
fn full_engine_replay_rejects_an_invisible_intermediate_hash_mismatch() {
    use baylee_core::{ids::PlayerId, preset::SeatController};
    use baylee_engine::choice::{PlayerAction, PriorityHold};
    use baylee_gamehost::{Session, replay};
    use std::ops::ControlFlow;

    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/acceptance-decks.txt"
    ));
    let a = baylee_cards::decks::load_acceptance(text, "Allytifact").unwrap();
    let b = baylee_cards::decks::load_acceptance(text, "Victory").unwrap();
    let mut preset = baylee_cards::decks::preset_for(3, &a, &b);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Open;
    }
    let mut session = Session::new_recorded(&preset, "test").unwrap();
    // A private automation setting changes the recorded engine state while
    // mulligans are pending, without changing the public table. Do not rely
    // on house opening-hand decisions to produce an invisible input.
    session
        .act(
            PlayerId::new(0),
            PlayerAction::SetPriorityHold(PriorityHold::PassWhenNothingToDo),
        )
        .unwrap();
    session
        .act(PlayerId::new(0), PlayerAction::Concede)
        .unwrap();
    let record = session.take_record();
    let rows: Vec<Value> = String::from_utf8(record.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let input_ns: Vec<u64> = rows
        .iter()
        .filter(|row| row["kind"] == "input")
        .map(|row| row["n"].as_u64().unwrap())
        .collect();
    let mut invisible = None;
    replay::project(
        &record,
        vec!["A".into(), "B".into()],
        replay::Perspective::Public,
        replay::Verify::Stop,
        |moment| {
            if moment.frames.is_empty() && moment.input.is_some() {
                invisible = moment.input;
            }
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    let invisible = invisible.expect("fixture has an invisible input");
    let mut proof = RecordProof {
        actual_seed: 3,
        terminal: json!({}),
        inputs: [Vec::new(), Vec::new()],
        input_count: input_ns.len(),
        declared: [Vec::new(), Vec::new()],
        input_ns,
        record,
    };
    let dir = TestDir::new("full-replay");
    let path = dir.path().join("tape.jsonl");
    let stop = json!({"kind":"stop","stop":"end","inputs":proof.input_count,
        "exact_to":proof.input_ns.last(),"diverged":null});
    std::fs::write(&path, serde_json::to_vec(&stop).unwrap()).unwrap();
    audit_tape(&path, &proof).unwrap();
    let mut damaged = rows;
    let row = damaged
        .iter_mut()
        .find(|row| row["kind"] == "input" && row["n"] == invisible)
        .unwrap();
    row["hash"] = json!("wrong");
    proof.record = damaged
        .iter()
        .fold(String::new(), |mut text, row| {
            writeln!(text, "{row}").unwrap();
            text
        })
        .into_bytes();
    assert!(audit_tape(&path, &proof).is_err());
}

#[test]
fn replay_receipt_requires_full_final_hash_with_noncontiguous_record_numbers() {
    let proof = RecordProof {
        actual_seed: 1,
        terminal: json!({}),
        inputs: [Vec::new(), Vec::new()],
        input_count: 3,
        declared: [Vec::new(), Vec::new()],
        input_ns: vec![0, 4, 9],
        record: Vec::new(),
    };
    let valid = json!({"kind":"stop", "stop":"end", "inputs":3,
        "exact_to":9, "seen_to":9, "diverged":null});
    assert!(audit_tape_rows(std::slice::from_ref(&valid), &proof).is_ok());
    for patch in [
        json!({"exact_to":null}), // visible-only verification
        json!({"exact_to":4}),    // verified prefix
        json!({"exact_to":3}),    // count is not the last record n
        json!({"inputs":2}),
        json!({"diverged":{"input":9,"by":"hash"}}),
        json!({"stop":"cut"}),
    ] {
        let mut stop = valid.clone();
        for (key, value) in patch.as_object().unwrap() {
            stop[key] = value.clone();
        }
        assert!(audit_tape_rows(&[stop], &proof).is_err(), "{patch}");
    }
    for check in ["seen", "unverified"] {
        assert!(
            audit_tape_rows(
                &[json!({"kind":"view", "n":4, "check":check}), valid.clone()],
                &proof
            )
            .is_err()
        );
    }
}

#[test]
fn anonymise_zeroes_human_ids_and_at_and_rejects_leaks() {
    let raw = concat!(
        r#"{"kind":"header","format":"Commander","rules":{"starting_life":40,"commander":true},"preset":{"seed":42,"seats":[{"controller":{"Human":{"user_id":99}},"deck":[]},{"controller":{"Human":{"user_id":100}},"deck":[]}]}}"#,
        "\n",
        r#"{"kind":"input","n":0,"seat":0,"by":"seat","action":"PassPriority","at":12345}"#,
        "\n",
        r#"{"kind":"end","winners":[0],"at":67890}"#,
        "\n"
    );
    let names = vec!["alice_secret".to_owned(), "bob_secret".to_owned()];
    let clean = anonymise(raw, &names).expect("should anonymise cleanly");
    assert!(clean.contains(r#""user_id":0"#));
    assert!(clean.contains(r#""at":0"#));
    assert!(!clean.contains("12345"));
    assert!(!clean.contains("99"));

    let leaked = concat!(
        r#"{"kind":"header","preset":{"seed":42,"seats":[{"controller":{"Human":{"user_id":99}},"name":"alice_secret"}]}}"#,
        "\n"
    );
    assert!(anonymise(leaked, &names).is_err());
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "Complete technical audit fixture binds all independent record and seat artifacts"
)]
fn full_audit_and_validation_matches_prepare_llm_bc4_contract() {
    let dir = TestDir::new("audit");
    let work = dir.path();
    std::fs::create_dir_all(work.join("official-export")).unwrap();
    std::fs::create_dir_all(work.join("transcripts")).unwrap();

    let rec_path = work.join("official-export/000001.jsonl.gz");
    let rec_lines = concat!(
        r#"{"kind":"header","format":"Constructed","rules":{"starting_life":20},"preset":{"seed":777,"seats":[{"name":"Seat 0","deck":[]},{"name":"Seat 1","deck":[]}]}}"#,
        "\n",
        r#"{"kind":"declared_mind","seat":0,"mind":{"kind":"llm_cli","provider":"agy","model":"gemini-4-argon-low","effort":"low"},"at":0}"#,
        "\n",
        r#"{"kind":"declared_mind","seat":1,"mind":{"kind":"llm_cli","provider":"agy","model":"gemini-3.8-flash-low","effort":"low"},"at":0}"#,
        "\n",
        r#"{"kind":"input","n":0,"seat":0,"by":"seat","action":"PassPriority","at":0}"#,
        "\n",
        r#"{"kind":"input","n":1,"seat":1,"by":"seat","action":"PassPriority","at":0}"#,
        "\n",
        r#"{"kind":"end","winners":[0],"at":0}"#,
        "\n"
    );
    {
        let f = std::fs::File::create(&rec_path).unwrap();
        let mut gz = GzEncoder::new(f, Compression::default());
        gz.write_all(rec_lines.as_bytes()).unwrap();
        gz.finish().unwrap();
    }
    let tape_path = work.join("public-replay.jsonl.gz");
    let tape_lines = concat!(
        r#"{"kind":"header","format":"Constructed"}"#,
        "\n",
        r#"{"kind":"stop","stop":"end","inputs":2,"exact_to":1,"diverged":null}"#,
        "\n"
    );
    {
        let f = std::fs::File::create(&tape_path).unwrap();
        let mut gz = GzEncoder::new(f, Compression::default());
        gz.write_all(tape_lines.as_bytes()).unwrap();
        gz.finish().unwrap();
    }
    let tr0_path = work.join("transcripts/game1-seat0.jsonl");
    std::fs::write(
        &tr0_path,
        concat!(
            r#"{"event":"answered","question":1,"by":"mind","action":"PassPriority"}"#,
            "\n",
            r#"{"event":"over"}"#,
            "\n"
        ),
    )
    .unwrap();
    let tr1_path = work.join("transcripts/game1-seat1.jsonl");
    std::fs::write(
        &tr1_path,
        concat!(
            r#"{"event":"answered","question":1,"by":"mind","action":"PassPriority"}"#,
            "\n",
            r#"{"event":"over"}"#,
            "\n"
        ),
    )
    .unwrap();

    let models = [
        "gemini-4-argon-low".to_owned(),
        "gemini-3.8-flash-low".to_owned(),
    ];
    let expected_minds = models.clone().map(|model| baylee_gamehost::record::Mind {
        kind: baylee_gamehost::record::MindKind::LlmCli,
        provider: "agy".into(),
        model,
        effort: "low".into(),
        level: String::new(),
    });
    let proof = audit_record(&rec_path, &expected_minds.clone().map(|mind| vec![mind])).unwrap();
    for (seat, mind) in expected_minds.iter().enumerate() {
        assert_eq!(
            proof.declared[seat],
            [json!({"mind": mind, "chain_index": 0, "first_input_n": seat})]
        );
    }
    assert_eq!(proof.actual_seed, 777);
    assert_eq!(proof.input_count, 2);
    let (counts0, q0) = audit_transcript(&tr0_path, &proof.inputs[0]).unwrap();
    let (counts1, q1) = audit_transcript(&tr1_path, &proof.inputs[1]).unwrap();
    assert!(!q0 && !q1);
    let prov0 = ordered_attribution(&tr0_path, &proof.inputs[0]).unwrap();
    let prov1 = ordered_attribution(&tr1_path, &proof.inputs[1]).unwrap();
    let stop = audit_tape_rows(&read_jsonl(&tape_path).unwrap(), &proof).unwrap();
    assert_eq!(stop["inputs"], 2);

    let prov0_path = work.join("input-provenance-seat0.json");
    let prov1_path = work.join("input-provenance-seat1.json");
    let rep_receipt = work.join("replay-receipt.json");
    write_atomic_json(&prov0_path, &prov0).unwrap();
    write_atomic_json(&prov1_path, &prov1).unwrap();
    write_atomic_json(&rep_receipt, &json!({"status": "verified", "stop": stop})).unwrap();
    write_atomic_json(&work.join("room-state.json"), &json!({"status": "closed"})).unwrap();

    let pin = |p: &Path, role: &str| {
        json!({
            "path": p.canonicalize().unwrap().to_string_lossy(),
            "sha256": sha256_file(p).unwrap(),
            "role": role,
        })
    };
    let artifacts = vec![
        pin(&rec_path, "anonymized_record"),
        pin(&tr0_path, "transcript_seat0"),
        pin(&prov0_path, "executed_provenance_seat0"),
        pin(&tr1_path, "transcript_seat1"),
        pin(&prov1_path, "executed_provenance_seat1"),
        pin(&tape_path, "replay_tape"),
        pin(&rep_receipt, "replay_receipt"),
    ];
    let result = json!({
        "schema_version": 1,
        "status": "complete",
        "game_id": "game1",
        "actual_seed": proof.actual_seed,
        "terminal": proof.terminal,
        "seats": [
            {
                "seat": 0,
                "model": models[0],
                "declared_mind_verified": true,
                "declared_minds": proof.declared[0],
                "transcript_answer_counts": counts0,
                "executed_input_sources": {"seat": 1},
            },
            {
                "seat": 1,
                "model": models[1],
                "declared_mind_verified": true,
                "declared_minds": proof.declared[1],
                "transcript_answer_counts": counts1,
                "executed_input_sources": {"seat": 1},
            }
        ],
        "checks": {
            "connection": true,
            "view_boundary": true,
            "full_record": true,
            "replay": true,
            "provenance": true,
        },
        "artifacts": artifacts,
        "record_sha256": sha256_file(&rec_path).unwrap(),
        "bc_only": true,
        "ppo_eligible": false,
        "quota_limited": false,
    });
    let req = json!({
        "models": models,
    });
    validate_technical_result(&result, &req, work).unwrap();
    // A seat whose declarations are not its chain's, in order, is refused.
    let mut misplaced = result.clone();
    misplaced["seats"][1]["declared_minds"][0]["chain_index"] = json!(1);
    assert!(validate_technical_result(&misplaced, &req, work).is_err());
    let mut missing = result;
    missing["seats"][0]
        .as_object_mut()
        .unwrap()
        .remove("declared_minds");
    assert!(validate_technical_result(&missing, &req, work).is_err());
}
