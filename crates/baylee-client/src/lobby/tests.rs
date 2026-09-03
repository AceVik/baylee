//! The lobby's tests: route mapping without a gateway, and node
//! decisions without a window.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use super::{http::*, preview::*, systems::*, ui::*};
use baylee_client_core::lobby::{
    DeckSummary, GameListing, GameQuery, GameSeat, GameSummary, SeatHandover,
};

fn body(request: &ehttp::Request) -> serde_json::Value {
    serde_json::from_slice(&request.body).expect("a JSON body")
}

fn answer(status: u16, body: &str) -> ehttp::Response {
    ehttp::Response {
        url: "http://gw/".to_string(),
        ok: (200..300).contains(&status),
        status,
        status_text: String::new(),
        headers: ehttp::Headers::new(&[]),
        bytes: body.as_bytes().to_vec(),
    }
}

#[test]
fn every_request_hits_the_route_the_gateway_serves() {
    let cases = [
        (
            LobbyRequest::LogIn {
                email: "a@b.c".to_string(),
                password: "pw".to_string(),
            },
            "POST",
            "http://gw/auth/login",
        ),
        (
            LobbyRequest::Register {
                email: "a@b.c".to_string(),
                display_name: "V".to_string(),
                password: "pw".to_string(),
            },
            "POST",
            "http://gw/auth/register",
        ),
        (LobbyRequest::ListDecks, "GET", "http://gw/decks"),
        (
            LobbyRequest::SaveDeck {
                deck_id: None,
                name: "d".to_string(),
                cards: vec!["1 Forest".to_string()],
                sideboard: Vec::new(),
                commander: None,
            },
            "POST",
            "http://gw/decks",
        ),
        (
            LobbyRequest::SaveDeck {
                deck_id: Some("d1".to_string()),
                name: "d".to_string(),
                cards: vec!["1 Forest".to_string()],
                sideboard: Vec::new(),
                commander: None,
            },
            "PUT",
            "http://gw/decks/d1",
        ),
        (LobbyRequest::LoadPool, "GET", "http://gw/pool?lang=en"),
        (
            LobbyRequest::LoadDeck {
                deck_id: "d1".to_string(),
            },
            "GET",
            "http://gw/decks/d1",
        ),
        (
            LobbyRequest::DeleteDeck {
                deck_id: "d1".to_string(),
            },
            "DELETE",
            "http://gw/decks/d1",
        ),
        (
            LobbyRequest::ListGames(GameQuery {
                q: "a room".to_string(),
                offset: 8,
                limit: 8,
            }),
            "GET",
            "http://gw/lobby/games?q=a%20room&offset=8&limit=8",
        ),
        (
            LobbyRequest::CreateGame {
                deck_id: "d1".to_string(),
                mode: GameMode::Ai,
                chairs: 2,
                name: String::new(),
                password: String::new(),
            },
            "POST",
            "http://gw/lobby/games",
        ),
        (
            LobbyRequest::JoinGame {
                game_id: "g1".to_string(),
                deck_id: "d1".to_string(),
                seat: None,
                password: String::new(),
            },
            "POST",
            "http://gw/lobby/games/g1/join",
        ),
    ];
    for (request, method, url) in cases {
        let (built, _) = build("http://gw", None, "en", request.clone());
        assert_eq!(built.method, method, "{request:?}");
        assert_eq!(built.url, url, "{request:?}");
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one assertion per body the gateway reads
fn the_bodies_carry_the_field_names_the_gateway_deserialises() {
    let (login, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::LogIn {
            email: "a@b.c".to_string(),
            password: "pw".to_string(),
        },
    );
    assert_eq!(
        body(&login),
        serde_json::json!({ "email": "a@b.c", "password": "pw" })
    );
    let (register, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::Register {
            email: "a@b.c".to_string(),
            display_name: "V".to_string(),
            password: "pw".to_string(),
        },
    );
    assert_eq!(
        body(&register),
        serde_json::json!({
            "email": "a@b.c",
            "display_name": "V",
            "password": "pw",
            "lang": "en"
        })
    );
    let (deck, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::SaveDeck {
            deck_id: None,
            name: "Starter".to_string(),
            cards: vec!["1 Forest".to_string()],
            sideboard: vec!["2 Naturalize".to_string()],
            commander: None,
        },
    );
    assert_eq!(
        body(&deck),
        serde_json::json!({
            "name": "Starter",
            "cards": ["1 Forest"],
            "sideboard": ["2 Naturalize"],
            "commander": null
        })
    );
    let (game, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::CreateGame {
            deck_id: "d1".to_string(),
            mode: GameMode::Open,
            chairs: 2,
            name: String::new(),
            password: String::new(),
        },
    );
    assert_eq!(
        body(&game),
        serde_json::json!({ "deck_id": "d1", "mode": "open", "seats": 2, "name": "", "password": "" })
    );
    let (join, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::JoinGame {
            game_id: "g1".to_string(),
            deck_id: "d1".to_string(),
            seat: None,
            password: String::new(),
        },
    );
    assert_eq!(
        body(&join),
        serde_json::json!({ "deck_id": "d1", "seat": null, "password": "" })
    );

    // Arranging a chair, which is the room's own verb.
    let (chair, expect) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::SetSeat {
            game_id: "g1".to_string(),
            seat: 2,
            kind: Some(SeatKind::Ai),
            ai: Some("sharp".to_string()),
            deck_id: None,
            team: Some(2),
        },
    );
    assert_eq!(chair.url, "http://gw/lobby/games/g1/seats/2");
    assert_eq!(
        body(&chair),
        serde_json::json!({ "kind": "ai", "ai": "sharp", "deck_id": null, "team": 2 })
    );
    assert!(
        matches!(expect, Expect::Moved),
        "the room moved, so the page being read is asked for again"
    );
}

#[test]
fn a_trailing_slash_on_the_gateway_does_not_double_up() {
    // `gateway_url()` trims one, but a hand-set `.env` is not the only way
    // in and a `//decks` is a 404 with no explanation.
    let (built, _) = build("http://gw/", None, "en", LobbyRequest::ListDecks);
    assert!(!built.url.contains("//decks"), "{}", built.url);
}

/// A search is a person's typing, and a person types `&`, `#` and spaces.
/// Any of them straight into a URL is a query the gateway reads as something
/// else — or, with a token, as somebody else's parameters.
#[test]
fn a_typed_search_survives_the_query_string() {
    let query = GameQuery {
        q: "tom & jerry #2".to_string(),
        offset: 16,
        limit: 8,
    };
    assert_eq!(
        super::http::params(&query),
        "q=tom%20%26%20jerry%20%232&offset=16&limit=8"
    );
    // The socket and the button ask the same question, in the same words:
    // the feed builds its URL out of this too.
    let (built, _) = build("http://gw", None, "en", LobbyRequest::ListGames(query));
    assert!(
        built
            .url
            .ends_with("q=tom%20%26%20jerry%20%232&offset=16&limit=8"),
        "{}",
        built.url
    );
}

#[test]
fn only_a_signed_in_lobby_sends_a_token() {
    let (anonymous, _) = build("http://gw", None, "en", LobbyRequest::ListDecks);
    assert_eq!(anonymous.headers.get("Authorization"), None);
    let (signed, _) = build("http://gw", Some("tok"), "en", LobbyRequest::ListDecks);
    assert_eq!(signed.headers.get("Authorization"), Some("Bearer tok"));
}

#[test]
fn a_json_body_says_so() {
    let (built, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::ListGames(GameQuery::default()),
    );
    assert!(built.body.is_empty(), "a GET carries none");
    let (built, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::SaveDeck {
            deck_id: None,
            name: "d".to_string(),
            cards: vec!["1 Forest".to_string()],
            sideboard: Vec::new(),
            commander: None,
        },
    );
    assert_eq!(built.headers.get("Content-Type"), Some("application/json"));
}

#[test]
fn the_gateways_own_answers_decode() {
    assert_eq!(
        decode(
            Lang::En,
            Expect::LoggedIn,
            &answer(200, r#"{"token":"tok","expires_at":123}"#)
        ),
        LobbyEvent::LoggedIn {
            token: "tok".to_string()
        }
    );
    assert_eq!(
        decode(
            Lang::En,
            Expect::Decks,
            &answer(
                200,
                r#"[{"id":"d1","name":"Allytifact","cards":96,"commander":null}]"#
            )
        ),
        LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 96,
            sideboard: 0,
            commander: None,
        }])
    );
    assert_eq!(
        decode(
            Lang::En,
            Expect::Seat,
            &answer(200, r#"{"game_id":"g1","seat":1,"seat_token":"st"}"#)
        ),
        LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 1,
            seat_token: "st".to_string(),
        })
    );
    assert_eq!(
        decode(Lang::En, Expect::Registered, &answer(200, r#"{"ok":true}"#)),
        LobbyEvent::Registered {
            confirmation_required: false,
        }
    );
    assert_eq!(
        decode(
            Lang::En,
            Expect::DeckSaved,
            &answer(200, r#"{"deck_id":"d1"}"#)
        ),
        LobbyEvent::DeckSaved {
            deck_id: Some("d1".to_string())
        }
    );
}

#[test]
fn a_body_that_makes_no_sense_is_a_failure_not_a_panic() {
    assert!(matches!(
        decode(
            Lang::En,
            Expect::LoggedIn,
            &answer(200, "<html>proxy</html>")
        ),
        LobbyEvent::Failed(_)
    ));
}

#[test]
fn a_refusal_is_shown_in_the_gateways_own_words() {
    assert_eq!(
        gateway_error(Lang::En, &answer(401, r#"{"error":"invalid credentials"}"#)),
        "invalid credentials"
    );
    assert_eq!(
        gateway_error(Lang::En, &answer(502, "<html>bad gateway</html>")),
        "the gateway answered 502"
    );
}

/// A headless app wired exactly as the plugin wires a real one. No
/// renderer, so this exercises the systems and the node tree, not pixels.
fn headless() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .init_state::<DuelPhase>()
        .add_message::<DuelCommand>()
        .add_message::<KeyboardInput>()
        .add_message::<Pointer<Click>>()
        .add_message::<Pointer<Scroll>>()
        .add_message::<Pointer<Drag>>()
        .add_message::<Pointer<DragEnd>>()
        // The duel plugin's startup system would load these; a test has
        // no asset server and does not need one to build a tree.
        .insert_resource(UiFonts {
            text: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        })
        .add_plugins(LobbyPlugin);
    // The startup probe asks a gateway whether sign-ups are open. Left
    // pointing at the default address it reaches a gateway that happens to
    // be running on this machine, and that answer lands a frame or two
    // later — inside whatever the test is measuring. An address no request
    // can be built from keeps a headless test off the network entirely.
    app.world_mut().resource_mut::<LobbyState>().gateway = String::new();
    app.update();
    app
}

fn presses(app: &mut App) -> Vec<Press> {
    let mut query = app.world_mut().query::<&Press>();
    let mut found: Vec<Press> = query.iter(app.world()).copied().collect();
    found.sort_by_key(|p| format!("{p:?}"));
    found
}

fn roots(app: &mut App) -> Vec<Entity> {
    let mut query = app.world_mut().query_filtered::<Entity, With<LobbyRoot>>();
    query.iter(app.world()).collect()
}

fn typed(ch: char) -> KeyboardInput {
    KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character(ch.to_string().into()),
        state: bevy::input::ButtonState::Pressed,
        text: Some(ch.to_string().into()),
        repeat: false,
        window: Entity::PLACEHOLDER,
    }
}

#[test]
fn the_sign_in_screen_builds_with_its_controls() {
    let mut app = headless();
    assert_eq!(roots(&mut app).len(), 1, "exactly one tree");
    let found = presses(&mut app);
    for wanted in [
        Press::Focus(Field::Email),
        Press::Focus(Field::Password),
        Press::Submit,
        Press::ToggleRegistering,
        Press::PlayOffline,
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    assert!(
        !found.contains(&Press::Focus(Field::DisplayName)),
        "the display name is only asked for when registering"
    );
}

#[test]
fn the_lobby_brings_its_own_camera() {
    let mut app = headless();
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, (With<Camera>, With<LobbyScreen>)>();
    assert_eq!(query.iter(app.world()).count(), 1);
}

#[test]
fn typing_reaches_the_form() {
    let mut app = headless();
    for ch in ['h', 'i'] {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(typed(ch));
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Email),
        "hi"
    );
}

#[test]
fn a_quiet_frame_does_not_rebuild_the_tree() {
    let mut app = headless();
    let before = roots(&mut app);
    app.update();
    app.update();
    assert_eq!(roots(&mut app), before, "the retained tree survived");
}

#[test]
fn the_table_screen_builds_once_there_is_a_deck() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
        });
        state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 96,
            sideboard: 0,
            commander: None,
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
    let found = presses(&mut app);
    for wanted in [
        Press::SignOut,
        Press::Refresh,
        Press::StarterDeck,
        Press::SelectDeck(0),
        Press::Host(GameMode::Ai),
        Press::OpenRoom(2),
        Press::OpenRoom(4),
        Press::Join(0),
        // The chairs of a waiting table are drawn for everyone, so a
        // player can take the one they want rather than whichever the
        // gateway would have handed them.
        Press::JoinSeat(0, 1),
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
}

