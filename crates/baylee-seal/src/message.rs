//! Sealing a message for a set of devices and opening it on one (§4.2), and
//! checking a reported message (§4.7).
//!
//! One body is sealed under a random message key `mk`, which is wrapped for
//! each recipient device through one ephemeral X25519 key per message. The
//! sender's identity key signs `C_MSG ‖ u32be(len hdr) ‖ hdr ‖ n ‖ epk ‖ ct`.
//!
//! Two additions to §4.2, from the crypto review:
//!
//! - **`ct` commits to `mk`**: `ct = SHA-256(C_COMMIT ‖ mk) ‖ AEAD(mk, n,
//!   body, aad = hdr)`. Poly1305 is not key-committing, so without it a
//!   sender could hand two recipients two wraps of two keys under which one
//!   signed ciphertext reads as two different bodies; a report's "verified"
//!   would then say only what the reporter saw. With it, every recipient's
//!   key is the one the signature covers.
//! - **The randomness is hedged**: `mk ‖ n ‖ e_seed` is HKDF-SHA256 with the
//!   88 drawn bytes as salt, the identity key's seed as input, and
//!   `C_HEDGE ‖ SHA-256(hdr) ‖ SHA-256(body)` as info. A good `Entropy`
//!   makes them random; a broken one that repeats still gives each header
//!   (which names a new id) its own key and nonce.
//!
//! The wraps are not signed (§4.3): each device is handed only its own, and
//! a changed wrap can only make the message undecryptable, never change what
//! it says or where it is addressed (the header names the conversation, and
//! is both signed and the body's associated data).

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

use crate::device::identity_key;
use crate::{
    C_COMMIT, C_HEDGE, C_MSG, C_WRAP, DeviceSecrets, Entropy, Error, KEY_LEN, NONCE_LEN, SIG_LEN,
    TAG_LEN, WRAP_LEN,
};

/// A device a message is sealed for: its id and its X25519 key, both as
/// fetched from the key directory and checked with
/// [`crate::DevicePublic::verify`] (which ties the id to the key). An id
/// that isn't the device's own only makes its wrap unopenable.
#[derive(Clone, Copy, Debug)]
pub struct Recipient<'a> {
    /// The device's id ([`crate::device_id`]).
    pub device: &'a str,
    /// The device's X25519 key.
    pub dk: [u8; KEY_LEN],
}

/// One device's wrapped message key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wrap {
    /// The device it opens on.
    pub device: String,
    /// `mk`, sealed for that device: 48 bytes.
    pub k: [u8; WRAP_LEN],
}

/// A sealed message, everything but the header the caller already has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sealed {
    /// The body's nonce.
    pub n: [u8; NONCE_LEN],
    /// The body: `SHA-256(C_COMMIT ‖ mk)` (32 bytes), then the body sealed
    /// under `mk` with the header as associated data.
    pub ct: Vec<u8>,
    /// The message's ephemeral X25519 key.
    pub epk: [u8; KEY_LEN],
    /// The sender's signature over header, nonce, ephemeral key and
    /// ciphertext.
    pub sig: [u8; SIG_LEN],
    /// One wrap per recipient device, in the order they were given.
    pub wraps: Vec<Wrap>,
}

impl Sealed {
    /// The message as a recipient sees it, with `hdr` the header it was
    /// sealed with.
    #[must_use]
    pub fn envelope<'a>(&'a self, hdr: &'a [u8]) -> Envelope<'a> {
        Envelope {
            hdr,
            n: &self.n,
            ct: &self.ct,
            epk: &self.epk,
            sig: &self.sig,
        }
    }
}

/// A message as delivered, without its wrap: what [`open`],
/// [`verify_message`] and [`verify_report`] read. The fields are slices as
/// decoded from the wire; their lengths are checked here.
#[derive(Clone, Copy, Debug)]
pub struct Envelope<'a> {
    /// The sender's header bytes, exactly as signed.
    pub hdr: &'a [u8],
    /// The body's nonce, 24 bytes.
    pub n: &'a [u8],
    /// The sealed body.
    pub ct: &'a [u8],
    /// The ephemeral X25519 key, 32 bytes.
    pub epk: &'a [u8],
    /// The sender's signature, 64 bytes.
    pub sig: &'a [u8],
}

