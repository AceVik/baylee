//! End-to-end tests for chat groups (`docs/protocol.md` §"Friends and chat
//! (beta.8)"): making one, inviting friends, joining, renaming, handing it
//! on, leaving and removing, what happens to the owner's chair when the
//! owner goes, and what each member's open socket is told.
//!
//! Real gateways, real sockets. A fresh gateway per test, so the allowances
//! (ten new groups a day, thirty invites an hour) start full in each.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::social::{
    self as wire, Down, GroupEvent, GroupInfo, Notice, PresenceSet, Snapshot, Status, Up,
};
use common::{Socket, WAIT_BUDGET, ask_ticket, dial, http, json_field, login, spawn_gateway};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

/// An account as the tests drive it: its session, its tag and its handle,
/// and the chat device it will open a socket with.
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

/// Registers the chat device drawn from `seed` for `who`.
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

/// The group frame `event` about group `id`, and the group as it was sent.
async fn group_frame(ws: &mut Socket, event: GroupEvent, id: &str) -> GroupInfo {
    let want = id.to_owned();
    let Down::Group { group, .. } = wait_for(
        ws,
        &format!("{event:?} of {id}"),
        move |d| matches!(d, Down::Group { event: e, group, .. } if *e == event && group.id == want),
    )
    .await
    else {
        unreachable!("waited for a group frame")
    };
    group
}

/// The notice `want` picks out in the conversation of group `id`.
async fn notice(ws: &mut Socket, id: &str, want: impl Fn(&Notice) -> bool) -> Notice {
    let conv = format!("grp:{id}");
    let Down::Notice { notice, .. } = wait_for(
        ws,
        "a notice",
        move |d| matches!(d, Down::Notice { conv: c, notice } if *c == conv && want(notice)),
    )
    .await
    else {
        unreachable!("waited for a notice")
    };
    notice
}

fn make_group(port: u16, owner: &Account, name: &str) -> String {
    let body = serde_json::json!({ "name": name }).to_string();
    let (status, answer) = http(port, "POST", "/social/groups", Some(&owner.session), &body);
    assert_eq!(status, 201, "make a group: {answer}");
    json_field(&answer, "id").to_owned()
}

fn invite(port: u16, group: &str, from: &Account, to: &Account) -> (u16, String) {
    http(
        port,
        "POST",
        &format!("/social/groups/{group}/invites"),
        Some(&from.session),
        &serde_json::json!({ "tag": to.tag }).to_string(),
    )
}

fn join(port: u16, group: &str, who: &Account) -> (u16, String) {
    http(
        port,
        "POST",
        &format!("/social/groups/{group}/join"),
        Some(&who.session),
        "",
    )
}

/// `who` is invited by `from` and joins.
fn invite_and_join(port: u16, group: &str, from: &Account, who: &Account) {
    let (status, body) = invite(port, group, from, who);
    assert_eq!(status, 204, "{body}");
    let (status, body) = join(port, group, who);
    assert_eq!(status, 204, "{body}");
}

fn rename(port: u16, group: &str, who: &Account, name: &str) -> (u16, String) {
    http(
        port,
        "PATCH",
        &format!("/social/groups/{group}"),
        Some(&who.session),
        &serde_json::json!({ "name": name }).to_string(),
    )
}

fn remove(port: u16, group: &str, by: &Account, member: &Account) -> (u16, String) {
    http(
        port,
        "DELETE",
        &format!("/social/groups/{group}/members/{}", member.tag),
        Some(&by.session),
        "",
    )
}

/// The members of the group in `snapshot`, in the order they joined, as
/// `(tag, owner)`.
fn members_in(snapshot: &Snapshot, group: &str) -> Vec<(String, bool)> {
    snapshot
        .groups
        .iter()
        .find(|g| g.id == group)
        .unwrap_or_else(|| panic!("no group {group} in {:?}", snapshot.groups))
        .members
        .iter()
        .map(|m| (m.person.tag.clone(), m.owner))
        .collect()
}

