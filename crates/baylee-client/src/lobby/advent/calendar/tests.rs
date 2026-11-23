use super::{Book, Screen, api};
use api::{Door, DoorState, Request, Response};

#[test]
fn advent_puzzle_screen_contains_the_players_real_life_moves_and_hint_numbers_in_both_languages() {
    use crate::{hud::UiFonts, lobby::advent::puzzle, shellkit::controls::Kit};
    use api::puzzle::{Goal, Phase, Puzzle, Step, Unit};
    use bevy::{ecs::system::RunSystemOnce, prelude::*};
    let unit = Unit {
        power: 2,
        toughness: 2,
        keywords: vec![],
        tapped: false,
    };
    let encounter = Puzzle {
        goal: Goal::Defeat,
        life: [17, 2],
        mana: 3,
        units: [vec![unit], vec![]],
        hand: vec![],
        steps: vec![Step {
            new_turn: None,
            draw: None,
            phase: Phase::Attack { guards: [None; 3] },
        }],
    };
    for german in [true, false] {
        let mut app = App::new();
        let mut book = Book::default();
        book.accept(
            Request::CurrentCalendar {},
            Ok(canonical(OPEN, DoorState::Open)),
        );
        book.screen = Screen::Door(1);
        book.accept(
            Request::Puzzle {
                year: 2026,
                door: 1,
            },
            Ok(Response::Puzzle {
                year: 2026,
                door: 1,
                puzzle: encounter.clone(),
            }),
        );
        app.insert_resource(book);
        app.world_mut()
            .run_system_once(move |mut cmd: Commands, book: Res<Book>| {
                let root = cmd.spawn(Node::default()).id();
                let fonts = UiFonts::default();
                let kit = Kit {
                    fonts: &fonts,
                    m: crate::lobby::header::kit_metrics(1100., 720., default(), default()),
                    german,
                };
                puzzle::draw(&mut cmd, root, kit, &book, 1);
            })
            .unwrap();
        let world = app.world_mut();
        let texts = world
            .query::<&Text>()
            .iter(world)
            .map(|t| &*t.0)
            .collect::<Vec<_>>();
        let expected = if german {
            [
                "Tür 1",
                "Du · 17 Leben",
                "Angriff mit A",
                "Deine Entscheidung · 3 Mana",
                "Kostenloser Hinweis 1",
            ]
        } else {
            [
                "Door 1",
                "You · 17 life",
                "Attack with A",
                "Your decision · 3 mana",
                "Free hint 1",
            ]
        };
        for line in expected {
            assert!(texts.contains(&line), "{line:?} absent from {texts:?}");
        }
        assert!(
            texts.iter().all(|s| !s.contains('{') && !s.contains('}')),
            "{texts:?}"
        );
    }
}

#[test]
fn physical_clicks_open_the_full_width_plaque_and_its_shell_controls() {
    use crate::{
        hud::UiFonts,
        lobby::{
            advent::{Action, draw, seasonal},
            deck_appearance::EquipReward,
            header::kit_metrics,
        },
        shellkit::{controls::Kit, focus::Activated},
        ui_button::testing,
    };
    use bevy::{ecs::system::RunSystemOnce, prelude::*};
    let mut app = testing::app();
    let mut book = Book::default();
    book.accept(
        Request::CurrentCalendar {},
        Ok(canonical(OPEN, DoorState::Open)),
    );
    app.insert_resource(book)
        .add_message::<Activated>()
        .add_message::<EquipReward>()
        .add_systems(Update, draw::actions);
    let (plaque, gallery) = app
        .world_mut()
        .run_system_once(|mut commands: Commands| {
            let fonts = UiFonts::default();
            let kit = Kit {
                fonts: &fonts,
                m: kit_metrics(1100., 720., default(), default()),
                german: false,
            };
            (
                seasonal::plaque(&mut commands, kit),
                draw::button(&mut commands, kit, "Gallery", Action::Gallery, "gallery", 0),
            )
        })
        .unwrap();
    app.update();
    let fixed = app.world().get::<Node>(plaque).unwrap().clone();
    assert_eq!(fixed.width, percent(100.));
    testing::click(&mut app, plaque);
    assert_eq!(app.world().resource::<Book>().screen, Screen::Calendar);
    testing::click(&mut app, gallery);
    assert_eq!(app.world().resource::<Book>().screen, Screen::Gallery);
    assert_eq!(app.world().get::<Node>(plaque).unwrap(), &fixed);
}

