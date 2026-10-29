//! Immutable manifest/controller behavior with real bounded subprocess jobs.
#![cfg(target_os = "linux")]
use baylee_train_ops::{files, pipeline};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn setup(root: &Path, command: &str) -> PathBuf {
    let manifest = root.join("plan.json");
    let run = root.join("run");
    let plan = json!({"version":1,"root":run,"cwd":env!("CARGO_MANIFEST_DIR"),"total_timeout_seconds":20.0,"stages":[{"name":"probe","command":[command],"timeout_seconds":5.0,"memory_gib":1.0,"inputs":[],"required_outputs":[run.join("jobs/probe/stdout.log")]}]});
    files::atomic_json(&manifest, &plan).unwrap();
    manifest
}

#[test]
fn completed_pipeline_is_immutable_and_revalidated() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = setup(tmp.path(), "true");
    let first = pipeline::run(&manifest, &AtomicBool::new(false)).unwrap();
    assert_eq!(first["phase"], "complete");
    assert_eq!(first["promoted"], false);
    assert_eq!(
        pipeline::run(&manifest, &AtomicBool::new(false)).unwrap(),
        first
    );
    fs::write(tmp.path().join("run/jobs/probe/stdout.log"), b"changed").unwrap();
    assert!(pipeline::run(&manifest, &AtomicBool::new(false)).is_err());
}

#[test]
fn failed_pipeline_is_preserved_not_retried() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = setup(tmp.path(), "false");
    assert!(pipeline::run(&manifest, &AtomicBool::new(false)).is_err());
    let status = files::read_json(&tmp.path().join("run/status.json")).unwrap();
    assert_eq!(status["phase"], "failed");
    assert_eq!(status["automatic_restart"], false);
    assert!(pipeline::run(&manifest, &AtomicBool::new(false)).is_err());
    assert_eq!(
        files::read_json(&tmp.path().join("run/status.json")).unwrap(),
        status
    );
}

#[test]
fn invalid_names_and_missing_artifacts_fail_closed() {
    for name in ["../escape", "missing"] {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = setup(tmp.path(), "true");
        let mut plan: Value = files::read_json(&manifest).unwrap();
        plan["stages"][0]["name"] = json!(name);
        plan["stages"][0]["required_outputs"] = json!([tmp.path().join("absent.pt")]);
        files::atomic_json(&manifest, &plan).unwrap();
        assert!(pipeline::run(&manifest, &AtomicBool::new(false)).is_err());
    }
}

#[test]
fn later_stage_cannot_change_an_earlier_artifact() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = setup(tmp.path(), "true");
    let mut plan = files::read_json(&manifest).unwrap();
    let earlier = tmp.path().join("run/jobs/probe/stdout.log");
    plan["stages"].as_array_mut().unwrap().push(json!({
        "name":"modify", "command":["cp", "/dev/null", earlier],
        "timeout_seconds":5.0,"memory_gib":1.0,"inputs":[],
        "required_outputs":[tmp.path().join("run/jobs/modify/stdout.log")]
    }));
    // First stage writes nonempty data; the later command truncates it.
    plan["stages"][0]["command"] = json!(["echo", "immutable output"]);
    files::atomic_json(&manifest, &plan).unwrap();
    let error = pipeline::run(&manifest, &AtomicBool::new(false)).unwrap_err();
    assert!(error.to_string().contains("changed before completion"));
    assert_eq!(
        files::read_json(&tmp.path().join("run/status.json")).unwrap()["phase"],
        "failed"
    );
    assert!(!tmp.path().join("run/result.json").exists());
}

#[test]
fn repeated_artifact_declarations_fail_before_any_job() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = setup(tmp.path(), "true");
    let mut plan = files::read_json(&manifest).unwrap();
    let mut second = plan["stages"][0].clone();
    second["name"] = json!("second");
    plan["stages"].as_array_mut().unwrap().push(second);
    files::atomic_json(&manifest, &plan).unwrap();
    assert!(pipeline::run(&manifest, &AtomicBool::new(false)).is_err());
    assert!(!tmp.path().join("run/status.json").exists());
}

#[test]
fn future_external_input_is_sealed_before_the_first_stage() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = setup(tmp.path(), "true");
    let external = tmp.path().join("external.bin");
    fs::write(&external, b"original data").unwrap();
    let mut plan = files::read_json(&manifest).unwrap();
    plan["stages"][0]["command"] = json!(["cp", "/dev/null", external]);
    plan["stages"].as_array_mut().unwrap().push(json!({
        "name":"consume", "command":["true"],
        "timeout_seconds":5.0,"memory_gib":1.0,"inputs":[external],
        "required_outputs":[tmp.path().join("run/jobs/consume/stdout.log")]
    }));
    files::atomic_json(&manifest, &plan).unwrap();
    let error = pipeline::run(&manifest, &AtomicBool::new(false)).unwrap_err();
    assert!(error.to_string().contains("sealed input digest mismatch"));
    assert!(!tmp.path().join("run/jobs/consume").exists());
    assert!(!tmp.path().join("run/result.json").exists());
}

#[test]
fn producer_outputs_can_feed_a_later_consumer_but_not_a_forward_dependency() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = setup(tmp.path(), "true");
    let earlier = tmp.path().join("run/jobs/probe/stdout.log");
    let mut plan = files::read_json(&manifest).unwrap();
    plan["stages"][0]["command"] = json!(["echo", "produced data"]);
    plan["stages"].as_array_mut().unwrap().push(json!({
        "name":"consume", "command":["cat", earlier],
        "timeout_seconds":5.0,"memory_gib":1.0,"inputs":[earlier],
        "required_outputs":[tmp.path().join("run/jobs/consume/stdout.log")]
    }));
    files::atomic_json(&manifest, &plan).unwrap();
    let result = pipeline::run(&manifest, &AtomicBool::new(false)).unwrap();
    assert_eq!(result["inputs"], json!({}));
    assert_eq!(
        fs::read(tmp.path().join("run/jobs/consume/stdout.log")).unwrap(),
        b"produced data\n"
    );

    let tmp = tempfile::tempdir().unwrap();
    let manifest = setup(tmp.path(), "true");
    let mut plan = files::read_json(&manifest).unwrap();
    plan["stages"][0]["inputs"] = plan["stages"][0]["required_outputs"].clone();
    files::atomic_json(&manifest, &plan).unwrap();
    assert!(pipeline::run(&manifest, &AtomicBool::new(false)).is_err());
    assert!(!tmp.path().join("run/status.json").exists());
}

#[test]
fn completed_run_reuse_rechecks_its_external_inputs() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = setup(tmp.path(), "true");
    let external = tmp.path().join("source.dat");
    fs::write(&external, b"unchanged").unwrap();
    let mut plan = files::read_json(&manifest).unwrap();
    plan["stages"][0]["inputs"] = json!([external]);
    files::atomic_json(&manifest, &plan).unwrap();
    pipeline::run(&manifest, &AtomicBool::new(false)).unwrap();
    fs::write(external, b"changed").unwrap();
    assert!(pipeline::run(&manifest, &AtomicBool::new(false)).is_err());
}