fn labels(app: &mut App) -> Vec<String> {
    let mut query = app.world_mut().query::<&Text>();
    query.iter(app.world()).map(|t| t.0.clone()).collect()
}

#[test]
fn a_table_we_are_waiting_at_is_announced_and_not_sat_at() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
        });
        state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 96,
            sideboard: 0,
            commander: None,
        }]));
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
        state.lobby.host(GameMode::Open);
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "0123456789".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
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
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
        });
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
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

/// A window of a given width, so the breakpoints can be exercised without
/// a windowing system.
fn sized(app: &mut App, width: f32) {
    let mut existing = app.world_mut().query::<&mut Window>();
    if let Some(mut window) = existing.iter_mut(app.world_mut()).next() {
        window.resolution.set(width, 900.0);
        return;
    }
    let mut window = Window::default();
    window.resolution.set(width, 900.0);
    app.world_mut().spawn(window);
}

#[test]
fn the_frame_follows_the_width() {
    assert_eq!(Frame::of(390.0), Frame::Phone, "a phone held upright");
    assert_eq!(Frame::of(759.0), Frame::Phone);
    assert_eq!(Frame::of(760.0), Frame::Tablet);
    assert_eq!(
        Frame::of(1024.0),
        Frame::Tablet,
        "a tablet, or a half window"
    );
    assert_eq!(Frame::of(1180.0), Frame::Desktop);
    assert_eq!(Frame::of(2560.0), Frame::Desktop);
}

