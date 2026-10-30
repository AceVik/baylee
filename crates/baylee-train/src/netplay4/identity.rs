//! Metadata contract validation and content identity without starting ONNX.
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Exact architecture label shared with the V5 checkpoint/export implementation.
pub const V5_ARCHITECTURE: &str = "baylee_ai_v5_relational_v1";
const V5_ARCHITECTURE_V2: &str = "baylee_ai_v5_relational_v2";
const V5_CANDIDATE_V2: &str = "baylee_ai_v5_relational_v2_candidate_v2";

/// Content identity of the exact graph and metadata accepted by the loader.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ModelIdentity {
    /// Major product family; older supported graphs remain V4.
    pub model_family: &'static str,
    /// Neural architecture's model contract version.
    pub model_version: u32,
    /// Input/candidate encoder contract, independent of model architecture.
    pub encoder_version: u32,
    /// Explicit input width for Encoder 9; omitted from archived identities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_feat_width: Option<u32>,
    /// Explicit value-head target, absent only on legacy unmarked exports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critic_semantics: Option<super::CriticSemantics4>,
    /// Required explicit architecture for V5; never inferred for archived V4.
    pub architecture: Option<String>,
    /// SHA256 of the loaded ONNX bytes.
    pub onnx_sha256: String,
    /// All present fixed-depth graphs; includes K0 observation graph identity.
    pub depth_graph_sha256: BTreeMap<u8, String>,
    /// SHA256 of the exact companion JSON bytes.
    pub metadata_sha256: String,
    /// Exported parent checkpoint identity when the exporter supplied it.
    pub source_checkpoint_sha256: Option<String>,
}

fn version(meta: &Value, key: &str) -> anyhow::Result<u32> {
    u32::try_from(
        meta[key]
            .as_u64()
            .with_context(|| format!("missing {key}"))?,
    )
    .with_context(|| format!("invalid {key}"))
}

fn contract(meta: &Value) -> anyhow::Result<(u32, u32, &'static str)> {
    let encoder = version(meta, "encoder_version")?;
    let model = version(meta, "model_version")?;
    let v4 = (encoder == 5 && model == 2) || ((encoder == 6 || encoder == 7) && model == 3);
    let v5 = (encoder == 7
        && model == 5
        && matches!(
            meta["architecture"].as_str(),
            Some(V5_ARCHITECTURE | V5_ARCHITECTURE_V2 | V5_CANDIDATE_V2)
        ))
        || (matches!(encoder, 8 | 9)
            && model == 5
            && matches!(
                meta["architecture"].as_str(),
                Some(V5_ARCHITECTURE_V2 | V5_CANDIDATE_V2)
            ));
    anyhow::ensure!(
        (v4 || v5) && meta["trajectory_version"] == 2 && meta["sampling_version"] == 2,
        "incompatible neural model/encoder/architecture contracts"
    );
    if encoder == 9 {
        anyhow::ensure!(
            meta["global_feat_width"] == 82
                && meta["config"]["global_feat_width"] == 82
                && meta["config"]["relational_revision"] == 2
                && matches!(
                    meta["config"]["candidate_context_revision"].as_u64(),
                    Some(1 | 2)
                ),
            "Encoder 9 requires explicit global width 82 and v5 relational revision 2, candidate revision 1 or 2"
        );
    } else if matches!(encoder, 7 | 8) {
        for width in [
            &meta["global_feat_width"],
            &meta["config"]["global_feat_width"],
        ] {
            anyhow::ensure!(
                width.is_null() || *width == 80,
                "Legacy Encoder 7/8 requires global width 80 when explicitly declared"
            );
        }
    }
    if meta["architecture"] == V5_CANDIDATE_V2 {
        anyhow::ensure!(
            meta["config"]["relational_revision"] == 2
                && meta["config"]["candidate_context_revision"] == 2,
            "candidate architecture/revision mismatch"
        );
        for key in ["d_model", "n_heads", "n_layers"] {
            anyhow::ensure!(
                meta["config"][key] == meta[key],
                "candidate config dimension mismatch: {key}"
            );
        }
    } else if v5 {
        anyhow::ensure!(
            meta["config"]["candidate_context_revision"].is_null()
                || meta["config"]["candidate_context_revision"] == 1,
            "legacy architecture cannot declare new candidate context"
        );
    }
    if v5 {
        let width = version(meta, "d_model")?;
        let heads = version(meta, "n_heads")?;
        let layers = version(meta, "n_layers")?;
        anyhow::ensure!(
            heads > 0 && width > 0 && width % heads == 0 && layers > 0,
            "invalid V5 hidden width, head count or depth"
        );
    }
    Ok((encoder, model, if v5 { "v5" } else { "v4" }))
}

fn paths(path: &Path) -> (PathBuf, PathBuf) {
    if path.is_dir() {
        return (path.join("model.onnx"), path.join("model.onnx.json"));
    }
    let companion = path.with_extension("onnx.json");
    let metadata = if companion.exists() {
        companion
    } else {
        path.with_file_name("meta.json")
    };
    (path.to_path_buf(), metadata)
}

