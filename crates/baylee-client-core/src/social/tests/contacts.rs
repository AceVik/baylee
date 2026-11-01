//! The social graph as the gateway last said it: snapshot, events, expiring invites and the drawer's order (§10.1, §11.3).

use baylee_protocol::social::{
    Friend, FriendEvent, GroupEvent, GroupInfo, GroupInvite, InviteKind, Member, Person, Place,
    Presence, Shown, Snapshot, SocialSettings, TableBrief, TableInvite, WhispersFrom,
};

use crate::social::contacts::Contacts;

fn person(tag: &str, handle: &str) -> Person {
    Person {
        tag: tag.to_string(),
        handle: handle.to_string(),
    }
}

fn presence(status: Shown, place: Option<Place>) -> Presence {
    Presence { status, place }
}

fn friend(tag: &str, handle: &str, status: Shown, place: Option<Place>) -> Friend {
    Friend {
        person: person(tag, handle),
        since: 100,
        presence: presence(status, place),
    }
}

fn group(id: &str, name: &str) -> GroupInfo {
    GroupInfo {
        id: id.to_string(),
        name: name.to_string(),
        members: vec![Member {
            person: person("0001", "Me#0001"),
            owner: true,
        }],
        invited: Vec::new(),
    }
}

fn group_invite(id: &str) -> GroupInvite {
    GroupInvite {
        group: id.to_string(),
        name: format!("Group {id}"),
        inviter: person("0002", "Ann#0002"),
        at: 5,
    }
}

fn table_invite(id: &str, expires_in: u32) -> TableInvite {
    TableInvite {
        id: id.to_string(),
        from: person("0002", "Ann#0002"),
        game: format!("game-{id}"),
        kind: InviteKind::Join,
        table: TableBrief {
            name: "Table".into(),
            seats: 4,
            taken: 1,
            state: "waiting".into(),
            locked: false,
        },
        expires_in,
    }
}

fn playing() -> Place {
    Place::Playing {
        game: None,
        name: "T".into(),
        seats: 4,
        running_secs: 60,
    }
}

fn room() -> Place {
    Place::Room {
        game: None,
        name: "R".into(),
        seats: 4,
        taken: 1,
        locked: false,
    }
}

fn with_friends(friends: Vec<Friend>) -> Contacts {
    Contacts {
        friends,
        ..Contacts::default()
    }
}

fn handles(list: &[&Friend]) -> Vec<String> {
    list.iter().map(|f| f.person.handle.clone()).collect()
}

#[test]
fn a_snapshot_makes_the_whole_graph_and_starts_the_invite_countdowns() {
    let settings = SocialSettings {
        whispers_from: WhispersFrom::Friends,
        requests_open: false,
        share_table: false,
    };
    let snapshot = Snapshot {
        friends: vec![friend("0010", "Fay#0010", Shown::Online, None)],
        incoming: vec![person("0020", "Gus#0020")],
        outgoing: vec![person("0030", "Hal#0030")],
        blocked: vec![person("0040", "Ivy#0040")],
        groups: vec![group("g1", "One"), group("g2", "Two")],
        group_invites: vec![group_invite("g3")],
        tables: Vec::new(),
        invites: vec![table_invite("i1", 600)],
        settings,
    };
    let contacts = Contacts::from_snapshot(&snapshot, 1_000_000);
    assert_eq!(contacts.friends, snapshot.friends);
    assert_eq!(contacts.incoming, snapshot.incoming);
    assert_eq!(contacts.outgoing, snapshot.outgoing);
    assert_eq!(contacts.blocked, snapshot.blocked);
    assert_eq!(contacts.groups, snapshot.groups);
    assert_eq!(contacts.group_invites, snapshot.group_invites);
    assert_eq!(contacts.settings, settings);
    assert_eq!(contacts.invites.len(), 1);
    assert_eq!(contacts.invites[0].invite, snapshot.invites[0]);
    assert_eq!(contacts.invites[0].expires_at, 1_000_000 + 600_000);
    assert!(contacts.is_friend("0010"));
    assert!(!contacts.is_friend("0020"));
    assert!(contacts.is_blocked("0040"));
    assert!(!contacts.is_blocked("0010"));
    assert_eq!(contacts.group("g2").map(|g| g.name.as_str()), Some("Two"));
    assert!(contacts.group("nope").is_none());
    assert_eq!(
        contacts.friend("0010").map(|f| f.person.handle.as_str()),
        Some("Fay#0010")
    );
}

