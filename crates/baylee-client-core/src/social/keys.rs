//! Whose devices this client encrypts to (§4.1, §4.4, §4.5): the key cache,
//! trusted on first use, with each contact's verified set of identity keys
//! and the differences a refetch finds.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use baylee_protocol::social::{AccountKeys, DeviceKey, valid_device_id};
use baylee_seal::b64::url;

/// One device of a contact, as fetched and checked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedDevice {
    /// Its id ([`baylee_seal::device_id`] of `ik`).
    pub id: String,
    /// Its identity key.
    pub ik: [u8; 32],
    /// Its key-agreement key.
    pub dk: [u8; 32],
    /// Its OS family (`macOS`, `Windows`).
    pub label: String,
    /// When it was added, Unix milliseconds.
    pub added_at: u64,
}

/// One account's devices, as last fetched.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactKeys {
    /// The handle the gateway named with them.
    pub handle: String,
    /// The devices, in the gateway's order.
    pub devices: Vec<CachedDevice>,
    /// When this client first saw keys for the account.
    pub first_seen: u64,
    /// The identity keys at the moment the player marked the account
    /// verified, sorted; the account is verified only while its current
    /// set equals this (§4.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_iks: Option<Vec<[u8; 32]>>,
    /// A known key set changed or its directory became ambiguous. Only explicit
    /// verification of the exact current set clears this persisted send barrier.
    /// Older states lack this history and require one confirmation on upgrade.
    #[serde(default = "upgrade_requires_confirmation")]
    pub requires_confirmation: bool,
    /// Last first-use or explicitly confirmed snapshot. An unconfirmed directory
    /// cannot replace these signing keys. Missing in older states means their
    /// already-stored devices, captured before the next directory replacement.
    #[serde(default)]
    pub pinned_devices: Option<Vec<CachedDevice>>,
}

const fn upgrade_requires_confirmation() -> bool {
    true
}

impl ContactKeys {
    /// The current identity keys, sorted.
    #[must_use]
    pub fn iks(&self) -> Vec<[u8; 32]> {
        let mut iks: Vec<[u8; 32]> = self.devices.iter().map(|d| d.ik).collect();
        iks.sort_unstable();
        iks
    }

    /// Whether the player verified exactly the devices there are now.
    #[must_use]
    pub fn verified(&self) -> bool {
        !self.requires_confirmation && self.verified_iks.as_ref().is_some_and(|v| *v == self.iks())
    }

    /// Identity and agreement keys in canonical order, binding a confirmation to
    /// the exact snapshot shown by the safety sheet, including signed DK rotation.
    #[must_use]
    pub fn public_keys(&self) -> Vec<([u8; 32], [u8; 32])> {
        let mut keys: Vec<_> = self.devices.iter().map(|d| (d.ik, d.dk)).collect();
        keys.sort_unstable();
        keys
    }

    /// The device named `id`.
    #[must_use]
    pub fn device(&self, id: &str) -> Option<&CachedDevice> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// Previously accepted sender identity, retained while a new directory awaits
    /// confirmation, including devices removed from that unconfirmed directory.
    #[must_use]
    pub fn pinned_device(&self, id: &str) -> Option<&CachedDevice> {
        self.pinned_devices
            .as_ref()
            .unwrap_or(&self.devices)
            .iter()
            .find(|d| d.id == id)
    }
}

/// What a refetch found changed for one account.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyDiff {
    /// The account's tag.
    pub tag: String,
    /// Devices it has now that it hadn't.
    pub added: Vec<CachedDevice>,
    /// Devices it had that it hasn't now.
    pub removed: Vec<CachedDevice>,
    /// Whether this was the first fetch (no diff is a change then).
    pub first: bool,
    /// Whether the account was verified before the change.
    pub was_verified: bool,
    /// How many devices the fetch dropped because their statement did not
    /// check (§4.1): a gateway that lies, or a damaged row.
    pub refused: usize,
}

impl KeyDiff {
    /// Whether anything changed that a conversation should be told of.
    #[must_use]
    pub fn changed(&self) -> bool {
        !self.first && (!self.added.is_empty() || !self.removed.is_empty() || self.refused != 0)
    }
}

/// The key cache: tag to devices, trusted on first use.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyCache {
    accounts: BTreeMap<String, ContactKeys>,
}

impl KeyCache {
    /// The account `tag`'s keys, if fetched.
    #[must_use]
    pub fn get(&self, tag: &str) -> Option<&ContactKeys> {
        self.accounts.get(tag)
    }

    /// Whether `tag`'s keys were fetched.
    #[must_use]
    pub fn has(&self, tag: &str) -> bool {
        self.accounts.contains_key(tag)
    }

