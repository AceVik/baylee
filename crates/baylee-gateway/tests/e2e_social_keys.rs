//! End-to-end tests for chat devices and their keys (`docs/protocol.md`
//! §"Friends and chat (beta.8)"): registering a device with a statement
//! that binds its keys to the account's tag, listing and removing devices,
//! which accounts' keys `GET /social/keys` hands out (and which it leaves
//! out as if they were not there), and what the social socket does with a
//! `hello` that names a device.
//!
//! Real signatures from `baylee_seal::testkit`, real sockets. A fresh
//! gateway per test, so the per-account allowances (five new devices a day,
//! twenty key fetches at once) start full in each.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::social::{self as wire, Down, PresenceSet, Snapshot, Status, Up};
use common::{
    Gateway, Socket, WAIT_BUDGET, ask_ticket, dial, http, json_field, login, spawn_gateway,
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

fn from_me(session: String, me: &str) -> Account {
    let me = json(me);
    Account {
        session,
        tag: me["tag"].as_str().expect("a tag").to_owned(),
        handle: me["handle"].as_str().expect("a handle").to_owned(),
    }
}

fn account(port: u16, name: &str) -> Account {
    let session = login(port, name, name);
    let (status, me) = http(port, "GET", "/me", Some(&session), "");
    assert_eq!(status, 200, "{me}");
    from_me(session, &me)
}

fn guest(port: u16) -> Account {
    let (status, body) = http(port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 200, "guest: {body}");
    let session = json_field(&body, "token").to_owned();
    let (status, me) = http(port, "GET", "/me", Some(&session), "");
    assert_eq!(status, 200, "{me}");
    from_me(session, &me)
}

fn b64(bytes: &[u8]) -> String {
    baylee_seal::b64::url::encode(bytes)
}

/// The body of `POST /social/devices` for `public`.
fn registration(public: &baylee_seal::DevicePublic, label: &str) -> String {
    serde_json::to_string(&wire::DeviceRegistration {
        ik: b64(&public.ik),
        dk: b64(&public.dk),
        dsig: b64(&public.dsig),
        label: label.to_owned(),
    })
    .expect("a registration")
}

/// `who` registers the device drawn from `seed`, statement and all, for
/// `tag`. The status and the body.
fn try_register(port: u16, who: &Account, tag: &str, seed: u64) -> (u16, String) {
    let (_, public) = baylee_seal::testkit::registered(tag, seed);
    http(
        port,
        "POST",
        "/social/devices",
        Some(&who.session),
        &registration(&public, "macOS"),
    )
}

/// `who` registers the device of `seed` for its own tag, and the id comes
/// back.
fn device(port: u16, who: &Account, seed: u64) -> String {
    let (status, body) = try_register(port, who, &who.tag, seed);
    assert_eq!(status, 201, "register a device: {body}");
    let id = baylee_seal::testkit::device(seed).device_id().to_string();
    assert_eq!(json_field(&body, "id"), id);
    id
}

fn list_devices(port: u16, who: &Account) -> Vec<wire::DeviceRow> {
    let (status, body) = http(port, "GET", "/social/devices", Some(&who.session), "");
    assert_eq!(status, 200, "{body}");
    serde_json::from_str::<wire::DeviceList>(&body)
        .expect("a device list")
        .devices
}

/// The keys `me` is handed for `tags`.
fn keys(port: u16, me: &Account, tags: &[&str]) -> Vec<wire::AccountKeys> {
    let query: Vec<String> = tags.iter().map(|t| format!("tag={t}")).collect();
    let path = format!("/social/keys?{}", query.join("&"));
    let (status, body) = http(port, "GET", &path, Some(&me.session), "");
    assert_eq!(status, 200, "keys: {body}");
    serde_json::from_str::<wire::KeysAnswer>(&body)
        .expect("keys")
        .keys
}

/// Checks the way a client does that every device of `keys` is its
/// account's: the statement verifies for the account's tag and the id is
/// the key's fingerprint.
fn assert_verified(account: &wire::AccountKeys) {
    assert!(!account.devices.is_empty(), "{} has no device", account.tag);
    for d in &account.devices {
        let take32 = |text: &str| -> [u8; 32] {
            baylee_seal::b64::url::decode(text)
                .expect("base64url")
                .try_into()
                .expect("32 bytes")
        };
        let (ik, dk) = (take32(&d.ik), take32(&d.dk));
        let dsig: [u8; 64] = baylee_seal::b64::url::decode(&d.dsig)
            .expect("base64url")
            .try_into()
            .expect("64 bytes");
        baylee_seal::verify_device(&account.tag, &ik, &dk, &dsig).expect("a signed device");
        assert_eq!(d.id, baylee_seal::device_id(&ik));
    }
}

fn tags_of(keys: &[wire::AccountKeys]) -> Vec<String> {
    let mut tags: Vec<String> = keys.iter().map(|k| k.tag.clone()).collect();
    tags.sort();
    tags
}

fn sorted(tags: &[&str]) -> Vec<String> {
    let mut tags: Vec<String> = tags.iter().map(|t| (*t).to_owned()).collect();
    tags.sort();
    tags
}

fn befriend(port: u16, a: &Account, b: &Account) {
    let body = serde_json::json!({ "handle": b.handle }).to_string();
    let (status, answer) = http(
        port,
        "POST",
        "/social/friends/requests",
        Some(&a.session),
        &body,
    );
    assert_eq!(status, 201, "{answer}");
    let path = format!("/social/friends/requests/{}/accept", a.tag);
    let (status, answer) = http(port, "POST", &path, Some(&b.session), "");
    assert_eq!(status, 204, "{answer}");
}

async fn send_up(ws: &mut Socket, up: &Up) {
    let text = serde_json::to_string(up).expect("a frame");
    ws.send(Message::Text(text.into())).await.expect("send");
}

/// Reads frames until one satisfies `want`, skipping the rest.
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

/// The close code the socket ends with, skipping any frame before it;
/// `None` for a socket that ends without one.
async fn closing(ws: &mut Socket) -> Option<u16> {
    tokio::time::timeout(WAIT_BUDGET, async {
        loop {
            match ws.next().await {
                Some(Ok(Message::Close(frame))) => return frame.map(|f| u16::from(f.code)),
                Some(Ok(_)) => {}
                Some(Err(_)) | None => return None,
            }
        }
    })
    .await
    .expect("the socket closed")
}

/// Dials the social socket of `who` and says hello as `device`, without
/// reading the answer.
async fn dial_hello(port: u16, who: &Account, device: &str) -> Socket {
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
    ws
}

/// Opens the social socket as `device` and reads through the welcome and
/// the snapshot.
async fn open(port: u16, who: &Account, device: &str) -> (Socket, Snapshot) {
    let mut ws = dial_hello(port, who, device).await;
    wait_for(&mut ws, "welcome", |d| matches!(d, Down::Welcome { .. })).await;
    let Down::Snapshot(snapshot) =
        wait_for(&mut ws, "snapshot", |d| matches!(d, Down::Snapshot(_))).await
    else {
        unreachable!("waited for a snapshot")
    };
    (ws, snapshot)
}

/// Five devices for the account `username`, put straight into the table:
/// a sixth over HTTP can then only meet the cap, not the allowance.
fn seed_devices(gw: &Gateway, username: &str, count: u32) {
    let id = gw.text(&format!(
        "SELECT id::text FROM account WHERE username = '{username}'"
    ));
    for n in 1..=count {
        gw.sql(&format!(
            "INSERT INTO social_device (id, account_id, ik, dk, dsig, label) VALUES \
             ('000000000000000{n}', '{id}', decode(lpad('{n}', 64, '0'), 'hex'), \
              decode(lpad('{n}', 64, '1'), 'hex'), decode(lpad('{n}', 128, '2'), 'hex'), 'seeded')"
        ));
    }
}

#[tokio::test]
async fn a_device_is_registered_listed_and_removed() {
    let gw = spawn_gateway("social_keys_device");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");

    assert!(list_devices(port, &ann).is_empty());
    let id = device(port, &ann, 1);
    // The same device again is the same answer, not a second device.
    let (status, body) = try_register(port, &ann, &ann.tag, 1);
    assert_eq!(status, 201, "{body}");
    assert_eq!(json_field(&body, "id"), id);
    let listed = list_devices(port, &ann);
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(
        (listed[0].id.as_str(), listed[0].label.as_str()),
        (id.as_str(), "macOS")
    );
    assert!(listed[0].added_at > 1_600_000_000_000, "milliseconds");
    assert!(listed[0].seen_at >= listed[0].added_at);

    let second = device(port, &ann, 2);
    let listed = list_devices(port, &ann);
    assert_eq!(
        listed.iter().map(|d| d.id.clone()).collect::<Vec<_>>(),
        vec![id.clone(), second.clone()],
        "oldest first"
    );
    assert!(list_devices(port, &bob).is_empty(), "devices are one's own");

    // Removing is one's own: another account's device is not found.
    let path = format!("/social/devices/{id}");
    let (status, _) = http(port, "DELETE", &path, Some(&bob.session), "");
    assert_eq!(status, 404);
    let (status, body) = http(port, "DELETE", &path, Some(&ann.session), "");
    assert_eq!(status, 204, "{body}");
    let (status, _) = http(port, "DELETE", &path, Some(&ann.session), "");
    assert_eq!(status, 404, "gone already");
    assert_eq!(
        list_devices(port, &ann)
            .iter()
            .map(|d| d.id.clone())
            .collect::<Vec<_>>(),
        vec![second]
    );
}

#[tokio::test]
async fn a_device_whose_statement_or_shape_is_wrong_is_refused() {
    let gw = spawn_gateway("social_keys_refused");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let (_, public) = baylee_seal::testkit::registered(&ann.tag, 1);
    let good = wire::DeviceRegistration {
        ik: b64(&public.ik),
        dk: b64(&public.dk),
        dsig: b64(&public.dsig),
        label: "macOS".into(),
    };
    let post = |body: &wire::DeviceRegistration| {
        http(
            port,
            "POST",
            "/social/devices",
            Some(&ann.session),
            &serde_json::to_string(body).expect("a registration"),
        )
    };

    // A statement signed for another account's tag does not hold here.
    let (status, body) = try_register(port, &ann, &bob.tag, 1);
    assert_eq!(status, 400, "{body}");
    // Keys that are not the signed ones.
    let (_, other) = baylee_seal::testkit::registered(&ann.tag, 2);
    let swapped = wire::DeviceRegistration {
        dk: b64(&other.dk),
        ..good.clone()
    };
    assert_eq!(post(&swapped).0, 400);

    // Shapes: base64 that is not base64url, keys of the wrong length, labels
    // that are not an OS family.
    let bad: Vec<wire::DeviceRegistration> = vec![
        wire::DeviceRegistration {
            ik: "not base64 at all!".into(),
            ..good.clone()
        },
        wire::DeviceRegistration {
            ik: b64(&public.ik[..31]),
            ..good.clone()
        },
        wire::DeviceRegistration {
            dk: b64(&[0; 33]),
            ..good.clone()
        },
        wire::DeviceRegistration {
            dsig: b64(&public.dsig[..63]),
            ..good.clone()
        },
        wire::DeviceRegistration {
            label: String::new(),
            ..good.clone()
        },
        wire::DeviceRegistration {
            label: "   ".into(),
            ..good.clone()
        },
        wire::DeviceRegistration {
            label: "x".repeat(25),
            ..good.clone()
        },
        wire::DeviceRegistration {
            label: "mac\u{202E}OS".into(),
            ..good.clone()
        },
        wire::DeviceRegistration {
            label: "mac\nOS".into(),
            ..good.clone()
        },
    ];
    for body in &bad {
        let (status, answer) = post(body);
        assert_eq!(status, 400, "{body:?}: {answer}");
    }
    assert!(
        list_devices(port, &ann).is_empty(),
        "nothing refused was kept"
    );
    // A 24-character label is the longest there is.
    let longest = wire::DeviceRegistration {
        label: "x".repeat(24),
        ..good
    };
    assert_eq!(post(&longest).0, 201);
}

#[tokio::test]
async fn a_guest_has_one_device_and_a_new_one_replaces_the_old() {
    let gw = spawn_gateway("social_keys_guest");
    let port = gw.port;
    let visitor = guest(port);
    let first = device(port, &visitor, 1);
    let (mut old_ws, _) = open(port, &visitor, &first).await;

    let second = device(port, &visitor, 2);
    let listed = list_devices(port, &visitor);
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0].id, second);
    assert_eq!(
        closing(&mut old_ws).await,
        Some(4002),
        "the replaced device's socket is closed as replaced"
    );
}

