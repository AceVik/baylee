//! Synthetic real gateway + Central + PostgreSQL migration, never production services.
#![forbid(unsafe_code)]
mod common;

use baylee_account::{
    Account, CentralHttp, Endpoint, Token,
    data::{AccountData, LegacyData},
    migration::{Credentials, LegacyHttp},
    sdk,
    wire::TicketPurpose,
};
use central_crypto::opaque::server::ServerKeys;
use central_http::DevelopmentServer;
use central_identity::IdentityService;
use central_postgres::{AccountStore, DevConfig};
use sea_orm::ConnectionTrait;
use std::{io::Write, net::TcpStream, path::PathBuf, sync::Arc, time::Duration};
use tokio::{runtime::Runtime, sync::oneshot};

struct BridgeFile(PathBuf);
impl Drop for BridgeFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn bridge_file(endpoint: &Endpoint, bridge: &Token) -> BridgeFile {
    let path = std::env::temp_dir().join(format!(
        "baylee-central-bridge-{}.json",
        uuid::Uuid::now_v7()
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path).unwrap();
    let guard = BridgeFile(path);
    let bytes = zeroize::Zeroizing::new(
        serde_json::to_vec(&serde_json::json!({"endpoint":endpoint.origin(),"bridge":bridge}))
            .unwrap(),
    );
    file.write_all(&bytes).unwrap();
    guard
}

fn database(role: &str, password: &str) -> DevConfig {
    assert_eq!(
        std::env::var("CENTRAL_TEST_FIXTURE").as_deref(),
        Ok("isolated-v1")
    );
    DevConfig::new(
        std::env::var("CENTRAL_TEST_PORT").unwrap().parse().unwrap(),
        role,
        std::env::var(password).unwrap(),
        "central_native_test",
    )
    .unwrap()
}

#[test]
#[ignore = "requires owned PostgreSQL fixture from scripts/test-central-integration.ps1"]
fn central_login_migrates_then_restores_and_admits_without_the_legacy_password() {
    let runtime = Runtime::new().unwrap();
    let operator = runtime
        .block_on(central_postgres::connect_dev(database(
            "central_migrator",
            "CENTRAL_TEST_MIGRATOR_PASSWORD",
        )))
        .unwrap();
    runtime.block_on(async {
        central_migration::apply(&operator).await.unwrap();
        operator.execute_unprepared("GRANT USAGE ON SCHEMA central_identity TO central_runtime;
            GRANT SELECT, INSERT ON central_identity.pending_accounts TO central_runtime;
            GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.accounts, central_identity.sessions, central_identity.gateway_tickets TO central_runtime").await.unwrap();
    });
    let store = runtime
        .block_on(AccountStore::connect_dev(database(
            "central_runtime",
            "CENTRAL_TEST_RUNTIME_PASSWORD",
        )))
        .unwrap();
    let server = runtime
        .block_on(DevelopmentServer::bind(
            central_policy::config::Config::from_pairs([
                ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
                ("BAYLEE_CENTRAL_REQUEST_BYTES", "1048576"),
            ])
            .unwrap(),
        ))
        .unwrap();
    let address = server.local_addr();
    let endpoint = Endpoint::development(&format!("http://{address}")).unwrap();
    let bridge = Token::generate().unwrap();
    let secret_file = bridge_file(&endpoint, &bridge);
    let gateway = common::spawn_gateway_with(
        "central",
        &[
            (
                "BAYLEE_CENTRAL_CONFIG_FILE",
                secret_file.0.to_string_lossy().into_owned(),
            ),
            ("BAYLEE_BIND", "127.0.0.1".into()),
            ("BAYLEE_REGISTRATION", "open".into()),
        ],
    );
    let gateway_address = format!("127.0.0.1:{}", gateway.port).parse().unwrap();
    let gateway_origin = format!("http://{gateway_address}");
    let service = Arc::new(
        IdentityService::new(
            store.clone(),
            &endpoint,
            ServerKeys::generate(),
            &bridge,
            &gateway_origin,
        )
        .unwrap(),
    );
    let (stop, stopped) = oneshot::channel();
    let served_endpoint = endpoint.clone();
    let serving = runtime.spawn(async move {
        server
            .run_with_identity(&served_endpoint, service, async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&gateway, &endpoint, &gateway_origin);
        ui_fixture(&endpoint, &gateway_origin);
    }));
    drop(gateway);
    drop(secret_file);
    let _ = stop.send(());
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), serving)
            .await
            .unwrap()
            .unwrap();
        store.close().await.unwrap();
        operator.close().await.unwrap();
    });
    for closed in [address, gateway_address] {
        // Windows may report loopback refusal after about two seconds; a
        // shorter timeout is not evidence that a listener remains or closed.
        let error = TcpStream::connect_timeout(&closed, Duration::from_secs(5))
            .expect_err("owned service listener must be closed");
        assert_eq!(error.kind(), std::io::ErrorKind::ConnectionRefused);
    }
    if let Err(failure) = result {
        std::panic::resume_unwind(failure);
    }
}

// Optional bounded handoff to an isolated dev-control client. This fixture has
// only synthetic accounts; the ready file contains no credentials or tokens.
fn ui_fixture(endpoint: &Endpoint, gateway: &str) {
    let Some(path) = std::env::var_os("BAYLEE_CENTRAL_UI_READY") else {
        return;
    };
    let path = PathBuf::from(path);
    std::fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "mode":"synthetic-only", "central":endpoint.origin(), "gateway":gateway,
            "username":"synthetic"
        }))
        .unwrap(),
    )
    .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(180);
    while std::time::Instant::now() < deadline && !path.with_extension("stop").exists() {
        std::thread::sleep(Duration::from_millis(200));
    }
    std::fs::remove_file(path).unwrap();
}

