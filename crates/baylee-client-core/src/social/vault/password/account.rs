//! A second way into a password vault for a Baylee Central account: its
//! data key wrapped under a key derived from the account's master key
//! (`central-crypto`'s local vault key, through the account worker), in a
//! file of its own beside the vault (`account-key.wrap`). The vault file is
//! never changed by it, so a gateway's own account still opens the vault
//! with its password, and a Central sign-in without a password (a pairing)
//! opens it with the account (`docs/password-vault.md` §"The account key").
//!
//! Version 1 is exactly 80 bytes: `BAYAKW01`, a 24-byte nonce, and the
//! XChaCha20-Poly1305 ciphertext and tag of the 32-byte data key, bound to
//! the vault's scope. A wrapper that opens to another data key (the vault
//! was made again) fails at the payload, never yields wrong keys.
//!
//! Beside it, `account-key.miss` says that this account's password did not
//! open this vault file, so a sign-in does not spend an Argon2id run on it
//! again. Version 1 is exactly 48 bytes: `BAYAKM01`, a 24-byte nonce, and the
//! XChaCha20-Poly1305 tag over nothing, with the scope and the whole vault
//! record as associated data. It holds no secret and nothing of the
//! password; another account's key, another scope or a changed vault file
//! does not verify it.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use super::{Error, Scope, Vault};

const MAGIC: &[u8; 8] = b"BAYAKW01";
/// Exact size of a wrapper file.
pub const WRAP_LEN: usize = 8 + 24 + 48;
/// The wrapper's file, beside `password.vault`.
pub const WRAP_FILE: &str = "account-key.wrap";

fn aad(scope: &Scope) -> Vec<u8> {
    [
        b"baylee/password-vault/account-key/v1\0".as_slice(),
        &scope.0,
        MAGIC,
    ]
    .concat()
}

const MISS_MAGIC: &[u8; 8] = b"BAYAKM01";
/// Exact size of a miss marker.
pub const MISS_LEN: usize = 8 + 24 + 16;
/// The miss marker's file, beside `password.vault`.
pub const MISS_FILE: &str = "account-key.miss";

fn miss_aad(scope: &Scope, record: &[u8]) -> Vec<u8> {
    [
        b"baylee/password-vault/account-miss/v1\0".as_slice(),
        &scope.0,
        MISS_MAGIC,
        record,
    ]
    .concat()
}

/// A marker that this account's password did not open `record`, for
/// [`MISS_FILE`].
///
/// # Errors
/// No safe randomness, or the cipher refusing.
pub fn miss_marker(
    scope: &Scope,
    account_key: &[u8; 32],
    record: &[u8],
) -> Result<[u8; MISS_LEN], Error> {
    let mut nonce = [0; 24];
    getrandom::fill(&mut nonce).map_err(|_| Error::Entropy)?;
    let tag = XChaCha20Poly1305::new(account_key.into())
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: &[],
                aad: &miss_aad(scope, record),
            },
        )
        .map_err(|_| Error::Format)?;
    let mut marker = [0; MISS_LEN];
    marker[..8].copy_from_slice(MISS_MAGIC);
    marker[8..32].copy_from_slice(&nonce);
    marker[32..].copy_from_slice(&tag);
    Ok(marker)
}

/// Whether `marker` is this account's miss marker for exactly `record`.
#[must_use]
pub fn is_miss_marker(scope: &Scope, account_key: &[u8; 32], record: &[u8], marker: &[u8]) -> bool {
    let Ok(marker) = <[u8; MISS_LEN]>::try_from(marker) else {
        return false;
    };
    let Ok(nonce) = <[u8; 24]>::try_from(&marker[8..32]) else {
        return false;
    };
    &marker[..8] == MISS_MAGIC
        && XChaCha20Poly1305::new(account_key.into())
            .decrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: &marker[32..],
                    aad: &miss_aad(scope, record),
                },
            )
            .is_ok_and(|plain| plain.is_empty())
}

