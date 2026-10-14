//! Headless picking dispatch: synthetic backend hits, real Bevy pointer
//! event generation, button observers, and independently delivered messages.

use bevy::camera::{ManualTextureViewHandle, NormalizedRenderTarget};
use bevy::input::InputPlugin;
use bevy::input_focus::{InputDispatchPlugin, InputFocusPlugin, pointer_focus::PointerFocusPlugin};
use bevy::picking::PickingSettings;
use bevy::picking::backend::HitData;
use bevy::picking::events::{
    PointerCancel, PointerClick, PointerDrag, PointerDragDrop, PointerDragEnd, PointerDragEnter,
    PointerDragLeave, PointerDragOver, PointerDragStart, PointerEnter, PointerLeave, PointerMove,
    PointerOut, PointerOver, PointerPress, PointerRelease, PointerScroll, PointerState,
    pointer_events,
};
use bevy::picking::hover::{HoverMap, PreviousHoverMap, update_is_hovered};
use bevy::picking::pointer::{
    Location, PointerAction, PointerId, PointerInput, PointerLocation, PointerMap,
};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ButtonPlugin};

/// The unfiltered widget events, kept separate from domain actions.
#[derive(Resource, Default)]
pub(crate) struct Activations(pub Vec<Entity>);

/// The production input/focus/button plugins, without window or render
/// backends. In particular pointer focus and keyboard dispatch both run.
pub(crate) fn input_plugins(app: &mut App) {
    app.add_plugins((
        InputPlugin,
        InputFocusPlugin,
        InputDispatchPlugin,
        PointerFocusPlugin,
        ButtonPlugin,
    ));
    super::install(app);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
}

/// Only the renderer's hit testing is replaced. `pointer_events` produces
/// both observer triggers and Messages, as it does in `DefaultPickingPlugins`.
pub(crate) fn app() -> App {
    let mut app = App::new();
    input_plugins(&mut app);
    app.init_resource::<HoverMap>()
        .init_resource::<PreviousHoverMap>()
        .init_resource::<PickingSettings>()
        .init_resource::<PointerState>()
        .init_resource::<PointerMap>()
        .init_resource::<Activations>()
        .add_message::<PointerInput>()
        .add_message::<PointerCancel>()
        .add_message::<PointerClick>()
        .add_message::<PointerPress>()
        .add_message::<PointerDragDrop>()
        .add_message::<PointerDragEnd>()
        .add_message::<PointerDragEnter>()
        .add_message::<PointerDrag>()
        .add_message::<PointerDragLeave>()
        .add_message::<PointerDragOver>()
        .add_message::<PointerDragStart>()
        .add_message::<PointerScroll>()
        .add_message::<PointerMove>()
        .add_message::<PointerOut>()
        .add_message::<PointerOver>()
        .add_message::<PointerLeave>()
        .add_message::<PointerEnter>()
        .add_message::<PointerRelease>()
        .add_observer(|event: On<Activate>, mut log: ResMut<Activations>| {
            log.0.push(event.entity);
        })
        .add_systems(PreUpdate, (update_is_hovered, pointer_events).chain());
    for id in [PointerId::Mouse, PointerId::Touch(1), PointerId::Touch(2)] {
        app.world_mut()
            .spawn((id, PointerLocation::new(location())));
    }
    app
}

fn location() -> Location {
    Location {
        target: NormalizedRenderTarget::TextureView(ManualTextureViewHandle(1)),
        position: Vec2::new(10.0, 10.0),
    }
}

/// One frame of backend hits and raw pointer input. Releases use the previous
/// hover map in Bevy, including touch disappearing in its release frame.
pub(crate) fn frame(
    app: &mut App,
    id: PointerId,
    target: Option<Entity>,
    actions: &[PointerAction],
) {
    let previous = app.world().resource::<HoverMap>().0.clone();
    app.insert_resource(PreviousHoverMap(previous));
    let hits = target
        .into_iter()
        .map(|entity| (entity, HitData::new(Entity::PLACEHOLDER, 0.0, None, None)))
        .collect();
    app.world_mut().resource_mut::<HoverMap>().insert(id, hits);
    let pointer = app
        .world()
        .resource::<PointerMap>()
        .get_entity(id)
        .expect("test pointer");
    let mut location = app
        .world()
        .get::<PointerLocation>(pointer)
        .expect("pointer location")
        .location
        .clone()
        .expect("located pointer");
    for action in actions {
        if let PointerAction::Move { delta } = action {
            location.position += *delta;
        }
        app.world_mut()
            .write_message(PointerInput::new(id, location.clone(), *action));
    }
    app.world_mut()
        .entity_mut(pointer)
        .insert(PointerLocation::new(location));
    app.update();
}

/// A primary click, with down and up dispatched in different frames.
pub(crate) fn click(app: &mut App, target: Entity) {
    use bevy::picking::pointer::PointerButton;
    frame(
        app,
        PointerId::Mouse,
        Some(target),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    frame(
        app,
        PointerId::Mouse,
        Some(target),
        &[PointerAction::Release(PointerButton::Primary)],
    );
}
