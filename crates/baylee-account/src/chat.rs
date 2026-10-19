//! Account-encrypted chat archive. One logical identity is active per gateway/account.
use crate::Error;
use serde::{
    Deserialize, Serialize,
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
};
use zeroize::Zeroizing;

/// Fixed-size secret material, wiped on drop and deliberately neither Debug nor Display.
pub struct DeviceKey(Zeroizing<[u8; 96]>);

impl DeviceKey {
    /// Take ownership of a zeroizing key allocation.
    #[must_use]
    pub const fn new(bytes: Zeroizing<[u8; 96]>) -> Self {
        Self(bytes)
    }
    /// Borrow only the chat device material; this never contains the account master key.
    #[must_use]
    pub fn bytes(&self) -> &[u8; 96] {
        &self.0
    }
}
impl Clone for DeviceKey {
    fn clone(&self) -> Self {
        Self(Zeroizing::new(*self.0))
    }
}
impl PartialEq for DeviceKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_slice() == other.0.as_slice()
    }
}
impl Eq for DeviceKey {}
impl Serialize for DeviceKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(96))?;
        for byte in self.0.iter() {
            sequence.serialize_element(byte)?;
        }
        sequence.end()
    }
}
impl<'de> Deserialize<'de> for DeviceKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;
        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = DeviceKey;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("one fixed-size chat device secret")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut bytes = Zeroizing::new([0; 96]);
                for byte in bytes.iter_mut() {
                    *byte = sequence
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::custom("invalid chat key length"))?;
                }
                if sequence.next_element::<u8>()?.is_some() {
                    return Err(serde::de::Error::custom("invalid chat key length"));
                }
                Ok(DeviceKey(bytes))
            }
        }
        deserializer.deserialize_seq(KeyVisitor)
    }
}

/// Logical chat identity. Retired keys remain available to decrypt old history.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatDevice {
    /// Canonical gateway origin; keys never cross gateway or account-tag boundaries.
    pub gateway: String,
    /// Gateway-local public account tag.
    pub tag: String,
    /// Stable archive slot, initially allocated by the installation that created this identity.
    pub installation: String,
    /// Public identifier derived from the signing key by the chat crypto library.
    pub device: String,
    /// Secret material, inside the centrally encrypted account document only.
    pub key: DeviceKey,
    /// Revocation tombstone: retaining decryptability must never reactivate a device.
    pub retired: bool,
    /// Latest complete per-device encrypted history snapshot, published only after all chunks.
    #[serde(default)]
    pub history: Option<HistorySnapshot>,
}

/// Immutable encrypted-object reference, authenticated by the encrypted manifest.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryChunk {
    /// PostgreSQL-generated object `UUIDv7`.
    pub id: [u8; 16],
    /// Exact authenticated object revision; readers reject both older and unexpected versions.
    pub revision: u64,
    /// Exact plaintext chunk length, at most 256 KiB.
    pub bytes: u32,
    /// SHA-256 of the plaintext chunk, retained only inside the encrypted manifest.
    pub digest: [u8; 32],
}

impl HistoryChunk {
    /// Match the exact authenticated chunk content for reuse and restore verification.
    #[must_use]
    pub fn matches(&self, bytes: &[u8]) -> bool {
        use sha2::{Digest, Sha256};
        bytes.len() == self.bytes as usize && <[u8; 32]>::from(Sha256::digest(bytes)) == self.digest
    }
}

/// One local ciphertext file, split into immutable, client-encrypted central objects.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryFile {
    /// `state.bin` or a lowercase HMAC-named file immediately below `history/`.
    pub name: String,
    /// Ordered chunks; object identifiers are unique throughout the snapshot.
    pub chunks: Vec<HistoryChunk>,
}

/// Complete per-device history manifest. Ciphertext files never contain a plaintext key.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistorySnapshot {
    /// Monotonic manifest generation, independent of object revisions.
    pub generation: u64,
    /// Complete list; no paths outside the bounded chat namespace are accepted.
    pub files: Vec<HistoryFile>,
}

