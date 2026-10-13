//! Shared fixtures: Alice sends, Bob receives, and a few helpers every file uses.
#![allow(dead_code)]

use baylee_seal::testkit::{Seeded, registered};
use baylee_seal::{DevicePublic, DeviceSecrets, Recipient, Sealed, seal};

/// Lower-case hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// `bytes` with one bit of its first byte flipped.
pub fn flipped(bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[0] ^= 1;
    out
}

/// The header every file's fixed message carries.
pub const HDR: &[u8] = br#"{"v":1,"conv":"dm:0012:af03","id":"m1"}"#;
/// The body every file's fixed message carries.
pub const BODY: &[u8] = b"hello, bob";

/// Alice (account "af03", seed 1) and Bob (account "0012", seed 2), and
/// Alice's message to Bob sealed from `Seeded::new(7)`.
pub struct Pair {
    pub alice: DeviceSecrets,
    pub alice_pub: DevicePublic,
    pub bob: DeviceSecrets,
    pub bob_pub: DevicePublic,
    pub sealed: Sealed,
}

pub fn pair() -> Pair {
    let (alice, alice_pub) = registered("af03", 1);
    let (bob, bob_pub) = registered("0012", 2);
    let sealed = seal(
        &alice,
        HDR,
        BODY,
        &[Recipient {
            device: &bob_pub.id,
            dk: bob_pub.dk,
        }],
        &mut Seeded::new(7),
    )
    .unwrap();
    Pair {
        alice,
        alice_pub,
        bob,
        bob_pub,
        sealed,
    }
}
