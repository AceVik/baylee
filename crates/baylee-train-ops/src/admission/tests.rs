use super::*;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;

fn row(seat: u8, cursor: u32, terminal: bool) -> Value {
    json!({"game_id":1,"seat":seat,"step_cursor":cursor,"is_terminal":terminal,"is_decision":true,"outcome":if seat==0 {1.0}else{0.0},"head":0,"cand_feat":[[],[]],"pairs":[],"target":{"single_idx":0},"provenance":{"source":"Net","temperature":0.7,"thinking_steps":2,"has_logprob":true,"behavior_logprob":-0.7}})
}

fn setup(edit: impl FnOnce(&mut Vec<Value>, &mut Value)) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("train")).unwrap();
    fs::write(root.join("net.pt"), b"test checkpoint identity only").unwrap();
    let sha = files::sha256(&root.join("net.pt")).unwrap();
    let mut meta = json!({"trajectory_version":2,"encoder_version":8,"policy_provenance_version":2,"sampling_version":2,"train_shards":["steps-0.jsonl.gz"],"val_shards":[]});
    let mut rows = vec![
        row(0, 0, false),
        row(1, 0, false),
        row(0, 1, true),
        row(1, 1, true),
    ];
    edit(&mut rows, &mut meta);
    files::atomic_json(&root.join("meta.json"), &meta).unwrap();
    for file in [
        "card_structure.json",
        "card_dsl.bin",
        "card_dsl_back.bin",
        "token_dsl.bin",
    ] {
        fs::write(root.join(file), b"bound fixture").unwrap();
    }
    let mut gz = GzEncoder::new(
        File::create(root.join("train/steps-0.jsonl.gz")).unwrap(),
        Compression::fast(),
    );
    for row in rows {
        serde_json::to_writer(&mut gz, &row).unwrap();
        gz.write_all(b"\n").unwrap();
    }
    gz.finish().unwrap();
    let collection = json!({"version":1,"purpose":"on_policy_training","source_checkpoint_sha256":sha,"mode":"raw","precision":"float32","policy_provenance_version":2,"sampling_version":2,"dataset_files":seal_dataset_files(root,&meta).unwrap(),"games":[{"game_id":1,"source_game":"fixture-game-1","source_run_sha256":"b".repeat(64),"record_sha256":"c".repeat(64),"complete":true,"capped":false,"fallback_answers":0,"refused_answers":0,"unencoded_decisions":0,"seats":[{"seat":0,"role":"learner","checkpoint_sha256":sha,"thinking_steps":2,"temperature":0.7},{"seat":1,"role":"historical","checkpoint_sha256":"a".repeat(64),"thinking_steps":2,"temperature":0.7}]}]});
    let mut collection = collection;
    collection["games"][0]["source_run_sha256"] = json!("c".repeat(64));
    collection["games"][0]["record_sha256"] = json!("d".repeat(64));
    collection["games"][0]["source_game"] = json!("fixture-game");
    let manifest = root.join("collection.json");
    files::atomic_json(&manifest, &collection).unwrap();
    let checkpoint = root.join("net.pt");
    (temp, checkpoint, manifest)
}

#[test]
fn admits_only_learner_and_binds_all_content() {
    let (temp, checkpoint, manifest) = setup(|_, _| {});
    let report = admit(temp.path(), &checkpoint, &manifest).unwrap();
    assert_eq!(report["decisions"], 2);
    assert_eq!(
        report["admitted_learner_histories"],
        json!([{"game_id":1,"seat":0}])
    );
    let receipt = temp.path().join("admission.json");
    files::atomic_json(&receipt, &report).unwrap();
    assert_eq!(verify(temp.path(), &checkpoint, &receipt).unwrap(), report);
    fs::write(temp.path().join("token_dsl.bin"), b"changed").unwrap();
    assert!(verify(temp.path(), &checkpoint, &receipt).is_err());
}

