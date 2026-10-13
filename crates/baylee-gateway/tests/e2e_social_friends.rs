//! End-to-end tests for the friends list (`docs/protocol.md` §"Friends and
//! chat (beta.8)"): requests, accepting, declining, cancelling, removing,
//! blocking, what the open social socket is told, and a gateway that has no
//! chat at all (`BAYLEE_SOCIAL=off`).
//!
//! Asked of a real gateway over HTTP and over its social socket. What a
//! player may see of another is the `GET /social/players/{handle}` lookup,
//! which says in four booleans how two accounts stand to each other; the
//! pushes are what the sockets are sent, read as `Down` frames.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::social::{
    self as wire, Down, FriendEvent, PresenceSet, Shown, Snapshot, Status, Up,
};
use common::{
    Socket, WAIT_BUDGET, ask_ticket, dial, http, json_field, login, spawn_gateway,
    spawn_gateway_with,
};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

/// An account as the tests drive it: its session, its tag and its handle.
struct Account {
    session: String,
    tag: String,
    handle: String,
}

fn json(body: &str) -> serde_json::Value {
    serde_json::from_str(body).unwrap_or_else(|e| panic!("not json ({e}): {body}"))
}

/// A registered account called `name`, with the tag and handle the gateway
/// gave it.
fn account(port: u16, name: &str) -> Account {
    let session = login(port, name, name);
    let (status, me) = http(port, "GET", "/me", Some(&session), "");
    assert_eq!(status, 200, "{me}");
    let me = json(&me);
    Account {
        session,
        tag: me["tag"].as_str().expect("a tag").to_owned(),
        handle: me["handle"].as_str().expect("a handle").to_owned(),
    }
}

/// A guest, as `POST /auth/guest` makes one.
fn guest(port: u16) -> Account {
    let (status, body) = http(port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 200, "guest: {body}");
    let session = json_field(&body, "token").to_owned();
    let (status, me) = http(port, "GET", "/me", Some(&session), "");
    assert_eq!(status, 200, "{me}");
    let me = json(&me);
    Account {
        session,
        tag: me["tag"].as_str().expect("a tag").to_owned(),
        handle: me["handle"].as_str().expect("a handle").to_owned(),
    }
}

/// The player lookup of `them` as `me` sees it.
fn lookup(port: u16, me: &Account, them: &Account) -> wire::PlayerLookup {
    let path = format!("/social/players/{}", them.handle.replace('#', "%23"));
    let (status, body) = http(port, "GET", &path, Some(&me.session), "");
    assert_eq!(status, 200, "lookup: {body}");
    serde_json::from_str(&body).expect("a lookup")
}

/// `(friend, incoming, outgoing, blocked)` of the lookup.
fn standing(port: u16, me: &Account, them: &Account) -> (bool, bool, bool, bool) {
    let l = lookup(port, me, them);
    (l.friend, l.incoming, l.outgoing, l.blocked)
}

/// `me` asks `them` to be friends; the status and the body.
fn ask(port: u16, me: &Account, them: &Account) -> (u16, String) {
    let body = serde_json::json!({ "handle": them.handle }).to_string();
    http(
        port,
        "POST",
        "/social/friends/requests",
        Some(&me.session),
        &body,
    )
}

/// `me` accepts the request `from` sent, and asserts it was taken.
fn accept(port: u16, me: &Account, from: &Account) {
    let path = format!("/social/friends/requests/{}/accept", from.tag);
    let (status, body) = http(port, "POST", &path, Some(&me.session), "");
    assert_eq!(status, 204, "accept: {body}");
}

/// Two accounts that are friends already.
fn befriend(port: u16, a: &Account, b: &Account) {
    let (status, body) = ask(port, a, b);
    assert_eq!(status, 201, "{body}");
    accept(port, b, a);
}

