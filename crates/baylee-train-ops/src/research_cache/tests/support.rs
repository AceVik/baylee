//! Synthetic storage fixtures: no engine outcome or numerical LP is certified.
use super::ops;
use flate2::{Compression, write::GzEncoder};
use ops::{
    files,
    research_contract::{FRAME_CONTRACT, contract},
};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, fs, io::Write as _, path::PathBuf};

pub fn sha(bytes: &[u8]) -> String {
    files::hex(&Sha256::digest(bytes))
}
pub fn jd(value: &Value) -> String {
    sha(&serde_json::to_vec(value).unwrap())
}
pub struct Fixture {
    pub _temporary: tempfile::TempDir,
    pub source: PathBuf,
    pub converted: PathBuf,
    pub cache: PathBuf,
    pub expected: String,
    pub raw: Vec<Vec<u8>>,
}

impl Fixture {
    #[allow(
        clippy::too_many_lines,
        reason = "Complete synthetic producer/ledger/conversion publication fixture"
    )]
    pub fn new(zero_targets: bool) -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("producer");
        let converted = temporary.path().join("converted");
        let cache = temporary.path().join("cache");
        fs::create_dir_all(source.join("records")).unwrap();
        fs::create_dir_all(converted.join("episodes")).unwrap();
        let model = json!({"encoder_version":7,"source_checkpoint_sha256":"a".repeat(64)});
        let run = json!({"collection_contract":{"version":2,"purpose":"failure_terminal_research_v1",
            "research_target_contract":contract(),"precision":"float32","seat_rng_rule":"independent_policy_seed_xor_physical_seat_v1",
            "learner_model_identity":model,"opponent_model_identity":model,"opponent_role":"self_play"},
            "training_data":false,"seats":2,"policy_mode":"raw","policy_contract":"raw_neural_sampler2_v1",
            "sampling_version":2,"policy_provenance_version":2,"thinking_steps":2,"temperature":0.7,
            "model_identity":model,"held_out_cards":false,"holdout_tier":"","planned_deals":5,"deal_offset":0,
            "evaluator_revision":"synthetic-storage-fixture","evaluator_dirty":true});
        files::atomic_json(&source.join("run.json"), &run).unwrap();
        let run_sha = files::sha256(&source.join("run.json")).unwrap();
        let identity = json!({"pid":u32::MAX,"start_ticks":1,"boot_id":"synthetic-dead-producer"});
        let planned: Vec<_> = (0..5).map(|deal| json!({"deal":deal,"seed":12,"policy_seed":19,"focal_seat":deal%2,"deck_keys":["left","right"]})).collect();
        files::atomic_json(
            &source.join("attempts.plan.json"),
            &json!({"schema":"failure_terminal_attempt_plan_v1",
            "source_run_sha256":run_sha,"producer_identity":identity,"attempts":planned}),
        )
        .unwrap();
        let plan_sha = files::sha256(&source.join("attempts.plan.json")).unwrap();
        drop(files::lock(&source.join("attempts.live.lock")).unwrap());
        let episodes = [
            episode("learner_announcement_loop", Some(0.0), &Value::Null),
            episode("unknown", None, &Value::Null),
            episode("mtg_terminal", Some(1.0), &json!({"kind":"won","seat":0})),
            episode("unknown", None, &Value::Null),
        ];
        let raw_record = b"{\"kind\":\"header\"}\n";
        let mut gzip = GzEncoder::new(Vec::new(), Compression::fast());
        gzip.write_all(raw_record).unwrap();
        let compressed = gzip.finish().unwrap();
        fs::write(source.join("records/records.gz"), &compressed).unwrap();
        let mut games = Vec::new();
        let mut index = Vec::new();
        for (deal, episode) in episodes.iter().enumerate() {
            let directory = source.join("attempts").join(format!("{deal:020}"));
            fs::create_dir_all(&directory).unwrap();
            files::atomic_json(&directory.join("claim.json"), &json!({"schema":"failure_terminal_attempt_plan_v1","plan_sha256":plan_sha,
                "source_run_sha256":run_sha,"producer_identity":identity,"deal":deal,"worker":deal})).unwrap();
            if deal < 3 && !zero_targets {
                let roles: Vec<_> = (0..2).map(|seat| json!({"seat":seat,"role":if seat==deal%2 {"learner"}else{"self_play"},"checkpoint_sha256":"a".repeat(64),"thinking_steps":2,"temperature":0.7})).collect();
                let game = json!({"i":deal,"game":format!("synthetic-{deal}"),"seed":12,"policy_seed":19,"focal_seat":deal%2,
                    "source_run_sha256":run_sha,"record_sha256":sha(raw_record),"shard":"records.gz","offset":0,"len":compressed.len(),
                    "research_episode":episode,"outcome":episode,"rollout_sidecar":[],"actor_roles":roles,"fallback_answers":0});
                files::atomic_json(&directory.join("game.json"), &game).unwrap();
                serde_json::to_writer(&mut index, &game).unwrap();
                index.push(b'\n');
                games.push(game);
            }
        }
        index.extend_from_slice(b"{\"i\":3,\"orphan\":true}\n{\"i\":");
        fs::write(source.join("games.jsonl"), index).unwrap();
        fs::write(source.join("records/orphan.partial"), b"partial evidence").unwrap();
        let job = temporary.path().join("job.json");
        files::atomic_json(&job, &json!({"guard":"rust-owned-group-v1","cleanup_verified":true,"status":"quarantined","reason":"deadline","child_identity":identity})).unwrap();
        let ledger = ops::research_attempts::finalize(&source, &job).unwrap();
        let mut source_seals: BTreeMap<PathBuf, String> =
            serde_json::from_value(ledger["source_files"].clone()).unwrap();
        for name in ["attempts.final.json", "attempts.complete.json"] {
            source_seals.insert(
                source.join(name),
                files::sha256(&source.join(name)).unwrap(),
            );
        }
        let mut attempts = Vec::new();
        let mut targets = Vec::new();
        let mut raw = Vec::new();
        for (deal, accounting) in ledger["attempts"].as_array().unwrap().iter().enumerate() {
            let status = accounting["status"].as_str().unwrap();
            let ret = if status == "recorded" {
                return_value(&episodes[deal])
            } else if status == "operational_unknown" {
                return_value(&episode("unknown", None, &Value::Null))
            } else {
                Value::Null
            };
            let known = !ret.is_null() && !ret["training_utility"].is_null();
            let mut receipt = json!({"attempt_id":deal+1,"source_run_sha256":run_sha,"source_index":deal,
                "source_attempt_status":status,"assigned":accounting["assigned"],"source_attempt_ledger_entry":accounting,"planned_attempt":planned[deal],
                "episode_id":null,"source_game":null,"source_record_sha256":null,"source_sidecars_sha256":null,"source_episode_sha256":null,
                "returns":ret,"episode_file":null,"frames":0,"actor_decisions":0,
                "unknown_reason":accounting["unknown_reason"],
                "target_disposition":match status {"recorded" if known=>"verified_research_return","recorded"=>"excluded_unknown_no_target_rows","operational_unknown"=>"excluded_operational_unknown_no_target_rows",_=>"not_started_no_target_rows"}});
            if status == "recorded" {
                let game = &games[deal];
                receipt["episode_id"] = json!(deal + 1);
                receipt["source_game"] = game["game"].clone();
                receipt["source_record_sha256"] = game["record_sha256"].clone();
                receipt["source_sidecars_sha256"] = json!(jd(&game["rollout_sidecar"]));
                receipt["source_episode_sha256"] = json!(jd(&game["research_episode"]));
                receipt["source_row_sha256"] = json!(jd(game));
                receipt["actor_roles"] = game["actor_roles"].clone();
            }
            if known {
                let name = format!("episodes/episode-{}.research.jsonl.gz", deal + 1);
                let mut file = GzEncoder::new(
                    fs::File::create(converted.join(&name)).unwrap(),
                    Compression::fast(),
                );
                for cursor in 0..5 {
                    let frame = frame(deal + 1, cursor, &ret);
                    let bytes =
                        format!("  {}  ", serde_json::to_string(&frame).unwrap()).into_bytes();
                    file.write_all(&bytes).unwrap();
                    file.write_all(b"\n").unwrap();
                    raw.push(bytes);
                }
                file.finish().unwrap();
                receipt["episode_file"] = json!(name);
                receipt["frames"] = json!(5);
                receipt["actor_decisions"] = json!(2);
                targets.push(receipt.clone());
            }
            attempts.push(receipt);
        }
        let mut bytes = Vec::new();
        for row in &attempts {
            serde_json::to_writer(&mut bytes, row).unwrap();
            bytes.push(b'\n');
        }
        fs::write(converted.join("attempts.jsonl"), bytes).unwrap();
        files::atomic_json(&converted.join("card_structure.json"), &json!({"roots":{}})).unwrap();
        for name in [
            "card_dsl.bin",
            "card_dsl_back.bin",
            "token_dsl.bin",
            "card_abilities.bin",
        ] {
            fs::write(converted.join(name), [0; 4]).unwrap();
        }
        let mut outputs = BTreeMap::new();
        for name in [
            "attempts.jsonl",
            "card_structure.json",
            "card_dsl.bin",
            "card_dsl_back.bin",
            "token_dsl.bin",
            "card_abilities.bin",
        ] {
            outputs.insert(
                name.to_owned(),
                files::sha256(&converted.join(name)).unwrap(),
            );
        }
        for receipt in &targets {
            let name = receipt["episode_file"].as_str().unwrap();
            outputs.insert(
                name.to_owned(),
                files::sha256(&converted.join(name)).unwrap(),
            );
        }
        let mut counts = ledger["counts"].clone();
        counts["recorded_unknown"] = json!(usize::from(!zero_targets));
        counts["target_episodes"] = json!(targets.len());
        let mut manifest = json!({"schema":"failure_terminal_research_conversion_v2","status":"conversion_only_unadmitted",
            "attempt_denominator_contract":"failure_terminal_attempt_ledger_v1","operational_abort_ledger_supported":true,"ppo_eligible":false,"ordinary_target_cache_compatible":false,
            "target_contract":contract(),"frame_contract":FRAME_CONTRACT,"preprocessing_version":1,"sampling_version":2,"policy_provenance_version":2,"encoder_version":7,
            "learner_model_identity":model,"planned_attempts":5,"unknown_attempts":if zero_targets {4}else{2},"attempt_counts":counts,"target_episodes":targets,"output_files":outputs,
            "source_runs":[{"directory":source,"source_run_sha256":run_sha,"games_index_sha256":files::sha256(&source.join("games.jsonl")).unwrap(),
                "collection_contract":run["collection_contract"],"evaluator_revision":run["evaluator_revision"],"evaluator_dirty":true,"source_files":source_seals,"finalized_attempt_ledger":ledger}]});
        manifest["content_fingerprint"] = json!(jd(&manifest));
        files::atomic_json(&converted.join("research.manifest.json"), &manifest).unwrap();
        let expected = files::sha256(&converted.join("research.manifest.json")).unwrap();
        files::atomic_json(&converted.join("research.complete.json"), &json!({"schema":"failure_terminal_research_conversion_receipt_v1","manifest_sha256":expected,"content_fingerprint":manifest["content_fingerprint"],"admitted":false})).unwrap();
        Self {
            _temporary: temporary,
            source,
            converted,
            cache,
            expected,
            raw,
        }
    }
}

