//! A device's keys (§4.1): an Ed25519 identity key that signs, an X25519
//! static key that receives wraps, and the vault key its records at rest are
//! sealed under.

use std::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

use crate::{C_DEVICE, C_DEVID, Entropy, Error, KEY_LEN, MAX_TAG_LEN, SECRETS_LEN, SIG_LEN};

/// One device's secrets: the identity key's seed, the key-agreement secret
/// and the vault key, each 32 bytes, wiped when dropped (`ed25519-dalek`'s,
/// `x25519-dalek`'s and `zeroize`'s own wiping). The public halves and the
/// id are worked out once, when the secrets are made. `Debug` prints the
/// device id only.
pub struct DeviceSecrets {
    signing: SigningKey,
    agreement: StaticSecret,
    vault_key: Zeroizing<[u8; KEY_LEN]>,
    ik: [u8; KEY_LEN],
    dk: [u8; KEY_LEN],
    id: String,
}

/// What a device publishes: its id, both public keys and the signature that
/// binds them to its account's tag. This is what `POST /social/devices`
/// carries and what `GET /social/keys` hands back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevicePublic {
    /// `lower_hex(SHA-256(C_DEVID ‖ ik)[0..8])`, 16 characters.
    pub id: String,
    /// The Ed25519 identity key.
    pub ik: [u8; KEY_LEN],
    /// The X25519 key wraps are made for.
    pub dk: [u8; KEY_LEN],
    /// Ed25519 over [`device_statement`] by the identity key.
    pub dsig: [u8; SIG_LEN],
}

impl DevicePublic {
    /// Checks everything a fetched device claims: that `id` is its identity
    /// key's ([`device_id`]) and that [`verify_device`] holds for `tag`.
    ///
    /// # Errors
    ///
    /// [`Error::Key`] for an id that is not the key's, else as
    /// [`verify_device`].
    pub fn verify(&self, tag: &str) -> Result<(), Error> {
        if device_id(&self.ik) != self.id {
            return Err(Error::Key);
        }
        verify_device(tag, &self.ik, &self.dk, &self.dsig)
    }
}

impl DeviceSecrets {
    /// A new device's secrets. Draws 96 bytes from `rng`, in this order:
    /// `ik_seed`, `dk_seed`, `vault_key`.
    pub fn generate(rng: &mut dyn Entropy) -> Self {
        let mut bytes = Zeroizing::new([0u8; SECRETS_LEN]);
        rng.fill(&mut bytes[..]);
        Self::from_bytes(&bytes)
    }

    /// The secrets from their stored form, `ik_seed ‖ dk_seed ‖ vault_key`.
    #[must_use]
    pub fn from_bytes(bytes: &[u8; SECRETS_LEN]) -> Self {
        let part = |i: usize| {
            let mut out = Zeroizing::new([0u8; KEY_LEN]);
            out.copy_from_slice(&bytes[i * KEY_LEN..(i + 1) * KEY_LEN]);
            out
        };
        let signing = SigningKey::from_bytes(&part(0));
        let agreement = StaticSecret::from(*part(1));
        let ik = signing.verifying_key().to_bytes();
        let dk = PublicKey::from(&agreement).to_bytes();
        Self {
            signing,
            agreement,
            vault_key: part(2),
            ik,
            dk,
            id: device_id(&ik),
        }
    }

    /// The secrets in their stored form, `ik_seed ‖ dk_seed ‖ vault_key`
    /// (what goes into the credential store, §11.1), wiped when dropped.
    #[must_use]
    pub fn to_bytes(&self) -> Zeroizing<[u8; SECRETS_LEN]> {
        let mut out = Zeroizing::new([0u8; SECRETS_LEN]);
        out[..KEY_LEN].copy_from_slice(self.signing.as_bytes());
        out[KEY_LEN..2 * KEY_LEN].copy_from_slice(self.agreement.as_bytes());
        out[2 * KEY_LEN..].copy_from_slice(&self.vault_key[..]);
        out
    }

    /// The Ed25519 identity key.
    #[must_use]
    pub fn ik(&self) -> [u8; KEY_LEN] {
        self.ik
    }

    /// The X25519 key wraps for this device are made for.
    #[must_use]
    pub fn dk(&self) -> [u8; KEY_LEN] {
        self.dk
    }

    /// This device's id, [`device_id`] of its identity key.
    #[must_use]
    pub fn device_id(&self) -> &str {
        &self.id
    }

    /// The key records at rest are sealed under (§4.8), and that keys the
    /// names of history files (§10.2).
    #[must_use]
    pub fn vault_key(&self) -> &[u8; KEY_LEN] {
        &self.vault_key
    }

