//! A hosted model in a room's chair: what the sheet offers (available
//! first, the rest greyed with when they are back), who may order one
//! (a registered account, never a guest), and what a chair says of it.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use crate::lobby::hosted::{HostedChair, HostedProfile, clock};

fn profile(id: &str, label: &str, available: bool, until: Option<i64>) -> HostedProfile {
    HostedProfile {
        id: id.into(),
        label: label.into(),
        vendor: "Anthropic".into(),
        kind: "api".into(),
        model: "m".into(),
        state: if available { "available" } else { "exhausted" }.into(),
        until_unix: until,
        available,
    }
}

/// A signed-in lobby that has the gateway's list.
fn listed() -> Lobby {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.list_hosted(), Some(LobbyRequest::HostedProfiles));
    assert!(!lobby.hosted_listed());
    lobby.apply(LobbyEvent::HostedProfiles(vec![
        profile("zeta", "Zeta", false, Some(3_600 * 14 + 60 * 5)),
        profile("sonnet", "Sonnet", true, None),
        profile("off", "Asleep", false, None),
        profile("alpha", "Alpha", true, None),
    ]));
    lobby
}

#[test]
fn available_profiles_come_first_and_the_rest_say_when_they_are_back() {
    let lobby = listed();
    assert!(lobby.hosted_listed());
    let rows = lobby.hosted_offer(3_600);
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["alpha", "sonnet", "off", "zeta"]);
    assert_eq!(rows[0].words, "Alpha · Anthropic");
    assert!(rows[0].available && rows[0].why.is_none());
    assert!(!rows[2].available);
    assert_eq!(rows[2].why.as_deref(), Some("not available now"));
    // 14:05 UTC, one hour east.
    assert_eq!(rows[3].why.as_deref(), Some("back from 15:05"));
}

#[test]
fn the_return_time_is_the_local_clock() {
    assert_eq!(clock(0, 0), "00:00");
    assert_eq!(clock(3_600 * 23 + 60 * 59, 7_200), "01:59");
    assert_eq!(clock(60, -3_600), "23:01");
}

#[test]
fn an_order_names_its_chair_and_only_an_available_profile() {
    let mut lobby = listed();
    assert_eq!(lobby.order_hosted("g", 2, "zeta"), None, "unavailable");
    assert_eq!(lobby.order_hosted("g", 2, "nobody"), None, "never listed");
    assert_eq!(
        lobby.order_hosted("g", 2, "sonnet"),
        Some(LobbyRequest::OrderHosted {
            game_id: "g".into(),
            seat: 2,
            profile: "sonnet".into(),
        })
    );
    // One request at a time.
    assert_eq!(lobby.cancel_hosted("g", 2), None);
    lobby.apply(LobbyEvent::Moved);
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(
        lobby.cancel_hosted("g", 2),
        Some(LobbyRequest::CancelHosted {
            game_id: "g".into(),
            seat: 2,
        })
    );
}

#[test]
fn a_guest_is_offered_no_hosted_model() {
    let mut guest = Lobby::new();
    guest.set_guests_enabled(true);
    guest.apply(LobbyEvent::GuestIn(KeptGuest {
        token: "guest-tok".into(),
        handle: "Casper#0007".into(),
    }));
    assert!(!guest.may_order_hosted());
    assert_eq!(guest.list_hosted(), None);
    assert_eq!(guest.order_hosted("g", 1, "sonnet"), None);
    assert!(seated_lobby().may_order_hosted());
    assert!(!offline_lobby().may_order_hosted());
}

#[test]
fn a_chair_says_its_state_and_where_the_data_goes() {
    let json = r#"{"seat": 1, "taken": true, "hosted": {"profile": "sonnet",
        "label": "Sonnet", "vendor": "Anthropic", "model": "m",
        "state": "starting", "note": null}}"#;
    let seat: GameSeat = serde_json::from_str(json).expect("a listed chair");
    let hosted = seat.hosted.expect("hosted");
    assert!(hosted.standing());
    assert_eq!(hosted.said(Lang::En), "Starting…");
    assert_eq!(hosted.said(Lang::De), "Startet…");
    assert_eq!(hosted.data_goes(Lang::De), "Spieldaten gehen an Anthropic");
    let failed = HostedChair {
        state: "failed".into(),
        note: Some("the seat agent went away".into()),
        ..hosted.clone()
    };
    assert!(!failed.standing());
    assert_eq!(
        failed.said(Lang::En),
        "The hosted model failed: the seat agent went away"
    );
    let ready = HostedChair {
        state: "ready".into(),
        ..hosted
    };
    assert_eq!(ready.said(Lang::En), "Ready");
    // A chair from a gateway that knows no hosted seats has none.
    let plain: GameSeat = serde_json::from_str(r#"{"seat": 0, "taken": false}"#).expect("chair");
    assert_eq!(plain.hosted, None);
}

