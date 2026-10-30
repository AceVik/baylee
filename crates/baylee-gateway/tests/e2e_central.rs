//! Synthetic real gateway + Central + PostgreSQL migration, never production services.
#![forbid(unsafe_code)]
#[path = "central/aliases.rs"]
mod aliases;
#[path = "e2e_central/chat.rs"]
mod chat;
mod common;
#[path = "central/fresh.rs"]
mod fresh;
#[path = "central/held.rs"]
mod held;
#[path = "central/official.rs"]
mod official;
#[path = "central/profiles.rs"]
mod profiles;
#[path = "central/registered.rs"]
mod registered;
#[path = "central/renewal.rs"]
mod renewal;
#[path = "central/terms.rs"]
mod terms;
#[path = "central/token.rs"]
mod token;
#[path = "central/upgrade.rs"]
mod upgrade;

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
use std::{
    io::Write,
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::{runtime::Runtime, sync::oneshot, task::JoinHandle};

struct BridgeFile(PathBuf);
impl Drop for BridgeFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn bridge_file(endpoint: &Endpoint, bridge: &Token) -> BridgeFile {
    config_file(&serde_json::json!({"endpoint":endpoint.origin(),"bridge":bridge}))
}

fn config_file(settings: &serde_json::Value) -> BridgeFile {
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
    let bytes = zeroize::Zeroizing::new(serde_json::to_vec(settings).unwrap());
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

/// What Central's runtime role may do: Central's `ops/grants.sql` at the pinned
/// revision, statement for statement.
const RUNTIME_GRANTS: &str = "GRANT USAGE ON SCHEMA central_identity TO central_runtime;
            GRANT SELECT, INSERT ON central_identity.pending_accounts TO central_runtime;
            GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.accounts, central_identity.sessions,
                central_identity.gateway_tickets, central_identity.private_objects TO central_runtime;
            GRANT SELECT, INSERT ON central_identity.migration_archives TO central_runtime;
            GRANT SELECT, INSERT, UPDATE ON central_identity.gateways TO central_runtime;
            GRANT SELECT, INSERT, UPDATE (enabled) ON central_identity.gateway_approvals TO central_runtime;
            GRANT SELECT, INSERT, UPDATE (revoked_at, last_used_at) ON central_identity.gateway_tokens TO central_runtime;
            GRANT SELECT, INSERT, DELETE ON central_identity.gateway_admins, central_identity.gateway_leases TO central_runtime;
            GRANT UPDATE (expires_at) ON central_identity.gateway_leases TO central_runtime;
            GRANT SELECT, INSERT ON central_identity.gateway_terms, central_identity.gateway_terms_texts,
                central_identity.terms_acceptances TO central_runtime;
            GRANT SELECT, UPDATE (uses_left) ON central_identity.beta_keys TO central_runtime;
            GRANT SELECT, INSERT, DELETE ON central_identity.account_roles TO central_runtime;
            GRANT SELECT, INSERT ON central_identity.audit_events TO central_runtime";

/// `/auth/config` says whether the gateway takes Central sign-ins: a native
/// client signs in through Central only where it does, and with the
/// gateway's own routes everywhere else. No Central answers here; the
/// gateway is bound by its configuration alone.
#[test]
fn auth_config_says_whether_the_gateway_takes_central_sign_ins() {
    let plain = common::spawn_gateway("central_unbound");
    let (status, config) = common::http(plain.port, "GET", "/auth/config", None, "");
    assert_eq!(status, 200, "{config}");
    assert!(config.contains("\"central\":false"), "{config}");

    let endpoint = Endpoint::development("http://127.0.0.1:9").unwrap();
    let file = bridge_file(&endpoint, &Token::generate().unwrap());
    let bound = common::spawn_gateway_with(
        "central_bound",
        &[(
            "BAYLEE_CENTRAL_CONFIG_FILE",
            file.0.to_string_lossy().into_owned(),
        )],
    );
    let (status, config) = common::http(bound.port, "GET", "/auth/config", None, "");
    assert_eq!(status, 200, "{config}");
    assert!(config.contains("\"central\":true"), "{config}");
}

#[test]
#[ignore = "requires owned PostgreSQL fixture from cargo dev central integration"]
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
        operator.execute_unprepared(RUNTIME_GRANTS).await.unwrap();
    });
    let store = runtime
        .block_on(AccountStore::connect_dev(database(
            "central_runtime",
            "CENTRAL_TEST_RUNTIME_PASSWORD",
        )))
        .unwrap();
    let served = serve(&runtime, &store, "central", None);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (gateway, endpoint, origin) = (&served.gateway, &served.endpoint, &served.origin);
        run(gateway, endpoint, origin);
        profiles::run(&runtime, &operator, gateway, endpoint, origin);
        changed_legacy_password_invalidates_pending_enrollment(gateway, endpoint, origin);
        upgrade::run(gateway, endpoint, origin);
        renewal::run(gateway, endpoint, origin);
        fresh::run(&runtime, &served.service, &operator, endpoint, gateway);
        registered::run(&runtime, &operator, endpoint);
        terms::run(&runtime, &operator, endpoint);
        aliases::run(gateway, endpoint);
        official::run(&runtime, &operator, endpoint, &served.bridge, origin);
        token::run(&runtime, &store, &operator, endpoint);
        held::run(&runtime, &store);
        ui_fixture(endpoint, origin);
    }));
    served.close(&runtime);
    runtime.block_on(async {
        store.close().await.unwrap();
        operator.close().await.unwrap();
    });
    if let Err(failure) = result {
        std::panic::resume_unwind(failure);
    }
}