#[test]
fn a_finger_gets_a_target_it_can_hit() {
    for width in [360.0_f32, 400.0, 700.0, 900.0, 1400.0] {
        let metrics = Metrics::of(width);
        assert!(
            metrics.tap >= 38.0,
            "{width} gave a {}px target",
            metrics.tap
        );
    }
    assert!(
        Metrics::of(390.0).tap >= 44.0,
        "a touch screen needs the full 44"
    );
    assert!(Metrics::of(390.0).stacked(), "a phone has one column");
    assert!(!Metrics::of(1400.0).stacked(), "a desktop has two");
}

#[test]
fn a_phone_drops_what_it_has_no_room_for() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
        });
        // A real address: `headless` empties it so the startup probe reaches
        // nothing, and an empty one would make the assertions below match any
        // empty label on the screen rather than this one.
        state.gateway = "http://gw.example:28766".to_string();
    }
    sized(&mut app, 1400.0);
    app.update();
    let wide = labels(&mut app);
    sized(&mut app, 390.0);
    app.update();
    let narrow = labels(&mut app);

    let gateway = app.world().resource::<LobbyState>().gateway.clone();
    assert!(wide.contains(&gateway), "a desktop has room to say where");
    assert!(
        !narrow.contains(&gateway),
        "a phone does not, and the address is reassurance rather than \
         information"
    );
    assert!(
        narrow.iter().any(|l| l == "Your decks"),
        "everything that matters is still there: {narrow:?}"
    );
}