impl HistorySnapshot {
    /// Check all allocation, path, chunk and context bounds before issuing object reads.
    ///
    /// # Errors
    /// Unsupported identifiers, duplicates, invalid paths and oversized snapshots.
    pub fn validate(&self) -> Result<(), Error> {
        let mut names = std::collections::BTreeSet::new();
        let mut objects = std::collections::BTreeSet::new();
        let mut total = 0u64;
        if self.generation == 0 || self.files.len() > 1025 {
            return Err(Error::Rejected);
        }
        for file in &self.files {
            let history_name = file
                .name
                .strip_prefix("history/")
                .and_then(|name| name.strip_suffix(".log"));
            if (file.name != "state.bin"
                && !history_name.is_some_and(|name| {
                    name.len() == 16
                        && name
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                }))
                || !names.insert(&file.name)
                || file.chunks.len() > 64
            {
                return Err(Error::Rejected);
            }
            let mut size = 0u64;
            for chunk in &file.chunks {
                if chunk.id[6] >> 4 != 7
                    || chunk.id[8] >> 6 != 2
                    || chunk.revision == 0
                    || chunk.bytes == 0
                    || chunk.bytes > 256 * 1024
                    || !objects.insert(chunk.id)
                {
                    return Err(Error::Rejected);
                }
                size += u64::from(chunk.bytes);
            }
            total += size;
        }
        if total > 256 * 1024 * 1024 {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}

impl ChatDevice {
    /// Validate bounded, canonical routing labels before persistence or restoration.
    ///
    /// # Errors
    /// Malformed origin, public identifiers or installation id.
    pub fn validate(&self) -> Result<(), Error> {
        if let Some(history) = &self.history {
            history.validate()?;
        }
        let origin_ok = central_api::gateway::validate_origin(&self.gateway, false).is_ok()
            || central_api::gateway::validate_origin(&self.gateway, true).is_ok();
        let id = self.installation.as_bytes();
        if !origin_ok
            || self.tag.is_empty()
            || self.tag.len() > 128
            || !self.tag.bytes().all(|b| b.is_ascii_alphanumeric())
            || self.device.is_empty()
            || self.device.len() > 96
            || !self
                .device
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            || id.len() != 36
            || id[14] != b'7'
            || !b"89ab".contains(&id[19])
            || !id.iter().enumerate().all(|(at, byte)| {
                if [8, 13, 18, 23].contains(&at) {
                    *byte == b'-'
                } else {
                    byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
                }
            })
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }

    #[cfg(any(feature = "client", test))]
    pub(crate) fn same_slot(&self, other: &Self) -> bool {
        self.gateway == other.gateway
            && self.tag == other.tag
            && self.installation == other.installation
    }
}

/// Bounded encrypted key archive. New installations resume the first active logical identity.
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatArchive {
    /// Current and retired identities, including imported legacy devices.
    pub devices: Vec<ChatDevice>,
    /// Installations that restored an identity; this does not create concurrent active sessions.
    #[serde(default)]
    pub installations: Vec<ChatInstallation>,
}

/// Known installation metadata, kept inside the encrypted account archive.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatInstallation {
    /// Canonical gateway origin.
    pub gateway: String,
    /// Gateway-local account tag.
    pub tag: String,
    /// Locally generated installation `UUIDv7`.
    pub id: String,
    /// The logical chat device that this installation last restored.
    pub device: String,
}
impl ChatArchive {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.devices.len() > 512 || self.installations.len() > 1024 {
            return Err(Error::Rejected);
        }
        let mut slots = std::collections::BTreeSet::new();
        let mut identities = std::collections::BTreeSet::new();
        for device in &self.devices {
            device.validate()?;
            if !slots.insert((&device.gateway, &device.tag, &device.installation))
                || !identities.insert((&device.gateway, &device.tag, &device.device))
            {
                return Err(Error::Rejected);
            }
        }
        let mut known = std::collections::BTreeSet::new();
        for installation in &self.installations {
            if !valid_installation(&installation.id)
                || !known.insert((&installation.gateway, &installation.tag, &installation.id))
                || !self.devices.iter().any(|device| {
                    device.gateway == installation.gateway
                        && device.tag == installation.tag
                        && device.device == installation.device
                })
            {
                return Err(Error::Rejected);
            }
        }
        Ok(())
    }

