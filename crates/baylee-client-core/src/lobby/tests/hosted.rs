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
