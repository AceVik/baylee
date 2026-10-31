//! Real registered-gateway admission, live revocation and separated admin authority.
use super::{CentralHttp, Endpoint, common, config_file, sdk};
use baylee_account::wire::TicketPurpose;
use baylee_account::{gateway::GatewayMetadata, wire::Discovery};
use futures_util::StreamExt as _;
use sea_orm::{ConnectionTrait as _, DatabaseConnection, DbBackend, Statement};
use tokio::runtime::Runtime;
#[path = "seat.rs"]
mod seat;

fn admit(
    port: u16,
    session: &sdk::Session,
    http: &CentralHttp,
    origin: &str,
    admin: bool,
) -> String {
    let ticket = session
        .gateway_capability(
            http,
            origin.into(),
            if admin {
                TicketPurpose::Admin
            } else {
                TicketPurpose::Admit
            },
        )
        .unwrap();
    let body = serde_json::to_string(&serde_json::json!({"ticket":ticket})).unwrap();
    let path = if admin {
        "/auth/central/admin"
    } else {
        "/auth/central/login"
    };
    let (status, body) = common::http(port, "POST", path, None, &body);
    assert_eq!(status, 200, "registered admission: {body}");
    let response: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(response.get("lease").is_none());
    assert!(response["username"].is_null());
    response["token"].as_str().unwrap().into()
}

pub(super) fn run(runtime: &Runtime, operator: &DatabaseConnection, endpoint: &Endpoint) {
    let http = CentralHttp::new(endpoint.clone());
    let Discovery::Central { identity } = sdk::discover(&http, "synthetic".into()).unwrap() else {
        panic!("completed migration must be discoverable");
    };
    let mut owner = sdk::login(&http, identity, super::PASSWORD.into()).unwrap();
    owner.confirm_restored(&http).unwrap();
    // The origin is an operator-approved synthetic audience. Requests below address
    // the real adapter port directly so the fixture never needs to reserve/release
    // a port before the process binds, nor route through an external reverse proxy.
    let origin = "http://127.0.0.1:45123";
    let metadata = GatewayMetadata {
        name: "Registered synthetic gateway".into(),
        version: "fixture".into(),
        website: format!("{origin}/"),
    };
    let registered = owner
        .register_gateway(&http, origin.into(), metadata.clone())
        .unwrap();
    // Approval is for the public list only (Central #51): a registered gateway
    // admits players while it waits, but the list does not show it yet.
    assert!(
        owner.gateway_ticket(&http, origin.into()).is_ok(),
        "a gateway awaiting approval admits players"
    );
    assert!(
        !sdk::gateway_directory(&http, None)
            .unwrap()
            .iter()
            .any(|entry| entry.origin == origin),
        "registration alone does not list an origin"
    );
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
    assert!(
        owner
            .gateway_capability(&http, origin.into(), TicketPurpose::Admin)
            .is_err(),
        "ownership does not silently grant gateway admin membership"
    );
    assert_eq!(
        owner
            .set_gateway_admins(&http, registered.gateway, 1, vec![identity.account])
            .unwrap(),
        2
    );
    let config = config_file(&serde_json::json!({
        "endpoint":endpoint.origin(), "origin":origin, "credential":registered.credential,
        "heartbeat":registered.heartbeat, "metadata":metadata,
    }));
    let admin_file = super::BridgeFile(std::env::temp_dir().join(format!(
        "baylee-central-admin-{}.port",
        uuid::Uuid::now_v7()
    )));
    let gateway = common::spawn_gateway_with(
        "registered",
        &[
            (
                "BAYLEE_CENTRAL_CONFIG_FILE",
                config.0.to_string_lossy().into_owned(),
            ),
            ("BAYLEE_REGISTRATION", "open".into()),
            (
                "BAYLEE_ADMIN_TOKEN",
                "synthetic-bootstrap-0123456789abcdef0123456789".into(),
            ),
            ("BAYLEE_ADMIN_BIND", "127.0.0.1:0".into()),
            (
                "BAYLEE_ADMIN_PORT_FILE",
                admin_file.0.to_string_lossy().into_owned(),
            ),
        ],
    );
    let admin_port: u16 = std::fs::read_to_string(&admin_file.0)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let player = check_access(
        &gateway,
        admin_port,
        &owner,
        &http,
        origin,
        registered.gateway,
    );
    check_revocation(&gateway, runtime, &owner, &http, origin, identity, &player);
    drop(gateway);
    drop(admin_file);
    drop(config);
}

