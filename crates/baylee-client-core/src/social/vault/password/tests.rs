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
        context.legacy_keys(None, None),
        Err(Error::Authentication)
    ));
    assert!(matches!(
        context.legacy_keys(None, Some("wrong")),
        Err(Error::Authentication)
    ));
    assert_eq!(
        context
            .legacy_keys(None, Some("synthetic password"))
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

/// A temporary config root, removed with the guard.
#[cfg(not(target_arch = "wasm32"))]
struct Root(std::path::PathBuf);

#[cfg(not(target_arch = "wasm32"))]
impl Root {
    fn new(tag: &str) -> Self {
        Self(std::env::temp_dir().join(format!("baylee-{tag}-{}", uuid::Uuid::now_v7())))
    }
    fn vault(&self, scope: &Scope, password: &str, secrets: u8) {
        let slot = crate::social::vault::SecretSlot::new("AB12", "https://example.test/gateway");
        drop(
            PasswordStore::open(&self.0, scope.clone(), slot, password, Some(&[secrets; 96]))
                .unwrap(),
        );
    }
    fn file(&self, scope: &Scope, name: &str) -> std::path::PathBuf {
        self.0.join(scope.folder()).join(name)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The key a test account would derive for a scope: anything fixed and
/// different per scope will do here.
#[cfg(not(target_arch = "wasm32"))]
fn account_key(scope: &Scope) -> zeroize::Zeroizing<[u8; 32]> {
    let mut key = zeroize::Zeroizing::new(*scope.as_bytes());
    key[0] ^= 0x5a;
    key
}

/// A Central sign-in with the password: the vaults it opens gain the
/// account's wrapper, the vault files stay byte for byte, a vault of another
/// password is left alone, and the next import needs no password.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_password_sign_in_gives_its_vaults_the_account_key_and_changes_none() {
    let root = Root::new("attach");
    let mine = scope();
    let other = Scope::new("CD34", "https://example.test/gateway").unwrap();
    root.vault(&mine, "synthetic password", 19);
    root.vault(&other, "another password", 23);
    let before = (
        std::fs::read(root.file(&mine, FILE_NAME)).unwrap(),
        std::fs::read(root.file(&other, FILE_NAME)).unwrap(),
    );
    let attached = attach_account_keys(&root.0, "synthetic password", |s| Some(account_key(s)), 8);
    assert_eq!(attached, 1);
    assert!(root.file(&mine, WRAP_FILE).exists());
    assert!(
        !root.file(&other, WRAP_FILE).exists(),
        "another password's vault"
    );
    assert_eq!(
        std::fs::read(root.file(&mine, FILE_NAME)).unwrap(),
        before.0
    );
    assert_eq!(
        std::fs::read(root.file(&other, FILE_NAME)).unwrap(),
        before.1
    );
    // Again: the wrapped vault is not tried twice.
    assert_eq!(
        attach_account_keys(&root.0, "synthetic password", |s| Some(account_key(s)), 8),
        0
    );
    // A pairing's chat start: the account's key alone opens it.
    let context = ContextGuard::open(&root.0, mine.clone()).unwrap();
    assert_eq!(
        context
            .legacy_keys(Some(&account_key(&mine)), None)
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96]
    );
    // Another account's key does not, and without a password that is a refusal.
    let mut wrong = account_key(&mine);
    wrong[1] ^= 1;
    assert!(matches!(
        context.legacy_keys(Some(&wrong), None),
        Err(Error::Authentication)
    ));
}

/// A vault without the wrapper opens with the password, which then writes
/// the wrapper; a limit of tries is held.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn an_import_with_the_password_leaves_the_wrapper_for_next_time() {
    let root = Root::new("import");
    let mine = scope();
    root.vault(&mine, "synthetic password", 19);
    assert_eq!(
        attach_account_keys(&root.0, "synthetic password", |s| Some(account_key(s)), 0),
        0,
        "a limit of none tries none"
    );
    let context = ContextGuard::open(&root.0, mine.clone()).unwrap();
    assert!(!root.file(&mine, WRAP_FILE).exists());
    assert_eq!(
        context
            .legacy_keys(Some(&account_key(&mine)), Some("synthetic password"))
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96]
    );
    assert!(root.file(&mine, WRAP_FILE).exists());
    assert_eq!(
        context
            .legacy_keys(Some(&account_key(&mine)), None)
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96]
    );
}

/// A vault another process holds is skipped, not waited for.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_held_vault_is_skipped() {
    let root = Root::new("held");
    let mine = scope();
    root.vault(&mine, "synthetic password", 19);
    let held = ContextGuard::open(&root.0, mine.clone()).unwrap();
    assert_eq!(
        attach_account_keys(&root.0, "synthetic password", |s| Some(account_key(s)), 8),
        0
    );
    drop(held);
    assert_eq!(
        attach_account_keys(&root.0, "synthetic password", |s| Some(account_key(s)), 8),
        1
    );
}

