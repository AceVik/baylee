//! End-to-end tests for deleting an account that has a social side
//! (`DELETE /account`, #292, and `docs/protocol.md` §"Friends and chat
//! (beta.8)"): its sockets close, its friends are told, its groups pass on
//! or go, its table invites lapse, and not a row of it is left.
//!
//! Asked of a real gateway over HTTP and sockets, and checked twice: by what
//! the routes and sockets say afterwards, and by what is left in the
//! database's own tables, because a socket that closed says nothing about a
//! row nobody reads.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::social::{self as wire, Down, FriendEvent, PresenceSet, Snapshot, Status, Up};
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

fn ask(port: u16, a: &Account, b: &Account) {
    let body = serde_json::json!({ "handle": b.handle }).to_string();
    let (status, answer) = http(
        port,
        "POST",
        "/social/friends/requests",
        Some(&a.session),
        &body,
    );
    assert_eq!(status, 201, "{answer}");
}

fn befriend(port: u16, a: &Account, b: &Account) {
    ask(port, a, b);
    let path = format!("/social/friends/requests/{}/accept", a.tag);
    let (status, answer) = http(port, "POST", &path, Some(&b.session), "");
    assert_eq!(status, 204, "{answer}");
}

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
    public.id
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

/// Whether the socket ends within the budget, whatever frames come first.
async fn ends(ws: &mut Socket) -> bool {
    tokio::time::timeout(WAIT_BUDGET, async {
        loop {
            match ws.next().await {
                Some(Ok(Message::Close(_)) | Err(_)) | None => return,
                Some(Ok(_)) => {}
            }
        }
    })
    .await
    .is_ok()
}

/// Opens the social socket of `who` as `device` and reads through the
/// welcome and the snapshot.
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

fn make_group(port: u16, owner: &Account, name: &str) -> String {
    let body = serde_json::json!({ "name": name }).to_string();
    let (status, answer) = http(port, "POST", "/social/groups", Some(&owner.session), &body);
    assert_eq!(status, 201, "make a group: {answer}");
    json_field(&answer, "id").to_owned()
}

fn invite_and_join(port: u16, group: &str, from: &Account, who: &Account) {
    let (status, body) = http(
        port,
        "POST",
        &format!("/social/groups/{group}/invites"),
        Some(&from.session),
        &serde_json::json!({ "tag": who.tag }).to_string(),
    );
    assert_eq!(status, 204, "{body}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/social/groups/{group}/join"),
        Some(&who.session),
        "",
    );
    assert_eq!(status, 204, "{body}");
}

/// How many rows of the social tables name the account `id`.
fn social_rows(gw: &Gateway, id: &str) -> Vec<(&'static str, i64)> {
    [
        ("social_device", "account_id = '{id}'"),
        ("friendship", "a = '{id}' OR b = '{id}'"),
        ("friend_request", "from_id = '{id}' OR to_id = '{id}'"),
        ("social_block", "blocker = '{id}' OR blocked = '{id}'"),
        ("group_member", "account_id = '{id}'"),
        ("group_invite", "account_id = '{id}' OR inviter_id = '{id}'"),
        ("chat_message", "sender_id = '{id}'"),
    ]
    .into_iter()
    .map(|(table, filter)| {
        let filter = filter.replace("{id}", id);
        (
            table,
            gw.scalar(&format!("SELECT count(*) FROM {table} WHERE {filter}")),
        )
    })
    .collect()
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario, start to finish
async fn deleting_an_account_closes_its_socket_tells_its_friends_and_leaves_no_row() {
    let gw = spawn_gateway("social_depart");
    let port = gw.port;
    let ann = account(port, "ann"); // the leaver
    let bob = account(port, "bob"); // her friend, in her group
    let cat = account(port, "cat"); // asked her, and is blocked
    let dan = account(port, "dan"); // she asked him
    let (ann_device, bob_device) = (device(port, &ann, 1), device(port, &bob, 2));
    device(port, &cat, 3);

    befriend(port, &ann, &bob);
    ask(port, &cat, &ann);
    ask(port, &ann, &dan);
    let block = serde_json::json!({ "tag": cat.tag }).to_string();
    let (status, _) = http(port, "POST", "/social/blocks", Some(&ann.session), &block);
    assert_eq!(status, 204);
    // A group she owns with bob in it, one she is alone in, and one she is
    // invited to and bob owns.
    let shared = make_group(port, &ann, "Shared");
    invite_and_join(port, &shared, &ann, &bob);
    let alone = make_group(port, &ann, "Alone");
    let bobs = make_group(port, &bob, "Bobs");
    let (status, _) = http(
        port,
        "POST",
        &format!("/social/groups/{bobs}/invites"),
        Some(&bob.session),
        &serde_json::json!({ "tag": ann.tag }).to_string(),
    );
    assert_eq!(status, 204);

    let ann_id = gw.text("SELECT id::text FROM account WHERE username = 'ann'");
    let bob_id = gw.text("SELECT id::text FROM account WHERE username = 'bob'");
    let before = social_rows(&gw, &ann_id);
    for (table, rows) in &before {
        if *table != "chat_message" {
            assert!(*rows > 0, "ann has {table} rows to lose: {before:?}");
        }
    }
    let bob_before = social_rows(&gw, &bob_id);

    let (mut ann_ws, _) = open(port, &ann, &ann_device).await;
    let (mut bob_ws, bob_start) = open(port, &bob, &bob_device).await;
    assert!(bob_start.friends.iter().any(|f| f.person.tag == ann.tag));

    // Goodbye.
    let (status, answer) = http(port, "DELETE", "/account", Some(&ann.session), "{}");
    assert_eq!(status, 204, "{answer}");

    // Her socket closes; her friend is told the friendship is gone.
    assert!(ends(&mut ann_ws).await, "ann's socket stayed open");
    let tag = ann.tag.clone();
    let Down::Friend { person, .. } = wait_for(&mut bob_ws, "the friendship to end", move |d| {
        matches!(d, Down::Friend { event: FriendEvent::Removed, person, .. } if person.tag == tag)
    })
    .await
    else {
        unreachable!()
    };
    assert_eq!(person.handle, ann.handle);

    // She cannot be found, signed in as, or written to.
    let path = format!("/social/players/{}", ann.handle.replace('#', "%23"));
    let (status, body) = http(port, "GET", &path, Some(&bob.session), "");
    assert_eq!(status, 404, "{body}");
    let (status, _) = http(port, "GET", "/me", Some(&ann.session), "");
    assert_eq!(status, 401);
    let (status, _) = http(
        port,
        "POST",
        &format!("/social/friends/requests/{}/accept", ann.tag),
        Some(&dan.session),
        "",
    );
    assert_eq!(status, 404, "her request to dan went with her");

    // The group she owned passes to bob, the one only she was in is gone.
    let (_, snapshot) = open(port, &bob, &bob_device).await;
    let shared_members: Vec<(String, bool)> = snapshot
        .groups
        .iter()
        .find(|g| g.id == shared)
        .expect("the shared group stands")
        .members
        .iter()
        .map(|m| (m.person.tag.clone(), m.owner))
        .collect();
    assert_eq!(shared_members, vec![(bob.tag.clone(), true)]);
    assert!(
        snapshot.groups.iter().all(|g| g.id != alone),
        "{:?}",
        snapshot.groups
    );
    let bobs_group = snapshot
        .groups
        .iter()
        .find(|g| g.id == bobs)
        .expect("bob's own group stands");
    assert!(
        bobs_group.invited.is_empty(),
        "her invite went: {bobs_group:?}"
    );
    assert!(snapshot.friends.is_empty(), "{:?}", snapshot.friends);
    let (status, _) = http(
        port,
        "PATCH",
        &format!("/social/groups/{shared}"),
        Some(&bob.session),
        r#"{"name":"Bob's now"}"#,
    );
    assert_eq!(status, 204, "bob owns it");
    let (status, _) = http(
        port,
        "PATCH",
        &format!("/social/groups/{alone}"),
        Some(&bob.session),
        r#"{"name":"Ghost"}"#,
    );
    assert_eq!(status, 404, "nobody was left in it");

    // Not a row of hers is left, and bob's are.
    for (table, rows) in social_rows(&gw, &ann_id) {
        assert_eq!(rows, 0, "{table} still names the leaver");
    }
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM chat_group WHERE id = '{alone}'"
        )),
        0
    );
    let bob_after = social_rows(&gw, &bob_id);
    let kept =
        |rows: &[(&str, i64)], table: &str| rows.iter().find(|(t, _)| *t == table).map(|(_, n)| *n);
    assert_eq!(
        kept(&bob_after, "social_device"),
        kept(&bob_before, "social_device")
    );
    assert_eq!(
        kept(&bob_after, "group_member"),
        kept(&bob_before, "group_member"),
        "bob is in the same two groups"
    );
}

