//! One UI coordinate space, with a minimum desktop canvas and a 4K table step.
//!
//! Below 1280 × 720 desktop layouts keep that minimum area and scale down
//! uniformly. OS DPI stays untouched; event coordinates are converted at
//! UI boundaries. Web/mobile retain their existing responsive layouts.
//!
//! Past 1920 × 1080 the shell draws a step larger (`shellkit::metrics::
//! screen_scale`); the table's HUD takes the same step, but not by
//! multiplying its hundreds of lengths. While a table is open Bevy's
//! [`UiScale`] carries the step, so every `Val::Px`, every font size and the
//! UI's own picking grow together. Back in the lobby only the minimum-canvas
//! fit remains (the shell handles its own big-screen scale). The one thing that must not grow is a position read
//! off the 3D projection: a seat bar is pinned to its seat. So every place
//! the table's HUD reads the window does it through [`space`], the window in
//! UI units (the window over the scale), and a `Lens` built on that size
//! answers in UI units too; drawn at the scale, they land back on the felt.
//! The 3D canvas reads the same space, so the felt makes room for the taller
//! hand zone and the table is framed as it is on a 1080p window.

use crate::DuelPhase;
use crate::shellkit::metrics::screen_scale;
use crate::shellkit::{Platform, TextSize, Viewport};
use bevy::prelude::*;

/// Minimum desktop layout canvas. Smaller windows scale the complete UI,
/// including its picking, instead of entering increasingly cramped layouts.
pub const DESKTOP_MIN: Vec2 = Vec2::new(1280.0, 720.0);

/// Uniform desktop shrink factor; mobile and web retain their own layouts.
/// A minimized/zero-size window keeps a finite, nonzero scale.
#[must_use]
pub fn fit_scale(window: Vec2, platform: Platform) -> f32 {
    if platform != Platform::Desktop || !window.is_finite() || window.min_element() <= 0.0 {
        return 1.0;
    }
    (window / DESKTOP_MIN).min_element().min(1.0)
}

/// Desktop shell canvas before the table's additional big-screen scale.
#[must_use]
pub fn desktop_space(window: &Window, ui: Option<&UiScale>) -> Vec2 {
    // Read the applied scale, including in isolated plugin tests. Without
    // the scaling system installed, layout must match the unscaled UI.
    let size = Vec2::new(window.width(), window.height());
    size / ui.map_or(1.0, |scale| scale.0.clamp(f32::EPSILON, 1.0))
}

/// Dimensions of [`desktop_space`] for shell layout builders.
#[must_use]
pub fn desktop_dimensions(window: &Window, ui: Option<&UiScale>) -> (f32, f32) {
    let size = desktop_space(window, ui);
    (size.x, size.y)
}

/// How much larger the table's HUD draws on a window this size (logical
/// pixels): the shell's big-screen step at the default text step, because the
/// table does not follow the text step. 1 up to 1920 × 1080, 1.25 at
/// 2560 × 1440, 1.75 at 3840 × 2160.
#[must_use]
pub fn table_scale(window: Vec2) -> f32 {
    screen_scale(Viewport::desktop(window.x, window.y), TextSize::M)
}

/// The window in the HUD's units: its logical size over the UI scale.
#[must_use]
pub fn space(window: &Window, ui: Option<&UiScale>) -> Vec2 {
    Vec2::new(window.width(), window.height()) / ui.map_or(1.0, |ui| ui.0.max(f32::EPSILON))
}

/// [`space`] of the window, if there is one.
#[must_use]
pub fn window_space(windows: &Query<&Window>, ui: Option<&UiScale>) -> Option<Vec2> {
    windows.single().ok().map(|window| space(window, ui))
}

/// A cursor (logical window pixels) in the HUD's units.
#[must_use]
pub fn cursor(window: &Window, ui: Option<&UiScale>) -> Option<Vec2> {
    window
        .cursor_position()
        .map(|at| at / ui.map_or(1.0, |ui| ui.0.max(f32::EPSILON)))
}

