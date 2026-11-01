//! A table at rest is not written by the crate root's own frame systems.
//!
//! Each runs every frame, and a `&mut duel` through the `ResMut` marks the
//! whole duel changed, which keeps every `duel.is_changed()`-guarded system
//! (the players' strip, the dial's scale, ...) running at rest. Found with
//! dev-control's `/changed`.

use super::*;
use baylee_client_core::test_support::ViewBuilder;

/// A duel with a view and a board, nothing pending of this client's own.
fn resting() -> Duel {
    let mut duel = Duel::default();
    duel.receive_view(ViewBuilder::new(2).build());
    rebuild_board(&mut duel);
    duel
}

/// Runs `system` for two frames and answers whether the second wrote the
/// duel.
fn writes_at_rest<M>(
    system: impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
) -> bool {
    let mut app = App::new();
    app.insert_resource(resting())
        .init_resource::<prefs::Prefs>()
        .add_systems(Update, system);
    app.update();
    let tick = app.world().read_change_tick();
    app.update();
    app.world()
        .resource_ref::<Duel>()
        .last_changed()
        .is_newer_than(tick, app.world().read_change_tick())
}

/// Red while it borrowed the duel mutably to ask whether a run was under way.
#[test]
fn no_mana_run_no_write() {
    assert!(!writes_at_rest(run_mana_plan));
}

/// Red while it borrowed the duel mutably to ask whether a way was chosen.
#[test]
fn no_chosen_cast_mode_no_write() {
    assert!(!writes_at_rest(answer_the_chosen_cast_mode));
}

/// Red while it borrowed the duel mutably to ask the yes-batch (idle) for an
/// answer every frame.
#[test]
fn the_autopilot_with_nothing_to_answer_does_not_write() {
    assert!(!writes_at_rest(run_autopilot));
}

/// Red while `poll_host` mutably dereferenced `InstalledHost` to poll it and
/// passed `&mut duel` through `ResMut<Duel>` into `follow_the_replay`,
/// marking both `InstalledHost` and `Duel` changed on 100% of frames at rest.
#[test]
fn an_idle_host_does_not_write_the_duel_or_the_host() {
    struct IdleHost;
    impl DuelHost for IdleHost {
        fn poll(&mut self) -> Vec<HostMessage> {
            Vec::new()
        }
        fn submit(&mut self, _: PlayerAction) {}
        fn ready(&mut self) {}
        fn seat(&self) -> baylee_core::ids::PlayerId {
            baylee_core::ids::PlayerId::new(0)
        }
    }

    let mut app = App::new();
    let mut images = Assets::<Image>::default();
    let card_textures = textures::CardTextures::new(&mut images, textures::default_budget_bytes());
    app.add_plugins(bevy::state::app::StatesPlugin)
        .insert_state(DuelPhase::Playing)
        .insert_resource(resting())
        .insert_resource(InstalledHost(Box::new(IdleHost)))
        .insert_resource(card_textures)
        .add_message::<DuelReport>()
        .add_systems(Update, poll_host);
    app.update();
    let tick = app.world().read_change_tick();
    app.update();
    let now = app.world().read_change_tick();
    assert!(
        !app.world()
            .resource_ref::<Duel>()
            .last_changed()
            .is_newer_than(tick, now),
        "an idle host must not mark Duel changed on every frame"
    );
    assert!(
        !app.world()
            .resource_ref::<InstalledHost>()
            .last_changed()
            .is_newer_than(tick, now),
        "polling an idle host must not mark InstalledHost changed on every frame"
    );
}

/// Red while `keep_the_table_connected` called `retry.schedule.settle()` on
/// every connected frame without checking `is_settled()`, marking `Reconnect`
/// changed on 100% of frames.
#[test]
fn a_connected_table_does_not_write_reconnect() {
    struct LocalHost;
    impl DuelHost for LocalHost {
        fn poll(&mut self) -> Vec<HostMessage> {
            Vec::new()
        }
        fn submit(&mut self, _: PlayerAction) {}
        fn ready(&mut self) {}
        fn seat(&self) -> baylee_core::ids::PlayerId {
            baylee_core::ids::PlayerId::new(0)
        }
    }

    let mut app = App::new();
    app.insert_resource(resting())
        .insert_resource(InstalledHost(Box::new(LocalHost)))
        .init_resource::<Reconnect>()
        .init_resource::<Time>()
        .add_message::<DuelReport>()
        .add_systems(Update, keep_the_table_connected);
    app.update();
    let tick = app.world().read_change_tick();
    app.update();
    let now = app.world().read_change_tick();
    assert!(
        !app.world()
            .resource_ref::<Reconnect>()
            .last_changed()
            .is_newer_than(tick, now),
        "a settled local/connected table must not mark Reconnect changed on every frame"
    );
}

/// Red while `watch_for_arrivals` mutably dereferenced `Sheen` and `ZoneWatch`
/// on every frame even when no sweep started, finished, or arrived.
#[test]
fn a_resting_board_does_not_write_sheen_or_zone_watch() {
    let mut app = App::new();
    app.insert_resource(resting())
        .init_resource::<sheen::Sheen>()
        .init_resource::<table::ZoneWatch>()
        .init_resource::<Time>()
        .add_systems(Update, sheen::watch_for_arrivals);
    app.update();
    let tick = app.world().read_change_tick();
    app.update();
    let now = app.world().read_change_tick();
    assert!(
        !app.world()
            .resource_ref::<sheen::Sheen>()
            .last_changed()
            .is_newer_than(tick, now),
        "a resting board with no active sweeps must not mark Sheen changed on every frame"
    );
    assert!(
        !app.world()
            .resource_ref::<table::ZoneWatch>()
            .last_changed()
            .is_newer_than(tick, now),
        "an already-observed view must not mark ZoneWatch changed on every frame"
    );
}

/// Red if `track_proposals` marks `Duel` changed when proposals and
/// activatable permanents match what the board already holds.
#[test]
fn track_proposals_does_not_write_the_duel_at_rest() {
    assert!(!writes_at_rest(table::track_proposals));
}

/// The settings over a table (`lobby::table_settings`), put away or
/// standing: asking whether the menu asked for them, or whether `Esc` puts
/// them away, writes neither the duel nor the lobby, whose change is a
/// rebuilt settings screen.
#[test]
fn the_settings_over_a_table_write_nothing_at_rest() {
    for standing in [false, true] {
        let mut lobby = LobbyState::new();
        if standing {
            lobby::table_settings::stand_over_table(&mut lobby);
        }
        let mut app = App::new();
        app.insert_resource(resting())
            .insert_resource(lobby)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(
                Update,
                (
                    lobby::table_settings::open_asked,
                    lobby::table_settings::close_on_escape,
                ),
            );
        app.update();
        let tick = app.world().read_change_tick();
        app.update();
        let now = app.world().read_change_tick();
        assert!(
            !app.world()
                .resource_ref::<Duel>()
                .last_changed()
                .is_newer_than(tick, now),
            "standing {standing}: the duel was written at rest"
        );
        assert!(
            !app.world()
                .resource_ref::<LobbyState>()
                .last_changed()
                .is_newer_than(tick, now),
            "standing {standing}: the lobby was written at rest"
        );
    }
}