/// A registered lobby the gateway answered `profiles` to.
fn answered(profiles: Vec<HostedProfile>) -> Lobby {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.list_hosted(), Some(LobbyRequest::HostedProfiles));
    lobby.apply(LobbyEvent::HostedProfiles(profiles));
    lobby
}

#[test]
fn the_hosted_entry_waits_for_a_list_that_holds_a_profile() {
    // No list has come: the gateway has not said what this account may seat.
    let lobby = seated_lobby();
    assert!(lobby.may_order_hosted());
    assert!(!lobby.hosted_listed());
    assert!(!lobby.offers_hosted(), "nothing was asked yet");
    // Asked, not yet answered.
    let mut asking = seated_lobby();
    assert_eq!(asking.list_hosted(), Some(LobbyRequest::HostedProfiles));
    assert!(!asking.offers_hosted(), "asked, not answered");
    // Answered with an empty list: no model is open to this account.
    let empty = answered(Vec::new());
    assert!(empty.hosted_listed());
    assert!(empty.hosted_profiles().is_empty());
    assert!(!empty.offers_hosted(), "an empty list offers nothing");
    // One profile, even an unavailable one: the player is shown it exists.
    let one = answered(vec![profile("sonnet", "Sonnet", true, None)]);
    assert!(one.offers_hosted());
    let greyed = answered(vec![profile("zeta", "Zeta", false, None)]);
    assert!(greyed.offers_hosted(), "a listed model is offered, greyed");
}

#[test]
fn a_new_ask_hides_the_entry_until_the_gateway_answers_again() {
    let mut lobby = answered(vec![profile("sonnet", "Sonnet", true, None)]);
    assert!(lobby.offers_hosted());
    // The list is asked afresh (a model may have been taken away): the old
    // answer does not stand in for the new one.
    assert_eq!(lobby.list_hosted(), Some(LobbyRequest::HostedProfiles));
    assert!(!lobby.offers_hosted(), "waiting for the new list");
    lobby.apply(LobbyEvent::HostedProfiles(Vec::new()));
    assert!(!lobby.offers_hosted(), "the new list holds none");
}

#[test]
fn a_guest_never_sees_the_entry_even_with_profiles_in_hand() {
    let mut guest = Lobby::new();
    guest.set_guests_enabled(true);
    guest.apply(LobbyEvent::GuestIn(KeptGuest {
        token: "guest-tok".into(),
        handle: "Casper#0007".into(),
    }));
    assert!(!guest.offers_hosted());
    // Whatever list reaches a guest, the entry stays hidden.
    guest.apply(LobbyEvent::HostedProfiles(vec![profile(
        "sonnet", "Sonnet", true, None,
    )]));
    assert!(guest.hosted_listed());
    assert!(!guest.offers_hosted());
    assert!(!offline_lobby().offers_hosted());
}

#[test]
fn signing_out_forgets_which_models_were_open() {
    let mut lobby = answered(vec![profile("sonnet", "Sonnet", true, None)]);
    assert!(lobby.offers_hosted());
    assert!(lobby.sign_out().is_some(), "the gateway ends the session");
    assert!(lobby.hosted_profiles().is_empty());
    assert!(!lobby.hosted_listed());
    assert!(!lobby.offers_hosted());
    // The next account signs in: it is shown nothing of the last one's
    // list until its own has come.
    lobby.set_field(Field::Username, "bob");
    lobby.set_field(Field::Password, "hunter22");
    assert!(lobby.submit().is_some());
    lobby.apply(LobbyEvent::LoggedIn {
        token: "tok-bob".to_string(),
        username: None,
    });
    assert!(lobby.may_order_hosted());
    assert!(lobby.hosted_profiles().is_empty());
    assert!(
        !lobby.offers_hosted(),
        "the previous account's list is gone"
    );
}

/// One card, so that a room's listing does not ask for the pool first.
fn pooled(lobby: &mut Lobby) {
    lobby.builder_mut().set_pool(
        vec![crate::deckbuilder::PoolCard {
            name: "Forest".into(),
            english_name: "Forest".into(),
            kinds: vec!["Land".into()],
            ..crate::deckbuilder::PoolCard::default()
        }],
        false,
    );
}

/// The listing of a waiting room `id` with one open chair, the host's own
/// where `yours`.
fn room_listing(id: &str, yours: bool) -> GameListing {
    GameListing::of(vec![GameSummary {
        id: id.into(),
        name: "Garden".into(),
        yours,
        state: "waiting".into(),
        seats: vec![
            GameSeat {
                you: true,
                taken: true,
                ..GameSeat::default()
            },
            GameSeat::default(),
        ],
        ..GameSummary::default()
    }])
}

