//! Regression coverage for retained public standings and validated result notifications.
use super::{headless, presses, settle, sized};
use crate::shellkit::focus::Stop;
use crate::{
    lobby::ranked_profile::OpenProfile,
    lobby::{Hub, LobbyState, Press, ranked::Action},
};
use baylee_account::ranked as api;
use bevy::prelude::*;

fn rank(league: api::League) -> api::Rank {
    api::Rank {
        league,
        division: None,
        champion_place: None,
    }
}

fn standings() -> api::Response {
    let entries = (1..=30u8)
        .map(|n| api::Row {
            account: [n; 16],
            name: Some(format!("Player {n:02}")),
            tag: Some(u32::from(n)),
            standing: api::Standing {
                format: api::Format::Standard,
                track: api::Track::Player,
                deck: None,
                elo: 1370 + i32::from(n),
                games: 5,
                wins: 3,
                losses: 2,
                draws: 0,
                placement_games: 5,
                career: api::Record {
                    games: 5,
                    wins: 3,
                    losses: 2,
                    draws: 0,
                },
                rank: rank(api::League::Unranked),
            },
            ai: None,
        })
        .collect();
    api::Response::Board {
        season: api::Season {
            id: [7; 16],
            name: "Solstice".into(),
            starts: 1,
            ends: None,
        },
        entries,
        next: None,
    }
}

#[test]
fn public_rows_keep_entities_and_scroll_region_on_a_same_page_refresh() {
    let mut app = headless();
    sized(&mut app, 1440.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.hub = Hub::Ranked;
        state.ranked_net.board = Some(standings());
    }
    settle(&mut app);
    let controls = presses(&mut app);
    assert!(controls.contains(&Press::Ranked(Action::Back)));
    assert!(!controls.contains(&Press::Ranked(Action::Join)));
    let row_entities = |app: &mut App| {
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<OpenProfile>>();
        let mut rows: Vec<_> = query.iter(app.world()).collect();
        rows.sort();
        rows
    };
    let before = row_entities(&mut app);
    assert_eq!(before.len(), 30);
    let list = |app: &mut App| {
        let mut query = app
            .world_mut()
            .query::<(Entity, &crate::lobby::Scrollable)>();
        query
            .iter(app.world())
            .find(|(_, s)| s.0 == crate::lobby::List::Ranked)
            .unwrap()
            .0
    };
    let scroller = list(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let work = &mut state.bypass_change_detection().ranked_net;
        work.board_busy = true;
        work.revision += 1;
    }
    app.update();
    assert_eq!(row_entities(&mut app), before);
    assert_eq!(list(&mut app), scroller);
    // The home lobby still receives feed/status changes during ranked play.
    // Its shell may rebuild; the actual board, focusable rows and bar stay put.
    app.world_mut().resource_mut::<LobbyState>().set_changed();
    app.update();
    assert_eq!(row_entities(&mut app), before);
    assert_eq!(list(&mut app), scroller);
}

/// The Play screen's ranked tile unfolds nothing in the lobby: it opens the
/// Ranked page, where the desk stands beside the public ladder, and Back
/// returns to Play.
#[test]
fn the_ranked_tile_opens_a_page_with_the_desk_beside_the_public_ladder() {
    use super::{press_target, stocked, tap_control};
    use crate::lobby::{orders, play::PlayPress};

    let mut app = headless();
    sized(&mut app, 1440.0);
    stocked(&mut app);
    settle(&mut app);
    let open = Press::Ranked(Action::Open);
    let create = Press::Play(PlayPress::CreateTable);
    let tile = press_target(&mut app, open);
    assert_eq!(
        app.world().get::<Stop>(tile),
        Some(&Stop::new(orders::PLAY.name, "ranked"))
    );
    let lobby = presses(&mut app);
    assert!(lobby.contains(&create));
    assert!(
        !lobby.iter().any(|p| matches!(
            p,
            Press::Ranked(Action::Format(_) | Action::Join | Action::ToggleDecks)
        )),
        "the queue stands on the Ranked page, not in the tile: {lobby:?}"
    );
    let mut artwork = app.world_mut().query::<&crate::challenge_art::Scene>();
    assert!(
        artwork
            .iter(app.world())
            .any(|scene| matches!(scene, crate::challenge_art::Scene::Ranked))
    );
    tap_control(&mut app, "open the Ranked page", |p| *p == open);
    assert_eq!(app.world().resource::<LobbyState>().hub, Hub::Ranked);
    let actions = presses(&mut app);
    // A gateway-only account reads the ladder; the search needs Central.
    assert!(!actions.contains(&Press::Ranked(Action::Join)));
    assert!(actions.contains(&Press::Ranked(Action::Refresh)));
    tap_control(&mut app, "return to Play", |p| {
        *p == Press::Ranked(Action::Back)
    });
    assert_eq!(app.world().resource::<LobbyState>().hub, Hub::Play);
    let tile = press_target(&mut app, open);
    assert_eq!(
        app.world().get::<Stop>(tile).map(|stop| stop.id),
        Some("ranked")
    );
}