    /// What this device publishes for the account `tag`: its id, both keys
    /// and the statement's signature.
    ///
    /// # Errors
    ///
    /// [`Error::Length`] when `tag` is longer than [`MAX_TAG_LEN`] bytes.
    pub fn public(&self, tag: &str) -> Result<DevicePublic, Error> {
        let statement = device_statement(tag, &self.ik, &self.dk)?;
        Ok(DevicePublic {
            id: self.id.clone(),
            ik: self.ik,
            dk: self.dk,
            dsig: self.sign(&statement),
        })
    }

    /// Ed25519 by the identity key over `message`.
    pub(crate) fn sign(&self, message: &[u8]) -> [u8; SIG_LEN] {
        self.signing.sign(message).to_bytes()
    }

    /// The X25519 secret.
    pub(crate) fn agreement(&self) -> &StaticSecret {
        &self.agreement
    }

    /// The identity key's seed, for hedging a message's randomness.
    pub(crate) fn ik_seed(&self) -> &[u8; KEY_LEN] {
        self.signing.as_bytes()
    }
}

impl fmt::Debug for DeviceSecrets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceSecrets")
            .field("device_id", &self.id)
            .finish_non_exhaustive()
    }
}

/// A device's id: `lower_hex(SHA-256(C_DEVID ‖ ik)[0..8])`, 16 characters.
/// It is the fingerprint the devices list shows.
#[must_use]
pub fn device_id(ik: &[u8; KEY_LEN]) -> String {
    let hash = Sha256::new()
        .chain_update(C_DEVID)
        .chain_update(ik)
        .finalize();
    hash[..8]
        .iter()
        .fold(String::with_capacity(16), |mut out, byte| {
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0x0f)]));
            out
        })
}

const HEX: &[u8; 16] = b"0123456789abcdef";

/// The bytes a device statement signs:
/// `C_DEVICE ‖ u8(len(tag)) ‖ tag ‖ ik ‖ dk`. It binds the X25519 key to the
/// account's tag, so a gateway can't move a device to another account.
///
/// # Errors
///
/// [`Error::Length`] when `tag` is longer than [`MAX_TAG_LEN`] bytes.
pub fn device_statement(
    tag: &str,
    ik: &[u8; KEY_LEN],
    dk: &[u8; KEY_LEN],
) -> Result<Vec<u8>, Error> {
    let len = u8::try_from(tag.len()).map_err(|_| Error::Length)?;
    debug_assert!(usize::from(len) <= MAX_TAG_LEN);
    let mut out = Vec::with_capacity(C_DEVICE.len() + 1 + tag.len() + 2 * KEY_LEN);
    out.extend_from_slice(C_DEVICE);
    out.push(len);
    out.extend_from_slice(tag.as_bytes());
    out.extend_from_slice(ik);
    out.extend_from_slice(dk);
    Ok(out)
}

/// Checks that a device's statement for `tag` was signed by its identity
/// key, and that both keys are fit for use. The gateway runs this when a
/// device registers, and every client when it fetches the keys.
///
/// # Errors
///
/// - [`Error::Length`]: `tag` longer than [`MAX_TAG_LEN`] bytes.
/// - [`Error::Key`]: `ik` is not a valid Ed25519 point.
/// - [`Error::WeakKey`]: `ik` is of small order.
/// - [`Error::NonContributory`]: `dk` is of small order, so a wrap under it
///   would be readable by anyone.
/// - [`Error::Signature`]: the signature does not verify, for instance
///   because the statement was made for another tag.
pub fn verify_device(
    tag: &str,
    ik: &[u8; KEY_LEN],
    dk: &[u8; KEY_LEN],
    dsig: &[u8; SIG_LEN],
) -> Result<(), Error> {
    let statement = device_statement(tag, ik, dk)?;
    let key = identity_key(ik)?;
    if !dk_is_contributory(dk) {
        return Err(Error::NonContributory);
    }
    key.verify_strict(&statement, &Signature::from_bytes(dsig))
        .map_err(|_| Error::Signature)
}

/// A sender's identity key, refused when malformed or of small order.
pub(crate) fn identity_key(ik: &[u8; KEY_LEN]) -> Result<VerifyingKey, Error> {
    let key = VerifyingKey::from_bytes(ik).map_err(|_| Error::Key)?;
    // `from_bytes` also takes a handful of non-canonical encodings (ZIP-215);
    // a key is accepted only in the one spelling that compresses back to it.
    if key.to_edwards().compress().to_bytes() != *ik {
        return Err(Error::Key);
    }
    if key.is_weak() {
        return Err(Error::WeakKey);
    }
    Ok(key)
}

/// Whether an exchange with `dk` can give anything but the all-zero secret.
/// A clamped scalar is a multiple of the cofactor, so it sends every point
/// of small order to zero: one exchange with any fixed secret finds all of
/// them.
fn dk_is_contributory(dk: &[u8; KEY_LEN]) -> bool {
    StaticSecret::from([1u8; KEY_LEN])
        .diffie_hellman(&PublicKey::from(*dk))
        .was_contributory()
}
