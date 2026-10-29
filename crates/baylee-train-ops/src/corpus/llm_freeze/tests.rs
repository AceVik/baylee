use super::*;
use flate2::{Compression, GzBuilder};
fn fixture(root: &Path, seed: u64, mtime: u32) -> Result<(Selected, Vec<u8>)> {
    let attempt = root.join("attempts/attempt-000000003");
    std::fs::create_dir_all(&attempt)?;
    let record = format!(
        "{}\n{}\n",
        json!({"kind":"header","preset":{"seed":seed,"seats":[{},{}]}}),
        json!({"kind":"end","winners":[0]})
    )
    .into_bytes();
    let mut encoder = GzBuilder::new()
        .mtime(mtime)
        .write(Vec::new(), Compression::default());
    encoder.write_all(&record)?;
    let compressed = encoder.finish()?;
    let mut artifacts = Vec::new();
    let mut evidence = vec![(
        "anonymized_record".to_owned(),
        "record.jsonl.gz".to_owned(),
        compressed.clone(),
    )];
    for seat in 0..2 {
        evidence.push((
            format!("transcript_seat{seat}"),
            format!("seat{seat}.jsonl"),
            b"{\"event\":\"over\"}\n".to_vec(),
        ));
        evidence.push((
            format!("executed_provenance_seat{seat}"),
            format!("origins{seat}.json"),
            b"{\"inputs\":[]}\n".to_vec(),
        ));
    }
    for (role, name, raw) in evidence {
        let path = attempt.join(name);
        std::fs::write(&path, &raw)?;
        artifacts.push(json!({"role":role,"path":path,"sha256":digest(&raw)}));
    }
    let request = json!({"attempt_id":"attempt-000000003","models":["flash","terra"],"decks":[{},{}],"bc_only":true,"ppo_eligible":false,"semantic_quality_gate":false});
    let technical = json!({"status":"complete","bc_only":true,"ppo_eligible":false,"actual_seed":seed,"terminal":{"draw":false,"winner_seat":0},"artifacts":artifacts,"record_sha256":digest(&compressed),"checks":{"connection":true,"view_boundary":true,"full_record":true,"replay":true,"provenance":true}});
    files::atomic_json(&attempt.join("request.json"), &request)?;
    files::atomic_json(&attempt.join("technical-result.json"), &technical)?;
    let path = attempt.join("receipt.json");
    let receipt = json!({"status":"admitted","request":request,"technical":technical});
    files::atomic_json(&path, &receipt)?;
    Ok((
        Selected {
            raw: std::fs::read(&path)?,
            path,
            receipt,
        },
        record,
    ))
}
#[test]
fn same_attempt_names_across_collectors_keep_independent_aliases_and_exact_evidence() -> Result<()>
{
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let (first, raw0) = fixture(&root.join("lane0"), 7, 0)?;
    let (second, raw1) = fixture(&root.join("lane9"), 11, 0)?;
    let selected = [first, second];
    let out = root.join("frozen");
    prepare_selected(&root.join("lane0"), &out, &selected)?;
    let meta = files::read_json(&out.join("run.json"))?;
    assert_eq!(meta["games"], 2);
    assert_eq!(meta["source_collectors"].as_array().unwrap().len(), 2);
    for ((source, selected), raw) in meta["sources"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&selected)
        .zip([raw0, raw1])
    {
        let game = source["frozen_game"].as_str().unwrap();
        let manifest = files::read_json(&out.join("llm").join(format!("{game}.json")))?;
        assert_eq!(manifest["source_game"], "attempt-000000003");
        assert_eq!(manifest["record_sha256"], digest(&raw));
        assert_eq!(
            std::fs::read(Path::new(
                manifest["admitted_receipt"]["path"].as_str().unwrap()
            ))?,
            selected.raw
        );
        assert_eq!(manifest["frozen_game"], source["frozen_game"]);
    }
    assert_eq!(meta["semantic_quality_gate"], false);
    assert_eq!(meta["ppo_eligible"], false);
    Ok(())
}
#[test]
fn duplicate_uncompressed_identity_and_artifact_tampering_never_publish() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let (first, _) = fixture(&root.join("lane0"), 7, 0)?;
    let (second, _) = fixture(&root.join("lane9"), 7, 99)?;
    let out = root.join("frozen");
    assert!(
        prepare_selected(&root.join("lane0"), &out, &[first, second])
            .unwrap_err()
            .to_string()
            .contains("Duplicate complete game")
    );
    assert!(!out.exists());
    assert!(root.join("frozen.partial").exists());
    let (selected, _) = fixture(&root.join("changed"), 1, 0)?;
    let path = selected.receipt["technical"]["artifacts"][0]["path"]
        .as_str()
        .unwrap();
    std::fs::write(path, b"changed")?;
    assert!(prepare_selected(&root.join("changed"), &root.join("rejected"), &[selected]).is_err());
    assert!(!root.join("rejected").exists());
    Ok(())
}
#[test]
fn exact_requested_codex_identity_is_bound_and_legacy_agy_is_preserved() -> Result<()> {
    let minds = json!([{"kind":"llm_cli","provider":"codex","model":"gpt-6.1-terra","effort":"low"},{"kind":"llm_cli","provider":"agy","model":"gemini-3.8-flash-low","effort":"low"}]);
    let request = json!({"models":["gpt-6.1-terra","gemini-3.8-flash-low"]});
    let rows = vec![
        json!({"kind":"declared_mind","seat":0,"mind":minds[0]}),
        json!({"kind":"declared_mind","seat":1,"mind":minds[1]}),
    ];
    let bound = json!({"requested_models":request["models"],"requested_minds":minds});
    let evidence = |v: &Value| -> Result<BTreeMap<String, Vec<u8>>> {
        Ok(BTreeMap::from([(
            "input_receipt".to_owned(),
            serde_json::to_vec(v)?,
        )]))
    };
    identity::verify(&request, &rows, &evidence(&bound)?)?;
    assert!(identity::verify(&request, &rows, &BTreeMap::new()).is_err());
    for (key, value) in [
        ("provider", "agy"),
        ("provider", "openai"),
        ("model", "other"),
        ("effort", "high"),
        ("kind", "llm_api"),
    ] {
        let mut changed = bound.clone();
        changed["requested_minds"][0][key] = json!(value);
        assert!(identity::verify(&request, &rows, &evidence(&changed)?).is_err());
    }
    identity::verify(
        &json!({"models":["agy0","agy1"]}),
        &[json!({"kind":"declared_mind","seat":0,"mind":{"provider":"agy"}})],
        &evidence(&json!({"requested_models":["agy0","agy1"],"sha256":{}}))?,
    )?;
    Ok(())
}
#[test]
fn earliest_admitted_selection_and_existing_output_are_explicit() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    fixture(&root.join("collector"), 9, 0)?;
    let out = root.join("frozen");
    assert!(prepare(&root.join("collector"), &out, 2).is_err());
    assert!(!out.exists());
    let report = prepare(&root.join("collector"), &out, 1)?;
    assert_eq!(report["games"], 1);
    assert!(prepare(&root.join("collector"), &out, 1).is_err());
    Ok(())
}