#[tokio::test]
async fn a_registered_account_at_its_cap_is_told_so_by_a_conflict() {
    let gw = spawn_gateway("social_keys_cap");
    let port = gw.port;
    let ann = account(port, "ann");
    seed_devices(&gw, "ann", 5);
    assert_eq!(list_devices(port, &ann).len(), 5);

    let (status, body) = try_register(port, &ann, &ann.tag, 6);
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("as many devices as it may"), "{body}");
    // Room made, room taken.
    let (status, _) = http(
        port,
        "DELETE",
        "/social/devices/0000000000000003",
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 204);
    device(port, &ann, 6);
    assert_eq!(list_devices(port, &ann).len(), 5);
}

/// Five new devices a day is the allowance and five devices the cap, so the
/// sixth device, over HTTP alone, would meet the allowance first; the cap
/// is asked before the allowance, so it is a conflict. Red on the code that
/// spent the allowance first.
#[tokio::test]
async fn a_sixth_device_over_http_is_a_conflict() {
    let gw = spawn_gateway("social_keys_sixth");
    let port = gw.port;
    let ann = account(port, "ann");
    for seed in 1..=5 {
        device(port, &ann, seed);
    }
    let (status, body) = try_register(port, &ann, &ann.tag, 6);
    assert_eq!(status, 409, "{body}");
}

