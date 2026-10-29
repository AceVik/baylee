use super::*;
use baylee_train as train;
use flate2::read::GzDecoder;
use std::io::Read as _;
#[path = "../../../../tests/support/research.rs"]
mod fixtures;

#[cfg(target_os = "linux")]
mod attempt_ledger;

struct Fixture {
    root: PathBuf,
    run: Value,
    rows: Vec<Value>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "baylee-research-conversion-{}",
            uuid::Uuid::now_v7()
        ));
        fs::create_dir_all(root.join("source/records")).unwrap();
        let model = json!({"model_family":"v5","model_version":5,"encoder_version":7,
            "architecture":"baylee_ai_v5_relational_v2","source_checkpoint_sha256":"a".repeat(64),
            "onnx_sha256":"b".repeat(64),"metadata_sha256":"c".repeat(64),
            "depth_graph_sha256":{"0":"d".repeat(64),"2":"e".repeat(64)}});
        let run = json!({"policy_mode":"raw","policy_contract":"raw_neural_sampler2_v1",
            "policy_provenance_version":2,"sampling_version":2,"thinking_steps":2,"temperature":0.7,
            "training_data":false,"seats":2,"held_out_cards":false,"holdout_tier":"","deal_offset":0,"planned_deals":3,
            "model_identity":model,"evaluator_revision":baylee_build::COMMIT,"evaluator_dirty":baylee_build::DIRTY,
            "collection_contract":{"version":2,"purpose":"failure_terminal_research_v1","precision":"float32",
                "research_target_contract":contract(),"seat_rng_rule":"independent_policy_seed_xor_physical_seat_v1",
                "learner_model_identity":model,"opponent_model_identity":model,"opponent_role":"self_play"}});
        files::atomic_json(&root.join("source/run.json"), &run).unwrap();
        let run_sha = files::sha256(&root.join("source/run.json")).unwrap();
        let source = [
            fixtures::loop_episode(0),
            fixtures::loop_episode(1),
            fixtures::natural_episode(0),
        ];
        let mut shard = Vec::new();
        let mut rows = Vec::new();
        for (index, fixture) in source.into_iter().enumerate() {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(&fixture.record).unwrap();
            let member = encoder.finish().unwrap();
            let focal = index % 2;
            let roles: Vec<_> = (0..2)
                .map(|seat| {
                    json!({"seat":seat,"role":if seat==focal {"learner"}else{"self_play"},
                "checkpoint_sha256":"a".repeat(64),"thinking_steps":2,"temperature":0.7})
                })
                .collect();
            rows.push(json!({"game":format!("fresh-{index}"),"i":index,"seed":12,"policy_seed":19,
                "source_run_sha256":run_sha,"record_sha256":sha(&fixture.record),"focal_seat":focal,"actor_roles":roles,
                "shard":"fixture.gz","offset":shard.len(),"len":member.len(),"fallback_answers":0,"refused_answers":0,
                "complete":fixture.episode["termination_kind"]=="mtg_terminal","capped":false,
                "outcome":fixture.episode,"research_episode":fixture.episode,"rollout_sidecar":fixture.sidecar,"fallback_events":[]}));
            shard.extend(member);
        }
        fs::write(root.join("source/records/fixture.gz"), shard).unwrap();
        let fixture = Self { root, run, rows };
        fixture.index();
        fixture
    }
    fn index(&self) {
        let mut bytes = Vec::new();
        for row in &self.rows {
            serde_json::to_writer(&mut bytes, row).unwrap();
            bytes.push(b'\n');
        }
        fs::write(self.root.join("source/games.jsonl"), bytes).unwrap();
        files::atomic_json(&self.root.join("source/run.complete.json"),&json!({"version":2,"purpose":"failure_terminal_research_v1",
            "source_run_sha256":files::sha256(&self.root.join("source/run.json")).unwrap(),
            "games_index_sha256":files::sha256(&self.root.join("source/games.jsonl")).unwrap(),"recorded_deals":3,"planned_deals":3})).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn fresh_conversion_retains_all_attempts_and_publishes_only_separate_known_target_histories() {
    let fixture = Fixture::new();
    let out = fixture.root.join("converted");
    convert(&Args {
        runs: vec![fixture.root.join("source")],
        attempt_ledger_runs: vec![],
        out: out.clone(),
    })
    .unwrap();
    let manifest = files::read_json(&out.join("research.manifest.json")).unwrap();
    assert_eq!(manifest["planned_attempts"], 3);
    assert_eq!(manifest["unknown_attempts"], 1);
    assert_eq!(manifest["target_episodes"].as_array().unwrap().len(), 2);
    assert_eq!(manifest["ppo_eligible"], false);
    let ledger: Vec<Value> = fs::read_to_string(out.join("attempts.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(ledger.len(), 3);
    assert_eq!(
        ledger[1]["target_disposition"],
        "excluded_unknown_no_target_rows"
    );
    assert!(ledger[1]["episode_file"].is_null());
    assert!(ledger[1]["returns"]["training_utility"].is_null());
    let mut fingerprint = manifest.clone();
    fingerprint
        .as_object_mut()
        .unwrap()
        .remove("content_fingerprint");
    assert_eq!(
        manifest["content_fingerprint"],
        sha(&serde_json::to_vec(&fingerprint).unwrap())
    );
    for (name, digest) in manifest["output_files"].as_object().unwrap() {
        assert_eq!(*digest, files::sha256(&out.join(name)).unwrap());
    }
    let name = ledger[0]["episode_file"].as_str().unwrap();
    let mut rows = String::new();
    GzDecoder::new(File::open(out.join(name)).unwrap())
        .read_to_string(&mut rows)
        .unwrap();
    let first: Value = serde_json::from_str(rows.lines().next().unwrap()).unwrap();
    assert!(first["features"].get("outcome").is_none());
    assert_eq!(first["returns"]["training_utility"], 0.0);
    assert!(first["returns"]["mtg_outcome"].is_null());
    assert!(out.join("research.complete.json").exists());
    assert!(!out.join("meta.json").exists());
    assert!(!out.join("collection.json").exists());
    assert!(baylee_train_ops::trajectory::cache::build(&out, "train", &[]).is_err());
    assert!(
        crate::on_policy::RunBinding::load(&fixture.root.join("source"), &fixture.run).is_err()
    );
    assert!(
        convert(&Args {
            runs: vec![fixture.root.join("source")],
            attempt_ledger_runs: vec![],
            out
        })
        .is_err()
    );
}

#[test]
fn source_contract_roles_denominator_and_file_tampering_fail_closed() {
    let mut fixture = Fixture::new();
    let source = fixture.root.join("source");
    let run = source::Run::load(&source).unwrap();
    let bytes = run.games[0].record().unwrap();
    run.games[0].verify(&bytes, 7).unwrap();
    fixture.rows[0]["actor_roles"][0]["role"] = json!("self_play");
    fixture.index();
    assert!(source::Run::load(&source).is_err());
    assert!(run.recheck().is_err());
    fixture.rows.pop();
    fixture.index();
    assert!(source::Run::load(&source).is_err());
    let mut ordinary = fixture.run.clone();
    ordinary["collection_contract"]["purpose"] = json!("on_policy_training");
    files::atomic_json(&source.join("run.json"), &ordinary).unwrap();
    assert!(source::Run::load(&source).is_err());
}