#[test]
fn crossing_a_breakpoint_rebuilds_the_tree() {
    let mut app = headless();
    sized(&mut app, 1400.0);
    app.update();
    let wide = roots(&mut app);
    app.update();
    assert_eq!(roots(&mut app), wide, "the same frame keeps its tree");
    sized(&mut app, 390.0);
    app.update();
    assert_ne!(
        roots(&mut app),
        wide,
        "a different frame is a different layout, not a resize"
    );
}

#[test]
fn a_table_that_is_full_offers_no_join() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
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
    assert!(!presses(&mut app).contains(&Press::Join(0)));
}

/// Two cards, in the shape `GET /pool` sends them.
fn pool_cards() -> Vec<baylee_client_core::PoolCard> {
    serde_json::from_value(serde_json::json!([
        {
            "index": 1,
            "name": "Llanowar Elves",
            "english_name": "Llanowar Elves",
            "mana_cost": "{G}",
            "cmc": 1,
            "colors": "G",
            "identity": "G",
            "type_line": "Creature — Elf Druid",
            "kinds": ["Creature"],
            "stats": "1/1",
            "oracle_text": "{T}: Add {G}.",
            "coverage": "implemented",
            "note": null,
            "commander": false,
            "basic_land": false
        },
        {
            "index": 2,
            "name": "Forest",
            "english_name": "Forest",
            "mana_cost": "",
            "cmc": 0,
            "colors": "",
            "identity": "G",
            "type_line": "Basic Land — Forest",
            "kinds": ["Land"],
            "stats": null,
            "oracle_text": "",
            "coverage": "implemented",
            "note": null,
            "commander": false,
            "basic_land": true
        }
    ]))
    .expect("the pool shape")
}

/// A lobby signed in, with a deck listed and the pool loaded.
fn stocked(app: &mut App) {
    let mut state = app.world_mut().resource_mut::<LobbyState>();
    state.lobby.apply(LobbyEvent::LoggedIn {
        token: "tok".to_string(),
    });
    state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
        id: "d1".to_string(),
        name: "Allytifact".to_string(),
        cards: 96,
        sideboard: 0,
        commander: None,
    }]));
    state.lobby.apply(LobbyEvent::Pool {
        cards: pool_cards(),
        has_text: true,
    });
}

#[test]
fn a_deck_can_be_opened_edited_and_thrown_away_from_the_list() {
    let mut app = headless();
    stocked(&mut app);
    app.update();
    let found = presses(&mut app);
    for wanted in [
        Press::NewDeck,
        Press::EditDeck(0),
        Press::DeleteDeck(0),
        Press::StarterDeck,
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
}

#[test]
fn the_builder_screen_builds_with_its_controls() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    let found = presses(&mut app);
    for wanted in [
        Press::CloseBuilder,
        Press::FocusBuild(BuildField::Search),
        Press::FocusBuild(BuildField::Name),
        Press::SetZone(Zone::Main),
        Press::SetZone(Zone::Side),
        Press::ToggleColor('G'),
        Press::SetKind(Some("Creature")),
        Press::SetCmc(0),
        Press::TogglePlayable,
        Press::CycleSort,
        // Both pool rows are offered, so the search does not have to be
        // used to reach a two-card pool.
        Press::AddCard(0),
        Press::AddCard(1),
        // Every row can be read as well as taken.
        Press::Inspect(0),
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    // Nothing is saveable yet: no name, no cards.
    assert!(
        !found.contains(&Press::SaveDeck),
        "a deck the gateway would refuse offers no save"
    );
}

#[test]
fn a_deck_worth_saving_offers_the_save() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        let builder = state.lobby.builder_mut();
        builder.set_name("Elves");
        assert!(builder.add(0, Zone::Main), "the pool has that card");
    }
    app.update();
    let found = presses(&mut app);
    assert!(found.contains(&Press::SaveDeck), "{found:?}");
    assert!(
        found.contains(&Press::RemoveRow(0)),
        "a card in the deck can come back out: {found:?}"
    );
}

