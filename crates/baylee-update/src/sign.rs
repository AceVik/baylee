//! Whether an archive is one of ours.
//!
//! Every release archive is signed with Ed25519 (RFC 8032) over its exact
//! bytes, and the signature is published beside it as `<archive>.sig`: one
//! line, the 64 signature bytes in standard base64 (88 characters, padded),
//! then a newline. Text rather than raw bytes so a person can read it on the
//! release page and paste it; [`parse_signature`] takes it with or without
//! the newline.
//!
//! The client trusts exactly the public keys in [`TRUSTED_KEYS`], compiled
//! in. The private half is a 32-byte seed that lives outside this
//! repository (`cargo run -p xtask -- update-key` writes it, and only to
//! `~/.config/baylee-release/update-signing.key`) and, base64, in the
//! repository's Actions secret `BAYLEE_UPDATE_SIGNING_KEY`.
//! `docs/releasing.md` §"Signing" has the rotation path: a new key is added
//! to the list in one release and signs from the next, and an old key leaves
//! the list only once no build that trusts only it is still asking.
//!
//! The `.sha256` beside each archive is checked too ([`checksum_matches`]),
//! but it proves only that the download is whole: anybody who could replace
//! the archive could replace its checksum. The signature is what counts.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{SIGNATURE_LENGTH, Signer as _};
pub use ed25519_dalek::{Signature, SigningKey, VerifyingKey};

/// The public keys an update may be signed with, newest first.
///
/// Base64 of the 32-byte Ed25519 public key. To rotate, add the new key in
/// front and keep the old one until no build that knows only the old one is
/// still in use (`docs/releasing.md` §"Rotating the key").
pub const TRUSTED_KEYS: &[&str] = &[
    // The first key, made 27.09.2026 by `xtask update-key` (#326).
    "Fp1yMpAzen+ZW6P2vzI6fkc2OrTlofxPKYGa3fwi59w=",
];

/// Why an archive, a key or a signature was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The `.sig` is not base64 of 64 bytes.
    MalformedSignature,
    /// A key is not base64 of a valid 32-byte Ed25519 public key.
    MalformedKey,
    /// A seed is not base64 of 32 bytes.
    MalformedSeed,
    /// The signature is well formed but no trusted key made it over these
    /// bytes: a tampered archive, a truncated one, or someone else's.
    NotOurs,
    /// The `.sha256` does not name this file's digest.
    ChecksumMismatch,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MalformedSignature => "the signature is not base64 of 64 bytes",
            Self::MalformedKey => "the key is not a base64 Ed25519 public key",
            Self::MalformedSeed => "the seed is not base64 of 32 bytes",
            Self::NotOurs => "no trusted key signed these bytes",
            Self::ChecksumMismatch => "the checksum does not match",
        })
    }
}

impl std::error::Error for Refused {}

/// One public key from its base64 text.
///
/// # Errors
///
/// [`Refused::MalformedKey`] for anything but a valid 32-byte point.
pub fn public_key(text: &str) -> Result<VerifyingKey, Refused> {
    let bytes = STANDARD
        .decode(text.trim())
        .map_err(|_| Refused::MalformedKey)?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| Refused::MalformedKey)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| Refused::MalformedKey)
}

/// [`TRUSTED_KEYS`], parsed.
///
/// # Panics
///
/// Never in a build that passed its tests: every compiled key is parsed by
/// `every_trusted_key_parses`.
#[must_use]
pub fn trusted_keys() -> Vec<VerifyingKey> {
    TRUSTED_KEYS
        .iter()
        .map(|text| public_key(text).expect("a compiled update key parses"))
        .collect()
}

/// A `.sig` file's contents, as a signature.
///
/// # Errors
///
/// [`Refused::MalformedSignature`] unless it is base64 of 64 bytes.
pub fn parse_signature(file: &[u8]) -> Result<Signature, Refused> {
    let text = std::str::from_utf8(file).map_err(|_| Refused::MalformedSignature)?;
    let bytes = STANDARD
        .decode(text.trim())
        .map_err(|_| Refused::MalformedSignature)?;
    let bytes: [u8; SIGNATURE_LENGTH] =
        bytes.try_into().map_err(|_| Refused::MalformedSignature)?;
    Ok(Signature::from_bytes(&bytes))
}

/// Whether `signature_file` is a signature by one of `keys` over `data`.
///
/// Answers which key, by its place in the list. `verify_strict`, which
/// also refuses the weak and malleable encodings RFC 8032 lets a lenient
/// verifier accept.
///
/// # Errors
///
/// [`Refused::MalformedSignature`] or [`Refused::NotOurs`].
pub fn verify(data: &[u8], signature_file: &[u8], keys: &[VerifyingKey]) -> Result<usize, Refused> {
    let signature = parse_signature(signature_file)?;
    keys.iter()
        .position(|key| key.verify_strict(data, &signature).is_ok())
        .ok_or(Refused::NotOurs)
}

