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
        let mut technical = json!({"schema_version":1,"status":"complete","record_sha256":"gzip-hash","actual_seed":33,"terminal":{"draw":false,"winner_seat":0},"seats":[{"seat":0,"model":models[0],"declared_mind_verified":true},{"seat":1,"model":models[1],"declared_mind_verified":true}],
          "checks":{"connection":true,"view_boundary":true,"full_record":true,"replay":true,"provenance":true},
          "artifacts":[{"role":"anonymized_record","sha256":"gzip-hash"}]});
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
    let mut f = Fixture::new("llm", "gemini-4-argon-low");
    let mut rows: Vec<Value> = std::str::from_utf8(&f.record)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    rows[1]["mind"]["provider"] = json!("codex");
    rows[1]["mind"]["model"] = json!("gpt-6.1-terra");
    f.record = rows
        .iter()
        .fold(String::new(), |mut record, row| {
            writeln!(record, "{row}").expect("write to String");
            record
        })
        .into_bytes();
    f.manifest["record_sha256"] = json!(sha(&f.record));
    f.manifest["seats"][0]["model"] = json!("gpt-6.1-terra");
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(f.root.join("request.json")).unwrap()).unwrap();
    request["models"][0] = json!("gpt-6.1-terra");
    let input = json!({"requested_models":request["models"],
        "requested_minds":[rows[1]["mind"],rows[2]["mind"]]});
    let input_pin = pin(&f.root, "input.json", &input);
    f.manifest["input_receipt"] = input_pin.clone();
    let mut technical: Value =
        serde_json::from_slice(&std::fs::read(f.root.join("technical.json")).unwrap()).unwrap();
    technical["seats"][0]["model"] = json!("gpt-6.1-terra");
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
