//! Chat's end-to-end encryption (the beta.8 social design, §4).
//!
//! Every construction here works on bytes: a message's header and body are
//! opaque to it, so the header the sender signed is exactly the header a
//! recipient checks, with no canonical form to agree on. What the bytes
//! *mean* (who a conversation's members are, whether an id was seen before,
//! how long a body may be) is the caller's: the client core opens a message
//! with its key cache and its seen ids around this crate, and the gateway
//! checks a device statement and a reported message with it.
//!
//! The crate does no I/O and draws no randomness of its own. A caller hands
//! in an [`Entropy`]: the client fills it from the operating system, tests
//! from [`testkit::Seeded`], which is what makes every output pinnable by a
//! known-answer test. It builds for `wasm32-unknown-unknown` with no backend
//! for `getrandom`, because nothing in its graph asks for one.
//!
//! | What | Where |
//! | --- | --- |
//! | A device's keys, its id and its signed statement (§4.1) | [`DeviceSecrets`], [`device_id`], [`verify_device`] |
//! | Sealing a message for devices, opening it on one (§4.2) | [`seal`], [`open`], [`verify_message`] |
//! | Checking a reported message (§4.7) | [`verify_report`] |
//! | Records at rest under the vault key (§4.8), history file names | [`at_rest_seal`], [`at_rest_open`], [`history_name`] |
//! | Safety numbers (§4.4) | [`fingerprint`], [`safety_number`] |
//! | Base64 on the wire and in reports | [`b64`] |
//!
//! Secrets live in [`zeroize::Zeroizing`] buffers and are wiped when dropped;
//! no `Debug` output of this crate prints one.

mod device;
mod message;
mod rest;
mod safety;

pub mod b64;
pub mod testkit;

pub use device::{DevicePublic, DeviceSecrets, device_id, device_statement, verify_device};
pub use message::{
    Envelope, Opened, Recipient, Sealed, Wrap, open, seal, verify_message, verify_report,
};
pub use rest::{RestKind, at_rest_open, at_rest_seal, history_name};
pub use safety::{fingerprint, safety_number, tag_order};

/// `"baylee/chat/device-id/v1\0"`: the domain of a device id's hash (§4.1).
pub const C_DEVID: &[u8] = b"baylee/chat/device-id/v1\0";
/// `"baylee/chat/device/v1\0"`: the domain of a device statement (§4.1).
pub const C_DEVICE: &[u8] = b"baylee/chat/device/v1\0";
/// `"baylee/chat/wrap/v1\0"`: the domain of a key wrap's HKDF info (§4.2).
pub const C_WRAP: &[u8] = b"baylee/chat/wrap/v1\0";
/// `"baylee/chat/msg/v1\0"`: the domain of a message signature (§4.2).
pub const C_MSG: &[u8] = b"baylee/chat/msg/v1\0";
/// `"baylee/chat/rest/v1\0"`: the domain of a record at rest (§4.8).
pub const C_REST: &[u8] = b"baylee/chat/rest/v1\0";
/// `"baylee/chat/commit/v1\0"`: the domain of a ciphertext's commitment to
/// its message key (an addition to §4.2, from the crypto review).
pub const C_COMMIT: &[u8] = b"baylee/chat/commit/v1\0";
/// `"baylee/chat/hedge/v1\0"`: the domain that hedges a message's drawn
/// randomness with its sender's key and contents.
pub const C_HEDGE: &[u8] = b"baylee/chat/hedge/v1\0";
/// `"baylee/chat/vault/v1\0"`: the domain the vault key's two subkeys are
/// derived under, one for records at rest and one for history file names.
pub const C_VAULT: &[u8] = b"baylee/chat/vault/v1\0";

/// A 32-byte key or seed.
pub const KEY_LEN: usize = 32;
/// An XChaCha20-Poly1305 nonce.
pub const NONCE_LEN: usize = 24;
/// An Ed25519 signature.
pub const SIG_LEN: usize = 64;
/// A Poly1305 tag, the bytes AEAD adds to what it seals.
pub const TAG_LEN: usize = 16;
/// A wrapped message key: 32 bytes sealed, plus its tag.
pub const WRAP_LEN: usize = KEY_LEN + TAG_LEN;
/// A device's stored secrets: `ik_seed ‖ dk_seed ‖ vault_key` (§11.1).
pub const SECRETS_LEN: usize = 3 * KEY_LEN;
/// The longest tag a device statement can name: its length is one byte.
pub const MAX_TAG_LEN: usize = 255;

/// Where randomness comes from. The crate never draws its own: the client
/// fills `out` from the operating system (`getrandom`), a test from
/// [`testkit::Seeded`].
///
/// Each function that takes one documents the order it draws in, because a
/// known-answer test depends on it.
pub trait Entropy {
    /// Fills `out` with fresh random bytes.
    fn fill(&mut self, out: &mut [u8]);
}

/// Why a check or an opening failed. The arms are distinct so that a caller
/// (and a test) can tell a forged message from one not meant for this
/// device; none carries a byte of the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A field had the wrong length: a nonce, key, signature or wrap, or a
    /// tag longer than [`MAX_TAG_LEN`] bytes.
    #[error("a field has the wrong length")]
    Length,
    /// A public key is not a valid curve point.
    #[error("a public key is not a valid point")]
    Key,
    /// An Ed25519 public key of small order, which every signature would
    /// verify under.
    #[error("a public key is of small order")]
    WeakKey,
    /// An X25519 exchange gave the all-zero secret: the other key is of
    /// small order, and a wrap under it would be readable by anyone.
    #[error("a key exchange was not contributory")]
    NonContributory,
    /// The signature does not verify: the header, nonce, ephemeral key,
    /// ciphertext or signature was changed, or another device signed it.
    #[error("the signature does not verify")]
    Signature,
    /// The wrapped message key does not open on this device: it was meant
    /// for another, or it was changed.
    #[error("the wrapped key does not open on this device")]
    Wrap,
    /// The body does not decrypt under the message key.
    #[error("the body does not decrypt")]
    Body,
    /// A reported body differs from what the message decrypts to.
    #[error("the reported body differs from the message")]
    Mismatch,
    /// A record at rest does not open: another key, another file, or
    /// damaged bytes.
    #[error("a stored record does not open")]
    Record,
    /// Base64 text that the encoding would not have written.
    #[error("the text is not valid base64")]
    Encoding,
}
