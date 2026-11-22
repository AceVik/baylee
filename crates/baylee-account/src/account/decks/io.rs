//! Encrypted object reads and generation-conditional account-root publication.
use super::{Account, Error};
use crate::{
    data::AccountData,
    decks::{Blob, Chunk, LibraryRef, Root},
};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::Zeroizing;

const CHUNK_SIZE: usize = 256 * 1024;
pub(super) const ROOT_LIMIT: usize = 8 * 1024 * 1024;
pub(super) const SNAPSHOT_LIMIT: usize = 1024 * 1024;
const MARKER: &[u8] = b"baylee/private-deck-reservation/1";
mod publication;

pub(super) struct Staged {
    chunks: Vec<(Uuid, u64)>,
    // Once a CAS may have reached Central, leave its objects intact unless
    // an authenticated read of that exact generation resolves the operation.
    publication_attempted: bool,
}
impl Staged {
    pub(super) fn new() -> Self {
        Self {
            chunks: Vec::new(),
            publication_attempted: false,
        }
    }
    pub(super) fn abort(self, account: &Account) {
        if !self.publication_attempted {
            for (id, revision) in self.chunks {
                let removed =
                    account
                        .session
                        .delete_object(&account.http, *id.as_bytes(), revision);
                if revision == 0 && removed == Err(Error::Conflict) {
                    // A timed-out chunk write can reach Central after we last
                    // observed the reservation. Only our first write can exist
                    // at this never-published freshly reserved identity.
                    let _ = account
                        .session
                        .delete_object(&account.http, *id.as_bytes(), 1);
                }
            }
        }
    }
}

impl Account {
    pub(in crate::account) fn refresh_library(&mut self) -> Result<Option<LibraryRef>, Error> {
        self.session.refresh(&self.http)?;
        self.data = AccountData::read(self.session.document())?;
        Ok(self.data.deck_library.clone())
    }
    pub(super) fn read_root(&mut self) -> Result<(Option<LibraryRef>, Root), Error> {
        let mut current = self.refresh_library()?;
        for _ in 0..3 {
            let Some(reference) = &current else {
                return Ok((None, Root::empty()));
            };
            let read = self
                .read_deck_blob::<Root>(&reference.blob, ROOT_LIMIT)
                .and_then(|root| {
                    root.validate(reference.generation)?;
                    Ok(root)
                });
            match read {
                Ok(root) => return Ok((current, root)),
                Err(error) => {
                    let latest = self.refresh_library()?;
                    if latest == current {
                        return Err(error);
                    }
                    current = latest;
                }
            }
        }
        Err(Error::Conflict)
    }
    pub(super) fn read_deck_blob<T: DeserializeOwned>(
        &self,
        blob: &Blob,
        limit: usize,
    ) -> Result<T, Error> {
        blob.validate(limit)?;
        let mut bytes = Zeroizing::new(Vec::with_capacity(
            usize::try_from(blob.bytes).map_err(|_| Error::Rejected)?,
        ));
        for chunk in &blob.chunks {
            let opened =
                self.session
                    .read_object(&self.http, *chunk.id.as_bytes(), chunk.revision)?;
            if opened.revision != chunk.revision
                || opened.plaintext.len()
                    != usize::try_from(chunk.bytes).map_err(|_| Error::Rejected)?
                || <[u8; 32]>::from(Sha256::digest(&opened.plaintext)) != chunk.digest
            {
                return Err(Error::Rejected);
            }
            bytes.extend_from_slice(&opened.plaintext);
        }
        if <[u8; 32]>::from(Sha256::digest(&bytes)) != blob.digest {
            return Err(Error::Rejected);
        }
        serde_json::from_slice(&bytes).map_err(|_| Error::Rejected)
    }
    fn fresh_object(&self, bytes: &[u8], stage: &mut Staged) -> Result<Uuid, Error> {
        let object = self.session.create_object(&self.http)?;
        let id = Uuid::from_bytes(object.id);
        stage.chunks.push((id, 0));
        if let Err(error) = self.session.write_object(&self.http, object.id, 0, bytes) {
            let opened = self
                .session
                .read_object(&self.http, object.id, 1)
                .map_err(|_| error)?;
            if opened.revision != 1 || opened.plaintext.as_slice() != bytes {
                return Err(Error::Conflict);
            }
        }
        if let Some(item) = stage.chunks.last_mut() {
            item.1 = 1;
        }
        Ok(id)
    }
    pub(super) fn stage_deck_id(&self, stage: &mut Staged) -> Result<Uuid, Error> {
        self.fresh_object(MARKER, stage)
    }
    pub(super) fn verify_reservation(&self, id: Uuid) -> Result<(), Error> {
        let object = self.session.read_object(&self.http, *id.as_bytes(), 1)?;
        if object.revision != 1 || object.plaintext.as_slice() != MARKER {
            return Err(Error::Rejected);
        }
        Ok(())
    }
    pub(super) fn write_deck_blob<T: Serialize>(
        &self,
        value: &T,
        limit: usize,
        stage: &mut Staged,
    ) -> Result<Blob, Error> {
        let bytes = Zeroizing::new(serde_json::to_vec(value).map_err(|_| Error::Rejected)?);
        if bytes.is_empty() || bytes.len() > limit {
            return Err(Error::Rejected);
        }
        let mut chunks = Vec::new();
        for part in bytes.chunks(CHUNK_SIZE) {
            let id = self.fresh_object(part, stage)?;
            chunks.push(Chunk {
                id,
                revision: 1,
                bytes: u32::try_from(part.len()).map_err(|_| Error::Rejected)?,
                digest: Sha256::digest(part).into(),
            });
        }
        Ok(Blob {
            chunks,
            bytes: u32::try_from(bytes.len()).map_err(|_| Error::Rejected)?,
            digest: Sha256::digest(&bytes).into(),
        })
    }
    pub(super) fn forget_blob(&self, blob: Blob) {
        for chunk in blob.chunks {
            let _ = self
                .session
                .delete_object(&self.http, *chunk.id.as_bytes(), chunk.revision);
        }
    }
    pub(super) fn publish_root(
        &mut self,
        base: Option<&LibraryRef>,
        mut root: Root,
        stage: &mut Staged,
    ) -> Result<(), Error> {
        root.generation = root.generation.checked_add(1).ok_or(Error::Rejected)?;
        root.validate(root.generation)?;
        let next = LibraryRef {
            generation: root.generation,
            blob: self.write_deck_blob(&root, ROOT_LIMIT, stage)?,
        };
        publication::publish(self, base, &next, stage)
    }
}