/// Registers a chat device for `who` and returns its id. `seed` is unique
/// among the accounts of one test.
fn device(port: u16, who: &Account, seed: u64) -> String {
    let (_, public) = baylee_seal::testkit::registered(&who.tag, seed);
    let body = serde_json::to_string(&wire::DeviceRegistration {
        ik: baylee_seal::b64::url::encode(&public.ik),
        dk: baylee_seal::b64::url::encode(&public.dk),
        dsig: baylee_seal::b64::url::encode(&public.dsig),
        label: "macOS".into(),
    })
    .expect("a registration");
    let (status, answer) = http(port, "POST", "/social/devices", Some(&who.session), &body);
    assert_eq!(status, 201, "register a device: {answer}");
    assert_eq!(json_field(&answer, "id"), public.id);
    public.id
}

async fn send_up(ws: &mut Socket, up: &Up) {
    let text = serde_json::to_string(up).expect("a frame");
    ws.send(Message::Text(text.into())).await.expect("send");
}

/// Reads frames until one satisfies `want`, skipping the rest.
///
/// Panics, with the frames it skipped, when the socket closes or nothing
/// fits in [`WAIT_BUDGET`].
async fn wait_for(ws: &mut Socket, what: &str, want: impl Fn(&Down) -> bool) -> Down {
    let mut seen: Vec<String> = Vec::new();
    let found = tokio::time::timeout(WAIT_BUDGET, async {
        loop {
            match ws.next().await {
                Some(Ok(Message::Text(text))) => {
                    let down: Down = serde_json::from_str(&text)
                        .unwrap_or_else(|e| panic!("not a frame ({e}): {text}"));
                    if want(&down) {
                        return down;
                    }
                    seen.push(text.to_string());
                }
                Some(Ok(Message::Close(frame))) => {
                    panic!("closed ({frame:?}) while waiting for {what}; saw {seen:?}")
                }
                Some(Ok(_)) => {}
                Some(Err(e)) => panic!("errored ({e}) while waiting for {what}; saw {seen:?}"),
                None => panic!("ended while waiting for {what}; saw {seen:?}"),
            }
        }
    })
    .await;
    found.unwrap_or_else(|_| panic!("no {what} within {WAIT_BUDGET:?}"))
}

