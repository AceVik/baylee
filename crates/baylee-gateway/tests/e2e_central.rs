//! Synthetic real gateway + Central + PostgreSQL migration, never production services.
#![forbid(unsafe_code)]
#[path = "central/aliases.rs"]
mod aliases;
#[path = "e2e_central/chat.rs"]
mod chat;
use crate::common;
#[path = "central/fresh.rs"]
mod fresh;
#[path = "central/held.rs"]
mod held;
#[path = "central/official.rs"]
mod official;
#[path = "central/pairing.rs"]
mod pairing;
#[path = "central/personal.rs"]
mod personal;
#[path = "central/registered.rs"]
mod registered;
#[path = "central/renewal.rs"]
mod renewal;
#[path = "central/revocation.rs"]
mod revocation;
#[path = "central/seat.rs"]
mod seat;
#[path = "central/terms.rs"]
mod terms;
#[path = "central/token.rs"]
mod token;

use baylee_account::{
    Account, CentralHttp, Endpoint, Token, data::AccountData, sdk, wire::GatewayPurpose,
};
use central_crypto::jwt::SigningKey;
use central_crypto::opaque::server::ServerKeys;
use central_http::DevelopmentServer;
use central_identity::{IdentityService, TokenKeys};
use central_postgres::{AccountStore, DevConfig, LegacyRow, LegacyStore};
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
    // The alias is known ahead; the gateway adds its own address once bound.
    config_file(&serde_json::json!({
        "endpoint":endpoint.origin(), "bridge":bridge, "origin":aliases::ALIAS,
    }))
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

/// A fresh signing key for one fixture's Central.
fn token_keys() -> TokenKeys {
    let key = SigningKey::from_seed(&SigningKey::generate_seed().unwrap());
    TokenKeys::new(key, Vec::new()).unwrap()
}

/// A gateway token for `origin`, as the client asks for one.
fn gateway_token(
    session: &sdk::Session,
    http: &CentralHttp,
    origin: &str,
    purpose: GatewayPurpose,
) -> baylee_account::SignedToken {
    session
        .gateway_token(http, origin.into(), purpose)
        .unwrap()
        .0
}

/// The body of a sign-in with `token`.
fn sign_in_body(token: &baylee_account::SignedToken) -> String {
    serde_json::json!({ "token": token.expose() }).to_string()
}

/// Every revocation Central holds for a gateway, signed fresh, as its push
/// would send them. Central's development service has no push sender, so the
/// fixture stands in for that one step and nothing else: the message is
/// Central's own, and the route it reaches is the gateway's real one.
fn central_messages(http: &CentralHttp, credential: &Token) -> Vec<baylee_account::SignedToken> {
    use baylee_account::wire::{Request, Response};
    let mut messages = Vec::new();
    let mut after = 0;
    loop {
        let asked = Request::GatewayRevocations { after };
        let Response::GatewayRevocations { token, last, .. } =
            http.exchange(&asked, Some(credential.into())).unwrap()
        else {
            panic!("Central answers what it asked about");
        };
        let Some(token) = token else {
            return messages;
        };
        messages.push(token);
        after = last;
    }
}

/// Posts one message to the gateway's invalidate route.
fn invalidate(port: u16, message: &str) -> u16 {
    common::http_bytes(
        port,
        "POST",
        "/auth/central/invalidate",
        None,
        "application/jwt",
        message.as_bytes(),
    )
    .0
}

/// What Central's push does: everything it holds for this gateway reaches it.
fn pushed(http: &CentralHttp, credential: &Token, port: u16) {
    for message in central_messages(http, credential) {
        assert_eq!(invalidate(port, message.expose()), 204);
    }
}