#[test]
fn a_phone_shows_one_half_of_the_builder_at_a_time() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 390.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    let cards = presses(&mut app);
    assert!(cards.contains(&Press::AddCard(0)), "the pool is showing");
    assert!(
        !cards.contains(&Press::SetZone(Zone::Side)),
        "and the deck is not: {cards:?}"
    );
    assert!(
        cards.contains(&Press::ShowPane(Pane::Deck)),
        "with a way over"
    );
    // The chips are folded away, or the list under them would be four
    // rows tall.
    assert!(
        !cards.contains(&Press::SetKind(Some("Creature"))),
        "{cards:?}"
    );
    assert!(cards.contains(&Press::ToggleFilters), "but reachable");
    app.world_mut().resource_mut::<LobbyState>().filters_open = true;
    app.update();
    assert!(
        presses(&mut app).contains(&Press::SetKind(Some("Creature"))),
        "unfolded, every filter is there"
    );
    app.world_mut().resource_mut::<LobbyState>().filters_open = false;

    app.world_mut().resource_mut::<LobbyState>().pane = Pane::Deck;
    app.update();
    let list = presses(&mut app);
    assert!(list.contains(&Press::SetZone(Zone::Side)), "{list:?}");
    assert!(!list.contains(&Press::AddCard(0)), "{list:?}");

    // Both halves are reachable on a desktop at once.
    sized(&mut app, 1400.0);
    app.update();
    let both = presses(&mut app);
    assert!(both.contains(&Press::AddCard(0)) && both.contains(&Press::SetZone(Zone::Side)));
}

#[test]
fn typing_reaches_the_builder_and_return_adds_the_first_hit() {
    let mut app = headless();
    stocked(&mut app);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    // A new deck starts on its name, which is what stops it being saved.
    for ch in ['E', 'l', 'f'] {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(typed(ch));
    }
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.builder().name(),
        "Elf"
    );

    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .focus_on(BuildField::Search);
    for ch in ['E', 'l', 'v'] {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(typed(ch));
    }
    app.update();
    {
        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.lobby.builder().text(), "Elv");
        assert_eq!(state.lobby.builder().results().len(), 1, "one match");
    }
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: Key::Enter,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert_eq!(
        state.lobby.builder().count_of(0, Zone::Main),
        1,
        "return took the one card the search left"
    );
}

#[test]
#[allow(clippy::float_cmp)] // every value here is exact by construction
fn a_long_list_can_be_scrolled_and_stops_at_both_ends() {
    // Three hundred pixels of window over nine hundred of cards.
    assert_eq!(scrolled(0.0, 120.0, 300.0, 900.0, 1.0), 120.0);
    assert_eq!(
        scrolled(500.0, 400.0, 300.0, 900.0, 1.0),
        600.0,
        "the bottom of the list is the end of it"
    );
    assert_eq!(
        scrolled(40.0, -400.0, 300.0, 900.0, 1.0),
        0.0,
        "and so is the top"
    );
    assert_eq!(
        scrolled(0.0, 50.0, 300.0, 300.0, 1.0),
        0.0,
        "a list that fits does not move at all"
    );
    // Physical sizes, logical offset: a 2× screen has half the room.
    assert_eq!(scrolled(0.0, 999.0, 300.0, 900.0, 0.5), 300.0);
}

#[test]
fn every_scrolling_list_carries_what_bevy_needs_to_scroll_it() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    let mut query = app
        .world_mut()
        .query_filtered::<(&Node, Option<&ScrollPosition>), With<Scrollable>>();
    let lists: Vec<_> = query.iter(app.world()).collect();
    assert_eq!(lists.len(), 2, "the pool and the deck each scroll");
    for (node, position) in lists {
        assert_eq!(node.overflow.y, OverflowAxis::Scroll);
        assert!(
            position.is_some(),
            "an overflow with no ScrollPosition only clips"
        );
    }
}

/// Presses one control by name, in one line.
fn press(app: &mut App, wanted: Press) {
    let target = press_target(app, wanted);
    tap(app, target);
    app.update();
}

