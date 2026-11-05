//! Encrypted device archives and immutable history chunks.
use super::{Account, AccountData, Error, Zeroizing};

impl Account {
    /// Refresh and restore all key records for one exact gateway/account scope.
    /// No absence is returned for network, authentication or format failures.
    ///
    /// # Errors
    /// Refuses unavailable or damaged documents; callers must never generate replacements then.
    pub fn chat_devices(
        &mut self,
        gateway: &str,
        tag: &str,
    ) -> Result<Vec<crate::chat::ChatDevice>, Error> {
        self.session.refresh(&self.http)?;
        let current = AccountData::read(self.session.document())?;
        let devices = current
            .chat
            .devices
            .iter()
            .filter(|entry| entry.gateway == gateway && entry.tag == tag)
            .cloned()
            .collect();
        self.data = current;
        Ok(devices)
    }

    /// Durably add an installation's keys before registering its public device with a gateway.
    /// A retry of the exact record is idempotent; replacement is always a conflict.
    ///
    /// # Errors
    /// Rejects conflicts, invalid records and unavailable storage, preserving every existing key.
    pub fn archive_chat_device(&mut self, device: &crate::chat::ChatDevice) -> Result<(), Error> {
        device.validate()?;
        for _ in 0..3 {
            self.session.refresh(&self.http)?;
            let mut current = AccountData::read(self.session.document())?;
            current.chat.insert(device.clone())?;
            let encoded = current.write()?;
            if encoded.as_slice() != self.session.document() {
                match self.session.save(&self.http, &encoded) {
                    Err(Error::Conflict) => continue,
                    Err(error) => return Err(error),
                    Ok(()) => {}
                }
            }
            self.data = current;
            return Ok(());
        }
        Err(Error::Conflict)
    }

    /// Retain decryptability while marking an installation's identity permanently retired.
    ///
    /// # Errors
    /// Unknown or changed records, conflicts and unavailable storage are refused.
    pub fn retire_chat_device(&mut self, device: &crate::chat::ChatDevice) -> Result<(), Error> {
        self.session.refresh(&self.http)?;
        let mut current = AccountData::read(self.session.document())?;
        let existing = current
            .chat
            .devices
            .iter_mut()
            .find(|entry| entry.same_slot(device))
            .ok_or(Error::Rejected)?;
        if existing.key != device.key || existing.device != device.device {
            return Err(Error::Conflict);
        }
        existing.retired = true;
        let encoded = current.write()?;
        if encoded.as_slice() != self.session.document() {
            self.session.save(&self.http, &encoded)?;
        }
        self.data = current;
        Ok(())
    }

    /// Remember an installation without creating a second active protocol identity.
    ///
    /// # Errors
    /// Invalid identifiers, unknown keys, revoked identities and CAS failures are rejected.
    pub fn remember_chat_installation(
        &mut self,
        device: &crate::chat::ChatDevice,
        installation: String,
    ) -> Result<(), Error> {
        self.session.refresh(&self.http)?;
        let mut current = AccountData::read(self.session.document())?;
        let known = current
            .chat
            .devices
            .iter()
            .find(|entry| entry.same_slot(device))
            .ok_or(Error::Rejected)?;
        if known.key != device.key || known.device != device.device || known.retired {
            return Err(Error::Rejected);
        }
        current.chat.remember(device, installation)?;
        let encoded = current.write()?;
        if encoded.as_slice() != self.session.document() {
            self.session.save(&self.http, &encoded)?;
        }
        self.data = current;
        Ok(())
    }

    /// Publish a complete uploaded snapshot without overwriting another manifest generation.
    ///
    /// # Errors
    /// Any key mismatch, retirement, stale generation or unavailable storage fails closed.
    pub fn save_chat_history(
        &mut self,
        device: &crate::chat::ChatDevice,
        expected: u64,
        history: crate::chat::HistorySnapshot,
    ) -> Result<(), Error> {
        history.validate()?;
        if history.generation != expected.checked_add(1).ok_or(Error::Rejected)? {
            return Err(Error::Rejected);
        }
        self.session.refresh(&self.http)?;
        let mut current = AccountData::read(self.session.document())?;
        let existing = current
            .chat
            .devices
            .iter_mut()
            .find(|entry| entry.same_slot(device))
            .ok_or(Error::Rejected)?;
        if existing.key != device.key || existing.device != device.device || existing.retired {
            return Err(Error::Conflict);
        }
        if existing.history.as_ref() == Some(&history) {
            self.data = current;
            return Ok(());
        }
        if existing
            .history
            .as_ref()
            .map_or(0, |value| value.generation)
            != expected
        {
            return Err(Error::Conflict);
        }
        existing.history = Some(history);
        self.session.save(&self.http, &current.write()?)?;
        self.data = current;
        Ok(())
    }

