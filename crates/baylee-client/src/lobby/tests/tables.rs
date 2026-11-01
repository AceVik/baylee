//! The table screen: the listing, the chairs in a room, and taking one. A table that is full offers no join, one this seat is waiting at is announced and never sat at, a rematch room asks to be played rather than readied, and a listing still in flight when the seat was granted must not dial a second time. Offline is the same screen with this process answering the requests, so it is pressed through here as well — from the offline button to a room of four, a chair moved onto a side, ready, start, and a host installed at the end of it — together with the three gateway controls it must not draw, since a search, a refresh and a room password are all questions about other people. Leaving the game again is the end screen's part.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn an_empty_search_has_a_working_way_back_to_all_tables() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".into(),
            username: None,
        });
        state.lobby.apply(LobbyEvent::Decks(Vec::new()));
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
        state.lobby.set_field(Field::Search, "an absent table");
    }
    app.update();
    assert!(
        presses(&mut app).contains(&Press::Play(PlayPress::CreateTable)),
        "room creation works before choosing a deck"
    );
    tap_control(&mut app, "clear the table search", |p| {
        *p == Press::Play(PlayPress::ClearSearch)
    });
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Search),
        ""
    );
    // The fresh response restores the first-deck invitation, not an empty
    // search and not an enabled action that would try to host without a deck.
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .apply(LobbyEvent::Games(GameListing::default()));
    app.update();
    assert!(labels(&mut app).contains(&Phrase::ShellNoOneWaiting.text(Lang::En).to_owned()));
    assert!(labels(&mut app).contains(&Phrase::DiscoveryHelp.text(Lang::En).to_owned()));
    assert!(!presses(&mut app).contains(&Press::Play(PlayPress::ChangeDeck)));
    assert!(presses(&mut app).contains(&Press::Play(PlayPress::CreateTable)));
    assert!(!presses(&mut app).contains(&Press::Play(PlayPress::ClearSearch)));
}

#[test]
fn the_table_screen_builds_once_there_is_a_deck() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None,
        });
        state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 96,
            copies: 100,
            sideboard: 2,
            side_copies: 4,
            commanders: Vec::new(),
            ..Default::default()
        }]));
        state
            .lobby
            .apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
                id: "0123456789abcdef".to_string(),
                state: "waiting".to_string(),
                seats: vec![
                    GameSeat {
                        seat: 0,
                        taken: true,
                        ..GameSeat::default()
                    },
                    GameSeat {
                        seat: 1,
                        taken: false,
                        ..GameSeat::default()
                    },
                ],
                ..GameSummary::default()
            }])));
    }
    app.update();
    assert!(labels(&mut app).contains(&Phrase::DiscoveryGateway.text(Lang::En).to_owned()));
    assert!(!labels(&mut app).iter().any(|text| text == "Allytifact"));
    let found = presses(&mut app);
    for wanted in [
        // Signing out is in the account pill's menu (the shell design, §2.1).
        Press::Header(super::super::header::HeaderPress::Account),
        Press::Play(PlayPress::CreateTable),
        Press::Play(PlayPress::Join(0)),
        // The chairs of a waiting table are drawn for everyone, so a
        // player can take the one they want rather than whichever the
        // gateway would have handed them.
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
}

/// A running table with this player's chair, laid out with the shipped fonts.
fn returning_game(width: f32, height: f32, lang: &str) -> App {
    let (mut app, fonts) = crate::face::tests::layout_app();
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
        .init_asset::<bevy::shader::Shader>()
        .add_plugins(bevy::camera::visibility::VisibilityPlugin)
        .add_plugins(bevy::state::app::StatesPlugin)
        .init_state::<DuelPhase>()
        .add_message::<DuelCommand>()
        .add_message::<KeyboardInput>()
        .add_message::<PointerClick>()
        .add_message::<PointerScroll>()
        .add_message::<PointerDrag>()
        .add_message::<PointerDragEnd>()
        .insert_resource(fonts)
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(LobbyPlugin);
    {
        let mut state = LobbyState::from_settings(crate::settings::ClientSettings {
            lang: lang.to_string(),
            ..crate::settings::ClientSettings::default()
        });
        state.gateway.clear();
        state.gateway_selected = true;
        state.lobby.set_gateway_ready(true);
        state.lobby.set_registration(Registration::Open);
        state.probes.insert(
            baylee_client_core::lobby::gateway_list::PINNED.to_string(),
            Probe::Asking,
        );
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".into(),
            username: None,
        });
        state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".into(),
            name: "Deck".into(),
            ..Default::default()
        }]));
        state
            .lobby
            .apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
                id: "g".into(),
                state: "playing".into(),
                seats: vec![GameSeat {
                    seat: 0,
                    taken: true,
                    you: true,
                    ready: true,
                    ..GameSeat::default()
                }],
                ..GameSummary::default()
            }])));
        *app.world_mut().resource_mut::<LobbyState>() = state;
    }
    let mut window = Window::default();
    window.resolution.set(width, height);
    app.world_mut().spawn(window);
    app.update();
    app.update();
    app
}