/// The whole rebinding flow, without a window: open the screen, arm a
/// row, press a key, and find it bound. Every step of it is a place the
/// keymap could quietly not be written.
#[test]
fn a_key_can_be_rebound_from_the_settings_screen() {
    use baylee_client_core::prefs::{Action, Chord};

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();

    press(&mut app, Press::OpenSettings);
    assert!(app.world().resource::<LobbyState>().settings.is_open());

    press(&mut app, Press::Rebind(Action::Confirm));
    assert_eq!(
        app.world().resource::<LobbyState>().settings.capturing(),
        Some(Action::Confirm),
        "the row is not waiting for a key"
    );

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyP);
    app.update();
    assert_eq!(
        app.world()
            .resource::<crate::prefs::Prefs>()
            .keymap()
            .chords(Action::Confirm),
        &[Chord::key("KeyP")],
        "the key was not bound"
    );
    assert_eq!(
        app.world().resource::<LobbyState>().settings.capturing(),
        None,
        "the row is still armed after taking a key"
    );

    // And it can be put back, one row at a time.
    press(&mut app, Press::ResetBinding(Action::Confirm));
    assert_eq!(
        app.world()
            .resource::<crate::prefs::Prefs>()
            .keymap()
            .chords(Action::Confirm),
        &[Chord::key("Enter")]
    );
}

/// Escape is a key a player may legitimately want to bind, so while a row
/// is armed it means "never mind" rather than "cancel". Backspace is the
/// other way out: unbinding is a real answer, since a pointer still
/// reaches everything.
#[test]
fn arming_a_row_can_be_backed_out_of_or_used_to_unbind() {
    use baylee_client_core::prefs::Action;

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    press(&mut app, Press::OpenSettings);

    press(&mut app, Press::Rebind(Action::Cancel));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().settings.capturing(),
        None
    );
    assert!(
        !app.world()
            .resource::<crate::prefs::Prefs>()
            .keymap()
            .chords(Action::Cancel)
            .is_empty(),
        "escape rebound the row instead of backing out of it"
    );

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    press(&mut app, Press::Rebind(Action::Cancel));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Backspace);
    app.update();
    assert!(
        app.world()
            .resource::<crate::prefs::Prefs>()
            .keymap()
            .chords(Action::Cancel)
            .is_empty(),
        "backspace did not unbind the row"
    );
}

#[test]
fn the_settings_screen_offers_every_switch_and_both_rails() {
    use baylee_client_core::automation::{RAIL_ROWS, RailSide};
    use baylee_client_core::prefs::{Action, AutoRule};

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    press(&mut app, Press::OpenSettings);

    let found = presses(&mut app);
    for action in Action::ALL {
        assert!(
            found.contains(&Press::Rebind(action)),
            "{action:?} cannot be rebound from the screen"
        );
    }
    for rule in AutoRule::ALL {
        assert!(
            found.contains(&Press::ToggleAuto(rule)),
            "{rule:?} is missing"
        );
    }
    for side in RailSide::BOTH {
        for row in RAIL_ROWS {
            assert!(
                found.contains(&Press::ToggleRail(side, row)),
                "{side:?}/{row:?} is missing from the rail"
            );
        }
    }
    assert!(found.contains(&Press::CloseSettings), "no way back");

    // A switch actually flips, and the screen redraws to say so.
    press(&mut app, Press::ToggleAuto(AutoRule::SkipEmptyBlocks));
    assert!(
        app.world()
            .resource::<crate::prefs::Prefs>()
            .auto()
            .skip_empty_blocks,
        "the switch did not take"
    );
}

/// Settings sit *over* the lobby: coming back has to land exactly where
/// the player left, including halfway through a deck.
#[test]
fn closing_the_settings_puts_the_lobby_back_as_it_was() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    assert!(matches!(
        app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Build
    ));

    // The builder has no settings button of its own — the screen is
    // reached from the tables or from sign-in — so it is opened here the
    // way a press would.
    app.world_mut().resource_mut::<LobbyState>().settings = SettingsPane::Open;
    app.update();
    press(&mut app, Press::CloseSettings);
    assert!(
        matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Build
        ),
        "the builder was lost"
    );
    assert!(
        presses(&mut app).contains(&Press::CloseBuilder),
        "not redrawn"
    );
}

/// The entity carrying a control, so a test can press it.
fn press_target(app: &mut App, wanted: Press) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &Press)>();
    let found = query.iter(app.world()).find(|(_, press)| **press == wanted);
    match found {
        Some((entity, _)) => entity,
        None => panic!("{wanted:?} is on screen"),
    }
}

/// A plain press on one control.
fn tap(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<Messages<Pointer<Click>>>()
        .write(aimed(
            entity,
            Click {
                button: PointerButton::Primary,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                duration: std::time::Duration::ZERO,
                count: 1,
            },
        ));
}

/// A pointer event aimed at one entity. The location is required and
/// never read by anything the lobby runs.
fn aimed<E: std::fmt::Debug + Clone + Reflect>(entity: Entity, event: E) -> Pointer<E> {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::window::WindowRef;
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Primary
                    .normalize(Some(Entity::PLACEHOLDER))
                    .expect("a window reference"),
            ),
            position: Vec2::ZERO,
        },
        event,
        entity,
    )
}

