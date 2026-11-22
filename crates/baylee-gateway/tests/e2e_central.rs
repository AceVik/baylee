//! Synthetic real gateway + Central + PostgreSQL migration, never production services.
#![forbid(unsafe_code)]
#[path = "central/aliases.rs"]
mod aliases;
#[path = "central/server.rs"]
mod server;
use server::{Served, serve};
#[path = "e2e_central/chat.rs"]
mod chat;
#[path = "e2e_central/private_decks.rs"]
mod private_decks;
#[path = "e2e_central/ranked.rs"]
mod ranked;
#[path = "central/tower_awards.rs"]
mod tower_awards;
#[path = "central/tower_native/mod.rs"]
mod tower_native;
use crate::common;
#[path = "central/challenges.rs"]
mod challenges;
#[path = "central/deck_import.rs"]
mod deck_import;
#[path = "central/demo.rs"]
mod demo;
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
#[path = "central/registration.rs"]
mod registration;
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
use central_http::DevelopmentServer;
use central_identity::TokenKeys;
use central_postgres::{AccountStore, DevConfig, LegacyRow, LegacyStore};
use sea_orm::ConnectionTrait;
use std::{io::Write, path::PathBuf, time::Duration};
use tokio::runtime::Runtime;

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

fn fixture_database(name: &str, role: &str, password: &str) -> DevConfig {
    assert_eq!(
        std::env::var("CENTRAL_TEST_FIXTURE").as_deref(),
        Ok("isolated-v1")
    );
    DevConfig::new(
        std::env::var("CENTRAL_TEST_PORT").unwrap().parse().unwrap(),
        role,
        std::env::var(password).unwrap(),
        name,
    )
    .unwrap()
}

/// Central's grants: its `ops/grants.sql` at the pinned revision, statement
/// for statement, the legacy store's role included.
const GRANTS: &str = include_str!("central/runtime-grants.sql");

/// The password every fixture account has at Central: [`LEGACY_HASH`] is
/// the official gateway's own hash of it.
const PASSWORD: &str = "synthetic legacy password";

/// The official gateway's own Argon2id hash of [`PASSWORD`], as the cutover
/// copied an account's into Central's legacy store. The gateway itself keeps
/// no hash any more (`docs/central-login.md` §"Cutover").
const LEGACY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$wittOwBy6Zxq/y0DGjZasA$fBlUSay3ZkJb5bDps9DdGXWgriHX9cjNYnp4ZXs80go";

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
    integration(false);
}

fn integration(ranked_only: bool) {
    let runtime = Runtime::new().unwrap();
    let name = if ranked_only {
        "central_ranked_test"
    } else {
        "central_native_test"
    };
    let operator = runtime
        .block_on(central_postgres::connect_dev(fixture_database(
            name,
            "central_migrator",
            "CENTRAL_TEST_MIGRATOR_PASSWORD",
        )))
        .unwrap();
    runtime.block_on(async {
        central_migration::apply(&operator).await.unwrap();
        operator.execute_unprepared(GRANTS).await.unwrap();
    });
    let store = runtime
        .block_on(AccountStore::connect_dev(fixture_database(
            name,
            "central_runtime",
            "CENTRAL_TEST_RUNTIME_PASSWORD",
        )))
        .unwrap();
    let legacy = runtime
        .block_on(LegacyStore::connect_dev(fixture_database(
            name,
            "central_legacy",
            "CENTRAL_TEST_LEGACY_PASSWORD",
        )))
        .unwrap();
    let served = serve(&runtime, &store, &legacy, "central", None);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if ranked_only {
            ranked::run(&runtime, &served, &store, &operator);
            return;
        }
        let (gateway, endpoint, origin) = (&served.gateway, &served.endpoint, &served.origin);
        if tower_awards::enabled() {
            tower_awards::run(&runtime, &served, &operator);
            return;
        }
        if served.tower.is_some() {
            tower_native::exercise(&runtime, &served);
            return;
        }
        if std::env::var("BAYLEE_CENTRAL_CHALLENGES_ONLY").as_deref() == Ok("1") {
            fresh::run(
                &runtime,
                &served.service,
                endpoint,
                gateway,
                &served.bridge,
                &operator,
            );
            if std::env::var("BAYLEE_CENTRAL_DEMO").as_deref() == Ok("1") {
                demo::run(&runtime, &served);
            }
            return;
        }
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
        fresh::run(
            &runtime,
            &served.service,
            endpoint,
            gateway,
            &served.bridge,
            &operator,
        );
        pairing::run(&runtime, &served.service, &operator, endpoint, gateway);
        registered::run(&runtime, &operator, endpoint);
        terms::run(&runtime, &operator, endpoint);
        aliases::run(gateway, endpoint, &served.bridge);
        revocation::run(&runtime, gateway, endpoint, origin, &served.bridge);
        official::run(&runtime, &operator, endpoint, &served.bridge, origin);
        personal::run(&runtime, &operator, endpoint);
        token::run(&runtime, &store, &operator, endpoint);
        held::run(&runtime, &store, &legacy, &operator);
        ui_fixture(&runtime, endpoint, gateway, origin);
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

// Optional bounded handoff to an isolated dev-control client. This fixture has
// only synthetic accounts; the ready file contains no credentials or tokens.
fn ui_fixture(runtime: &Runtime, endpoint: &Endpoint, host: &common::Gateway, gateway: &str) {
    struct Attached(tokio::task::JoinHandle<()>);
    impl Drop for Attached {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let Some(path) = std::env::var_os("BAYLEE_CENTRAL_UI_READY") else {
        return;
    };
    let path = PathBuf::from(path);
    let _agent = Attached(runtime.block_on(common::attach_agent(host)));
    std::fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "mode":"synthetic-only", "central":endpoint.origin(), "gateway":gateway,
            "username":"synthetic"
        }))
        .unwrap(),
    )
    .unwrap();
    let seconds = std::env::var("BAYLEE_CENTRAL_UI_HOLD_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(180)
        .min(7380);
    let deadline = std::time::Instant::now() + Duration::from_secs(seconds);
    while std::time::Instant::now() < deadline && !path.with_extension("stop").exists() {
        std::thread::sleep(Duration::from_millis(200));
    }
    std::fs::remove_file(path).unwrap();
}