#[test]
fn encoder9_admission_requires_real82_column_rows() {
    for width in [0, 80, 82] {
        let (temp, checkpoint, manifest) = setup(|rows, meta| {
            meta["encoder_version"] = json!(9);
            meta["global_feat_width"] = json!(82);
            for row in rows {
                row["state"] = json!({"global_feat":vec![0.0;width]});
            }
        });
        assert_eq!(
            admit(temp.path(), &checkpoint, &manifest).is_ok(),
            width == 82
        );
    }
}

#[test]
fn rejects_broken_history_and_off_policy_actions() {
    for variant in 0..7 {
        let (temp, checkpoint, manifest) = setup(|rows, meta| match variant {
            0 => rows[2]["step_cursor"] = json!(2),
            1 => rows[2]["is_terminal"] = json!(false),
            2 => rows[0]["provenance"]["source"] = json!("House"),
            3 => rows[0]["provenance"]["has_logprob"] = json!(false),
            4 => rows[0]["provenance"]["thinking_steps"] = json!(8),
            5 => meta["ppo_eligible"] = json!(false),
            _ => {
                rows[0]["head"] = json!(1);
                rows[0]["target"] = json!({"multi_mask":[1,0],"free_mask":[1]});
            }
        });
        assert!(
            admit(temp.path(), &checkpoint, &manifest).is_err(),
            "variant {variant}"
        );
    }
}

#[test]
fn rejects_unbound_roles_and_invalid_games() {
    for variant in 0..5 {
        let (temp, checkpoint, manifest) = setup(|_, _| {});
        let mut value = files::read_json(&manifest).unwrap();
        match variant {
            0 => value["games"][0]["capped"] = json!(true),
            1 => value["games"][0]["fallback_answers"] = json!(1),
            2 => value["games"][0]["seats"][0]["checkpoint_sha256"] = json!("b".repeat(64)),
            3 => value["purpose"] = json!("development_evaluation"),
            _ => value["games"][0]["seats"][0]["role"] = json!("unknown"),
        }
        files::atomic_json(&manifest, &value).unwrap();
        assert!(
            admit(temp.path(), &checkpoint, &manifest).is_err(),
            "variant {variant}"
        );
    }
}

#[test]
fn original_source_namespace_cannot_be_counted_as_multiple_converted_games() {
    for variant in 0..4 {
        let (temp, checkpoint, manifest) = setup(|rows, _| {
            let duplicate = rows
                .iter()
                .cloned()
                .map(|mut row| {
                    row["game_id"] = json!(2);
                    row
                })
                .collect::<Vec<_>>();
            rows.extend(duplicate);
        });
        let mut value = files::read_json(&manifest).unwrap();
        let mut second = value["games"][0].clone();
        second["game_id"] = json!(2);
        match variant {
            1 => second["record_sha256"] = json!("e".repeat(64)),
            2 => second["source_game"] = json!("different-original-game"),
            3 => second["source_run_sha256"] = json!("f".repeat(64)),
            _ => {}
        }
        value["games"].as_array_mut().unwrap().push(second);
        files::atomic_json(&manifest, &value).unwrap();
        let result = admit(temp.path(), &checkpoint, &manifest);
        if variant < 2 {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("duplicate original source game")
            );
        } else {
            let receipt = result.unwrap();
            assert_eq!(receipt["completed_games"], 2);
            assert_eq!(receipt["learner_histories"], 2);
            assert_eq!(receipt["decisions"], 4);
        }
    }
}

mod mixture {
    use super::*;
    use crate::exploration::{CONTRACT, RuleProposal, mixture_logprob, net_probability};

    const EPSILON: f64 = 0.25;

    /// The learner (seat 0) explored: its first decision played the layer's
    /// answer (`rule:mass`), its second sampled what the layer answered.
    fn explore(rows: &mut [Value], meta: &mut Value) {
        meta["behavior_mixture"] = json!({"version":1,"contract":CONTRACT,
            "supervised_policy_weight":0.0,"proposers":["net","rules","rule:mass"]});
        for (at, row) in rows.iter_mut().enumerate() {
            row["loss_weight"] = json!(0.0);
            if row["seat"] == 0 {
                let played = at == 0;
                let rule = if played {
                    RuleProposal::Differs
                } else {
                    RuleProposal::Same
                };
                let mu =
                    mixture_logprob(net_probability(true, -0.7, played), EPSILON, rule, played);
                row["behavior_mixture"] = json!({"mixture_logprob":mu,"rule":rule,
                    "played":played,"proposer":if played {2} else {0}});
            }
        }
    }

