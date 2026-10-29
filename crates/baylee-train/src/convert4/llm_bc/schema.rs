use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilePin {
    pub path: PathBuf,
    pub sha256: String,
}
impl FilePin {
    pub(super) fn bytes(&self) -> anyhow::Result<Vec<u8>> {
        ensure!(
            self.path.is_absolute(),
            "LLM evidence path must be absolute"
        );
        ensure!(
            std::fs::metadata(&self.path)?.len() <= 64 * 1024 * 1024,
            "LLM evidence too large"
        );
        let bytes = std::fs::read(&self.path)?;
        ensure!(
            super::sha(&bytes) == self.sha256,
            "LLM evidence hash mismatch"
        );
        Ok(bytes)
    }
    pub(super) fn json(&self) -> anyhow::Result<Value> {
        serde_json::from_slice(&self.bytes()?).context("LLM evidence JSON")
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SeatManifest {
    pub seat: u8,
    pub model: String,
    pub transcript: FilePin,
    pub origins: FilePin,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u8,
    pub contract: String,
    pub source_game: String,
    /// Storage alias when several collectors reuse the same attempt identifier.
    /// The original `source_game` still binds the unchanged admission evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frozen_game: Option<String>,
    pub record_sha256: String,
    pub record_artifact_sha256: String,
    pub request: FilePin,
    pub admitted_receipt: FilePin,
    pub technical_result: FilePin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_receipt: Option<FilePin>,
    pub seats: Vec<SeatManifest>,
}
