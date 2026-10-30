use super::{bound, common, digest, map, marked, positive, small, text};
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
pub(super) fn validate(root: &Path, meta: &Value) -> Result<()> {
    if !marked(meta, "llm_bc_") {
        return Ok(());
    }
    ensure!(
        !marked(meta, "shadow_bc_"),
        "LLM BC cannot also claim shadow labels"
    );
    common(meta, "llm_bc_", "verified_executed_llm_v1")?;
    let sources = meta["llm_bc_sources"]
        .as_array()
        .filter(|s| !s.is_empty())
        .context("LLM BC missing source identities")?;
    let labels = map(meta, "llm_bc_label_sha256")?;
    let manifests = map(meta, "llm_bc_manifest_sha256")?;
    check_frames(root, meta)?;
    let mut seen = Sources::default();
    for source in sources {
        check_source(root, meta, source, labels, manifests, &mut seen)?;
    }
    ensure!(
        seen.label_names == labels.keys().cloned().collect()
            && seen.manifest_names == manifests.keys().cloned().collect(),
        "LLM BC unbound companion files"
    );
    Ok(())
}
fn check_frames(root: &Path, meta: &Value) -> Result<()> {
    let frames = map(meta, "llm_bc_frame_sha256")?;
    let mut expected_frames = BTreeSet::new();
    for split in ["train", "val"] {
        let key = format!("{split}_shards");
        let mut seen = BTreeSet::new();
        for name in meta[&key]
            .as_array()
            .context("LLM BC invalid shard names")?
        {
            let name = name.as_str().context("LLM BC invalid shard name")?;
            ensure!(
                Path::new(name).file_name().is_some_and(|n| n == name) && seen.insert(name),
                "LLM BC invalid shard names"
            );
            expected_frames.insert(format!("{split}/{name}"));
        }
    }
    ensure!(
        !expected_frames.is_empty() && expected_frames == frames.keys().cloned().collect(),
        "LLM BC unbound or missing full-history shards"
    );
    for (name, value) in frames {
        bound(root, name, value)?;
    }
    Ok(())
}
#[derive(Default)]
struct Sources<'a> {
    games: BTreeSet<u64>,
    identities: BTreeSet<(&'a str, &'a str, &'a str)>,
    records: BTreeSet<&'a str>,
    label_names: BTreeSet<String>,
    manifest_names: BTreeSet<String>,
}
fn check_source<'a>(
    root: &Path,
    meta: &Value,
    source: &'a Value,
    labels: &serde_json::Map<String, Value>,
    manifests: &serde_json::Map<String, Value>,
    seen: &mut Sources<'a>,
) -> Result<()> {
    let game = positive(source, "converted_game_id")?;
    let source_game = text(source, "source_game")?;
    ensure!(
        seen.games.insert(game) && !source_game.is_empty() && source_game.chars().count() <= 256,
        "LLM BC duplicate or invalid source game"
    );
    for key in [
        "source_namespace",
        "run_sha256",
        "manifest_sha256",
        "record_sha256",
    ] {
        ensure!(digest(&source[key]), "LLM BC invalid source digest");
    }
    ensure!(
        source["source_namespace"] == source["run_sha256"],
        "LLM BC namespace differs from run"
    );
    let identity = (
        text(source, "source_namespace")?,
        source_game,
        text(source, "record_sha256")?,
    );
    ensure!(
        seen.identities.insert(identity) && seen.records.insert(text(source, "record_sha256")?),
        "LLM BC duplicate source game identity"
    );
    let name = format!("llm-bc/game-{game:06}.jsonl");
    let manifest_name = format!("llm-bc/game-{game:06}.manifest.json");
    ensure!(
        source["converted_labels"] == name && source["converted_manifest"] == manifest_name,
        "LLM BC noncanonical companion path"
    );
    let path = bound(
        root,
        &name,
        labels.get(&name).context("LLM BC missing label digest")?,
    )?;
    let manifest_path = bound(
        root,
        &manifest_name,
        manifests
            .get(&manifest_name)
            .context("LLM BC missing manifest digest")?,
    )?;
    ensure!(
        manifests[&manifest_name] == source["manifest_sha256"],
        "LLM BC manifest source identity differs"
    );
    small(&manifest_path)?;
    small(&path)?;
    let models = manifest(&files::read_json(&manifest_path)?, source)?;
    let mut header = json!({"kind":"header","schema":1,"converted_game_id":game,"source_namespace":source["source_namespace"],"source_game":source_game,"run_sha256":source["run_sha256"],"manifest_sha256":source["manifest_sha256"],"record_sha256":source["record_sha256"],"label_use":"bc_only","ppo_eligible":false});
    if matches!(meta["encoder_version"].as_u64(), Some(8 | 9)) {
        header["encoder_version"] = meta["encoder_version"].clone();
    }
    check_labels(&path, &header, &models)?;
    seen.label_names.insert(name);
    seen.manifest_names.insert(manifest_name);
    Ok(())
}
fn artifact(value: &Value) -> Result<()> {
    ensure!(
        value.is_object()
            && value["path"]
                .as_str()
                .is_some_and(|p| Path::new(p).is_absolute())
            && digest(&value["sha256"]),
        "LLM BC manifest artifact role/digest missing"
    );
    Ok(())
}
fn manifest(value: &Value, source: &Value) -> Result<BTreeMap<u64, String>> {
    ensure!(
        value.is_object()
            && value["schema_version"].as_u64() == Some(1)
            && value["contract"] == "admitted_executed_llm_v1"
            && value["source_game"] == source["source_game"]
            && value["record_sha256"] == source["record_sha256"]
            && digest(&value["record_artifact_sha256"]),
        "LLM BC manifest record binding differs"
    );
    for key in ["request", "admitted_receipt", "technical_result"] {
        artifact(&value[key])?;
    }
    let seats = value["seats"]
        .as_array()
        .filter(|s| (2..=8).contains(&s.len()))
        .context("LLM BC manifest seat count invalid")?;
    let mut models = BTreeMap::new();
    for row in seats {
        let seat = row["seat"]
            .as_u64()
            .context("LLM BC manifest seat invalid")?;
        let model = text(row, "model")?;
        ensure!(
            !model.is_empty() && models.insert(seat, model.to_owned()).is_none(),
            "LLM BC manifest seat/model binding invalid"
        );
        artifact(&row["transcript"])?;
        artifact(&row["origins"])?;
    }
    ensure!(
        models.keys().copied().eq(0..seats.len() as u64),
        "LLM BC manifest seats incomplete"
    );
    if models.values().any(|m| m == "gpt-6.1-terra") || value.get("input_receipt").is_some() {
        artifact(&value["input_receipt"])?;
    }
    Ok(models)
}
fn check_labels(path: &Path, header: &Value, models: &BTreeMap<u64, String>) -> Result<()> {
    let mut lines = BufReader::new(File::open(path)?).lines();
    let first: Value =
        serde_json::from_str(&lines.next().context("LLM BC missing companion header")??)?;
    ensure!(first == *header, "LLM BC companion header binding differs");
    let expected: BTreeSet<_> = [
        "kind",
        "seat",
        "step_cursor",
        "record_input_n",
        "executed_source",
        "ambiguous",
        "model",
        "executed_action",
        "target",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let mut footer = None;
    let mut count = 0u64;
    let mut cursors = BTreeSet::new();
    let mut inputs = BTreeSet::new();
    for line in lines {
        let row: Value = serde_json::from_str(&line?)?;
        ensure!(footer.is_none(), "LLM BC data after completed footer");
        if row["kind"] == "completed" {
            footer = Some(row);
            continue;
        }
        ensure!(
            row.as_object()
                .is_some_and(|r| r.keys().cloned().collect::<BTreeSet<_>>() == expected)
                && row["kind"] == "label"
                && row["executed_source"] == "llm"
                && row["ambiguous"] == false,
            "LLM BC non-LLM/ambiguous policy label"
        );
        let seat = row["seat"].as_u64().context("LLM BC invalid seat")?;
        let cursor = row["step_cursor"]
            .as_u64()
            .context("LLM BC invalid cursor")?;
        let input = row["record_input_n"]
            .as_u64()
            .context("LLM BC invalid input")?;
        ensure!(
            models.contains_key(&seat) && cursors.insert((seat, cursor)) && inputs.insert(input),
            "LLM BC duplicate/invalid frame or executed input"
        );
        ensure!(
            row["model"].as_str() == models.get(&seat).map(String::as_str)
                && (row["executed_action"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty())
                    || row["executed_action"]
                        .as_object()
                        .is_some_and(|o| !o.is_empty()))
                && row["target"].as_object().is_some_and(|o| !o.is_empty()),
            "LLM BC invalid label payload"
        );
        count += 1;
    }
    ensure!(
        footer == Some(json!({"kind":"completed","labels":count,"ppo_eligible":false})),
        "LLM BC completion/count differs"
    );
    Ok(())
}
