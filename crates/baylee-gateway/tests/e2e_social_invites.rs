//! End-to-end tests for table invites (`docs/protocol.md` §"Friends and chat
//! (beta.8)"): a player at a table invites a friend to sit down at a waiting
//! room or to watch a running table, the invite stands in for the room's
//! password once, and both sockets are told how it ended.
//!
//! Real gateways with real rooms; an agent is attached only where a game is
//! to run, which a waiting room does not need.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::social::{
    self as wire, Down, InviteAnswer, InviteKind, PresenceSet, Snapshot, Status, Up,
};
use common::{
    Socket, WAIT_BUDGET, ask_ticket, attach_agent, dial, http, json_field, login, spawn_gateway,
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

/// The invite `id` as the invitee's socket is sent it.
async fn invite_frame(ws: &mut Socket, id: &str) -> wire::TableInvite {
    let want = id.to_owned();
    let Down::Invite(invite) = wait_for(
        ws,
        "the invite",
        move |d| matches!(d, Down::Invite(i) if i.id == want),
    )
    .await
    else {
        unreachable!("waited for an invite")
    };
    invite
}

/// How invite `id` ended, as a socket is told.
async fn invite_gone(ws: &mut Socket, id: &str) -> Option<InviteAnswer> {
    let want = id.to_owned();
    let Down::InviteGone { answer, .. } = wait_for(
        ws,
        "the invite to end",
        move |d| matches!(d, Down::InviteGone { id, .. } if *id == want),
    )
    .await
    else {
        unreachable!("waited for an invite to end")
    };
    answer
}

/// A waiting room of `seats` chairs `host` opens, with `password` unless it
/// is empty. Its id.
fn room(port: u16, host: &Account, seats: u8, password: &str) -> String {
    let body = serde_json::json!({
        "seats": seats, "name": "Kitchen", "password": password
    })
    .to_string();
    let (status, answer) = http(port, "POST", "/lobby/games", Some(&host.session), &body);
    assert_eq!(status, 200, "a waiting room: {answer}");
    json_field(&answer, "game_id").to_owned()
}

/// A game `host` plays against the house, already running.
fn running_table(port: u16, host: &Account, spectators: bool) -> String {
    let deck = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, answer) = http(port, "POST", "/decks", Some(&host.session), deck);
    assert_eq!(status, 200, "a deck: {answer}");
    let body = serde_json::json!({
        "deck_id": json_field(&answer, "deck_id"),
        "mode": "ai",
        "allow_spectators": spectators,
    })
    .to_string();
    let (status, answer) = http(port, "POST", "/lobby/games", Some(&host.session), &body);
    assert_eq!(status, 200, "a game against the house: {answer}");
    json_field(&answer, "game_id").to_owned()
}

fn join(port: u16, game: &str, who: &Account, extra: &serde_json::Value) -> (u16, String) {
    http(
        port,
        "POST",
        &format!("/lobby/games/{game}/join"),
        Some(&who.session),
        &extra.to_string(),
    )
}

fn leave(port: u16, game: &str, who: &Account) {
    let (status, answer) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/leave"),
        Some(&who.session),
        "",
    );
    assert_eq!(status, 204, "leave: {answer}");
}

fn invite(port: u16, from: &Account, to: &Account, game: &str) -> (u16, String) {
    http(
        port,
        "POST",
        "/social/invites",
        Some(&from.session),
        &serde_json::json!({ "tag": to.tag, "game": game }).to_string(),
    )
}

