//! Keep harness coordinates inside the client and choose its test display.

use bevy::app::MainScheduleOrder;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow};
use bevy::winit::WinitMonitors;

use super::DevControl;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct RestorePointer;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct HidePointer;

pub(super) fn install(app: &mut App) {
    app.add_systems(RestorePointer, restore_pointer)
        .add_systems(HidePointer, hide_pointer);
    if super::passive() {
        app.add_systems(Update, lift_while_capturing);
    }
    let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
    order.insert_before(PreUpdate, RestorePointer);
    order.insert_before(Last, HidePointer);
}

/// How long a passive window stays above the others after its last capture:
/// a check takes its screenshots a few at a time, and a window that went
/// down between them would have to come up and be drawn afresh for each.
const LIFT_LINGER: std::time::Duration = std::time::Duration::from_secs(1);

/// Whether the window is lifted now, and when a capture was last seen.
fn lifted(
    capturing: bool,
    last: Option<std::time::Duration>,
    now: std::time::Duration,
) -> (bool, Option<std::time::Duration>) {
    let last = if capturing { Some(now) } else { last };
    let lift = last.is_some_and(|at| now.saturating_sub(at) < LIFT_LINGER);
    (lift, last)
}

/// Lifts a passive window above the others only while something is captured
/// from it, and lets it sink back a moment after. It no longer stays on top
/// for the whole run (owner, 29.10.2026), but wgpu acquires no drawable for a
/// window macOS reports occluded, so a covered window would capture nothing
/// but black: while a screenshot is pending or a recording takes frames, it
/// stands above the others, still without the focus and still letting the
/// pointer through. A screenshot asked of a covered window is answered from
/// the first frame drawn after it came up.
fn lift_while_capturing(
    shots: Query<(), With<bevy::render::view::screenshot::Screenshot>>,
    recorder: Option<Res<super::recording::Recorder>>,
    time: Res<Time<Real>>,
    mut last: Local<Option<std::time::Duration>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let capturing = !shots.is_empty() || recorder.is_some_and(|recorder| recorder.capturing());
    let (lift, seen) = lifted(capturing, *last, time.elapsed());
    *last = seen;
    let level = if lift {
        bevy::window::WindowLevel::AlwaysOnTop
    } else {
        bevy::window::WindowLevel::Normal
    };
    if let Ok(mut window) = windows.single_mut()
        && window.window_level != level
    {
        window.window_level = level;
    }
}

fn restore_pointer(control: Res<DevControl>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Some(at) = control.virtual_cursor
        && let Ok(mut window) = windows.single_mut()
    {
        window
            .bypass_change_detection()
            .set_cursor_position(Some(at));
    }
}

fn hide_pointer(control: Res<DevControl>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if control.virtual_cursor.is_some()
        && let Ok(mut window) = windows.single_mut()
    {
        // winit's Last/changed_windows warps for Some(position), even if a
        // different Window field triggered change detection. None never warps.
        // All client input/layout systems have already run; next frame restores
        // the virtual position before picking and hit testing consume it.
        window.bypass_change_detection().set_cursor_position(None);
    }
}

pub(crate) fn monitor_selection() -> Result<Option<MonitorSelection>, String> {
    std::env::var("BAYLEE_DEV_MONITOR")
        .ok()
        .map(|value| parse_monitor(&value))
        .transpose()
}

fn parse_monitor(value: &str) -> Result<MonitorSelection, String> {
    if value == "primary" {
        return Ok(MonitorSelection::Primary);
    }
    value
        .parse::<usize>()
        .map(MonitorSelection::Index)
        .map_err(|_| {
            "BAYLEE_DEV_MONITOR must be 'primary' or a zero-based /monitors index".to_string()
        })
}

pub(crate) fn configure_monitor(
    monitors: Res<WinitMonitors>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(Some(selection)) = monitor_selection() else {
        return;
    };
    if let MonitorSelection::Index(index) = selection
        && monitors.nth(index).is_none()
    {
        // The standalone window is still hidden during Startup. Do not show
        // a fallback window on whichever display the owner is working on.
        baylee_client_core::say_err!(
            "BAYLEE_DEV_MONITOR index {index} is unavailable; closing the test client"
        );
        exit.write(AppExit::error());
        return;
    }
    if let Ok(mut window) = windows.single_mut() {
        // The native monitor inventory was empty during initial window creation.
        // Position while hidden and unmaximized; winit applies the position
        // before the maximize request in Last, on the selected display.
        window.position = bevy::window::WindowPosition::Centered(selection);
        window.set_maximized(super::window_size().is_none());
    }
}