fn file_digest(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16384];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(&hasher.finalize()))
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("write to String");
    }
    output
}

pub(super) fn verify_depth_graph(
    path: &Path,
    depth: u8,
    identity: &ModelIdentity,
) -> anyhow::Result<()> {
    let expected = identity
        .depth_graph_sha256
        .get(&depth)
        .context("depth graph was absent from initial model identity")?;
    anyhow::ensure!(
        &file_digest(path)? == expected,
        "depth graph changed since initial model identity"
    );
    Ok(())
}

pub(super) fn read_model(path: &Path) -> anyhow::Result<(PathBuf, Value, ModelIdentity)> {
    let (graph, metadata) = paths(path);
    let bytes = std::fs::read(metadata)?;
    let meta: Value = serde_json::from_slice(&bytes)?;
    let (encoder_version, model_version, model_family) = contract(&meta)?;
    let critic_semantics = super::critic::read_semantics(&meta)?;
    let source_checkpoint_sha256 = meta["source_checkpoint_sha256"].as_str().map(str::to_owned);
    let parent = graph.parent().context("model graph parent")?;
    let mut depth_graph_sha256 = BTreeMap::new();
    for depth in [0, 1, 2, 4, 8] {
        let path = parent.join(format!("model.k{depth}.onnx"));
        if path.exists() {
            depth_graph_sha256.insert(depth, file_digest(&path)?);
        }
    }
    let identity = ModelIdentity {
        model_family,
        model_version,
        encoder_version,
        global_feat_width: (encoder_version == 9).then_some(82),
        critic_semantics,
        architecture: if model_family == "v5" {
            Some(
                meta["architecture"]
                    .as_str()
                    .context("V5 architecture")?
                    .to_owned(),
            )
        } else {
            None
        },
        onnx_sha256: file_digest(&graph)?,
        depth_graph_sha256,
        metadata_sha256: hex_digest(&Sha256::digest(&bytes)),
        source_checkpoint_sha256,
    };
    Ok((graph, meta, identity))
}

