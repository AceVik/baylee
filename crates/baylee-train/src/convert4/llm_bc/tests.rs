use super::{LlmBc4, sha};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture {
    root: PathBuf,
    manifest: Value,
    record: Vec<u8>,
}
impl Fixture {
    fn new(origin: &str, declared_model: &str) -> Self {
        Self::counted(origin, declared_model, [Some((1, 0)), Some((1, 0))])
    }
    /// A game whose seats say they were answered (mind, house) times, or
    /// say nothing (`None`).
    fn counted(origin: &str, declared_model: &str, counts: [Option<(u64, u64)>; 2]) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "baylee-llm-bc-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let models = ["gemini-4-argon-low", "gemini-3.8-flash-low"];
        let record_rows = [
            json!({"kind":"header","preset":{"seed":33,"seats":[{},{}]}}),
            json!({"kind":"declared_mind","seat":0,"mind":{"kind":"llm_cli","provider":"agy","model":declared_model,"effort":"low"}}),
            json!({"kind":"declared_mind","seat":1,"mind":{"kind":"llm_cli","provider":"agy","model":models[1],"effort":"low"}}),
            json!({"kind":"input","n":1,"seat":0,"by":"seat","action":"MulliganKeep"}),
            json!({"kind":"input","n":2,"seat":1,"by":"seat","action":"PassPriority"}),
            json!({"kind":"end","winners":[0]}),
        ];
        let mut record_str = String::new();
        for v in &record_rows {
            use std::fmt::Write as _;
            writeln!(record_str, "{v}").unwrap();
        }
        let record = record_str.into_bytes();
        let request = json!({"attempt_id":"attempt-3","models":models,"bc_only":true,"ppo_eligible":false,"semantic_quality_gate":false});
        let mut technical = json!({"schema_version":1,"status":"complete","record_sha256":"gzip-hash","actual_seed":33,"terminal":{"draw":false,"winner_seat":0},"seats":[{"seat":0,"model":models[0],"declared_mind_verified":true,"transcript_answer_counts":{"llm":1,"house":0,"standing":0,"other":0}},{"seat":1,"model":models[1],"declared_mind_verified":true,"transcript_answer_counts":{"llm":1,"house":0,"standing":0,"other":0}}],
          "checks":{"connection":true,"view_boundary":true,"full_record":true,"replay":true,"provenance":true},
          "artifacts":[{"role":"anonymized_record","sha256":"gzip-hash"}]});
        for (seat, counts) in counts.iter().enumerate() {
            let row = technical["seats"][seat].as_object_mut().unwrap();
            match counts {
                Some((mind, house)) => {
                    row.insert(
                        "transcript_answer_counts".into(),
                        json!({"llm": mind, "house": house, "standing": 0, "other": 0}),
                    );
                }
                None => {
                    row.remove("transcript_answer_counts");
                }
            }
        }
        let mut seats = Vec::new();
        for (seat, model) in models.iter().enumerate() {
            let action = if seat == 0 {
                "MulliganKeep"
            } else {
                "PassPriority"
            };
            let text = format!(
                "{}\n{}\n",
                json!({"event":"answered","by":"mind","question":1,"action":action}),
                json!({"event":"over"})
            );
            let path = root.join(format!("transcript{seat}.jsonl"));
            std::fs::write(&path, text.as_bytes()).unwrap();
            let transcript = json!({"path":path,"sha256":sha(text.as_bytes())});
            let origins = json!({"ppo_eligible":false,"inputs":[{"record_input_n":seat+1,"action":action,
              "possible_sources":[if seat==0{origin}else{"llm"}],"ambiguous":false,"possible_send_count":1,"policy_target_admitted":false}]});
            let origins = pin(&root, &format!("origins{seat}.json"), &origins);
            technical["artifacts"].as_array_mut().unwrap().push(
                json!({"role":format!("transcript_seat{seat}"),"sha256":transcript["sha256"]}),
            );
            technical["artifacts"].as_array_mut().unwrap().push(json!({"role":format!("executed_provenance_seat{seat}"),"sha256":origins["sha256"]}));
            seats
                .push(json!({"seat":seat,"model":model,"transcript":transcript,"origins":origins}));
        }
        let receipt = json!({"status":"admitted","request":request,"technical":technical});
        let manifest = json!({"schema_version":1,"contract":"admitted_executed_llm_v1","source_game":"attempt-3","record_sha256":sha(&record),"record_artifact_sha256":"gzip-hash",
          "request":pin(&root,"request.json",&request),"technical_result":pin(&root,"technical.json",&technical),"admitted_receipt":pin(&root,"receipt.json",&receipt),"seats":seats});
        Self {
            root,
            manifest,
            record,
        }
    }
    fn load(&self) -> Result<LlmBc4, String> {
        LlmBc4::load(
            b"{\"llm_bc_version\":1}",
            &serde_json::to_vec(&self.manifest).unwrap(),
            &self.record,
            "attempt-3",
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
fn pin(root: &std::path::Path, name: &str, value: &Value) -> Value {
    let path = root.join(name);
    let bytes = serde_json::to_vec(value).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    json!({"path":path,"sha256":sha(&bytes)})
}
#[test]
fn frozen_storage_alias_preserves_original_admission_binding() {
    let mut f = Fixture::new("llm", "gemini-4-argon-low");
    f.manifest["frozen_game"] = json!("source-lane-a-record-hash");
    assert!(f.load().is_ok());
    // Renaming the storage object must never rewrite the admitted attempt.
    f.manifest["source_game"] = json!("source-lane-a-record-hash");
    assert!(
        LlmBc4::load(
            b"{\"llm_bc_version\":1}",
            &serde_json::to_vec(&f.manifest).unwrap(),
            &f.record,
            "source-lane-a-record-hash"
        )
        .is_err()
    );
}

#[test]
fn verified_inputs_are_consumed_once_and_forced_decisions_have_no_policy_label() {
    let f = Fixture::new("llm", "gemini-4-argon-low");
    let mut labels = f.load().unwrap();
    assert!(labels.finish().is_err());
    assert!(
        labels
            .input(
                1,
                0,
                &baylee_engine::choice::PlayerAction::MulliganKeep,
                false
            )
            .unwrap()
            .is_some()
    );
    assert!(
        labels
            .input(
                1,
                0,
                &baylee_engine::choice::PlayerAction::MulliganKeep,
                false
            )
            .is_err()
    );
    assert!(
        labels
            .input(
                2,
                1,
                &baylee_engine::choice::PlayerAction::PassPriority,
                true
            )
            .unwrap()
            .is_none()
    );
    labels.finish().unwrap();
}

#[test]
fn concede_is_consumed_and_verified_without_emitting_policy_target() {
    let mut f = Fixture::new("llm", "gemini-4-argon-low");
    let rows = [
        json!({"kind":"header","preset":{"seed":33,"seats":[{},{}]}}),
        json!({"kind":"declared_mind","seat":0,"mind":{"kind":"llm_cli","provider":"agy","model":"gemini-4-argon-low","effort":"low"}}),
        json!({"kind":"declared_mind","seat":1,"mind":{"kind":"llm_cli","provider":"agy","model":"gemini-3.8-flash-low","effort":"low"}}),
        json!({"kind":"input","n":1,"seat":0,"by":"seat","action":"MulliganKeep"}),
        json!({"kind":"input","n":2,"seat":1,"by":"seat","action":"Concede"}),
        json!({"kind":"end","winners":[0]}),
    ];
    let mut record_str = String::new();
    for v in &rows {
        use std::fmt::Write as _;
        writeln!(record_str, "{v}").unwrap();
    }
    f.record = record_str.into_bytes();
    f.manifest["record_sha256"] = json!(sha(&f.record));
    let text = format!(
        "{}\n{}\n",
        json!({"event":"answered","by":"mind","question":1,"action":"Concede"}),
        json!({"event":"over"})
    );
    let t_path = f.root.join("transcript1.jsonl");
    std::fs::write(&t_path, text.as_bytes()).unwrap();
    let origins = json!({
        "ppo_eligible": false,
        "inputs": [{
            "record_input_n": 2,
            "action": "Concede",
            "possible_sources": ["llm"],
            "ambiguous": false,
            "possible_send_count": 1,
            "policy_target_admitted": false
        }]
    });
    let origins_pin = pin(&f.root, "origins1.json", &origins);
    let t_sha = sha(text.as_bytes());
    f.manifest["seats"][1]["transcript"]["sha256"] = json!(t_sha);
    f.manifest["seats"][1]["origins"] = origins_pin.clone();
    let mut technical: Value =
        serde_json::from_slice(&std::fs::read(f.root.join("technical.json")).unwrap()).unwrap();
    for art in technical["artifacts"].as_array_mut().unwrap() {
        if art["role"] == "transcript_seat1" {
            art["sha256"] = json!(t_sha);
        } else if art["role"] == "executed_provenance_seat1" {
            art["sha256"] = origins_pin["sha256"].clone();
        }
    }
    let mut receipt: Value =
        serde_json::from_slice(&std::fs::read(f.root.join("receipt.json")).unwrap()).unwrap();
    receipt["technical"] = technical.clone();
    f.manifest["technical_result"] = pin(&f.root, "technical.json", &technical);
    f.manifest["admitted_receipt"] = pin(&f.root, "receipt.json", &receipt);
    let mut labels = f.load().unwrap();
    assert!(
        labels
            .input(
                1,
                0,
                &baylee_engine::choice::PlayerAction::MulliganKeep,
                false
            )
            .unwrap()
            .is_some()
    );
    assert!(
        labels
            .input(2, 1, &baylee_engine::choice::PlayerAction::Concede, false)
            .unwrap()
            .is_none()
    );
    labels.finish().unwrap();
}
#[test]
fn fabricated_origin_and_wrong_declared_model_are_rejected() {
    assert!(Fixture::new("house", "gemini-4-argon-low").load().is_err());
    assert!(Fixture::new("llm", "wrong-model").load().is_err());
}
#[test]
fn record_hash_evidence_hash_and_input_action_mismatch_are_rejected() {
    let mut f = Fixture::new("llm", "gemini-4-argon-low");
    let mut labels = f.load().unwrap();
    assert!(
        labels
            .input(
                1,
                0,
                &baylee_engine::choice::PlayerAction::MulliganTake,
                false
            )
            .is_err()
    );
    f.manifest["record_sha256"] = json!("wrong");
    assert!(f.load().is_err());
    f.manifest["record_sha256"] = json!(sha(&f.record));
    std::fs::write(f.root.join("origins0.json"), b"tampered").unwrap();
    assert!(f.load().is_err());
}
#[test]
fn ambiguity_keeps_multiple_origins_and_never_becomes_an_llm_target() {
    let transcript = vec![
        json!({"event":"answered","question":1,"by":"mind","action":"PassPriority"}),
        json!({"event":"answered","question":2,"by":"house","action":"PassPriority"}),
        json!({"event":"over"}),
    ];
    let inputs = vec![json!({"n":1,"action":"PassPriority"})];
    let origins = json!({"ppo_eligible":false,"inputs":[{"record_input_n":1,"action":"PassPriority","possible_sources":["llm","house"],"ambiguous":true,"possible_send_count":2,"policy_target_admitted":false}]});
    assert_eq!(
        super::alignment::verify(&transcript, &inputs, &origins).unwrap(),
        vec![false]
    );
}

fn codex_fixture() -> Fixture {
    bound_fixture(
        &json!({"kind":"llm_cli","provider":"codex","model":"gpt-6.1-terra","effort":"low"}),
    )
}

/// A game whose seat 0 was asked for `mind` and declared it, with the
/// requested identity receipt bound.
fn bound_fixture(mind: &Value) -> Fixture {
    let model = mind["model"].clone();
    let mut f = Fixture::new("llm", "gemini-4-argon-low");
    let mut rows: Vec<Value> = std::str::from_utf8(&f.record)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    rows[1]["mind"] = mind.clone();
    f.record = rows
        .iter()
        .fold(String::new(), |mut record, row| {
            writeln!(record, "{row}").expect("write to String");
            record
        })
        .into_bytes();
    f.manifest["record_sha256"] = json!(sha(&f.record));
    f.manifest["seats"][0]["model"] = model.clone();
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(f.root.join("request.json")).unwrap()).unwrap();
    request["models"][0] = model.clone();
    let input = json!({"requested_models":request["models"],
        "requested_minds":[rows[1]["mind"],rows[2]["mind"]]});
    let input_pin = pin(&f.root, "input.json", &input);
    f.manifest["input_receipt"] = input_pin.clone();
    let mut technical: Value =
        serde_json::from_slice(&std::fs::read(f.root.join("technical.json")).unwrap()).unwrap();
    technical["seats"][0]["model"] = model;
    technical["artifacts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"role":"input_receipt","sha256":input_pin["sha256"]}));
    f.manifest["request"] = pin(&f.root, "request.json", &request);
    f.manifest["technical_result"] = pin(&f.root, "technical.json", &technical);
    f.manifest["admitted_receipt"] = pin(
        &f.root,
        "receipt.json",
        &json!({"status":"admitted","request":request,"technical":technical}),
    );
    f
}

#[test]
fn codex_terra_needs_exact_bound_requested_identity() {
    let mut f = codex_fixture();
    assert!(f.load().is_ok());
    f.manifest.as_object_mut().unwrap().remove("input_receipt");
    assert!(f.load().is_err());
}

/// A model behind an API teaches at the effort its profile set, and like
/// Codex only with the receipt that binds what was asked for.
#[test]
fn an_api_teacher_needs_exact_bound_requested_identity() {
    for mind in [
        json!({"kind":"llm_api","provider":"anthropic","model":"claude-haiku-5-5","effort":"medium"}),
        json!({"kind":"llm_api","provider":"openai","model":"deepseek-flash","effort":"low"}),
        json!({"kind":"llm_api","provider":"openai","model":"gemini-3.5-flash-lite"}),
    ] {
        let mut f = bound_fixture(&mind);
        assert!(f.load().is_ok(), "{mind}");
        f.manifest.as_object_mut().unwrap().remove("input_receipt");
        assert!(f.load().is_err(), "{mind} without its receipt");
    }
    for spoof in [
        json!({"kind":"llm_api","provider":"elsewhere","model":"claude-haiku-5-5","effort":"low"}),
        json!({"kind":"llm_api","provider":"anthropic","model":"claude-haiku-5-5","level":"steady"}),
        json!({"kind":"house","provider":"anthropic","model":"claude-haiku-5-5"}),
    ] {
        assert!(bound_fixture(&spoof).load().is_err(), "{spoof}");
    }
}

#[test]
fn codex_provider_model_effort_kind_spoofs_are_rejected() {
    for (field, spoof) in [
        ("provider", "agy"),
        ("provider", "openai"),
        ("model", "gpt-unknown"),
        ("effort", "high"),
        ("kind", "llm_api"),
    ] {
        let mut f = codex_fixture();
        let mut rows: Vec<Value> = std::str::from_utf8(&f.record)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        rows[1]["mind"][field] = json!(spoof);
        f.record = rows
            .iter()
            .fold(String::new(), |mut record, row| {
                writeln!(record, "{row}").expect("write to String");
                record
            })
            .into_bytes();
        f.manifest["record_sha256"] = json!(sha(&f.record));
        assert!(f.load().is_err(), "{field}={spoof}");
    }
}

/// A game of `llm_collector4 collect --evaluation` may have seated any deck:
/// numbers only, never BC.
#[test]
fn an_evaluation_game_is_numbers_only_never_bc() {
    let mut f = Fixture::new("llm", "gemini-4-argon-low");
    let read = |name: &str| -> Value {
        serde_json::from_slice(&std::fs::read(f.root.join(name)).unwrap()).unwrap()
    };
    let mut request = read("request.json");
    let technical = read("technical.json");
    request["numbers_only"] = json!(true);
    let receipt = json!({"status":"admitted","request":request,"technical":technical});
    f.manifest["request"] = pin(&f.root, "request.json", &request);
    f.manifest["admitted_receipt"] = pin(&f.root, "receipt.json", &receipt);
    let error = f.load().err().unwrap();
    assert!(error.contains("numbers only"), "{error}");
}

/// The corpus rule at admission: a game the house answered more than 5 % of
/// a seat's questions of is refused with its reason, exactly 5 % is kept, a
/// game that does not say who answered is refused, and a clean game converts
/// as before (`house_share`).
#[test]
fn a_game_the_house_answered_over_five_percent_of_a_seat_of_is_refused() {
    let over = Fixture::counted("llm", "gemini-4-argon-low", [Some((120, 0)), Some((94, 6))]);
    let Err(reason) = over.load() else {
        panic!("a 6 % house seat was admitted");
    };
    assert_eq!(
        reason,
        "LLM game attempt-3 refused: house answered 6 of 100 (6.0 %) of seat 1's questions, \
         over 5 %"
    );
    let edge = Fixture::counted("llm", "gemini-4-argon-low", [Some((95, 5)), Some((19, 1))]);
    assert!(edge.load().is_ok(), "exactly 5 % is kept");
    let silent = Fixture::counted("llm", "gemini-4-argon-low", [Some((1, 0)), None]);
    assert!(
        silent
            .load()
            .err()
            .is_some_and(|reason| reason.contains("seat 1 does not say who answered")),
        "a game without its counts was admitted"
    );
    // A clean game is admitted as before the rule (every other test here is
    // one, its seats answered by their minds alone).
    assert!(Fixture::new("llm", "gemini-4-argon-low").load().is_ok());
}