/// Report `01a16cc3`: the way back to a running game is a compact action in
/// the row, not a separate 132-pixel block. Its hit area stays at least 44 px.
#[test]
fn return_to_game_is_compact_in_both_languages_and_desktop_sizes() {
    for lang in ["en", "de"] {
        for (width, height) in [(960.0, 584.0), (1728.0, 1052.0)] {
            let mut app = returning_game(width, height, lang);
            let mut query = app.world_mut().query::<(&Press, &Node, &ComputedNode)>();
            let (node, computed) = query
                .iter(app.world())
                .find_map(|(press, node, computed)| {
                    matches!(press, Press::Play(PlayPress::Return)).then_some((node, computed))
                })
                .expect("the running table offers its way back");
            assert_eq!(node.width, Val::Auto, "the wide action width returned");
            let logical = computed.size() * computed.inverse_scale_factor();
            assert!(
                logical.x < 132.0,
                "{lang} at {width}x{height}: {} px wide",
                logical.x
            );
            assert!(
                logical.x >= 44.0 && logical.y >= 44.0,
                "{lang} at {width}x{height}: hit area shrank to {logical}"
            );
        }
    }
}

/// Offline play, pressed all the way through to a table.
///
/// Every decision in this sequence has a test of its own — the lobby's
/// screens, the performer's room, the preset the start button builds — and
/// not one of them says the buttons are wired to any of it. This is the path
/// a player's finger takes: the same `clicks` system, the same mailbox a
/// gateway's answers would arrive through, and at the end a host actually
/// installed for a table of four.
#[test]
fn offline_play_can_be_pressed_all_the_way_to_a_table() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());

    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| {
        *p == Press::Front(FrontPress::PlayOffline)
    });
    assert_eq!(
        *app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Table,
        "offline play opens the table screen, not a duel"
    );

    // The sheet opens a table of four (the Commander template's).
    open_a_table(&mut app);
    {
        let state = app.world().resource::<LobbyState>();
        let room = state.lobby.games().first().expect("the room is listed");
        assert_eq!(room.seats.len(), 4);
        assert!(!state.connected, "nothing has started yet");
    }

    // A chair moved onto a side, to prove the room's own controls reach the
    // performer and not only the two buttons that open and close it.
    press(&mut app, Press::Room(RoomPress::SetTeams));
    tap_control(&mut app, "chair one onto a side", |p| {
        matches!(p, Press::Room(RoomPress::SeatTeam(0, 0, _)))
    });
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.games()[0].seats[0].team,
        Some(1)
    );

    assert!(
        !presses(&mut app)
            .iter()
            .any(|p| matches!(p, Press::Room(RoomPress::Ready(..)))),
        "the host starts directly"
    );
    tap_control(&mut app, "the start button", |p| {
        *p == Press::Room(RoomPress::StartRoom(0))
    });

    let state = app.world().resource::<LobbyState>();
    let Screen::Seated(handover) = state.lobby.screen() else {
        panic!("the start button seats you: {:?}", state.lobby.screen())
    };
    assert!(handover.local, "and the game it seats you at runs here");
    assert!(state.connected, "with a host installed for it");
    assert!(app.world().contains_resource::<InstalledHost>());
}

