//! Context-aware feature visibility predicates across Offline, Guest,
//! Local-Account, and Central-Account sessions, plus offline field focus and
//! room permanent card-query search.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn capability_matrix_distinguishes_offline_guest_local_and_central_sessions() {
    // 1. Unauthenticated sign-in face (no Central session, no gateway session).
    let fresh = Lobby::new();
    assert!(!fresh.can_play_online());
    assert!(!fresh.has_registered_gateway_account());
    assert!(!fresh.can_use_contacts(false));
    assert!(!fresh.has_central_account(false));
    assert!(!fresh.can_enter_challenges(false));
    assert!(!fresh.can_enter_tower(false));
    assert!(!fresh.can_use_server_replays());
    assert!(!fresh.can_use_deck_history());
    assert!(!fresh.can_use_banlists());

    // 2. Offline play: no online rooms, no contacts, no server replays/history/banlists.
    let offline = offline_lobby();
    assert!(!offline.can_play_online());
    assert!(!offline.has_registered_gateway_account());
    assert!(!offline.can_use_contacts(false));
    assert!(!offline.has_central_account(false));
    assert!(!offline.can_enter_challenges(false));
    assert!(!offline.can_enter_tower(false));
    assert!(!offline.can_use_server_replays());
    assert!(!offline.can_use_deck_history());
    assert!(!offline.can_use_banlists());

    // 3. Online Guest: can play online and use server replays/deck-history/banlists,
    //    but NEVER has contacts, Central Challenges, or Central Tower Echoes.
    let mut guest = Lobby::new();
    guest.apply(LobbyEvent::GuestIn(KeptGuest {
        token: "guest-tok".to_string(),
        handle: "Casper#0007".to_string(),
    }));
    assert!(guest.can_play_online());
    assert!(!guest.has_registered_gateway_account());
    assert!(!guest.can_use_contacts(false));
    assert!(!guest.can_use_contacts(true));
    assert!(!guest.has_central_account(false));
    assert!(!guest.has_central_account(true));
    assert!(!guest.can_enter_challenges(false));
    assert!(!guest.can_enter_challenges(true));
    assert!(!guest.can_enter_tower(false));
    assert!(!guest.can_enter_tower(true));
    assert!(guest.can_use_server_replays());
    assert!(guest.can_use_deck_history());
    assert!(guest.can_use_banlists());

    // 4. Local registered gateway account (non-Central): has contacts & gateway features,
    //    no Central Challenges or Central Tower Echoes.
    let local = seated_lobby();
    assert!(local.can_play_online());
    assert!(local.has_registered_gateway_account());
    assert!(local.can_use_contacts(false));
    assert!(!local.has_central_account(false));
    assert!(!local.can_enter_challenges(false));
    assert!(!local.can_enter_tower(false));
    assert!(local.can_use_server_replays());
    assert!(local.can_use_deck_history());
    assert!(local.can_use_banlists());

    // 5. Central account before joining a community gateway: has Contacts,
    //    Challenges, and Central-backed Tower Echoes; no community gateway
    //    replays / deck history / banlists yet.
    let central_door = Lobby::new();
    assert!(!central_door.can_play_online());
    assert!(central_door.can_use_contacts(true));
    assert!(central_door.has_central_account(true));
    assert!(central_door.can_enter_challenges(true));
    assert!(central_door.can_enter_tower(true));
    assert!(!central_door.can_use_server_replays());
    assert!(!central_door.can_use_deck_history());
    assert!(!central_door.can_use_banlists());

    // 6. Central account signed into a gateway: full feature access.
    let mut central_online = seated_lobby();
    central_online.set_central_bound(true);
    assert!(central_online.can_play_online());
    assert!(central_online.has_registered_gateway_account());
    assert!(central_online.can_use_contacts(true));
    assert!(central_online.has_central_account(true));
    assert!(central_online.can_enter_challenges(true));
    assert!(central_online.can_enter_tower(true));
    assert!(central_online.can_use_server_replays());
    assert!(central_online.can_use_deck_history());
    assert!(central_online.can_use_banlists());
}

#[test]
fn offline_table_focus_omits_room_password_and_refuses_typing() {
    let mut lobby = offline_lobby();
    lobby.focus_on(Field::Search);
    let mut visited = Vec::new();
    for _ in 0..4 {
        lobby.cycle_focus(Tab::Next);
        visited.push(lobby.focus());
    }
    assert!(
        !visited.contains(&Field::RoomPassword),
        "offline focus cycle must skip RoomPassword: {visited:?}"
    );

    lobby.focus_on(Field::RoomPassword);
    assert!(!lobby.typing_here());
    lobby.type_char('s');
    lobby.type_char('e');
    assert!(
        lobby.room_password().is_empty(),
        "typing into hidden RoomPassword while offline must do nothing"
    );
}

#[test]
fn room_board_search_understands_card_query_syntax() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Pool {
        cards: vec![
            PoolCard {
                index: 1,
                name: "Llanowar Elves".to_string(),
                english_name: "Llanowar Elves".to_string(),
                mana_cost: "{G}".to_string(),
                cmc: 1,
                colors: "G".to_string(),
                identity: "G".to_string(),
                type_line: "Creature — Elf Druid".to_string(),
                kinds: vec!["Creature".to_string()],
                stats: Some("1/1".to_string()),
                oracle_text: "{T}: Add {G}.".to_string(),
                coverage: Coverage::Implemented,
                ..PoolCard::default()
            },
            PoolCard {
                index: 2,
                name: "Forest".to_string(),
                english_name: "Forest".to_string(),
                cmc: 0,
                identity: "G".to_string(),
                type_line: "Basic Land — Forest".to_string(),
                kinds: vec!["Land".to_string()],
                coverage: Coverage::Implemented,
                basic_land: true,
                ..PoolCard::default()
            },
        ],
        has_text: true,
    });

    lobby.set_field(Field::RoomBoard(0), "t:land");
    assert_eq!(lobby.room_matches(0), vec![1]);

    lobby.set_field(Field::RoomBoard(0), "c:g t:creature");
    assert_eq!(lobby.room_matches(0), vec![0]);

    lobby.set_field(Field::RoomBoard(0), "o:add");
    assert_eq!(lobby.room_matches(0), vec![0]);

    lobby.set_field(Field::RoomBoard(0), "mv=0");
    assert_eq!(lobby.room_matches(0), vec![1]);
}
