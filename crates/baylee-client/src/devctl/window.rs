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
    let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
    order.insert_before(PreUpdate, RestorePointer);
    order.insert_before(Last, HidePointer);
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
