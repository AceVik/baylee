//! An unsaved Central builder visit is recoverable after signing in to the
//! same account. The original offline draft file belongs to the device.
use super::{Draft, Drafts};
use baylee_account::{Account, Error};
use baylee_seal::{RestKind, at_rest_open, at_rest_seal};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::{io::Read, sync::Arc};
use zeroize::Zeroizing;

const MAX_FILE: u64 = 32 * 1024 * 1024;

pub(super) fn fingerprint(draft: &Draft) -> [u8; 32] {
    let bytes = Zeroizing::new(
        serde_json::to_vec(draft).expect("the deck draft contains only strings and arrays"),
    );
    Sha256::digest(bytes.as_slice()).into()
}

#[derive(Clone)]
pub(crate) struct Private {
    key: Arc<Zeroizing<[u8; 32]>>,
    file: Arc<str>,
}

/// Stored only inside the account's authenticated local recovery envelope.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ReservedCreate {
    pub(crate) id: uuid::Uuid,
    pub(crate) first: baylee_account::decks::DeckDraft,
}

impl Private {
    pub(crate) fn for_account(account: &Account) -> Result<Self, Error> {
        let scope = Sha256::digest(b"baylee/local-builder-drafts/v1").into();
        let key = account.local_vault_key(&scope)?;
        Ok(Self::new(key))
    }

    pub(crate) fn new(key: Zeroizing<[u8; 32]>) -> Self {
        let mut name = Sha256::new();
        name.update(b"baylee/local-builder-drafts/filename/v1");
        name.update(key.as_slice());
        let digest = name.finalize();
        let mut file = String::from("builder-drafts-");
        for byte in digest.as_slice() {
            let _ = write!(file, "{byte:02x}");
        }
        file.push_str(".sealed");
        Self {
            key: Arc::new(key),
            file: file.into(),
        }
    }

    pub(super) fn read(&self) -> Result<Drafts, ()> {
        let Some(path) = crate::settings::store::path(&self.file) else {
            return Ok(Drafts::default());
        };
        let reader = match std::fs::File::open(path) {
            Ok(reader) => reader,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Drafts::default());
            }
            Err(_) => return Err(()),
        };
        let mut bytes = Vec::new();
        reader
            .take(MAX_FILE + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ())?;
        if bytes.len() as u64 > MAX_FILE {
            return Err(());
        }
        self.open(&bytes)
    }

    fn open(&self, bytes: &[u8]) -> Result<Drafts, ()> {
        let plain =
            at_rest_open(&self.key, RestKind::BuilderDraft, &self.file, bytes).map_err(|_| ())?;
        serde_json::from_slice(&plain).map_err(|_| ())
    }

    fn seal(&self, decks: &Drafts) -> Option<Vec<u8>> {
        let plain = Zeroizing::new(serde_json::to_vec(decks).ok()?);
        let bytes = at_rest_seal(
            &self.key,
            RestKind::BuilderDraft,
            &self.file,
            &plain,
            &mut crate::social::OsEntropy,
        )
        .ok()?;
        (bytes.len() as u64 <= MAX_FILE).then_some(bytes)
    }

    pub(super) fn write(&self, decks: &Drafts) {
        if decks.decks.is_empty() && decks.creates.is_empty() {
            crate::settings::store::remove_named(&self.file);
        } else {
            let _ = self.write_checked(decks);
        }
    }

    pub(super) fn write_checked(&self, decks: &Drafts) -> Result<(), Error> {
        let path = crate::settings::store::path(&self.file).ok_or(Error::Unavailable)?;
        let bytes = self.seal(decks).ok_or(Error::Unavailable)?;
        crate::settings::store::write_at_checked(&path, bytes).map_err(|_| Error::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lost_cloud_create_reply_reopens_the_same_anchor_but_a_clone_gets_a_new_intent() {
        let vault = Private::new(Zeroizing::new([9; 32]));
        let id = uuid::Uuid::now_v7();
        let draft = Draft {
            name: "First build".into(),
            main: vec!["1 Island".into()],
            ..Draft::default()
        };
        let initial = baylee_account::decks::DeckDraft {
            name: draft.name.clone(),
            cards: draft.main.clone(),
            format: "freeform".into(),
            ..Default::default()
        };
        let mut before = Drafts {
            epoch: 1,
            read: true,
            ..Drafts::default()
        };
        before.keep_visit(7, &draft);
        before.record_create(
            7,
            ReservedCreate {
                id,
                first: initial.clone(),
            },
        );
        // The cloud committed this ID. The application died before the response.
        let envelope = vault.seal(&before).unwrap();
        let mut after = Drafts {
            epoch: 2,
            read: true,
            ..vault.open(&envelope).unwrap()
        };
        let (recovered, fork) = after.recover(&Draft::default()).unwrap();
        assert!(!fork && recovered == draft);
        after.resume_create(12);
        let retry = after.create_for(12).unwrap();
        assert_eq!(retry.id, id);
        assert!(retry.first == initial);
        // A populated import and another actual new visit may have the same contents.
        assert!(after.recover(&draft).is_none());
        after.keep_visit(13, &draft);
        assert!(after.create_for(13).is_none());
        assert_eq!(after.create_for(12).unwrap().id, id);
        assert!(
            Private::new(Zeroizing::new([8; 32]))
                .open(&envelope)
                .is_err()
        );
    }

    #[test]
    fn a_new_deck_draft_is_recoverable_only_by_its_original_account() {
        let original = Private::new(Zeroizing::new([1; 32]));
        let other = Private::new(Zeroizing::new([2; 32]));
        let draft = Draft {
            name: "private name".into(),
            ..Draft::default()
        };
        let drafts = Drafts {
            decks: std::collections::BTreeMap::from([("new".into(), draft)]),
            ..Drafts::default()
        };
        let bytes = original.seal(&drafts).unwrap();
        assert!(!bytes.windows(12).any(|part| part == b"private name"));
        assert_ne!(original.file, other.file);
        assert!(other.open(&bytes).is_err());
        assert_eq!(
            Private::new(Zeroizing::new([1; 32]))
                .open(&bytes)
                .unwrap()
                .decks,
            drafts.decks
        );
    }

    #[test]
    fn an_older_private_edit_forks_once_after_a_newer_device_save() {
        let remote = Draft {
            editing: Some("original".into()),
            name: "newer remote".into(),
            ..Draft::default()
        };
        let old = Draft {
            name: "my unsaved edit".into(),
            ..remote.clone()
        };
        let mut drafts = Drafts {
            epoch: 1,
            read: true,
            ..Drafts::default()
        };
        drafts.decks.insert("original".into(), old.clone());
        drafts.bases.insert("original".into(), [8; 32]);
        let (mut fork, independent) = drafts.recover(&remote).unwrap();
        assert!(independent && fork.editing.is_none());
        assert_eq!(fork.name, old.name);
        drafts
            .forks
            .insert(9, ("original".into(), fingerprint(&old)));
        fork.name = "recovered copy".into();
        drafts.committed(8, &fork, Some("other"));
        assert_eq!(drafts.of(Some("original")), Some(&old));
        drafts.committed(9, &fork, Some("mine"));
        assert!(drafts.of(Some("original")).is_none());
        assert!(drafts.recover(&remote).is_none());
    }
}