#[test]
fn a_swipe_scrolls_the_list_rather_than_adding_the_card_under_it() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();

    // The row a finger would land on, and the list it sits in. Layout
    // never runs here, so the list is told how big it is.
    let mut rows = app.world_mut().query::<(Entity, &Press)>();
    let card = rows
        .iter(app.world())
        .find(|(_, press)| **press == Press::AddCard(0))
        .map(|(entity, _)| entity)
        .expect("a card row");
    let mut lists = app.world_mut().query_filtered::<Entity, With<Scrollable>>();
    let list = lists.iter(app.world()).next().expect("a scrolling list");
    app.world_mut().entity_mut(list).insert(ComputedNode {
        size: Vec2::new(300.0, 300.0),
        content_size: Vec2::new(300.0, 900.0),
        ..default()
    });

    app.world_mut()
        .resource_mut::<Messages<Pointer<Drag>>>()
        .write(aimed(
            card,
            Drag {
                button: PointerButton::Primary,
                distance: Vec2::new(0.0, -40.0),
                delta: Vec2::new(0.0, -40.0),
            },
        ));
    app.world_mut()
        .resource_mut::<Messages<Pointer<DragEnd>>>()
        .write(aimed(
            card,
            DragEnd {
                button: PointerButton::Primary,
                distance: Vec2::new(0.0, -40.0),
            },
        ));
    app.world_mut()
        .resource_mut::<Messages<Pointer<Click>>>()
        .write(aimed(
            card,
            Click {
                button: PointerButton::Primary,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                duration: std::time::Duration::ZERO,
                count: 1,
            },
        ));
    app.update();

    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .count_of(0, Zone::Main),
        0,
        "a swipe is not a tap"
    );
    assert_eq!(
        app.world()
            .entity(list)
            .get::<ScrollPosition>()
            .map(|p| p.y),
        Some(40.0),
        "and it moved the list under the finger"
    );
}

#[test]
fn leaving_a_deck_with_unsaved_work_takes_two_presses() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        state.lobby.builder_mut().set_name("Half a deck");
    }
    app.update();
    let back = press_target(&mut app, Press::CloseBuilder);

    tap(&mut app, back);
    app.update();
    assert!(
        matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Build
        ),
        "the first press asks rather than leaves"
    );
    assert!(
        labels(&mut app).iter().any(|l| l == "Leave without saving"),
        "and says so"
    );

    let back = press_target(&mut app, Press::CloseBuilder);
    tap(&mut app, back);
    app.update();
    assert!(matches!(
        app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Table
    ));
}

#[test]
fn a_card_can_be_read_in_the_builder() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .inspect(0);
    app.update();
    let shown = labels(&mut app);
    assert!(
        shown.iter().any(|l| l == "{T}: Add {G}."),
        "the rules text is on screen: {shown:?}"
    );
    assert!(presses(&mut app).contains(&Press::CloseCard), "and closes");
}

#[test]
fn an_edit_answers_with_no_body_and_that_is_not_a_failure() {
    // `PUT /decks/{id}` is a 204. Reading an id out of nothing is not an
    // error here — the builder already holds the one it is editing.
    assert_eq!(
        decode(Lang::En, Expect::DeckSaved, &answer(204, "")),
        LobbyEvent::DeckSaved { deck_id: None }
    );
}

#[test]
fn a_list_keeps_its_place_when_adding_a_card_rebuilds_it() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    app.world_mut()
        .resource_mut::<Scrolled>()
        .set(List::Pool, 90.0);

    // Adding a card changes the lobby, which rebuilds the whole tree.
    let card = press_target(&mut app, Press::AddCard(0));
    tap(&mut app, card);
    app.update();

    let mut lists = app.world_mut().query::<(&ScrollPosition, &Scrollable)>();
    let pool = lists
        .iter(app.world())
        .find(|(_, which)| which.0 == List::Pool)
        .map(|(position, _)| position.y)
        .expect("the pool list");
    assert!(
        (pool - 90.0).abs() < f32::EPSILON,
        "the new list opens where the old one was, not at the top: {pool}"
    );

    // A different search *is* a different list, and starts at the top.
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .focus_on(BuildField::Search);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(typed('F'));
    app.update();
    assert!(app.world().resource::<Scrolled>().get(List::Pool).abs() < f32::EPSILON);
}

#[test]
fn the_pool_and_a_saved_deck_decode() {
    let cards = serde_json::to_string(&serde_json::json!({
        "total": 2,
        "pool_hash": "abc",
        "lang": "en",
        "has_text": true,
        "cards": []
    }))
    .expect("a body");
    assert_eq!(
        decode(Lang::En, Expect::Pool, &answer(200, &cards)),
        LobbyEvent::Pool {
            cards: Vec::new(),
            has_text: true
        }
    );
    assert_eq!(
        decode(
            Lang::En,
            Expect::DeckLoaded,
            &answer(
                200,
                r#"{"id":"d1","name":"Elves","cards":["4 Llanowar Elves"],
                   "sideboard":["1 Forest"],"commander":null}"#
            )
        ),
        LobbyEvent::DeckLoaded {
            id: "d1".to_string(),
            name: "Elves".to_string(),
            cards: vec!["4 Llanowar Elves".to_string()],
            sideboard: vec!["1 Forest".to_string()],
            commander: None,
        }
    );
    assert_eq!(
        decode(Lang::En, Expect::DeckDeleted, &answer(204, "")),
        LobbyEvent::DeckDeleted
    );
}

