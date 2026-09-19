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

/// A standing answer is refused for two different reasons, and the player is
/// told which.
///
/// Both are still a 400 — an answer for a card this build cannot play could
/// never fire, and storing it would spend a bounded budget on nothing and
/// leave an index that breaks the next read-modify-write. What was wrong was
/// only the message: a perfectly good handle for a real card was called
/// `unknown card`, which accuses the client of sending nonsense.
///
/// Both indices are derived. `4000000` is past the end of the ledger however
/// it grows; the other is the first ledger row this build compiles no card
/// for, so it stays a real card as the pool grows rather than becoming one.
#[test]
fn a_standing_answer_for_a_real_card_is_not_refused_as_unknown() {
    let gw = spawn_gateway("automation-reasons");
    let token = login(gw.port, "picky@example.com", "picky_player");

    let real_but_unplayable = baylee_cards_index::ROWS
        .iter()
        .find(|row| baylee_cards::by_index(row.index).is_none())
        .expect("this build compiles 2716 of 33 694 cards");
    let put = format!(
        "{{\"answers\":[{{\"card\":{},\"ability\":0,\"yes\":true}}]}}",
        real_but_unplayable.index.get()
    );
    let (status, body) = http(gw.port, "PUT", "/automation", Some(&token), &put);
    assert_eq!(status, 400, "it is still refused: {body}");
    assert!(
        body.contains("this server cannot play it"),
        "{} is a real card and was called unknown: {body}",
        real_but_unplayable.name
    );

    // The counter-half: a number that is no card keeps the old answer, or
    // the assertion above would pass on a route that says one thing to
    // everybody.
    let bad = "{\"answers\":[{\"card\":4000000,\"ability\":0,\"yes\":true}]}";
    let (status, body) = http(gw.port, "PUT", "/automation", Some(&token), bad);
    assert_eq!(status, 400, "{body}");
    assert!(
        body.contains("unknown card"),
        "a number that is no card at all is still unknown: {body}"
    );

    // And nothing was stored by either refusal.
    let (status, body) = http(gw.port, "GET", "/automation", Some(&token), "");
    assert_eq!(status, 200);
    assert!(
        body.contains("\"answers\":[]"),
        "a refused answer reached the store: {body}"
    );
}
