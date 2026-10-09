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