/// The synthetic player's sign-in, with the password its move brought to
/// Central. Central allows an account eight a minute, and the steps before a
/// caller may have spent them, so a busy answer waits.
fn synthetic_signed_in(
    http: &CentralHttp,
    identity: baylee_account::wire::Identity,
) -> sdk::Session {
    for _ in 0..18 {
        match sdk::login(http, identity, PASSWORD.into()) {
            Ok(session) => return session,
            Err(baylee_account::Error::Busy) => {
                std::thread::sleep(Duration::from_secs(5));
            }
            Err(error) => panic!("the synthetic player's sign-in: {error}"),
        }
    }
    panic!("the synthetic player's sign-in stayed busy for 90 seconds");
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

/// Central's grants: its `ops/grants.sql` at the pinned revision, statement
/// for statement, the legacy store's role included.
const GRANTS: &str = include_str!("central/runtime-grants.sql");

/// The password every fixture account keeps: the official gateway's own
/// hash of it is `common::LEGACY_HASH`.
const PASSWORD: &str = "synthetic legacy password";

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
        operator.execute_unprepared(GRANTS).await.unwrap();
    });
    let store = runtime
        .block_on(AccountStore::connect_dev(database(
            "central_runtime",
            "CENTRAL_TEST_RUNTIME_PASSWORD",
        )))
        .unwrap();
    let legacy = runtime
        .block_on(LegacyStore::connect_dev(database(
            "central_legacy",
            "CENTRAL_TEST_LEGACY_PASSWORD",
        )))
        .unwrap();
    let served = serve(&runtime, &store, &legacy, "central", None);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (gateway, endpoint, origin) = (&served.gateway, &served.endpoint, &served.origin);
        run(
            gateway,
            endpoint,
            origin,
            &runtime,
            &operator,
            &served.bridge,
        );
        renewal::run(
            gateway,
            endpoint,
            origin,
            &runtime,
            &operator,
            &served.bridge,
        );
        fresh::run(&runtime, &served.service, &operator, endpoint, gateway);
        pairing::run(&runtime, &served.service, &operator, endpoint, gateway);
        registered::run(&runtime, &operator, endpoint);
        terms::run(&runtime, &operator, endpoint);
        aliases::run(gateway, endpoint, &served.bridge);
        revocation::run(&runtime, gateway, endpoint, origin, &served.bridge);
        official::run(&runtime, &operator, endpoint, &served.bridge, origin);
        personal::run(&runtime, &operator, endpoint);
        token::run(&runtime, &store, &operator, endpoint);
        held::run(&runtime, &store, &legacy, &operator);
        ui_fixture(endpoint, origin);
    }));
    served.close(&runtime);
    runtime.block_on(async {
        legacy.close().await.unwrap();
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
    legacy: &LegacyStore,
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
            token_keys(),
        )
        .unwrap()
        .with_aliases(vec![aliases::ALIAS.into()])
        .unwrap()
        .with_terms(terms)
        .with_legacy(legacy.clone()),
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
    common::central_ready(gateway.port);
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

/// The cutover for one gateway (`docs/central-login.md` §"Cutover"): the
/// release's migration ends every password session, and the gateway's own
/// `legacy export` goes into Central's legacy store, as the operator pipes it
/// into `central-service --legacy-import-stdin`. Answers how many moved in.
fn cutover(
    gateway: &common::Gateway,
    runtime: &Runtime,
    operator: &sea_orm::DatabaseConnection,
) -> u64 {
    // What `m20261030_000026_end_password_sessions` did at this gateway's
    // start, for the accounts made since.
    gateway.sql(
        "DELETE FROM session_token WHERE account_id IN
             (SELECT id FROM account WHERE NOT guest AND NOT central_only)",
    );
    let exported = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        .args(["legacy", "export"])
        .env("DATABASE_URL", gateway.database_url())
        .stdin(std::process::Stdio::null())
        .output()
        .expect("the gateway's export runs");
    assert!(
        exported.status.success(),
        "{}",
        String::from_utf8_lossy(&exported.stderr)
    );
    let said = String::from_utf8_lossy(&exported.stderr);
    assert!(said.starts_with("legacy export: "), "{said}");
    let document: serde_json::Value = serde_json::from_slice(&exported.stdout).unwrap();
    assert_eq!(document["format"], "baylee-legacy-accounts/1");
    let bytes = |value: &serde_json::Value| -> Vec<u8> {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|byte| u8::try_from(byte.as_u64().unwrap()).unwrap())
            .collect()
    };
    let text = |value: &serde_json::Value| value.as_str().map(str::to_owned);
    let rows: Vec<LegacyRow> = document["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| LegacyRow {
            legacy_id: text(&row["legacy_id"]).unwrap(),
            username: text(&row["username"]).unwrap(),
            display_name: text(&row["display_name"]),
            tag: row["tag"].as_u64().map(|tag| u32::try_from(tag).unwrap()),
            salt: bytes(&row["password"]["salt"]).try_into().unwrap(),
            hash: bytes(&row["password"]["hash"]).try_into().unwrap(),
            terms_version: text(&row["terms_version"]),
            must_change_password: row["must_change_password"].as_bool().unwrap(),
            ban: row["ban"].as_object().map(|ban| ban["until"].as_i64()),
            snapshot: bytes(&row["snapshot"]),
        })
        .collect();
    let count = document["count"].as_u64().unwrap();
    let digest: [u8; 32] = bytes(&document["digest"]).try_into().unwrap();
    let imported = runtime
        .block_on(central_postgres::import_legacy(
            operator, rows, count, digest,
        ))
        .unwrap();
    assert_eq!((imported.count, imported.digest), (count, digest));
    count
}

