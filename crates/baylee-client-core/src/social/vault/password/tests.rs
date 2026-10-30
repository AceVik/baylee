use super::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn central_context_keeps_legacy_ciphertext_and_refuses_replaced_installation_ids() {
    use crate::social::vault::SecretSlot;
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let root =
        std::env::temp_dir().join(format!("baylee-central-context-{}", uuid::Uuid::now_v7()));
    let _cleanup = Cleanup(root.clone());
    let slot = SecretSlot::new("AB12", "https://example.test/gateway");
    drop(
        PasswordStore::open(
            &root,
            scope(),
            slot.clone(),
            "synthetic password",
            Some(&[19; 96]),
        )
        .unwrap(),
    );
    let path = root.join(scope().folder()).join(FILE_NAME);
    let ciphertext = std::fs::read(&path).unwrap();
    let context = ContextGuard::open(&root, scope()).unwrap();
    assert!(matches!(
        PasswordStore::open(&root, scope(), slot, "synthetic password", None),
        Err(Error::Busy)
    ));
    let installation = context.installation().unwrap();
    assert_eq!(context.installation().unwrap(), installation);
    assert!(matches!(
        context.legacy_keys(None),
        Err(Error::Authentication)
    ));
    assert!(matches!(
        context.legacy_keys(Some("wrong")),
        Err(Error::Authentication)
    ));
    assert_eq!(
        context
            .legacy_keys(Some("synthetic password"))
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96]
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        ciphertext,
        "import must preserve the original encrypted recovery source"
    );
    std::fs::write(
        context.directory().join("central-installation.id"),
        b"damaged",
    )
    .unwrap();
    assert!(matches!(context.installation(), Err(Error::Format)));
    context.retire().unwrap();
    assert!(matches!(context.installation(), Err(Error::Authentication)));
}

fn scope() -> Scope {
    Scope::new("AB12", "https://example.test/gateway").unwrap()
}

#[test]
fn wrapper_authenticates_password_scope_and_payload_and_rewrap_preserves_dek() {
    let original = [73; 96];
    let mut vault = Vault::create(scope(), "a long account password", &original).unwrap();
    let before = *vault.record();
    assert!(!before.windows(original.len()).any(|w| w == original));
    assert_eq!(&*vault.secrets().unwrap(), &original);
    assert!(matches!(
        Vault::unlock(scope(), "wrong", &before),
        Err(Error::Authentication)
    ));
    assert!(matches!(
        Vault::unlock(
            Scope::new("AB13", "https://example.test/gateway").unwrap(),
            "a long account password",
            &before
        ),
        Err(Error::Authentication)
    ));
    for at in [
        0,
        8,
        HEADER,
        HEADER + 24,
        WRAP_END,
        WRAP_END + 24,
        RECORD_LEN - 1,
    ] {
        let mut damaged = before;
        damaged[at] ^= 1;
        assert!(
            Vault::unlock(scope(), "a long account password", &damaged).is_err(),
            "byte {at}"
        );
    }
    vault.rewrap("new account password").unwrap();
    assert_eq!(&before[WRAP_END..], &vault.record()[WRAP_END..]);
    assert_ne!(&before[..WRAP_END], &vault.record()[..WRAP_END]);
    assert!(Vault::unlock(scope(), "a long account password", vault.record()).is_err());
    let reopened = Vault::unlock(scope(), "new account password", vault.record()).unwrap();
    assert_eq!(&*reopened.secrets().unwrap(), &original);
    assert_eq!(
        &*Vault::unlock(scope(), "a long account password", &before)
            .unwrap()
            .secrets()
            .unwrap(),
        &original
    );
}

#[test]
fn bounded_format_rejects_before_kdf_and_scope_keeps_gateway_path_case() {
    assert!(matches!(
        Vault::unlock(scope(), "password", &[0; RECORD_LEN]),
        Err(Error::Format)
    ));
    assert!(matches!(
        Vault::unlock(scope(), "password", &[0; RECORD_LEN + 1]),
        Err(Error::Format)
    ));
    assert!(derive("", &[0; 16]).is_err());
    assert!(derive(&"x".repeat(MAX_PASSWORD + 1), &[0; 16]).is_err());
    assert_eq!(
        scope(),
        Scope::new("AB12", "https://EXAMPLE.test:443/gateway/").unwrap()
    );
    assert_ne!(
        scope(),
        Scope::new("AB12", "https://example.test/Gateway").unwrap()
    );
    assert_ne!(
        scope(),
        Scope::new("AB12", "http://example.test/gateway").unwrap()
    );
    for (tag, url) in [
        ("../x", "https://example.test"),
        ("AB12", "https://u:p@example.test"),
        ("AB12", "https://example.test?x"),
        ("", "https://example.test"),
    ] {
        assert!(Scope::new(tag, url).is_err());
    }
}

#[test]
fn independent_vaults_and_payload_rotations_use_fresh_randomness() {
    let mut a = Vault::create(scope(), "password", &[31; 96]).unwrap();
    let b = Vault::create(scope(), "password", &[31; 96]).unwrap();
    assert_ne!(a.record(), b.record());
    let before = *a.record();
    a.replace(&[42; 96]).unwrap();
    assert_eq!(&before[..WRAP_END], &a.record()[..WRAP_END]);
    assert_ne!(&before[WRAP_END..], &a.record()[WRAP_END..]);
    assert_eq!(&*a.secrets().unwrap(), &[42; 96]);
}

