//! End-to-end test for `GET /health`.
//!
//! The route's whole reason for existing is that the evidence it replaces was
//! too weak: an open port says `bind` succeeded, and a gateway can be bound,
//! logging nothing, and unable to host a single game. So the test worth
//! writing is not "does it answer 200" — the harness now asserts that on
//! every spawn, three dozen times a suite — but that each field *moves* when
//! the thing it describes moves. A field that is always `0` passes a presence
//! check and tells a monitor nothing.
//!
//! `agents.connected` is the one that carries the ticket. Without an agent
//! the gateway answers `503` to `POST /lobby/games`, which is the most common
//! "why is nothing working" state and was invisible from outside; here it is
//! read before and after a real agent socket.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{attach_agent, http, json_field, json_number, spawn_gateway, spawn_gateway_with};

#[tokio::test(flavor = "multi_thread")]
async fn health_answers_what_a_monitor_cannot_learn_from_an_open_port() {
    let gw = spawn_gateway("health");

    let (status, body) = http(gw.port, "GET", "/health", None, "");
    assert_eq!(status, 200, "health: {body}");
    assert!(body.contains("\"ok\":true"), "ok: {body}");
    assert!(body.contains("\"database\":true"), "database: {body}");

    // No agent has connected yet, and that is a field rather than a failure:
    // this gateway is correct, it simply cannot host a game. Both halves are
    // asserted, because a 503 here would break the other three dozen tests
    // that spawn an agentless gateway and expect it to serve.
    assert_eq!(json_number(&body, "connected"), 0, "agents: {body}");
    assert_eq!(json_number(&body, "running"), 0, "games: {body}");

    // The test database is a schema of its own with no ingest in it, so the
    // catalog is reachable and empty. `off` would mean the schema could not
    // be applied at all, which is a different fault with the same symptom —
    // no card text — and the point of spelling the states apart.
    assert_eq!(json_field(&body, "state"), "empty", "catalog: {body}");

    // One binary, one version, whichever route is asked. These are built from
    // the same `baylee_build` constants; the assertion is what stops a second
    // spelling being introduced later and the two drifting.
    let (_, source) = http(gw.port, "GET", "/source", None, "");
    assert_eq!(
        json_field(&body, "version"),
        json_field(&source, "version"),
        "health {body}\nsource {source}"
    );

    // Unauthenticated, so the answer may not carry a secret. The agent token
    // is the one this process holds that would be worth the most to a reader
    // — it is what lets anything claim to be an agent — and it is checked by
    // value rather than by field name so that adding a field cannot smuggle
    // it back in under another spelling.
    assert!(
        !body.contains(&gw.agent_token),
        "the agent token is in the health answer: {body}"
    );
    assert!(!body.contains("postgres://"), "a database URL: {body}");

    // And now the field moves. `attach_agent` returns once the gateway has
    // registered the socket, so no polling is needed here; if that ever
    // stops being true this assertion is where it will be noticed.
    let agent = attach_agent(&gw).await;
    let (status, body) = http(gw.port, "GET", "/health", None, "");
    assert_eq!(status, 200, "health after attach: {body}");
    assert_eq!(
        json_number(&body, "connected"),
        1,
        "an agent is connected, and health still says none: {body}"
    );

    agent.abort();
}

/// `GET /info` is what a client asks before it saves a gateway: the name to
/// show, and the two versions it decides compatibility on.
///
/// Both halves of the name are played, set and unset, because a field that
/// is always there passes a presence check and a client that falls back to
/// the address would never be exercised. The versions are compared with the
/// constants a client compares them with, and the build fields with
/// `/source`, which is the same function answering.
#[tokio::test(flavor = "multi_thread")]
async fn info_names_the_gateway_and_the_versions_a_client_decides_on() {
    let named = spawn_gateway_with(
        "info-named",
        &[("BAYLEE_GATEWAY_NAME", "  Baylee Test Hall  ".into())],
    );
    let (status, body) = http(named.port, "GET", "/info", None, "");
    assert_eq!(status, 200, "info: {body}");
    assert_eq!(
        json_field(&body, "name"),
        "Baylee Test Hall",
        "trimmed: {body}"
    );
    assert_eq!(
        json_number(&body, "protocol_version"),
        i64::from(baylee_protocol::PROTOCOL_VERSION),
        "protocol: {body}"
    );
    assert_eq!(
        json_number(&body, "view_version"),
        i64::from(baylee_view::VIEW_VERSION),
        "view: {body}"
    );
    let (_, source) = http(named.port, "GET", "/source", None, "");
    for field in ["version", "commit", "build"] {
        assert_eq!(
            json_field(&body, field),
            json_field(&source, field),
            "{field}: info {body}\nsource {source}"
        );
    }
    assert!(
        !body.contains(&named.agent_token),
        "a secret in info: {body}"
    );

    let unnamed = spawn_gateway("info-unnamed");
    let (status, body) = http(unnamed.port, "GET", "/info", None, "");
    assert_eq!(status, 200, "info: {body}");
    assert!(
        !body.contains("\"name\""),
        "no name set, so none is sent and the client shows the address: {body}"
    );
}
