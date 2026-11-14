//! Locally configured executables, probed before connecting to a gateway.
use baylee_protocol::execution::{
    Identity, MAX_CAPABILITIES, MAX_METADATA_BYTES, valid_capabilities,
};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt as _;

/// The default installation's known siblings only. No directory scan, PATH
/// search, remote name or executable sent by a gateway. A developer with only
/// one built engine can still run that one; deployment requires the whole set.
pub(crate) fn installed_siblings(primary: &Path) -> Vec<PathBuf> {
    let Some(directory) = primary.parent() else {
        return Vec::new();
    };
    ["baylee-engine-standard-loops", "baylee-engine-tower"]
        .map(|name| directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)))
        .into_iter()
        .filter(|path| path.is_file())
        .collect()
}

/// Installed artifacts. Paths never travel over the control link.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory(Vec<(Identity, PathBuf)>);

impl Inventory {
    /// Admits a bounded, unambiguous local mapping. No relative/PATH lookup at spawn.
    ///
    /// # Errors
    /// Empty/duplicate/invalid identities, or nonabsolute/duplicate executable paths.
    pub fn new(entries: Vec<(Identity, PathBuf)>) -> Result<Self, &'static str> {
        let identities: Vec<_> = entries.iter().map(|(id, _)| id.clone()).collect();
        if !valid_capabilities(&identities)
            || entries.iter().enumerate().any(|(at, (_, path))| {
                !path.is_absolute() || entries[..at].iter().any(|(_, other)| other == path)
            })
        {
            return Err("invalid or ambiguous local engine inventory");
        }
        Ok(Self(entries))
    }

    /// Probes only operator-configured paths, with bounded output and runtime.
    ///
    /// # Errors
    /// Any configured executable fails its isolated metadata command.
    pub async fn discover(paths: &[PathBuf]) -> Result<Self, &'static str> {
        if paths.is_empty() || paths.len() > MAX_CAPABILITIES {
            return Err("configure between one and sixteen engine executables");
        }
        let mut entries = Vec::with_capacity(paths.len());
        for path in paths {
            let path = std::fs::canonicalize(path).map_err(|_| "engine executable not found")?;
            let identity = probe(&path).await?;
            entries.push((identity, path));
        }
        Self::new(entries)
    }

    /// The identities the gateway may select; no path, arguments or environment.
    #[must_use]
    pub fn metadata(&self) -> Vec<Vec<u8>> {
        self.0.iter().map(|(id, _)| id.to_metadata()).collect()
    }

    /// Exact lookup. A missing variant/build never falls back to another artifact.
    #[must_use]
    pub fn executable(&self, identity: &Identity) -> Option<&Path> {
        self.0
            .iter()
            .find(|(id, _)| id == identity)
            .map(|(_, path)| path.as_path())
    }
}

async fn probe(path: &Path) -> Result<Identity, &'static str> {
    let mut child = tokio::process::Command::new(path)
        .arg("--metadata")
        .env_remove(crate::AGENT_TOKEN_VAR)
        .env_remove(crate::ENGINE_TOKEN_VAR)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| "cannot probe engine executable")?;
    let output = child.stdout.take().expect("piped metadata output");
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let mut bytes = Vec::with_capacity(MAX_METADATA_BYTES);
        output
            .take((MAX_METADATA_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| "cannot read engine metadata")?;
        let identity = Identity::from_metadata(&bytes).ok_or("invalid engine metadata")?;
        let exit = child
            .wait()
            .await
            .map_err(|_| "cannot reap metadata probe")?;
        if !exit.success() {
            return Err("engine metadata command failed");
        }
        Ok(identity)
    })
    .await
    .unwrap_or(Err("engine metadata command timed out"));
    if result.is_err() {
        let _ = child.start_kill();
        child
            .wait()
            .await
            .map_err(|_| "cannot reap failed metadata probe")?;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_protocol::execution::Variant;

    #[test]
    fn exact_mapping_refuses_duplicates_relative_paths_and_every_identity_skew() {
        let id = Identity::native(Variant::Standard, "build", "revision", 19);
        let path = std::env::temp_dir().join("standard");
        let inventory = Inventory::new(vec![(id.clone(), path.clone())]).unwrap();
        assert_eq!(inventory.executable(&id), Some(path.as_path()));
        assert!(Inventory::new(vec![]).is_err());
        assert!(Inventory::new(vec![(id.clone(), "relative".into())]).is_err());
        assert!(
            Inventory::new(vec![(id.clone(), path.clone()), (id.clone(), path.clone())]).is_err()
        );
        let mut different = id.clone();
        different.variant = Variant::Tower;
        assert!(inventory.executable(&different).is_none());
        for field in ["build", "features", "rules", "cards", "ai"] {
            let mut different = id.clone();
            match field {
                "build" => different.build.push('x'),
                "features" => different.features = "b".repeat(64),
                "rules" => different.rules.push('x'),
                "cards" => different.cards.push('x'),
                _ => different.ai.push('x'),
            }
            assert!(inventory.executable(&different).is_none());
        }
        assert_eq!(Identity::from_metadata(&inventory.metadata()[0]), Some(id));
    }
}
