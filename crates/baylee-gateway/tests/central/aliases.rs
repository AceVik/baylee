//! The official gateway by its former address (`docs/central-login.md`
//! §"The official gateway's aliases"): an installed client asks Central for a
//! ticket for the origin it knows, and the official gateway, reached under
//! it, redeems that ticket with its bridge as its own, and the lease holds.
//! A ticket for another origin, even one Central issues, is not the bridge's.
//! Synthetic data only.
use super::{CentralHttp, Endpoint, Token, common, sdk};
use baylee_account::{gateway::GatewayMetadata, wire::Discovery};

/// The official gateway's alias in this fixture: Central takes it as the
/// bridge's (`IdentityService::with_aliases` in `serve`), and the requests
/// below reach the gateway at its own port, as the old host's do live.
pub(super) const ALIAS: &str = "http://127.0.0.1:45131";

/// A registered gateway's origin, which is not the official gateway's.
const REGISTERED: &str = "http://127.0.0.1:45133";

fn login(port: u16, ticket: &Token) -> (u16, String) {
    let body = serde_json::to_string(&serde_json::json!({ "ticket": ticket })).unwrap();
    common::http(port, "POST", "/auth/central/login", None, &body)
}

pub(super) fn run(gateway: &common::Gateway, endpoint: &Endpoint) {
    let http = CentralHttp::new(endpoint.clone());
    let Discovery::Central { identity } = sdk::discover(&http, "synthetic".into()).unwrap() else {
        panic!("completed migration must be discoverable");
    };
    let mut session = super::synthetic_signed_in(&http, identity);
    session.confirm_restored(&http).unwrap();

    let ticket = session.gateway_ticket(&http, ALIAS.into()).unwrap();
    let (status, body) = login(gateway.port, &ticket);
    assert_eq!(status, 200, "a ticket for the alias admits: {body}");
    let response: serde_json::Value = serde_json::from_str(&body).unwrap();
    let token = response["token"].as_str().unwrap().to_owned();
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&token), "").0,
        200,
        "the alias lease holds"
    );

    let metadata = GatewayMetadata {
        name: "Another synthetic gateway".into(),
        version: "fixture".into(),
        website: format!("{REGISTERED}/"),
    };
    session
        .register_gateway(&http, REGISTERED.into(), metadata)
        .unwrap();
    let foreign = session.gateway_ticket(&http, REGISTERED.into()).unwrap();
    let (status, body) = login(gateway.port, &foreign);
    assert!(
        matches!(status, 401 | 403),
        "another gateway's ticket is refused here: {status} {body}"
    );
    assert!(
        session
            .gateway_ticket(&http, "http://127.0.0.1:45135".into())
            .is_err(),
        "Central issues no ticket for an origin nobody holds"
    );

    // The lease was checked with Central, not merely stored: it ends with
    // the Central session.
    session.logout(&http).unwrap();
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&token), "").0,
        401,
        "signing out at Central ends the alias lease"
    );
}