#[test]
fn room_tab_visits_only_expanded_inputs_in_both_directions() {
    let mut app = headless();
    hold_the_clock(&mut app);
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());
    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| {
        *p == Press::Front(FrontPress::PlayOffline)
    });
    open_a_table(&mut app);
    // Every field the caret visits on a whole round of Tab, either way: the
    // kit's walker over the room's stops (`lobby::orders::ROOM`).
    let round = |app: &mut App, backwards: bool| {
        let mut seen = Vec::new();
        for _ in 0..40 {
            if backwards {
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::ShiftLeft);
            }
            app.world_mut()
                .resource_mut::<Messages<KeyboardInput>>()
                .write(pressed(KeyCode::Tab, Key::Tab));
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(KeyCode::ShiftLeft);
            let state = app.world().resource::<LobbyState>();
            if state.lobby.typing_here() {
                let field = state.lobby.focus();
                if !seen.contains(&field) {
                    seen.push(field);
                }
            }
        }
        seen
    };
    // The room draws no box until a drawer is open: a round of Tab types
    // into nothing (the room's name is the Create-table sheet's, WP2).
    assert_eq!(round(&mut app, false), Vec::<Field>::new());
    assert_eq!(round(&mut app, true), Vec::<Field>::new());
    open_drawer(&mut app, 1);
    assert_eq!(round(&mut app, false), vec![Field::RoomBoard(1)]);
    assert_eq!(round(&mut app, true), vec![Field::RoomBoard(1)]);
    // The drawer stays open through Edit rules.
    five_lands(&mut app);
    tap_control(&mut app, "edit a starting card", |p| {
        *p == Press::Room(RoomPress::RoomCardEdit(1, 0))
    });
    let both = round(&mut app, false);
    assert!(
        both.contains(&Field::RoomBoard(1))
            && both.contains(&Field::RoomCounter)
            && both.len() == 2,
        "{both:?}"
    );
    open_drawer(&mut app, 1);
    assert_eq!(round(&mut app, true), Vec::<Field>::new());
}

/// A still clock for a test that presses Apply in an open room. The room's
/// two-second re-read of the table list (`systems::watch`; offline there is
/// no feed) marks the lobby busy for a few frames, and an Apply pressed in
/// one of them is refused: on the wall clock those frames fell wherever a
/// loaded machine put them.
fn hold_the_clock(app: &mut App) {
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
}

/// Opens (or shuts) a seat's drawer the way the host does: its `⋯`, then
/// Starting position.
fn open_drawer(app: &mut App, seat: u8) {
    press(
        app,
        Press::Shared(SharedPress::OpenMenu(ShellMenu::Seat(seat))),
    );
    app.update();
    tap_control(app, "starting position", |p| {
        *p == Press::Room(RoomPress::RoomSetup(seat))
    });
}

/// The room's rules to the five-land start, through Edit rules (WP2).
fn five_lands(app: &mut App) {
    press(app, Press::Room(RoomPress::EditRules));
    app.update();
    press(
        app,
        Press::Play(PlayPress::Template(
            client_core::lobby::play::Template::FiveLand,
        )),
    );
    app.update();
    tap_control(app, "apply", |p| *p == Press::Play(PlayPress::Open));
}

#[test]
fn starting_cards_expand_per_seat_without_losing_the_draft_or_hidden_focus() {
    let mut app = headless();
    hold_the_clock(&mut app);
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());
    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| {
        *p == Press::Front(FrontPress::PlayOffline)
    });
    open_a_table(&mut app);
    assert!(!presses(&mut app).contains(&Press::Shared(SharedPress::Focus(Field::RoomBoard(0)))));
    // A nonempty preset stays visible as a count even while its editor is shut.
    five_lands(&mut app);
    assert!(
        labels(&mut app)
            .iter()
            .any(|s| s.contains("Starting cards · 5"))
    );
    open_drawer(&mut app, 0);
    tap_control(&mut app, "focus card search", |p| {
        *p == Press::Shared(SharedPress::Focus(Field::RoomBoard(0)))
    });
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .set_field(Field::RoomBoard(0), "island");
    open_drawer(&mut app, 1);
    let controls = presses(&mut app);
    assert!(!controls.contains(&Press::Shared(SharedPress::Focus(Field::RoomBoard(0)))));
    assert!(controls.contains(&Press::Shared(SharedPress::Focus(Field::RoomBoard(1)))));
    let state = app.world().resource::<LobbyState>();
    // Never the hidden box: seat one's board search is gone with its drawer.
    assert!(!matches!(
        state.lobby.focus(),
        Field::RoomBoard(0) | Field::RoomCounter | Field::RoomName
    ));
    assert_eq!(state.lobby.field(Field::RoomBoard(0)), "island");
    assert_eq!(
        state.lobby.room_draft().unwrap().setup.seats[0]
            .permanents
            .len(),
        5
    );
    open_drawer(&mut app, 1);
    assert!(!presses(&mut app).contains(&Press::Shared(SharedPress::Focus(Field::RoomBoard(1)))));
}

