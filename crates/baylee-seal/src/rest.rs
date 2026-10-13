//! Records at rest (§4.8): the chat's state file and each history entry,
//! sealed under the device's vault key.
//!
//! A record is `n(24) ‖ XChaCha20-Poly1305(rest_key).encrypt(n, json,
//! aad = C_REST ‖ kind ‖ name)`. The kind and the file's own name are the
//! associated data, so a record moved into another file, or from history
//! into state, fails to open rather than being read there.
//!
//! From the crypto review, two additions to §4.8:
//!
//! - The vault key is never used directly: `rest_key` and the key that
//!   names history files ([`history_name`]) are two HKDF-SHA256 subkeys of
//!   it (`C_VAULT ‖ "rest"`, `C_VAULT ‖ "name"`).
//! - The nonce is synthetic: HKDF-SHA256 with the 24 drawn bytes as salt,
//!   `rest_key` as input and `C_REST ‖ "nonce" ‖ aad ‖ SHA-256(plain)` as
//!   info. The vault key lives as long as the device, so a repeated draw
//!   must not repeat a nonce for two different records.
//!
//! What a record doesn't have is freshness: an older copy of a file, put
//! back, opens. The state's seen ids then forget messages seen since, which
//! the gateway's own refusal of a duplicate id still covers for the
//! messages it holds.

use chacha20poly1305::XNonce;
use chacha20poly1305::aead::{Aead, Payload};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::message::aead;
use crate::{C_REST, C_VAULT, Entropy, Error, KEY_LEN, NONCE_LEN, TAG_LEN};

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

/// Seals `plain` as a record of `kind` in the file `name`. Draws 24 bytes
/// from `rng` for the record's nonce, which is hedged with the record
/// itself (the module's notes).
///
/// # Errors
///
/// [`Error::Length`] only for input past what the cipher can seal (256 GiB).
///
/// # Panics
///
/// Never: the derivations ask HKDF for far less than its limit.
pub fn at_rest_seal(
    vault_key: &[u8; KEY_LEN],
    kind: RestKind,
    name: &str,
    plain: &[u8],
    rng: &mut dyn Entropy,
) -> Result<Vec<u8>, Error> {
    let key = subkey(vault_key, b"rest");
    let aad = aad(kind, name);
    let mut drawn = [0u8; NONCE_LEN];
    rng.fill(&mut drawn);
    let mut n = [0u8; NONCE_LEN];
    Hkdf::<Sha256>::new(Some(&drawn), &key[..])
        .expand_multi_info(&[C_REST, b"nonce", &aad, &Sha256::digest(plain)], &mut n)
        .expect("24 bytes is far under HKDF-SHA256's limit");
    let ct = aead(&key)
        .encrypt(
            &XNonce::from(n),
            Payload {
                msg: plain,
                aad: &aad,
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
    aead(&subkey(vault_key, b"rest"))
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

/// The name a conversation's history file goes by: 16 hex characters keyed
/// by the vault key, so a file's name says nothing of who it is with, and
/// two devices name the same conversation differently.
///
/// # Panics
///
/// Never: the derivations ask HKDF for far less than its limit.
#[must_use]
pub fn history_name(vault_key: &[u8; KEY_LEN], conv: &str) -> String {
    let mut out = [0u8; 8];
    Hkdf::<Sha256>::from_prk(&subkey(vault_key, b"name")[..])
        .expect("a 32-byte key is a valid PRK")
        .expand(conv.as_bytes(), &mut out)
        .expect("8 bytes is far under HKDF-SHA256's limit");
    out.iter().fold(String::with_capacity(16), |mut hex, byte| {
        hex.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        hex.push(char::from(b"0123456789abcdef"[usize::from(byte & 0x0f)]));
        hex
    })
}

/// One of the vault key's subkeys: HKDF-SHA256 over the vault key, expanded
/// over `C_VAULT ‖ label`.
fn subkey(vault_key: &[u8; KEY_LEN], label: &[u8]) -> Zeroizing<[u8; KEY_LEN]> {
    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    Hkdf::<Sha256>::new(None, vault_key)
        .expand_multi_info(&[C_VAULT, label], &mut out[..])
        .expect("32 bytes is far under HKDF-SHA256's limit");
    out
}