#[test]
fn a_request_in_waits_once_even_if_it_arrives_twice() {
    let mut c = Contacts::default();
    let ann = person("0002", "Ann#0002");
    c.friend_event(FriendEvent::RequestIn, &ann, None, 1);
    c.friend_event(FriendEvent::RequestIn, &ann, None, 2);
    assert_eq!(c.incoming, vec![ann]);
    assert!(c.outgoing.is_empty());
}

#[test]
fn a_request_out_is_listed_as_sent_once() {
    let mut c = Contacts::default();
    let ann = person("0002", "Ann#0002");
    c.friend_event(FriendEvent::RequestOut, &ann, None, 1);
    c.friend_event(FriendEvent::RequestOut, &ann, None, 2);
    assert_eq!(c.outgoing, vec![ann]);
    assert!(c.incoming.is_empty());
}

#[test]
fn accepting_a_request_makes_a_friend_and_clears_both_request_lists() {
    let mut c = Contacts::default();
    let ann = person("0002", "Ann#0002");
    c.friend_event(FriendEvent::RequestIn, &ann, None, 1);
    c.friend_event(FriendEvent::RequestOut, &ann, None, 1);
    let here = presence(Shown::Online, Some(Place::Lobby));
    c.friend_event(FriendEvent::Accepted, &ann, Some(here.clone()), 77);
    assert!(c.incoming.is_empty() && c.outgoing.is_empty());
    assert_eq!(c.friends.len(), 1);
    assert_eq!(c.friends[0].person, ann);
    assert_eq!(c.friends[0].since, 77);
    assert_eq!(c.friends[0].presence, here);
}

#[test]
fn a_friend_accepted_without_presence_is_offline() {
    let mut c = Contacts::default();
    let ann = person("0002", "Ann#0002");
    c.friend_event(FriendEvent::Accepted, &ann, None, 5);
    assert_eq!(c.friends[0].presence, Presence::offline());
}

#[test]
fn accepting_twice_keeps_one_friend_with_the_newer_row() {
    let mut c = Contacts::default();
    let ann = person("0002", "Ann#0002");
    c.friend_event(FriendEvent::Accepted, &ann, None, 5);
    c.friend_event(
        FriendEvent::Accepted,
        &ann,
        Some(presence(Shown::Away, None)),
        9,
    );
    assert_eq!(c.friends.len(), 1);
    assert_eq!(c.friends[0].since, 9);
    assert_eq!(c.friends[0].presence.status, Shown::Away);
}

#[test]
fn removing_a_friend_only_removes_that_friend() {
    let mut c = Contacts::default();
    let ann = person("0002", "Ann#0002");
    let bob = person("0003", "Bob#0003");
    c.friend_event(FriendEvent::Accepted, &ann, None, 1);
    c.friend_event(FriendEvent::Accepted, &bob, None, 1);
    c.friend_event(FriendEvent::Removed, &ann, None, 2);
    assert!(!c.is_friend("0002"));
    assert!(c.is_friend("0003"));
}

