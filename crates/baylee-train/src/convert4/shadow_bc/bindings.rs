use super::{BTreeMap, Context, Label, ShadowBc4, Value, ensure, expected_contract, sha};

pub(super) fn load(
    run: &[u8],
    bytes: &[u8],
    record: &[u8],
    game: &str,
    catalog: &str,
) -> anyhow::Result<ShadowBc4> {
    ensure!(
        bytes.len() <= 64 * 1024 * 1024,
        "Shadow sidecar exceeds bounded game limit"
    );
    let mut lines = bytes.split(|b| *b == b'\n').filter(|l| !l.is_empty());
    let header: Value = serde_json::from_slice(lines.next().context("Missing shadow header")?)?;
    validate_header(run, &header, game, catalog)?;
    let record_sha256 = sha(record);
    let mut labels = BTreeMap::new();
    let mut complete = None;
    let mut last_offset = None;
    for line in lines {
        ensure!(line.len() <= 256 * 1024, "Oversize shadow row");
        let value: Value = serde_json::from_slice(line)?;
        ensure!(complete.is_none(), "Rows after shadow completion");
        match value["kind"].as_str() {
            Some("label") => {
                let label: Label = serde_json::from_value(value)?;
                ensure!(
                    label.kind == "label"
                        && label.usage == "bc_only"
                        && label.teacher_source == "house"
                        && !label.teacher_has_logprob,
                    "Invalid label provenance"
                );
                ensure!(
                    last_offset.is_none_or(|last| label.record_byte_offset > last),
                    "Duplicate/unordered shadow offsets"
                );
                last_offset = Some(label.record_byte_offset);
                bind_record(&label, record)?;
                ensure!(
                    labels.insert(label.record_input_n, label).is_none(),
                    "Duplicate shadow input label"
                );
            }
            Some("completed") => complete = Some(value),
            _ => anyhow::bail!("Unknown/incomplete shadow sidecar row"),
        }
    }
    let complete = complete.context("Missing completed shadow footer")?;
    let counts = &complete["counts"];
    let mut accounted = 0_u64;
    for field in [
        "labels",
        "mapping_failures",
        "teacher_refits",
        "rejected_executions",
    ] {
        accounted = accounted
            .checked_add(counts[field].as_u64().context("Missing shadow counter")?)
            .context("Shadow counter overflow")?;
    }
    ensure!(
        counts["queries"].as_u64() == Some(accounted),
        "Inconsistent shadow query accounting"
    );
    ensure!(
        complete["schema"] == 1
            && complete["label_use"] == "bc_only"
            && complete["ppo_certification"] == false
            && complete["record_sha256"] == record_sha256
            && complete["record_bytes"].as_u64() == Some(record.len() as u64)
            && complete["counts"]["labels"].as_u64() == Some(labels.len() as u64),
        "Shadow completion/record/count mismatch"
    );
    let last = record
        .split(|b| *b == b'\n')
        .rfind(|l| !l.is_empty())
        .context("Empty record")?;
    ensure!(
        serde_json::from_slice::<Value>(last)?["kind"] == "end",
        "Shadow record missing End"
    );
    Ok(ShadowBc4 {
        encoder_version: u32::try_from(
            serde_json::from_slice::<Value>(run)?["model_identity"]["encoder_version"]
                .as_u64()
                .context("Missing shadow encoder")?,
        )?,
        header,
        record_sha256,
        labels,
    })
}

fn is_digest(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn bind_record(label: &Label, record: &[u8]) -> anyhow::Result<()> {
    let offset = label.record_byte_offset;
    ensure!(
        offset > 0 && offset < record.len() && record[offset - 1] == b'\n',
        "Invalid shadow input byte boundary"
    );
    for line in record[offset..]
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
    {
        let row: Value = serde_json::from_slice(line)?;
        if row["kind"] == "declared_mind" {
            continue;
        }
        ensure!(
            row["kind"] == "input"
                && row["by"] == "seat"
                && row["seat"] == label.seat
                && row["n"] == label.record_input_n
                && row["hash"] == label.record_input_hash
                && row["action"] == serde_json::to_value(&label.executed_action)?,
            "Shadow label bound to a different executed input"
        );
        return Ok(());
    }
    anyhow::bail!("Missing bound input")
}

fn validate_header(run: &[u8], header: &Value, game: &str, catalog: &str) -> anyhow::Result<()> {
    let run_value: Value = serde_json::from_slice(run)?;
    ensure!(
        run_value["policy_mode"] == "raw"
            && run_value["policy_contract"] == "raw_neural_sampler2_v1"
            && run_value["policy_provenance_version"] == 2
            && run_value["sampling_version"] == 2,
        "Shadow requires current Raw provenance, never historical certification"
    );
    let identity = &run_value["model_identity"];
    ensure!(
        ((identity["encoder_version"] == 7
            && matches!(identity["model_version"].as_u64(), Some(3 | 5)))
            || (identity["encoder_version"] == 8
                && identity["model_version"] == 5
                && matches!(
                    identity["architecture"].as_str(),
                    Some("baylee_ai_v5_relational_v2" | "baylee_ai_v5_relational_v2_candidate_v2")
                ))),
        "Shadow encoder/model mismatch"
    );
    ensure!(
        (identity["model_version"] == 3 && identity["model_family"] == "v4")
            || (identity["model_version"] == 5
                && identity["model_family"] == "v5"
                && matches!(
                    identity["architecture"].as_str(),
                    Some(
                        "baylee_ai_v5_relational_v1"
                            | "baylee_ai_v5_relational_v2"
                            | "baylee_ai_v5_relational_v2_candidate_v2"
                    )
                )),
        "Unknown shadow behavior architecture"
    );
    for key in ["source_checkpoint_sha256", "onnx_sha256", "metadata_sha256"] {
        ensure!(
            identity[key].as_str().is_some_and(is_digest),
            "Missing behavior model identity {key}"
        );
    }
    let profile = header["contract"]["teacher"]
        .as_str()
        .context("Missing teacher profile")?;
    ensure!(
        matches!(profile, "sharp" | "expert")
            && header["contract"] == expected_contract(profile)
            && run_value["shadow_teacher"] == expected_contract(profile),
        "Unknown shadow teacher contract"
    );
    ensure!(
        header["kind"] == "header"
            && header["schema"] == 1
            && header["game"] == game
            && header["run_sha256"] == sha(run)
            && header["behavior_model_identity"] == *identity
            && header["structure_sha256"] == catalog
            && run_value["shadow_structure_sha256"] == catalog
            && header["teacher_seed"].is_u64(),
        "Shadow run/game/model/catalog header mismatch"
    );
    Ok(())
}