#[test]
fn ranked_desk_scrolls_and_wears_an_only_when_needed_scrollbar() {
    use crate::lobby::{List, Scrollable, Scrolled, scrollbars::OnlyWhenNeeded};
    use crate::shellkit::surfaces::SheetScroller;
    use bevy::input::mouse::MouseScrollUnit;
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::ui_widgets::Scrollbar;

    let mut app = headless();
    sized(&mut app, 1280.0);
    app.world_mut().resource_mut::<LobbyState>().hub = Hub::Ranked;
    settle(&mut app);

    let (desk_scroller, node, has_sheet_scroller, has_scroll_pos) = app
        .world_mut()
        .query::<(
            Entity,
            &Scrollable,
            &Node,
            Has<SheetScroller>,
            Has<ScrollPosition>,
        )>()
        .iter(app.world())
        .find(|(_, s, ..)| s.0 == List::RankedDesk)
        .map(|(e, _, n, sheet, pos)| (e, n.clone(), sheet, pos))
        .expect("wide Ranked desk owns a Scrollable(List::RankedDesk) viewport");
    assert_eq!(node.overflow.y, OverflowAxis::Scroll);
    assert!(
        has_sheet_scroller,
        "desk viewport accepts sheet scroll events"
    );
    assert!(has_scroll_pos, "desk viewport carries ScrollPosition");

    let (track_entity, is_only_when_needed) = app
        .world_mut()
        .query::<(Entity, &Scrollbar, Has<OnlyWhenNeeded>)>()
        .iter(app.world())
        .find(|(_, bar, _)| bar.target == desk_scroller)
        .map(|(e, _, own)| (e, own))
        .expect("Ranked desk has a scrollbar targeting its scroller");
    assert!(is_only_when_needed);

    // Simulate 1280x720 overflow (506 px viewport, 720 px content): scrollbar
    // appears and mouse-wheel scrolling moves the desk and persists in Scrolled.
    app.world_mut()
        .entity_mut(desk_scroller)
        .insert(ComputedNode {
            size: Vec2::new(360.0, 506.0),
            content_size: Vec2::new(360.0, 720.0),
            inverse_scale_factor: 1.0,
            ..default()
        });
    app.update();
    assert_eq!(
        app.world().get::<Node>(track_entity).unwrap().display,
        Display::Flex,
        "overflowing Ranked desk reveals its scrollbar"
    );

    let pointer = bevy::picking::events::Pointer::new(
        PointerId::Mouse,
        Location {
            target: bevy::camera::NormalizedRenderTarget::Window(
                bevy::window::WindowRef::Primary
                    .normalize(Some(Entity::PLACEHOLDER))
                    .expect("window ref"),
            ),
            position: Vec2::new(220.0, 400.0),
        },
    );
    app.world_mut().write_message(PointerScroll {
        entity: desk_scroller,
        pointer,
        unit: MouseScrollUnit::Line,
        x: 0.0,
        y: -5.0,
        hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        phase: bevy::input::touch::TouchPhase::Moved,
    });
    app.update();
    let offset = app.world().get::<ScrollPosition>(desk_scroller).unwrap().y;
    assert!(
        offset > 0.0,
        "wheel scroll moves the Ranked desk ({offset})"
    );
    assert!(
        (app.world().resource::<Scrolled>().get(List::RankedDesk) - offset).abs() < 0.01,
        "Ranked desk scroll offset persists across rebuilds"
    );
}
