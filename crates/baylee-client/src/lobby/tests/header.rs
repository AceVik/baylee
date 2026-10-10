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
    seated_as(false)
}

/// The same, hosting it when `host`.
fn seated_as(host: bool) -> App {
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
                yours: host,
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

/// The gateway answers what the seating asked (the listing again), so the
/// lobby is not busy and its Leave is live.
fn answered(app: &mut App) {
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let games = state.lobby.games().to_vec();
        state.lobby.apply(LobbyEvent::Games(GameListing {
            total: games.len(),
            games,
            ..GameListing::default()
        }));
    }
    app.update();
    assert!(!app.world().resource::<LobbyState>().lobby.busy());
}

/// DESIGN-v5 §5: an empty chair offers AI to the host on every build, and
/// Language model only on a desktop build with its store open
/// (`tableseats::offer`; never wasm, Android or iOS, never a test): there
/// live, or off with its reason where `baylee-seat` is missing (the tests
/// below). Elsewhere it is absent, never drawn with a tag.
#[test]
fn an_empty_chair_offers_a_language_model_only_where_one_can_sit() {
    let mut app = seated_as(true);
    answered(&mut app);
    let drawn = presses(&mut app);
    assert!(drawn.contains(&Press::Shared(SharedPress::OpenMenu(ShellMenu::SeatAi(3)))));
    let llm = drawn
        .iter()
        .filter(|p| matches!(p, Press::Room(RoomPress::OpenChair(..))))
        .count();
    assert_eq!(
        crate::tableseats::offer(),
        crate::tableseats::Offer::Absent,
        "a test opens no store"
    );
    assert_eq!(llm, 0, "no Language model button where none can sit");
    assert!(
        !labels(&mut app).iter().any(|l| l.contains("desktop")),
        "and no tag saying why"
    );
}

/// The room as a desktop build offers it: `offer` stands for what
/// `tableseats::offer` would find (a test opens no store).
fn hosting_with(offer: crate::tableseats::Offer) -> App {
    let mut app = seated_as(true);
    answered(&mut app);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .llm
        .offer_in_test = Some(offer);
    app.update();
    app
}

fn opens_chair(p: &Press) -> bool {
    matches!(p, Press::Room(RoomPress::OpenChair(_, 3)))
}

/// Disabled controls carrying a press this predicate accepts.
fn off_presses(app: &mut App, pick: impl Fn(&Press) -> bool) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<&Press, With<crate::shellkit::controls::Disabled>>();
    query.iter(app.world()).filter(|p| pick(p)).count()
}

/// A desktop with its bridge offers the open chair a language model, live,
/// whether or not a profile is set up yet (the chair's sheet guides then).
#[test]
fn a_desktop_with_its_bridge_offers_a_language_model() {
    let mut app = hosting_with(crate::tableseats::Offer::Ready);
    assert_eq!(
        presses(&mut app).iter().filter(|p| opens_chair(p)).count(),
        1
    );
    assert_eq!(off_presses(&mut app, opens_chair), 0);
}

/// The beta.6 fault: a desktop whose install lacks `baylee-seat` drew no
/// Language model at all. It now draws it off, saying why.
#[test]
fn a_desktop_without_its_bridge_shows_the_language_model_off_with_why() {
    let mut app = hosting_with(crate::tableseats::Offer::Missing);
    assert_eq!(
        presses(&mut app).iter().filter(|p| opens_chair(p)).count(),
        0,
        "nothing to press"
    );
    assert_eq!(off_presses(&mut app, opens_chair), 1, "but drawn, off");
    let why = Phrase::RoomNoBridge.text(Lang::En);
    assert!(
        labels(&mut app).iter().any(|l| l == why),
        "its reason is drawn"
    );
}

/// A chair's sheet with no profile in the settings file explains profiles
/// and keys, and its Set up leads to Settings › Language models.
#[test]
fn a_chair_without_a_profile_leads_to_the_language_model_settings() {
    let mut app = hosting_with(crate::tableseats::Offer::Ready);
    // Opened as `OpenChair` opens it, without reading this machine's file.
    app.world_mut().resource_mut::<LobbyState>().chair_sheet = Some(3);
    app.update();
    let guide = Phrase::RoomNoModel.text(Lang::En);
    assert!(labels(&mut app).iter().any(|l| l == guide), "the guide");
    tap_control(&mut app, "Set up a model", |p| {
        *p == Press::Room(RoomPress::SetUpModel)
    });
    let state = app.world().resource::<LobbyState>();
    assert!(state.settings_open());
    assert_eq!(
        state.settings_section(),
        baylee_client_core::settings_map::Section::LanguageModels
    );
    assert_eq!(state.chair_sheet, None);
}

