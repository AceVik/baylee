//! Restore paged, staged legacy deck exports before Central seals their final link.
use super::{Account, AccountData, Error, Source, legacy_decks, overlay};
use central_api::legacy::{ArchiveManifest, source_digest, source_generation};

impl Account {
    fn source_receipt(
        &mut self,
        source: [u8; 32],
        manifest: ArchiveManifest,
    ) -> Result<Option<[u8; 32]>, Error> {
        self.refresh_library()?;
        let legacy = self.data.legacy.as_ref().ok_or(Error::Rejected)?;
        if legacy.digest != source || legacy.paged != Some(manifest) {
            return Err(Error::Conflict);
        }
        let Some(digest) = legacy.restored_digest else {
            return Ok(None);
        };
        let root = legacy.restored_root.ok_or(Error::Rejected)?;
        if source_generation(&legacy.account_id, digest, manifest.bytes) != manifest.generation {
            return Err(Error::Rejected);
        }
        let marker = legacy_decks::import_key(legacy);
        let origin = legacy.gateway.clone();
        let account = legacy.account_id.clone();
        self.verify_prelink_decks(root, marker, &origin, &account, None)?;
        Ok(Some(digest))
    }

    /// A confirmed move can lose its response after the server closes access to its source.
    /// The local, pre-close encrypted receipt permits completion in a fresh session.
    pub(super) fn finish_paged_move(&mut self) -> Result<(), Error> {
        let legacy = self.data.legacy.as_ref().ok_or(Error::Rejected)?;
        let source = legacy.digest;
        let manifest = legacy.paged.ok_or(Error::Rejected)?;
        let complete = legacy.complete;
        let closed = self.session.migration_closed(&self.http)?;
        if closed && complete {
            return Ok(());
        }
        let receipt = match self.source_receipt(source, manifest) {
            Err(Error::Conflict) if !closed => None,
            result => result?,
        };
        if closed {
            return receipt.map(|_| ()).ok_or(Error::Rejected);
        }
        let digest = match receipt {
            Some(digest) => digest,
            None => self.restore_paged_decks(source, manifest)?,
        };
        match self
            .session
            .legacy_finish_archive(&self.http, manifest.generation, digest)
        {
            Ok(()) => Ok(()),
            Err(Error::Unavailable | Error::Conflict | Error::Rejected)
                if self.session.migration_closed(&self.http)? =>
            {
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    fn restore_paged_decks(
        &mut self,
        source: [u8; 32],
        manifest: ArchiveManifest,
    ) -> Result<[u8; 32], Error> {
        let legacy = self.data.legacy.as_ref().ok_or(Error::Rejected)?;
        if legacy.digest != source || legacy.paged != Some(manifest) {
            return Err(Error::Conflict);
        }
        let owner = legacy.account_id.clone();
        let origin = legacy.gateway.clone();
        let marker = legacy_decks::import_key(legacy);
        let bytes = self.session.legacy_archive(&self.http, &owner, &manifest)?;
        let digest = source_digest(&bytes);
        let archive: Source = serde_json::from_slice(&bytes).map_err(|_| Error::Rejected)?;
        drop(bytes);
        if !archive.preferences.is_object() {
            return Err(Error::Rejected);
        }
        let decks = legacy_decks::decode_heads(
            &origin,
            &owner,
            archive.decks.as_array().ok_or(Error::Rejected)?,
        )?;
        let checks = super::decks::source_checks(&decks)?;
        self.import_decks(marker, decks)?;
        let root = self.refresh_library()?.ok_or(Error::Rejected)?.blob.digest;
        self.verify_prelink_decks(root, marker, &origin, &owner, Some(checks))?;
        for _ in 0..3 {
            if self.refresh_library()?.as_ref().map(|r| r.blob.digest) != Some(root) {
                return Err(Error::Conflict);
            }
            let mut current = AccountData::read(self.session.document())?;
            let legacy = current.legacy.as_mut().ok_or(Error::Rejected)?;
            if legacy.digest != source || legacy.paged != Some(manifest) {
                return Err(Error::Conflict);
            }
            if legacy.restored_digest == Some(digest) && legacy.restored_root == Some(root) {
                return Ok(digest);
            }
            legacy.restored_digest = Some(digest);
            legacy.restored_root = Some(root);
            // Preserve independent current settings (including unknown keys)
            // while restoring every missing original preference.
            let mut preferences = archive.preferences.clone();
            overlay(&mut preferences, current.preferences);
            current.preferences = preferences;
            match self.session.save(&self.http, &current.write()?) {
                Ok(()) => {
                    self.data = current;
                    return Ok(digest);
                }
                Err(Error::Conflict | Error::Unavailable) => (),
                Err(error) => return Err(error),
            }
        }
        Err(Error::Conflict)
    }

    pub(super) fn confirm_legacy_closed(&mut self, digest: [u8; 32]) -> Result<(), Error> {
        for _ in 0..3 {
            self.refresh_library()?;
            let mut current = AccountData::read(self.session.document())?;
            let legacy = current.legacy.as_mut().ok_or(Error::Rejected)?;
            if legacy.digest != digest {
                return Err(Error::Conflict);
            }
            if legacy.complete {
                return Ok(());
            }
            legacy.complete = true;
            match self.session.save(&self.http, &current.write()?) {
                Ok(()) => {
                    self.data = current;
                    return Ok(());
                }
                Err(Error::Conflict | Error::Unavailable) => (),
                Err(error) => return Err(error),
            }
        }
        Err(Error::Conflict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::legacy::{ArchiveSource, LegacySnapshot};

    #[test]
    fn enrollment_withholds_empty_library_success_until_manifest_and_source_are_verified() {
        let owner = uuid::Uuid::now_v7().to_string();
        let bytes = 512 * 1024;
        let sha = [21; 32];
        let manifest = ArchiveManifest {
            version: 1,
            bytes,
            generation: source_generation(&owner, sha, bytes),
        };
        let document = central_api::encode(&ArchiveSource {
            paged_deck_source: manifest,
        })
        .unwrap();
        let mut data = super::super::from_source(
            vec![],
            "http://127.0.0.1:1234",
            LegacySnapshot {
                legacy_id: owner,
                username: "test".into(),
                digest: source_digest(&document),
                document,
            },
        )
        .unwrap();
        assert!(data.write().is_ok());
        let stored = data.legacy.as_mut().unwrap();
        assert_eq!(stored.paged, Some(manifest));
        stored.complete = true;
        assert!(data.write().is_err());
        let stored = data.legacy.as_mut().unwrap();
        stored.complete = false;
        stored.restored_digest = Some([99; 32]);
        stored.restored_root = Some([17; 32]);
        assert!(data.write().is_err());
    }
}
