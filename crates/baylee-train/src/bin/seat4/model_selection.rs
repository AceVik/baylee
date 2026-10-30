//! Validate supervisor selections without loading or mutating a running brain.
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context as _, ensure};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

#[derive(Clone, Debug)]
pub struct Selected {
    pub model: PathBuf,
    pub label: String,
    pub model_sha256: String,
    pub score: Option<f64>,
    pub benchmark_id: Option<String>,
    contract: Value,
}

#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    model: PathBuf,
    label: String,
    score: f64,
    benchmark_id: String,
    sha256: BTreeMap<String, String>,
}

pub fn digest(path: &Path) -> anyhow::Result<String> {
    let mut input = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; 65536];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    let mut hex = String::with_capacity(64);
    for byte in hash.finalize() {
        write!(&mut hex, "{byte:02x}")?;
    }
    Ok(hex)
}

fn contract(model: &Path) -> anyhow::Result<Value> {
    let meta: Value = serde_json::from_slice(&std::fs::read(model.join("model.onnx.json"))?)?;
    ensure!(
        !matches!(meta["encoder_version"].as_u64(), Some(8 | 9)),
        "Encoders 8 and 9 are offline-only until payment-context wire support is available"
    );
    let v5 = meta["encoder_version"] == 7 && meta["model_version"] == 5;
    ensure!(
        (v5 || matches!(
            (
                meta["encoder_version"].as_u64(),
                meta["model_version"].as_u64()
            ),
            (Some(5), Some(2)) | (Some(6 | 7), Some(3))
        )) && meta["trajectory_version"] == 2
            && meta["sampling_version"] == 2,
        "incompatible neural model contract"
    );
    let mut result = serde_json::json!({
        "encoder_version":meta["encoder_version"],"model_version":meta["model_version"],
        "trajectory_version":meta["trajectory_version"],"sampling_version":meta["sampling_version"],
        "structure_version":meta["structure_version"],"structure_sha256":meta["structure_sha256"],
        "dsl_version":meta["dsl_version"],
    });
    if v5 {
        bind_v5_contract(&meta, &mut result)?;
    }
    Ok(result)
}

fn bind_v5_contract(meta: &Value, result: &mut Value) -> anyhow::Result<()> {
    let (revision, candidate_revision) = match meta["architecture"].as_str() {
        Some("baylee_ai_v5_relational_v1") => (1, 1),
        Some("baylee_ai_v5_relational_v2") => (2, 1),
        Some("baylee_ai_v5_relational_v2_candidate_v2") => (2, 2),
        Some("baylee_ai_v5_relational_v2_candidate_v3") => (2, 3),
        _ => anyhow::bail!("unsupported V5 architecture"),
    };
    let cfg = &meta["config"];
    ensure!(
        cfg["candidate_context_revision"] == candidate_revision
            || (candidate_revision == 1 && cfg["candidate_context_revision"].is_null()),
        "V5 candidate architecture/revision mismatch"
    );
    ensure!(
        cfg["relational_revision"] == revision,
        "V5 architecture/revision mismatch"
    );
    ensure!(
        cfg["relational_steps"].as_u64().is_some_and(|n| n <= 4)
            && cfg["relational_width"].as_u64().is_some_and(|n| n > 0),
        "invalid V5 relational dimensions"
    );
    let width = meta["d_model"].as_u64().context("missing V5 d_model")?;
    let heads = meta["n_heads"].as_u64().context("missing V5 n_heads")?;
    let layers = meta["n_layers"].as_u64().context("missing V5 n_layers")?;
    ensure!(
        width > 0 && heads > 0 && width % heads == 0 && layers > 0,
        "invalid V5 model dimensions"
    );
    result["architecture"] = meta["architecture"].clone();
    result["candidate_context_revision"] = serde_json::json!(candidate_revision);
    for key in ["d_model", "n_heads", "n_layers"] {
        ensure!(
            cfg[key] == meta[key],
            "V5 config/top-level dimension mismatch: {key}"
        );
        result[key] = meta[key].clone();
    }
    for key in [
        "relational_revision",
        "relational_steps",
        "relational_width",
    ] {
        result[key] = cfg[key].clone();
    }
    Ok(())
}

impl Selected {
    pub fn fixed(model: &Path) -> anyhow::Result<Self> {
        let model = model
            .canonicalize()
            .context("resolve fixed model directory")?;
        ensure!(
            model.is_dir(),
            "--model must name an exported model directory"
        );
        Ok(Self {
            label: model
                .file_name()
                .context("model directory name")?
                .to_string_lossy()
                .into_owned(),
            model_sha256: digest(&model.join("net.pt"))?,
            contract: contract(&model)?,
            model,
            score: None,
            benchmark_id: None,
        })
    }
}

/// A proposal does not replace the selected model until ONNX loading succeeds.
/// The supervisor must never rewrite a model directory that has been published.
pub fn proposal(path: &Path, current: &Selected, thinking: u8) -> anyhow::Result<Selected> {
    let manifest: Manifest = serde_json::from_slice(&std::fs::read(path)?)?;
    ensure!(manifest.schema_version == 1, "unsupported selection schema");
    ensure!(
        manifest.model.is_absolute() && manifest.model.is_dir(),
        "selection model must be an absolute directory"
    );
    ensure!(
        !manifest.label.trim().is_empty()
            && !manifest.benchmark_id.trim().is_empty()
            && manifest.score.is_finite(),
        "invalid selection label, benchmark or score"
    );
    if let Some(benchmark) = &current.benchmark_id {
        ensure!(
            manifest.benchmark_id == *benchmark,
            "selection benchmark changed"
        );
    }
    if let Some(score) = current.score {
        ensure!(manifest.score > score, "selection is not strictly better");
    }
    let model = manifest.model.canonicalize()?;
    let candidate_contract = contract(&model)?;
    ensure!(
        candidate_contract == current.contract,
        "selection model contract/catalog changed"
    );
    let mut required = vec![
        "net.pt".to_owned(),
        "model.onnx.json".to_owned(),
        "model.onnx".to_owned(),
        "model.k0.onnx".to_owned(),
        format!("model.k{thinking}.onnx"),
    ];
    if matches!(candidate_contract["encoder_version"].as_u64(), Some(6 | 7)) {
        required.push("card_structure.json".into());
    }
    // Every published graph is pinned, including graphs lazily opened later.
    for file in std::fs::read_dir(&model)? {
        let file = file?;
        if file
            .path()
            .extension()
            .is_some_and(|extension| extension == "onnx")
        {
            required.push(file.file_name().to_string_lossy().into_owned());
        }
    }
    for name in required {
        ensure!(
            manifest.sha256.contains_key(&name),
            "selection omitted required file hash"
        );
    }
    for (name, expected) in &manifest.sha256 {
        let mut components = Path::new(name).components();
        ensure!(
            matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none(),
            "selection file must be a basename"
        );
        ensure!(
            expected.len() == 64 && expected.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "invalid selection checksum"
        );
        ensure!(
            digest(&model.join(name))? == expected.to_ascii_lowercase(),
            "selection file checksum differs"
        );
    }
    Ok(Selected {
        model,
        label: manifest.label,
        model_sha256: manifest.sha256["net.pt"].to_ascii_lowercase(),
        score: Some(manifest.score),
        benchmark_id: Some(manifest.benchmark_id),
        contract: candidate_contract,
    })
}

#[cfg(test)]
mod tests;