/// DESIGN-v5 §5 "Leaving": the host's Leave hands the table on, so it is
/// asked first and sends nothing until the answer; a guest's leaves at once.
#[test]
fn the_hosts_leave_is_asked_first() {
    let mut app = seated_as(true);
    answered(&mut app);
    press(&mut app, Press::Room(RoomPress::LeaveTable(0)));
    {
        let state = app.world().resource::<LobbyState>();
        assert!(
            matches!(
                state.confirmation,
                Some(super::super::confirm::Destructive::LeaveHosting(ref id)) if id == "pod"
            ),
            "asked"
        );
        assert!(!state.lobby.busy(), "nothing sent before the answer");
    }
    assert!(
        labels(&mut app)
            .iter()
            .any(|l| l == "Leave \u{201c}Thursday pod\u{201d}?"),
        "the question names the table"
    );
    press(&mut app, Press::Shared(SharedPress::ConfirmDestructive));
    let state = app.world().resource::<LobbyState>();
    assert!(state.confirmation.is_none());
    assert!(state.lobby.busy(), "the leave went out");

    let mut app = seated();
    answered(&mut app);
    press(&mut app, Press::Room(RoomPress::LeaveTable(0)));
    let state = app.world().resource::<LobbyState>();
    assert!(state.confirmation.is_none(), "a guest is not asked");
    assert!(state.lobby.busy());
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

/// DESIGN-v5 §4, §17 WP2: while my game is being played, Play's hero leads
/// with the gold Return and its two starts stand disabled with the reason;
/// on a Phone the header's pill is the way back and the hero has no Return.
#[test]
fn while_my_game_runs_the_hero_returns_and_its_starts_wait() {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::Games(GameListing {
            games: vec![GameSummary {
                id: "pod".to_string(),
                name: "Thursday pod".to_string(),
                host: Some("me#0007".to_string()),
                yours: true,
                state: "playing".to_string(),
                seats: vec![
                    GameSeat {
                        seat: 0,
                        taken: true,
                        player: Some("me#0007".to_string()),
                        you: true,
                        ..GameSeat::default()
                    },
                    GameSeat {
                        seat: 1,
                        taken: true,
                        player: Some("Ole#0009".to_string()),
                        ..GameSeat::default()
                    },
                ],
                ..GameSummary::default()
            }],
            total: 1,
            ..GameListing::default()
        }));
    }
    window(&mut app, 1400.0, 900.0);
    for _ in 0..3 {
        app.update();
    }
    let off = |app: &mut App, wanted: Press| {
        let mut query = app
            .world_mut()
            .query_filtered::<&Press, With<crate::shellkit::controls::Disabled>>();
        query.iter(app.world()).any(|p| *p == wanted)
    };
    // The hero's Return is its stop "return"; a table row has its own.
    let hero_return = |app: &mut App| {
        let mut query = app
            .world_mut()
            .query::<(&Press, &crate::shellkit::focus::Stop)>();
        query
            .iter(app.world())
            .filter(|(p, s)| **p == Press::Play(PlayPress::Return) && s.id == "return")
            .count()
    };
    assert_eq!(hero_return(&mut app), 1, "gold Return");
    assert!(
        off(&mut app, Press::Play(PlayPress::PlayHouse)),
        "Play the house waits"
    );
    assert!(
        off(&mut app, Press::Play(PlayPress::CreateTable)),
        "Create table waits"
    );
    window(&mut app, 844.0, 390.0);
    app.update();
    app.update();
    assert_eq!(hero_return(&mut app), 0, "no hero Return");
    assert_eq!(
        count(&mut app, Press::Header(HeaderPress::Return)),
        1,
        "the header's pill"
    );
    assert!(off(&mut app, Press::Play(PlayPress::PlayHouse)));
}

