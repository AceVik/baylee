use super::{atomic_json, progress, publication::*, read_json};
use anyhow::Result;
use serde_json::{Value, json};
use std::path::PathBuf;

struct Fixture {
    root: PathBuf,
    model: PathBuf,
    /// What the fixture made: its root, and for [`Fixture::through_symlink`]
    /// the directory the root links to.
    made: Vec<PathBuf>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for path in &self.made {
            if path.is_symlink() {
                std::fs::remove_file(path).unwrap();
            } else {
                std::fs::remove_dir_all(path).unwrap();
            }
        }
    }
}
impl Fixture {
    fn new() -> Result<Self> {
        let root =
            std::env::temp_dir().join(format!("baylee-publication-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&root)?;
        Self::at(root.clone(), vec![root])
    }
    /// A fixture whose root is a symlink to the directory that holds it, so
    /// every path a job records runs through the link, on any system.
    #[cfg(unix)]
    fn through_symlink() -> Result<Self> {
        let id = uuid::Uuid::now_v7();
        let real = std::env::temp_dir().join(format!("baylee-publication-real-{id}"));
        let link = std::env::temp_dir().join(format!("baylee-publication-link-{id}"));
        std::fs::create_dir_all(&real)?;
        std::os::unix::fs::symlink(&real, &link)?;
        Self::at(link.clone(), vec![link, real])
    }
    fn at(root: PathBuf, made: Vec<PathBuf>) -> Result<Self> {
        let model = root.join("candidate");
        std::fs::create_dir_all(&model)?;
        for (name, bytes) in [
            ("net.pt", "frozen"),
            ("model.k0.onnx", "passive"),
            ("model.k1.onnx", "decision"),
            ("model.onnx", "decision"),
            ("card_structure.json", "structure"),
        ] {
            std::fs::write(model.join(name), bytes)?;
        }
        atomic_json(
            &model.join("model.onnx.json"),
            &json!({"graphs":{"0":"model.k0.onnx","1":"model.k1.onnx"},"parity_max_abs_diff":0.0,"checked_frames":1}),
        )?;
        atomic_json(
            &root.join("progress.json"),
            &json!({"best_development_candidate":model,"best_development_score":0.5,"rounds":[]}),
        )?;
        Ok(Self { root, model, made })
    }
    fn evaluation(&self, label: &str, shape: &str, utility: f64) -> Result<()> {
        let g: Value = serde_json::from_str(include_str!("fixtures/python_golden.json"))?;
        let mut v = g["candidate"].clone();
        v["model"] = json!(self.model);
        v["decks_manifest"] = json!("decks.json");
        for row in v["deal_details"].as_array_mut().unwrap() {
            row["net_utility"] = json!(utility);
        }
        let path = self.root.join(format!("evaluation/{label}-{shape}.json"));
        atomic_json(&path, &v)?;
        let mut inputs = serde_json::Map::new();
        inputs.insert("decks.json".into(), json!("frozen-decks"));
        for name in [
            "net.pt",
            "model.onnx.json",
            "model.k0.onnx",
            "model.k1.onnx",
        ] {
            let p = self.model.join(name);
            inputs.insert(p.to_str().unwrap().into(), json!(digest(&p)?));
        }
        atomic_json(
            &self
                .root
                .join(format!("jobs/eval-{label}-{shape}/job.json")),
            &json!({"status":"complete","executable_sha256":"engine","inputs":inputs,"command":["eval4","--raw-policy","--out",path,"--model",self.model,"--decks-manifest","decks.json"]}),
        )?;
        Ok(())
    }
    fn confirmation(&self, delta: f64) -> Result<()> {
        atomic_json(
            &self.root.join("confirmation-selection.json"),
            &json!({"model":self.model,"model_sha256":digest(&self.model.join("net.pt"))?,"selected_before_confirmation":true}),
        )?;
        for shape in ["duels", "commander"] {
            self.evaluation("initial", shape, 0.5)?;
            self.evaluation("confirmation-initial", shape, 0.5)?;
            self.evaluation("confirmation-selected", shape, 0.5 + delta)?;
        }
        Ok(())
    }
}

#[test]
fn confirmation_uses_raw_verified_outcomes_and_fails_closed_on_changed_graphs() -> Result<()> {
    let f = Fixture::new()?;
    assert_eq!(
        confirmation_assessment(&f.root, &f.model)?["status"],
        "pending"
    );
    for (delta, veto) in [(0.0, true), (-0.01, true), (0.001, false)] {
        f.confirmation(delta)?;
        let report = confirmation_assessment(&f.root, &f.model)?;
        assert_eq!(report["veto"], veto);
        assert!((report["mean_paired_utility_difference"].as_f64().unwrap() - delta).abs() < 1e-12);
    }
    assert_eq!(
        confirmation_assessment(&f.root, &f.root.join("other"))?["status"],
        "not_this_candidate"
    );
    std::fs::write(f.model.join("model.k0.onnx"), "mutated")?;
    assert!(
        confirmation_assessment(&f.root, &f.model)
            .unwrap_err()
            .to_string()
            .contains("input changed")
    );
    Ok(())
}

#[test]
fn veto_is_reported_before_same_model_shortcut_and_preserves_pin_bytes() -> Result<()> {
    let f = Fixture::new()?;
    f.confirmation(-0.01)?;
    let output = f.root.join("selected-model.json");
    atomic_json(
        &output,
        &json!({"model":f.model,"score":0.5,"benchmark_id":"old"}),
    )?;
    let before = std::fs::read(&output)?;
    assert!(!publish(&f.root, &output)?);
    assert_eq!(std::fs::read(&output)?, before);
    let status = read_json(&f.root.join("selected-model.json.confirmation-status.json"))?;
    assert_eq!(
        status["decision"],
        "confirmation_veto_existing_selection_requires_review"
    );
    assert_eq!(status["review_required"], true);
    std::fs::remove_file(&output)?;
    assert!(!publish(&f.root, &output)?);
    assert!(!output.exists());
    Ok(())
}

#[test]
fn manifest_and_publication_verify_real_inputs_and_completed_jobs() -> Result<()> {
    let f = Fixture::new()?;
    for shape in ["duels", "commander"] {
        f.evaluation("initial", shape, 0.5)?;
    }
    let value = manifest(&f.root)?;
    assert_eq!(value["score"], 0.5);
    assert_eq!(value["sha256"]["net.pt"], digest(&f.model.join("net.pt"))?);
    let output = f.root.join("selected-model.json");
    assert!(publish(&f.root, &output)?);
    assert!(!publish(&f.root, &output)?);
    let inspected = progress::inspect(&f.root)?;
    assert_eq!(inspected["evaluations"].as_object().unwrap().len(), 2);
    assert!(
        f.root
            .join("evaluation/initial-duels.uncertainty.json")
            .exists()
    );
    let (v, job) = (
        read_json(&f.root.join("evaluation/initial-duels.json"))?,
        read_json(&f.root.join("jobs/eval-initial-duels/job.json"))?,
    );
    verified_model_inputs(&v, &job)?;
    std::fs::write(f.model.join("model.onnx"), "wrong alias")?;
    assert!(verified_model_inputs(&v, &job).is_err());
    Ok(())
}

#[test]
fn research_or_unknown_critic_never_replaces_a_live_selection() -> Result<()> {
    let f = Fixture::new()?;
    let metadata_path = f.model.join("model.onnx.json");
    let original = read_json(&metadata_path)?;
    let output = f.root.join("selected-model.json");
    atomic_json(
        &output,
        &json!({"model":f.root.join("existing-live"),"score":0.0}),
    )?;
    let before = std::fs::read(&output)?;
    for extra in [
        json!({"critic_semantics":null}),
        json!({"critic_semantics":{"kind":"unknown"}}),
        json!({"critic_semantics":{"kind":"failure_terminal_research_utility_v1",
            "objective":"failure_terminal_utility_minus_frozen_source_value_uncentered_v1",
            "target_contract":"failure_terminal_research_v1"}}),
        json!({"critic_semantics":{"kind":"mtg_side_outcome_utility_v1","objective":"conflict"}}),
        json!({"research_ppo_contract":null}),
        json!({"critic_semantics":{"kind":"mtg_side_outcome_utility_v1"},"research_ppo_progress":{}}),
        json!({}),
        json!({"critic_semantics":{"kind":"mtg_side_outcome_utility_v1"}}),
    ] {
        let mut metadata = original.clone();
        metadata
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        atomic_json(&metadata_path, &metadata)?;
        // Re-seal the actual metadata in otherwise valid completed evaluations:
        // refusal must concern its target, not a stale file digest.
        for shape in ["duels", "commander"] {
            f.evaluation("initial", shape, 0.5)?;
        }
        if extra == json!({})
            || extra == json!({"critic_semantics":{"kind":"mtg_side_outcome_utility_v1"}})
        {
            manifest(&f.root)?;
        } else {
            assert!(
                publish(&f.root, &output)
                    .unwrap_err()
                    .to_string()
                    .contains("critic semantics")
            );
            assert_eq!(std::fs::read(&output)?, before);
        }
    }
    Ok(())
}

#[test]
fn concurrent_incomplete_logs_are_ignored_and_failed_jobs_finish_inspection() -> Result<()> {
    let f = Fixture::new()?;
    let job = f.root.join("jobs/supervised");
    std::fs::create_dir_all(&job)?;
    std::fs::write(
        job.join("stdout.log"),
        "[train4] step=20\n[validation4] {\"val_mse\":0.2}\n[validation4] {broken",
    )?;
    atomic_json(
        &job.join("job.json"),
        &json!({"status":"quarantined","reason":"failed"}),
    )?;
    let value = progress::inspect(&f.root)?;
    assert_eq!(value["finished"], true);
    assert_eq!(value["validation"]["val_mse"], 0.2);
    assert_eq!(value["latest_step"], "[train4] step=20");
    Ok(())
}

#[test]
fn lower_scores_and_incomparable_benchmarks_preserve_current_selection() -> Result<()> {
    let f = Fixture::new()?;
    for shape in ["duels", "commander"] {
        f.evaluation("initial", shape, 0.5)?;
    }
    let candidate = manifest(&f.root)?;
    let output = f.root.join("selected.json");
    let mut current =
        json!({"model":f.root.join("old"),"score":0.6,"benchmark_id":candidate["benchmark_id"]});
    atomic_json(&output, &current)?;
    assert!(!publish(&f.root, &output)?);
    assert_eq!(read_json(&output)?, current);
    current["score"] = json!(0.4);
    current["benchmark_id"] = json!("different");
    atomic_json(&output, &current)?;
    assert!(
        publish(&f.root, &output)
            .unwrap_err()
            .to_string()
            .contains("Different benchmark")
    );
    assert_eq!(read_json(&output)?, current);
    current["benchmark_id"] = candidate["benchmark_id"].clone();
    atomic_json(&output, &current)?;
    assert!(publish(&f.root, &output)?);
    // The manifest names the model by canonical path, as every comparison
    // here reads it; a temporary directory is a symlink on macOS.
    assert_eq!(read_json(&output)?["model"], json!(f.model.canonicalize()?));
    Ok(())
}

#[test]
fn partial_exports_and_mutated_checkpoint_are_never_selected() -> Result<()> {
    let f = Fixture::new()?;
    for shape in ["duels", "commander"] {
        f.evaluation("initial", shape, 0.5)?;
    }
    manifest(&f.root)?;
    std::fs::write(f.model.join("net.pt"), "changed")?;
    assert!(
        manifest(&f.root)
            .unwrap_err()
            .to_string()
            .contains("input changed")
    );
    std::fs::write(f.model.join("net.pt"), "frozen")?;
    std::fs::remove_file(f.model.join("model.k0.onnx"))?;
    assert!(manifest(&f.root).is_err());
    Ok(())
}

/// Jobs record their inputs as declared, here through a symlinked root (as
/// every macOS temporary directory is, under `/var`): publication and the
/// confirmation match them by canonical path, and still refuse a checkpoint
/// changed since its comparison.
#[cfg(unix)]
#[test]
fn a_symlinked_root_publishes_and_still_refuses_a_changed_checkpoint() -> Result<()> {
    let f = Fixture::through_symlink()?;
    assert_ne!(f.root.canonicalize()?, f.root);
    for shape in ["duels", "commander"] {
        f.evaluation("initial", shape, 0.5)?;
    }
    assert_eq!(manifest(&f.root)?["score"], 0.5);
    assert!(publish(&f.root, &f.root.join("selected-model.json"))?);
    f.confirmation(0.001)?;
    assert_eq!(confirmation_assessment(&f.root, &f.model)?["veto"], false);
    std::fs::write(f.model.join("model.k0.onnx"), "mutated")?;
    assert!(manifest(&f.root).is_err());
    assert!(confirmation_assessment(&f.root, &f.model).is_err());
    Ok(())
}