/// Registering the device one already has is an answer and changes nothing,
/// so it costs no allowance. Red on the code that counted every call that
/// passed the statement check.
#[tokio::test]
async fn registering_the_same_device_again_is_never_rate_limited() {
    let gw = spawn_gateway("social_keys_again");
    let port = gw.port;
    let ann = account(port, "ann");
    for _ in 0..8 {
        let (status, body) = try_register(port, &ann, &ann.tag, 1);
        assert_eq!(status, 201, "{body}");
    }
}

#[tokio::test]
async fn new_devices_past_five_a_day_are_rate_limited() {
    let gw = spawn_gateway("social_keys_rate");
    let port = gw.port;
    let ann = account(port, "ann");
    for seed in 1..=5 {
        device(port, &ann, seed);
    }
    // Room for a sixth, so the cap is not what answers.
    let (status, _) = http(
        port,
        "DELETE",
        &format!(
            "/social/devices/{}",
            baylee_seal::testkit::device(5).device_id()
        ),
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 204);
    let (status, body) = try_register(port, &ann, &ann.tag, 6);
    assert_eq!(status, 429, "{body}");
    assert_eq!(list_devices(port, &ann).len(), 4);
    // The allowance is the account's own.
    let bob = account(port, "bob");
    device(port, &bob, 7);
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario, five kinds of account
async fn keys_are_handed_out_for_oneself_friends_group_members_and_table_mates_only() {
    let gw = spawn_gateway("social_keys_who");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob"); // ann's friend
    let cat = account(port, "cat"); // a stranger
    let dan = account(port, "dan"); // bob's friend, in a group with ann
    let eve = account(port, "eve"); // at ann's table
    for (n, who) in [&ann, &bob, &cat, &dan, &eve].into_iter().enumerate() {
        device(port, who, 10 + n as u64);
    }
    let everyone = [
        ann.tag.as_str(),
        bob.tag.as_str(),
        cat.tag.as_str(),
        dan.tag.as_str(),
        eve.tag.as_str(),
    ];

    // Oneself and a stranger: only oneself.
    let answer = keys(port, &ann, &everyone);
    assert_eq!(tags_of(&answer), sorted(&[&ann.tag]), "{answer:?}");
    assert_eq!(answer[0].handle, ann.handle);
    answer.iter().for_each(assert_verified);

    // A friend.
    befriend(port, &ann, &bob);
    let answer = keys(port, &ann, &everyone);
    assert_eq!(tags_of(&answer), sorted(&[&ann.tag, &bob.tag]));
    answer.iter().for_each(assert_verified);
    // Both ways round, and in the other spelling of a query.
    let (status, body) = http(
        port,
        "GET",
        &format!("/social/keys?tag={},{}", ann.tag, cat.tag),
        Some(&bob.session),
        "",
    );
    assert_eq!(status, 200, "{body}");
    let comma: wire::KeysAnswer = serde_json::from_str(&body).expect("keys");
    assert_eq!(tags_of(&comma.keys), sorted(&[&ann.tag]));

    // A group's members: invited is not yet in.
    befriend(port, &bob, &dan);
    let (status, body) = http(
        port,
        "POST",
        "/social/groups",
        Some(&ann.session),
        r#"{"name":"Crew"}"#,
    );
    assert_eq!(status, 201, "{body}");
    let group = json_field(&body, "id").to_owned();
    let invite = |from: &Account, who: &Account| {
        http(
            port,
            "POST",
            &format!("/social/groups/{group}/invites"),
            Some(&from.session),
            &serde_json::json!({ "tag": who.tag }).to_string(),
        )
    };
    assert_eq!(invite(&ann, &bob).0, 204);
    let join = |who: &Account| {
        http(
            port,
            "POST",
            &format!("/social/groups/{group}/join"),
            Some(&who.session),
            "",
        )
    };
    assert_eq!(join(&bob).0, 204);
    assert_eq!(invite(&bob, &dan).0, 204);
    let answer = keys(port, &ann, &everyone);
    assert_eq!(
        tags_of(&answer),
        sorted(&[&ann.tag, &bob.tag]),
        "an invitee is not a member"
    );
    assert_eq!(join(&dan).0, 204);
    let answer = keys(port, &ann, &everyone);
    assert_eq!(tags_of(&answer), sorted(&[&ann.tag, &bob.tag, &dan.tag]));
    answer.iter().for_each(assert_verified);
    let answer = keys(port, &dan, &everyone);
    assert_eq!(
        tags_of(&answer),
        sorted(&[&ann.tag, &bob.tag, &dan.tag]),
        "dan is a friend of bob and a member with ann; never of the rest"
    );

    // The players at one table, while they sit there.
    let (status, body) = http(
        port,
        "POST",
        "/lobby/games",
        Some(&ann.session),
        r#"{"seats":2,"name":"Kitchen"}"#,
    );
    assert_eq!(status, 200, "a waiting room: {body}");
    let game = json_field(&body, "game_id").to_owned();
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/join"),
        Some(&eve.session),
        "{}",
    );
    assert_eq!(status, 200, "{body}");
    let answer = keys(port, &ann, &everyone);
    assert_eq!(
        tags_of(&answer),
        sorted(&[&ann.tag, &bob.tag, &dan.tag, &eve.tag])
    );
    answer.iter().for_each(assert_verified);
    let answer = keys(port, &eve, &everyone);
    assert_eq!(
        tags_of(&answer),
        sorted(&[&ann.tag, &eve.tag]),
        "the table mate sees ann and itself, not ann's friends"
    );
    let (status, _) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/leave"),
        Some(&eve.session),
        "",
    );
    assert_eq!(status, 204);
    let answer = keys(port, &ann, &everyone);
    assert_eq!(
        tags_of(&answer),
        sorted(&[&ann.tag, &bob.tag, &dan.tag]),
        "she stood up"
    );

    // Throughout, the stranger was never an entry, with a device or without.
    assert!(
        answer.iter().all(|k| k.tag != cat.tag),
        "a stranger's keys: {answer:?}"
    );
}