fn run(gateway: &common::Gateway, endpoint: &Endpoint, origin: &str) {
    let old_token = common::login(gateway.port, "synthetic", "Synthetic");
    assert_eq!(
        common::http(
            gateway.port,
            "PUT",
            "/settings",
            Some(&old_token),
            r#"{"future":{"a":1}}"#
        )
        .0,
        200
    );
    let legacy = LegacyHttp::development(origin).unwrap();
    let credentials = Credentials {
        username: "synthetic".into(),
        password: "a-very-fine-password".into(),
    };
    let enrollment = legacy.enroll(&credentials).unwrap();
    let http = CentralHttp::new(endpoint.clone());
    let data = AccountData {
        version: 1,
        gateways: vec![origin.into()],
        preferences: enrollment.snapshot.preferences,
        legacy: Some(LegacyData {
            digest: enrollment.snapshot.digest,
            complete: false,
            gateway: origin.into(),
            account_id: enrollment.snapshot.account_id,
            username: enrollment.snapshot.username,
            decks: enrollment.snapshot.decks,
        }),
    };
    sdk::enroll(
        &http,
        enrollment.identity,
        enrollment.ticket,
        credentials.password.clone(),
        &data.write().unwrap(),
    )
    .unwrap();
    assert!(
        legacy.enroll(&credentials).is_err(),
        "a second legacy login cannot replace enrollment"
    );
    let mut session = sdk::login(&http, enrollment.identity, credentials.password.clone()).unwrap();
    assert_eq!(
        AccountData::read(session.document()).unwrap().preferences,
        data.preferences
    );
    session.confirm_restored(&http).unwrap();
    let data = reconcile(gateway, &legacy, session, data, &http, origin, &old_token);
    restored_account(endpoint, origin, gateway.port, &data);
}

fn reconcile(
    gateway: &common::Gateway,
    legacy: &LegacyHttp,
    mut session: sdk::Session,
    mut data: AccountData,
    http: &CentralHttp,
    origin: &str,
    old_token: &str,
) -> AccountData {
    let admit = session.gateway_ticket(http, origin.into()).unwrap();
    assert!(
        legacy.export(admit).is_err(),
        "admission must not authorize a migration export"
    );
    // Another device edits before cutover. Old sessions still work until the latest copy is restored.
    assert_eq!(
        common::http(
            gateway.port,
            "PUT",
            "/settings",
            Some(old_token),
            r#"{"future":{"a":2}}"#
        )
        .0,
        200
    );
    let first_digest = data.legacy.as_ref().unwrap().digest;
    assert!(
        legacy
            .link(
                session
                    .gateway_capability(http, origin.into(), TicketPurpose::Link)
                    .unwrap(),
                first_digest
            )
            .is_err()
    );
    let latest = legacy
        .export(
            session
                .gateway_capability(http, origin.into(), TicketPurpose::Export)
                .unwrap(),
        )
        .unwrap();
    assert_ne!(latest.digest, first_digest);
    data.preferences = latest.preferences;
    data.legacy.as_mut().unwrap().digest = latest.digest;
    data.legacy.as_mut().unwrap().decks = latest.decks;
    session.save(http, &data.write().unwrap()).unwrap();
    session.refresh(http).unwrap();
    session.confirm_restored(http).unwrap();
    legacy
        .link(
            session
                .gateway_capability(http, origin.into(), TicketPurpose::Link)
                .unwrap(),
            latest.digest,
        )
        .unwrap();
    assert_eq!(
        common::http(gateway.port, "GET", "/settings", Some(old_token), "").0,
        401
    );
    assert_eq!(
        common::http(
            gateway.port,
            "POST",
            "/auth/login",
            None,
            r#"{"username":"synthetic","password":"a-very-fine-password"}"#
        )
        .0,
        401
    );
    legacy
        .link(
            session
                .gateway_capability(http, origin.into(), TicketPurpose::Link)
                .unwrap(),
            latest.digest,
        )
        .unwrap();
    data.legacy.as_mut().unwrap().complete = true;
    session.save(http, &data.write().unwrap()).unwrap();
    session.logout(http).unwrap();
    data
}

fn restored_account(endpoint: &Endpoint, origin: &str, port: u16, expected: &AccountData) {
    let mut first = Account::login(
        endpoint.clone(),
        "synthetic".into(),
        "a-very-fine-password".into(),
        vec!["https://unused.invalid".into()],
    )
    .unwrap();
    let mut second = Account::login(
        endpoint.clone(),
        "synthetic".into(),
        "a-very-fine-password".into(),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(second.data().gateways, expected.gateways);
    assert_eq!(second.data().preferences, expected.preferences);
    first
        .save(Some(serde_json::json!({"device_one":1})), None)
        .unwrap();
    second
        .save(Some(serde_json::json!({"device_two":2})), None)
        .unwrap();
    assert_eq!(second.data().preferences["device_one"], 1);
    assert_eq!(second.data().preferences["device_two"], 2);
    // A form still showing an old remote value must not revert it on its next edit.
    first
        .save(Some(serde_json::json!({"future":{"a":3}})), None)
        .unwrap();
    second
        .save(
            Some(serde_json::json!({"future":{"a":2},"device_two":3})),
            None,
        )
        .unwrap();
    assert_eq!(second.data().preferences["future"]["a"], 3);
    second
        .save(
            Some(serde_json::json!({"future":{"a":2},"device_two":4})),
            None,
        )
        .unwrap();
    assert_eq!(second.data().preferences["future"]["a"], 3);
    let admitted = first.admit(origin).unwrap();
    let token = admitted["token"].as_str().unwrap();
    assert_eq!(common::http(port, "GET", "/me", Some(token), "").0, 200);
    first.logout().unwrap();
    second.logout().unwrap();
}
