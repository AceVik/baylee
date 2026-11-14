use crate::{Error, process};
use baylee_protocol::execution::{Identity, MAX_METADATA_BYTES, Variant};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{io::AsyncReadExt as _, sync::oneshot};

/// Probed local binary. Paths never come from a player, gateway or job header.
#[derive(Clone)]
pub struct Artifact {
    pub(crate) path: PathBuf,
    pub(crate) identity: Identity,
}

impl Artifact {
    /// Probe one absolute operator-configured executable with an empty environment.
    /// The owned probe is killed and reaped even if this future is dropped.
    /// # Errors
    /// Invalid local path, oversized/invalid metadata, wrong variant or failed probe.
    pub async fn probe(path: &Path) -> Result<Self, Error> {
        if !path.is_absolute() {
            return Err(Error::Contract);
        }
        let path = std::fs::canonicalize(path).map_err(|_| Error::Process)?;
        let (keep, cancelled) = oneshot::channel();
        let worker = tokio::spawn(probe(path, cancelled));
        let result = worker.await.map_err(|_| Error::Process)?;
        drop(keep);
        result
    }

    /// Exact compiled contracts that this local binary reported.
    #[must_use]
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
}

async fn probe(path: PathBuf, cancelled: oneshot::Receiver<()>) -> Result<Artifact, Error> {
    let mut child = process::command(&path, "--metadata")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| Error::Process)?;
    let stdout = child.stdout.take().ok_or(Error::Process)?;
    let pid = child.id().ok_or(Error::Process)?;
    let read = async {
        let mut bytes = Vec::with_capacity(MAX_METADATA_BYTES);
        stdout
            .take((MAX_METADATA_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| Error::Process)?;
        let identity = Identity::from_metadata(&bytes).ok_or(Error::Contract)?;
        if identity.variant != Variant::Tower
            || !child.wait().await.map_err(|_| Error::Process)?.success()
        {
            return Err(Error::Contract);
        }
        Ok(identity)
    };
    let result = tokio::select! {
        biased;
        _ = cancelled => Err(Error::Cancelled),
        error = process::memory(pid, baylee_worker::Limits::default().resident_bytes) => Err(error),
        result = tokio::time::timeout(Duration::from_secs(5), read) => result.unwrap_or(Err(Error::Budget)),
    };
    process::reap(&mut child, result.is_err()).await?;
    result.map(|identity| Artifact { path, identity })
}
