//! A registered gateway's terms of use over real Central, gateway and
//! PostgreSQL (`docs/protocol.md` §"Terms of use (WG-1)"): the gateway
//! publishes its default terms to Central, refuses a Central-bound session
//! whose account has not accepted them, lets it go on once it accepted them
//! at Central, and asks again after an admin published a new version.
use super::{CentralHttp, Endpoint, common, config_file, sdk};
use baylee_account::wire::TicketPurpose;
use baylee_account::{
    gateway::GatewayMetadata,
    terms::{TermsSubject, TermsVersion},
    wire::Discovery,
};
use sea_orm::{ConnectionTrait as _, DatabaseConnection, DbBackend, Statement};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

const ORIGIN: &str = "http://127.0.0.1:45124";
const CONSOLE: &str = "synthetic-terms-console-0123456789abcdef01234";

fn admit(port: u16, session: &sdk::Session, http: &CentralHttp) -> serde_json::Value {
    signed_in(
        port,
        session,
        http,
        TicketPurpose::Admit,
        "/auth/central/login",
    )
}

fn signed_in(
    port: u16,
    session: &sdk::Session,
    http: &CentralHttp,
    purpose: TicketPurpose,
    path: &str,
) -> serde_json::Value {
    let ticket = session
        .gateway_capability(http, ORIGIN.into(), purpose)
        .unwrap();
    let body = serde_json::to_string(&serde_json::json!({"ticket":ticket})).unwrap();
    let (status, body) = common::http(port, "POST", path, None, &body);
    assert_eq!(status, 200, "admission: {body}");
    serde_json::from_str(&body).unwrap()
}

/// What the gateway serves as its terms in force: version and digest.
fn served(port: u16) -> TermsVersion {
    let (status, body) = common::http(port, "GET", "/terms?lang=de", None, "");
    assert_eq!(status, 200, "{body}");
    let terms: serde_json::Value = serde_json::from_str(&body).unwrap();
    let hex = terms["digest"].as_str().unwrap();
    let mut digest = [0; 32];
    for (i, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
    }
    TermsVersion {
        version: terms["version"].as_str().unwrap().into(),
        digest,
    }
}

/// Until Central holds `version` as the gateway's current terms.
fn held_at_central(http: &CentralHttp, version: &TermsVersion) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if sdk::gateway_terms(http, ORIGIN)
            .unwrap()
            .is_some_and(|document| document.version == *version)
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the gateway never published its terms"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// `GET /me`'s status, and whose terms the refusal names.
fn me(port: u16, token: &str) -> (u16, Option<String>) {
    let (status, body) = common::http(port, "GET", "/me", Some(token), "");
    let refused = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|body| body["terms_required"].as_str().map(str::to_owned));
    (status, refused)
}

