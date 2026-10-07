//! The shell's header and strips over the lobby (WP0b-3; the shell design
//! §2.1, §2.5, §17): the seated strip on every screen but the room, the
//! Phone's Return pill in its place, a session that ended landing on the
//! front door with its sentence and the draft kept, and the pill reading
//! `/me` and `/lobby/stats`.

use super::super::header::HeaderPress;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::shellkit::header::SeatStrip;
use baylee_client_core::lobby::strips::{LobbyStats, Me};
use baylee_client_core::lobby::{GameSeat, GameSummary};

/// The window at a size, both sides.
fn window(app: &mut App, width: f32, height: f32) {
    let mut existing = app.world_mut().query::<&mut Window>();
    if let Some(mut window) = existing.iter_mut(app.world_mut()).next() {
        window.resolution.set(width, height);
        return;
    }
    let mut window = Window::default();
    window.resolution.set(width, height);
    app.world_mut().spawn(window);
}

/// Signed in with a deck, holding a chair at a waiting room "Thursday pod"
/// (three of four chairs taken, one ready), as the listing says it.
fn seated() -> App {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let seat = |n: u32, who: Option<&str>, you: bool, ready: bool| GameSeat {
            seat: n,
            taken: who.is_some(),
            player: who.map(str::to_string),
            you,
            ready,
            ..GameSeat::default()
        };
        state.lobby.apply(LobbyEvent::Games(GameListing {
            games: vec![GameSummary {
                id: "pod".to_string(),
                name: "Thursday pod".to_string(),
                host: Some("Maik#0012".to_string()),
                state: "waiting".to_string(),
                seats: vec![
                    seat(0, Some("Maik#0012"), false, true),
                    seat(1, Some("me#0007"), true, false),
                    seat(2, Some("Ole#0009"), false, false),
                    seat(3, None, false, false),
                ],
                ..GameSummary::default()
            }],
            total: 1,
            ..GameListing::default()
        }));
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "pod".to_string(),
            seat: 1,
            seat_token: "st".to_string(),
            local: false,
        }));
    }
    window(&mut app, 1400.0, 900.0);
    for _ in 0..3 {
        app.update();
    }
    app
}

fn strips(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<(), With<SeatStrip>>();
    query.iter(app.world()).count()
}

fn count(app: &mut App, wanted: Press) -> usize {
    presses(app).into_iter().filter(|p| *p == wanted).count()
}

/// §2.1, M-7: the seated strip stands on every screen but the room — the
/// room is what it returns to and has its own Leave.
#[test]
fn the_strip_is_on_every_screen_but_the_room() {
    let mut app = seated();
    assert_eq!(strips(&mut app), 0, "the room draws no strip");
    assert!(
        labels(&mut app).iter().all(|l| !l.starts_with("Seated at")),
        "nor its sentence"
    );

    // Settings, over the room.
    app.world_mut().resource_mut::<LobbyState>().settings = SettingsPane::Open;
    app.update();
    assert_eq!(strips(&mut app), 1, "Settings");
    assert!(
        labels(&mut app)
            .iter()
            .any(|l| l == "Seated at \u{201c}Thursday pod\u{201d} · 3 of 4 seated · 1 ready"),
        "the strip says where and how full"
    );
    assert_eq!(count(&mut app, Press::Header(HeaderPress::Leave)), 1);
    // Return closes what stands over the room.
    press(&mut app, Press::Header(HeaderPress::Return));
    assert!(!app.world().resource::<LobbyState>().settings.is_open());
    assert_eq!(strips(&mut app), 0, "back in the room");

    // The builder.
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let _ = state.lobby.build_deck();
    }
    app.update();
    assert_eq!(
        *app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Build
    );
    assert_eq!(strips(&mut app), 1, "the builder");
}

/// S4-3: on a Phone the strip folds into the header as the gold Return pill
/// in the gateway dot's place, and no strip is drawn; wider, the strip
/// stands and the header has no Return of its own.
#[test]
fn a_phone_header_returns_and_draws_no_strip() {
    let mut app = seated();
    app.world_mut().resource_mut::<LobbyState>().settings = SettingsPane::Open;
    app.update();
    assert_eq!(strips(&mut app), 1);
    assert_eq!(
        count(&mut app, Press::Header(HeaderPress::Return)),
        1,
        "the strip's Return only"
    );
    window(&mut app, 844.0, 390.0);
    app.update();
    assert_eq!(strips(&mut app), 0, "a Phone draws no strip");
    assert_eq!(
        count(&mut app, Press::Header(HeaderPress::Return)),
        1,
        "the header's Return pill"
    );
    assert_eq!(
        count(&mut app, Press::Header(HeaderPress::Gateway)),
        0,
        "in the gateway dot's place"
    );
    assert!(labels(&mut app).iter().any(|l| l == "Return"));
}