/// A vault this account's password did not open is not tried again while
/// it is the same file, so the vaults past the limit are reached at the
/// next sign-ins; another account's sign-in tries it anew.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_failed_vault_is_not_tried_again_so_the_limit_moves_on() {
    let root = Root::new("limit");
    let mut scopes: Vec<Scope> = ["AB12", "CD34", "EF56"]
        .iter()
        .map(|tag| Scope::new(tag, "https://example.test/gateway").unwrap())
        .collect();
    scopes.sort();
    // The player's own vault sorts last, behind two of other passwords.
    let mine = scopes.pop().unwrap();
    for other in &scopes {
        root.vault(other, "another password", 23);
    }
    root.vault(&mine, "synthetic password", 19);
    let sign_in = |limit| {
        attach_account_keys(
            &root.0,
            "synthetic password",
            |s| Some(account_key(s)),
            limit,
        )
    };
    assert_eq!(sign_in(1), 0);
    assert_eq!(sign_in(1), 0);
    assert_eq!(sign_in(1), 1, "the third sign-in reaches the third vault");
    assert!(root.file(&mine, WRAP_FILE).exists());
    let markers: Vec<Vec<u8>> = scopes
        .iter()
        .map(|other| {
            assert!(!root.file(other, WRAP_FILE).exists());
            std::fs::read(root.file(other, MISS_FILE)).unwrap()
        })
        .collect();
    assert_eq!(sign_in(8), 0, "nothing is left to try");
    for (other, marker) in scopes.iter().zip(&markers) {
        assert_eq!(&std::fs::read(root.file(other, MISS_FILE)).unwrap(), marker);
    }
    // Another account does not take these markers for its own.
    let another = |s: &Scope| {
        let mut key = account_key(s);
        key[1] ^= 1;
        Some(key)
    };
    assert_eq!(
        attach_account_keys(&root.0, "another password", another, 1),
        1
    );
    assert!(root.file(&scopes[0], WRAP_FILE).exists());
    assert_eq!(
        std::fs::read(root.file(&scopes[1], MISS_FILE)).unwrap(),
        markers[1]
    );
}

/// A wrapper that cannot be checked or read (a newer client's version, a
/// damaged copy, a restored backup's mode or link) counts as none: the
/// password still imports, and writes a good wrapper in its place.
#[cfg(unix)]
#[test]
fn a_wrapper_that_cannot_be_read_is_no_wrapper() {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
    let root = Root::new("unreadable");
    let mine = scope();
    root.vault(&mine, "synthetic password", 19);
    let key = account_key(&mine);
    let wrap = root.file(&mine, WRAP_FILE);
    let mode = |m| std::fs::set_permissions(&wrap, std::fs::Permissions::from_mode(m)).unwrap();
    let context = ContextGuard::open(&root.0, mine.clone()).unwrap();
    std::fs::write(&wrap, [0; WRAP_LEN + 1]).unwrap();
    mode(0o600);
    assert_eq!(
        context
            .legacy_keys(Some(&key), Some("synthetic password"))
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96],
        "a wrapper of another length"
    );
    assert_eq!(std::fs::read(&wrap).unwrap().len(), WRAP_LEN);
    mode(0o644);
    assert!(
        matches!(
            context.legacy_keys(Some(&key), None),
            Err(Error::Authentication)
        ),
        "an unchecked wrapper is never read"
    );
    assert_eq!(
        context
            .legacy_keys(Some(&key), Some("synthetic password"))
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96],
        "a wrapper others may read"
    );
    assert_eq!(
        std::fs::metadata(&wrap).unwrap().permissions().mode() & 0o777,
        0o600
    );
    std::fs::hard_link(&wrap, root.file(&mine, "copy.wrap")).unwrap();
    assert_eq!(
        context
            .legacy_keys(Some(&key), Some("synthetic password"))
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96],
        "a wrapper with a second link"
    );
    assert_eq!(std::fs::metadata(&wrap).unwrap().nlink(), 1);
    assert_eq!(
        context
            .legacy_keys(Some(&key), None)
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96]
    );
}

/// A wrapper that cannot be written never costs the import: the password
/// opened the vault, so its keys are answered.
#[cfg(unix)]
#[test]
fn an_unwritable_folder_still_imports() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = Root::new("unwritable");
    let mine = scope();
    root.vault(&mine, "synthetic password", 19);
    let context = ContextGuard::open(&root.0, mine.clone()).unwrap();
    let dir = context.directory().to_path_buf();
    let mode = |m| std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(m)).unwrap();
    mode(0o500);
    let keys = context.legacy_keys(Some(&account_key(&mine)), Some("synthetic password"));
    mode(0o700);
    assert_eq!(keys.unwrap().unwrap().as_slice(), &[19; 96]);
    assert!(!root.file(&mine, WRAP_FILE).exists());
}