pub(super) fn run(runtime: &Runtime, operator: &DatabaseConnection, endpoint: &Endpoint) {
    let http = CentralHttp::new(endpoint.clone());
    let Discovery::Central { identity } = sdk::discover(&http, "synthetic".into()).unwrap() else {
        panic!("a migrated account");
    };
    let mut owner = sdk::login(&http, identity, super::PASSWORD.into()).unwrap();
    owner.confirm_restored(&http).unwrap();
    let metadata = GatewayMetadata {
        name: "Synthetic gateway with terms".into(),
        version: "fixture".into(),
        website: format!("{ORIGIN}/"),
    };
    let registered = approved(
        runtime,
        operator,
        &owner,
        &http,
        identity.account,
        &metadata,
    );
    let config = config_file(&serde_json::json!({
        "endpoint":endpoint.origin(), "origin":ORIGIN, "credential":registered.credential,
        "heartbeat":registered.heartbeat, "metadata":metadata,
    }));
    let admin_file = super::BridgeFile(std::env::temp_dir().join(format!(
        "baylee-central-terms-admin-{}.port",
        uuid::Uuid::now_v7()
    )));
    let gateway = common::spawn_gateway_with(
        "central-terms",
        &[
            (
                "BAYLEE_CENTRAL_CONFIG_FILE",
                config.0.to_string_lossy().into_owned(),
            ),
            ("BAYLEE_REGISTRATION", "open".into()),
            // Blank: the default terms every gateway ships.
            ("BAYLEE_TERMS_PATH", String::new()),
            ("BAYLEE_ADMIN_TOKEN", CONSOLE.into()),
            ("BAYLEE_ADMIN_BIND", "127.0.0.1:0".into()),
            (
                "BAYLEE_ADMIN_PORT_FILE",
                admin_file.0.to_string_lossy().into_owned(),
            ),
        ],
    );
    let console: u16 = std::fs::read_to_string(&admin_file.0)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let first = served(gateway.port);
    held_at_central(&http, &first);

    let answer = admit(gateway.port, &owner, &http);
    assert_eq!(answer["terms_stale"], true, "{answer}");
    assert!(answer.get("central_terms_stale").is_none(), "{answer}");
    let token = answer["token"].as_str().unwrap().to_owned();
    assert_eq!(me(gateway.port, &token), (403, Some("gateway".into())));
    let (status, body) = common::http(
        gateway.port,
        "POST",
        "/account/terms",
        Some(&token),
        &serde_json::json!({"version": first.version}).to_string(),
    );
    assert_eq!(status, 409, "a Central account accepts at Central: {body}");
    let mut forged = first.clone();
    forged.digest[0] ^= 1;
    assert!(
        owner
            .accept_terms(
                &http,
                TermsSubject::Gateway {
                    origin: ORIGIN.into()
                },
                &forged
            )
            .is_err(),
        "only the text the gateway published can be accepted"
    );
    owner
        .accept_terms(
            &http,
            TermsSubject::Gateway {
                origin: ORIGIN.into(),
            },
            &first,
        )
        .unwrap();
    assert_eq!(me(gateway.port, &token), (200, None), "accepted at Central");
    assert_eq!(admit(gateway.port, &owner, &http)["terms_stale"], false);

    republished(&gateway, console, &owner, &http);
    drop(gateway);
    drop(admin_file);
    drop(config);
}

/// An admin publishes a new version: Central holds it at once, and every
/// player is asked again.
fn republished(gateway: &common::Gateway, console: u16, owner: &sdk::Session, http: &CentralHttp) {
    let admin = signed_in(
        gateway.port,
        owner,
        http,
        TicketPurpose::Admin,
        "/auth/central/admin",
    )["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let (status, current) = common::http(console, "GET", "/admin/terms", Some(&admin), "");
    assert_eq!(status, 200, "{current}");
    let current: serde_json::Value = serde_json::from_str(&current).unwrap();
    assert_eq!(current["terms"]["source"], "default");
    assert_eq!(current["terms"]["at_central"], true);
    let (status, published) = common::http(
        console,
        "POST",
        "/admin/terms",
        Some(&admin),
        &serde_json::json!({
            "version": "fixture-2",
            "texts": [
                {"lang": "de", "markdown": "# Bedingungen\n\nNeu.\n"},
                {"lang": "en", "markdown": "# Terms\n\nNew.\n"},
            ],
        })
        .to_string(),
    );
    assert_eq!(status, 201, "{published}");
    let published: serde_json::Value = serde_json::from_str(&published).unwrap();
    assert_eq!(published["terms"]["source"], "published");
    assert_eq!(published["terms"]["at_central"], true, "{published}");
    let token = admit(gateway.port, owner, http)["token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(me(gateway.port, &token), (403, Some("gateway".into())));
    let second = served(gateway.port);
    assert_eq!(second.version, "fixture-2");
    owner
        .accept_terms(
            http,
            TermsSubject::Gateway {
                origin: ORIGIN.into(),
            },
            &second,
        )
        .unwrap();
    assert_eq!(me(gateway.port, &token), (200, None));
}

/// Registered, approved by the operator, and the owner one of its admins.
fn approved(
    runtime: &Runtime,
    operator: &DatabaseConnection,
    owner: &sdk::Session,
    http: &CentralHttp,
    account: [u8; 16],
    metadata: &GatewayMetadata,
) -> sdk::GatewayRegistration {
    let registered = owner
        .register_gateway(http, ORIGIN.into(), metadata.clone())
        .unwrap();
    owner
        .set_gateway_admins(http, registered.gateway, 1, vec![account])
        .unwrap();
    runtime.block_on(async {
        operator
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO central_identity.gateway_approvals (gateway_id) VALUES ($1)",
                [uuid::Uuid::from_bytes(registered.gateway).into()],
            ))
            .await
            .unwrap();
    });
    registered
}