    #[cfg(feature = "client")]
    pub(crate) fn remember(&mut self, device: &ChatDevice, id: String) -> Result<(), Error> {
        if !valid_installation(&id) {
            return Err(Error::Rejected);
        }
        if let Some(known) = self.installations.iter_mut().find(|known| {
            known.gateway == device.gateway && known.tag == device.tag && known.id == id
        }) {
            known.device.clone_from(&device.device);
        } else {
            if self.installations.len() >= 1024 {
                return Err(Error::Rejected);
            }
            self.installations.push(ChatInstallation {
                gateway: device.gateway.clone(),
                tag: device.tag.clone(),
                id,
                device: device.device.clone(),
            });
        }
        self.validate()
    }

    #[cfg(any(feature = "client", test))]
    pub(crate) fn insert(&mut self, device: ChatDevice) -> Result<(), Error> {
        device.validate()?;
        if let Some(existing) = self.devices.iter().find(|entry| entry.same_slot(&device)) {
            return if existing.key == device.key
                && existing.device == device.device
                && existing.retired == device.retired
            {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        if self.devices.len() >= 512
            || self.devices.iter().any(|entry| {
                entry.gateway == device.gateway
                    && entry.tag == device.tag
                    && entry.device == device.device
            })
        {
            return Err(Error::Conflict);
        }
        self.devices.push(device);
        Ok(())
    }
}

fn valid_installation(value: &str) -> bool {
    let id = value.as_bytes();
    id.len() == 36
        && id[14] == b'7'
        && b"89ab".contains(&id[19])
        && id.iter().enumerate().all(|(at, byte)| {
            if [8, 13, 18, 23].contains(&at) {
                *byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn device() -> ChatDevice {
        ChatDevice {
            gateway: "https://gateway.example".into(),
            tag: "1234".into(),
            installation: "019a0000-0000-7000-8000-000000000001".into(),
            device: "device_one".into(),
            key: DeviceKey::new(Zeroizing::new([7; 96])),
            retired: false,
            history: None,
        }
    }
    #[test]
    fn restored_archive_preserves_keys_and_rejects_conflicting_or_duplicate_identity() {
        let original = device();
        let mut archive = ChatArchive::default();
        archive.insert(original.clone()).unwrap();
        archive.insert(original.clone()).unwrap();
        assert_eq!(archive.devices.len(), 1);
        let mut changed = original.clone();
        changed.key = DeviceKey::new(Zeroizing::new([8; 96]));
        assert!(matches!(archive.insert(changed), Err(Error::Conflict)));
        let mut cloned_installation = original;
        cloned_installation.installation = "019a0000-0000-7000-8000-000000000002".into();
        assert!(matches!(
            archive.insert(cloned_installation),
            Err(Error::Conflict)
        ));
        let encoded = Zeroizing::new(serde_json::to_vec(&archive).unwrap());
        let restored: ChatArchive = serde_json::from_slice(&encoded).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.devices[0].key.bytes(), &[7; 96]);
        for len in [0, 95, 97] {
            assert!(
                serde_json::from_slice::<DeviceKey>(&serde_json::to_vec(&vec![7u8; len]).unwrap())
                    .is_err()
            );
        }
    }

    #[test]
    fn chunk_reuse_requires_exact_size_and_digest_and_manifests_bind_paths() {
        use sha2::{Digest, Sha256};
        let content = b"synthetic encrypted history";
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let chunk = HistoryChunk {
            id,
            revision: 1,
            bytes: u32::try_from(content.len()).unwrap(),
            digest: Sha256::digest(content).into(),
        };
        assert!(chunk.matches(content));
        assert!(!chunk.matches(b"synthetic encrypted historx"));
        assert!(!chunk.matches(&content[..content.len() - 1]));
        let mut history = HistorySnapshot {
            generation: 1,
            files: vec![HistoryFile {
                name: "state.bin".into(),
                chunks: vec![chunk.clone()],
            }],
        };
        history.validate().unwrap();
        history.files[0].name = "history/0123456789abcdef.log".into();
        history.validate().unwrap();
        history.files.push(history.files[0].clone());
        assert!(history.validate().is_err());
        history.files.pop();
        for path in [
            "../state.bin",
            "history/../state.bin",
            "history/0123456789ABCDEF.log",
            "history/0123456789abcdef.log/extra",
        ] {
            history.files[0].name = path.into();
            assert!(history.validate().is_err());
        }
    }
}
