//! Native command boundaries use the same fixtures and contracts as the library.
use anyhow::Result;
use baylee_train_ops::files::{atomic_json, read_json};
use serde_json::{Value, json};
use std::process::Command;

fn fixture() -> Result<Value> {
    Ok(serde_json::from_str(include_str!(
        "../src/evaluation/fixtures/python_golden.json"
    ))?)
}

#[test]
fn summarize_and_paired_commands_publish_verified_native_reports() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let g = fixture()?;
    let candidate = root.join("candidate.json");
    let reference = root.join("reference.json");
    for (label, path) in [("candidate", &candidate), ("reference", &reference)] {
        let mut v = g[label].clone();
        v["decks_manifest"] = json!("decks.json");
        atomic_json(path, &v)?;
        atomic_json(
            &root.join(format!("{label}-job.json")),
            &json!({"status":"complete","executable_sha256":"engine","inputs":{"decks.json":"decks"},"command":["eval4","--raw-policy","--out",path,"--model","test","--decks-manifest","decks.json"]}),
        )?;
    }
    let output = root.join("summary.json");
    let result = Command::new(env!("CARGO_BIN_EXE_summarize_eval4"))
        .arg(&candidate)
        .arg("--out")
        .arg(&output)
        .args(["--resamples", "100"])
        .output()?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let summary = read_json(&output)?;
    assert_eq!(summary["deals"], 4);
    assert_eq!(summary["independent_tables"], 2);
    assert_eq!(serde_json::from_slice::<Value>(&result.stdout)?, summary);
    let output = root.join("paired.json");
    let result = Command::new(env!("CARGO_BIN_EXE_paired_eval4"))
        .arg(&candidate)
        .arg(&reference)
        .arg("--candidate-job")
        .arg(root.join("candidate-job.json"))
        .arg("--reference-job")
        .arg(root.join("reference-job.json"))
        .arg("--out")
        .arg(&output)
        .args(["--resamples", "100"])
        .output()?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let paired = read_json(&output)?;
    assert_eq!(paired["outcomes"]["better"], 1);
    assert_eq!(paired["outcomes"]["worse"], 1);
    assert!(paired["evaluation_sha256"]["candidate"].is_string());
    Ok(())
}

#[test]
fn progress_command_observes_existing_files_and_import_errors_stay_anonymous() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    atomic_json(
        &root.join("evaluation/model.json"),
        &fixture()?["candidate"],
    )?;
    let output = root.join("report");
    let result = Command::new(env!("CARGO_BIN_EXE_report_experiment4"))
        .arg("--experiment")
        .arg(root)
        .arg("--out")
        .arg(&output)
        .output()?;
    assert!(result.status.success());
    assert_eq!(
        read_json(&output.join("progress.json"))?["status"],
        "one_shot"
    );
    assert!(root.join("evaluation/model.uncertainty.json").exists());
    let export = root.join("export");
    std::fs::create_dir(&export)?;
    std::fs::write(export.join("private-name@example.com"), b"private content")?;
    let result = Command::new(env!("CARGO_BIN_EXE_import_live_records4"))
        .arg("--export-dir")
        .arg(export)
        .arg("--out")
        .arg(root.join("corpus"))
        .output()?;
    assert!(!result.status.success());
    let error = String::from_utf8(result.stderr)?;
    assert!(error.contains("import refused"));
    assert!(!error.contains("private"));
    assert!(!root.join("corpus").exists());
    Ok(())
}

#[test]
fn maintenance_refuses_missing_exporter_environment_before_any_side_effect() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let status = root.join("status.json");
    let result = Command::new(env!("CARGO_BIN_EXE_live_play_maintenance4"))
        .env_remove("DATABASE_URL")
        .arg("--experiment")
        .arg(root)
        .arg("--selection")
        .arg(root.join("selection.json"))
        .arg("--gateway-bin")
        .arg(root.join("missing-gateway"))
        .arg("--corpus")
        .arg(root.join("corpus"))
        .arg("--status")
        .arg(&status)
        .arg("--once")
        .output()?;
    assert!(!result.status.success());
    assert!(!status.exists());
    assert!(String::from_utf8(result.stderr)?.contains("DATABASE_URL"));
    Ok(())
}
