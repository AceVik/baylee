use super::*;
#[test]
fn failed_or_disconnected_tensor_workers_never_publish_partial_files() -> Result<()> {
    let root = tempfile::tempdir()?;
    let out = root.path().join("checkpoint.pt");
    std::fs::write(&out, b"previous-complete-checkpoint")?;
    let (publication, receipt) = Publication::prepare(&json!({"kind":"file","out":out}))?;
    let partial = PathBuf::from(receipt["temporary"].as_str().unwrap());
    std::fs::write(&partial, b"partial-torch-zip")?;
    drop(publication);
    assert!(!partial.exists());
    assert_eq!(std::fs::read(&out)?, b"previous-complete-checkpoint");
    let mut replies = Vec::new();
    serve(
        std::io::Cursor::new(format!(
            "{}\n",
            json!({"op":"prepare","kind":"directory","out":root.path().join("new")})
        )),
        &mut replies,
    )?;
    let reply: Value = serde_json::from_slice(&replies)?;
    assert!(!Path::new(reply["value"]["temporary"].as_str().unwrap()).exists());
    assert!(!root.path().join("new").exists());
    let truncated = root.path().join("truncated.pt");
    let input = format!(
        "{}\n{}\nabc",
        json!({"op":"prepare","kind":"file","out":truncated}),
        json!({"op":"write","length":10})
    );
    let mut replies = Vec::new();
    serve(std::io::Cursor::new(input), &mut replies)?;
    assert!(!truncated.exists());
    assert!(String::from_utf8(replies)?.contains("Incomplete tensor transport block"));
    Ok(())
}
#[test]
fn explicit_tensor_success_commits_and_existing_directories_are_preserved() -> Result<()> {
    let root = tempfile::tempdir()?;
    let out = root.path().join("model");
    let (mut publication, receipt) = Publication::prepare(&json!({"kind":"directory","out":out}))?;
    let temporary = Path::new(receipt["temporary"].as_str().unwrap());
    std::fs::write(temporary.join("net.pt"), b"complete-tensor-payload")?;
    publication.commit(&json!({"metadata":{"complete":true}}))?;
    drop(publication);
    assert_eq!(
        files::read_json(&out.join("report.json"))?["complete"],
        true
    );
    assert_eq!(std::fs::read_dir(&out)?.count(), 2);
    assert!(Publication::prepare(&json!({"kind":"directory","out":out})).is_err());
    Ok(())
}
