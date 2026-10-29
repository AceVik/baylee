use super::super::publish::ASSETS;
use super::Consumer;
use crate::files;
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
pub(super) fn inputs(package: &Path, legacy: bool) -> Result<Vec<PathBuf>> {
    let marker = files::read_json(&package.join("ready.json"))?;
    let metadata = files::read_json(&package.join("meta.json"))?;
    ensure!(
        marker["metadata_sha256"] == files::sha256(&package.join("meta.json"))?,
        "published package metadata changed"
    );
    ensure!(
        marker["version"] == 2 || legacy,
        "legacy ready marker needs explicit fresh-manifest admission"
    );
    let mut paths = Vec::new();
    for name in ASSETS.into_iter().map(str::to_owned).chain(
        metadata["train_shards"]
            .as_array()
            .context("package shards")?
            .iter()
            .map(|v| format!("train/{}", v.as_str().unwrap_or(""))),
    ) {
        let path = package.join(&name);
        ensure!(
            path.canonicalize()?.starts_with(package.canonicalize()?),
            "package input escapes its root"
        );
        let digest = files::sha256(&path)?;
        if marker["version"] == 2 {
            ensure!(
                marker["inputs"][&name] == digest,
                "published package input changed: {name}"
            );
        }
        paths.push(path);
    }
    let allowed = metadata["source_game_ids"]
        .as_array()
        .context("package game IDs")?
        .iter()
        .map(|id| Ok(u32::try_from(id.as_u64().context("game ID")?)?))
        .collect::<Result<BTreeSet<_>>>()?;
    ensure!(
        !allowed.is_empty()
            && allowed.len()
                == metadata["source_game_ids"]
                    .as_array()
                    .context("source game IDs")?
                    .len()
            && marker["games"] == allowed.len(),
        "duplicate/empty game IDs or ready count mismatch"
    );
    ensure!(
        metadata["val_shards"].as_array().is_some_and(Vec::is_empty),
        "validation rows in a training package"
    );
    let shards = metadata["train_shards"]
        .as_array()
        .context("shards")?
        .iter()
        .map(|v| Ok(v.as_str().context("shard name")?.to_owned()))
        .collect::<Result<Vec<_>>>()?;
    let cache = crate::trajectory::cache::build(package, "train", &shards)?;
    let (actual, count) = super::super::verify_index(&cache.index_path, &allowed)?;
    ensure!(
        actual == allowed && marker["frames"] == count && metadata["train_steps"] == count,
        "package game/frame mismatch"
    );
    paths.extend([package.join("meta.json"), package.join("ready.json")]);
    Ok(paths)
}

pub(super) fn command(
    plan: &Consumer,
    package: &Path,
    output: &Path,
    checkpoint: &Path,
) -> Result<Vec<String>> {
    for placeholder in ["{package}", "{output}", "{checkpoint}", "{validation_data}"] {
        ensure!(
            plan.train_command.iter().any(|v| v.contains(placeholder)),
            "explicit train_command must bind {placeholder}"
        );
    }
    Ok(plan
        .train_command
        .iter()
        .map(|argument| {
            argument
                .replace("{package}", &package.to_string_lossy())
                .replace("{output}", &output.to_string_lossy())
                .replace("{checkpoint}", &checkpoint.to_string_lossy())
                .replace("{validation_data}", &plan.validation_data.to_string_lossy())
        })
        .collect())
}

pub(super) fn validation(root: &Path) -> Result<Vec<PathBuf>> {
    let metadata = files::read_json(&root.join("meta.json"))?;
    let mut paths = ASSETS
        .into_iter()
        .map(|name| root.join(name))
        .collect::<Vec<_>>();
    paths.push(root.join("meta.json"));
    for name in metadata["val_shards"]
        .as_array()
        .context("fixed validation shards")?
    {
        let name = name.as_str().context("validation shard name")?;
        ensure!(
            Path::new(name).components().count() == 1
                && Path::new(name).file_name().is_some_and(|v| v == name),
            "validation shard path escapes split"
        );
        paths.push(root.join("val").join(name));
    }
    for path in &paths {
        ensure!(
            path.canonicalize()?.starts_with(root.canonicalize()?),
            "validation input escapes its dataset"
        );
    }
    Ok(paths)
}