#[tokio::test]
async fn a_key_request_is_bounded_and_needs_a_session() {
    let gw = spawn_gateway("social_keys_bounds");
    let port = gw.port;
    let ann = account(port, "ann");

    let (status, _) = http(port, "GET", "/social/keys?tag=0001", None, "");
    assert_eq!(status, 401);
    // Sixty tags is the most one request names.
    let sixty: Vec<String> = (1..=60).map(|n| format!("tag={n:04x}")).collect();
    let (status, body) = http(
        port,
        "GET",
        &format!("/social/keys?{}", sixty.join("&")),
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 200, "{body}");
    let (status, body) = http(
        port,
        "GET",
        &format!("/social/keys?{}&tag=00ff", sixty.join("&")),
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 400, "{body}");
    // Nobody and nothing: an empty answer, not a refusal.
    let (status, body) = http(
        port,
        "GET",
        "/social/keys?tag=ffffff&tag=zz&tag=",
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 200, "{body}");
    assert!(json(&body)["keys"].as_array().is_some_and(Vec::is_empty));

    // Twenty at once, then one a second: a steady stream is turned away.
    let mut refused = false;
    for _ in 0..60 {
        let (status, _) = http(port, "GET", "/social/keys?tag=0001", Some(&ann.session), "");
        if status == 429 {
            refused = true;
            break;
        }
        assert_eq!(status, 200);
    }
    assert!(refused, "sixty key fetches in a burst were all answered");
}