fn check_access(
    gateway: &common::Gateway,
    admin_port: u16,
    owner: &sdk::Session,
    http: &CentralHttp,
    origin: &str,
    registered: [u8; 16],
) -> String {
    common::login(gateway.port, "synthetic", "LocalName");
    let player = admit(gateway.port, owner, http, origin, false);
    assert_eq!(
        gateway.scalar("SELECT count(*) FROM account WHERE NOT guest"),
        2,
        "Central identities never link by matching login names"
    );
    assert_eq!(gateway.scalar("SELECT count(*) FROM account WHERE central_only AND username IS NULL AND password_hash IS NULL"), 1);
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&player), "").0,
        200
    );
    concurrent_lease_checks(gateway.port, &player);
    assert_eq!(
        common::http(admin_port, "GET", "/admin/stats", Some(&player), "").0,
        401
    );
    let admin = admit(gateway.port, owner, http, origin, true);
    assert_eq!(
        common::http(admin_port, "GET", "/admin/stats", Some(&admin), "").0,
        200
    );
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&admin), "").0,
        401,
        "administration and gameplay bearers cannot substitute for each other"
    );
    assert_eq!(
        owner
            .set_gateway_admins(http, registered, 2, Vec::new())
            .unwrap(),
        3
    );
    assert_eq!(
        common::http(admin_port, "GET", "/admin/stats", Some(&admin), "").0,
        401,
        "removing an admin immediately invalidates its existing gateway bearer"
    );
    let listing = sdk::gateway_directory(http, None).unwrap();
    assert!(
        listing.iter().any(|item| item.id == registered
            && item.online
            && item.metadata.version == env!("CARGO_PKG_VERSION")),
        "the gateway publishes its actual version using the separate heartbeat"
    );
    gateway.sql("UPDATE account SET banned_at = clock_timestamp(), ban_reason = 'synthetic ban', banned_by = 'fixture' WHERE central_only");
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&player), "").0,
        403,
        "Central authorization never bypasses local bans"
    );
    gateway.sql("UPDATE account SET banned_at = NULL, ban_reason = NULL, banned_by = NULL WHERE central_only");
    player
}

fn concurrent_lease_checks(port: u16, token: &str) {
    let ready = std::sync::Barrier::new(12);
    std::thread::scope(|scope| {
        let ready = &ready;
        let checks: Vec<_> = (0..12)
            .map(|_| {
                scope.spawn(move || {
                    ready.wait();
                    common::http(port, "GET", "/me", Some(token), "").0
                })
            })
            .collect();
        for check in checks {
            assert_eq!(
                check.join().unwrap(),
                200,
                "a bounded simultaneous recheck burst must queue, not revoke healthy sessions"
            );
        }
    });
}

fn check_revocation(
    gateway: &common::Gateway,
    runtime: &Runtime,
    owner: &sdk::Session,
    http: &CentralHttp,
    origin: &str,
    identity: baylee_account::wire::Identity,
    player: &str,
) {
    let mut active = seat::LiveSeat::open(runtime, gateway, player);
    let game_id = active.game.clone();
    let seat_token = active.token.clone();
    let seat_body = serde_json::json!({"socket":"seat", "game":game_id}).to_string();
    assert_eq!(
        common::ask_ticket(gateway.port, &seat_token, &seat_body).0,
        200
    );
    let lobby_url = common::lobby_url(gateway.port, player, "");
    let (mut socket, _) = runtime
        .block_on(tokio_tungstenite::connect_async(lobby_url))
        .unwrap();
    let mut newer = sdk::login(http, identity, super::PASSWORD.into()).unwrap();
    newer.confirm_restored(http).unwrap();
    // Do not wait for the old socket's periodic check first: a replacement
    // installation must attach while that old connection is still winding down.
    let fresh = admit(gateway.port, &newer, http, origin, false);
    let mut resumed = seat::ResumedSeat::claim(runtime, gateway, &fresh);
    assert_eq!(resumed.game, game_id);
    assert_ne!(resumed.token, seat_token);
    active.assert_revoked(runtime);
    resumed.assert_continues(runtime, &active.opening);
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(player), "").0,
        401
    );
    assert_eq!(
        common::ask_ticket(gateway.port, &seat_token, &seat_body).0,
        401,
        "a seat capability cannot outlive the Central session that minted it"
    );
    assert_eq!(
        common::http(
            gateway.port,
            "GET",
            &format!("/games/{game_id}/cosmetics"),
            Some(&seat_token),
            ""
        )
        .0,
        401
    );
    runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(8), async {
            while let Some(frame) = socket.next().await {
                match frame {
                    Ok(tokio_tungstenite::tungstenite::Message::Close(_)) | Err(_) => return,
                    Ok(_) => {}
                }
            }
        })
        .await
        .expect("old idle lobby socket must close after central replacement");
    });
    assert!(owner.check_active(http).is_err());
    assert_eq!(
        gateway.scalar("SELECT count(*) FROM account WHERE central_only"),
        1,
        "repeated admission reuses the immutable projection"
    );
    newer.logout(http).unwrap();
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&fresh), "").0,
        401
    );
    drop(socket);
}
