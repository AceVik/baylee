//! A session outlives its bearers and its gateway token (`docs/central-login.md`
//! §"Sessions"): a bearer renews without Central, a later gateway token carries
//! the session on, and neither changes what a socket or a chair holds on to.
use super::{Account, Endpoint, PASSWORD, Token, common, cutover};
use sea_orm::DatabaseConnection;
use tokio::runtime::Runtime;

const USERNAME: &str = "renewing";

fn bearer_of(body: &str) -> String {
    common::json_field(body, "token").to_owned()
}

pub(super) fn run(
    gateway: &common::Gateway,
    endpoint: &Endpoint,
    origin: &str,
    runtime: &Runtime,
    operator: &DatabaseConnection,
    bridge: &Token,
) {
    common::login(gateway.port, USERNAME, "Renewing");
    assert_eq!(cutover(gateway, runtime, operator), 1);
    let account = Account::login(
        endpoint.clone(),
        USERNAME.into(),
        PASSWORD.into(),
        Vec::new(),
    )
    .unwrap();
    let admitted = account.admit(origin).unwrap();
    let first = admitted.token.clone();
    let renew_in = admitted.renew_in.unwrap();
    assert!((44..=108).contains(&renew_in), "{renew_in}");
    assert_eq!(common::http(gateway.port, "GET", "/me", Some(&first), "").0, 200);

    // Renewal needs no Central, and only the current bearer may ask.
    let (status, body) =
        common::http(gateway.port, "POST", "/auth/session/renew", Some(&first), "");
    assert_eq!(status, 200, "{body}");
    let second = bearer_of(&body);
    assert_ne!(first, second);
    assert_eq!(common::http(gateway.port, "GET", "/me", Some(&second), "").0, 200);
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&first), "").0,
        200,
        "the old bearer serves what was in flight"
    );
    assert_eq!(
        common::http(gateway.port, "POST", "/auth/session/renew", Some(&first), "").0,
        401,
        "a used bearer cannot renew, so a copy of it cannot fork the session"
    );

    // A later gateway token carries the session on, with a new bearer: the way
    // back for a client whose renewal was lost.
    let refreshed = account.refresh_gateway(origin).unwrap();
    assert_ne!(refreshed.token, second);
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&refreshed.token), "").0,
        200
    );
    assert_eq!(
        common::http(gateway.port, "POST", "/auth/session/renew", Some(&second), "").0,
        401,
        "a refresh retires the bearer before it too"
    );

    // Signing out still ends it at once, renewed or not.
    let bearer = refreshed.token.clone();
    account.logout().unwrap();
    let http = super::CentralHttp::new(endpoint.clone());
    super::pushed(&http, bridge, gateway.port);
    assert_eq!(common::http(gateway.port, "GET", "/me", Some(&bearer), "").0, 401);
}