/// Another account's wrapper (or one from before the vault was made again)
/// does not open the vault; the password does, and replaces it.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_foreign_wrapper_is_replaced_by_the_password() {
    let root = Root::new("foreign");
    let mine = scope();
    root.vault(&mine, "synthetic password", 19);
    let mut foreign = account_key(&mine);
    foreign[1] ^= 1;
    assert_eq!(
        attach_account_keys(&root.0, "synthetic password", |_| Some(foreign.clone()), 8),
        1
    );
    let before = std::fs::read(root.file(&mine, WRAP_FILE)).unwrap();
    let key = account_key(&mine);
    let context = ContextGuard::open(&root.0, mine.clone()).unwrap();
    assert!(matches!(
        context.legacy_keys(Some(&key), None),
        Err(Error::Authentication)
    ));
    assert_eq!(
        context
            .legacy_keys(Some(&key), Some("synthetic password"))
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96]
    );
    assert_ne!(std::fs::read(root.file(&mine, WRAP_FILE)).unwrap(), before);
    assert_eq!(
        context
            .legacy_keys(Some(&key), None)
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96]
    );
}

/// A sign-in replaces a wrapper its account's key does not open (another
/// account's, or a vault's made again since) and leaves one it opens alone.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_sign_in_replaces_a_stale_wrapper() {
    let root = Root::new("stale");
    let mine = scope();
    root.vault(&mine, "synthetic password", 19);
    let mut foreign = account_key(&mine);
    foreign[1] ^= 1;
    assert_eq!(
        attach_account_keys(&root.0, "synthetic password", |_| Some(foreign.clone()), 8),
        1
    );
    assert_eq!(
        attach_account_keys(&root.0, "synthetic password", |s| Some(account_key(s)), 8),
        1,
        "the stale wrapper is replaced"
    );
    let current = std::fs::read(root.file(&mine, WRAP_FILE)).unwrap();
    assert_eq!(
        attach_account_keys(&root.0, "synthetic password", |s| Some(account_key(s)), 8),
        0
    );
    assert_eq!(
        std::fs::read(root.file(&mine, WRAP_FILE)).unwrap(),
        current,
        "the current wrapper is left alone"
    );
    let context = ContextGuard::open(&root.0, mine.clone()).unwrap();
    assert_eq!(
        context
            .legacy_keys(Some(&account_key(&mine)), None)
            .unwrap()
            .unwrap()
            .as_slice(),
        &[19; 96]
    );
}

#[test]
fn a_folder_name_is_read_back_as_its_scope() {
    let scope = scope();
    let name = scope.folder();
    let name = name.strip_prefix("vault/").unwrap();
    assert_eq!(Scope::from_folder_name(name), Some(scope));
    for bad in [
        "",
        "ab",
        &"A".repeat(64),
        &"g".repeat(64),
        &"a".repeat(63),
        &"a".repeat(65),
    ] {
        assert_eq!(Scope::from_folder_name(bad), None, "{bad}");
    }
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
#[ignore = "the client still pins the old address; runs from the flip on (#402)"]
fn the_live_gateway_s_vault_outlives_the_move_of_its_address() {
    use crate::lobby::gateway_list::{KEYED, PINNED};
    use crate::social::{disk::chat_folder, vault::SecretSlot};
    let old = Scope::new("AB12", KEYED).unwrap();
    let new = Scope::new("AB12", PINNED).unwrap();
    assert_eq!(old, new);
    assert_eq!(old.folder(), new.folder());
    let sealed = Vault::create(old, "a long account password", &[42; 96]).unwrap();
    let opened = Vault::unlock(new, "a long account password", sealed.record()).unwrap();
    assert_eq!(&*opened.secrets().unwrap(), &[42; 96]);
    assert_eq!(chat_folder(KEYED, "AB12"), chat_folder(PINNED, "AB12"));
    assert_eq!(
        chat_folder(&format!("{PINNED}/"), "AB12"),
        chat_folder(KEYED, "AB12")
    );
    assert_eq!(
        SecretSlot::new("AB12", KEYED),
        SecretSlot::new("AB12", PINNED)
    );
    let elsewhere = Scope::new("AB12", "https://play.example").unwrap();
    assert_ne!(elsewhere, Scope::new("AB12", PINNED).unwrap());
    assert_ne!(
        chat_folder("https://play.example", "AB12"),
        chat_folder(PINNED, "AB12")
    );
}