/// `lobby` opens a room `id` of its own and is seated at it: what the
/// listing that shows the room chains into.
fn opens_room(lobby: &mut Lobby, id: &str) -> Option<LobbyRequest> {
    assert!(lobby.host(GameMode::Open).is_some(), "the table is opened");
    assert!(
        matches!(
            lobby.apply(LobbyEvent::Seated(SeatHandover {
                game_id: id.into(),
                seat: 0,
                seat_token: "ticket".into(),
                local: false,
            })),
            Some(LobbyRequest::ListGames(_))
        ),
        "the room is listed"
    );
    lobby.apply(LobbyEvent::Games(room_listing(id, true)))
}

/// D5 (beta.7 acceptance): the chair's "Gateway model" entry showed only
/// after the local "Language model" sheet was opened once, the one press
/// that asked the list, and a desktop without its bridge, or a build with
/// no local control at all, never got that far. The list is now asked as
/// a registered host's room opens, with no press: the entry is there from
/// the start, quietly (no loading veil), and asked once per room.
#[test]
fn a_hosts_room_asks_the_hosted_list_as_it_opens() {
    let mut lobby = seated_lobby();
    pooled(&mut lobby);
    assert!(!lobby.offers_hosted());
    assert_eq!(
        opens_room(&mut lobby, "room"),
        Some(LobbyRequest::HostedProfiles),
        "asked as the room opens, before any chair is touched"
    );
    assert!(lobby.busy() && lobby.refreshing(), "a quiet read");
    assert!(!lobby.hosted_listed());
    assert_eq!(
        lobby.apply(LobbyEvent::HostedProfiles(vec![profile(
            "sonnet", "Sonnet", true, None
        )])),
        None
    );
    assert!(lobby.offers_hosted(), "the entry is there from the start");
    assert!(!lobby.busy());
    // The room is listed again (the feed, a refresh): asked once per room.
    assert_eq!(
        lobby.apply(LobbyEvent::Games(room_listing("room", true))),
        None
    );
    assert_eq!(
        lobby.apply(LobbyEvent::Games(room_listing("room", true))),
        None
    );
    assert!(lobby.offers_hosted());
    // The entry's own sheet still asks afresh.
    assert_eq!(lobby.list_hosted(), Some(LobbyRequest::HostedProfiles));
}

/// A room left forgets that it was asked for; the next room asks again.
#[test]
fn the_next_room_asks_again() {
    let mut lobby = seated_lobby();
    pooled(&mut lobby);
    assert_eq!(
        opens_room(&mut lobby, "first"),
        Some(LobbyRequest::HostedProfiles)
    );
    lobby.apply(LobbyEvent::HostedProfiles(Vec::new()));
    assert!(!lobby.offers_hosted(), "no model is open to this account");
    assert!(matches!(
        lobby.apply(LobbyEvent::Left),
        Some(LobbyRequest::ListGames(_))
    ));
    assert_eq!(lobby.apply(LobbyEvent::Games(GameListing::default())), None);
    assert_eq!(
        opens_room(&mut lobby, "second"),
        Some(LobbyRequest::HostedProfiles)
    );
    lobby.apply(LobbyEvent::HostedProfiles(vec![profile(
        "sonnet", "Sonnet", false, None,
    )]));
    assert!(lobby.offers_hosted(), "a listed model is offered, greyed");
}

/// The pool a seat needs goes first; the list follows with the next
/// listing, still unasked until then.
#[test]
fn the_pool_goes_before_the_hosted_list() {
    let mut lobby = seated_lobby();
    assert_eq!(opens_room(&mut lobby, "room"), Some(LobbyRequest::LoadPool));
    pooled(&mut lobby);
    assert_eq!(
        lobby.apply(LobbyEvent::Games(room_listing("room", true))),
        Some(LobbyRequest::HostedProfiles)
    );
}

/// Only the host orders a model for a chair: a seat at somebody else's
/// room asks nothing, and neither does a guest's own room.
#[test]
fn only_a_registered_hosts_room_asks_the_hosted_list() {
    let mut player = seated_lobby();
    pooled(&mut player);
    assert!(matches!(
        player.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "theirs".into(),
            seat: 1,
            seat_token: "ticket".into(),
            local: false,
        })),
        Some(LobbyRequest::ListGames(_))
    ));
    assert_eq!(
        player.apply(LobbyEvent::Games(room_listing("theirs", false))),
        None
    );
    assert!(!player.busy());

    let mut guest = Lobby::new();
    guest.set_guests_enabled(true);
    guest.apply(LobbyEvent::GuestIn(KeptGuest {
        token: "guest-tok".into(),
        handle: "Casper#0007".into(),
    }));
    guest.apply(LobbyEvent::Decks(vec![DeckSummary {
        id: "d1".into(),
        name: "Allytifact".into(),
        cards: 60,
        ..DeckSummary::default()
    }]));
    guest.apply(LobbyEvent::Games(GameListing::default()));
    pooled(&mut guest);
    assert_eq!(opens_room(&mut guest, "room"), None);
    assert!(!guest.offers_hosted());
}