/// An opened message: its body, and the message key a report reveals
/// (§4.7). Both are wiped when dropped.
pub struct Opened {
    /// The body's plaintext bytes.
    pub body: Zeroizing<Vec<u8>>,
    /// The message key, which opens this one message and nothing else.
    pub mk: Zeroizing<[u8; KEY_LEN]>,
}

impl std::fmt::Debug for Opened {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Opened")
            .field("body_len", &self.body.len())
            .finish_non_exhaustive()
    }
}

/// Seals `body` under the header `hdr` for every device in `recipients`,
/// signed by `sender`.
///
/// Draws 88 bytes from `rng` at once, hedges them with the sender's key and
/// the message (see the module's notes) and splits the result into `mk`
/// (32), the body's nonce `n` (24) and the ephemeral secret `e_seed` (32).
/// Every other byte follows from them, so a seeded `rng` gives the same
/// output every time.
///
/// # Errors
///
/// - [`Error::NonContributory`]: a recipient's key is of small order, so a
///   wrap under it would be readable by anyone; nothing is sealed.
/// - [`Error::Length`]: a header of 4 GiB or more.
pub fn seal(
    sender: &DeviceSecrets,
    hdr: &[u8],
    body: &[u8],
    recipients: &[Recipient<'_>],
    rng: &mut dyn Entropy,
) -> Result<Sealed, Error> {
    let drawn = hedged(sender, hdr, body, rng);
    let mut mk = Zeroizing::new([0u8; KEY_LEN]);
    mk.copy_from_slice(&drawn[..KEY_LEN]);
    let mut n = [0u8; NONCE_LEN];
    n.copy_from_slice(&drawn[KEY_LEN..KEY_LEN + NONCE_LEN]);
    let mut e_seed = Zeroizing::new([0u8; KEY_LEN]);
    e_seed.copy_from_slice(&drawn[KEY_LEN + NONCE_LEN..]);

    let mut ct = commitment(&mk).to_vec();
    ct.extend(
        aead(&mk)
            .encrypt(
                &XNonce::from(n),
                Payload {
                    msg: body,
                    aad: hdr,
                },
            )
            .map_err(|_| Error::Length)?,
    );
    let e = StaticSecret::from(*e_seed);
    let epk = PublicKey::from(&e).to_bytes();
    let salt = Sha256::digest(hdr);

    let mut wraps = Vec::with_capacity(recipients.len());
    for recipient in recipients {
        let shared = e.diffie_hellman(&PublicKey::from(recipient.dk));
        if !shared.was_contributory() {
            return Err(Error::NonContributory);
        }
        let (kek, nonce) = wrap_key(
            &salt,
            shared.as_bytes(),
            &epk,
            &recipient.dk,
            recipient.device,
        );
        let sealed = aead(&kek)
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: &mk[..],
                    aad: recipient.device.as_bytes(),
                },
            )
            .map_err(|_| Error::Length)?;
        let k = sealed.try_into().map_err(|_| Error::Length)?;
        wraps.push(Wrap {
            device: recipient.device.to_owned(),
            k,
        });
    }

    let sig = sender.sign(&signed_bytes(hdr, &n, &epk, &ct)?);
    Ok(Sealed {
        n,
        ct,
        epk,
        sig,
        wraps,
    })
}

/// Opens a message on device `me`, with `sender_ik` the identity key of the
/// device the header names and `k` this device's own wrap. In this order it
/// checks the signature, unwraps the message key and decrypts the body.
///
/// What it doesn't check is the caller's (§4.2): that the header parses and
/// names this conversation and sender, that the sender's device is known,
/// that the id is new, and that the sender is a member.
///
/// # Errors
///
/// - [`Error::Length`]: a field of the wrong length, or a ciphertext
///   shorter than its tag.
/// - [`Error::Key`] / [`Error::WeakKey`]: `sender_ik` unfit to verify with.
/// - [`Error::Signature`]: the header, nonce, ephemeral key, ciphertext or
///   signature was changed, or another device signed it.
/// - [`Error::NonContributory`]: the ephemeral key is of small order.
/// - [`Error::Wrap`]: `k` was made for another device, or changed.
/// - [`Error::Body`]: the body does not decrypt under the unwrapped key.
pub fn open(
    me: &DeviceSecrets,
    sender_ik: &[u8; KEY_LEN],
    envelope: &Envelope<'_>,
    k: &[u8],
) -> Result<Opened, Error> {
    let fields = Fields::of(envelope)?;
    if k.len() != WRAP_LEN {
        return Err(Error::Length);
    }
    verify(sender_ik, envelope, &fields)?;

    let my_dk = me.dk();
    let shared = me.agreement().diffie_hellman(&PublicKey::from(fields.epk));
    if !shared.was_contributory() {
        return Err(Error::NonContributory);
    }
    let salt = Sha256::digest(envelope.hdr);
    let (kek, nonce) = wrap_key(
        &salt,
        shared.as_bytes(),
        &fields.epk,
        &my_dk,
        me.device_id(),
    );
    let unwrapped = Zeroizing::new(
        aead(&kek)
            .decrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: k,
                    aad: me.device_id().as_bytes(),
                },
            )
            .map_err(|_| Error::Wrap)?,
    );
    let mut mk = Zeroizing::new([0u8; KEY_LEN]);
    if unwrapped.len() != KEY_LEN {
        return Err(Error::Wrap);
    }
    mk.copy_from_slice(&unwrapped);
    // A key the signature's commitment doesn't name is not this message's.
    if !committed(&mk, envelope.ct) {
        return Err(Error::Wrap);
    }
    let body = decrypt_body(&mk, envelope, &fields).map_err(|_| Error::Body)?;
    Ok(Opened { body, mk })
}

