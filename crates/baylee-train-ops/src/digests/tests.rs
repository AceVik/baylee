use super::*;

#[test]
fn source_modules_are_sorted_and_bytecode_excluded() {
    let dir = tempfile::tempdir().unwrap();
    let module = dir.path().join("module");
    fs::create_dir(&module).unwrap();
    fs::write(module.join("a.py"), b"a").unwrap();
    fs::write(module.join("b.py"), b"b").unwrap();
    fs::write(module.join("a.pyc"), b"ignored").unwrap();
    let digest = source_digest(dir.path(), &[PathBuf::from("module")]).unwrap();
    assert_eq!(
        digest,
        hex(&Sha256::digest(b"module/a.py\0a\0module/b.py\0b\0"))
    );
    fs::write(module.join("a.pyc"), b"changed").unwrap();
    assert_eq!(
        digest,
        source_digest(dir.path(), &[PathBuf::from("module")]).unwrap()
    );
    fs::write(module.join("b.py"), b"new code").unwrap();
    assert_ne!(
        digest,
        source_digest(dir.path(), &[PathBuf::from("module")]).unwrap()
    );
}

#[test]
fn unchanged_hits_and_changed_seals_do_not_rewrite_expected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input");
    fs::write(&path, b"original").unwrap();
    let mut cache = InputDigestCache::default();
    let first = cache.sha256(&path).unwrap();
    assert_eq!(
        first,
        "0682c5f2076f099c34cfdd15a9e063849ed437a49677e6fcc5b4198c76575be5"
    );
    let before = cache.statistics();
    assert_eq!(cache.sha256(&path).unwrap(), first);
    assert_eq!(cache.statistics().bytes_read, before.bytes_read);
    assert_eq!(cache.statistics().hits, 1);
    let expected = BTreeMap::from([(path.clone(), first.clone())]);
    fs::write(&path, b"modified").unwrap();
    assert!(cache.verify(&expected).is_err());
    assert_eq!(expected[&path], first);
    assert!(cache.sha256(dir.path()).is_err());
}

#[test]
fn concurrent_edit_rejected_without_caching() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input");
    fs::write(&path, b"original").unwrap();
    let mut cache = InputDigestCache::default();
    assert!(
        cache
            .hash_with(&path, || fs::write(&path, b"modified").unwrap())
            .is_err()
    );
    let digest = cache.sha256(&path).unwrap();
    assert_eq!(digest, hex(&Sha256::digest(b"modified")));
    assert_eq!(cache.statistics().hashes, 2);
    assert_eq!(cache.statistics().hits, 0);
}

#[cfg(unix)]
#[test]
fn replacement_and_symlink_retarget_invalidate_receipts() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    let link = dir.path().join("link");
    fs::write(&a, b"original").unwrap();
    fs::write(&b, b"replaced").unwrap();
    symlink(&a, &link).unwrap();
    let mut cache = InputDigestCache::default();
    let first = cache.sha256(&link).unwrap();
    fs::remove_file(&link).unwrap();
    symlink(&b, &link).unwrap();
    assert_ne!(cache.sha256(&link).unwrap(), first);
    fs::rename(&a, &b).unwrap();
    assert_eq!(cache.sha256(&link).unwrap(), first);
    assert_eq!(cache.statistics().hits, 0);
}

#[cfg(unix)]
#[test]
fn restored_mtime_still_invalidates_changed_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input");
    fs::write(&path, b"original").unwrap();
    let mut cache = InputDigestCache::default();
    let first = cache.sha256(&path).unwrap();
    let modified = path.metadata().unwrap().modified().unwrap();
    fs::write(&path, b"modified").unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    assert_ne!(cache.sha256(&path).unwrap(), first);
}

#[cfg(target_os = "linux")]
#[test]
fn fifo_rejected_without_blocking() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fifo");
    rustix::fs::mknodat(
        rustix::fs::CWD,
        &path,
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        0,
    )
    .unwrap();
    assert!(InputDigestCache::default().sha256(&path).is_err());
    assert!(open_regular(&path).is_err());
}