#[test]
fn the_starter_deck_is_one_the_gateway_will_accept() {
    let rows = starter_rows();
    assert!(
        !rows.is_empty(),
        "the acceptance file has an {STARTER} deck"
    );
    assert!(rows.len() <= 250, "the gateway caps the list at 250 rows");
    for row in &rows {
        let (count, name) = row.split_once(' ').expect("\"N Card Name\"");
        let count: u32 = count.parse().expect("a leading count");
        assert!((1..=4).contains(&count), "{row}");
        // The gateway resolves every name against the same registry, and
        // answers a miss with a 400 that says only "unknown card".
        assert!(
            baylee_cards::decks::by_name(name).is_some(),
            "{name} is not in the registry"
        );
    }
}
/// The picker is a dialog over the whole builder, and every control it
/// offers has to be reachable — a carousel with no way to move it, or a
/// finish with no way to choose it, is a dialog a player is stuck in.
#[test]
fn the_printing_picker_offers_every_control_it_needs() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        let asked = state.lobby.builder_mut().open_picker(0, Zone::Main);
        assert_eq!(asked, Some(LobbyRequest::LoadPrintings { card: 1 }));
        state.lobby.apply(LobbyEvent::Printings {
            card: 1,
            printings: serde_json::from_value(serde_json::json!([
                {
                    "scryfall_id": "11111111-2222-3333-4444-555555555555",
                    "oracle_id": "o", "lang": "en", "set": "m19",
                    "set_name": "Core Set 2019", "collector_number": "314",
                    "finishes": ["nonfoil", "foil"], "name": "Llanowar Elves"
                },
                {
                    "scryfall_id": "66666666-7777-8888-9999-aaaaaaaaaaaa",
                    "oracle_id": "o", "lang": "de", "set": "dom",
                    "set_name": "Dominaria", "collector_number": "168",
                    "finishes": ["nonfoil"], "name": "Elfen von Llanowar"
                }
            ]))
            .expect("printings decode"),
            from_catalog: true,
        });
    }
    app.update();
    let found = presses(&mut app);
    for wanted in [
        Press::PickerStep(-1),
        Press::PickerStep(1),
        Press::PickerGo(1),
        Press::PickerLang(None),
        Press::PickerLang(Some(0)),
        Press::PickerLang(Some(1)),
        Press::PickerFinish(Finish::Foil),
        Press::PickerConfirm,
        Press::PickerClose,
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
}

/// The row the pool draws is the one that opens the picker; without it
/// the whole feature is unreachable from the builder.
#[test]
fn a_pool_row_offers_a_way_to_choose_its_printing() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
    }
    app.update();
    let found = presses(&mut app);
    assert!(found.contains(&Press::PickPrint(0)), "{found:?}");
}
/// A deck row previews the printing it names, not the one the registry
/// happens to point at — that is the whole point of having chosen one.
#[test]
fn a_deck_row_previews_the_printing_it_names() {
    use baylee_client_core::deckbuilder::PoolCard;
    let card = PoolCard {
        scryfall_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
        ..PoolCard::default()
    };
    let chosen = baylee_core::deckrow::PrintChoice {
        scryfall_id: Some("11111111-2222-3333-4444-555555555555".to_string()),
        lang: Some("de".to_string()),
        finish: Some(Finish::Foil),
        ..baylee_core::deckrow::PrintChoice::default()
    };
    let hover = hover_of_entry(&card, &chosen);
    let url = hover.url.expect("a real id has art");
    assert!(
        url.contains("11111111-2222-3333-4444-555555555555"),
        "{url}"
    );
    assert_eq!(hover.finish, FinishTreatment::Foil);

    // A row that only narrowed by set has no id of its own and falls
    // back to the card's, rather than previewing nothing at all.
    let vague = baylee_core::deckrow::PrintChoice {
        set: Some("M11".to_string()),
        ..baylee_core::deckrow::PrintChoice::default()
    };
    let fallback = hover_of_entry(&card, &vague).url.expect("falls back");
    assert!(
        fallback.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
        "{fallback}"
    );
}

/// The pool's own rows preview plainly: a player has not chosen a finish
/// there, and showing one would be inventing a choice.
#[test]
fn a_pool_row_previews_plainly() {
    use baylee_client_core::deckbuilder::PoolCard;
    let card = PoolCard {
        scryfall_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
        ..PoolCard::default()
    };
    assert_eq!(hover_of_card(&card).finish, FinishTreatment::Plain);
    assert!(hover_of_card(&card).url.is_some());
}

/// A card with no usable printing must preview nothing rather than fetch
/// a guaranteed 404 — the nil id is what a preset carries.
#[test]
fn a_card_with_no_printing_previews_nothing() {
    use baylee_client_core::deckbuilder::PoolCard;
    let card = PoolCard {
        scryfall_id: "00000000-0000-0000-0000-000000000000".to_string(),
        ..PoolCard::default()
    };
    assert!(hover_of_card(&card).url.is_none());
}