/// Checks only the signature of a message by the device whose identity key
/// is `sender_ik`: a message that passes was sent, as it is, by that
/// device, whoever it was addressed to.
///
/// # Errors
///
/// [`Error::Length`], [`Error::Key`], [`Error::WeakKey`] and
/// [`Error::Signature`], as for [`open`].
pub fn verify_message(sender_ik: &[u8; KEY_LEN], envelope: &Envelope<'_>) -> Result<(), Error> {
    let fields = Fields::of(envelope)?;
    verify(sender_ik, envelope, &fields)
}

/// Checks a reported message (§4.7): the signature by `sender_ik`, then
/// that `mk` decrypts the ciphertext to exactly `body`. Only a report that
/// passes both is marked verified for moderation.
///
/// # Errors
///
/// - [`Error::Length`], [`Error::Key`], [`Error::WeakKey`],
///   [`Error::Signature`] as for [`open`].
/// - [`Error::Body`]: `mk` does not open the ciphertext.
/// - [`Error::Mismatch`]: it opens, to something other than `body` (the
///   comparison takes the same time wherever the bytes differ).
pub fn verify_report(
    sender_ik: &[u8; KEY_LEN],
    envelope: &Envelope<'_>,
    mk: &[u8; KEY_LEN],
    body: &[u8],
) -> Result<(), Error> {
    let fields = Fields::of(envelope)?;
    verify(sender_ik, envelope, &fields)?;
    if !committed(mk, envelope.ct) {
        return Err(Error::Body);
    }
    let plain = decrypt_body(mk, envelope, &fields).map_err(|_| Error::Body)?;
    if plain.len() == body.len() && bool::from(plain.as_slice().ct_eq(body)) {
        Ok(())
    } else {
        Err(Error::Mismatch)
    }
}

/// An envelope's fixed-length fields, checked.
struct Fields {
    n: [u8; NONCE_LEN],
    epk: [u8; KEY_LEN],
    sig: [u8; SIG_LEN],
}

impl Fields {
    fn of(envelope: &Envelope<'_>) -> Result<Self, Error> {
        if envelope.ct.len() < COMMIT_LEN + TAG_LEN {
            return Err(Error::Length);
        }
        Ok(Self {
            n: envelope.n.try_into().map_err(|_| Error::Length)?,
            epk: envelope.epk.try_into().map_err(|_| Error::Length)?,
            sig: envelope.sig.try_into().map_err(|_| Error::Length)?,
        })
    }
}

fn verify(
    sender_ik: &[u8; KEY_LEN],
    envelope: &Envelope<'_>,
    fields: &Fields,
) -> Result<(), Error> {
    let key = identity_key(sender_ik)?;
    let signed = signed_bytes(envelope.hdr, &fields.n, &fields.epk, envelope.ct)?;
    key.verify_strict(&signed, &ed25519_dalek::Signature::from_bytes(&fields.sig))
        .map_err(|_| Error::Signature)
}