#[tokio::test]
async fn deleting_an_account_withdraws_its_table_invites_and_a_guest_leaves_as_cleanly() {
    let gw = spawn_gateway("social_depart_invites");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    befriend(port, &ann, &bob);
    let bob_device = device(port, &bob, 2);
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;

    let room = {
        let body = r#"{"seats":3,"name":"Kitchen"}"#;
        let (status, answer) = http(port, "POST", "/lobby/games", Some(&ann.session), body);
        assert_eq!(status, 200, "a waiting room: {answer}");
        json_field(&answer, "game_id").to_owned()
    };
    let (status, body) = http(
        port,
        "POST",
        "/social/invites",
        Some(&ann.session),
        &serde_json::json!({ "tag": bob.tag, "game": room }).to_string(),
    );
    assert_eq!(status, 201, "{body}");
    let id = json_field(&body, "id").to_owned();
    wait_for(
        &mut bob_ws,
        "the invite",
        |d| matches!(d, Down::Invite(i) if i.id == id),
    )
    .await;

    let (status, answer) = http(port, "DELETE", "/account", Some(&ann.session), "{}");
    assert_eq!(status, 204, "{answer}");
    let Down::InviteGone { answer, .. } = wait_for(
        &mut bob_ws,
        "the invite to lapse",
        |d| matches!(d, Down::InviteGone { id: gone, .. } if *gone == id),
    )
    .await
    else {
        unreachable!()
    };
    assert_eq!(answer, None, "nobody answered it");
    let (_, snapshot) = open(port, &bob, &bob_device).await;
    assert!(snapshot.invites.is_empty(), "{:?}", snapshot.invites);

    // A guest that has a device and a socket goes with its last session.
    let (status, body) = http(port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 200, "{body}");
    let visitor = json_field(&body, "token").to_owned();
    let (status, me) = http(port, "GET", "/me", Some(&visitor), "");
    assert_eq!(status, 200, "{me}");
    let visitor = Account {
        session: visitor,
        tag: json(&me)["tag"].as_str().expect("a tag").to_owned(),
        handle: json(&me)["handle"].as_str().expect("a handle").to_owned(),
    };
    let guest_device = device(port, &visitor, 9);
    let (mut guest_ws, _) = open(port, &visitor, &guest_device).await;
    let (status, answer) = http(port, "DELETE", "/account", Some(&visitor.session), "{}");
    assert_eq!(status, 204, "{answer}");
    assert!(ends(&mut guest_ws).await, "the guest's socket stayed open");
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM social_device WHERE id = '{guest_device}'"
        )),
        0
    );
}