/// What the cutover left for one gateway (`docs/central-login.md`
/// §"Cutover"): no password session, and every account from before it that
/// has not moved yet in Central's legacy store. The gateway's export tool is
/// gone with the cutover, so the fixture writes Central's rows itself, as the
/// import took them: the account's id, name and tag here, the fixture's
/// password hash, and its preferences and decks as the source snapshot.
/// Answers how many accounts the store took.
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
    // The snapshot has the shape the gateway takes the link's digest over
    // (`baylee_db::central`): the preferences document, and each deck with
    // its version history.
    let accounts: serde_json::Value = serde_json::from_str(&gateway.text(
        "SELECT COALESCE(jsonb_agg(jsonb_build_object(
             'legacy_id', a.id::text, 'username', a.username_key,
             'display_name', a.display_name, 'tag', a.tag,
             'terms_version', a.terms_version,
             'snapshot', jsonb_build_object(
                 'preferences', COALESCE((SELECT doc FROM client_settings s
                     WHERE s.account_id = a.id), '{}'::jsonb),
                 'decks', COALESCE((SELECT jsonb_agg(to_jsonb(d) || jsonb_build_object('history',
                     COALESCE((SELECT jsonb_agg(to_jsonb(v) ORDER BY v.version)
                         FROM deck_version v WHERE v.deck_id = d.id), '[]'::jsonb)) ORDER BY d.id)
                     FROM deck d WHERE d.account_id = a.id), '[]'::jsonb))::text)
             ORDER BY a.id), '[]'::jsonb)::text
         FROM account a
         WHERE NOT a.guest AND NOT a.central_only AND a.username_key IS NOT NULL
             AND NOT EXISTS (SELECT 1 FROM central_account_link c
                 WHERE c.account_id = a.id AND c.central_id IS NOT NULL)",
    ))
    .unwrap();
    let (salt, hash) = legacy_password();
    let text = |value: &serde_json::Value| value.as_str().map(str::to_owned);
    let mut rows: Vec<LegacyRow> = accounts
        .as_array()
        .unwrap()
        .iter()
        .map(|row| LegacyRow {
            legacy_id: text(&row["legacy_id"]).unwrap(),
            username: text(&row["username"]).unwrap(),
            // Central keeps a name only when it keeps the rule.
            display_name: text(&row["display_name"])
                .filter(|name| central_api::profile::valid_display_name(name)),
            tag: row["tag"].as_u64().map(|tag| u32::try_from(tag).unwrap()),
            salt,
            hash,
            terms_version: text(&row["terms_version"]),
            must_change_password: false,
            ban: None,
            snapshot: text(&row["snapshot"]).unwrap().into_bytes(),
        })
        .collect();
    rows.sort_by(|a, b| a.legacy_id.as_bytes().cmp(b.legacy_id.as_bytes()));
    let count = rows.len() as u64;
    let digest = central_postgres::legacy_digest(&rows);
    let imported = runtime
        .block_on(central_postgres::import_legacy(
            operator, rows, count, digest,
        ))
        .unwrap();
    assert_eq!((imported.count, imported.digest), (count, digest));
    count
}

/// The salt and output of [`LEGACY_HASH`], as Central's legacy store keeps a
/// password: Argon2id under the parameters the official gateway hashed with.
fn legacy_password() -> ([u8; 16], [u8; 32]) {
    use base64::Engine as _;
    let mut fields = LEGACY_HASH
        .strip_prefix("$argon2id$v=19$m=19456,t=2,p=1$")
        .expect("the pinned parameters")
        .split('$');
    let mut decode = || {
        base64::engine::general_purpose::STANDARD_NO_PAD
            .decode(fields.next().expect("a salt and an output"))
            .expect("PHC base64")
    };
    let salt = decode().try_into().expect("a 16-byte salt");
    let hash = decode().try_into().expect("a 32-byte output");
    (salt, hash)
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
    private_decks::seed_gateway(gateway.port, &old_token);
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
    // The gateway links the account at its first admission and keeps its
    // game data, name and tag.
    assert_eq!(
        gateway.scalar("SELECT count(*) FROM central_account_link WHERE central_id IS NOT NULL"),
        0
    );
    let admitted = account.admit(origin).unwrap();
    assert_eq!(
        gateway.scalar(&format!(
            "SELECT count(*) FROM account a JOIN central_account_link c ON c.account_id = a.id
             WHERE a.id = '{id}'::uuid AND a.central_only
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
    let deck = private_decks::first_device(&mut first);
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
    private_decks::second_device(&mut first, &mut second, deck);
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
