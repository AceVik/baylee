//! Records at rest (§4.8): the chat's state file and each history entry,
//! sealed under the device's vault key.
//!
//! A record is `n(24) ‖ XChaCha20-Poly1305(vault_key).encrypt(n, json,
//! aad = C_REST ‖ kind ‖ name)`. The kind and the file's own name are the
//! associated data, so a record moved into another file, or from history
//! into state, fails to open rather than being read there.

use chacha20poly1305::XNonce;
use chacha20poly1305::aead::{Aead, Payload};
use zeroize::Zeroizing;

use crate::message::aead;
use crate::{C_REST, Entropy, Error, KEY_LEN, NONCE_LEN, TAG_LEN};

/// Which kind of file a record lives in. A closed set, so that `kind ‖ name`
/// can't be read two ways: neither kind's word is a prefix of the other's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestKind {
    /// The state file (`"state"`).
    State,
    /// A conversation's history file (`"history"`).
    History,
}

impl RestKind {
    fn word(self) -> &'static [u8] {
        match self {
            Self::State => b"state",
            Self::History => b"history",
        }
    }
}

/// Seals `plain` as a record of `kind` in the file `name`. Draws the
/// record's 24-byte nonce from `rng`, a new one every record.
///
/// # Errors
///
/// [`Error::Length`] only for input past what the cipher can seal (256 GiB).
pub fn at_rest_seal(
    vault_key: &[u8; KEY_LEN],
    kind: RestKind,
    name: &str,
    plain: &[u8],
    rng: &mut dyn Entropy,
) -> Result<Vec<u8>, Error> {
    let mut n = [0u8; NONCE_LEN];
    rng.fill(&mut n);
    let ct = aead(vault_key)
        .encrypt(
            &XNonce::from(n),
            Payload {
                msg: plain,
                aad: &aad(kind, name),
            },
        )
        .map_err(|_| Error::Length)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&n);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Opens a record of `kind` from the file `name`; wiped when dropped.
///
/// # Errors
///
/// [`Error::Record`] for a record sealed under another key, for another
/// kind or file, or damaged; [`Error::Length`] for one too short to hold a
/// nonce and a tag.
pub fn at_rest_open(
    vault_key: &[u8; KEY_LEN],
    kind: RestKind,
    name: &str,
    record: &[u8],
) -> Result<Zeroizing<Vec<u8>>, Error> {
    if record.len() < NONCE_LEN + TAG_LEN {
        return Err(Error::Length);
    }
    let (n, ct) = record.split_at(NONCE_LEN);
    let n: [u8; NONCE_LEN] = n.try_into().map_err(|_| Error::Length)?;
    aead(vault_key)
        .decrypt(
            &XNonce::from(n),
            Payload {
                msg: ct,
                aad: &aad(kind, name),
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::Record)
}

fn aad(kind: RestKind, name: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(C_REST.len() + 7 + name.len());
    out.extend_from_slice(C_REST);
    out.extend_from_slice(kind.word());
    out.extend_from_slice(name.as_bytes());
    out
}