#[cfg(unix)]
#[test]
fn native_vault_is_exclusive_private_durable_and_never_overwrites_on_failure() {
    use crate::social::vault::{SecretSlot, SecretStore, encode_secrets};
    use std::os::unix::fs::{PermissionsExt as _, symlink};
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("baylee-vault-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir(&root).unwrap();
    let slot = SecretSlot::new("AB12", "https://example.test/gateway");
    assert!(matches!(
        PasswordStore::open(&root, scope(), slot.clone(), "password", None),
        Err(Error::Missing)
    ));
    let store =
        PasswordStore::open(&root, scope(), slot.clone(), "password", Some(&[17; 96])).unwrap();
    let path = root.join(scope().folder()).join(FILE_NAME);
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(matches!(
        PasswordStore::open(&root, scope(), slot.clone(), "password", None),
        Err(Error::Busy)
    ));
    assert!(
        store
            .load(&SecretSlot::new("OTHER", "https://example.test"))
            .is_err()
    );
    let value = encode_secrets(&baylee_seal::DeviceSecrets::from_bytes(&[21; 96]));
    store.store(&slot, &value).unwrap();
    store.rewrap("new password").unwrap();
    drop(store);
    let before = std::fs::read(&path).unwrap();
    assert!(PasswordStore::open(&root, scope(), slot.clone(), "password", None).is_err());
    assert!(matches!(
        PasswordStore::open(&root, scope(), slot.clone(), "new password", Some(&[9; 96])),
        Err(Error::Exists)
    ));
    assert_eq!(before, std::fs::read(&path).unwrap());
    let reopened = PasswordStore::open(&root, scope(), slot.clone(), "new password", None).unwrap();
    assert_eq!(
        reopened.load(&slot).unwrap().as_deref(),
        Some(value.as_str())
    );
    drop(reopened);
    std::fs::rename(&path, path.with_extension("backup")).unwrap();
    symlink(path.with_extension("backup"), &path).unwrap();
    assert!(PasswordStore::open(&root, scope(), slot, "new password", None).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn trusted_root_alias_is_resolved_but_vault_symlink_is_refused() {
    use crate::social::vault::SecretSlot;
    use std::os::unix::fs::{PermissionsExt as _, symlink};
    let temp = std::env::temp_dir();
    let root = temp.join(format!("baylee-vault-alias-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir(&root).unwrap();
    let alias = temp.join(format!("baylee-vault-link-{}", uuid::Uuid::now_v7()));
    symlink(&root, &alias).unwrap();
    let slot = SecretSlot::new("AB12", "https://example.test/gateway");
    let store =
        PasswordStore::open(&alias, scope(), slot.clone(), "password", Some(&[7; 96])).unwrap();
    drop(store);
    let store = PasswordStore::open(&root, scope(), slot.clone(), "password", None).unwrap();
    drop(store);
    let dir = root.join(scope().folder());
    let moved = dir.with_extension("saved");
    std::fs::rename(&dir, &moved).unwrap();
    symlink(&moved, &dir).unwrap();
    assert!(PasswordStore::open(&alias, scope(), slot, "password", None).is_err());
    std::fs::remove_file(&dir).unwrap();
    std::fs::rename(&moved, &dir).unwrap();
    std::fs::set_permissions(root.join("vault"), std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(
        PasswordStore::open(
            &root,
            scope(),
            SecretSlot::new("AB12", "https://example.test/gateway"),
            "password",
            None
        )
        .is_err()
    );
    std::fs::remove_file(alias).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_rewrap_is_transactional_and_debug_is_redacted() {
    let mut vault = Vault::create(scope(), "password", &[55; 96]).unwrap();
    let before = *vault.record();
    assert!(vault.rewrap("").is_err());
    assert_eq!(before, *vault.record());
    assert_eq!(format!("{vault:?}"), "Vault([redacted])");
}

/// A vault an installed client sealed for the live gateway under its former
/// address opens under the new one: same folder, same associated data. The
/// chat folder and the keychain slot an old client wrote are found the same
/// way; another gateway's are not.
#[test]
fn the_live_gateway_s_vault_outlives_the_move_of_its_address() {
    use crate::lobby::gateway_list::{FORMER, PINNED};
    use crate::social::{disk::chat_folder, vault::SecretSlot};
    let old = Scope::new("AB12", FORMER[0]).unwrap();
    let new = Scope::new("AB12", PINNED).unwrap();
    assert_eq!(old, new);
    assert_eq!(old.folder(), new.folder());
    let sealed = Vault::create(old, "a long account password", &[42; 96]).unwrap();
    let opened = Vault::unlock(new, "a long account password", sealed.record()).unwrap();
    assert_eq!(&*opened.secrets().unwrap(), &[42; 96]);
    assert_eq!(chat_folder(FORMER[0], "AB12"), chat_folder(PINNED, "AB12"));
    assert_eq!(
        chat_folder(&format!("{PINNED}/"), "AB12"),
        chat_folder(FORMER[0], "AB12")
    );
    assert_eq!(
        SecretSlot::new("AB12", FORMER[0]),
        SecretSlot::new("AB12", PINNED)
    );
    let elsewhere = Scope::new("AB12", "https://play.example").unwrap();
    assert_ne!(elsewhere, Scope::new("AB12", PINNED).unwrap());
    assert_ne!(
        chat_folder("https://play.example", "AB12"),
        chat_folder(PINNED, "AB12")
    );
}