fn episode(kind: &str, utility: Option<f32>, outcome: &Value) -> Value {
    json!({"kind":"research_termination","target_contract":contract(),"termination_kind":kind,"training_utility":utility,"mtg_outcome":outcome,"bootstrap":false})
}
fn return_value(episode: &Value) -> Value {
    json!({"termination_kind":episode["termination_kind"],"training_utility":episode["training_utility"],"mtg_outcome":episode["mtg_outcome"],"bootstrap":false})
}
fn frame(episode: usize, cursor: usize, ret: &Value) -> Value {
    let decision = (1..=3).contains(&cursor);
    let candidate = cursor == 1 || cursor == 2;
    let head = match cursor {
        1 => 1,
        3 => 3,
        _ => 0,
    };
    let count = if candidate { 2 } else { 0 };
    json!({"schema":FRAME_CONTRACT,"returns":ret,"training_terminal":cursor==4,"actor_eligible":candidate,
        "features":{"game_id":episode,"seat":0,"step_cursor":cursor,"turn":1,"is_decision":decision,"loss_weight":f32::from(decision),"head":head,"pending_kind":0,
        "state":{"entity_feat":[],"entity_object_ids":[],"entity_group_members":[],"entity_card_ids":[],"entity_face_ids":[],"entity_token_ids":[],"entity_rule_refs":[],"entity_zone_ids":[],"entity_rel_controller":[],
            "seat_feat":vec![vec![0.;64];8],"global_feat":vec![0.;80],"edges":[]},
        "cand_feat":vec![vec![0.;48];count],"cand_entity_idx":vec![-1;count],"cand_rel_seat_idx":vec![-1;count],"cand_card_id":vec![1;count],"cand_face_id":vec![0;count],"cand_ability_idx":vec![-1;count],"cand_rule_refs":vec![Vec::<Value>::new();count],"pairs":[],
        "target":{"single_idx":u32::from(cursor==2),"multi_mask":if cursor==1 {vec![1,0]}else{vec![]},"free_mask":if cursor==1 {vec![0,0]}else{vec![]},"pick_sequence":if cursor==1 {vec![1,0]}else{vec![]},"scalar_norm":0.5},
        "provenance":{"source":if decision {"Net"}else{"Forced"},"has_logprob":candidate,"behavior_logprob":if candidate {-0.7}else{0.},"thinking_steps":if decision {2}else{0},"temperature":if decision {0.7}else{0.}}}})
}