#[test]
fn a_request_gone_clears_it_from_both_lists() {
    let mut c = Contacts::default();
    let ann = person("0002", "Ann#0002");
    let bob = person("0003", "Bob#0003");
    c.friend_event(FriendEvent::RequestIn, &ann, None, 1);
    c.friend_event(FriendEvent::RequestOut, &bob, None, 1);
    c.friend_event(FriendEvent::RequestGone, &ann, None, 2);
    c.friend_event(FriendEvent::RequestGone, &bob, None, 2);
    assert!(c.incoming.is_empty());
    assert!(c.outgoing.is_empty());
}

#[test]
fn blocking_removes_the_friend_and_the_requests_and_lists_the_block_once() {
    let mut c = Contacts::default();
    let ann = person("0002", "Ann#0002");
    c.friend_event(FriendEvent::Accepted, &ann, None, 1);
    c.friend_event(FriendEvent::RequestIn, &ann, None, 1);
    c.friend_event(FriendEvent::RequestOut, &ann, None, 1);
    c.friend_event(FriendEvent::Blocked, &ann, None, 2);
    c.friend_event(FriendEvent::Blocked, &ann, None, 3);
    assert!(c.friends.is_empty());
    assert!(c.incoming.is_empty() && c.outgoing.is_empty());
    assert_eq!(c.blocked, vec![ann]);
    assert!(c.is_blocked("0002"));
}

#[test]
fn unblocking_removes_only_the_block() {
    let mut c = Contacts::default();
    let ann = person("0002", "Ann#0002");
    let bob = person("0003", "Bob#0003");
    c.friend_event(FriendEvent::Blocked, &ann, None, 1);
    c.friend_event(FriendEvent::Blocked, &bob, None, 1);
    c.friend_event(FriendEvent::Unblocked, &ann, None, 2);
    assert!(!c.is_blocked("0002"));
    assert!(c.is_blocked("0003"));
    assert!(c.friends.is_empty(), "unblocking does not befriend");
}

#[test]
fn set_presence_changes_the_named_friend_only() {
    let mut c = with_friends(vec![
        friend("0010", "Fay#0010", Shown::Offline, None),
        friend("0011", "Gus#0011", Shown::Offline, None),
    ]);
    c.set_presence("0010", presence(Shown::Dnd, Some(Place::Lobby)));
    c.set_presence("9999", presence(Shown::Online, None));
    assert_eq!(c.friend("0010").unwrap().presence.status, Shown::Dnd);
    assert_eq!(c.friend("0011").unwrap().presence.status, Shown::Offline);
    assert_eq!(c.friends.len(), 2);
}

#[test]
fn joining_a_group_adds_it_and_answers_its_invite() {
    let mut c = Contacts {
        group_invites: vec![group_invite("g1"), group_invite("g2")],
        ..Contacts::default()
    };
    c.group_event(GroupEvent::Joined, &group("g1", "One"), None, 0);
    assert_eq!(c.groups.len(), 1);
    assert_eq!(c.groups[0].id, "g1");
    assert_eq!(
        c.group_invites
            .iter()
            .map(|i| i.group.as_str())
            .collect::<Vec<_>>(),
        ["g2"]
    );
}

#[test]
fn an_updated_group_replaces_its_row_in_place() {
    let mut c = Contacts::default();
    c.group_event(GroupEvent::Joined, &group("g1", "One"), None, 0);
    c.group_event(GroupEvent::Joined, &group("g2", "Two"), None, 0);
    c.group_event(GroupEvent::Updated, &group("g1", "Uno"), None, 0);
    let names: Vec<&str> = c.groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["Uno", "Two"], "order is the drawer's, kept");
}

#[test]
fn an_update_for_an_unknown_group_adds_it() {
    let mut c = Contacts::default();
    c.group_event(GroupEvent::Updated, &group("g1", "One"), None, 0);
    assert_eq!(c.groups.len(), 1);
}