    /// Upload a new immutable history chunk, encrypted inside the SDK.
    ///
    /// # Errors
    /// Allocation/write failure is not published to a manifest; ambiguous objects remain unreferenced.
    pub fn create_chat_chunk(&self, bytes: &[u8]) -> Result<crate::chat::HistoryChunk, Error> {
        use sha2::{Digest, Sha256};
        if bytes.is_empty() || bytes.len() > 256 * 1024 {
            return Err(Error::Rejected);
        }
        let object = self.session.create_object(&self.http)?;
        let revision = self
            .session
            .write_object(&self.http, object.id, object.revision, bytes)?;
        Ok(crate::chat::HistoryChunk {
            id: object.id,
            revision,
            bytes: u32::try_from(bytes.len()).map_err(|_| Error::Rejected)?,
            digest: Sha256::digest(bytes).into(),
        })
    }

    /// Read one exact revision/length from an authenticated history manifest.
    ///
    /// # Errors
    /// Changed, truncated, unavailable or unowned chunks are never treated as absent history.
    pub fn read_chat_chunk(
        &self,
        chunk: &crate::chat::HistoryChunk,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        let object = self
            .session
            .read_object(&self.http, chunk.id, chunk.revision)?;
        if object.revision != chunk.revision || !chunk.matches(&object.plaintext) {
            return Err(Error::Rejected);
        }
        Ok(object.plaintext)
    }

    /// Remove an unreferenced old chunk after a new manifest was acknowledged.
    ///
    /// # Errors
    /// A changed revision is not deleted; remote failure leaves harmless encrypted garbage.
    pub fn delete_chat_chunk(&self, chunk: &crate::chat::HistoryChunk) -> Result<(), Error> {
        self.session
            .delete_object(&self.http, chunk.id, chunk.revision)
            .map(|_| ())
    }

    /// Restore a complete ciphertext-file snapshot through authenticated, bounded object reads.
    ///
    /// # Errors
    /// A missing or damaged chunk aborts the whole snapshot without a partial-success result.
    pub fn read_chat_history(
        &self,
        device: &crate::chat::ChatDevice,
    ) -> Result<std::collections::BTreeMap<String, Vec<u8>>, Error> {
        device.validate()?;
        let mut files = std::collections::BTreeMap::new();
        if let Some(history) = &device.history {
            for file in &history.files {
                let mut bytes = Vec::new();
                for chunk in &file.chunks {
                    bytes.extend_from_slice(&self.read_chat_chunk(chunk)?);
                }
                files.insert(file.name.clone(), bytes);
            }
        }
        Ok(files)
    }

    /// Upload a complete per-device snapshot, then atomically publish its encrypted manifest.
    /// Old chunks are removed only after publication is acknowledged.
    ///
    /// # Errors
    /// Oversize input, source/key changes or any failed upload leave the previous manifest intact.
    pub fn upload_chat_history(
        &mut self,
        device: &crate::chat::ChatDevice,
        files: &std::collections::BTreeMap<String, Vec<u8>>,
    ) -> Result<(), Error> {
        use crate::chat::{HistoryFile, HistorySnapshot};
        // Validate all paths and sizes before allocating any server objects.
        let skeleton = HistorySnapshot {
            generation: 1,
            files: files
                .keys()
                .map(|name| HistoryFile {
                    name: name.clone(),
                    chunks: Vec::new(),
                })
                .collect(),
        };
        skeleton.validate()?;
        if files.values().any(|bytes| bytes.len() > 16 * 1024 * 1024)
            || files.values().map(Vec::len).sum::<usize>() > 256 * 1024 * 1024
        {
            return Err(Error::Rejected);
        }
        let records = self.chat_devices(&device.gateway, &device.tag)?;
        let current = records
            .iter()
            .find(|entry| entry.same_slot(device))
            .ok_or(Error::Rejected)?;
        if current.key != device.key || current.device != device.device || current.retired {
            return Err(Error::Conflict);
        }
        let previous = current.history.clone();
        let expected = previous.as_ref().map_or(0, |history| history.generation);
        let mut history = HistorySnapshot {
            generation: expected.checked_add(1).ok_or(Error::Rejected)?,
            files: Vec::new(),
        };
        for (name, bytes) in files {
            let mut chunks = Vec::new();
            let prior = previous
                .as_ref()
                .and_then(|history| history.files.iter().find(|file| file.name == *name));
            for (at, chunk) in bytes.chunks(256 * 1024).enumerate() {
                if let Some(kept) = prior
                    .and_then(|file| file.chunks.get(at))
                    .filter(|kept| kept.matches(chunk))
                {
                    chunks.push(kept.clone());
                } else {
                    chunks.push(self.create_chat_chunk(chunk)?);
                }
            }
            history.files.push(HistoryFile {
                name: name.clone(),
                chunks,
            });
        }
        if previous
            .as_ref()
            .is_some_and(|prior| prior.files == history.files)
        {
            return Ok(());
        }
        let referenced: std::collections::BTreeSet<_> = history
            .files
            .iter()
            .flat_map(|file| file.chunks.iter().map(|chunk| chunk.id))
            .collect();
        self.save_chat_history(device, expected, history)?;
        if let Some(previous) = previous {
            for file in previous.files {
                for chunk in file.chunks {
                    if !referenced.contains(&chunk.id) {
                        let _ = self.delete_chat_chunk(&chunk);
                    }
                }
            }
        }
        Ok(())
    }
}