/// A Central development server over the fixture's store, its identity
/// service (with `terms` in force) and a gateway bridged to it.
struct Served {
    endpoint: Endpoint,
    gateway: common::Gateway,
    origin: String,
    service: Arc<IdentityService>,
    /// The bridge credential the gateway holds, for a second bridge-mode
    /// gateway (`central/official.rs`).
    bridge: Token,
    secret_file: BridgeFile,
    stop: oneshot::Sender<()>,
    serving: JoinHandle<()>,
    address: SocketAddr,
    gateway_address: SocketAddr,
}

fn serve(
    runtime: &Runtime,
    store: &AccountStore,
    label: &str,
    terms: Option<baylee_account::terms::TermsDocument>,
) -> Served {
    let server = runtime
        .block_on(DevelopmentServer::bind(
            central_policy::config::Config::from_pairs([
                ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
                ("BAYLEE_CENTRAL_REQUEST_BYTES", "1056768"),
            ])
            .unwrap(),
        ))
        .unwrap();
    let address = server.local_addr();
    let endpoint = Endpoint::development(&format!("http://{address}")).unwrap();
    let bridge = Token::generate().unwrap();
    let secret_file = bridge_file(&endpoint, &bridge);
    let gateway = common::spawn_gateway_with(
        label,
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
    let origin = format!("http://{gateway_address}");
    let service = Arc::new(
        IdentityService::new(
            store.clone(),
            &endpoint,
            ServerKeys::generate(),
            &bridge,
            &origin,
        )
        .unwrap()
        .with_aliases(vec![aliases::ALIAS.into()])
        .unwrap()
        .with_terms(terms),
    );
    let (stop, stopped) = oneshot::channel();
    let served_endpoint = endpoint.clone();
    let identity = Arc::clone(&service);
    let serving = runtime.spawn(async move {
        server
            .run_with_identity(&served_endpoint, identity, async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    Served {
        endpoint,
        gateway,
        origin,
        service,
        bridge,
        secret_file,
        stop,
        serving,
        address,
        gateway_address,
    }
}

impl Served {
    /// Stops the gateway and Central, and checks both listeners are gone.
    fn close(self, runtime: &Runtime) {
        drop(self.gateway);
        drop(self.secret_file);
        let _ = self.stop.send(());
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), self.serving)
                .await
                .unwrap()
                .unwrap();
        });
        for closed in [self.address, self.gateway_address] {
            // Windows may report loopback refusal after about two seconds; a
            // shorter timeout is not evidence that a listener remains or closed.
            let error = TcpStream::connect_timeout(&closed, Duration::from_secs(5))
                .expect_err("owned service listener must be closed");
            assert_eq!(error.kind(), std::io::ErrorKind::ConnectionRefused);
        }
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
        chat: baylee_account::chat::ChatArchive::default(),
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
    first
        .save(Some(serde_json::json!({"device_one":1})), None)
        .unwrap();
    let chat = chat::first_device(&mut first, origin);
    let admitted = first.admit(origin).unwrap();
    let old_gateway_token = admitted.token.clone();
    assert_eq!(
        common::http(port, "GET", "/me", Some(&old_gateway_token), "").0,
        200
    );
    let mut second = Account::login(
        endpoint.clone(),
        "synthetic".into(),
        "a-very-fine-password".into(),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(second.data().gateways, expected.gateways);
    assert_eq!(second.data().preferences["device_one"], 1);
    assert_eq!(
        second.data().preferences["future"],
        expected.preferences["future"]
    );
    assert!(
        first
            .save(Some(serde_json::json!({"revoked":true})), None)
            .is_err()
    );
    assert_eq!(
        common::http(port, "GET", "/me", Some(&old_gateway_token), "").0,
        401,
        "a successful new Central login immediately revokes the previous gateway lease"
    );
    second
        .save(Some(serde_json::json!({"device_two":2})), None)
        .unwrap();
    assert_eq!(second.data().preferences["device_one"], 1);
    assert_eq!(second.data().preferences["device_two"], 2);
    chat::second_device(&mut first, &mut second, chat);
    let admitted = second.admit(origin).unwrap();
    let token = admitted.token.as_str();
    assert_eq!(common::http(port, "GET", "/me", Some(token), "").0, 200);
    let _ = first.logout();
    second.logout().unwrap();
    assert_eq!(
        common::http(port, "GET", "/me", Some(token), "").0,
        401,
        "central logout cannot leave a sliding gateway session authorized"
    );
}

fn changed_legacy_password_invalidates_pending_enrollment(
    gateway: &common::Gateway,
    endpoint: &Endpoint,
    origin: &str,
) {
    common::login(gateway.port, "password-race", "PasswordRace");
    let legacy = LegacyHttp::development(origin).unwrap();
    let old = Credentials {
        username: "password-race".into(),
        password: "a-very-fine-password".into(),
    };
    let enrollment = legacy.enroll(&old).unwrap();
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
        chat: baylee_account::chat::ChatArchive::default(),
    };
    sdk::enroll(
        &http,
        enrollment.identity,
        enrollment.ticket,
        old.password.clone(),
        &data.write().unwrap(),
    )
    .unwrap();
    let mut session = sdk::login(&http, enrollment.identity, old.password.clone()).unwrap();
    session.confirm_restored(&http).unwrap();
    let stale_link = session
        .gateway_capability(&http, origin.into(), TicketPurpose::Link)
        .unwrap();
    assert_eq!(common::http(gateway.port, "POST", "/auth/password", None,
        r#"{"username":"password-race","password":"a-very-fine-password","new_password":"new-synthetic-password"}"#).0, 200);
    assert!(session.refresh(&http).is_err());
    assert!(
        legacy
            .link(stale_link, data.legacy.as_ref().unwrap().digest)
            .is_err()
    );
    let fresh = legacy
        .enroll(&Credentials {
            username: old.username.clone(),
            password: "new-synthetic-password".into(),
        })
        .unwrap();
    assert_eq!(fresh.identity.account, enrollment.identity.account);
    assert!(fresh.identity.epoch > enrollment.identity.epoch);
    sdk::enroll(
        &http,
        fresh.identity,
        fresh.ticket,
        "new-synthetic-password".into(),
        &data.write().unwrap(),
    )
    .unwrap();
    let mut restored = sdk::login(&http, fresh.identity, "new-synthetic-password".into()).unwrap();
    restored.confirm_restored(&http).unwrap();
    legacy
        .link(
            restored
                .gateway_capability(&http, origin.into(), TicketPurpose::Link)
                .unwrap(),
            fresh.snapshot.digest,
        )
        .unwrap();
    let admitted = baylee_account::GatewayHttp::development(origin)
        .unwrap()
        .admit(restored.gateway_ticket(&http, origin.into()).unwrap())
        .unwrap();
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&admitted.token), "").0,
        200
    );
    restored.logout(&http).unwrap();
}
