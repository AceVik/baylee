//! A gateway linked by a token pasted into its console (baylee-central
//! `docs/gateway-registry.md` §"Gateway tokens", `docs/central-login.md`
//! §"Linking a gateway with a token"): no operator file; the owner registers
//! the origin and mints the token at Central, an administrator approves it,
//! the console takes the token, Central's players sign in there, its entry
//! is listed online, a revoked token shows as such, and removing it unlinks.
use super::{CentralHttp, Endpoint, TicketPurpose, common, sdk};
use baylee_account::{Token, owner, wire::Discovery};
use central_postgres::{AccountStore, Session};
use sea_orm::{ConnectionTrait as _, DatabaseConnection, DbBackend, Statement};
use tokio::runtime::Runtime;

/// The console's token in this test only.
const ADMIN_TOKEN: &str = "e2e-central-token-console-0123456789abcdef";

/// One console request as the feedback service sends it: status and body.
fn console(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {ADMIN_TOKEN}\r\nX-Baylee-Admin: viktor\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let status = raw
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap();
    let body = raw.split_once("\r\n\r\n").map_or("", |(_, b)| b).to_owned();
    (status, body)
}

fn paste(admin: u16, token: &str) -> (u16, String) {
    console(
        admin,
        "PUT",
        "/admin/central/token",
        &serde_json::json!({ "token": token }).to_string(),
    )
}

/// A gateway with a console and no operator file, talking to `endpoint`;
/// its console's port.
fn spawn(endpoint: &Endpoint) -> (common::Gateway, u16) {
    let port_file = std::env::temp_dir().join(format!(
        "baylee-central-token-console-{}.port",
        uuid::Uuid::now_v7()
    ));
    let gateway = common::spawn_gateway_with(
        "central_token",
        &[
            ("BAYLEE_ADMIN_TOKEN", ADMIN_TOKEN.into()),
            ("BAYLEE_ADMIN_BIND", "127.0.0.1:0".into()),
            (
                "BAYLEE_ADMIN_PORT_FILE",
                port_file.to_string_lossy().into_owned(),
            ),
            ("BAYLEE_CENTRAL_ENDPOINT", endpoint.origin().to_owned()),
            ("BAYLEE_BIND", "127.0.0.1".into()),
            ("BAYLEE_REGISTRATION", "open".into()),
            ("BAYLEE_GATEWAY_NAME", "Token table".into()),
        ],
    );
    let admin = std::fs::read_to_string(&port_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let _ = std::fs::remove_file(&port_file);
    (gateway, admin)
}

/// At Central: the owner registers `origin` on the website, an administrator
/// approves it (the operator's SQL stands in), the owner mints a token. The
/// owner's browser session, the token and the gateway's id.
fn minted(
    runtime: &Runtime,
    store: &AccountStore,
    operator: &DatabaseConnection,
    identity: baylee_account::wire::Identity,
    origin: &str,
) -> (Token, Token, [u8; 16]) {
    let token = Token::generate().unwrap();
    let browser = Token::generate().unwrap();
    let gateway = runtime.block_on(async {
        let credential = store.credential(identity).await.unwrap().unwrap();
        store
            .issue_session(&credential, Session::Browser(&browser))
            .await
            .unwrap();
        let id = store
            .add_gateway(&browser, origin, "Token table", true)
            .await
            .unwrap();
        operator
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO central_identity.gateway_approvals (gateway_id) VALUES ($1)",
                [uuid::Uuid::from_bytes(id).into()],
            ))
            .await
            .unwrap();
        let scopes = owner::GatewayScopes {
            sso: true,
            listing: true,
        };
        store
            .create_gateway_token(&browser, id, "e2e", scopes, 30, &token)
            .await
            .unwrap();
        id
    });
    (browser, token, gateway)
}

/// The console takes the real token, padded as a paste comes, and shows no
/// more of it than its end.
fn links(admin: u16, token: &Token, origin: &str, gateway: [u8; 16]) {
    let (status, _) = paste(admin, "bgt_nope");
    assert_eq!(status, 400);
    let unknown = owner::token_text(&Token::generate().unwrap());
    let (status, body) = paste(admin, &unknown);
    assert_eq!(status, 422, "{body}");
    let text = owner::token_text(token);
    let (status, body) = paste(admin, &format!(" {} ", text.as_str()));
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains(&text[4..]), "never the token: {body}");
    let linked: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(linked["source"], "token");
    assert_eq!(linked["token"]["last4"], &text[text.len() - 4..]);
    assert_eq!(linked["central"]["state"], "approved");
    assert_eq!(linked["central"]["origin"], origin);
    assert_eq!(
        linked["central"]["gateway"],
        uuid::Uuid::from_bytes(gateway).to_string()
    );
}

/// Listed online, under the gateway's own name, after the link's heartbeat.
fn listed(http: &CentralHttp, origin: &str) -> bool {
    (0..40).any(|_| {
        let online = sdk::gateway_directory(http, None)
            .unwrap_or_default()
            .iter()
            .any(|g| g.origin == origin && g.online && g.metadata.name == "Token table");
        if !online {
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        online
    })
}

pub(super) fn run(
    runtime: &Runtime,
    store: &AccountStore,
    operator: &DatabaseConnection,
    endpoint: &Endpoint,
) {
    let (gateway, admin) = spawn(endpoint);
    let origin = format!("http://127.0.0.1:{}", gateway.port);
    let (status, body) = console(admin, "GET", "/admin/central", "");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"source\":\"none\""), "{body}");
    let http = CentralHttp::new(endpoint.clone());
    let Discovery::Central { identity } = sdk::discover(&http, "synthetic".into()).unwrap() else {
        panic!("the synthetic account is at Central");
    };
    let mut player = super::synthetic_signed_in(&http, identity);
    // as the client does after its sign-in, before it asks for tickets
    player.confirm_restored(&http).unwrap();
    let (browser, token, id) = minted(runtime, store, operator, identity, &origin);
    links(admin, &token, &origin, id);
    let (_, config) = common::http(gateway.port, "GET", "/auth/config", None, "");
    assert!(config.contains("\"central\":true"), "{config}");
    // Central's player signs in there.
    let ticket = player
        .gateway_capability(&http, origin.clone(), TicketPurpose::Admit)
        .unwrap();
    let body = serde_json::json!({ "ticket": ticket }).to_string();
    let (status, body) = common::http(gateway.port, "POST", "/auth/central/login", None, &body);
    assert_eq!(status, 200, "a token-linked gateway admits: {body}");
    assert!(listed(&http, &origin), "listed online");
    // Revoked at Central: the console says so.
    runtime.block_on(async {
        let gateways = store.owner_gateways(&browser).await.unwrap();
        let e2e = gateways[0].tokens.iter().find(|t| t.name == "e2e").unwrap();
        store
            .revoke_own_gateway_token(&browser, e2e.id)
            .await
            .unwrap();
    });
    let (_, body) = console(admin, "GET", "/admin/central", "");
    assert!(body.contains("\"check\":\"rejected\""), "{body}");
    // Removing it unlinks.
    let (status, body) = console(admin, "DELETE", "/admin/central/token", "");
    assert_eq!(status, 204, "{body}");
    let (_, config) = common::http(gateway.port, "GET", "/auth/config", None, "");
    assert!(config.contains("\"central\":false"), "{config}");
    assert_eq!(console(admin, "DELETE", "/admin/central/token", "").0, 404);
    drop(gateway);
}