/// And the one-tap duel is one tap.
///
/// `mode: "ai"` seats you at once — the lobby takes that handover straight
/// to the table without waiting for anybody — so a performer that opened a
/// room and started nothing would hand over a seat with no game behind it,
/// and this button would fail every single press.
#[test]
fn playing_the_house_offline_is_still_one_press() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());

    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| {
        *p == Press::Front(FrontPress::PlayOffline)
    });
    let words = labels(&mut app);
    assert!(words.contains(&Phrase::OfflineReadyToPlay.text(Lang::En).to_string()));
    assert!(!words.contains(&Phrase::NoTablesOpen.text(Lang::En).to_string()));
    let mut controls = app.world_mut().query::<&Press>();
    assert_eq!(
        controls
            .iter(app.world())
            .filter(|p| **p == Press::Play(PlayPress::PlayHouse))
            .count(),
        1
    );
    let mut lists = app.world_mut().query::<&Scrollable>();
    assert!(!lists.iter(app.world()).any(|list| list.0 == List::Games));
    tap_control(&mut app, "play the house", |p| {
        *p == Press::Play(PlayPress::PlayHouse)
    });

    let state = app.world().resource::<LobbyState>();
    assert!(
        matches!(state.lobby.screen(), Screen::Seated(h) if h.local),
        "one press seats you: {:?}",
        state.lobby.screen()
    );
    assert!(state.connected, "with a host installed for it");
}

/// Offline, the screen asks nothing that only a gateway could answer.
///
/// A search over open tables, the refresh beside it and a room password are
/// three questions about other people, and offline there are none: the only
/// table that can exist is the one this process is holding, and it is on the
/// screen already. They were drawn anyway, and pressing either button sent a
/// request the performer answered with "that needs a gateway" — a control
/// whose whole behaviour is a refusal.
#[test]
fn the_offline_lobby_draws_none_of_the_gateways_controls() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());

    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| {
        *p == Press::Front(FrontPress::PlayOffline)
    });
    let presses: Vec<Press> = {
        let mut query = app.world_mut().query::<&Press>();
        query.iter(app.world()).copied().collect()
    };
    assert!(
        presses.contains(&Press::Play(PlayPress::CreateTable)),
        "the table's own controls stay: {presses:?}"
    );
    assert!(
        !presses.contains(&Press::Shared(SharedPress::Focus(Field::Search))),
        "nothing to search: {presses:?}"
    );
    assert!(
        !presses
            .iter()
            .any(|p| matches!(p, Press::Play(PlayPress::Chip(_)))),
        "nothing to filter: {presses:?}"
    );
    let words = labels(&mut app);
    assert!(
        !words.iter().any(|l| l.contains("ROOM PASSWORD")),
        "and no lock on a room nobody else can reach: {words:?}"
    );
}

#[test]
fn a_table_we_are_waiting_at_is_announced_and_not_sat_at() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None,
        });
        state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 96,
            sideboard: 0,
            commanders: Vec::new(),
            ..Default::default()
        }]));
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
        state.lobby.host(GameMode::Open);
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "0123456789".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
            local: false,
        }));
    }
    app.update();
    assert!(
        labels(&mut app)
            .iter()
            .any(|l| l.contains("waiting for an opponent")),
        "the open table is on screen"
    );
    // And no duel was opened: a socket here would be closed straight back.
    assert!(app.world().get_resource::<InstalledHost>().is_none());
}

#[test]
fn a_reply_that_lands_after_the_seat_was_taken_does_not_dial_again() {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        // Against the house, which is the one seat that is playable the
        // moment it is granted: `mode:"ai"` orders an engine before the
        // gateway answers. A chair at a *room* is held rather than played,
        // so asking for one of those would set up a different situation
        // altogether — the seat screen would still be waiting.
        state.lobby.host(GameMode::Ai);
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
            local: false,
        }));
        // Stand in for a dial that already succeeded.
        state.connected = true;
    }
    // A `ListGames` that was already in flight when the seat was granted.
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Event(LobbyEvent::Games(GameListing::default())));
    app.update();
    assert!(
        matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Seated(_)
        ),
        "a second dial would have failed and unseated us"
    );
}