fn run(
    gateway: &common::Gateway,
    endpoint: &Endpoint,
    origin: &str,
    runtime: &Runtime,
    operator: &sea_orm::DatabaseConnection,
    bridge: &Token,
) {
    // An account from before the cutover, with game data and settings.
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
    let id = gateway.text("SELECT id::text FROM account WHERE username_key = 'synthetic'");
    let tag = gateway.scalar("SELECT tag::bigint FROM account WHERE username_key = 'synthetic'");
    assert_eq!(cutover(gateway, runtime, operator), 1);
    // No password goes to the gateway any more, and its old session ended.
    assert_eq!(
        common::http(gateway.port, "GET", "/settings", Some(&old_token), "").0,
        401
    );
    for path in [
        "/auth/login",
        "/auth/register",
        "/auth/password",
        "/auth/central/migrate",
        "/auth/central/link",
    ] {
        let body = format!(r#"{{"username":"synthetic","password":"{PASSWORD}"}}"#);
        assert_eq!(
            common::http(gateway.port, "POST", path, None, &body).0,
            404,
            "{path} is gone"
        );
    }
    // The client's sign-in: the password goes to Central once, and the move
    // closes there.
    let account = Account::login(
        endpoint.clone(),
        "synthetic".into(),
        PASSWORD.into(),
        vec![origin.into()],
    )
    .unwrap();
    let moved = account.data().legacy.as_ref().unwrap();
    assert!(moved.complete && moved.account_id == id && moved.gateway == origin);
    assert_eq!(account.data().preferences["future"]["a"], 1);
    assert_eq!(
        runtime
            .block_on(operator.query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Postgres,
                "SELECT count(*) AS rows FROM central_legacy.accounts",
            )))
            .unwrap()
            .unwrap()
            .try_get::<i64>("", "rows")
            .unwrap(),
        0,
        "the closed move's row is gone from the legacy store"
    );
    let expected = AccountData::read(&account.data().write().unwrap()).unwrap();
    // The gateway links the account at its first admission, keeps its game
    // data, name and tag, and retires the password.
    assert_eq!(
        gateway.scalar("SELECT count(*) FROM central_account_link WHERE central_id IS NOT NULL"),
        0
    );
    let admitted = account.admit(origin).unwrap();
    assert_eq!(
        gateway.scalar(&format!(
            "SELECT count(*) FROM account a JOIN central_account_link c ON c.account_id = a.id
             WHERE a.id = '{id}'::uuid AND a.central_only AND a.password_hash IS NULL
                 AND c.central_id IS NOT NULL AND a.tag = {tag} AND a.display_name = 'Synthetic'"
        )),
        1
    );
    let (status, settings) =
        common::http(gateway.port, "GET", "/settings", Some(&admitted.token), "");
    assert_eq!(status, 200, "{settings}");
    assert!(settings.contains("future"), "{settings}");
    account.logout().unwrap();
    // A second move of the same account finds nothing: it signs in through
    // OPAQUE, and the store has no row for it.
    assert!(
        sdk::legacy_login(
            &CentralHttp::new(endpoint.clone()),
            "synthetic".into(),
            PASSWORD
        )
        .is_err()
    );
    restored_account(endpoint, origin, gateway.port, &expected, bridge);
}

fn restored_account(
    endpoint: &Endpoint,
    origin: &str,
    port: u16,
    expected: &AccountData,
    bridge: &Token,
) {
    let mut first = Account::login(
        endpoint.clone(),
        "synthetic".into(),
        PASSWORD.into(),
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
        PASSWORD.into(),
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
    pushed(&CentralHttp::new(endpoint.clone()), bridge, port);
    assert_eq!(
        common::http(port, "GET", "/me", Some(&old_gateway_token), "").0,
        401,
        "a successful new Central login ends the session before it, at every gateway"
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
    pushed(&CentralHttp::new(endpoint.clone()), bridge, port);
    assert_eq!(
        common::http(port, "GET", "/me", Some(token), "").0,
        401,
        "central logout cannot leave a gateway session authorized"
    );
}