/// Opens `who`'s social socket as `device` and reads through the welcome
/// and the snapshot.
async fn open(port: u16, who: &Account, device: &str) -> (Socket, Snapshot) {
    let (status, body) = ask_ticket(port, &who.session, r#"{"socket":"social"}"#);
    assert_eq!(status, 200, "a social ticket: {body}");
    let url = format!(
        "ws://127.0.0.1:{port}{}",
        wire::social_socket_path(json_field(&body, "ticket"))
    );
    let mut ws = dial(&url).await.expect("the social socket opens");
    let hello = Up::Hello {
        device: device.to_owned(),
        presence: PresenceSet {
            status: Status::Online,
            activity: None,
        },
    };
    send_up(&mut ws, &hello).await;
    wait_for(&mut ws, "welcome", |d| matches!(d, Down::Welcome { .. })).await;
    let Down::Snapshot(snapshot) =
        wait_for(&mut ws, "snapshot", |d| matches!(d, Down::Snapshot(_))).await
    else {
        unreachable!("waited for a snapshot")
    };
    (ws, snapshot)
}

/// The friend event `event` about `about` on a socket.
async fn friend_event(ws: &mut Socket, event: FriendEvent, about: &Account) -> Down {
    let tag = about.tag.clone();
    wait_for(ws, &format!("{event:?} about {tag}"), move |d| {
        matches!(d, Down::Friend { event: e, person, .. } if *e == event && person.tag == tag)
    })
    .await
}

#[tokio::test]
async fn a_request_shows_on_both_sides_and_accepting_it_makes_friends() {
    let gw = spawn_gateway("social_friends_accept");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    assert_eq!(standing(port, &ann, &bob), (false, false, false, false));

    let (status, body) = ask(port, &ann, &bob);
    assert_eq!(status, 201, "{body}");
    assert_eq!(json(&body), serde_json::json!({"state": "pending"}));
    assert_eq!(
        standing(port, &ann, &bob),
        (false, false, true, false),
        "the asker sees it outgoing"
    );
    assert_eq!(
        standing(port, &bob, &ann),
        (false, true, false, false),
        "the asked sees it incoming"
    );
    // Asking again changes nothing: the request is one.
    let (status, body) = ask(port, &ann, &bob);
    assert_eq!(status, 201, "{body}");
    assert_eq!(json(&body), serde_json::json!({"state": "pending"}));

    accept(port, &bob, &ann);
    assert_eq!(standing(port, &ann, &bob), (true, false, false, false));
    assert_eq!(standing(port, &bob, &ann), (true, false, false, false));

    // The lookup is the same for the tag alone, and says whom it found.
    let l = lookup(port, &ann, &bob);
    assert_eq!((l.tag, l.handle), (bob.tag.clone(), bob.handle.clone()));
}

#[tokio::test]
async fn a_request_can_be_declined_cancelled_and_a_friend_removed() {
    let gw = spawn_gateway("social_friends_undo");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");

    // Declined by the asked: gone on both sides, and nothing left to decline.
    assert_eq!(ask(port, &ann, &bob).0, 201);
    let decline = format!("/social/friends/requests/{}/decline", ann.tag);
    let (status, body) = http(port, "POST", &decline, Some(&bob.session), "");
    assert_eq!(status, 204, "{body}");
    assert_eq!(standing(port, &ann, &bob), (false, false, false, false));
    assert_eq!(standing(port, &bob, &ann), (false, false, false, false));
    let (status, _) = http(port, "POST", &decline, Some(&bob.session), "");
    assert_eq!(status, 404, "a second decline finds no request");

    // Cancelled by the asker.
    assert_eq!(ask(port, &ann, &cat).0, 201);
    let cancel = format!("/social/friends/requests/{}", cat.tag);
    let (status, body) = http(port, "DELETE", &cancel, Some(&ann.session), "");
    assert_eq!(status, 204, "{body}");
    assert_eq!(standing(port, &ann, &cat), (false, false, false, false));
    assert_eq!(standing(port, &cat, &ann), (false, false, false, false));
    let (status, _) = http(port, "DELETE", &cancel, Some(&ann.session), "");
    assert_eq!(status, 404, "a second cancel finds no request");

    // The asked cannot accept what was cancelled.
    let accept_path = format!("/social/friends/requests/{}/accept", ann.tag);
    let (status, _) = http(port, "POST", &accept_path, Some(&cat.session), "");
    assert_eq!(status, 404);

    // Removed by either side.
    befriend(port, &ann, &bob);
    assert_eq!(standing(port, &ann, &bob), (true, false, false, false));
    let remove = format!("/social/friends/{}", bob.tag);
    let (status, body) = http(port, "DELETE", &remove, Some(&ann.session), "");
    assert_eq!(status, 204, "{body}");
    assert_eq!(standing(port, &ann, &bob), (false, false, false, false));
    assert_eq!(standing(port, &bob, &ann), (false, false, false, false));
    let (status, _) = http(port, "DELETE", &remove, Some(&ann.session), "");
    assert_eq!(status, 404, "no longer friends");
    let (status, _) = http(
        port,
        "DELETE",
        &format!("/social/friends/{}", ann.tag),
        Some(&bob.session),
        "",
    );
    assert_eq!(status, 404, "the other side finds nothing either");
}

#[tokio::test]
async fn asking_someone_who_asked_already_makes_friends_at_once() {
    let gw = spawn_gateway("social_friends_mutual");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");

    assert_eq!(ask(port, &ann, &bob).0, 201);
    let (status, body) = ask(port, &bob, &ann);
    assert_eq!(status, 201, "{body}");
    assert_eq!(json(&body), serde_json::json!({"state": "friends"}));
    assert_eq!(standing(port, &ann, &bob), (true, false, false, false));
    assert_eq!(standing(port, &bob, &ann), (true, false, false, false));

    // Friends already: a conflict whichever side asks.
    for (from, to) in [(&ann, &bob), (&bob, &ann)] {
        let (status, body) = ask(port, from, to);
        assert_eq!(status, 409, "{body}");
        assert!(body.contains("already friends"), "{body}");
    }
}

#[tokio::test]
async fn a_player_who_takes_no_requests_is_refused_and_the_odd_requests_are_too() {
    let gw = spawn_gateway("social_friends_closed");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");

    let closed =
        r#"{"whispers_from":"friends_and_tables","requests_open":false,"share_table":true}"#;
    let (status, body) = http(port, "PUT", "/social/settings", Some(&bob.session), closed);
    assert_eq!(status, 204, "{body}");
    let (status, body) = ask(port, &ann, &bob);
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("takes no friend requests"), "{body}");
    assert_eq!(standing(port, &ann, &bob), (false, false, false, false));
    // A player who takes none may still ask.
    assert_eq!(ask(port, &bob, &ann).0, 201);

    let open = r#"{"whispers_from":"friends_and_tables","requests_open":true,"share_table":true}"#;
    let (status, _) = http(port, "PUT", "/social/settings", Some(&bob.session), open);
    assert_eq!(status, 204);
    assert_eq!(ask(port, &ann, &bob).0, 201, "open again");

    // Oneself, a stranger, a handle without its tag.
    let (status, body) = ask(port, &ann, &ann);
    assert_eq!(status, 400, "{body}");
    let nobody = Account {
        session: String::new(),
        tag: "ffffff".into(),
        handle: "Nobody#ffffff".into(),
    };
    assert_eq!(ask(port, &ann, &nobody).0, 404);
    let plain = serde_json::json!({ "handle": "bob" }).to_string();
    let (status, _) = http(
        port,
        "POST",
        "/social/friends/requests",
        Some(&ann.session),
        &plain,
    );
    assert_eq!(status, 400);
    // No session, no friends.
    let (status, _) = http(port, "GET", "/social/devices", None, "");
    assert_eq!(status, 401);
}

