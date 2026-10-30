use super::{bound, common, digest, map, marked, positive, small, text};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
pub(super) fn validate(root: &Path, meta: &Value) -> Result<()> {
    if !marked(meta, "shadow_bc_") {
        return Ok(());
    }
    common(meta, "shadow_bc_", "verified_view_only_teacher_v1")?;
    let sources = meta["shadow_bc_sources"]
        .as_array()
        .filter(|s| !s.is_empty())
        .context("Missing shadow BC source identities")?;
    let hashes = map(meta, "shadow_bc_label_sha256")?;
    let mut names = BTreeSet::new();
    let mut games = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for source in sources {
        let game = positive(source, "converted_game_id")?;
        let name = text(source, "converted_labels")?;
        let identity = (
            text(source, "source_namespace")?,
            text(source, "source_game")?,
            text(source, "record_sha256")?,
        );
        ensure!(
            games.insert(game)
                && identities.insert(identity)
                && name == format!("shadow-bc/game-{game:06}.jsonl")
                && names.insert(name.to_owned()),
            "Duplicate or inconsistent shadow BC game identity"
        );
        for key in [
            "source_namespace",
            "run_sha256",
            "labels_sha256",
            "record_sha256",
        ] {
            ensure!(digest(&source[key]), "Invalid shadow BC source digest");
        }
        ensure!(
            source["source_namespace"] == source["run_sha256"],
            "Shadow namespace differs from source run"
        );
        let path = bound(
            root,
            name,
            hashes.get(name).context("unbound shadow BC companion")?,
        )?;
        small(&path)?;
        let mut rows = BufReader::new(File::open(path)?).lines();
        let header: Value =
            serde_json::from_str(&rows.next().context("missing shadow BC header")??)?;
        let mut footer = None;
        let mut count = 0u64;
        let mut cursors = BTreeSet::new();
        for line in rows {
            let row: Value = serde_json::from_str(&line?)?;
            ensure!(footer.is_none(), "Data after completed shadow BC footer");
            match row["kind"].as_str() {
                Some("completed") => footer = Some(row),
                Some("label") => {
                    let seat = row["seat"]
                        .as_u64()
                        .filter(|s| *s < 8)
                        .context("invalid shadow BC seat")?;
                    let cursor = row["step_cursor"]
                        .as_u64()
                        .context("invalid shadow BC cursor")?;
                    ensure!(
                        cursors.insert((seat, cursor)),
                        "Duplicate shadow BC frame identity"
                    );
                    count += 1;
                }
                _ => anyhow::bail!("Unknown shadow BC companion record"),
            }
        }
        let mut expected = json!({"kind":"header","schema":1,"converted_game_id":game,"source_namespace":source["source_namespace"],"source_game":source["source_game"],"run_sha256":source["run_sha256"],"original_labels_sha256":source["labels_sha256"],"record_sha256":source["record_sha256"],"label_use":"bc_only","ppo_eligible":false});
        if matches!(meta["encoder_version"].as_u64(), Some(8 | 9)) {
            expected["encoder_version"] = meta["encoder_version"].clone();
        }
        ensure!(
            header == expected
                && footer == Some(json!({"kind":"completed","labels":count,"ppo_eligible":false})),
            "Shadow BC companion binding/footer mismatch"
        );
    }
    ensure!(
        names == hashes.keys().cloned().collect(),
        "Unbound shadow BC companion files"
    );
    Ok(())
}