#[tokio::test]
async fn a_group_is_made_by_a_registered_account_with_a_name_that_fits() {
    let gw = spawn_gateway("social_groups_create");
    let port = gw.port;
    let ann = account(port, "ann");

    let id = make_group(port, &ann, "Crew");
    assert!(wire::valid_uuid(&id), "{id}");
    assert_ne!(make_group(port, &ann, "Crew"), id, "a name is not an id");
    // 48 characters, and in any script, are the longest there are.
    make_group(port, &ann, &"a".repeat(48));
    make_group(port, &ann, "Café 日本語 ✨");

    for (name, why) in [
        (String::new(), "empty"),
        ("   ".into(), "blank"),
        ("a".repeat(49), "49 characters"),
        ("\u{202E}Crew".into(), "a bidi override"),
        ("Cr\u{200B}ew".into(), "a zero-width space"),
        ("Cr\new".into(), "a control character"),
    ] {
        let body = serde_json::json!({ "name": name }).to_string();
        let (status, answer) = http(port, "POST", "/social/groups", Some(&ann.session), &body);
        assert_eq!(status, 400, "{why}: {answer}");
    }

    // A guest has no groups.
    let (status, body) = http(port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 200, "{body}");
    let guest = json_field(&body, "token").to_owned();
    let (status, answer) = http(
        port,
        "POST",
        "/social/groups",
        Some(&guest),
        r#"{"name":"Crew"}"#,
    );
    assert_eq!(status, 403, "{answer}");
    let (status, _) = http(port, "POST", "/social/groups", None, r#"{"name":"Crew"}"#);
    assert_eq!(status, 401);
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one scenario
async fn a_friend_is_invited_told_and_joins_and_the_members_are_told_of_it() {
    let gw = spawn_gateway("social_groups_invite");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat"); // a friend of bob, not of ann
    let dan = account(port, "dan"); // nobody's friend
    befriend(port, &ann, &bob);
    befriend(port, &bob, &cat);
    let (ann_device, bob_device) = (device(port, &ann, 1), device(port, &bob, 2));
    let (mut ann_ws, _) = open(port, &ann, &ann_device).await;
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;

    let group = make_group(port, &ann, "Crew");
    let made = group_frame(&mut ann_ws, GroupEvent::Joined, &group).await;
    assert_eq!(made.name, "Crew");
    assert_eq!(
        made.members
            .iter()
            .map(|m| (&m.person.tag, m.owner))
            .collect::<Vec<_>>(),
        vec![(&ann.tag, true)],
        "the maker owns it"
    );

    // Whom an account may invite: its own friends, and only as a member.
    let (status, body) = invite(port, &group, &ann, &dan);
    assert_eq!(status, 403, "a stranger: {body}");
    let (status, body) = invite(port, &group, &ann, &cat);
    assert_eq!(status, 403, "a friend of another member: {body}");
    let (status, body) = invite(port, &group, &bob, &cat);
    assert_eq!(status, 403, "bob is not in the group yet: {body}");
    let (status, _) = invite(port, "01923f3c-7a2b-7c00-8000-0123456789ab", &ann, &bob);
    assert_eq!(status, 404, "no such group");
    let (status, _) = invite(port, "not-a-group", &ann, &bob);
    assert_eq!(status, 404);

    // The invitation arrives as an invite, and again changes nothing.
    let (status, body) = invite(port, &group, &ann, &bob);
    assert_eq!(status, 204, "{body}");
    let invited = group_frame(&mut bob_ws, GroupEvent::Invited, &group).await;
    assert_eq!(
        invited.invited.iter().map(|p| &p.tag).collect::<Vec<_>>(),
        vec![&bob.tag]
    );
    assert_eq!(invite(port, &group, &ann, &bob).0, 204);
    let updated = group_frame(&mut ann_ws, GroupEvent::Updated, &group).await;
    assert_eq!(updated.invited.len(), 1, "the owner is told of the invitee");

    // A new socket sees the invitation in its snapshot, and the group not
    // yet among bob's.
    let (_, snapshot) = open(port, &bob, &bob_device).await;
    assert_eq!(snapshot.group_invites.len(), 1);
    assert_eq!(snapshot.group_invites[0].group, group);
    assert_eq!(snapshot.group_invites[0].inviter.tag, ann.tag);
    assert!(snapshot.groups.is_empty());

    // Nobody but the invitee joins with it.
    let (status, _) = join(port, &group, &cat);
    assert_eq!(status, 404, "no invite to this group");
    let (status, _) = join(port, &group, &dan);
    assert_eq!(status, 404);
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;
    let (status, body) = join(port, &group, &bob);
    assert_eq!(status, 204, "{body}");
    let joined = group_frame(&mut bob_ws, GroupEvent::Joined, &group).await;
    assert_eq!(joined.members.len(), 2);
    assert!(joined.invited.is_empty());
    // The members: the group changed, and a line says who joined.
    let updated = group_frame(&mut ann_ws, GroupEvent::Updated, &group).await;
    assert_eq!(
        updated
            .members
            .iter()
            .map(|m| (&m.person.tag, m.owner))
            .collect::<Vec<_>>(),
        vec![(&ann.tag, true), (&bob.tag, false)],
        "in the order they joined"
    );
    let Notice::MemberJoined { person } = notice(&mut ann_ws, &group, |n| {
        matches!(n, Notice::MemberJoined { .. })
    })
    .await
    else {
        unreachable!()
    };
    assert_eq!(person.tag, bob.tag);
    notice(&mut bob_ws, &group, |n| {
        matches!(n, Notice::MemberJoined { .. })
    })
    .await;

    // A member is not invited a second time, nor joins a second time.
    let (status, body) = invite(port, &group, &ann, &bob);
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("already a member"), "{body}");
    let (status, _) = join(port, &group, &bob);
    assert_eq!(status, 404, "the invite was spent");
    // A member invites its own friends now.
    let (status, body) = invite(port, &group, &bob, &cat);
    assert_eq!(status, 204, "{body}");
}

#[tokio::test]
async fn an_invite_is_declined_by_the_invitee_or_withdrawn_by_its_inviter() {
    let gw = spawn_gateway("social_groups_decline");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    befriend(port, &ann, &bob);
    befriend(port, &ann, &cat);
    let (bob_device, cat_device) = (device(port, &bob, 2), device(port, &cat, 3));
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;
    let (mut cat_ws, _) = open(port, &cat, &cat_device).await;
    let group = make_group(port, &ann, "Crew");

    let invite_path = |who: &Account| format!("/social/groups/{group}/invites/{}", who.tag);
    assert_eq!(invite(port, &group, &ann, &bob).0, 204);
    group_frame(&mut bob_ws, GroupEvent::Invited, &group).await;
    // The invitee says no.
    let (status, body) = http(port, "DELETE", &invite_path(&bob), Some(&bob.session), "");
    assert_eq!(status, 204, "{body}");
    group_frame(&mut bob_ws, GroupEvent::InviteGone, &group).await;
    assert_eq!(join(port, &group, &bob).0, 404, "nothing to join with");
    let (status, _) = http(port, "DELETE", &invite_path(&bob), Some(&bob.session), "");
    assert_eq!(status, 404, "declined already");

    // The inviter takes it back; a stranger to it cannot.
    assert_eq!(invite(port, &group, &ann, &cat).0, 204);
    group_frame(&mut cat_ws, GroupEvent::Invited, &group).await;
    let (status, _) = http(port, "DELETE", &invite_path(&cat), Some(&bob.session), "");
    assert_eq!(status, 403, "not bob's to withdraw");
    let (status, body) = http(port, "DELETE", &invite_path(&cat), Some(&ann.session), "");
    assert_eq!(status, 204, "{body}");
    group_frame(&mut cat_ws, GroupEvent::InviteGone, &group).await;
    assert_eq!(join(port, &group, &cat).0, 404);
}

#[tokio::test]
async fn only_the_owner_renames_a_group_and_the_members_are_told() {
    let gw = spawn_gateway("social_groups_rename");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    befriend(port, &ann, &bob);
    let bob_device = device(port, &bob, 2);
    let group = make_group(port, &ann, "Crew");
    invite_and_join(port, &group, &ann, &bob);
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;

    let (status, body) = rename(port, &group, &bob, "Mutiny");
    assert_eq!(status, 403, "a member: {body}");
    let (status, body) = rename(port, &group, &ann, &"a".repeat(49));
    assert_eq!(status, 400, "{body}");
    let (status, body) = rename(port, &group, &ann, "");
    assert_eq!(status, 400, "{body}");
    let (status, body) = rename(port, &group, &ann, "\u{202E}Crew");
    assert_eq!(status, 400, "{body}");
    let (status, _) = rename(port, "01923f3c-7a2b-7c00-8000-0123456789ab", &ann, "Crew");
    assert_eq!(status, 404);

    let (status, body) = rename(port, &group, &ann, "Fleet");
    assert_eq!(status, 204, "{body}");
    let renamed = group_frame(&mut bob_ws, GroupEvent::Updated, &group).await;
    assert_eq!(renamed.name, "Fleet");
    let Notice::Renamed { name } =
        notice(&mut bob_ws, &group, |n| matches!(n, Notice::Renamed { .. })).await
    else {
        unreachable!()
    };
    assert_eq!(name, "Fleet");
}

#[tokio::test]
async fn the_owner_hands_a_group_on_and_only_to_a_member() {
    let gw = spawn_gateway("social_groups_hand_over");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    befriend(port, &ann, &bob);
    befriend(port, &ann, &cat);
    let (ann_device, bob_device) = (device(port, &ann, 1), device(port, &bob, 2));
    let group = make_group(port, &ann, "Crew");
    invite_and_join(port, &group, &ann, &bob);
    assert_eq!(invite(port, &group, &ann, &cat).0, 204);
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;

    let hand = |from: &Account, to: &Account| {
        http(
            port,
            "POST",
            &format!("/social/groups/{group}/owner"),
            Some(&from.session),
            &serde_json::json!({ "tag": to.tag }).to_string(),
        )
    };
    assert_eq!(hand(&bob, &bob).0, 403, "a member hands nothing on");
    assert_eq!(hand(&ann, &cat).0, 404, "an invitee is not a member");
    assert_eq!(hand(&ann, &ann).0, 404, "nor is oneself the next owner");
    let (status, body) = hand(&ann, &bob);
    assert_eq!(status, 204, "{body}");
    let Notice::OwnerNow { person } = notice(&mut bob_ws, &group, |n| {
        matches!(n, Notice::OwnerNow { .. })
    })
    .await
    else {
        unreachable!()
    };
    assert_eq!(person.tag, bob.tag);

    // The chair went with it.
    assert_eq!(rename(port, &group, &ann, "Mine").0, 403);
    assert_eq!(rename(port, &group, &bob, "Bob's").0, 204);
    let (_, snapshot) = open(port, &ann, &ann_device).await;
    assert_eq!(
        members_in(&snapshot, &group),
        vec![(ann.tag.clone(), false), (bob.tag.clone(), true)]
    );
}

#[tokio::test]
async fn a_member_leaves_and_the_owner_removes_but_a_member_cannot() {
    let gw = spawn_gateway("social_groups_leave");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    let dan = account(port, "dan");
    for friend in [&bob, &cat, &dan] {
        befriend(port, &ann, friend);
    }
    let (ann_device, bob_device, cat_device) = (
        device(port, &ann, 1),
        device(port, &bob, 2),
        device(port, &cat, 3),
    );
    let group = make_group(port, &ann, "Crew");
    for friend in [&bob, &cat] {
        invite_and_join(port, &group, &ann, friend);
    }
    let (mut ann_ws, _) = open(port, &ann, &ann_device).await;
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;
    let (mut cat_ws, _) = open(port, &cat, &cat_device).await;

    // A member cannot remove another one, and the group has no one else.
    let (status, body) = remove(port, &group, &cat, &bob);
    assert_eq!(status, 403, "{body}");
    let (status, _) = remove(port, &group, &ann, &dan);
    assert_eq!(status, 404, "dan was never in it");

    // A member leaves with its own tag.
    let (status, body) = remove(port, &group, &bob, &bob);
    assert_eq!(status, 204, "{body}");
    let left = group_frame(&mut bob_ws, GroupEvent::Left, &group).await;
    assert_eq!(left.id, group);
    let Notice::MemberLeft { person } = notice(&mut ann_ws, &group, |n| {
        matches!(n, Notice::MemberLeft { .. })
    })
    .await
    else {
        unreachable!()
    };
    assert_eq!(person.tag, bob.tag);
    let (status, _) = remove(port, &group, &bob, &bob);
    assert_eq!(status, 404, "not a member any more");
    assert_eq!(rename(port, &group, &bob, "Back").0, 403);

    // The owner removes another.
    let (status, body) = remove(port, &group, &ann, &cat);
    assert_eq!(status, 204, "{body}");
    group_frame(&mut cat_ws, GroupEvent::Left, &group).await;
    let (_, snapshot) = open(port, &ann, &ann_device).await;
    assert_eq!(members_in(&snapshot, &group), vec![(ann.tag.clone(), true)]);
    let (_, snapshot) = open(port, &cat, &cat_device).await;
    assert!(snapshot.groups.is_empty(), "{:?}", snapshot.groups);
}

#[tokio::test]
async fn when_the_owner_leaves_the_member_who_joined_first_owns_the_group() {
    let gw = spawn_gateway("social_groups_reown");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    befriend(port, &ann, &bob);
    befriend(port, &ann, &cat);
    let (bob_device, cat_device) = (device(port, &bob, 2), device(port, &cat, 3));
    let group = make_group(port, &ann, "Crew");
    // Cat is invited first and joins last: the order is joining, not asking.
    assert_eq!(invite(port, &group, &ann, &cat).0, 204);
    invite_and_join(port, &group, &ann, &bob);
    assert_eq!(join(port, &group, &cat).0, 204);
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;
    let (mut cat_ws, _) = open(port, &cat, &cat_device).await;

    let (status, body) = remove(port, &group, &ann, &ann);
    assert_eq!(status, 204, "{body}");
    for ws in [&mut bob_ws, &mut cat_ws] {
        let Notice::OwnerNow { person } =
            notice(ws, &group, |n| matches!(n, Notice::OwnerNow { .. })).await
        else {
            unreachable!()
        };
        assert_eq!(person.tag, bob.tag, "the first to join");
    }
    let (_, snapshot) = open(port, &cat, &cat_device).await;
    assert_eq!(
        members_in(&snapshot, &group),
        vec![(bob.tag.clone(), true), (cat.tag.clone(), false)]
    );
    assert_eq!(rename(port, &group, &bob, "Bob's").0, 204);
    assert_eq!(rename(port, &group, &cat, "Cat's").0, 403);
    assert_eq!(rename(port, &group, &ann, "Hers").0, 403, "ann is out");
}

#[tokio::test]
async fn a_group_nobody_is_in_is_gone_and_its_invitees_are_told() {
    let gw = spawn_gateway("social_groups_last");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    befriend(port, &ann, &bob);
    let bob_device = device(port, &bob, 2);
    let group = make_group(port, &ann, "Crew");
    assert_eq!(invite(port, &group, &ann, &bob).0, 204);
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;

    let (status, body) = remove(port, &group, &ann, &ann);
    assert_eq!(status, 204, "{body}");
    let gone = group_frame(&mut bob_ws, GroupEvent::Deleted, &group).await;
    assert_eq!(gone.name, "Crew");
    assert_eq!(rename(port, &group, &ann, "Again").0, 404, "no such group");
    assert_eq!(join(port, &group, &bob).0, 404, "nor an invite to it");
    let (status, _) = http(
        port,
        "DELETE",
        &format!("/social/groups/{group}"),
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 404);
}

#[tokio::test]
async fn the_owner_deletes_a_group_and_members_and_invitees_are_told() {
    let gw = spawn_gateway("social_groups_delete");
    let port = gw.port;
    let ann = account(port, "ann");
    let bob = account(port, "bob");
    let cat = account(port, "cat");
    befriend(port, &ann, &bob);
    befriend(port, &ann, &cat);
    let (bob_device, cat_device) = (device(port, &bob, 2), device(port, &cat, 3));
    let group = make_group(port, &ann, "Crew");
    invite_and_join(port, &group, &ann, &bob);
    assert_eq!(invite(port, &group, &ann, &cat).0, 204);
    let (mut bob_ws, _) = open(port, &bob, &bob_device).await;
    let (mut cat_ws, _) = open(port, &cat, &cat_device).await;

    let path = format!("/social/groups/{group}");
    let (status, body) = http(port, "DELETE", &path, Some(&bob.session), "");
    assert_eq!(status, 403, "a member does not delete: {body}");
    let (status, body) = http(port, "DELETE", &path, Some(&ann.session), "");
    assert_eq!(status, 204, "{body}");
    group_frame(&mut bob_ws, GroupEvent::Deleted, &group).await;
    group_frame(&mut cat_ws, GroupEvent::Deleted, &group).await;
    let (status, _) = http(port, "DELETE", &path, Some(&ann.session), "");
    assert_eq!(status, 404);
    assert_eq!(rename(port, &group, &ann, "Ghost").0, 404);
    assert_eq!(join(port, &group, &cat).0, 404, "the invite went with it");
}

#[tokio::test]
async fn a_group_holds_fifty_members_and_invitees_and_an_account_is_in_twenty_groups() {
    let gw = spawn_gateway("social_groups_caps");
    let port = gw.port;
    let ann = account(port, "ann");
    let zed = account(port, "zed");
    let zoe = account(port, "zoe");
    befriend(port, &ann, &zed);
    befriend(port, &ann, &zoe);
    // Forty-eight members who are in already: accounts put into the tables,
    // as registering and befriending forty-eight would run into the sign-up
    // allowance and prove the same count.
    gw.sql(
        "INSERT INTO account (id, display_name, username, username_key, lang, created_at, password_hash) \
         SELECT uuidv7(), 'm' || n, 'm' || n, 'm' || n, 'en', now(), 'not a hash' FROM generate_series(1, 48) n",
    );
    let group = make_group(port, &ann, "Crowd");
    gw.sql(&format!(
        "INSERT INTO group_member (group_id, account_id) \
         SELECT '{group}', id FROM account WHERE username ~ '^m[0-9]+$'"
    ));
    // Forty-nine members, so one more invite fits and the next does not.
    let (status, body) = invite(port, &group, &ann, &zed);
    assert_eq!(status, 204, "the fiftieth place: {body}");
    let (status, body) = invite(port, &group, &ann, &zoe);
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("the group is full"), "{body}");
    // An invite taken back frees its place.
    let (status, _) = http(
        port,
        "DELETE",
        &format!("/social/groups/{group}/invites/{}", zed.tag),
        Some(&ann.session),
        "",
    );
    assert_eq!(status, 204);
    assert_eq!(invite(port, &group, &ann, &zoe).0, 204);

    // Twenty groups is the most an account is in.
    let id = gw.text("SELECT id::text FROM account WHERE username = 'zoe'");
    gw.sql(&format!(
        "WITH g AS (INSERT INTO chat_group (name) \
                    SELECT 'g' || n FROM generate_series(1, 20) n RETURNING id) \
         INSERT INTO group_member (group_id, account_id, owner) \
         SELECT id, '{id}', true FROM g"
    ));
    let body = serde_json::json!({ "name": "One too many" }).to_string();
    let (status, answer) = http(port, "POST", "/social/groups", Some(&zoe.session), &body);
    assert_eq!(status, 409, "{answer}");
    assert!(answer.contains("in as many groups"), "{answer}");
    // And a pending invite to a twenty-first cannot be taken.
    let (status, answer) = join(port, &group, &zoe);
    assert_eq!(status, 409, "{answer}");
}