const DAY: i64 = 86400;
// 1 December 2026 00:00 UTC and the server-selected close, 7 January midnight.
const OPEN: i64 = 1_796_083_200;
const CLOSE: i64 = OPEN + 37 * DAY;
fn canonical(now: i64, state: DoorState) -> Response {
    Response::Calendar {
        year: 2026,
        starts_at: OPEN,
        closes_at: CLOSE,
        now,
        hero: None,
        doors: (1..=24)
            .map(|door| Door {
                door,
                state: if door == 1 { state } else { DoorState::Locked },
                enables_at: OPEN + i64::from(door - 1) * DAY,
            })
            .collect(),
    }
}
#[test]
fn fresh_clients_use_centrals_year_through_new_year_and_close_at_the_authoritative_boundary() {
    for (now, eligible) in [
        (OPEN - 1, false),
        (OPEN, true),
        (OPEN + 30 * DAY, true),
        (OPEN + 31 * DAY, true),
        (CLOSE - 1, true),
        (CLOSE, false),
    ] {
        let mut book = Book::default();
        book.accept(
            Request::CurrentCalendar {},
            Ok(canonical(now, DoorState::Open)),
        );
        assert_eq!(book.year(), 2026);
        assert_eq!(book.released(), eligible);
        book.screen = Screen::Calendar;
        book.retry();
        assert!(
            book.work
                .iter()
                .any(|r| matches!(r, Request::CurrentCalendar {}))
        );
    }
    let mut missing = Book::default();
    missing.accept(Request::CurrentCalendar {}, Ok(Response::NoCalendar {}));
    assert!(!missing.released());
}
#[test]
fn an_unrevealed_solved_door_remains_collectible_and_drives_the_attention_state() {
    let mut book = Book::default();
    book.accept(
        Request::CurrentCalendar {},
        Ok(canonical(OPEN + DAY, DoorState::Solved)),
    );
    assert!(book.unsolved());
    assert!(book.work.iter().any(|r| matches!(
        r,
        Request::Claim {
            year: 2026,
            door: 1
        }
    )));
    let mut late = Book::default();
    late.accept(
        Request::Calendar { year: 2026 },
        Ok(canonical(CLOSE, DoorState::Solved)),
    );
    assert!(!late.released());
    assert!(late.work.iter().any(|r| matches!(
        r,
        Request::Claim {
            year: 2026,
            door: 1
        }
    )));
}

fn reward(id: u8) -> api::Reward {
    api::Reward {
        id: [id; 16],
        kind: api::Kind::Sleeve,
        title: api::Text {
            de: "Test".into(),
            en: "Test".into(),
        },
        asset: api::AssetMeta {
            mime: api::Mime::Webp,
            tier: api::Finish::Standard,
            digest: [id; 32],
            size: 16,
            width: 2,
            height: 2,
        },
        year: 2026,
        door: id,
        earned_at: OPEN + i64::from(id) * DAY,
    }
}
#[test]
fn pagination_keeps_the_published_gallery_intact_until_the_final_page() {
    let mut book = Book::default();
    let one = reward(1);
    let two = reward(2);
    book.accept(
        Request::Inventory { after: None },
        Ok(Response::Inventory {
            items: vec![one.clone()],
            next: None,
        }),
    );
    let unchanged = book.revision;
    book.accept(
        Request::Inventory { after: None },
        Ok(Response::Inventory {
            items: vec![two.clone()],
            next: Some(two.id),
        }),
    );
    assert_eq!(book.inventory, std::slice::from_ref(&one));
    assert_eq!(book.revision, unchanged);
    book.accept(
        Request::Inventory {
            after: Some(two.id),
        },
        Err(baylee_account::Error::Unavailable),
    );
    assert!(book.inventory_retry.is_some());
    assert_eq!(book.inventory, std::slice::from_ref(&one));
    book.accept(
        Request::Inventory { after: None },
        Ok(Response::Inventory {
            items: vec![two.clone()],
            next: Some(two.id),
        }),
    );
    book.accept(
        Request::Inventory {
            after: Some(two.id),
        },
        Ok(Response::Inventory {
            items: vec![one.clone()],
            next: None,
        }),
    );
    assert_eq!(book.inventory, vec![two, one]);
    assert!(book.inventory_retry.is_none());
}
#[test]
fn repeat_calendar_refresh_and_late_previous_year_submit_do_not_repaint_a_door() {
    let mut book = Book::default();
    book.accept(
        Request::CurrentCalendar {},
        Ok(canonical(OPEN, DoorState::Open)),
    );
    book.screen = Screen::Door(1);
    let revision = book.revision;
    book.accept(
        Request::CurrentCalendar {},
        Ok(canonical(OPEN + 2, DoorState::Open)),
    );
    assert_eq!(book.revision, revision);
    book.accept(
        Request::Submit {
            year: 2025,
            door: 1,
            line: api::puzzle::Line(Vec::new()),
        },
        Ok(Response::Submission { won: true }),
    );
    assert_eq!(book.revision, revision);
    assert_eq!(book.door(1).unwrap().state, DoorState::Open);
    book.accept(
        Request::Submit {
            year: 2026,
            door: 1,
            line: api::puzzle::Line(Vec::new()),
        },
        Ok(Response::Submission { won: true }),
    );
    assert_eq!(book.screen, Screen::Door(1));
    assert_eq!(book.door(1).unwrap().state, DoorState::Claimed);
    assert!(!book.unsolved());
    let revision = book.revision;
    book.accept(
        Request::CurrentCalendar {},
        Ok(canonical(OPEN + 5, DoorState::Claimed)),
    );
    assert_eq!(book.revision, revision);
}