#[test]
fn leaving_or_deleting_a_group_removes_it_and_its_invite() {
    for event in [GroupEvent::Left, GroupEvent::Deleted] {
        let mut c = Contacts::default();
        c.group_event(GroupEvent::Joined, &group("g1", "One"), None, 0);
        c.group_event(GroupEvent::Joined, &group("g2", "Two"), None, 0);
        c.group_invites = vec![group_invite("g1"), group_invite("g3")];
        c.group_event(event, &group("g1", "One"), None, 0);
        assert_eq!(c.groups.len(), 1, "{event:?}");
        assert_eq!(c.groups[0].id, "g2", "{event:?}");
        assert_eq!(c.group_invites.len(), 1, "{event:?}");
        assert_eq!(c.group_invites[0].group, "g3", "{event:?}");
    }
}

#[test]
fn an_invite_gone_removes_the_invite_and_keeps_the_group() {
    let mut c = Contacts::default();
    c.group_event(GroupEvent::Joined, &group("g1", "One"), None, 0);
    c.group_invites = vec![group_invite("g1"), group_invite("g2")];
    c.group_event(GroupEvent::InviteGone, &group("g1", "One"), None, 0);
    assert_eq!(c.groups.len(), 1);
    assert_eq!(c.group_invites.len(), 1);
    assert_eq!(c.group_invites[0].group, "g2");
}

#[test]
fn an_invited_event_changes_nothing_until_the_next_snapshot() {
    let mut c = Contacts {
        group_invites: vec![group_invite("g2")],
        ..Contacts::default()
    };
    c.group_event(GroupEvent::Invited, &group("g1", "One"), None, 0);
    assert!(c.groups.is_empty());
    assert_eq!(c.group_invites.len(), 1);
}

#[test]
fn table_invites_lapse_at_their_time() {
    let snapshot = Snapshot {
        invites: vec![table_invite("short", 10), table_invite("long", 600)],
        ..Snapshot::default()
    };
    let mut c = Contacts::from_snapshot(&snapshot, 0);
    assert!(c.expire_invites(9_999).is_empty(), "nothing lapsed yet");
    assert_eq!(c.invites.len(), 2);
    assert!(
        !c.expire_invites(10_000).is_empty(),
        "exactly at its time it is gone"
    );
    assert_eq!(c.invites.len(), 1);
    assert_eq!(c.invites[0].invite.id, "long");
    assert!(c.expire_invites(10_001).is_empty());
    assert!(!c.expire_invites(600_000).is_empty());
    assert!(c.invites.is_empty());
}

#[test]
fn something_waits_for_an_answer_while_a_request_or_an_invite_is_open() {
    let mut c = Contacts::default();
    assert!(!c.waiting());

    c.incoming.push(person("0002", "Ann#0002"));
    assert!(c.waiting(), "a request to me");
    c.incoming.clear();

    c.group_invites.push(group_invite("g1"));
    assert!(c.waiting(), "a group invite");
    c.group_invites.clear();

    c.friend_event(
        FriendEvent::RequestOut,
        &person("0003", "Bob#0003"),
        None,
        1,
    );
    c.friend_event(FriendEvent::Blocked, &person("0004", "Cy#0004"), None, 1);
    c.friends
        .push(friend("0005", "Di#0005", Shown::Online, None));
    assert!(
        !c.waiting(),
        "my own request, a block and a friend wait for nobody"
    );

    let snapshot = Snapshot {
        invites: vec![table_invite("i1", 60)],
        ..Snapshot::default()
    };
    let mut c = Contacts::from_snapshot(&snapshot, 0);
    assert!(c.waiting(), "a table invite");
    c.expire_invites(60_000);
    assert!(!c.waiting(), "and not once it lapsed");
}

#[test]
fn online_count_counts_every_status_but_offline() {
    let c = with_friends(vec![
        friend("0010", "A#0010", Shown::Online, None),
        friend("0011", "B#0011", Shown::Away, None),
        friend("0012", "C#0012", Shown::Dnd, None),
        friend("0013", "D#0013", Shown::Offline, None),
        friend("0014", "E#0014", Shown::Offline, None),
    ]);
    assert_eq!(c.online_count(), 3);
    assert_eq!(Contacts::default().online_count(), 0);
}