/// The other end of the same press. A room opened by somebody else's rematch
/// is a waiting table with this player's chair in it, and the only thing that
/// distinguishes it is the flag the gateway sets — without which the row
/// would offer *ready*, be answered `200`, and change nothing at all.
#[test]
fn a_rematch_room_in_the_list_asks_to_be_played_rather_than_readied() {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state
            .lobby
            .apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
                id: "g2".to_string(),
                name: "Kitchen".to_string(),
                state: "waiting".to_string(),
                rematch: true,
                seats: vec![
                    GameSeat {
                        seat: 0,
                        taken: true,
                        you: true,
                        ready: false,
                        ..GameSeat::default()
                    },
                    GameSeat {
                        seat: 1,
                        taken: true,
                        ready: true,
                        ..GameSeat::default()
                    },
                ],
                ..GameSummary::default()
            }])));
    }
    app.update();
    let found = presses(&mut app);
    assert!(
        found.contains(&Press::Play(PlayPress::PlayAgain(0))),
        "{found:?}"
    );
    assert!(
        !found.contains(&Press::Room(RoomPress::Ready(0, true))),
        "ready cannot claim a reserved chair: {found:?}"
    );
}

#[test]
fn a_table_that_is_full_offers_no_join() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None,
        });
        state
            .lobby
            .apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
                id: "g".to_string(),
                state: "playing".to_string(),
                seats: vec![
                    GameSeat {
                        seat: 0,
                        taken: true,
                        ..GameSeat::default()
                    },
                    GameSeat {
                        seat: 1,
                        taken: true,
                        ..GameSeat::default()
                    },
                ],
                ..GameSummary::default()
            }])));
    }
    app.update();
    assert!(!presses(&mut app).contains(&Press::Play(PlayPress::Join(0))));
}

/// `KEYBOARD.md` §1.7: the room opens with the ring's place on the first
/// seat (hidden until a key is pressed).
#[test]
fn the_room_opens_on_its_first_seat() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());
    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| {
        *p == Press::Front(FrontPress::PlayOffline)
    });
    open_a_table(&mut app);
    for _ in 0..4 {
        app.update();
    }
    let focused = app
        .world()
        .resource::<bevy::input_focus::InputFocus>()
        .get();
    let stop = focused.and_then(|e| app.world().get::<crate::shellkit::focus::Stop>(e).copied());
    assert_eq!(stop.map(|s| (s.table, s.id)), Some(("room", "seats")));
}

/// Into a room of a table opened offline, with the device's settings the
/// tour's glue reads.
fn seated_in_a_room() -> App {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());
    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| {
        *p == Press::Front(FrontPress::PlayOffline)
    });
    open_a_table(&mut app);
    for _ in 0..4 {
        app.update();
    }
    app
}

/// The room chapter's last bubble leaves the table the tour opened, as the
/// room's own Leave does: the host is asked first (09.10.: a real table
/// stayed behind after the tour).
#[test]
fn the_tours_leave_leaves_the_room_as_the_rooms_own_leave_does() {
    let mut app = seated_in_a_room();
    let (game, yours) = {
        let state = app.world().resource::<LobbyState>();
        let seat = state.lobby.awaiting().expect("seated in the room");
        let game = state
            .lobby
            .games()
            .iter()
            .find(|g| g.id == seat.game_id)
            .expect("the room's table is listed");
        (game.id.clone(), game.yours)
    };
    app.world_mut()
        .resource_mut::<crate::tour::TourDesk>()
        .asked = Some(crate::tour::TourPress::LeaveTable);
    for _ in 0..4 {
        app.update();
    }
    let state = app.world().resource::<LobbyState>();
    if yours {
        assert!(
            matches!(
                &state.confirmation,
                Some(super::confirm::Destructive::LeaveHosting(id)) if *id == game
            ),
            "the host's Leave is not asked"
        );
    } else {
        assert!(state.lobby.awaiting().is_none(), "still in the room");
    }
}

