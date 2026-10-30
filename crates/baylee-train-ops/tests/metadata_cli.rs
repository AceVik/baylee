//! Native metadata writers accept filenames relative to the caller's directory.
use anyhow::Result;
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn relative_json_and_artifact_outputs_are_durable() -> Result<()> {
    let root = tempfile::tempdir()?;
    for (task, filename) in [
        ("write-json", "meta.json"),
        ("append-json", "metrics.jsonl"),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_train_ops"))
            .current_dir(root.path())
            .args([task, "--out", filename])
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let write = child.stdin.take().unwrap().write_all(b"{\"step\":1}");
        let output = child.wait_with_output()?;
        write?;
        assert!(output.status.success(), "{output:?}");
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(root.path().join(filename))?)?;
        assert_eq!(value["step"], 1);
    }
    let output = Command::new(env!("CARGO_BIN_EXE_train_ops"))
        .current_dir(root.path())
        .args(["copy-file", "--source", "meta.json", "--out", "copy.json"])
        .output()?;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::read(root.path().join("copy.json"))?,
        fs::read(root.path().join("meta.json"))?
    );
    assert_eq!(fs::read_dir(root.path())?.count(), 3);
    Ok(())
}
