//! The glue between the chat and `baylee-seal` (§4.2): building a header,
//! sealing a body for the devices of a conversation's members, and opening
//! a delivery after checking that its header matches what the gateway
//! attests.
//!
//! This is the only place an [`Up::Send`]'s envelope is built, and it is
//! built only from [`baylee_seal::seal`]'s output: no path puts plaintext
//! into a frame.

use baylee_protocol::social::{
    Delivered, HEADER_VERSION, Header, MAX_BODY_BYTES, MAX_HEADER_BYTES, Sealed, Sender, Up, Wrap,
};
use baylee_seal::b64::url;
use baylee_seal::{DeviceSecrets, Entropy, Envelope, Recipient};
use zeroize::Zeroizing;

use super::convs::Body;
use super::keys::CachedDevice;

/// The kind of a v1 message.
pub const KIND_TEXT: &str = "text";

/// A message older than this (by its sender's clock) is refused on arrival
/// (§4.2 step 4), and seen ids are kept this long (§4.6).
pub const MAX_AGE_MS: u64 = 30 * 24 * 60 * 60 * 1000;

/// A `UUIDv7` (RFC 9562) from `now_ms` and ten drawn bytes: 48 bits of
/// milliseconds, version 7, 74 random bits, the RFC's variant. Lower case,
/// hyphenated.
pub fn uuid_v7(now_ms: u64, rng: &mut dyn Entropy) -> String {
    let mut bytes = [0u8; 16];
    bytes[..6].copy_from_slice(&now_ms.to_be_bytes()[2..]);
    rng.fill(&mut bytes[6..]);
    bytes[6] = 0x70 | (bytes[6] & 0x0f);
    bytes[8] = 0x80 | (bytes[8] & 0x3f);
    let hex = hex(&bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// `bytes` in lower-case hex.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// The milliseconds a `UUIDv7` was made at; `None` for anything else.
#[must_use]
pub fn uuid_ms(id: &str) -> Option<u64> {
    if !baylee_protocol::social::valid_uuid(id) || id.as_bytes()[14] != b'7' {
        return None;
    }
    let hex: String = id.chars().filter(|c| *c != '-').take(12).collect();
    u64::from_str_radix(&hex, 16).ok()
}

/// A header's bytes: the protocol's [`Header`] as JSON, signed as bytes.
#[must_use]
pub fn header_bytes(id: &str, conv: &str, from: &str, dev: &str, ts: u64) -> Vec<u8> {
    serde_json::to_vec(&Header {
        v: HEADER_VERSION,
        id: id.to_string(),
        conv: conv.to_string(),
        from: from.to_string(),
        dev: dev.to_string(),
        ts,
        kind: KIND_TEXT.to_string(),
    })
    .unwrap_or_default()
}

/// Why a message could not be sealed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SealFault {
    /// The body or header is over its limit (§8.5).
    TooLarge,
    /// A recipient's key is unfit (`baylee-seal` refused it).
    Key,
}

/// A message sealed for sending: the frame's `sealed`, ready for
/// [`Up::Send`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outgoing {
    /// The envelope, every field base64url.
    pub sealed: Sealed,
}

impl Outgoing {
    /// The frame that sends it.
    #[must_use]
    pub fn frame(&self, req: u32, conv: &str) -> Up {
        Up::Send {
            req,
            conv: conv.to_string(),
            sealed: self.sealed.clone(),
        }
    }
}

/// Seals `body` under `hdr` for `devices` (every device of every member but
/// this one), signed by `me`.
///
/// # Errors
///
/// [`SealFault::TooLarge`] past [`MAX_HEADER_BYTES`] or [`MAX_BODY_BYTES`];
/// [`SealFault::Key`] when a device's key is unfit.
pub fn seal_for(
    me: &DeviceSecrets,
    hdr: &[u8],
    body: &Body,
    devices: &[&CachedDevice],
    rng: &mut dyn Entropy,
) -> Result<Outgoing, SealFault> {
    let plain = Zeroizing::new(serde_json::to_vec(body).map_err(|_| SealFault::TooLarge)?);
    if hdr.len() > MAX_HEADER_BYTES || plain.len() > MAX_BODY_BYTES {
        return Err(SealFault::TooLarge);
    }
    let recipients: Vec<Recipient<'_>> = devices
        .iter()
        .filter(|d| d.id != me.device_id())
        .map(|d| Recipient {
            device: &d.id,
            dk: d.dk,
        })
        .collect();
    let sealed =
        baylee_seal::seal(me, hdr, &plain, &recipients, rng).map_err(|_| SealFault::Key)?;
    Ok(Outgoing {
        sealed: Sealed {
            hdr: url::encode(hdr),
            n: url::encode(&sealed.n),
            ct: url::encode(&sealed.ct),
            epk: url::encode(&sealed.epk),
            sig: url::encode(&sealed.sig),
            wraps: sealed
                .wraps
                .iter()
                .map(|w| Wrap {
                    d: w.device.clone(),
                    k: url::encode(&w.k),
                })
                .collect(),
        },
    })
}