    fn declare(manifest: &Path, epsilon: f64) {
        let mut value = files::read_json(manifest).unwrap();
        value["games"][0]["behavior_mixture"] = json!({"contract":CONTRACT,"epsilon":epsilon});
        files::atomic_json(manifest, &value).unwrap();
    }

    #[test]
    fn a_consistent_mixture_is_admitted_and_counted() {
        let (temp, checkpoint, manifest) = setup(|rows, meta| explore(rows, meta));
        declare(&manifest, EPSILON);
        let report = admit(temp.path(), &checkpoint, &manifest).unwrap();
        assert_eq!(report["decisions"], 2, "π is audited as before");
        assert_eq!(report["behavior_mixture"]["explored_decisions"], 2);
        assert_eq!(report["behavior_mixture"]["rule_answers_played"], 1);
        let receipt = temp.path().join("admission.json");
        files::atomic_json(&receipt, &report).unwrap();
        assert_eq!(verify(temp.path(), &checkpoint, &receipt).unwrap(), report);
        // Without exploration the receipt names none.
        let (plain, checkpoint, manifest) = setup(|_, _| {});
        let report = admit(plain.path(), &checkpoint, &manifest).unwrap();
        assert!(report.get("behavior_mixture").is_none());
    }

    #[test]
    fn a_tampered_mixture_is_refused() {
        type Edit = fn(&mut Vec<Value>, &mut Value);
        let variants: [(&str, Edit); 9] = [
            ("μ moved", |rows, _| {
                let mu = rows[0]["behavior_mixture"]["mixture_logprob"]
                    .as_f64()
                    .unwrap();
                rows[0]["behavior_mixture"]["mixture_logprob"] = json!(mu - 0.01);
            }),
            ("π moved under the same μ", |rows, _| {
                rows[0]["provenance"]["behavior_logprob"] = json!(-0.8);
            }),
            ("played what did not differ", |rows, _| {
                rows[0]["behavior_mixture"]["rule"] = json!("same");
            }),
            ("a played answer credited to the net", |rows, _| {
                rows[0]["behavior_mixture"]["proposer"] = json!(0);
            }),
            ("a proposer outside the table", |rows, _| {
                rows[0]["behavior_mixture"]["proposer"] = json!(7);
            }),
            ("an explored decision without μ", |rows, _| {
                rows[2].as_object_mut().unwrap().remove("behavior_mixture");
            }),
            ("μ on the opponent's frame", |rows, _| {
                rows[1]["behavior_mixture"] = rows[2]["behavior_mixture"].clone();
            }),
            ("a decision with a supervised weight", |rows, _| {
                rows[1]["loss_weight"] = json!(1.0);
            }),
            ("a dataset that does not declare its mixture", |_, meta| {
                meta.as_object_mut().unwrap().remove("behavior_mixture");
            }),
        ];
        for (why, edit) in variants {
            let (temp, checkpoint, manifest) = setup(|rows, meta| {
                explore(rows, meta);
                edit(rows, meta);
            });
            declare(&manifest, EPSILON);
            assert!(admit(temp.path(), &checkpoint, &manifest).is_err(), "{why}");
        }
        // The manifest's ε is part of the check, and so is its declaration.
        let (temp, checkpoint, manifest) = setup(|rows, meta| explore(rows, meta));
        declare(&manifest, 0.3);
        assert!(
            admit(temp.path(), &checkpoint, &manifest).is_err(),
            "another ε"
        );
        let (temp, checkpoint, manifest) = setup(|rows, meta| explore(rows, meta));
        assert!(
            admit(temp.path(), &checkpoint, &manifest).is_err(),
            "frames with μ in a game that declares none"
        );
    }
}