impl Vault {
    /// The data key wrapped under `account_key`, for [`WRAP_FILE`].
    ///
    /// # Errors
    /// No safe randomness, or the cipher refusing.
    pub fn account_wrap(&self, account_key: &[u8; 32]) -> Result<[u8; WRAP_LEN], Error> {
        let mut nonce = [0; 24];
        getrandom::fill(&mut nonce).map_err(|_| Error::Entropy)?;
        let sealed = XChaCha20Poly1305::new(account_key.into())
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: &self.key[..],
                    aad: &aad(&self.scope),
                },
            )
            .map_err(|_| Error::Format)?;
        let mut wrap = [0; WRAP_LEN];
        wrap[..8].copy_from_slice(MAGIC);
        wrap[8..32].copy_from_slice(&nonce);
        wrap[32..].copy_from_slice(&sealed);
        Ok(wrap)
    }

    /// Opens a vault record with its account wrapper instead of a password;
    /// the whole envelope is authenticated before anything is released.
    ///
    /// # Errors
    /// A wrapper of another size or version, another account's key, another
    /// scope, or a vault made again since the wrapper was written.
    pub fn unlock_with_account(
        scope: Scope,
        account_key: &[u8; 32],
        wrap: &[u8],
        record: &[u8],
    ) -> Result<Self, Error> {
        let wrap: [u8; WRAP_LEN] = wrap.try_into().map_err(|_| Error::Format)?;
        if &wrap[..8] != MAGIC {
            return Err(Error::Format);
        }
        let plain = Zeroizing::new(
            XChaCha20Poly1305::new(account_key.into())
                .decrypt(
                    &XNonce::from(<[u8; 24]>::try_from(&wrap[8..32]).map_err(|_| Error::Format)?),
                    Payload {
                        msg: &wrap[32..],
                        aad: &aad(&scope),
                    },
                )
                .map_err(|_| Error::Authentication)?,
        );
        let mut key = Zeroizing::new([0; 32]);
        if plain.len() != 32 {
            return Err(Error::Format);
        }
        key.copy_from_slice(&plain);
        Self::with_key(scope, key, record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(tag: &str) -> Scope {
        Scope::new(tag, "https://gateway.example").unwrap()
    }

    const PASSWORD: &str = "a long account password";

    #[test]
    fn a_miss_marker_holds_for_its_account_scope_and_vault_file_only() {
        let vault = Vault::create(scope("abc1"), PASSWORD, &[7; 96]).unwrap();
        let record = *vault.record();
        let marker = miss_marker(&scope("abc1"), &[9; 32], &record).unwrap();
        assert!(is_miss_marker(&scope("abc1"), &[9; 32], &record, &marker));
        assert!(
            !is_miss_marker(&scope("abc1"), &[8; 32], &record, &marker),
            "another account's key"
        );
        assert!(
            !is_miss_marker(&scope("abc2"), &[9; 32], &record, &marker),
            "another vault's scope"
        );
        let again = Vault::create(scope("abc1"), PASSWORD, &[7; 96]).unwrap();
        assert!(
            !is_miss_marker(&scope("abc1"), &[9; 32], again.record(), &marker),
            "a vault made again"
        );
        for at in 0..MISS_LEN {
            let mut damaged = marker;
            damaged[at] ^= 1;
            assert!(
                !is_miss_marker(&scope("abc1"), &[9; 32], &record, &damaged),
                "byte {at}"
            );
        }
        for len in [0, MISS_LEN - 1, MISS_LEN + 1] {
            assert!(!is_miss_marker(
                &scope("abc1"),
                &[9; 32],
                &record,
                &vec![0; len]
            ));
        }
        assert_ne!(
            marker,
            miss_marker(&scope("abc1"), &[9; 32], &record).unwrap(),
            "a fresh nonce each time"
        );
    }

    #[test]
    fn the_account_opens_what_the_password_opens_and_the_vault_is_unchanged() {
        let secrets = [7; 96];
        let vault = Vault::create(scope("abc1"), PASSWORD, &secrets).unwrap();
        let record = *vault.record();
        let wrap = vault.account_wrap(&[9; 32]).unwrap();
        let opened = Vault::unlock_with_account(scope("abc1"), &[9; 32], &wrap, &record).unwrap();
        assert_eq!(opened.secrets().unwrap().as_slice(), secrets.as_slice());
        assert_eq!(opened.record(), &record, "the vault file is not touched");
        // The password still opens it, as before.
        assert!(Vault::unlock(scope("abc1"), PASSWORD, &record).is_ok());
    }

    #[test]
    fn another_key_scope_or_vault_is_refused() {
        let vault = Vault::create(scope("abc1"), PASSWORD, &[7; 96]).unwrap();
        let record = *vault.record();
        let wrap = vault.account_wrap(&[9; 32]).unwrap();
        assert_eq!(
            Vault::unlock_with_account(scope("abc1"), &[8; 32], &wrap, &record).unwrap_err(),
            Error::Authentication,
            "another account's key"
        );
        assert_eq!(
            Vault::unlock_with_account(scope("abc2"), &[9; 32], &wrap, &record).unwrap_err(),
            Error::Authentication,
            "another vault's scope"
        );
        let again = Vault::create(scope("abc1"), PASSWORD, &[7; 96]).unwrap();
        assert!(
            Vault::unlock_with_account(scope("abc1"), &[9; 32], &wrap, again.record()).is_err(),
            "a vault made again has another data key"
        );
    }

    #[test]
    fn every_byte_of_the_wrapper_counts() {
        let vault = Vault::create(scope("abc1"), PASSWORD, &[7; 96]).unwrap();
        let record = *vault.record();
        let wrap = vault.account_wrap(&[9; 32]).unwrap();
        for at in 0..WRAP_LEN {
            let mut damaged = wrap;
            damaged[at] ^= 1;
            assert!(
                Vault::unlock_with_account(scope("abc1"), &[9; 32], &damaged, &record).is_err(),
                "byte {at}"
            );
        }
        for len in [0, WRAP_LEN - 1, WRAP_LEN + 1] {
            assert_eq!(
                Vault::unlock_with_account(scope("abc1"), &[9; 32], &vec![0; len], &record)
                    .unwrap_err(),
                Error::Format
            );
        }
    }

    #[test]
    fn two_wrappers_of_one_vault_differ() {
        let vault = Vault::create(scope("abc1"), PASSWORD, &[7; 96]).unwrap();
        assert_ne!(
            vault.account_wrap(&[9; 32]).unwrap(),
            vault.account_wrap(&[9; 32]).unwrap(),
            "a fresh nonce each time"
        );
    }
}