/// Applies the desktop canvas fit and, at a table, the big-screen step.
/// Writes only on change: changing this resource relays out every UI tree.
pub fn follow_the_window(
    phase: Option<Res<State<DuelPhase>>>,
    windows: Query<&Window>,
    mut ui: ResMut<UiScale>,
) {
    let open = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let want = match windows.single() {
        Ok(window) => {
            let size = Vec2::new(window.width(), window.height());
            let fit = fit_scale(size, Platform::current());
            fit * if open { table_scale(size / fit) } else { 1.0 }
        }
        _ => 1.0,
    };
    if (ui.0 - want).abs() > f32::EPSILON {
        ui.0 = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_canvas_shrinks_uniformly_and_keeps_other_platforms_unchanged() {
        for (size, want) in [
            (Vec2::new(1280.0, 720.0), 1.0),
            (Vec2::new(960.0, 540.0), 0.75),
            (Vec2::new(960.0, 600.0), 0.75),
            (Vec2::new(800.0, 600.0), 0.625),
            (Vec2::new(1920.0, 540.0), 0.75),
            (Vec2::new(1920.0, 1080.0), 1.0),
        ] {
            let scale = fit_scale(size, Platform::Desktop);
            assert!((scale - want).abs() < 1e-6);
            let canvas = size / scale;
            assert!(canvas.x >= DESKTOP_MIN.x && canvas.y >= DESKTOP_MIN.y);
            assert!(((canvas.x / canvas.y) - size.x / size.y).abs() < 1e-6);
            for platform in [Platform::Web, Platform::Mobile] {
                assert!((fit_scale(size, platform) - 1.0).abs() < 1e-6);
            }
        }
        for size in [Vec2::ZERO, Vec2::new(0.0, 720.0), Vec2::NAN] {
            assert!((fit_scale(size, Platform::Desktop) - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn small_desktop_canvas_preserves_cursor_and_table_projection_at_both_dpis() {
        for dpi in [1.0, 2.0] {
            let mut window = Window {
                resolution: bevy::window::WindowResolution::new(960, 540)
                    .with_scale_factor_override(dpi),
                ..default()
            };
            window.resolution.set(960.0, 540.0);
            window.set_cursor_position(Some(Vec2::new(360.0, 202.5)));
            let ui = UiScale(0.75);
            assert!(space(&window, Some(&ui)).distance(DESKTOP_MIN) < 1e-3);
            assert!(
                cursor(&window, Some(&ui))
                    .unwrap()
                    .distance(Vec2::new(480.0, 270.0))
                    < 1e-3
            );
        }
        for (measured, projected) in shelves(Vec2::new(960.0, 540.0), 0.75) {
            assert!((measured.middle * 0.75).distance(projected.middle) < 0.5);
            assert!((measured.along * 0.75 - projected.along).abs() < 0.5);
        }
    }

    #[test]
    fn the_table_scale_is_one_up_to_1080p_and_steps_past_it() {
        for (w, h) in [(1280.0, 720.0), (1708.0, 1032.0), (1920.0, 1080.0)] {
            assert!(
                (table_scale(Vec2::new(w, h)) - 1.0).abs() < f32::EPSILON,
                "{w}x{h}"
            );
        }
        assert!((table_scale(Vec2::new(2560.0, 1440.0)) - 1.25).abs() < 1e-6);
        assert!((table_scale(Vec2::new(3840.0, 2160.0)) - 1.75).abs() < 1e-6);
    }

    /// The hand's cards keep their share of the window's height: 129 px of
    /// 1080 at 1080p, and 1.75 × that of 2160 at 4K (it was 129 of 2160).
    #[test]
    fn a_hand_card_keeps_its_share_of_a_big_window() {
        let share = |h: f32, w: f32| crate::hud::HAND_CARD_H * table_scale(Vec2::new(w, h)) / h;
        let at_1080 = share(1080.0, 1920.0);
        let at_2160 = share(2160.0, 3840.0);
        assert!(at_2160 > 0.85 * at_1080, "{at_2160} vs {at_1080}");
        assert!(share(1440.0, 2560.0) > 0.9 * at_1080);
    }

    /// Every seat's shelf, measured as the client does, in a window of this
    /// size under this UI scale, and the shelves projected straight onto the
    /// window (logical pixels) by the same rig.
    fn shelves(window: Vec2, ui: f32) -> Vec<(crate::hud::Shelf, crate::hud::Shelf)> {
        use crate::table::{CameraRig, Canvas, Lens, ShownRig, TableCamera, apply_camera_rig};
        use baylee_client_core::layout::TableLayout;
        use baylee_core::ids::PlayerId;

        let mut app = App::new();
        app.world_mut().spawn(Window {
            resolution: bevy::window::WindowResolution::new(window.x as u32, window.y as u32)
                .with_scale_factor_override(1.0),
            ..default()
        });
        // The canvas the camera frames in, as `frame_table` reads it.
        let canvas = Canvas::hud(window / ui);
        let seats: Vec<_> = (0..4).map(PlayerId::new).collect();
        let table = TableLayout::new(&seats, canvas.aspect(), None);
        let duel = crate::Duel {
            layout: Some(table.clone()),
            ..Default::default()
        };
        app.insert_resource(CameraRig::home(&table, canvas))
            .insert_resource(duel)
            .insert_resource(UiScale(ui))
            .init_resource::<ShownRig>()
            .init_resource::<crate::hud::Shelves>()
            .init_resource::<Time>()
            .init_resource::<crate::prefs::Prefs>()
            .add_systems(
                Update,
                (apply_camera_rig, crate::hud::measure_shelves).chain(),
            );
        app.world_mut().spawn((TableCamera, Transform::default()));
        app.update();
        let rig = app
            .world()
            .resource::<ShownRig>()
            .rig()
            .expect("the camera was applied");
        let lens = Lens::new(rig, window);
        let measured = app.world().resource::<crate::hud::Shelves>().0.clone();
        assert_eq!(
            measured.len(),
            table.on_felt().count(),
            "every seat measured"
        );
        table
            .on_felt()
            .map(|slot| {
                let ui_shelf = measured
                    .iter()
                    .find_map(|(p, s)| (*p == slot.player).then_some(*s))
                    .expect("the seat's shelf");
                let corners = lens.corners(slot.ledge_corners()).expect("in front");
                (ui_shelf, crate::hud::Shelf::of(corners, false))
            })
            .collect()
    }

    /// At 3840 × 2160 the HUD draws 1.75 times larger, and a seat's bar still
    /// stands on its seat: the shelf, measured in the UI's units and drawn at
    /// the scale, is where the rig projects the seat's ledge on the window.
    #[test]
    fn a_seat_bar_stays_on_its_seat_at_4k() {
        let window = Vec2::new(3840.0, 2160.0);
        let ui = table_scale(window);
        assert!((ui - 1.75).abs() < 1e-6);
        for (measured, projected) in shelves(window, ui) {
            let drawn = measured.middle * ui;
            assert!(
                drawn.distance(projected.middle) < 0.5,
                "drawn at {drawn}, the seat is at {}",
                projected.middle
            );
            assert!((measured.along * ui - projected.along).abs() < 0.5);
        }
    }

    /// Up to 1920 × 1080 nothing changes: the scale is 1 and the shelves are
    /// the window's own pixels.
    #[test]
    fn nothing_moves_at_1080p() {
        let window = Vec2::new(1920.0, 1080.0);
        let ui = table_scale(window);
        assert!((ui - 1.0).abs() < f32::EPSILON);
        for (measured, projected) in shelves(window, ui) {
            assert!(measured.middle.distance(projected.middle) < 1e-3);
        }
    }

    #[test]
    fn the_ui_scale_follows_the_table_and_returns_to_one_in_the_lobby() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<DuelPhase>()
            .init_resource::<UiScale>()
            .add_systems(Update, follow_the_window);
        app.world_mut().spawn(Window {
            resolution: bevy::window::WindowResolution::new(3840, 2160)
                .with_scale_factor_override(1.0),
            ..default()
        });
        app.update();
        assert!((app.world().resource::<UiScale>().0 - 1.0).abs() < 1e-6);
        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Playing);
        app.update();
        app.update();
        assert!((app.world().resource::<UiScale>().0 - 1.75).abs() < 1e-6);
        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Closed);
        app.update();
        app.update();
        assert!((app.world().resource::<UiScale>().0 - 1.0).abs() < 1e-6);
    }
}

#[cfg(test)]
mod regressions;