#[test]
fn the_online_section_is_ordered_by_status_then_place_then_name() {
    let c = with_friends(vec![
        friend("0010", "Zed#0010", Shown::Online, Some(Place::Lobby)),
        friend("0011", "Amy#0011", Shown::Dnd, Some(playing())),
        friend("0012", "Bea#0012", Shown::Away, Some(playing())),
        friend("0013", "Cal#0013", Shown::Online, Some(room())),
        friend("0014", "Dan#0014", Shown::Online, Some(playing())),
        friend("0015", "Eve#0015", Shown::Online, Some(Place::Building)),
        friend("0016", "Fox#0016", Shown::Online, None),
        friend("0017", "Gil#0017", Shown::Online, Some(playing())),
        friend("0018", "Hub#0018", Shown::Away, Some(Place::Lobby)),
        friend("0019", "Ian#0019", Shown::Offline, None),
    ]);
    let view = c.view();
    assert_eq!(
        handles(&view.online),
        [
            // online: playing (by name), room, lobby, elsewhere (by name)
            "Dan#0014", "Gil#0017", "Cal#0013", "Zed#0010", "Eve#0015", "Fox#0016",
            // away: playing, lobby
            "Bea#0012", "Hub#0018", // dnd
            "Amy#0011",
        ]
    );
    assert_eq!(handles(&view.offline), ["Ian#0019"]);
}

#[test]
fn places_outside_the_three_known_ones_rank_together_by_name() {
    let c = with_friends(vec![
        friend("0010", "Cy#0010", Shown::Online, Some(Place::House)),
        friend(
            "0011",
            "Al#0011",
            Shown::Online,
            Some(Place::Watching {
                game: None,
                name: "T".into(),
            }),
        ),
        friend("0012", "Bo#0012", Shown::Online, None),
    ]);
    assert_eq!(handles(&c.view().online), ["Al#0011", "Bo#0012", "Cy#0010"]);
}

#[test]
fn names_sort_without_regard_to_case_and_ties_go_by_tag() {
    let c = with_friends(vec![
        friend("0012", "bob#0012", Shown::Online, Some(Place::Lobby)),
        friend("0011", "Alice#0011", Shown::Online, Some(Place::Lobby)),
        friend("0014", "Bob#0014", Shown::Online, Some(Place::Lobby)),
        friend("0013", "alex#0013", Shown::Offline, None),
        friend("0010", "Zoe#0010", Shown::Offline, None),
        friend("0015", "Beth#0015", Shown::Offline, None),
    ]);
    let view = c.view();
    assert_eq!(
        handles(&view.online),
        ["Alice#0011", "bob#0012", "Bob#0014"]
    );
    assert_eq!(
        handles(&view.offline),
        ["alex#0013", "Beth#0015", "Zoe#0010"]
    );
}

#[test]
fn the_view_hands_through_the_other_sections() {
    let snapshot = Snapshot {
        incoming: vec![person("0020", "Gus#0020")],
        outgoing: vec![person("0030", "Hal#0030")],
        blocked: vec![person("0040", "Ivy#0040")],
        groups: vec![group("g1", "One"), group("g2", "Two")],
        group_invites: vec![group_invite("g3")],
        invites: vec![table_invite("i1", 600)],
        ..Snapshot::default()
    };
    let c = Contacts::from_snapshot(&snapshot, 0);
    let view = c.view();
    assert_eq!(view.incoming, snapshot.incoming);
    assert_eq!(view.outgoing, snapshot.outgoing);
    assert_eq!(view.blocked, snapshot.blocked);
    assert_eq!(view.groups, snapshot.groups);
    assert_eq!(view.group_invites, snapshot.group_invites);
    assert_eq!(view.invites.len(), 1);
    assert!(view.online.is_empty() && view.offline.is_empty());
}
