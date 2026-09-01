//! End-to-end test for remembered standing answers: the gateway stores a
//! seat's "always answer yes to this ability" per **account**, so it can be
//! replayed into every new game.
//!
//! The engine addresses those answers by `AbilityRef`, a handle that says
//! nothing about a particular game — that is what makes them storable at
//! all — and the gateway must not take a client's word for what a valid
//! handle is.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http, login, spawn_gateway};

fn ondu_cleric() -> u32 {
    baylee_cards::by_oracle_id("f4232466-dd6a-49bf-be6c-95905c3ded17")
        .expect("the card pool has Ondu Cleric")
        .index
        .get()
}

#[test]
fn standing_answers_are_remembered_per_account() {
    let gw = spawn_gateway("automation");
    let token = login(gw.port, "cleric@example.com", "cleric_fan");

    let (status, body) = http(gw.port, "GET", "/automation", Some(&token), "");
    assert_eq!(status, 200, "empty listing: {body}");
    assert!(
        body.contains("\"answers\":[]"),
        "expected nothing yet: {body}"
    );

    // Ondu Cleric's rally trigger — the card the feature was asked for by
    // name. Sent twice and out of order to prove the gateway normalises.
    let card = ondu_cleric();
    let put = format!(
        "{{\"answers\":[{{\"card\":{card},\"ability\":1,\"yes\":false}},\
          {{\"card\":{card},\"ability\":0,\"yes\":true}},\
          {{\"card\":{card},\"ability\":0,\"yes\":true}}]}}"
    );
    let (status, body) = http(gw.port, "PUT", "/automation", Some(&token), &put);
    assert_eq!(status, 200, "store answers: {body}");
    assert!(
        body.contains("\"stored\":2"),
        "the duplicate was not collapsed: {body}"
    );

    let (status, body) = http(gw.port, "GET", "/automation", Some(&token), "");
    assert_eq!(status, 200);
    assert!(
        body.contains(&format!("\"card\":{card}")) && body.contains("\"yes\":true"),
        "the answers did not come back: {body}"
    );

    // A handle no card can ever produce is refused rather than stored: it
    // could never fire, and junk in the store outlives the request.
    let bad = "{\"answers\":[{\"card\":4000000,\"ability\":0,\"yes\":true}]}";
    let (status, _) = http(gw.port, "PUT", "/automation", Some(&token), bad);
    assert_eq!(status, 400, "an unknown card was accepted");

    // And it is per account: a second account sees none of it.
    let other = login(gw.port, "other@example.com", "other_player");
    let (status, body) = http(gw.port, "GET", "/automation", Some(&other), "");
    assert_eq!(status, 200);
    assert!(
        body.contains("\"answers\":[]"),
        "one account's setting leaked into another: {body}"
    );

    // Unauthenticated callers get nothing.
    let (status, _) = http(gw.port, "GET", "/automation", None, "");
    assert_eq!(
        status, 401,
        "an anonymous caller read an account's settings"
    );
}
