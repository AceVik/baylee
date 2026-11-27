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
