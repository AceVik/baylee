//! Scaling must preserve hit testing and avoid idle relayouts.
use super::*;

fn window(width: u32, height: u32) -> Window {
    Window {
        resolution: bevy::window::WindowResolution::new(width, height)
            .with_scale_factor_override(1.0),
        ..default()
    }
}

#[test]
fn invalid_dimensions_never_produce_a_zero_or_nonfinite_fit() {
    for value in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for size in [Vec2::new(value, 720.0), Vec2::new(1280.0, value)] {
            let fit = fit_scale(size, Platform::Desktop);
            assert!(fit.is_finite() && fit > 0.0);
            assert!((fit - 1.0).abs() < f32::EPSILON);
        }
    }
}

#[test]
fn minimum_canvas_and_cursor_round_trip_across_aspect_ratios() {
    for (width, height) in [
        (320, 240),
        (800, 600),
        (720, 1280),
        (1280, 720),
        (2560, 720),
        (3840, 2160),
    ] {
        let mut window = window(width, height);
        let fit = fit_scale(
            Vec2::new(window.width(), window.height()),
            Platform::Desktop,
        );
        let ui = UiScale(fit);
        let canvas = desktop_space(&window, Some(&ui));
        assert!(canvas.x >= DESKTOP_MIN.x - 0.01 && canvas.y >= DESKTOP_MIN.y - 0.01);
        for fraction in [0.0, 0.25, 0.5, 0.99] {
            let position = Vec2::new(window.width(), window.height()) * fraction;
            window.set_cursor_position(Some(position));
            let recovered = cursor(&window, Some(&ui)).expect("cursor inside window") * fit;
            assert!(recovered.distance(position) < 0.01);
        }
        window.set_cursor_position(None);
        assert!(cursor(&window, Some(&ui)).is_none());
    }
}

#[derive(Resource, Default)]
struct ScaleWrites(usize);

fn count_scale_writes(scale: Res<UiScale>, mut writes: ResMut<ScaleWrites>) {
    writes.0 += usize::from(scale.is_changed());
}

#[test]
fn idle_frames_do_not_retrigger_layout_and_a_resize_writes_once() {
    let mut app = App::new();
    app.init_resource::<UiScale>()
        .init_resource::<ScaleWrites>()
        .add_systems(Update, (follow_the_window, count_scale_writes).chain());
    let entity = app.world_mut().spawn(window(800, 600)).id();
    app.update();
    assert!((app.world().resource::<UiScale>().0 - 0.625).abs() < 1e-6);
    let first = app.world().resource::<ScaleWrites>().0;
    assert_eq!(first, 1);
    for _ in 0..12 {
        app.update();
    }
    assert_eq!(app.world().resource::<ScaleWrites>().0, first);
    app.world_mut()
        .entity_mut(entity)
        .get_mut::<Window>()
        .unwrap()
        .resolution
        .set(1280.0, 720.0);
    app.update();
    assert!((app.world().resource::<UiScale>().0 - 1.0).abs() < 1e-6);
    assert_eq!(app.world().resource::<ScaleWrites>().0, first + 1);
    app.update();
    assert_eq!(app.world().resource::<ScaleWrites>().0, first + 1);
}

#[test]
fn missing_or_ambiguous_windows_reset_the_scale_instead_of_reusing_a_table_fit() {
    for count in [0, 2] {
        let mut app = App::new();
        app.insert_resource(UiScale(0.625))
            .add_systems(Update, follow_the_window);
        for _ in 0..count {
            app.world_mut().spawn(window(800, 600));
        }
        app.update();
        assert!((app.world().resource::<UiScale>().0 - 1.0).abs() < 1e-6);
    }
}

#[test]
fn shell_space_ignores_table_enlargement_while_hud_space_preserves_it() {
    let window = window(3840, 2160);
    let ui = UiScale(1.75);
    let raw = Vec2::new(window.width(), window.height());
    assert!(desktop_space(&window, Some(&ui)).distance(raw) < 0.01);
    assert!((space(&window, Some(&ui)) * ui.0).distance(raw) < 0.01);
    assert!(space(&window, None).distance(raw) < 0.01);
    assert!(desktop_space(&window, None).distance(raw) < 0.01);
}
