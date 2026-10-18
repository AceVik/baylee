use super::*;
use social::disk::ChatDisk as _;

fn fixture() -> (std::path::PathBuf, Session) {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("baylee-vault-flow-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir(&root).unwrap();
    let session = Session {
        gateway: "https://example.test/one".into(),
        token: "session-token".into(),
        handle: "Alice#AB12".into(),
        tag: "AB12".into(),
        guest: false,
    };
    (root, session)
}

#[test]
fn accepted_password_change_with_same_token_locks_before_reopening() {
    let (root, session) = fixture();
    let me = Me {
        tag: session.tag.clone(),
        handle: session.handle.clone(),
        guest: false,
        gateway: session.gateway.clone(),
    };
    let (machine, _) = Social::start(
        me,
        None,
        None,
        social::ChatPrefs::default(),
        now_ms(),
        &mut OsEntropy,
    );
    let mut host = SocialHost {
        machine: Some(machine),
        session: Some(session.clone()),
        info: InfoState::Known(Some((baylee_protocol::social::SOCIAL_VERSION, 30))),
        ..Default::default()
    };
    let mut lobby = LobbyState::from_settings(crate::settings::ClientSettings::default());
    let request = baylee_client_core::lobby::LobbyRequest::ChangePassword {
        username: "alice".into(),
        password: "old".into(),
        new_password: "new".into(),
    };
    let mut login = super::super::Login::from_request(&session.gateway, &request).unwrap();
    login.accept(&session.token);
    lobby.vault_login = Some(login);
    follow(&mut host, &mut lobby);
    assert!(host.machine.is_none());
    assert!(host.store.is_none());
    assert!(lobby.vault_login.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ending_session_drops_stores_and_cancels_inflight_job() {
    let mut host = SocialHost {
        store: Some(Box::new(social::vault::MemorySecrets::new())),
        disk: Some(Box::new(social::disk::MemoryDisk::new())),
        ..Default::default()
    };
    let cancelled = Arc::new(AtomicBool::new(false));
    let (_sender, receiver) = mpsc::channel();
    host.vault.job = Some(Job {
        receiver: Mutex::new(receiver),
        cancelled: cancelled.clone(),
    });
    host.end();
    assert!(host.store.is_none());
    assert!(host.disk.is_none());
    assert!(host.machine.is_none());
    assert!(cancelled.load(Ordering::Acquire));
}

#[test]
fn explicit_creation_reopen_reset_and_change_keep_the_same_device() {
    let (root, session) = fixture();
    let cancelled = AtomicBool::new(false);
    let slot = social::SecretSlot::new(&session.tag, &session.gateway);
    assert!(matches!(
        prepare(
            &root,
            &session,
            Choice::Unlock,
            "password",
            None,
            None,
            &cancelled
        ),
        Err(Error::Missing)
    ));
    let (_, store) = prepare(
        &root,
        &session,
        Choice::Create,
        "password",
        None,
        None,
        &cancelled,
    )
    .unwrap();
    let key = Zeroizing::new(store.load(&slot).unwrap().unwrap());
    drop(store);
    assert!(matches!(
        prepare(
            &root,
            &session,
            Choice::Unlock,
            "admin-reset",
            Some("temporary-password"),
            None,
            &cancelled
        ),
        Err(Error::Authentication)
    ));
    let (_, store) = prepare(
        &root,
        &session,
        Choice::Unlock,
        "new-password",
        Some("password"),
        None,
        &cancelled,
    )
    .unwrap();
    assert_eq!(store.load(&slot).unwrap().as_deref(), Some(key.as_str()));
    drop(store);
    assert!(
        prepare(
            &root,
            &session,
            Choice::Unlock,
            "password",
            None,
            None,
            &cancelled
        )
        .is_err()
    );
    let (_, store) = prepare(
        &root,
        &session,
        Choice::Unlock,
        "new-password",
        None,
        None,
        &cancelled,
    )
    .unwrap();
    assert_eq!(store.load(&slot).unwrap().as_deref(), Some(key.as_str()));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn legacy_file_migration_preserves_source_history_and_is_idempotent() {
    let (root, session) = fixture();
    let cancelled = AtomicBool::new(false);
    let slot = social::SecretSlot::new(&session.tag, &session.gateway);
    let old_folder = root.join(social::disk::chat_folder(&session.gateway, &session.tag));
    let old = social::vault::FileSecrets::new(old_folder.join(social::vault::KEY_FILE));
    let device = baylee_seal::DeviceSecrets::generate(&mut OsEntropy);
    let value = social::vault::encode_secrets(&device);
    old.store(&slot, &value).unwrap();
    let me = Me {
        tag: session.tag.clone(),
        handle: session.handle.clone(),
        guest: false,
        gateway: session.gateway.clone(),
    };
    let (_, effects) = Social::start(
        me,
        None,
        Some(value.to_string()),
        social::ChatPrefs::default(),
        now_ms(),
        &mut OsEntropy,
    );
    let mut old_disk = social::disk::DirDisk::new(&old_folder);
    for effect in effects {
        if let Some(result) = social::disk::apply(&effect, &mut old_disk) {
            result.unwrap();
        }
    }
    old_disk
        .write(
            "history/0123456789abcdef.log",
            b"existing-encrypted-history",
        )
        .unwrap();
    let before = std::fs::read(old_folder.join(social::disk::STATE_FILE)).unwrap();
    let (disk, store) = prepare(
        &root,
        &session,
        Choice::ImportFile,
        "password",
        None,
        None,
        &cancelled,
    )
    .unwrap();
    assert_eq!(store.load(&slot).unwrap().as_deref(), Some(value.as_str()));
    assert_eq!(disk.read(social::disk::STATE_FILE).unwrap(), before);
    assert_eq!(
        disk.read("history/0123456789abcdef.log").unwrap(),
        b"existing-encrypted-history"
    );
    assert_eq!(old.load(&slot).unwrap().as_deref(), Some(value.as_str()));
    assert_eq!(
        std::fs::read(old_folder.join(social::disk::STATE_FILE)).unwrap(),
        before
    );
    drop(store);
    let (_, store) = prepare(
        &root,
        &session,
        Choice::ImportFile,
        "password",
        None,
        None,
        &cancelled,
    )
    .unwrap();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancel_and_failed_unlock_do_not_create_or_replace_a_vault() {
    let (root, session) = fixture();
    let cancelled = AtomicBool::new(true);
    assert!(matches!(
        prepare(
            &root,
            &session,
            Choice::Create,
            "password",
            None,
            None,
            &cancelled
        ),
        Err(Error::Busy)
    ));
    assert!(std::fs::read_dir(&root).unwrap().next().is_none());
    cancelled.store(false, Ordering::Release);
    let (_, store) = prepare(
        &root,
        &session,
        Choice::Create,
        "password",
        None,
        None,
        &cancelled,
    )
    .unwrap();
    drop(store);
    let path = root
        .join(Scope::new(&session.tag, &session.gateway).unwrap().folder())
        .join(social::vault::password::FILE_NAME);
    let before = std::fs::read(&path).unwrap();
    assert!(
        prepare(
            &root,
            &session,
            Choice::Unlock,
            "wrong",
            None,
            Some("new"),
            &cancelled
        )
        .is_err()
    );
    assert!(
        prepare(
            &root,
            &session,
            Choice::Create,
            "new",
            None,
            None,
            &cancelled
        )
        .is_err()
    );
    assert_eq!(before, std::fs::read(path).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrupt_chat_state_blocks_start_without_replacing_keys() {
    let (root, session) = fixture();
    let cancelled = AtomicBool::new(false);
    let (mut disk, store) = prepare(
        &root,
        &session,
        Choice::Create,
        "password",
        None,
        None,
        &cancelled,
    )
    .unwrap();
    disk.write(social::disk::STATE_FILE, b"damaged state")
        .unwrap();
    drop(store);
    assert!(matches!(
        prepare(
            &root,
            &session,
            Choice::Unlock,
            "password",
            None,
            None,
            &cancelled
        ),
        Err(Error::Authentication)
    ));
    assert_eq!(
        disk.read(social::disk::STATE_FILE).unwrap(),
        b"damaged state"
    );
    std::fs::remove_dir_all(root).unwrap();
}
