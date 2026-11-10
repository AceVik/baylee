//! Preserve authenticated legacy state while adopting Central's account identity.
use super::{
    ChatDevice, ChatDisk, DeviceKey, Error, Handle, MemoryDisk, OsEntropy, Session, Social,
    Zeroizing, select_device, social,
};

pub(super) fn prepare(
    handle: &Handle,
    session: &Session,
    records: &mut Vec<ChatDevice>,
    derived: DeviceKey,
    load: impl Fn(&ChatDevice) -> Result<MemoryDisk, Error>,
) -> Result<(ChatDevice, MemoryDisk), Error> {
    let device = select(records, &derived, session)?;
    let known = records.iter().any(|record| record.device == device.device);
    let legacy = records
        .iter()
        .find(|record| !record.retired && record.device != device.device);
    let base = if device.history.is_some() || legacy.is_none() {
        load(&device)?
    } else {
        migrate(legacy.ok_or(Error::Authentication)?, &device, &load)?
    };
    // Publish no replacement or retirement. An interrupted initial upload leaves
    // history=None; the next attempt reconstructs the same state from the old slot.
    if !known {
        handle
            .archive_chat_device(device.clone())
            .map_err(super::account_err)?;
        records.push(device.clone());
    }
    Ok((device, base))
}

fn select(
    records: &[ChatDevice],
    derived: &DeviceKey,
    session: &Session,
) -> Result<ChatDevice, Error> {
    let keys = baylee_seal::DeviceSecrets::from_bytes(derived.bytes());
    if let Some(known) = records
        .iter()
        .find(|record| record.device == keys.device_id())
    {
        if known.retired || known.key.bytes()[..64] != derived.bytes()[..64] {
            return Err(Error::Authentication);
        }
        return Ok(known.clone());
    }
    let mut material = Zeroizing::new(*derived.bytes());
    if let Some(legacy) = select_device(records) {
        // The wire identity changes, but authenticated history filenames and
        // ciphertext keep their original key. Browser local trust uses its own
        // derived vault key; that key never participates in message encryption.
        material[64..].copy_from_slice(&legacy.key.bytes()[64..]);
    } else if !records.is_empty() {
        return Err(Error::Authentication);
    }
    Ok(ChatDevice {
        gateway: session.gateway.clone(),
        tag: session.tag.clone(),
        installation: uuid::Uuid::now_v7().to_string(),
        device: keys.device_id().into(),
        key: DeviceKey::new(material),
        retired: false,
        history: None,
    })
}

fn migrate(
    legacy: &ChatDevice,
    next: &ChatDevice,
    load: &impl Fn(&ChatDevice) -> Result<MemoryDisk, Error>,
) -> Result<MemoryDisk, Error> {
    let mut base = load(legacy)?;
    let previous = baylee_seal::DeviceSecrets::from_bytes(legacy.key.bytes());
    let current = baylee_seal::DeviceSecrets::from_bytes(next.key.bytes());
    if let Some(state) = base.read(social::disk::STATE_FILE) {
        let migrated =
            Social::migrate_identity(&previous, &current, &next.tag, &state, &mut OsEntropy)
                .map_err(|_| Error::Authentication)?;
        base.files.insert(social::disk::STATE_FILE.into(), migrated);
    } else if legacy.history.is_some() || !base.files.is_empty() {
        return Err(Error::Format);
    }
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session {
            gateway: "https://gateway.example".into(),
            tag: "0007".into(),
            handle: "Alice#0007".into(),
            token: "synthetic".into(),
            guest: false,
        }
    }
    fn key(value: u8) -> DeviceKey {
        DeviceKey::new(Zeroizing::new([value; 96]))
    }

    #[test]
    fn new_identity_keeps_the_old_archive_key_and_slot_without_replacement() {
        let me = session();
        let legacy = select(&[], &key(41), &me).unwrap();
        let selected = select(std::slice::from_ref(&legacy), &key(42), &me).unwrap();
        assert_ne!(selected.device, legacy.device);
        assert_ne!(selected.installation, legacy.installation);
        assert_eq!(selected.key.bytes()[..64], [42; 64]);
        assert_eq!(selected.key.bytes()[64..], legacy.key.bytes()[64..]);
        let records = vec![legacy.clone(), selected.clone()];
        let restored = select(&records, &key(42), &me).unwrap();
        assert_eq!(restored.installation, selected.installation);
        assert!(restored.key == selected.key);
        assert!(records[0].key == legacy.key);
    }

    #[test]
    fn retired_or_conflicting_identities_are_never_reactivated() {
        let me = session();
        let mut retired = select(&[], &key(41), &me).unwrap();
        retired.retired = true;
        assert!(select(std::slice::from_ref(&retired), &key(41), &me).is_err());
        assert!(select(&[retired], &key(42), &me).is_err());
        let mut corrupt = select(&[], &key(41), &me).unwrap();
        corrupt.key = key(42);
        assert!(select(&[corrupt], &key(41), &me).is_err());
    }
}
