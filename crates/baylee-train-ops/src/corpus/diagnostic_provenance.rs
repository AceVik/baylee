//! Immutable source and completed-job receipts for tensor diagnostics.
use super::bc_metadata::digest;
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(Default)]
struct Reader {
    inputs: BTreeMap<String, String>,
}
impl Reader {
    fn hash(&mut self, path: &Path) -> Result<String> {
        let hash = files::sha256(path)?;
        self.inputs.insert(
            path.canonicalize()?.to_string_lossy().into_owned(),
            hash.clone(),
        );
        Ok(hash)
    }
    fn json(&mut self, path: &Path) -> Result<Value> {
        self.hash(path)?;
        files::read_json(path)
    }
    fn format(&mut self, run: &Path) -> Result<String> {
        let path = run.join("run.json");
        if !path.exists() {
            return Ok("unknown".into());
        }
        let metadata = self.json(&path)?;
        let mut shape = metadata.get("shape").unwrap_or(&metadata["format"]).clone();
        let seats = &metadata["seats"];
        let teams = &metadata["teams"];
        let job = run
            .parent()
            .and_then(Path::parent)
            .context("source run lacks parent")?
            .join("jobs")
            .join(run.file_name().context("source run lacks name")?)
            .join("job.json");
        if shape.is_null() && job.exists() {
            let job = self.json(&job)?;
            let command = job["command"].as_array().cloned().unwrap_or_default();
            let option = |name: &str| -> Option<&str> {
                let indices = command
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.as_str() == Some(name))
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                if indices.len() != 1 {
                    return None;
                }
                command.get(indices[0] + 1)?.as_str()
            };
            if job["status"] == "complete"
                && option("--out").is_some_and(|out| {
                    Path::new(out).canonicalize().ok() == run.canonicalize().ok()
                })
            {
                shape = option("--shape")
                    .or_else(|| option("--shapes"))
                    .map_or(Value::Null, |s| json!(s));
                if let Some(value) = option("--seats") {
                    ensure!(
                        Some(value.parse::<u64>()?) == seats.as_u64(),
                        "Source format metadata disagrees with its completed job"
                    );
                }
            }
        }
        let (Some(shape), Some(seats), Some(teams)) = (
            shape.as_str().filter(|s| !s.contains(',')),
            seats.as_u64(),
            teams.as_array(),
        ) else {
            return Ok("unknown".into());
        };
        let teams = if teams.is_empty() {
            "ffa".into()
        } else {
            teams
                .iter()
                .map(|t| t.as_str().map_or_else(|| t.to_string(), str::to_owned))
                .collect::<Vec<_>>()
                .join(",")
        };
        Ok(format!("{shape}/seats={seats}/teams={teams}"))
    }
}
fn entries(reader: &mut Reader, root: &Path, meta: &Value) -> Result<Vec<Value>> {
    if let Some(entries) = meta.get("package_sources").filter(|v| !v.is_null()) {
        return entries
            .as_array()
            .cloned()
            .context("invalid source markers");
    }
    let markers = root.join("source_markers.json");
    if markers.exists() {
        return reader
            .json(&markers)?
            .as_array()
            .cloned()
            .context("invalid source markers");
    }
    let mut paths = std::fs::read_dir(root.join("ready"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|s| s == "json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths.iter().map(|p| reader.json(p)).collect()
}
/// Resolve trustworthy source-format metadata without guessing from directory names.
///
/// # Errors
/// Rejects source aliases, changed/malformed receipts and inconsistent completed jobs.
pub fn read(value: &Value) -> Result<Value> {
    let root =
        PathBuf::from(value["root"].as_str().context("missing diagnostic root")?).canonicalize()?;
    let meta = &value["meta"];
    let mut reader = Reader::default();
    let namespace = if let Some(namespace) = meta.get("source_namespace").filter(|v| !v.is_null()) {
        namespace.clone()
    } else {
        let declared = meta["source_dataset"]
            .as_str()
            .map_or(root.as_path(), Path::new);
        let source = if declared.is_absolute() {
            super::bc_metadata::resolve(declared)?
        } else {
            root.clone()
        };
        let path = source.join("meta.json");
        json!({"dataset":source,"meta_sha256":reader.hash(&if path.exists(){path}else{root.join("meta.json")})?})
    };
    let mut formats = BTreeMap::new();
    let mut sources = BTreeMap::new();
    let mut seen = BTreeMap::new();
    let mut run_formats = BTreeMap::new();
    for entry in entries(&mut reader, &root, meta)? {
        if entry["split"] != value["split"] {
            continue;
        }
        let game = entry["game_id"]
            .as_u64()
            .context("missing diagnostic game id")?;
        let identity = json!([
            entry["source_shard"],
            entry["source_offset"],
            entry["source_len"]
        ]);
        ensure!(
            seen.get(&game).is_none_or(|prior| *prior == identity),
            "Diagnostic game ID aliases different source games"
        );
        seen.insert(game, identity);
        if entry["structure_sha256"] != value["structure_digest"]
            || entry["replay_hashes_checked"] != true
        {
            continue;
        }
        let Some(shard) = entry["source_shard"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(Path::new)
            .filter(|p| p.is_absolute())
        else {
            continue;
        };
        let run = shard
            .parent()
            .and_then(Path::parent)
            .context("source shard lacks run directory")?;
        if !run_formats.contains_key(run) {
            run_formats.insert(run.to_owned(), reader.format(run)?);
        }
        formats.insert(game, run_formats[run].clone());
        sources.insert(game,json!({"source_shard":entry["source_shard"],"source_offset":entry["source_offset"],"source_len":entry["source_len"]}));
    }
    Ok(
        json!({"inputs":reader.inputs,"namespace_id":digest(&namespace)?,"namespace":namespace,"formats":formats,"sources":sources}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completed_jobs_bind_format_and_conflicting_source_aliases_fail() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        let run = root.join("runs/source");
        let job = root.join("jobs/source");
        std::fs::create_dir_all(&run)?;
        std::fs::create_dir_all(&job)?;
        files::atomic_json(&run.join("run.json"), &json!({"seats":4,"teams":[1,2,1,2]}))?;
        let entry = json!({"game_id":1,"source_shard":run.join("records/a.gz"),"source_offset":0,"source_len":1,"split":"val","structure_sha256":"catalog","replay_hashes_checked":true});
        let mut request = json!({"root":root,"split":"val","structure_digest":"catalog","meta":{"source_namespace":{"conversion":"one"},"package_sources":[entry]}});
        assert_eq!(read(&request)?["formats"]["1"], "unknown");
        let job_path = job.join("job.json");
        files::atomic_json(
            &job_path,
            &json!({"status":"complete","command":["selfplay","--out",run,"--shapes","commander","--seats","4"]}),
        )?;
        let result = read(&request)?;
        assert_eq!(result["formats"]["1"], "commander/seats=4/teams=1,2,1,2");
        assert_eq!(
            // Keyed by canonical path; a temporary directory is a symlink on macOS.
            result["inputs"][job_path.canonicalize()?.to_str().unwrap()],
            files::sha256(&job_path)?
        );
        let mut alias = entry;
        alias["source_offset"] = json!(99);
        request["meta"]["package_sources"]
            .as_array_mut()
            .unwrap()
            .push(alias);
        assert!(read(&request).unwrap_err().to_string().contains("aliases"));
        Ok(())
    }
}
