//! A revocation, end to end, against a real Central (`docs/central-login.md`
//! §"Sessions"): sign in, play a frame, revoke, the socket closes saying why,
//! and nothing further reaches the engine. What Central's push would carry is
//! Central's own signed message; a second run has no push at all, and the
//! gateway's catch-up has to learn on its own.
use super::{CentralHttp, Endpoint, GatewayPurpose, Runtime, Token, common, seat, sdk};
use baylee_account::wire::Discovery;
use std::time::Duration;

/// Longer than one round of the gateway's catch-up.
const CATCH_UP: Duration = Duration::from_secs(40);

fn signed_in(http: &CentralHttp) -> sdk::Session {
    let Discovery::Central { identity } = sdk::discover(http, "synthetic".into()).unwrap() else {
        panic!("completed migration must be discoverable");
    };
    let mut session = super::synthetic_signed_in(http, identity);
    session.confirm_restored(http).unwrap();
    session
}

/// A sign-in, answering the bearer, and the token kept for trying again.
fn admit(
    port: u16,
    session: &sdk::Session,
    http: &CentralHttp,
    origin: &str,
) -> (String, baylee_account::SignedToken) {
    let token = super::gateway_token(session, http, origin, GatewayPurpose::Admit);
    let body = super::sign_in_body(&token);
    let (status, answer) = common::http(port, "POST", "/auth/central/login", None, &body);
    assert_eq!(status, 200, "{answer}");
    (common::json_field(&answer, "token").to_owned(), token)
}

fn pushed_run(
    runtime: &Runtime,
    gateway: &common::Gateway,
    endpoint: &Endpoint,
    origin: &str,
    bridge: &Token,
) {
    let http = CentralHttp::new(endpoint.clone());
    let session = signed_in(&http);
    let (bearer, token) = admit(gateway.port, &session, &http, origin);

    // `LiveSeat::open` proves a frame goes through to the engine and back.
    let mut seat = seat::LiveSeat::open(runtime, gateway, &bearer);

    // A renewal is no end: the same session, and its socket stands.
    let renew = common::http(gateway.port, "POST", "/auth/session/renew", Some(&bearer), "");
    assert_eq!(renew.0, 200, "{}", renew.1);
    seat.assert_live(runtime);

    // Central ends the session: the token still has most of an hour to run.
    session.logout(&http).unwrap();
    let messages = super::central_messages(&http, bridge);
    let message = messages.last().expect("the sign-out is numbered");

    // Nothing but Central's message, whole, is taken.
    let forged = message.expose().replace('.', "x");
    assert_eq!(super::invalidate(gateway.port, &forged), 401);
    let (body, signature) = message.expose().rsplit_once('.').unwrap();
    let flipped = if signature.starts_with('A') { 'B' } else { 'A' };
    let tampered = format!("{body}.{flipped}{}", &signature[1..]);
    assert_eq!(super::invalidate(gateway.port, &tampered), 401);
    let (status, _) = common::http_bytes(
        gateway.port,
        "POST",
        "/auth/central/invalidate",
        None,
        "text/plain",
        message.expose().as_bytes(),
    );
    assert_eq!(status, 400, "only `application/jwt` is read");
    seat.assert_live(runtime);

    // The real message ends it, and a second copy changes nothing.
    assert_eq!(super::invalidate(gateway.port, message.expose()), 204);
    assert_eq!(
        super::invalidate(gateway.port, message.expose()),
        204,
        "taken before: Central need not send it again"
    );
    seat.assert_closed_for_a_refusal(runtime);

    // No way back in with what the client held.
    let renewed_bearer = common::json_field(&renew.1, "token").to_owned();
    for held in [&bearer, &renewed_bearer] {
        assert_eq!(common::http(gateway.port, "GET", "/me", Some(held), "").0, 401);
        assert_eq!(
            common::http(gateway.port, "POST", "/auth/session/renew", Some(held), "").0,
            401
        );
    }
    let body = super::sign_in_body(&token);
    for path in ["/auth/central/login", "/auth/central/refresh"] {
        assert_eq!(
            common::http(gateway.port, "POST", path, None, &body).0,
            401,
            "a token still in date, of a revoked session, opens nothing at {path}"
        );
    }
}

/// Central cannot reach the gateway: no message is posted, and the gateway
/// learns within one round of its own.
fn caught_up_run(
    runtime: &Runtime,
    gateway: &common::Gateway,
    endpoint: &Endpoint,
    origin: &str,
) {
    let http = CentralHttp::new(endpoint.clone());
    let session = signed_in(&http);
    let (bearer, _) = admit(gateway.port, &session, &http, origin);
    let mut seat = seat::LiveSeat::open(runtime, gateway, &bearer);
    session.logout(&http).unwrap();
    seat.assert_closes_within(runtime, CATCH_UP);
}

pub(super) fn run(
    runtime: &Runtime,
    gateway: &common::Gateway,
    endpoint: &Endpoint,
    origin: &str,
    bridge: &Token,
) {
    pushed_run(runtime, gateway, endpoint, origin, bridge);
    caught_up_run(runtime, gateway, endpoint, origin);
}