#[tokio::test]
async fn a_guest_has_no_friends_list_on_either_side() {
    let gw = spawn_gateway("social_friends_guest");
    let port = gw.port;
    let ann = account(port, "ann");
    let visitor = guest(port);

    let (status, body) = ask(port, &visitor, &ann);
    assert_eq!(status, 403, "a guest asks: {body}");
    let (status, body) = ask(port, &ann, &visitor);
    assert_eq!(status, 403, "a guest is asked: {body}");
    assert!(body.contains("guests have no friends list"), "{body}");

    let path = format!("/social/players/{}", ann.handle.replace('#', "%23"));
    let (status, body) = http(port, "GET", &path, Some(&visitor.session), "");
    assert_eq!(status, 403, "a guest looks a player up: {body}");
    let (status, _) = http(
        port,
        "POST",
        &format!("/social/friends/requests/{}/accept", ann.tag),
        Some(&visitor.session),
        "",
    );
    assert_eq!(status, 403);
    let (status, body) = http(
        port,
        "POST",
        "/social/groups",
        Some(&visitor.session),
        r#"{"name":"Crew"}"#,
    );
    assert_eq!(status, 403, "a guest makes a group: {body}");
    // What a guest may do: block and set its settings.
    let block = serde_json::json!({ "tag": ann.tag }).to_string();
    let (status, _) = http(
        port,
        "POST",
        "/social/blocks",
        Some(&visitor.session),
        &block,
    );
    assert_eq!(status, 204);
    let settings = r#"{"whispers_from":"friends","requests_open":true,"share_table":false}"#;
    let (status, _) = http(
        port,
        "PUT",
        "/social/settings",
        Some(&visitor.session),
        settings,
    );
    assert_eq!(status, 204);
}