pub(super) fn native_pointer(
    control: &mut DevControl,
    buttons: &ButtonInput<MouseButton>,
    windows: &mut Query<(Entity, &mut Window), With<PrimaryWindow>>,
) -> String {
    if !control.clicking.is_empty() || buttons.get_pressed().next().is_some() {
        return "{\"error\":\"finish pointer requests and release buttons first\"}".to_string();
    }
    control.virtual_cursor = None;
    if let Ok((_, mut window)) = windows.single_mut() {
        window.bypass_change_detection().set_cursor_position(None);
    }
    "{\"ok\":true,\"native\":true}".to_string()
}

pub(super) fn monitors_json(monitors: Option<&WinitMonitors>) -> String {
    let Some(monitors) = monitors else {
        return serde_json::json!({"error": "native monitor inventory unavailable"}).to_string();
    };
    let mut rows = Vec::new();
    // Use the same ordering as WindowPosition::Centered(MonitorSelection::Index).
    // ECS entity query order is not a monitor index.
    while let Some(monitor) = monitors.nth(rows.len()) {
        let position = monitor.position();
        let size = monitor.size();
        rows.push(serde_json::json!({
            "index": rows.len(), "name": monitor.name(),
            "x": position.x, "y": position.y,
            "width": size.width, "height": size.height,
            "scale": monitor.scale_factor(),
        }));
    }
    serde_json::json!({"monitors": rows}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_is_lifted_while_capturing_and_sinks_a_moment_after() {
        let at = std::time::Duration::from_secs;
        let ms = std::time::Duration::from_millis;
        // Never captured: it stays where the owner's windows put it.
        assert_eq!(lifted(false, None, at(5)), (false, None));
        // A capture lifts it at once.
        assert_eq!(lifted(true, None, at(5)), (true, Some(at(5))));
        // It lingers a moment for the next screenshot of a run...
        assert_eq!(
            lifted(false, Some(at(5)), at(5) + ms(900)),
            (true, Some(at(5)))
        );
        // ...and then sinks back behind the owner's windows.
        assert_eq!(
            lifted(false, Some(at(5)), at(5) + LIFT_LINGER),
            (false, Some(at(5)))
        );
    }

    /// The system, not just the rule: a pending screenshot lifts the window
    /// in the frame it is asked for, and an empty queue leaves a fresh window
    /// at the normal level.
    #[test]
    fn a_pending_screenshot_lifts_the_window_and_nothing_else_does() {
        let (mut app, _tx) = super::super::tests::harness();
        app.init_resource::<Time<Real>>()
            .add_systems(Update, lift_while_capturing);
        let level = |app: &mut App| {
            let mut windows = app
                .world_mut()
                .query_filtered::<&Window, With<PrimaryWindow>>();
            windows.single(app.world()).unwrap().window_level
        };
        app.update();
        assert_eq!(level(&mut app), bevy::window::WindowLevel::Normal);
        app.world_mut()
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window());
        app.update();
        assert_eq!(level(&mut app), bevy::window::WindowLevel::AlwaysOnTop);
    }

    #[test]
    fn monitor_selection_is_explicit() {
        assert_eq!(parse_monitor("primary").unwrap(), MonitorSelection::Primary);
        assert_eq!(parse_monitor("2").unwrap(), MonitorSelection::Index(2));
        for bad in ["", "-1", "1.5", "display2", "999999999999999999999999"] {
            assert!(parse_monitor(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn virtual_pointer_is_hidden_even_when_another_window_field_changes() {
        let (mut app, tx) = super::super::tests::harness();
        app.add_systems(RestorePointer, restore_pointer)
            .add_systems(HidePointer, hide_pointer);
        let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
        order.insert_before(PreUpdate, RestorePointer);
        order.insert_before(Last, HidePointer);
        let _reply = super::super::tests::ask(&tx, "/pointer", r#"{"x":37,"y":52}"#);
        app.update();
        for _ in 0..4 {
            app.world_mut().run_schedule(RestorePointer);
            let mut windows = app
                .world_mut()
                .query_filtered::<&mut Window, With<PrimaryWindow>>();
            let mut window = windows.single_mut(app.world_mut()).unwrap();
            assert_eq!(window.cursor_position(), Some(Vec2::new(37.0, 52.0)));
            window.resolution.set(900.0, 700.0);
            app.update();
            let window = windows.single(app.world()).unwrap();
            assert_eq!(window.physical_cursor_position(), None);
        }
        let reply = super::super::tests::ask(&tx, "/pointer/native", "{}");
        app.update();
        assert!(reply.recv().unwrap().contains("\"native\":true"));
        assert!(
            app.world()
                .resource::<DevControl>()
                .virtual_cursor
                .is_none()
        );
        let mut windows = app
            .world_mut()
            .query_filtered::<&mut Window, With<PrimaryWindow>>();
        windows
            .single_mut(app.world_mut())
            .unwrap()
            .set_cursor_position(Some(Vec2::new(9.0, 8.0)));
        app.update();
        assert_eq!(
            windows.single(app.world()).unwrap().cursor_position(),
            Some(Vec2::new(9.0, 8.0))
        );
    }
}
