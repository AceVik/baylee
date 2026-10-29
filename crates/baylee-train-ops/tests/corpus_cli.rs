//! Read-only diagnostics and immutable preparation have native CLI boundaries.
use anyhow::Result;
use baylee_train_ops::files::{atomic_json, read_json};
use serde_json::{Value, json};
use std::process::Command;
#[test]
fn audit_command_reports_aliases_without_changing_dataset_bytes() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    atomic_json(
        &root.join("meta.json"),
        &json!({"cards":[1,2],"encoder_version":7,"dsl_version":1}),
    )?;
    atomic_json(
        &root.join("catalog.json"),
        &json!({"cards":[{"id":1,"name":"one"},{"id":2,"name":"two"}]}),
    )?;
    let bytes = vec![0u8; 2 * 160 * 4];
    std::fs::write(root.join("card_dsl.bin"), &bytes)?;
    std::fs::write(root.join("card_abilities.bin"), [0u8; 8])?;
    let output = root.join("aliases.json");
    let result = Command::new(env!("CARGO_BIN_EXE_audit_representation4"))
        .arg("--data")
        .arg(root)
        .arg("--catalog")
        .arg(root.join("catalog.json"))
        .arg("--out")
        .arg(&output)
        .output()?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(read_json(&output)?["collision_groups"], 1);
    assert!(
        serde_json::from_slice::<Value>(&result.stdout)?
            .get("groups")
            .is_none()
    );
    assert_eq!(std::fs::read(root.join("card_dsl.bin"))?, bytes);
    Ok(())
}
#[test]
fn insufficient_collector_prefix_does_not_create_a_frozen_run() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    std::fs::create_dir(root.join("attempts"))?;
    let out = root.join("frozen");
    let result = Command::new(env!("CARGO_BIN_EXE_prepare_llm_bc4"))
        .arg("--collector")
        .arg(root)
        .arg("--out")
        .arg(&out)
        .args(["--games", "1"])
        .output()?;
    assert!(!result.status.success());
    assert!(String::from_utf8(result.stderr)?.contains("complete admitted games"));
    assert!(!out.exists());
    Ok(())
}