#[tokio::test]
async fn blocking_ends_a_friendship_and_the_requests_and_unblocking_lifts_it() {
    let gw = spawn_gateway("social_friends_block");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    let dan = account(port, "dan");

    befriend(port, &ann, &bob);
    assert_eq!(ask(port, &cat, &ann).0, 201);
    assert_eq!(ask(port, &ann, &dan).0, 201);

    for other in [&bob, &cat, &dan] {
        let body = serde_json::json!({ "tag": other.tag }).to_string();
        let (status, answer) = http(port, "POST", "/social/blocks", Some(&ann.session), &body);
        assert_eq!(status, 204, "block: {answer}");
    }
    assert_eq!(
        standing(port, &ann, &bob),
        (false, false, false, true),
        "the friendship ended"
    );
    assert_eq!(standing(port, &bob, &ann), (false, false, false, false));
    assert_eq!(
        standing(port, &ann, &cat),
        (false, false, false, true),
        "their request went"
    );
    assert_eq!(standing(port, &cat, &ann), (false, false, false, false));
    assert_eq!(
        standing(port, &ann, &dan),
        (false, false, false, true),
        "her request went"
    );
    assert_eq!(standing(port, &dan, &ann), (false, false, false, false));

    // Unblocking lifts the block and nothing else comes back as friends.
    let lift = format!("/social/blocks/{}", bob.tag);
    let (status, body) = http(port, "DELETE", &lift, Some(&ann.session), "");
    assert_eq!(status, 204, "{body}");
    assert_eq!(standing(port, &ann, &bob), (false, false, false, false));
    let (status, _) = http(port, "DELETE", &lift, Some(&ann.session), "");
    assert_eq!(status, 404, "no longer blocked");
    let me = serde_json::json!({ "tag": ann.tag }).to_string();
    let (status, _) = http(port, "POST", "/social/blocks", Some(&ann.session), &me);
    assert_eq!(status, 400, "one cannot block oneself");
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario with a sentinel
async fn a_request_from_a_blocked_account_looks_sent_and_never_reaches_the_blocker() {
    let gw = spawn_gateway("social_friends_blocked_ask");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    let (ann_device, bob_device) = (device(port, &ann, 1), device(port, &bob, 2));
    let (mut ann_ws, _) = open(port, &ann, &ann_device).await;
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;

    let block = serde_json::json!({ "tag": bob.tag }).to_string();
    let (status, _) = http(port, "POST", "/social/blocks", Some(&ann.session), &block);
    assert_eq!(status, 204);
    friend_event(&mut ann_ws, FriendEvent::Blocked, &bob).await;

    // The blocked account is told it went out: a pending answer, an
    // outgoing request on its own socket and in its own lookup.
    let (status, body) = ask(port, &bob, &ann);
    assert_eq!(status, 201, "{body}");
    assert_eq!(json(&body), serde_json::json!({"state": "pending"}));
    friend_event(&mut bob_ws, FriendEvent::RequestOut, &ann).await;
    assert!(lookup(port, &bob, &ann).outgoing);

    // The blocker sees nothing of it.
    assert!(
        !lookup(port, &ann, &bob).incoming,
        "the blocker's lookup shows no request"
    );
    // A later frame on the blocker's own socket orders the check: frames
    // reach a socket in the order they were pushed, so a request that was
    // pushed would be ahead of it.
    let other = serde_json::json!({ "tag": cat.tag }).to_string();
    let (status, _) = http(port, "POST", "/social/blocks", Some(&ann.session), &other);
    assert_eq!(status, 204);
    let tag = cat.tag.clone();
    let mut ahead = Vec::new();
    loop {
        let frame = wait_for(&mut ann_ws, "the sentinel", |_| true).await;
        match &frame {
            Down::Friend {
                event: FriendEvent::Blocked,
                person,
                ..
            } if person.tag == tag => break,
            _ => ahead.push(frame),
        }
    }
    assert!(
        !ahead.iter().any(|f| matches!(
            f,
            Down::Friend {
                event: FriendEvent::RequestIn,
                ..
            }
        )),
        "the blocker was pushed a request: {ahead:?}"
    );
    // Nor is it in the snapshot a new socket gets.
    drop(ann_ws);
    let (_, snapshot) = open(port, &ann, &ann_device).await;
    assert!(snapshot.incoming.is_empty(), "{:?}", snapshot.incoming);
    assert!(snapshot.blocked.iter().any(|p| p.tag == bob.tag));

    // Unblocked, the block is gone.
    let (status, _) = http(
        port,
        "DELETE",
        &format!("/social/blocks/{}", bob.tag),
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 204);
    assert!(!lookup(port, &ann, &bob).blocked);
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario, five pushes
async fn the_open_sockets_are_told_of_a_request_its_answer_and_the_end_of_a_friendship() {
    let gw = spawn_gateway("social_friends_push");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    let (ann_device, bob_device, cat_device) = (
        device(port, &ann, 1),
        device(port, &bob, 2),
        device(port, &cat, 3),
    );
    let (mut ann_ws, ann_start) = open(port, &ann, &ann_device).await;
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;
    let (mut cat_ws, _) = open(port, &cat, &cat_device).await;
    assert!(ann_start.friends.is_empty() && ann_start.incoming.is_empty());

    // A request: the asked is told RequestIn, the asker RequestOut.
    assert_eq!(ask(port, &ann, &bob).0, 201);
    let Down::Friend {
        person, presence, ..
    } = friend_event(&mut bob_ws, FriendEvent::RequestIn, &ann).await
    else {
        unreachable!()
    };
    assert_eq!(person.handle, ann.handle);
    assert_eq!(presence, None, "a request carries no presence");
    friend_event(&mut ann_ws, FriendEvent::RequestOut, &bob).await;

    // Accepted: both are told, each with the other's presence.
    accept(port, &bob, &ann);
    for (ws, other) in [(&mut ann_ws, &bob), (&mut bob_ws, &ann)] {
        let Down::Friend {
            person, presence, ..
        } = friend_event(ws, FriendEvent::Accepted, other).await
        else {
            unreachable!()
        };
        assert_eq!(person.handle, other.handle);
        let presence = presence.expect("an accepted friend arrives with its presence");
        assert_eq!(presence.status, Shown::Online, "both sockets are open");
    }

    // A request that is cancelled is withdrawn from the asked.
    assert_eq!(ask(port, &ann, &cat).0, 201);
    friend_event(&mut cat_ws, FriendEvent::RequestIn, &ann).await;
    let (status, _) = http(
        port,
        "DELETE",
        &format!("/social/friends/requests/{}", cat.tag),
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 204);
    friend_event(&mut cat_ws, FriendEvent::RequestGone, &ann).await;
    friend_event(&mut ann_ws, FriendEvent::RequestGone, &cat).await;

    // One that is declined is withdrawn from both.
    assert_eq!(ask(port, &cat, &ann).0, 201);
    friend_event(&mut ann_ws, FriendEvent::RequestIn, &cat).await;
    let (status, _) = http(
        port,
        "POST",
        &format!("/social/friends/requests/{}/decline", cat.tag),
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 204);
    friend_event(&mut ann_ws, FriendEvent::RequestGone, &cat).await;
    friend_event(&mut cat_ws, FriendEvent::RequestGone, &ann).await;

    // The end of a friendship is pushed to both.
    let (status, _) = http(
        port,
        "DELETE",
        &format!("/social/friends/{}", ann.tag),
        Some(&bob.session),
        "",
    );
    assert_eq!(status, 204);
    friend_event(&mut ann_ws, FriendEvent::Removed, &bob).await;
    friend_event(&mut bob_ws, FriendEvent::Removed, &ann).await;
}

/// Every route under `/social/…`, each with a body that would be read were
/// chat on: a refusal then is the gateway's, not an extractor's.
fn every_social_request() -> Vec<(&'static str, String, String)> {
    let device = serde_json::to_string(&wire::DeviceRegistration {
        ik: baylee_seal::b64::url::encode(&[1; 32]),
        dk: baylee_seal::b64::url::encode(&[2; 32]),
        dsig: baylee_seal::b64::url::encode(&[3; 64]),
        label: "macOS".into(),
    })
    .expect("a registration");
    let id = "01923f3c-7a2b-7c00-8000-0123456789ab";
    vec![
        ("POST", "/social/devices".into(), device),
        ("GET", "/social/devices".into(), String::new()),
        (
            "DELETE",
            "/social/devices/0123456789abcdef".into(),
            String::new(),
        ),
        ("GET", "/social/keys?tag=0001".into(), String::new()),
        ("GET", "/social/players/Ann%230001".into(), String::new()),
        (
            "POST",
            "/social/friends/requests".into(),
            r#"{"handle":"Ann#0001"}"#.into(),
        ),
        (
            "POST",
            "/social/friends/requests/0001/accept".into(),
            String::new(),
        ),
        (
            "POST",
            "/social/friends/requests/0001/decline".into(),
            String::new(),
        ),
        (
            "DELETE",
            "/social/friends/requests/0001".into(),
            String::new(),
        ),
        ("DELETE", "/social/friends/0001".into(), String::new()),
        ("POST", "/social/blocks".into(), r#"{"tag":"0001"}"#.into()),
        ("DELETE", "/social/blocks/0001".into(), String::new()),
        (
            "PUT",
            "/social/settings".into(),
            r#"{"whispers_from":"friends","requests_open":true,"share_table":true}"#.into(),
        ),
        ("POST", "/social/groups".into(), r#"{"name":"Crew"}"#.into()),
        (
            "PATCH",
            format!("/social/groups/{id}"),
            r#"{"name":"Crew"}"#.into(),
        ),
        ("DELETE", format!("/social/groups/{id}"), String::new()),
        (
            "POST",
            format!("/social/groups/{id}/invites"),
            r#"{"tag":"0001"}"#.into(),
        ),
        ("POST", format!("/social/groups/{id}/join"), String::new()),
        (
            "DELETE",
            format!("/social/groups/{id}/invites/0001"),
            String::new(),
        ),
        (
            "DELETE",
            format!("/social/groups/{id}/members/0001"),
            String::new(),
        ),
        (
            "POST",
            format!("/social/groups/{id}/owner"),
            r#"{"tag":"0001"}"#.into(),
        ),
        (
            "POST",
            "/social/invites".into(),
            r#"{"tag":"0001","game":"g"}"#.into(),
        ),
        ("DELETE", format!("/social/invites/{id}"), String::new()),
    ]
}

#[tokio::test]
async fn a_gateway_with_chat_off_refuses_every_social_route_and_says_so_in_info() {
    let off = spawn_gateway_with("social_friends_off", &[("BAYLEE_SOCIAL", "off".into())]);
    let port = off.port;
    let session = login(port, "ann", "ann");

    for (method, path, body) in every_social_request() {
        let (status, answer) = http(port, method, &path, Some(&session), &body);
        assert_eq!(status, 403, "{method} {path}: {answer}");
        assert_eq!(
            json_field(&answer, "error"),
            "this gateway has no chat",
            "{method} {path}"
        );
    }
    let (status, answer) = ask_ticket(port, &session, r#"{"socket":"social"}"#);
    assert_eq!(status, 403, "{answer}");
    assert_eq!(json_field(&answer, "error"), "this gateway has no chat");
    // The other tickets are untouched.
    let (status, _) = ask_ticket(port, &session, r#"{"socket":"lobby"}"#);
    assert_eq!(status, 200);

    let (status, info) = http(port, "GET", "/info", None, "");
    assert_eq!(status, 200, "{info}");
    assert_eq!(json(&info)["social"], serde_json::Value::Null, "{info}");
}

#[tokio::test]
async fn info_says_which_chat_a_gateway_has_and_how_long_it_keeps_a_message() {
    let default = spawn_gateway("social_friends_info");
    let (status, info) = http(default.port, "GET", "/info", None, "");
    assert_eq!(status, 200, "{info}");
    assert_eq!(
        json(&info)["social"],
        serde_json::json!({"version": 1, "keep_days": 14}),
        "{info}"
    );

    let short = spawn_gateway_with(
        "social_friends_info_3",
        &[("BAYLEE_SOCIAL_KEEP_DAYS", "3".into())],
    );
    let (status, info) = http(short.port, "GET", "/info", None, "");
    assert_eq!(status, 200, "{info}");
    assert_eq!(
        json(&info)["social"],
        serde_json::json!({"version": 1, "keep_days": 3}),
        "{info}"
    );
}
