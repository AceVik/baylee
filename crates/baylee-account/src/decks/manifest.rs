#[cfg(feature = "client")]
use super::{DeckSave, DeckSource, DeckSummary};
use crate::Error;
use serde::{Deserialize, Serialize};
#[cfg(feature = "client")]
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use uuid::Uuid;

pub(super) const CHUNK_BYTES: usize = 256 * 1024;

/// The encrypted account document points to this exact immutable library generation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryRef {
    pub(crate) generation: u64,
    pub(crate) blob: Blob,
}
impl LibraryRef {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.generation == 0 {
            return Err(Error::Rejected);
        }
        self.blob.validate(8 * 1024 * 1024)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Blob {
    pub(crate) bytes: u32,
    pub(crate) digest: [u8; 32],
    pub(crate) chunks: Vec<Chunk>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Chunk {
    pub(crate) id: Uuid,
    pub(crate) revision: u64,
    pub(crate) bytes: u32,
    pub(crate) digest: [u8; 32],
}
impl Blob {
    pub(crate) fn validate(&self, limit: usize) -> Result<(), Error> {
        let n = usize::try_from(self.bytes).map_err(|_| Error::Rejected)?;
        if n == 0 || n > limit || self.chunks.len() != n.div_ceil(CHUNK_BYTES) {
            return Err(Error::Rejected);
        }
        let mut seen = BTreeSet::new();
        for (index, chunk) in self.chunks.iter().enumerate() {
            if chunk.revision != 1
                || chunk.id.get_version_num() != 7
                || !seen.insert(chunk.id)
                || usize::try_from(chunk.bytes).map_err(|_| Error::Rejected)?
                    != (n - index * CHUNK_BYTES).min(CHUNK_BYTES)
            {
                return Err(Error::Rejected);
            }
        }
        Ok(())
    }
}

#[cfg(feature = "client")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Head {
    pub(crate) summary: DeckSummary,
    pub(crate) source: Option<DeckSource>,
    pub(crate) log: Blob,
}
#[cfg(feature = "client")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Root {
    pub(crate) kind: String,
    pub(crate) generation: u64,
    pub(crate) heads: BTreeMap<Uuid, Head>,
    pub(crate) removed: BTreeMap<Uuid, Option<DeckSource>>,
    pub(crate) imports: BTreeSet<[u8; 32]>,
}
#[cfg(feature = "client")]
impl Root {
    pub(crate) fn empty() -> Self {
        Self {
            kind: "baylee/deck-library/1".into(),
            generation: 0,
            heads: BTreeMap::new(),
            removed: BTreeMap::new(),
            imports: BTreeSet::new(),
        }
    }
    pub(crate) fn validate(&self, generation: u64) -> Result<(), Error> {
        if self.kind != "baylee/deck-library/1"
            || self.generation != generation
            || self.heads.len() > 1000
            || self.removed.len() > 16_384
            || self.imports.len() > 8192
        {
            return Err(Error::Rejected);
        }
        for (id, head) in &self.heads {
            let s = &head.summary;
            if *id != s.id
                || id.get_version_num() != 7
                || self.removed.contains_key(id)
                || s.revision == 0
                || s.name.is_empty()
                || s.name.len() > 64
                || s.format.len() > 64
                || s.commanders.len() > 2
                || s.commanders.iter().any(|s| s.len() > 512)
                || s.cards > 250
                || s.sideboard > 250
            {
                return Err(Error::Rejected);
            }
            head.log.validate(8 * 1024 * 1024)?;
        }
        Ok(())
    }
}

#[cfg(feature = "client")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Revision {
    pub(crate) save: DeckSave,
    pub(crate) blob: Blob,
}
#[cfg(feature = "client")]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Log {
    pub(crate) kind: String,
    pub(crate) deck: Uuid,
    pub(crate) history: Vec<Revision>,
}
#[cfg(feature = "client")]
impl Log {
    pub(crate) fn empty(deck: Uuid) -> Self {
        Self {
            kind: "baylee/deck-history/1".into(),
            deck,
            history: Vec::new(),
        }
    }
    pub(crate) fn validate(&self, head: &Head) -> Result<(), Error> {
        if self.kind != "baylee/deck-history/1"
            || self.deck != head.summary.id
            || self.history.len() > 8192
            || self.history.last().map(|v| v.save.revision) != Some(head.summary.revision)
        {
            return Err(Error::Rejected);
        }
        let mut revision = 0;
        for v in &self.history {
            if v.save.revision <= revision
                || v.save.cards > 250
                || v.save.sideboard > 250
                || v.save.summary.as_ref().is_some_and(|s| s.len() > 1024)
            {
                return Err(Error::Rejected);
            }
            v.blob.validate(1024 * 1024)?;
            revision = v.save.revision;
        }
        Ok(())
    }
}