/// DESIGN-v5 §5 (S4-2): the Create-table sheet's Players is a segmented
/// control, and a stepper on a Phone.
#[test]
fn players_is_a_stepper_on_a_phone() {
    for (width, height, stepper) in [(1400.0, 900.0, false), (844.0, 390.0, true)] {
        let mut app = headless();
        stocked(&mut app);
        window(&mut app, width, height);
        for _ in 0..3 {
            app.update();
        }
        press(&mut app, Press::Play(PlayPress::CreateTable));
        app.update();
        let drawn = presses(&mut app);
        assert_eq!(
            drawn.contains(&Press::Play(PlayPress::StepPlayers(true))),
            stepper,
            "{width}: stepper"
        );
        assert_eq!(
            drawn.contains(&Press::Play(PlayPress::Players(3))),
            !stepper,
            "{width}: segments"
        );
    }
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

/// `KEYBOARD.md` W2 steps 3-6: in the Create-table sheet the arrows choose
/// along a radio group (players, template, clock), and Enter on a choice is
/// Open table.
#[test]
fn the_sheets_choices_are_chosen_by_the_arrows_and_enter_opens() {
    let mut app = headless();
    stocked(&mut app);
    window(&mut app, 1400.0, 900.0);
    for _ in 0..3 {
        app.update();
    }
    press(&mut app, Press::Play(PlayPress::CreateTable));
    app.update();
    let key = |app: &mut App, code: KeyCode, logical: Key| {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(code);
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(pressed(code, logical));
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(code);
        app.update();
        app.update();
    };
    let draft = |app: &App| {
        app.world()
            .resource::<LobbyState>()
            .play
            .sheet
            .as_ref()
            .map(|(d, _)| d.clone())
    };
    let stop = |app: &mut App| {
        let focused = app
            .world()
            .resource::<bevy::input_focus::InputFocus>()
            .get();
        focused.and_then(|e| {
            app.world()
                .get::<crate::shellkit::focus::Stop>(e)
                .map(|s| (s.id, s.item))
        })
    };
    key(&mut app, KeyCode::Tab, Key::Tab);
    assert_eq!(stop(&mut app).map(|s| s.0), Some("players"));
    key(&mut app, KeyCode::Tab, Key::Tab);
    assert_eq!(
        stop(&mut app).map(|s| s.0),
        Some("templates"),
        "{:?}",
        stop(&mut app)
    );
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(KeyCode::ArrowRight, Key::ArrowRight));
    app.update();
    assert_eq!(stop(&mut app), Some(("templates", 1)));
    assert_eq!(
        draft(&app).map(|d| d.template),
        Some(baylee_client_core::lobby::play::Template::Duel),
        "the arrow chose"
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(KeyCode::Enter, Key::Enter));
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert!(state.play.sheet.is_none(), "Enter opened the table");
    assert!(state.lobby.busy(), "the table was asked for");
}

/// The header's brand is the logo, an image named "Baylee" for assistive
/// technology, and no text node says the brand any more.
#[test]
fn the_header_wears_the_logo_and_no_wordmark() {
    let mut app = seated();
    let mut marks = app
        .world_mut()
        .query_filtered::<(&ImageNode, &bevy::a11y::AccessibilityNode), With<crate::shellkit::header::BrandMark>>();
    let found: Vec<_> = marks
        .iter(app.world())
        .map(|(_, a11y)| a11y.label().map(str::to_string))
        .collect();
    assert_eq!(found, vec![Some("Baylee".to_string())]);
    let mut texts = app.world_mut().query::<&Text>();
    assert!(
        texts.iter(app.world()).all(|t| t.0 != "Baylee"),
        "a text node still says the brand"
    );
}

/// The logo's cut is the smallest at least as tall as it is drawn, and the
/// largest past that: never stretched up from a small one.
#[test]
fn the_logo_picks_the_cut_its_height_asks_for() {
    use crate::shellkit::header::cut_for;
    assert_eq!(cut_for(36.0), 0);
    assert_eq!(cut_for(48.0), 0);
    assert_eq!(cut_for(49.0), 1);
    assert_eq!(cut_for(90.0), 1);
    assert_eq!(cut_for(150.0), 2);
    assert_eq!(cut_for(400.0), 2);
}