fn close_invite(port: u16, who: &Account, id: &str, body: &str) -> (u16, String) {
    http(
        port,
        "DELETE",
        &format!("/social/invites/{id}"),
        Some(&who.session),
        body,
    )
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario, start to finish
async fn an_invite_stands_in_for_the_password_once_and_both_are_told_it_was_taken() {
    let gw = spawn_gateway("social_invites_join");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    befriend(port, &ann, &bob);
    let (ann_device, bob_device) = (device(port, &ann, 1), device(port, &bob, 2));
    let (mut ann_ws, _) = open(port, &ann, &ann_device).await;
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;

    let game = room(port, &ann, 3, "kitchen-only");
    let (status, body) = join(port, &game, &bob, &serde_json::json!({}));
    assert_eq!(status, 403, "the room is locked: {body}");

    // The invite: created, and pushed to the invitee as a way to sit down.
    let (status, body) = invite(port, &ann, &bob, &game);
    assert_eq!(status, 201, "{body}");
    let id = json_field(&body, "id").to_owned();
    let pushed = invite_frame(&mut bob_ws, &id).await;
    assert_eq!(pushed.kind, InviteKind::Join);
    assert_eq!(pushed.game, game);
    assert_eq!(pushed.from.tag, ann.tag);
    assert_eq!(pushed.from.handle, ann.handle);
    assert_eq!(pushed.table.name, "Kitchen");
    assert_eq!(
        (pushed.table.seats, pushed.table.taken, pushed.table.locked),
        (3, 1, true)
    );
    assert_eq!(pushed.table.state, "waiting");
    assert!(
        (1..=600).contains(&pushed.expires_in),
        "ten minutes at most: {}",
        pushed.expires_in
    );
    // A socket opened later is handed it with the rest.
    let (mut bob_ws, snapshot) = open(port, &bob, &bob_device).await;
    assert_eq!(snapshot.invites.len(), 1, "{:?}", snapshot.invites);
    assert_eq!(snapshot.invites[0].id, id);

    // It is bob's alone: another player's join with it is the wrong password.
    let (status, body) = join(port, &game, &cat, &serde_json::json!({ "invite": id }));
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("wrong password"), "{body}");
    // So is a made-up one.
    let (status, _) = join(
        port,
        &game,
        &bob,
        &serde_json::json!({ "invite": "01923f3c-7a2b-7c00-8000-0123456789ab" }),
    );
    assert_eq!(status, 403);

    // Bob sits down with it and no password; the invite is spent.
    let (status, body) = join(port, &game, &bob, &serde_json::json!({ "invite": id }));
    assert_eq!(status, 200, "{body}");
    assert!(!json_field(&body, "seat_token").is_empty());
    assert_eq!(
        invite_gone(&mut ann_ws, &id).await,
        Some(InviteAnswer::Accepted),
        "the inviter learns it was taken"
    );
    assert_eq!(
        invite_gone(&mut bob_ws, &id).await,
        Some(InviteAnswer::Accepted),
        "the invitee's other devices stop showing it"
    );
    let (_, snapshot) = open(port, &bob, &bob_device).await;
    assert!(snapshot.invites.is_empty(), "{:?}", snapshot.invites);

    // Once: leaving and coming back with the same invite is a locked door.
    leave(port, &game, &bob);
    let (status, body) = join(port, &game, &bob, &serde_json::json!({ "invite": id }));
    assert_eq!(status, 403, "a spent invite: {body}");
    // The password still opens it.
    let (status, body) = join(
        port,
        &game,
        &bob,
        &serde_json::json!({ "password": "kitchen-only" }),
    );
    assert_eq!(status, 200, "{body}");
}