/// A tour set aside (here: a lobby chapter waiting for Play while the player
/// sits in a room) shows nothing, so it holds neither the keys nor the
/// stack's modal (09.10.: a client that looked frozen at a table).
#[test]
fn a_parked_tour_holds_neither_the_keys_nor_the_modal() {
    let mut app = seated_in_a_room();
    // The room's own chapter seen, so the Play chapter waits rather than
    // giving way to it.
    app.world_mut()
        .resource_mut::<crate::settings::ClientSettings>()
        .tours
        .seen
        .insert(baylee_client_core::tour::mark(
            baylee_client_core::tour::Tour::Lobby,
            "room",
        ));
    app.world_mut().resource_mut::<crate::tour::TourDesk>().run =
        baylee_client_core::tour::Run::chapter(baylee_client_core::tour::Tour::Lobby, 1, false, 0);
    for _ in 0..4 {
        app.update();
    }
    let desk = app.world().resource::<crate::tour::TourDesk>();
    assert!(desk.parked, "a Play chapter stood up in the room");
    assert!(!desk.holds_keyboard(), "a parked tour holds the keys");
    let stack = app.world().resource::<crate::shellkit::keys::ShellStack>();
    assert!(!stack.stack.modal, "a parked tour left the room modal");
    let state = app.world().resource::<LobbyState>();
    assert_eq!(
        crate::lobby::shortcuts::modal_by(state, baylee_client_core::shellkeys::Context::Room),
        Vec::<&str>::new()
    );
}

/// A listed local delegate or hosted model, without any provider or bridge.
fn llm_deck_room(hosted: bool, host: bool) -> App {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::Decks(vec![
            DeckSummary {
                id: "mine".into(),
                name: "Mine".into(),
                ..Default::default()
            },
            DeckSummary {
                id: "chosen".into(),
                name: "For the model".into(),
                ..Default::default()
            },
        ]));
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "deck-room".into(),
            seat: 0,
            seat_token: "seat".into(),
            local: false,
        }));
        let followup = state
            .lobby
            .apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
                id: "deck-room".into(),
                state: "waiting".into(),
                yours: host,
                seats: vec![
                    GameSeat {
                        seat: 0,
                        taken: true,
                        you: true,
                        host,
                        deck: "Mine".into(),
                        ..Default::default()
                    },
                    GameSeat {
                        seat: 1,
                        taken: true,
                        delegated_by: (!hosted).then(|| "Host".into()),
                        hosted: hosted.then(|| client_core::lobby::hosted::HostedChair {
                            profile: "test".into(),
                            label: "Hosted model".into(),
                            vendor: "Test".into(),
                            model: "model".into(),
                            state: "starting".into(),
                            note: None,
                        }),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }])));
        if host {
            // Opening the host's room also asks for hosted profiles. Finish
            // that exchange, just as the real mailbox does; dropping the
            // returned request would leave this fixture busy forever.
            assert_eq!(followup, Some(LobbyRequest::HostedProfiles));
            assert!(state.lobby.busy());
            assert!(super::super::room::deck_request(&mut state, 0, 1, 1).is_none());
            assert_eq!(
                state.lobby.apply(LobbyEvent::HostedProfiles(Vec::new())),
                None
            );
        } else {
            assert_eq!(followup, None);
        }
        assert!(!state.lobby.busy());
    }
    app.update();
    app
}

#[test]
fn local_and_hosted_llm_chairs_offer_saved_decks_to_the_host() {
    use super::super::room::{RoomPress, deck_request};
    for hosted in [false, true] {
        let mut app = llm_deck_room(hosted, true);
        tap_control(&mut app, "the model's deck picker", |p| {
            *p == Press::Room(RoomPress::RoomDeckPicker(1))
        });
        assert!(presses(&mut app).contains(&Press::Room(RoomPress::RoomDeck(0, 1, 1))));
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let selected = state.lobby.selected();
        assert!(
            !state.lobby.busy(),
            "the room's initial reads have completed"
        );
        assert_eq!(
            deck_request(&mut state, 0, 1, 1),
            Some(LobbyRequest::SetSeat {
                game_id: "deck-room".into(),
                seat: 1,
                kind: None,
                ai: None,
                deck_id: Some("chosen".into()),
                team: None,
            }),
            "hosted={hosted}: the chosen user deck goes to the model's chair"
        );
        assert_eq!(
            state.lobby.selected(),
            selected,
            "the host keeps their own pick"
        );
    }
}

