//! The table tour over a real game hosted in this process: the opening
//! hands decided, then kept (09.10.: the tour ran its casting chapter over
//! the mulligan, and T8's Next could never come alive).

use baylee_client_core::tour::{Mode, Run, Tour};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::PlayerAction;
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;

use crate::{Duel, DuelPhase, DuelReport, InstalledHost, textures};

/// A house game in this process at its opening hands, the table tour's
/// system running beside the message loop.
fn table() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .add_plugins(bevy::state::app::StatesPlugin)
        .init_asset::<Image>();
    let textures = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        textures::CardTextures::new(&mut images, 1 << 20)
    };
    let host = crate::host::LocalHost::new(
        &crate::host::tests::duel_preset(),
        PlayerId::new(0),
        &["You", "AI"],
    )
    .expect("host");
    app.insert_resource(textures)
        .init_resource::<Duel>()
        .init_state::<DuelPhase>()
        .add_message::<DuelReport>()
        .add_message::<Pointer<Click>>()
        .init_resource::<super::TourDesk>()
        .insert_resource(crate::settings::ClientSettings::default())
        .insert_resource(InstalledHost(Box::new(host)))
        .add_systems(
            Update,
            (crate::poll_host, crate::flush_outbox, super::table::tours).chain(),
        );
    app.world_mut()
        .resource_mut::<NextState<DuelPhase>>()
        .set(DuelPhase::Opening);
    for _ in 0..4 {
        app.update();
    }
    app
}

fn deciding(app: &App) -> bool {
    app.world()
        .resource::<Duel>()
        .view
        .as_ref()
        .is_some_and(|v| !v.deciding.is_empty())
}

fn desk(app: &App) -> &super::TourDesk {
    app.world().resource::<super::TourDesk>()
}

/// Keeps this seat's hand and runs frames until no seat decides any more.
fn keep(app: &mut App) {
    app.world_mut()
        .resource_mut::<Duel>()
        .submit(PlayerAction::MulliganKeep);
    for _ in 0..40 {
        app.update();
        if !deciding(app) {
            break;
        }
    }
    assert!(!deciding(app), "the opening hands were never all kept");
}

/// The practice game's tour does not start over the opening hands; it
/// starts at its first step once they are kept.
#[test]
fn the_table_tour_starts_after_the_keep() {
    let mut app = table();
    assert!(deciding(&app), "the game opens at its opening hands");
    app.world_mut().resource_mut::<super::TourDesk>().practice = true;
    for _ in 0..4 {
        app.update();
    }
    assert!(
        desk(&app).run.is_none(),
        "the tour started over the mulligan"
    );
    keep(&mut app);
    app.update();
    let run = desk(&app)
        .run
        .as_ref()
        .expect("the tour started after the keep");
    assert_eq!(run.tour, Tour::Table);
    assert_eq!(run.current().id, "T1");
    assert!(!desk(&app).parked);
}

/// T8 asks for a cast: standing over the opening hands it is set aside
/// (drawn nowhere, holding no key), and it comes back once they are kept.
#[test]
fn a_cast_try_it_parks_over_the_opening_hands_and_comes_back() {
    let mut app = table();
    let mut run = Run::chapter(Tour::Table, 0, false, 2).expect("the table tour");
    let mut tours = baylee_client_core::tour::Tours::default();
    while run.current().id != "T8" {
        run.next(&mut tours);
    }
    app.world_mut().resource_mut::<super::TourDesk>().run = Some(run);
    app.update();
    assert!(desk(&app).parked, "T8 stood over the mulligan");
    assert!(desk(&app).shown().is_none());
    assert!(!desk(&app).holds_keyboard());
    keep(&mut app);
    app.update();
    let desk = desk(&app);
    assert!(!desk.parked, "T8 did not come back after the keep");
    let run = desk.shown().expect("T8 is drawn again");
    assert_eq!(run.current().id, "T8");
    assert_eq!(run.mode, Mode::Try);
}