#[test]
fn escaped_artifacts_and_changed_receipts_leave_only_retained_partial_outputs() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let (mut escaped, _) = fixture(&root.join("escaped"), 1, 0)?;
    let outside = root.join("outside.gz");
    let source = escaped.receipt["technical"]["artifacts"][0]["path"]
        .as_str()
        .unwrap();
    std::fs::copy(source, &outside)?;
    escaped.receipt["technical"]["artifacts"][0]["path"] = json!(outside);
    files::atomic_json(
        &escaped.path.parent().unwrap().join("technical-result.json"),
        &escaped.receipt["technical"],
    )?;
    files::atomic_json(&escaped.path, &escaped.receipt)?;
    escaped.raw = std::fs::read(&escaped.path)?;
    assert!(
        prepare_selected(&root.join("escaped"), &root.join("out-escaped"), &[escaped])
            .unwrap_err()
            .to_string()
            .contains("Escaping artifact")
    );
    let (changed, _) = fixture(&root.join("changed-receipt"), 2, 0)?;
    let mut raw = changed.raw.clone();
    raw.push(b' ');
    std::fs::write(&changed.path, raw)?;
    assert!(
        prepare_selected(
            &root.join("changed-receipt"),
            &root.join("out-changed"),
            &[changed]
        )
        .unwrap_err()
        .to_string()
        .contains("receipt changed")
    );
    assert!(!root.join("out-changed").exists());
    Ok(())
}

