//! The official gateway by its former address (`docs/central-login.md`
//! §"The official gateway's aliases"): an installed client asks Central for a
//! token for the origin it knows, and the official gateway, reached under it,
//! takes that token as its own. A token for another origin, even one Central
//! issues, is not for this gateway. Synthetic data only.
use super::{CentralHttp, Endpoint, GatewayPurpose, Token, common, sdk};
use baylee_account::{SignedToken, gateway::GatewayMetadata, wire::Discovery};

/// The official gateway's alias in this fixture: Central takes it as the
/// bridge's (`IdentityService::with_aliases` in `serve`), and the requests
/// below reach the gateway at its own port, as the old host's do live.
pub(super) const ALIAS: &str = "http://127.0.0.1:45131";

/// A registered gateway's origin, which is not the official gateway's.
const REGISTERED: &str = "http://127.0.0.1:45133";

fn login(port: u16, token: &SignedToken) -> (u16, String) {
    let body = super::sign_in_body(token);
    common::http(port, "POST", "/auth/central/login", None, &body)
}

pub(super) fn run(gateway: &common::Gateway, endpoint: &Endpoint, bridge: &Token) {
    let http = CentralHttp::new(endpoint.clone());
    let Discovery::Central { identity } = sdk::discover(&http, "synthetic".into()).unwrap() else {
        panic!("completed migration must be discoverable");
    };
    let mut session = super::synthetic_signed_in(&http, identity);
    session.confirm_restored(&http).unwrap();

    let token = super::gateway_token(&session, &http, ALIAS, GatewayPurpose::Admit);
    let (status, body) = login(gateway.port, &token);
    assert_eq!(status, 200, "a token for the alias admits: {body}");
    let response: serde_json::Value = serde_json::from_str(&body).unwrap();
    let bearer = response["token"].as_str().unwrap().to_owned();
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&bearer), "").0,
        200,
        "the alias session holds"
    );

    let metadata = GatewayMetadata {
        name: "Another synthetic gateway".into(),
        version: "fixture".into(),
        website: format!("{REGISTERED}/"),
    };
    session
        .register_gateway(&http, REGISTERED.into(), metadata)
        .unwrap();
    let foreign = super::gateway_token(&session, &http, REGISTERED, GatewayPurpose::Admit);
    let (status, body) = login(gateway.port, &foreign);
    assert_eq!(
        status, 401,
        "another gateway's token is refused here: {body}"
    );
    assert!(
        session
            .gateway_token(
                &http,
                "http://127.0.0.1:45135".into(),
                GatewayPurpose::Admit
            )
            .is_err(),
        "Central issues no token for an origin nobody holds"
    );

    // Signing out at Central reaches this gateway, and the session ends.
    session.logout(&http).unwrap();
    super::pushed(&http, bridge, gateway.port);
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&bearer), "").0,
        401,
        "signing out at Central ends the alias session"
    );
}
