//! One-time logical-identity migration preserving authenticated peer trust and history.
use super::{
    ArchiveError, DeviceSecrets, Entropy, RestKind, STATE_FILE, Social, StoredState, Zeroizing,
    at_rest_open, at_rest_seal,
};

impl Social {
    /// Rebind an authenticated protocol state to a new account-derived identity.
    /// Preserve every peer pin, pending verification barrier, replay ID, conversation
    /// and preference. History remains byte-for-byte usable because the vault key
    /// must stay equal. Only own-device registration and own-device cache are reset.
    /// This pure operation changes no disk and emits no network effects.
    ///
    /// # Errors
    /// Refuses malformed/unauthenticated state, another vault key, an invalid own
    /// tag or a state belonging to a different old device. Never returns defaults.
    pub fn migrate_identity(
        previous: &DeviceSecrets,
        next: &DeviceSecrets,
        own_tag: &str,
        bytes: &[u8],
        rng: &mut dyn Entropy,
    ) -> Result<Vec<u8>, ArchiveError> {
        if previous.vault_key() != next.vault_key()
            || !baylee_protocol::social::valid_tag(own_tag)
            || !u32::from_str_radix(own_tag, 16).is_ok_and(|tag| (1..=0x7fff_ffff).contains(&tag))
        {
            return Err(ArchiveError);
        }
        Self::validate_stored_state(previous, bytes)?;
        let plain = at_rest_open(previous.vault_key(), RestKind::State, STATE_FILE, bytes)
            .map_err(|_| ArchiveError)?;
        let mut state: StoredState = serde_json::from_slice(&plain).map_err(|_| ArchiveError)?;
        state.device_id = next.device_id().into();
        state.registered = false;
        state.own_devices.clear();
        state.keys.forget(own_tag);
        let encoded = Zeroizing::new(serde_json::to_vec(&state).map_err(|_| ArchiveError)?);
        at_rest_seal(next.vault_key(), RestKind::State, STATE_FILE, &encoded, rng)
            .map_err(|_| ArchiveError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::social::keys::{CachedDevice, ContactKeys};
    use std::collections::BTreeMap;

    struct Draw(u8);
    impl Entropy for Draw {
        fn fill(&mut self, out: &mut [u8]) {
            out.fill(self.0);
            self.0 = self.0.wrapping_add(1);
        }
    }

    fn stored(key: &DeviceSecrets) -> StoredState {
        let peer = CachedDevice {
            id: "a".repeat(16),
            ik: [3; 32],
            dk: [4; 32],
            label: "Test".into(),
            added_at: 100,
        };
        let contact = ContactKeys {
            handle: "Bob#0009".into(),
            devices: vec![peer.clone()],
            first_seen: 99,
            verified_iks: Some(vec![peer.ik]),
            requires_confirmation: true,
            pinned_devices: Some(vec![peer]),
        };
        let cache =
            serde_json::json!({ "accounts": { "0009": contact, "0007": ContactKeys::default() } });
        StoredState {
            v: super::super::STATE_VERSION,
            device_id: key.device_id().into(),
            registered: true,
            keys: serde_json::from_value(cache).unwrap(),
            seen: BTreeMap::from([("019a3d70-0000-7000-8000-000000000001".into(), 100)]),
            own_devices: vec![key.device_id().into()],
            nicknames: BTreeMap::from([("0009".into(), "Trusted peer".into())]),
            convs: vec![super::super::StoredConv {
                id: "dm:0007:0009".into(),
                title: "Bob".into(),
            }],
            ..StoredState::default()
        }
    }

    #[test]
    fn identity_migration_preserves_peer_pins_barriers_replay_and_history_index() {
        let old = DeviceSecrets::from_bytes(&[41; 96]);
        let mut bytes = [42; 96];
        bytes[64..].copy_from_slice(old.vault_key());
        let next = DeviceSecrets::from_bytes(&bytes);
        let before = stored(&old);
        let sealed = at_rest_seal(
            old.vault_key(),
            RestKind::State,
            STATE_FILE,
            &serde_json::to_vec(&before).unwrap(),
            &mut Draw(5),
        )
        .unwrap();
        let migrated =
            Social::migrate_identity(&old, &next, "0007", &sealed, &mut Draw(9)).unwrap();
        Social::validate_stored_state(&next, &migrated).unwrap();
        let plain = at_rest_open(next.vault_key(), RestKind::State, STATE_FILE, &migrated).unwrap();
        let after: StoredState = serde_json::from_slice(&plain).unwrap();
        let mut expected = before;
        expected.device_id = next.device_id().into();
        expected.registered = false;
        expected.own_devices.clear();
        expected.keys.forget("0007");
        assert_eq!(after, expected);
        assert!(Social::validate_stored_state(&old, &migrated).is_err());
        // The source archive is still authenticated under its original identity.
        Social::validate_stored_state(&old, &sealed).unwrap();
    }

    #[test]
    fn damaged_state_and_a_changed_vault_never_become_an_empty_new_identity() {
        let old = DeviceSecrets::from_bytes(&[41; 96]);
        let different_vault = DeviceSecrets::from_bytes(&[42; 96]);
        let state = at_rest_seal(
            old.vault_key(),
            RestKind::State,
            STATE_FILE,
            &serde_json::to_vec(&stored(&old)).unwrap(),
            &mut Draw(5),
        )
        .unwrap();
        assert!(
            Social::migrate_identity(&old, &different_vault, "0007", &state, &mut Draw(9)).is_err()
        );
        for own in ["", "0000", "00007", "000A"] {
            assert!(Social::migrate_identity(&old, &old, own, &state, &mut Draw(9)).is_err());
        }
        let mut damaged = state.clone();
        damaged[12] ^= 1;
        assert!(Social::migrate_identity(&old, &old, "0007", &damaged, &mut Draw(9)).is_err());
    }
}