/// §2.5 (S4-12): a `401` on the feed or a request lands on the front door
/// with the sentence, and what was being written is kept.
#[test]
fn a_session_that_ended_lands_on_the_front_door_with_the_draft_kept() {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let _ = state.lobby.build_deck();
        let builder = state.lobby.builder_mut();
        builder.set_name("Weltenbaum");
        assert!(builder.add(0, Zone::Main), "a card is added");
    }
    app.update();
    let draft = app
        .world()
        .resource::<LobbyState>()
        .lobby
        .builder()
        .entries(Zone::Main)
        .to_vec();
    assert!(!draft.is_empty());
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Expired);
    app.update();
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert!(matches!(state.lobby.screen(), Screen::SignIn { .. }));
    assert_eq!(
        state.lobby.status(),
        "Signed out \u{2014} your session ended · Sign in again"
    );
    assert_eq!(state.lobby.builder().entries(Zone::Main), draft.as_slice());
    assert_eq!(state.lobby.builder().name(), "Weltenbaum");
}

/// The account pill names `/me`'s handle, and the gateway pill counts what
/// `/lobby/stats` said; before it answers, the tables the listing knows.
#[test]
fn the_pills_read_me_and_the_stats() {
    let mut app = headless();
    stocked(&mut app);
    {
        // No gateway address: nothing here may reach the network, whose
        // answers would land inside what is measured.
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
    }
    window(&mut app, 1920.0, 1080.0);
    app.update();
    assert!(labels(&mut app).iter().any(|l| l == "0 waiting"));
    {
        let mailbox = app.world().resource::<Mailbox>().clone();
        let epoch = app.world().resource::<LobbyState>().gateway_epoch;
        let mut box_ = mailbox.0.lock().expect("mailbox");
        box_.push(Reply::Remote(
            epoch,
            Box::new(Reply::Me(Me {
                handle: "AceVik#0007".to_string(),
                guest: false,
            })),
        ));
        box_.push(Reply::Remote(
            epoch,
            Box::new(Reply::Stats(LobbyStats {
                players_online: 12,
                tables_waiting: 3,
                games_running: 4,
            })),
        ));
    }
    app.update();
    app.update();
    let drawn = labels(&mut app);
    assert!(drawn.iter().any(|l| l == "AceVik#0007"), "{drawn:?}");
    assert!(
        drawn.iter().any(|l| l == "3 tables · 12 online"),
        "{drawn:?}"
    );
    // A second, equal answer changes nothing (no rebuild).
    let before = app.world().resource::<UiRebuilds>().total;
    {
        let mailbox = app.world().resource::<Mailbox>().clone();
        let epoch = app.world().resource::<LobbyState>().gateway_epoch;
        mailbox.0.lock().expect("mailbox").push(Reply::Remote(
            epoch,
            Box::new(Reply::Stats(LobbyStats {
                players_online: 12,
                tables_waiting: 3,
                games_running: 4,
            })),
        ));
    }
    app.update();
    app.update();
    let r = app.world().resource::<UiRebuilds>();
    assert_eq!(
        r.total, before,
        "state {} prefs {} cast {} frame {}",
        r.state, r.prefs, r.cast, r.frame
    );
}

/// The bell rings for somebody sitting down at the player's table: its count
/// on the header, and a toast.
#[test]
fn the_bell_rings_when_somebody_sits_down() {
    let mut app = seated();
    app.world_mut().resource_mut::<LobbyState>().settings = SettingsPane::Open;
    app.update();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let mut listing = GameListing {
            games: state.lobby.games().to_vec(),
            total: 1,
            ..GameListing::default()
        };
        listing.games[0].seats[3].taken = true;
        listing.games[0].seats[3].player = Some("Lina#0031".to_string());
        state.lobby.apply(LobbyEvent::Games(listing));
    }
    app.update();
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.bell.unread(), 2, "joined, and the table is full");
    let drawn = labels(&mut app);
    assert!(
        drawn.iter().any(|l| l == "2"),
        "the count on the bell: {drawn:?}"
    );
    let mut lanes = app
        .world_mut()
        .query_filtered::<(), With<super::super::header::ToastLane>>();
    assert_eq!(lanes.iter(app.world()).count(), 1, "a toast");
    // Opening the bell reads it.
    press(&mut app, Press::Header(HeaderPress::Bell));
    assert_eq!(app.world().resource::<LobbyState>().bell.unread(), 0);
    assert!(
        labels(&mut app)
            .iter()
            .any(|l| l == "Lina#0031 sat down at your table")
    );
}