/// The signing key held in a seed's base64 text (the secret's format).
///
/// # Errors
///
/// [`Refused::MalformedSeed`] unless it is base64 of 32 bytes.
pub fn signing_key(seed_text: &str) -> Result<SigningKey, Refused> {
    let bytes = STANDARD
        .decode(seed_text.trim())
        .map_err(|_| Refused::MalformedSeed)?;
    let seed: [u8; 32] = bytes.try_into().map_err(|_| Refused::MalformedSeed)?;
    Ok(SigningKey::from_bytes(&seed))
}

/// A `.sig` file's contents for `data`: base64 and a newline.
#[must_use]
pub fn sign(key: &SigningKey, data: &[u8]) -> String {
    let signature = key.sign(data);
    format!("{}\n", STANDARD.encode(signature.to_bytes()))
}

/// The base64 text of a public key, as [`TRUSTED_KEYS`] holds it.
#[must_use]
pub fn public_text(key: &VerifyingKey) -> String {
    STANDARD.encode(key.to_bytes())
}

/// A fresh seed from the operating system's generator, as base64 text.
///
/// # Errors
///
/// When the system has no entropy to give.
pub fn new_seed_text() -> Result<String, getrandom::Error> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed)?;
    let text = STANDARD.encode(seed);
    seed.fill(0);
    Ok(text)
}