/// `C_MSG ‖ u32be(len hdr) ‖ hdr ‖ n ‖ epk ‖ ct`. The header's length
/// prefix and the fixed lengths of `n` and `epk` leave only `ct` open-ended,
/// and it comes last, so no two messages sign the same bytes.
fn signed_bytes(
    hdr: &[u8],
    n: &[u8; NONCE_LEN],
    epk: &[u8; KEY_LEN],
    ct: &[u8],
) -> Result<Vec<u8>, Error> {
    let len = u32::try_from(hdr.len()).map_err(|_| Error::Length)?;
    let mut out = Vec::with_capacity(C_MSG.len() + 4 + hdr.len() + NONCE_LEN + KEY_LEN + ct.len());
    out.extend_from_slice(C_MSG);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(hdr);
    out.extend_from_slice(n);
    out.extend_from_slice(epk);
    out.extend_from_slice(ct);
    Ok(out)
}

/// A wrap's key and nonce: HKDF-SHA256 with `salt = SHA-256(hdr)` and the
/// shared secret, expanded over `C_WRAP ‖ epk ‖ dk ‖ device_id` to 56
/// bytes. The ephemeral key and the header are new each message, so each
/// key is used for one wrap only and a derived nonce is safe.
fn wrap_key(
    salt: &[u8],
    shared: &[u8; KEY_LEN],
    epk: &[u8; KEY_LEN],
    dk: &[u8; KEY_LEN],
    device: &str,
) -> (Zeroizing<[u8; KEY_LEN]>, [u8; NONCE_LEN]) {
    let mut okm = Zeroizing::new([0u8; KEY_LEN + NONCE_LEN]);
    Hkdf::<Sha256>::new(Some(salt), shared)
        .expand_multi_info(&[C_WRAP, epk, dk, device.as_bytes()], &mut okm[..])
        .expect("56 bytes is far under HKDF-SHA256's limit of 8160");
    let mut kek = Zeroizing::new([0u8; KEY_LEN]);
    kek.copy_from_slice(&okm[..KEY_LEN]);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&okm[KEY_LEN..]);
    (kek, nonce)
}

/// XChaCha20-Poly1305 under `key`, built from a slice so no unwiped copy
/// of the key is made on the way; the cipher wipes its own when dropped.
pub(crate) fn aead(key: &[u8; KEY_LEN]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new_from_slice(key).expect("a 32-byte key")
}

/// The bytes at the front of `ct` that name its key: `SHA-256(C_COMMIT ‖ mk)`.
const COMMIT_LEN: usize = 32;

fn commitment(mk: &[u8; KEY_LEN]) -> [u8; COMMIT_LEN] {
    Sha256::new()
        .chain_update(C_COMMIT)
        .chain_update(mk)
        .finalize()
        .into()
}

/// Whether `ct` commits to `mk`, compared in constant time.
fn committed(mk: &[u8; KEY_LEN], ct: &[u8]) -> bool {
    ct.len() >= COMMIT_LEN && bool::from(commitment(mk).ct_eq(&ct[..COMMIT_LEN]))
}

/// The body under `mk`: the AEAD after the commitment, `hdr` as its
/// associated data.
fn decrypt_body(
    mk: &[u8; KEY_LEN],
    envelope: &Envelope<'_>,
    fields: &Fields,
) -> Result<Zeroizing<Vec<u8>>, chacha20poly1305::Error> {
    aead(mk)
        .decrypt(
            &XNonce::from(fields.n),
            Payload {
                msg: &envelope.ct[COMMIT_LEN..],
                aad: envelope.hdr,
            },
        )
        .map(Zeroizing::new)
}

/// `mk ‖ n ‖ e_seed`: 88 bytes drawn from `rng`, hedged with the sender's
/// identity seed and the message (the module's notes).
fn hedged(
    sender: &DeviceSecrets,
    hdr: &[u8],
    body: &[u8],
    rng: &mut dyn Entropy,
) -> Zeroizing<[u8; DRAWN]> {
    let mut drawn = Zeroizing::new([0u8; DRAWN]);
    rng.fill(&mut drawn[..]);
    let mut out = Zeroizing::new([0u8; DRAWN]);
    Hkdf::<Sha256>::new(Some(&drawn[..]), sender.ik_seed())
        .expand_multi_info(
            &[C_HEDGE, &Sha256::digest(hdr), &Sha256::digest(body)],
            &mut out[..],
        )
        .expect("88 bytes is far under HKDF-SHA256's limit of 8160");
    out
}

/// What a message draws: `mk` (32), `n` (24) and `e_seed` (32).
const DRAWN: usize = KEY_LEN + NONCE_LEN + KEY_LEN;
