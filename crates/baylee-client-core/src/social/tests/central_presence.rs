//! A friend as Central shows one, beside what this gateway knows of them:
//! nothing Central did not show may be read out of the two together.

use baylee_protocol::social::{FriendEvent, GroupEvent, Person, Place, Presence, Shown};
use central_api::presence::{self, Activity, Table};

use crate::social::contacts::{Contacts, place_name, shown_by_central};

const HERE: &str = "https://play.example";

fn central(activity: Option<Activity>) -> presence::Shown {
    presence::Shown {
        away: false,
        activity,
    }
}

fn table() -> Table {
    Table {
        origin: HERE.into(),
        game: "game-1".into(),
        name: "Friday".into(),
        seats: 4,
        taken: 3,
    }
}

fn gateway(status: Shown, place: Place) -> Presence {
    Presence {
        status,
        place: Some(place),
    }
}

fn playing_here() -> Place {
    Place::Playing {
        game: Some("seen-here".into()),
        name: "Seen here".into(),
        seats: 2,
        running_secs: 90,
    }
}

fn room_here() -> Place {
    Place::Room {
        game: Some("seen-here".into()),
        name: "Seen here".into(),
        seats: 2,
        taken: 1,
        locked: true,
    }
}

#[test]
fn away_is_shown_as_away_and_anything_else_as_online() {
    for away in [false, true] {
        let shown = presence::Shown {
            away,
            activity: None,
        };
        let expected = if away { Shown::Away } else { Shown::Online };
        assert_eq!(shown_by_central(shown, None, None, None).status, expected);
    }
}

#[test]
fn a_shared_table_here_can_be_reached_and_one_elsewhere_is_only_named() {
    let at = |here: Option<&str>| {
        shown_by_central(central(Some(Activity::Table)), Some(&table()), here, None).place
    };
    let room = |game: Option<&str>| {
        Some(Place::Room {
            game: game.map(str::to_owned),
            name: "Friday".into(),
            seats: 4,
            taken: 3,
            locked: false,
        })
    };
    assert_eq!(at(Some(HERE)), room(Some("game-1")));
    assert_eq!(at(Some("https://other.example")), room(None));
    assert_eq!(at(None), room(None));
    let playing = shown_by_central(
        central(Some(Activity::Playing)),
        Some(&table()),
        Some(HERE),
        None,
    );
    assert_eq!(
        playing.place,
        Some(Place::Playing {
            game: Some("game-1".into()),
            name: "Friday".into(),
            seats: 4,
            running_secs: 0,
        })
    );
}

#[test]
fn a_table_that_cannot_be_real_is_not_shown_and_not_reachable() {
    let mut bad = table();
    bad.taken = 5;
    let shown = shown_by_central(central(Some(Activity::Table)), Some(&bad), Some(HERE), None);
    assert_eq!(
        shown.place,
        Some(Place::Room {
            game: None,
            name: String::new(),
            seats: 0,
            taken: 0,
            locked: false,
        })
    );
}

#[test]
fn what_this_gateway_knows_wins_where_it_agrees_with_central() {
    let place = |activity, known: Place| {
        let seen = gateway(Shown::Online, known);
        shown_by_central(central(activity), Some(&table()), Some(HERE), Some(&seen)).place
    };
    assert_eq!(
        place(Some(Activity::Playing), playing_here()),
        Some(playing_here())
    );
    assert_eq!(place(Some(Activity::Table), room_here()), Some(room_here()));
    assert_eq!(place(None, Place::Building), Some(Place::Building));
    // Where they disagree, Central's activity is the newer word.
    assert!(matches!(
        place(Some(Activity::Table), playing_here()),
        Some(Place::Room { name, .. }) if name == "Friday"
    ));
    assert!(matches!(
        place(Some(Activity::Playing), room_here()),
        Some(Place::Playing { name, .. }) if name == "Friday"
    ));
}

#[test]
fn a_friend_this_gateway_calls_offline_is_taken_from_central_alone() {
    let seen = gateway(Shown::Offline, playing_here());
    let shown = shown_by_central(central(None), None, Some(HERE), Some(&seen));
    assert_eq!(shown.place, Some(Place::Lobby));
    assert_eq!(shown.status, Shown::Online);
}

#[test]
fn only_a_table_with_a_name_is_named() {
    assert_eq!(place_name(&playing_here()), Some("Seen here"));
    assert_eq!(place_name(&room_here()), Some("Seen here"));
    let nameless = Place::Playing {
        game: None,
        name: String::new(),
        seats: 2,
        running_secs: 0,
    };
    assert_eq!(place_name(&nameless), None);
    for place in [Place::Lobby, Place::Building, Place::House] {
        assert_eq!(place_name(&place), None);
    }
}

fn person(tag: &str) -> Person {
    Person {
        tag: tag.into(),
        handle: format!("{tag}#0001"),
        guest: false,
    }
}

fn group(id: &str, name: &str) -> baylee_protocol::social::GroupInfo {
    baylee_protocol::social::GroupInfo {
        id: id.into(),
        name: name.into(),
        members: Vec::new(),
        invited: Vec::new(),
    }
}

#[test]
fn an_invitation_that_names_its_inviter_is_listed_at_once_and_once() {
    let mut contacts = Contacts::default();
    let invited = |contacts: &mut Contacts, name: &str, by: &str, at| {
        let by = person(by);
        contacts.group_event(GroupEvent::Invited, &group("g", name), Some(&by), at);
    };
    contacts.group_event(
        GroupEvent::Invited,
        &group("h", "Other"),
        Some(&person("cy")),
        1,
    );
    invited(&mut contacts, "Old name", "ann", 5);
    invited(&mut contacts, "New name", "bob", 9);
    let seen: Vec<_> = contacts
        .group_invites
        .iter()
        .map(|i| {
            (
                i.group.as_str(),
                i.name.as_str(),
                i.inviter.tag.as_str(),
                i.at,
            )
        })
        .collect();
    assert_eq!(seen, [("h", "Other", "cy", 1), ("g", "New name", "bob", 9)]);
    assert!(contacts.waiting());
}

#[test]
fn an_accepted_friend_keeps_the_presence_that_came_along() {
    let mut contacts = Contacts::default();
    let presence = gateway(Shown::Away, Place::House);
    contacts.friend_event(
        FriendEvent::Accepted,
        &person("ann"),
        Some(presence.clone()),
        7,
    );
    let friend = contacts.friend("ann").unwrap();
    assert_eq!(friend.presence, presence);
    assert_eq!(friend.since, 7);
}