#[test]
fn local_and_hosted_llm_deck_presses_recheck_host_authority() {
    use super::super::room::{RoomPress, deck_request};
    for hosted in [false, true] {
        let mut app = llm_deck_room(hosted, false);
        assert!(!presses(&mut app).contains(&Press::Room(RoomPress::RoomDeckPicker(1))));
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        assert!(deck_request(&mut state, 0, 1, 1).is_none());
        assert!(!state.lobby.busy());
    }
    let mut app = llm_deck_room(false, true);
    tap_control(&mut app, "the model's deck picker", |p| {
        *p == Press::Room(RoomPress::RoomDeckPicker(1))
    });
    let mut state = app.world_mut().resource_mut::<LobbyState>();
    let mut games = state.lobby.games().to_vec();
    games[0].seats[1].delegated_by = None; // A person took this chair meanwhile.
    state.lobby.apply(LobbyEvent::Games(GameListing::of(games)));
    assert!(deck_request(&mut state, 0, 1, 1).is_none());
}

#[test]
fn protected_table_password_is_requested_only_after_selecting_it() {
    let mut app = headless();
    stocked(&mut app);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
            id: "protected-room".into(),
            name: "Friends".into(),
            locked: true,
            state: "waiting".into(),
            seats: vec![
                GameSeat::default(),
                GameSeat {
                    seat: 1,
                    ..default()
                },
            ],
            ..default()
        }])));
    app.update();
    assert!(!presses(&mut app).contains(&Press::Shared(SharedPress::Focus(Field::RoomPassword))));
    press(&mut app, Press::Play(PlayPress::Join(0)));
    app.update();
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .play
            .password_for
            .as_deref(),
        Some("protected-room")
    );
    assert!(presses(&mut app).contains(&Press::Play(PlayPress::JoinProtected)));
    assert!(
        !app.world().resource::<LobbyState>().lobby.busy(),
        "opening the prompt sends no request"
    );
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(KeyCode::Tab, Key::Tab));
    app.update();
    let focused = app
        .world()
        .resource::<bevy::input_focus::InputFocus>()
        .get()
        .expect("modal focus");
    let stop = app
        .world()
        .get::<crate::shellkit::focus::Stop>(focused)
        .expect("modal stop");
    assert_eq!(stop.table, orders::CREATE.name);
    assert_eq!(stop.id, "cancel");
    for _ in 0..2 {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(pressed(KeyCode::Tab, Key::Tab));
        app.update();
    }
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.focus(),
        Field::RoomPassword
    );
    press(&mut app, Press::Play(PlayPress::CloseSheet));
    app.update();
    assert!(
        app.world()
            .resource::<LobbyState>()
            .play
            .password_for
            .is_none()
    );
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.focus(),
        Field::Search
    );
}

#[test]
fn discovery_sidebar_restores_its_own_scroll_after_search_rebuilds() {
    let mut app = headless();
    stocked(&mut app);
    app.world_mut()
        .resource_mut::<Scrolled>()
        .set(List::Discovery, 80.0);
    app.world_mut()
        .resource_mut::<Scrolled>()
        .set(List::Games, 112.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .set_field(Field::Search, "friends");
    app.update();
    let mut lists = app.world_mut().query::<(&Scrollable, &ScrollPosition)>();
    let sidebar = lists
        .iter(app.world())
        .find(|(list, _)| list.0 == List::Discovery)
        .expect("scrollable gateway sidebar");
    assert!((sidebar.1.y - 80.0).abs() < f32::EPSILON);
    assert!((app.world().resource::<Scrolled>().get(List::Games) - 112.0).abs() < f32::EPSILON);
}

/// Play's deck picker scrolls a long shelf: its body is a `Scrollable` the
/// wheel moves, and a bar stands beside it aimed at that body (report
/// 30.10.2026: the body was only clipped, so the wheel moved nothing and
/// no bar said there was more).
#[test]
fn the_deck_picker_scrolls_and_has_its_bar() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None,
        });
        state.lobby.apply(LobbyEvent::Decks(
            (0..24)
                .map(|n| DeckSummary {
                    id: format!("d{n}"),
                    name: format!("Deck {n}"),
                    cards: 60,
                    copies: 60,
                    ..Default::default()
                })
                .collect(),
        ));
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
        state.play.picker = true;
    }
    app.update();
    let picker = app
        .world_mut()
        .query::<(Entity, &Scrollable)>()
        .iter(app.world())
        .find(|(_, s)| s.0 == List::DeckPicker)
        .map(|(entity, _)| entity)
        .expect("the picker's body scrolls to the wheel");
    let aimed = app
        .world_mut()
        .query::<&bevy::ui_widgets::Scrollbar>()
        .iter(app.world())
        .any(|bar| bar.target == picker);
    assert!(aimed, "and a bar is aimed at it");
}