    /// Every cached tag.
    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.accounts.keys().map(String::as_str)
    }

    /// Takes one account's fetched keys: each device's statement is checked
    /// against `tag` and its id against its key ([`baylee_seal::verify_device`],
    /// [`baylee_seal::device_id`]). A malformed, duplicate or oversized directory
    /// never partially replaces trusted keys. Returns what changed or was refused.
    pub fn take(&mut self, keys: &AccountKeys, now_ms: u64) -> KeyDiff {
        let mut refused = usize::from(keys.devices.len() > baylee_protocol::social::MAX_DEVICES);
        let candidates = if refused == 0 {
            keys.devices.as_slice()
        } else {
            &[]
        };
        let devices: Vec<CachedDevice> = candidates
            .iter()
            .filter_map(|d| {
                let checked = check(&keys.tag, d);
                if checked.is_none() {
                    refused += 1;
                }
                checked
            })
            .collect();
        let unique: BTreeSet<_> = devices.iter().map(|device| &device.id).collect();
        refused += devices.len() - unique.len();
        if refused != 0 {
            let contact = self.accounts.get_mut(&keys.tag);
            let first = contact.is_none();
            let was_verified = contact.as_ref().is_some_and(|contact| contact.verified());
            if let Some(contact) = contact {
                contact.requires_confirmation = true;
            }
            return KeyDiff {
                tag: keys.tag.clone(),
                first,
                was_verified,
                refused,
                ..KeyDiff::default()
            };
        }
        let entry = self.accounts.entry(keys.tag.clone());
        let first = matches!(entry, std::collections::btree_map::Entry::Vacant(_));
        let contact = entry.or_insert_with(|| ContactKeys {
            first_seen: now_ms,
            ..ContactKeys::default()
        });
        let was_verified = contact.verified();
        if first {
            contact.pinned_devices = Some(devices.clone());
        } else if contact.pinned_devices.is_none() {
            contact.pinned_devices = Some(contact.devices.clone());
        }
        let added: Vec<_> = devices
            .iter()
            .filter(|d| {
                contact
                    .device(&d.id)
                    .is_none_or(|old| old.ik != d.ik || old.dk != d.dk)
            })
            .cloned()
            .collect();
        let removed: Vec<_> = contact
            .devices
            .iter()
            .filter(|old| {
                !devices
                    .iter()
                    .any(|d| d.id == old.id && d.ik == old.ik && d.dk == old.dk)
            })
            .cloned()
            .collect();
        contact.requires_confirmation |= !first && (!added.is_empty() || !removed.is_empty());
        contact.handle = super::text::clean_name(&keys.handle);
        contact.devices = devices;
        KeyDiff {
            tag: keys.tag.clone(),
            added,
            removed,
            first,
            was_verified,
            refused,
        }
    }

    /// Verifies exactly the displayed public keys; stale, empty and unknown sets
    /// are refused. A later directory change cannot inherit this confirmation.
    pub fn verify(&mut self, tag: &str, expected: &[([u8; 32], [u8; 32])]) -> bool {
        match self.accounts.get_mut(tag) {
            Some(contact) if !expected.is_empty() && contact.public_keys() == expected => {
                contact.verified_iks = Some(contact.iks());
                contact.requires_confirmation = false;
                contact.pinned_devices = Some(contact.devices.clone());
                true
            }
            _ => false,
        }
    }

    /// Drops `tag`'s verification.
    pub fn unverify(&mut self, tag: &str) {
        if let Some(contact) = self.accounts.get_mut(tag) {
            contact.verified_iks = None;
        }
    }

    /// Preserve a known trust history while refusing an ambiguous directory response.
    pub fn reject(&mut self, tag: &str) {
        if let Some(contact) = self.accounts.get_mut(tag) {
            contact.requires_confirmation = true;
        }
    }

    /// Explicitly resets `tag`'s trust history. Only a deliberate user-requested
    /// reset may call this; fetch/refusal/retry paths must preserve existing pins.
    pub fn forget(&mut self, tag: &str) {
        self.accounts.remove(tag);
    }
}

/// One fetched device, checked: its keys decode, its id is its key's, and
/// its statement for `tag` verifies.
fn check(tag: &str, device: &DeviceKey) -> Option<CachedDevice> {
    let ik: [u8; 32] = url::decode_array(&device.ik).ok()?;
    let dk: [u8; 32] = url::decode_array(&device.dk).ok()?;
    let dsig: [u8; 64] = url::decode_array(&device.dsig).ok()?;
    if !valid_device_id(&device.id) || baylee_seal::device_id(&ik) != device.id {
        return None;
    }
    baylee_seal::verify_device(tag, &ik, &dk, &dsig).ok()?;
    Some(CachedDevice {
        id: device.id.clone(),
        ik,
        dk,
        label: super::text::clean_name(&device.label)
            .chars()
            .take(baylee_protocol::social::MAX_DEVICE_LABEL)
            .collect(),
        added_at: device.added_at,
    })
}

/// A device id as the devices list shows it: four groups of four.
#[must_use]
pub fn grouped_id(id: &str) -> String {
    id.as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}