#[tokio::test]
async fn friends_are_told_when_the_devices_of_an_account_change() {
    let gw = spawn_gateway("social_keys_changed");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    befriend(port, &ann, &bob);
    let bob_device = device(port, &bob, 2);
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;

    let ann_device = device(port, &ann, 1);
    let tag = ann.tag.clone();
    let changed = move |d: &Down| matches!(d, Down::KeysChanged { tag: t } if *t == tag);
    wait_for(&mut bob_ws, "keys changed after a registration", &changed).await;
    let (status, _) = http(
        port,
        "DELETE",
        &format!("/social/devices/{ann_device}"),
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 204);
    wait_for(&mut bob_ws, "keys changed after a removal", &changed).await;
}

#[tokio::test]
async fn a_hello_for_a_device_that_is_not_the_accounts_is_refused_and_closed() {
    let gw = spawn_gateway("social_keys_hello");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    device(port, &ann, 1);
    let bobs = device(port, &bob, 2);

    // Another account's device.
    let mut ws = dial_hello(port, &ann, &bobs).await;
    let refused = wait_for(&mut ws, "a refusal", |d| matches!(d, Down::Refused { .. })).await;
    assert!(
        matches!(
            refused,
            Down::Refused {
                why: wire::Why::Device,
                ..
            }
        ),
        "{refused:?}"
    );
    assert!(matches!(closing(&mut ws).await, None | Some(1000)));

    // A device nobody registered.
    let mut ws = dial_hello(port, &ann, "0123456789abcdef").await;
    let refused = wait_for(&mut ws, "a refusal", |d| matches!(d, Down::Refused { .. })).await;
    assert!(
        matches!(
            refused,
            Down::Refused {
                why: wire::Why::Device,
                ..
            }
        ),
        "{refused:?}"
    );
    assert!(matches!(closing(&mut ws).await, None | Some(1000)));

    // What is not a device id at all.
    let mut ws = dial_hello(port, &ann, "not-a-device").await;
    let refused = wait_for(&mut ws, "a refusal", |d| matches!(d, Down::Refused { .. })).await;
    assert!(
        matches!(
            refused,
            Down::Refused {
                why: wire::Why::Device,
                ..
            }
        ),
        "{refused:?}"
    );
}

#[tokio::test]
async fn a_second_socket_for_the_same_device_closes_the_first_and_a_removed_device_closes_its_own()
{
    let gw = spawn_gateway("social_keys_replaced");
    let port = gw.port;
    let ann = account(port, "ann");
    let id = device(port, &ann, 1);

    let (mut first, _) = open(port, &ann, &id).await;
    let (mut second, _) = open(port, &ann, &id).await;
    assert_eq!(
        closing(&mut first).await,
        Some(4001),
        "the first socket is closed as replaced"
    );
    // The second goes on: removing the device closes it as removed.
    let (status, _) = http(
        port,
        "DELETE",
        &format!("/social/devices/{id}"),
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 204);
    assert_eq!(closing(&mut second).await, Some(4003));
}