/// Validates metadata and fingerprints the graph without opening a model session.
///
/// # Errors
/// Refuses missing/corrupt files or unsupported model/encoder/architecture pairs.
pub fn inspect_model_identity(path: &Path) -> anyhow::Result<ModelIdentity> {
    read_model(path).map(|(_, _, identity)| identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn candidate_context_revision_has_a_distinct_strict_identity() {
        for encoder in [7, 8] {
            let mut meta = json!({"encoder_version":encoder,"model_version":5,
                "architecture":V5_CANDIDATE_V2,"trajectory_version":2,"sampling_version":2,
                "d_model":32,"n_heads":4,"n_layers":1,
                "config":{"relational_revision":2,"candidate_context_revision":2,
                    "d_model":32,"n_heads":4,"n_layers":1}});
            assert_eq!(contract(&meta).unwrap(), (encoder, 5, "v5"));
            meta["config"]["candidate_context_revision"] = json!(1);
            assert!(contract(&meta).is_err());
            meta["config"]["candidate_context_revision"] = json!(2);
            meta["config"]["relational_revision"] = json!(1);
            assert!(contract(&meta).is_err());
            meta["config"]["relational_revision"] = json!(2);
            meta["architecture"] = json!(V5_ARCHITECTURE_V2);
            assert!(contract(&meta).is_err());
        }
    }

    #[test]
    fn encoder8_requires_v5_relational_v2_without_migrating_legacy_contracts() {
        let mut meta = json!({"encoder_version":8,"model_version":5,
            "architecture":V5_ARCHITECTURE_V2,"trajectory_version":2,"sampling_version":2,
            "d_model":640,"n_heads":10,"n_layers":12});
        assert_eq!(contract(&meta).unwrap(), (8, 5, "v5"));
        meta["architecture"] = json!(V5_ARCHITECTURE);
        assert!(contract(&meta).is_err());
        meta["architecture"] = json!(V5_ARCHITECTURE_V2);
        meta["model_version"] = json!(3);
        assert!(contract(&meta).is_err());
        meta["model_version"] = json!(5);
        meta["encoder_version"] = json!(9);
        assert!(contract(&meta).is_err());
    }

    #[test]
    fn encoder9_binds_explicit_width_and_matching_relational_candidate_revision() {
        for (architecture, candidate) in [(V5_ARCHITECTURE_V2, 1), (V5_CANDIDATE_V2, 2)] {
            let valid = json!({"encoder_version":9,"model_version":5,"architecture":architecture,
                "trajectory_version":2,"sampling_version":2,"global_feat_width":82,
                "d_model":32,"n_heads":4,"n_layers":1,"config":{"d_model":32,"n_heads":4,"n_layers":1,
                    "relational_revision":2,"candidate_context_revision":candidate,"global_feat_width":82}});
            assert_eq!(contract(&valid).unwrap(), (9, 5, "v5"));
            for pointer in [
                "/global_feat_width",
                "/config/global_feat_width",
                "/config/relational_revision",
                "/config/candidate_context_revision",
            ] {
                let mut broken = valid.clone();
                *broken.pointer_mut(pointer).unwrap() = Value::Null;
                assert!(contract(&broken).is_err(), "missing {pointer}");
            }
            for encoder in [7, 8] {
                let mut wrong = valid.clone();
                wrong["encoder_version"] = json!(encoder);
                assert!(contract(&wrong).is_err());
            }
            let mut wrong = valid;
            wrong["config"]["candidate_context_revision"] = json!(3 - candidate);
            assert!(contract(&wrong).is_err());
        }
    }

    #[test]
    fn v4_contracts_remain_supported_and_v5_requires_its_own_architecture() {
        for (encoder, model) in [(5, 2), (6, 3), (7, 3), (7, 5)] {
            let mut meta = json!({"encoder_version":encoder,"model_version":model,
                "trajectory_version":2,"sampling_version":2,"d_model":640,"n_heads":10,"n_layers":12});
            if model == 5 {
                assert!(contract(&meta).is_err());
                meta["architecture"] = json!("unknown");
                assert!(contract(&meta).is_err());
                meta["architecture"] = json!(V5_ARCHITECTURE);
            }
            let (_, _, family) = contract(&meta).unwrap();
            assert_eq!(family, if model == 5 { "v5" } else { "v4" });
        }
        let invalid = json!({"encoder_version":6,"model_version":5,"architecture":V5_ARCHITECTURE,
            "trajectory_version":2,"sampling_version":2});
        assert!(contract(&invalid).is_err());
    }

    #[test]
    fn identity_pins_model_metadata_and_each_runtime_depth_graph_without_onnx() {
        struct Directory(PathBuf);
        impl Drop for Directory {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let directory = Directory(
            std::env::temp_dir().join(format!("baylee-model-identity-{}", uuid::Uuid::now_v7())),
        );
        std::fs::create_dir(&directory.0).unwrap();
        let meta = json!({"encoder_version":7,"model_version":5,"architecture":V5_ARCHITECTURE,
            "trajectory_version":2,"sampling_version":2,"d_model":640,"n_heads":10,"n_layers":12});
        std::fs::write(
            directory.0.join("model.onnx.json"),
            serde_json::to_vec(&meta).unwrap(),
        )
        .unwrap();
        std::fs::write(directory.0.join("model.onnx"), b"main graph fixture").unwrap();
        std::fs::write(
            directory.0.join("model.k0.onnx"),
            b"observation graph fixture",
        )
        .unwrap();
        let identity = inspect_model_identity(&directory.0).unwrap();
        assert_eq!(identity.model_family, "v5");
        assert!(identity.critic_semantics.is_none());
        assert!(
            serde_json::to_value(&identity)
                .unwrap()
                .get("critic_semantics")
                .is_none()
        );
        assert_eq!(identity.architecture.as_deref(), Some(V5_ARCHITECTURE));
        assert_eq!(
            identity.onnx_sha256,
            hex_digest(&Sha256::digest(b"main graph fixture"))
        );
        let path = directory.0.join("model.k0.onnx");
        verify_depth_graph(&path, 0, &identity).unwrap();
        std::fs::write(&path, b"changed graph").unwrap();
        assert!(verify_depth_graph(&path, 0, &identity).is_err());
        assert_ne!(inspect_model_identity(&directory.0).unwrap(), identity);
        assert!(verify_depth_graph(&path, 2, &identity).is_err());
        let mut meta = meta;
        for semantics in [
            json!({"kind":"mtg_side_outcome_utility_v1"}),
            json!({"kind":"failure_terminal_research_utility_v1",
                "objective":"failure_terminal_utility_minus_frozen_source_value_uncentered_v1",
                "target_contract":"failure_terminal_research_v1"}),
        ] {
            meta["critic_semantics"] = semantics.clone();
            std::fs::write(
                directory.0.join("model.onnx.json"),
                serde_json::to_vec(&meta).unwrap(),
            )
            .unwrap();
            let marked = inspect_model_identity(&directory.0).unwrap();
            assert_eq!(
                serde_json::to_value(marked).unwrap()["critic_semantics"],
                semantics
            );
        }
        meta["critic_semantics"] = json!({"kind":"unrecognized"});
        std::fs::write(
            directory.0.join("model.onnx.json"),
            serde_json::to_vec(&meta).unwrap(),
        )
        .unwrap();
        assert!(inspect_model_identity(&directory.0).is_err());
    }

    #[test]
    fn v5_revision_two_is_explicit_and_unknown_revision_three_is_refused() {
        let mut meta = serde_json::json!({"encoder_version":7,"model_version":5,
            "architecture":V5_ARCHITECTURE_V2,"trajectory_version":2,"sampling_version":2,
            "d_model":640,"n_heads":10,"n_layers":12});
        assert_eq!(contract(&meta).unwrap().2, "v5");
        meta["architecture"] = serde_json::json!("baylee_ai_v5_relational_v3");
        assert!(contract(&meta).is_err());
    }
}
