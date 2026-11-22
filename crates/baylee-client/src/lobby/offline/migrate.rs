//! Strict, read-only migration of the device's own shelf. The offline player's
//! tolerant recovery path is deliberately independent of this operation.
use super::{BUILTIN, DECKS, StoredDeck};
use baylee_account::{
    Account, Error,
    decks::{DeckDraft, DeckSource, ImportedDeck, ImportedVersion},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, VecDeque},
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Write},
    path::Path,
};
use uuid::Uuid;

struct Snapshot {
    draft: DeckDraft,
    saved_at: i64,
    revision: u64,
    key: [u8; 32],
}

#[derive(Default)]
pub(in crate::lobby) struct Device {
    rows: Option<VecDeque<Snapshot>>,
    pub(in crate::lobby) scanned: usize,
    pub(in crate::lobby) imported: usize,
}

fn install_id(dir: &Path) -> Result<Uuid, Error> {
    let path = dir.join("offline-deck-source");
    match File::open(&path) {
        Ok(file) => return stored_id(file),
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(_) => return Err(Error::Unavailable),
    }
    // Publish only a complete marker, even when two clients import concurrently.
    let id = Uuid::now_v7();
    let temp = dir.join(format!(".offline-deck-source-{id}"));
    let mut create = OpenOptions::new();
    create.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        create.mode(0o600);
    }
    let mut file = create.open(&temp).map_err(|_| Error::Unavailable)?;
    let installed = file
        .write_all(id.as_bytes())
        .and_then(|()| file.sync_all())
        .and_then(|()| std::fs::hard_link(&temp, &path));
    drop(file);
    let _ = std::fs::remove_file(temp);
    match installed {
        Ok(()) => Ok(id),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            stored_id(File::open(path).map_err(|_| Error::Unavailable)?)
        }
        Err(_) => Err(Error::Unavailable),
    }
}

fn stored_id(file: File) -> Result<Uuid, Error> {
    let mut bytes = Vec::new();
    file.take(17)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    Uuid::from_slice(&bytes)
        .ok()
        .filter(|id| id.get_version_num() == 7)
        .ok_or(Error::Rejected)
}

const LIMIT: u64 = 64 * 1024 * 1024;

fn read() -> Result<VecDeque<Snapshot>, Error> {
    let path = crate::settings::store::path(DECKS).ok_or(Error::Unavailable)?;
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(VecDeque::new()),
        Err(_) => return Err(Error::Unavailable),
    };
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes.len() as u64 > LIMIT {
        return Err(Error::Rejected);
    }
    let device = install_id(path.parent().ok_or(Error::Unavailable)?)?;
    parse(&bytes, device)
}

fn parse(bytes: &[u8], device: Uuid) -> Result<VecDeque<Snapshot>, Error> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Shelf {
        decks: Vec<serde_json::Value>,
    }
    let shelf: Shelf = serde_json::from_slice(bytes).map_err(|_| Error::Rejected)?;
    let mut ids = BTreeSet::new();
    let mut queue = VecDeque::new();
    for mut value in shelf.decks {
        if value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|id| id.starts_with(BUILTIN))
        {
            continue;
        }
        let fields = value.as_object_mut().ok_or(Error::Rejected)?;
        let legacy = fields
            .remove("commander")
            .map(serde_json::from_value::<Option<String>>)
            .transpose()
            .map_err(|_| Error::Rejected)?
            .flatten()
            .filter(|name| !name.is_empty());
        if fields.keys().any(|key| {
            !matches!(
                key.as_str(),
                "id" | "account_id"
                    | "name"
                    | "cards"
                    | "sideboard"
                    | "commanders"
                    | "format"
                    | "sleeve"
                    | "playmat"
                    | "updated_at"
            )
        }) {
            return Err(Error::Rejected);
        }
        let mut row: StoredDeck = serde_json::from_value(value).map_err(|_| Error::Rejected)?;
        if !matches!(row.account_id.as_str(), "" | "offline")
            || !ids.insert(row.id.clone())
            || row.id.is_empty()
        {
            return Err(Error::Rejected);
        }
        if let Some(name) = legacy {
            if row.commanders.is_empty() {
                row.commanders.push(name);
            } else if !row.commanders.contains(&name) {
                return Err(Error::Rejected);
            }
        }
        let updated = row.effective_updated_at();
        let revision = updated.max(1);
        let saved_at = i64::try_from(updated).map_err(|_| Error::Rejected)?;
        let format = row.format();
        let draft = DeckDraft {
            name: row.name,
            format,
            cards: row.cards,
            sideboard: row.sideboard,
            commanders: row.commanders,
            sleeve: row.sleeve,
            playmat: row.playmat,
            source: Some(DeckSource {
                gateway: None,
                account: Some(format!("local:{device}")),
                deck: row.id,
                revision,
            }),
            ..Default::default()
        };
        draft.validate()?;
        let mut digest = Sha256::new();
        digest.update(b"baylee/offline-deck-import/v1\0");
        digest.update(serde_json::to_vec(&draft).map_err(|_| Error::Rejected)?);
        queue.push_back(Snapshot {
            draft,
            saved_at,
            revision,
            key: digest.finalize().into(),
        });
    }
    Ok(queue)
}