#[test]
fn codex_requested_identity_receipt_is_preserved_as_exact_hash_bound_evidence() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let collector = root.join("collector");
    let (mut selected, record) = fixture(&collector, 7, 0)?;
    let attempt = selected.path.parent().unwrap();
    let minds = json!([{"kind":"llm_cli","provider":"codex","model":"gpt-6.1-terra","effort":"low"},{"kind":"llm_cli","provider":"agy","model":"gemini-3.8-flash-low","effort":"low"}]);
    selected.receipt["request"]["models"] = json!(["gpt-6.1-terra", "gemini-3.8-flash-low"]);
    let bound =
        json!({"requested_models":selected.receipt["request"]["models"],"requested_minds":minds});
    let input = attempt.join("input-receipt.json");
    files::atomic_json(&input, &bound)?;
    let mut rows = std::str::from_utf8(&record)?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<serde_json::Result<Vec<_>>>()?;
    rows.insert(1, json!({"kind":"declared_mind","seat":0,"mind":minds[0]}));
    rows.insert(2, json!({"kind":"declared_mind","seat":1,"mind":minds[1]}));
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::default());
    for row in rows {
        serde_json::to_writer(&mut encoder, &row)?;
        encoder.write_all(b"\n")?;
    }
    let compressed = encoder.finish()?;
    let technical = &mut selected.receipt["technical"];
    for artifact in technical["artifacts"].as_array_mut().unwrap() {
        if artifact["role"] == "anonymized_record" {
            std::fs::write(artifact["path"].as_str().unwrap(), &compressed)?;
            artifact["sha256"] = json!(digest(&compressed));
        }
    }
    technical["record_sha256"] = json!(digest(&compressed));
    technical["artifacts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"role":"input_receipt","path":input,"sha256":files::sha256(&input)?}));
    files::atomic_json(&attempt.join("request.json"), &selected.receipt["request"])?;
    files::atomic_json(
        &attempt.join("technical-result.json"),
        &selected.receipt["technical"],
    )?;
    files::atomic_json(&selected.path, &selected.receipt)?;
    selected.raw = std::fs::read(&selected.path)?;
    let out = root.join("frozen");
    prepare_selected(&collector, &out, &[selected])?;
    let path = std::fs::read_dir(out.join("llm"))?.next().unwrap()?.path();
    let manifest = files::read_json(&path)?;
    let pin = &manifest["input_receipt"];
    let bytes = std::fs::read(pin["path"].as_str().unwrap())?;
    assert_eq!(pin["sha256"], digest(&bytes));
    assert_eq!(serde_json::from_slice::<Value>(&bytes)?, bound);
    Ok(())
}

#[cfg(unix)]
#[test]
fn source_alias_is_stable_across_symlink_paths() -> Result<()> {
    let root = tempfile::tempdir()?;
    let collector = root.path().join("collector");
    let (selected, record) = fixture(&collector, 1, 0)?;
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(&collector, &alias)?;
    let attempt = selected
        .path
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(
        frozen_game_key(&collector, attempt, &digest(&record))?,
        frozen_game_key(&alias, attempt, &digest(&record))?
    );
    Ok(())
}
