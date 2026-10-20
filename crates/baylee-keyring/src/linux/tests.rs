use super::*;

#[test]
fn preserves_existing_attributes_and_rejects_plaintext_or_ambiguity() {
    let entry = Entry::new("baylee-chat", "player@gateway.example").unwrap();
    assert_eq!(
        entry.attributes(),
        HashMap::from([
            ("service", "baylee-chat"),
            ("username", "player@gateway.example")
        ])
    );
    assert_eq!(encrypted(Some(ENCRYPTED_SESSION)), Ok(()));
    for algorithm in [None, Some("plain"), Some(""), Some("unknown")] {
        assert_eq!(encrypted(algorithm), Err(Error::InsecureSession));
    }
    assert_eq!(unique::<u8>(vec![]), Err(Error::NoEntry));
    assert_eq!(unique(vec![7]), Ok(7));
    assert_eq!(unique(vec![7, 8]), Err(Error::Ambiguous));
}

#[test]
fn deadline_drops_pending_operation_without_detached_worker() {
    struct Dropped(std::rc::Rc<std::cell::Cell<bool>>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
    let dropped = std::rc::Rc::new(std::cell::Cell::new(false));
    let guard = Dropped(dropped.clone());
    let result: Result<(), Error> = run_with_timeout(
        async move {
            let _guard = guard;
            std::future::pending().await
        },
        Duration::from_millis(5),
    );
    assert_eq!(result, Err(Error::Timeout));
    assert!(dropped.get());
}

#[test]
#[ignore = "requires cargo dev keyring-linux and its isolated bus/home"]
fn isolated_secret_service() {
    let fixture = std::env::var("BAYLEE_KEYRING_FIXTURE").expect("isolated fixture only");
    assert_eq!(std::env::var("HOME").unwrap(), fixture);
    assert_eq!(
        std::env::var("XDG_DATA_HOME").unwrap(),
        format!("{fixture}/data")
    );
    assert!(std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok());
    let entry = crate::Entry::new("baylee-keyring-fixture", "legacy-user").unwrap();
    assert_eq!(entry.get_password(), Err(Error::NoEntry));
    // Seed exactly the legacy provider's attributes, including an optional
    // target decoration. No new schema-name may hide the existing value.
    let legacy = Entry::new("baylee-keyring-fixture", "legacy-user").unwrap();
    run(async {
        let service = open().await?;
        let mut attrs = legacy.attributes();
        attrs.insert("target", "legacy-collection");
        service
            .store_future(
                None,
                attrs,
                Some(libsecret::COLLECTION_DEFAULT.as_str()),
                "legacy fixture",
                &Value::new("old-secret", "text/plain"),
            )
            .await
            .map_err(|_| Error::Unavailable)
    })
    .unwrap();
    assert_eq!(entry.get_password().unwrap(), "old-secret");
    entry.get_credential().unwrap();
    entry.set_password("new-secret-ä\nsecond-line").unwrap();
    assert_eq!(entry.get_password().unwrap(), "new-secret-ä\nsecond-line");
    assert_eq!(entry.set_password("invalid\0secret"), Err(Error::Invalid));
    assert_eq!(entry.get_password().unwrap(), "new-secret-ä\nsecond-line");
    entry.delete_credential().unwrap();
    assert_eq!(entry.get_credential(), Err(Error::NoEntry));
    entry.set_password("created-by-baylee").unwrap();
    assert_eq!(entry.get_password().unwrap(), "created-by-baylee");
    // A distinct extra attribute creates another match for the legacy pair.
    run(async {
        let service = open().await?;
        let mut attrs = legacy.attributes();
        attrs.insert("target", "other-collection");
        service
            .store_future(
                None,
                attrs,
                Some(libsecret::COLLECTION_DEFAULT.as_str()),
                "ambiguous fixture",
                &Value::new("other-secret", "text/plain"),
            )
            .await
            .map_err(|_| Error::Unavailable)
    })
    .unwrap();
    assert_eq!(entry.get_credential(), Err(Error::Ambiguous));
    assert_eq!(entry.get_password(), Err(Error::Ambiguous));
    assert_eq!(
        entry.set_password("must-not-overwrite"),
        Err(Error::Ambiguous)
    );
    assert_eq!(entry.delete_credential(), Err(Error::Ambiguous));
    run(async {
        let service = open().await?;
        let items = service
            .search_future(
                None,
                legacy.attributes(),
                SearchFlags::ALL | SearchFlags::UNLOCK,
            )
            .await
            .map_err(|_| Error::Unavailable)?;
        assert_eq!(
            items.len(),
            2,
            "ambiguous delete must preserve both credentials"
        );
        let mut values = Vec::new();
        for item in items {
            item.load_secret_future()
                .await
                .map_err(|_| Error::Unavailable)?;
            values.push(item.secret().unwrap().get());
            item.delete_future().await.map_err(|_| Error::Unavailable)?;
        }
        values.sort();
        assert_eq!(
            values,
            [b"created-by-baylee".to_vec(), b"other-secret".to_vec()]
        );
        Ok(())
    })
    .unwrap();
}