impl Device {
    /// Read the file on the worker, then publish at most one private deck per job.
    pub(in crate::lobby) fn step(&mut self, account: &mut Account) -> Result<bool, Error> {
        if self.rows.is_none() {
            self.rows = Some(read()?);
        }
        let rows = self.rows.as_mut().ok_or(Error::Unavailable)?;
        let Some(row) = rows.front() else {
            return Ok(true);
        };
        let version = ImportedVersion {
            draft: row.draft.clone(),
            saved_at: row.saved_at,
            revision: row.revision,
        };
        self.imported += account.import_decks_count(
            row.key,
            vec![ImportedDeck {
                versions: vec![version],
            }],
        )?;
        rows.pop_front();
        self.scanned += 1;
        Ok(rows.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DEVICE: Uuid = Uuid::from_bytes([9; 16]);

    #[test]
    fn migration_is_idempotent_preserves_old_metadata_and_excludes_house_decks() {
        let bytes = br#"{"decks":[
            {"id":"builtin:plain","name":"House","cards":["1 Island"]},
            {"id":"old-save","name":"Harbour","cards":["1 Island # plan"],
             "commander":"Sol'kanar the Swamp King","updated_at":42,"sleeve":"device-art"}
        ]}"#;
        let mut decks = parse(bytes, DEVICE).unwrap();
        assert_eq!(decks.len(), 1);
        let one = decks.pop_front().unwrap();
        assert_eq!(one.draft.cards, ["1 Island # plan"]);
        assert_eq!(one.draft.commanders, ["Sol'kanar the Swamp King"]);
        assert_eq!(one.draft.sleeve.as_deref(), Some("device-art"));
        assert!(one.draft.cosmetic_origin.is_none());
        assert_eq!(one.saved_at, 42);
        assert_eq!(one.key, parse(bytes, DEVICE).unwrap()[0].key);
        assert_ne!(
            one.key,
            parse(bytes, Uuid::from_bytes([8; 16])).unwrap()[0].key
        );
    }

    #[test]
    fn concurrent_imports_share_the_same_complete_device_marker() {
        let dir = std::env::temp_dir().join(format!("baylee-source-{}", Uuid::now_v7()));
        std::fs::create_dir(&dir).unwrap();
        std::thread::scope(|scope| {
            let other = scope.spawn(|| install_id(&dir).unwrap());
            let id = install_id(&dir).unwrap();
            assert_eq!(other.join().unwrap(), id);
            assert_eq!(install_id(&dir).unwrap(), id);
        });
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn damaged_and_unknown_owned_rows_never_silently_disappear() {
        for bytes in [
            r#"{"decks":[{"id":"a","cards":1}]}"#,
            r#"{"decks":[{"id":"a","name":"x","cards":[],"future_private_state":["x"]}]}"#,
            r#"{"decks":[{"id":"a","name":"x","cards":[]},{"id":"a","name":"y","cards":[]}]}"#,
            r#"{"decks":[{"id":"a","name":"x","cards":[],"account_id":"other"}]}"#,
        ] {
            assert!(parse(bytes.as_bytes(), DEVICE).is_err());
        }
    }
}