/// Why a delivery was not opened. None carries any of its plaintext.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenFault {
    /// The header doesn't decode or parse, or isn't version 1 text.
    Header,
    /// The header names another conversation, sender or device than the
    /// gateway attests (§4.2 step 1): a forwarded or rerouted message.
    Mismatch,
    /// The sending device isn't in the key cache.
    UnknownDevice,
    /// The signature, wrap or body failed (`baylee-seal`'s check).
    Check(baylee_seal::Error),
    /// The body doesn't parse, or has no text.
    Body,
}

/// A delivery's decoded fields.
pub struct Decoded {
    hdr: Vec<u8>,
    n: Vec<u8>,
    ct: Vec<u8>,
    epk: Vec<u8>,
    sig: Vec<u8>,
    k: Vec<u8>,
    /// The parsed header.
    pub header: Header,
}

impl Decoded {
    /// Decodes a delivery and checks its header against the frame (§4.2
    /// step 1): version 1, kind text, its `conv` the frame's, its `from` and
    /// `dev` the sender the gateway attests.
    ///
    /// # Errors
    ///
    /// [`OpenFault::Header`] or [`OpenFault::Mismatch`].
    pub fn read(conv: &str, from: &Sender, delivered: &Delivered) -> Result<Self, OpenFault> {
        let field = |text: &str| url::decode(text).map_err(|_| OpenFault::Header);
        let hdr = field(&delivered.hdr)?;
        if hdr.len() > MAX_HEADER_BYTES {
            return Err(OpenFault::Header);
        }
        let header: Header = serde_json::from_slice(&hdr).map_err(|_| OpenFault::Header)?;
        if header.v != HEADER_VERSION || header.kind != KIND_TEXT {
            return Err(OpenFault::Header);
        }
        if header.conv != conv || header.from != from.tag || header.dev != from.device {
            return Err(OpenFault::Mismatch);
        }
        Ok(Self {
            n: field(&delivered.n)?,
            ct: field(&delivered.ct)?,
            epk: field(&delivered.epk)?,
            sig: field(&delivered.sig)?,
            k: field(&delivered.k)?,
            hdr,
            header,
        })
    }

    /// Opens it on `me` with the sending device's keys (§4.2 steps 2, 3 and
    /// the body's parse; step 4 onwards is the machine's).
    ///
    /// # Errors
    ///
    /// [`OpenFault::Check`] from `baylee-seal`, [`OpenFault::Body`] for a
    /// body that isn't one.
    pub fn open(&self, me: &DeviceSecrets, sender: &CachedDevice) -> Result<Opened, OpenFault> {
        let envelope = Envelope {
            hdr: &self.hdr,
            n: &self.n,
            ct: &self.ct,
            epk: &self.epk,
            sig: &self.sig,
        };
        let opened =
            baylee_seal::open(me, &sender.ik, &envelope, &self.k).map_err(OpenFault::Check)?;
        if opened.body.len() > MAX_BODY_BYTES {
            return Err(OpenFault::Body);
        }
        let body: Body = serde_json::from_slice(&opened.body).map_err(|_| OpenFault::Body)?;
        Ok(Opened {
            body,
            mk: opened.mk,
            plain: opened.body,
        })
    }
}

/// An opened message: its parsed body, and what a report needs.
pub struct Opened {
    /// The body as sent (not yet cleaned).
    pub body: Body,
    /// The message key, which a report reveals (§4.7).
    pub mk: Zeroizing<[u8; 32]>,
    /// The body's bytes as they decrypted, which a report must carry
    /// unchanged.
    pub plain: Zeroizing<Vec<u8>>,
}
