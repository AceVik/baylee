//! End-to-end test for the headers a browser needs to read an answer.
//!
//! The browser client is served from a different origin by construction — the
//! page is a `trunk serve` on :8080 and the gateway is not, which is the whole
//! reason `?gateway=…` exists. So every request it makes is cross-origin, and
//! a gateway that says nothing about CORS is a gateway no browser client can
//! use. That is not a theory: before this layer existed, `OPTIONS
//! /auth/login` from `http://localhost:8080` was answered `405 Method Not
//! Allowed` — axum resolves a path to the methods a handler registered, and
//! no handler registers `OPTIONS` — so the lobby's very first request never
//! left the browser.
//!
//! The two halves fail differently and are therefore tested separately. A
//! request carrying `Authorization` is **preflighted** and is not sent at all
//! when the preflight is refused; a plain `GET` is a *simple* request that is
//! sent, answered, and then discarded by the browser when the answer has no
//! `Access-Control-Allow-Origin` on it. A test that only checked the
//! preflight would pass with every GET in the client still broken.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http_headers, spawn_gateway};

/// The origin a browser client actually reports: `trunk serve`'s default.
const PAGE: &str = "http://localhost:8080";

#[test]
fn a_browser_on_another_origin_may_read_what_the_gateway_answers() {
    let gw = spawn_gateway("cors");

    // The preflight for the lobby's first request. `authorization` and
    // `content-type` are what make it preflighted in the first place.
    let (status, head) = http_headers(
        gw.port,
        "OPTIONS",
        "/auth/login",
        &[
            ("Origin", PAGE),
            ("Access-Control-Request-Method", "POST"),
            (
                "Access-Control-Request-Headers",
                "authorization, content-type",
            ),
        ],
    );
    assert_eq!(status, 204, "the preflight was refused: {head}");
    assert!(
        head.contains("access-control-allow-origin: *"),
        "no allowed origin on the preflight: {head}"
    );
    assert!(
        head.contains("access-control-allow-headers: authorization, content-type"),
        "the bearer token may not be sent: {head}"
    );
    assert!(
        head.contains("post"),
        "POST is not among the allowed methods: {head}"
    );

    // The other half: a simple request, which no preflight protects.
    let (status, head) = http_headers(gw.port, "GET", "/source", &[("Origin", PAGE)]);
    assert_eq!(status, 200, "the source route answered {status}");
    assert!(
        head.contains("access-control-allow-origin: *"),
        "a simple GET the browser would throw away: {head}"
    );

    // The pair that must not drift apart. `*` is only defensible because this
    // gateway carries a bearer token in a header and sets no cookie, so a
    // cross-origin request brings nothing ambient with it. Saying
    // `Allow-Credentials` beside `*` is the combination the fetch spec
    // refuses outright — and the day it were honoured it would be the day a
    // stranger's page could act as whoever is signed in.
    assert!(
        !head.contains("access-control-allow-credentials"),
        "credentials must never be allowed beside a wildcard origin: {head}"
    );
}