#[tokio::test]
async fn an_invite_is_declined_withdrawn_or_replaced_and_both_sockets_are_told() {
    let gw = spawn_gateway("social_invites_end");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    befriend(port, &ann, &bob);
    let (ann_device, bob_device) = (device(port, &ann, 1), device(port, &bob, 2));
    let (mut ann_ws, _) = open(port, &ann, &ann_device).await;
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;
    let game = room(port, &ann, 3, "kitchen-only");

    // Declined by the invitee, who says so.
    let (status, body) = invite(port, &ann, &bob, &game);
    assert_eq!(status, 201, "{body}");
    let declined = json_field(&body, "id").to_owned();
    invite_frame(&mut bob_ws, &declined).await;
    let (status, body) = close_invite(port, &bob, &declined, r#"{"answer":"declined"}"#);
    assert_eq!(status, 204, "{body}");
    assert_eq!(
        invite_gone(&mut ann_ws, &declined).await,
        Some(InviteAnswer::Declined)
    );
    assert_eq!(
        invite_gone(&mut bob_ws, &declined).await,
        Some(InviteAnswer::Declined)
    );
    let (status, _) = close_invite(port, &bob, &declined, r#"{"answer":"declined"}"#);
    assert_eq!(status, 404, "closed already");
    let (status, _) = join(
        port,
        &game,
        &bob,
        &serde_json::json!({ "invite": declined }),
    );
    assert_eq!(status, 403, "a declined invite opens nothing");

    // Withdrawn by the inviter: nobody answered it.
    let (status, body) = invite(port, &ann, &bob, &game);
    assert_eq!(status, 201, "{body}");
    let withdrawn = json_field(&body, "id").to_owned();
    invite_frame(&mut bob_ws, &withdrawn).await;
    let (status, _) = close_invite(port, &cat, &withdrawn, "");
    assert_eq!(status, 404, "a stranger to it finds none");
    let (status, body) = close_invite(port, &ann, &withdrawn, "");
    assert_eq!(status, 204, "{body}");
    assert_eq!(invite_gone(&mut bob_ws, &withdrawn).await, None);
    assert_eq!(invite_gone(&mut ann_ws, &withdrawn).await, None);

    // The invitee lets it go without a word: no answer either.
    let (status, body) = invite(port, &ann, &bob, &game);
    assert_eq!(status, 201, "{body}");
    let ignored = json_field(&body, "id").to_owned();
    invite_frame(&mut bob_ws, &ignored).await;
    let (status, _) = close_invite(port, &bob, &ignored, "");
    assert_eq!(status, 204);
    assert_eq!(invite_gone(&mut ann_ws, &ignored).await, None);

    // The same inviter asking the same friend for the same table again
    // replaces the first.
    let (_, body) = invite(port, &ann, &bob, &game);
    let first = json_field(&body, "id").to_owned();
    invite_frame(&mut bob_ws, &first).await;
    let (status, body) = invite(port, &ann, &bob, &game);
    assert_eq!(status, 201, "{body}");
    let second = json_field(&body, "id").to_owned();
    assert_ne!(first, second);
    assert_eq!(invite_gone(&mut bob_ws, &first).await, None);
    invite_frame(&mut bob_ws, &second).await;
    let (status, _) = join(port, &game, &bob, &serde_json::json!({ "invite": first }));
    assert_eq!(status, 403, "the replaced one opens nothing");
    let (status, _) = join(port, &game, &bob, &serde_json::json!({ "invite": second }));
    assert_eq!(status, 200);
}

#[tokio::test]
async fn only_a_friend_is_invited_and_only_to_a_table_the_inviter_sits_at() {
    let gw = spawn_gateway("social_invites_refused");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    let dan = account(port, "dan");
    let eve = account(port, "eve");
    befriend(port, &ann, &bob);
    befriend(port, &bob, &cat);
    befriend(port, &ann, &cat);

    let mine = room(port, &ann, 3, "");
    let cats = room(port, &cat, 2, "");

    let (status, body) = invite(port, &ann, &dan, &mine);
    assert_eq!(status, 403, "a stranger: {body}");
    // A friend, to a table the inviter is not at, or to none at all.
    let (status, body) = invite(port, &ann, &bob, &cats);
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("do not sit at that table"), "{body}");
    let (status, _) = invite(port, &ann, &bob, "no-such-game");
    assert_eq!(status, 403);
    // The table a friend sits at is the friend's to invite to, not a bystander's.
    let (status, _) = invite(port, &bob, &ann, &mine);
    assert_eq!(status, 403, "bob does not sit at ann's table");
    // Seated, an invitee may invite on: cat sits down with ann.
    let (status, body) = join(port, &mine, &cat, &serde_json::json!({}));
    assert_eq!(status, 200, "{body}");
    let (status, body) = invite(port, &cat, &bob, &mine);
    assert_eq!(status, 201, "a player seated there: {body}");

    // A room without a free chair has nothing to offer a friend.
    befriend(port, &ann, &eve);
    let (status, body) = invite(port, &ann, &eve, &mine);
    assert_eq!(status, 201, "one chair is free: {body}");
    let (status, body) = join(port, &mine, &bob, &serde_json::json!({}));
    assert_eq!(status, 200, "{body}");
    let (status, body) = invite(port, &ann, &eve, &mine);
    assert_eq!(status, 403, "no chair is free now: {body}");
    assert!(body.contains("no chair or watching"), "{body}");

    // A guest has no friends to invite.
    let (status, body) = http(port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 200, "{body}");
    let guest = Account {
        session: json_field(&body, "token").to_owned(),
        tag: ann.tag.clone(),
        handle: ann.handle.clone(),
    };
    let (status, _) = invite(port, &guest, &ann, &mine);
    assert_eq!(status, 403);
}

#[tokio::test]
async fn a_running_table_is_offered_to_watch_only_when_it_allows_spectators() {
    let gw = spawn_gateway("social_invites_watch");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    befriend(port, &ann, &bob);
    befriend(port, &cat, &bob);
    let bob_device = device(port, &bob, 2);
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;

    let closed = running_table(port, &ann, false);
    let (status, body) = invite(port, &ann, &bob, &closed);
    assert_eq!(status, 403, "a table that allows no spectators: {body}");

    let open_table = running_table(port, &cat, true);
    let (status, body) = invite(port, &cat, &bob, &open_table);
    assert_eq!(status, 201, "{body}");
    let id = json_field(&body, "id").to_owned();
    let pushed = invite_frame(&mut bob_ws, &id).await;
    assert_eq!(pushed.kind, InviteKind::Watch);
    assert_eq!(pushed.table.state, "playing");
    assert_eq!(pushed.game, open_table);

    // A watch invite points at the table; it is no chair and no password.
    let (status, body) = join(
        port,
        &open_table,
        &bob,
        &serde_json::json!({ "invite": id }),
    );
    assert_eq!(status, 409, "a running table takes no new player: {body}");
    let (status, body) = close_invite(port, &bob, &id, r#"{"answer":"declined"}"#);
    assert_eq!(status, 204, "{body}");
}