/// Lower-case hex, as `sha256sum` prints a digest.
#[must_use]
pub fn hex_of(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// Whether a `.sha256` file names `digest` for `file_name`.
///
/// The format `sha256sum` and `shasum -a 256` both write: the hex digest,
/// two spaces (or a space and `*` for binary mode), the file name. The name
/// must match too, so one target's checksum is not accepted for another's
/// archive.
///
/// # Errors
///
/// [`Refused::ChecksumMismatch`].
pub fn checksum_matches(file: &str, file_name: &str, digest: &[u8; 32]) -> Result<(), Refused> {
    let line = file.lines().next().unwrap_or_default();
    let (hex, name) = line.split_once(' ').ok_or(Refused::ChecksumMismatch)?;
    let name = name
        .trim_start_matches(' ')
        .trim_start_matches('*')
        .trim_end();
    if hex.eq_ignore_ascii_case(&hex_of(digest)) && name == file_name {
        Ok(())
    } else {
        Err(Refused::ChecksumMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest as _, Sha256};

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    /// Every key the client ships with parses, so `trusted_keys` never
    /// panics in a build that passed this test.
    #[test]
    fn every_trusted_key_parses() {
        assert!(!TRUSTED_KEYS.is_empty());
        for text in TRUSTED_KEYS {
            public_key(text).unwrap_or_else(|_| panic!("{text} does not parse"));
        }
        assert_eq!(trusted_keys().len(), TRUSTED_KEYS.len());
    }

    /// RFC 8032 §7.1, test 2: the verifier is Ed25519 and not something
    /// that merely round-trips with its own signer.
    #[test]
    fn the_verifier_agrees_with_rfc_8032() {
        let public = hex("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c");
        let message = hex("72");
        let signature = hex(
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da\
             085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
        );
        let key = VerifyingKey::from_bytes(&public.clone().try_into().unwrap()).unwrap();
        let file = format!("{}\n", STANDARD.encode(&signature));
        assert_eq!(verify(&message, file.as_bytes(), &[key]), Ok(0));
        // The seed of the same vector signs the same bytes.
        let seed = hex("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb");
        let signer = SigningKey::from_bytes(&seed.try_into().unwrap());
        assert_eq!(sign(&signer, &message), file);
        assert_eq!(signer.verifying_key().to_bytes().to_vec(), public);
    }

    #[test]
    fn a_signature_over_the_archive_verifies() {
        let data = b"an archive";
        let file = sign(&key(1), data);
        assert!(file.ends_with('\n'));
        assert_eq!(file.trim().len(), 88);
        assert_eq!(
            verify(data, file.as_bytes(), &[key(1).verifying_key()]),
            Ok(0)
        );
        // Without its newline, as someone pasting it would leave it.
        assert_eq!(
            verify(data, file.trim().as_bytes(), &[key(1).verifying_key()]),
            Ok(0)
        );
    }

    #[test]
    fn one_changed_byte_is_refused() {
        let mut data = b"an archive of some length".to_vec();
        let file = sign(&key(1), &data);
        for at in [0, data.len() / 2, data.len() - 1] {
            data[at] ^= 1;
            assert_eq!(
                verify(&data, file.as_bytes(), &[key(1).verifying_key()]),
                Err(Refused::NotOurs),
                "byte {at} flipped"
            );
            data[at] ^= 1;
        }
    }

    #[test]
    fn a_truncated_archive_is_refused() {
        let data = b"an archive of some length";
        let file = sign(&key(1), data);
        assert_eq!(
            verify(
                &data[..data.len() - 1],
                file.as_bytes(),
                &[key(1).verifying_key()]
            ),
            Err(Refused::NotOurs)
        );
    }

    #[test]
    fn someone_elses_key_is_refused() {
        let data = b"an archive";
        let file = sign(&key(2), data);
        assert_eq!(
            verify(data, file.as_bytes(), &[key(1).verifying_key()]),
            Err(Refused::NotOurs)
        );
    }

    #[test]
    fn a_missing_or_broken_signature_is_refused() {
        let keys = [key(1).verifying_key()];
        assert_eq!(verify(b"x", b"", &keys), Err(Refused::MalformedSignature));
        assert_eq!(
            verify(b"x", b"not base64!", &keys),
            Err(Refused::MalformedSignature)
        );
        // Base64, but of 63 bytes: a truncated signature file.
        let short = STANDARD.encode([0u8; 63]);
        assert_eq!(
            verify(b"x", short.as_bytes(), &keys),
            Err(Refused::MalformedSignature)
        );
        // 64 zero bytes is well formed and signs nothing.
        let zero = STANDARD.encode([0u8; 64]);
        assert_eq!(verify(b"x", zero.as_bytes(), &keys), Err(Refused::NotOurs));
        // No key at all trusts nothing.
        assert_eq!(
            verify(b"x", sign(&key(1), b"x").as_bytes(), &[]),
            Err(Refused::NotOurs)
        );
    }

    /// The rotation path: a list holding the new key and the old one
    /// accepts what either signed, and says which; a list without the old
    /// key refuses what only it signed.
    #[test]
    fn a_key_list_accepts_the_old_key_and_the_new_one() {
        let (old, new) = (key(1), key(2));
        let both = [new.verifying_key(), old.verifying_key()];
        let data = b"an archive";
        assert_eq!(verify(data, sign(&new, data).as_bytes(), &both), Ok(0));
        assert_eq!(verify(data, sign(&old, data).as_bytes(), &both), Ok(1));
        assert_eq!(
            verify(data, sign(&old, data).as_bytes(), &[new.verifying_key()]),
            Err(Refused::NotOurs)
        );
    }

    #[test]
    fn a_seed_round_trips_through_its_text() {
        let text = new_seed_text().unwrap();
        let signer = signing_key(&text).unwrap();
        let public = public_key(&public_text(&signer.verifying_key())).unwrap();
        assert_eq!(public, signer.verifying_key());
        // With the newline `gh secret set < file` would pass on.
        assert_eq!(
            signing_key(&format!("{text}\n")).unwrap().to_bytes(),
            signer.to_bytes()
        );
        assert_eq!(signing_key("c2hvcnQ="), Err(Refused::MalformedSeed));
        assert_eq!(signing_key(""), Err(Refused::MalformedSeed));
        // Two seeds are not the same seed.
        assert_ne!(new_seed_text().unwrap(), text);
    }

    #[test]
    fn a_key_that_is_not_a_key_is_refused() {
        assert_eq!(public_key(""), Err(Refused::MalformedKey));
        assert_eq!(
            public_key(&STANDARD.encode([1u8; 31])),
            Err(Refused::MalformedKey)
        );
    }

    #[test]
    fn the_checksum_file_is_read_as_sha256sum_writes_it() {
        let data = b"an archive";
        let digest: [u8; 32] = Sha256::digest(data).into();
        let ours = hex_of(&digest);
        let name = "baylee-client-0.1.0-beta.3-x86_64-unknown-linux-gnu.tar.gz";
        assert_eq!(
            checksum_matches(&format!("{ours}  {name}\n"), name, &digest),
            Ok(())
        );
        // `shasum`'s binary mode.
        assert_eq!(
            checksum_matches(&format!("{ours} *{name}\n"), name, &digest),
            Ok(())
        );
        // Another archive's name, another digest, nothing at all.
        assert_eq!(
            checksum_matches(&format!("{ours}  other.tar.gz\n"), name, &digest),
            Err(Refused::ChecksumMismatch)
        );
        let other: [u8; 32] = Sha256::digest(b"other").into();
        assert_eq!(
            checksum_matches(&format!("{ours}  {name}\n"), name, &other),
            Err(Refused::ChecksumMismatch)
        );
        assert_eq!(
            checksum_matches("", name, &digest),
            Err(Refused::ChecksumMismatch)
        );
    }
}
