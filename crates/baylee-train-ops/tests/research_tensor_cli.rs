//! Actual native tensor process: explicit diagnostics and pinned input admission.
#![cfg(target_os = "linux")]
use baylee_train_ops as ops;
#[path = "../src/research_cache/tests/support.rs"]
mod support;
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "One actual process protocol and admission round-trip fixture"
)]
fn actual_native_tensor_service_and_dedicated_admission_cli() {
    let f = support::Fixture::new(false);
    ops::research_cache::build(&f.converted, &f.cache, &f.expected).unwrap();
    assert_eq!(f.raw.len(), 10);
    assert!(f.source.is_dir());
    let cache_sha = ops::files::sha256(&f.cache.join("research-cache.manifest.json")).unwrap();
    let exe = env!("CARGO_BIN_EXE_research_tensor4");
    let command = || {
        let mut c = Command::new(exe);
        c.arg("serve")
            .arg("--cache-dir")
            .arg(&f.cache)
            .arg("--expected-conversion-manifest-sha256")
            .arg(&f.expected)
            .arg("--expected-cache-manifest-sha256")
            .arg(&cache_sha);
        c
    };
    assert!(!command().output().unwrap().status.success());
    let mut child = command()
        .arg("--diagnostic-unadmitted")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"{\"history\":0,\"start\":0,\"length\":5}\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let newline = output.stdout.iter().position(|b| *b == b'\n').unwrap();
    let handshake: Value = serde_json::from_slice(&output.stdout[..newline]).unwrap();
    assert_eq!(handshake["research_input_admitted"], false);
    assert_eq!(handshake["attempt_counts"]["planned"], 5);
    assert!(output.stdout.len() > newline + 16);
    let manifest = ops::files::read_json(&f.cache.join("research-cache.manifest.json")).unwrap();
    let conversion = &manifest["source_conversion_manifest"];
    let source = &conversion["source_runs"][0];
    let ledger = &source["finalized_attempt_ledger"];
    let job =
        ops::files::read_json(std::path::Path::new(ledger["guard_job"].as_str().unwrap())).unwrap();
    let parent = f.source.parent().unwrap();
    let pins = parent.join("pins.json");
    let admission = parent.join("admission.json");
    ops::files::atomic_json(&pins,&json!({"schema":"failure_terminal_research_input_pins_v1","cache_dir":f.cache,"conversion_manifest_sha256":f.expected,"cache_manifest_sha256":cache_sha,
        "source_checkpoint":{"path":parent.join("source.pt"),"sha256":conversion["learner_model_identity"]["source_checkpoint_sha256"]},
        "learner_model_identity":conversion["learner_model_identity"],"catalog":conversion["catalog"],"producers":[{"source_run_sha256":source["source_run_sha256"],"guard_job":ledger["guard_job"],
        "guard_job_sha256":ledger["guard_job_sha256"],"executable":job["executable"],"executable_sha256":job["executable_sha256"],"collection_contract":source["collection_contract"]}]})).unwrap();
    let pins_sha = ops::files::sha256(&pins).unwrap();
    for action in ["admit", "verify"] {
        let mut c = Command::new(exe);
        c.arg(action)
            .arg("--pins")
            .arg(&pins)
            .arg("--expected-pins-sha256")
            .arg(&pins_sha)
            .arg(if action == "admit" {
                "--out"
            } else {
                "--admission"
            })
            .arg(&admission);
        let output = c.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = command()
        .arg("--pins")
        .arg(&pins)
        .arg("--expected-pins-sha256")
        .arg(&pins_sha)
        .arg("--admission")
        .arg(&admission)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(metadata["research_input_admitted"], true);
    assert_eq!(metadata["admission"]["learner_histories"], 2);
    assert_eq!(metadata["ordinary_ppo_admitted"], false);
    assert!(
        ops::admission::admit(
            &f.cache,
            &parent.join("source.pt"),
            &parent.join("ordinary.json")
        )
        .is_err()
    );
}
