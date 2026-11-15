//! The lobby's scrolling lists, and where each one was left.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// What one line of wheel travel moves a list, in logical pixels.
pub(crate) const WHEEL_LINE: f32 = 32.0;

/// A list that scrolls its own contents, and which one it is.
///
/// `Overflow::scroll_y` only *clips*: Bevy moves the content when
/// [`ScrollPosition`] changes and nothing changes it on its own. Without this
/// system a sixty-row result list would simply end at the bottom of the panel
/// with no way to reach the rest.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Scrollable(pub(crate) List);

mod memory;
pub(crate) use memory::{List, Scrolled};

/// Turns a wheel or a swipe into scrolling on the list under the pointer.
#[allow(clippy::too_many_arguments)]
pub(super) fn scrolls(
    mut wheels: MessageReader<PointerScroll>,
    mut drags: MessageReader<PointerDrag>,
    parents: Query<&ChildOf>,
    scrollbars: Query<&bevy::ui_widgets::Scrollbar>,
    mut lists: Query<(&mut ScrollPosition, &ComputedNode, &Scrollable)>,
    mut memory: ResMut<Scrolled>,
    entrance: Res<super::entrance::Entrance>,
    (journey, ui): (Option<Res<crate::arrival::Journey>>, Option<Res<UiScale>>),
) {
    if journey.as_ref().is_some_and(|j| j.active()) || entrance.active() {
        wheels.clear();
        drags.clear();
        return;
    }
    let ui = ui.as_deref().map_or(1.0, |ui| ui.0);
    for wheel in wheels.read() {
        let travel = match wheel.unit {
            MouseScrollUnit::Line => wheel.y * WHEEL_LINE,
            MouseScrollUnit::Pixel => wheel.y / ui,
        };
        // A wheel pushed away from the reader moves the content up, which is
        // an *increase* in the scroll offset.
        scroll_lineage(wheel.entity, -travel, &parents, &mut lists, &mut memory);
    }
    for drag in drags.read() {
        if in_scrollbar(drag.entity, &scrollbars, &parents) {
            continue;
        }
        // A finger drags the content itself, so it goes the other way again.
        scroll_lineage(
            drag.entity,
            -drag.delta.y / ui,
            &parents,
            &mut lists,
            &mut memory,
        );
    }
}

fn in_scrollbar(
    entity: Entity,
    scrollbars: &Query<&bevy::ui_widgets::Scrollbar>,
    parents: &Query<&ChildOf>,
) -> bool {
    let mut current = Some(entity);
    while let Some(e) = current {
        if scrollbars.contains(e) {
            return true;
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    false
}

/// Scrolls the nearest list at or above an entity, so a gesture over a row
/// scrolls the list the row is in.
fn scroll_lineage(
    entity: Entity,
    by: f32,
    parents: &Query<&ChildOf>,
    lists: &mut Query<(&mut ScrollPosition, &ComputedNode, &Scrollable)>,
    memory: &mut Scrolled,
) {
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Ok((mut position, computed, which)) = lists.get_mut(e) {
            let before = position.y;
            position.y = crate::hud::scrolled(
                position.y,
                by,
                computed.size().y,
                computed.content_size().y,
                computed.inverse_scale_factor(),
            );
            if (position.y - before).abs() > f32::EPSILON {
                memory.set(which.0, position.y);
                return;
            }
            // An exhausted inner list must not trap a short screen's form.
            // Continue up to the first ancestor that can consume the gesture.
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
}

#[cfg(test)]
mod scrolling_tests {
    use super::*;

    #[test]
    fn drag_and_pixel_scroll_follow_the_pointer_under_ui_scale_and_dpi() {
        use bevy::camera::{ManualTextureViewHandle, NormalizedRenderTarget};
        use bevy::picking::{
            events::Pointer,
            pointer::{Location, PointerId},
        };
        for ui in [0.5, 0.75, 1.0, 1.75] {
            for dpi in [1.0, 2.0] {
                let mut app = App::new();
                app.insert_resource(UiScale(ui))
                    .init_resource::<Scrolled>()
                    .init_resource::<crate::lobby::entrance::Entrance>()
                    .add_message::<PointerScroll>()
                    .add_message::<PointerDrag>()
                    .add_systems(Update, scrolls);
                let scale = ui * dpi;
                let entity = app
                    .world_mut()
                    .spawn((
                        ScrollPosition::default(),
                        Scrollable(List::Table),
                        ComputedNode {
                            size: Vec2::new(300.0, 100.0) * scale,
                            content_size: Vec2::new(300.0, 2000.0) * scale,
                            inverse_scale_factor: 1.0 / scale,
                            ..default()
                        },
                    ))
                    .id();
                let pointer = Pointer::new(
                    PointerId::Mouse,
                    Location {
                        target: NormalizedRenderTarget::TextureView(ManualTextureViewHandle(1)),
                        position: Vec2::ZERO,
                    },
                );
                app.world_mut().write_message(PointerDrag {
                    entity,
                    pointer: pointer.clone(),
                    button: PointerButton::Primary,
                    distance: Vec2::new(0.0, -150.0),
                    delta: Vec2::new(0.0, -150.0),
                });
                app.update();
                let scroll = |app: &App| app.world().get::<ScrollPosition>(entity).unwrap().y * ui;
                assert!(
                    (scroll(&app) - 150.0).abs() < 1e-3,
                    "drag: UI {ui}, DPI {dpi}"
                );
                app.world_mut().write_message(PointerScroll {
                    entity,
                    pointer,
                    unit: MouseScrollUnit::Pixel,
                    x: 0.0,
                    y: -90.0,
                    hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                    phase: bevy::input::touch::TouchPhase::Moved,
                });
                app.update();
                assert!(
                    (scroll(&app) - 240.0).abs() < 1e-3,
                    "wheel: UI {ui}, DPI {dpi}"
                );
            }
        }
    }

    #[test]
    #[allow(clippy::type_complexity)] // explicit Bevy SystemState for exercising the actual scroll ancestry
    fn a_full_or_exhausted_inner_list_passes_scroll_to_the_form() {
        use bevy::ecs::system::SystemState;
        let mut world = World::new();
        world.init_resource::<Scrolled>();
        let computed = |content| ComputedNode {
            size: Vec2::new(300.0, 100.0),
            content_size: Vec2::new(300.0, content),
            inverse_scale_factor: 1.0,
            ..default()
        };
        let form = world
            .spawn((
                ScrollPosition::default(),
                computed(600.0),
                Scrollable(List::Table),
            ))
            .id();
        let inner = world
            .spawn((
                ScrollPosition::default(),
                computed(100.0),
                Scrollable(List::Gateways),
                ChildOf(form),
            ))
            .id();
        let mut system = SystemState::<(
            Query<&ChildOf>,
            Query<(&mut ScrollPosition, &ComputedNode, &Scrollable)>,
            ResMut<Scrolled>,
        )>::new(&mut world);
        let scroll = |world: &mut World,
                      system: &mut SystemState<(
            Query<&ChildOf>,
            Query<(&mut ScrollPosition, &ComputedNode, &Scrollable)>,
            ResMut<Scrolled>,
        )>,
                      by| {
            let (parents, mut lists, mut memory) = system.get_mut(world).expect("scroll query");
            scroll_lineage(inner, by, &parents, &mut lists, &mut memory);
        };
        scroll(&mut world, &mut system, 30.0);
        assert!((world.get::<ScrollPosition>(form).unwrap().y - 30.0).abs() < 0.001);
        world.entity_mut(inner).insert(computed(300.0));
        scroll(&mut world, &mut system, 30.0);
        assert!((world.get::<ScrollPosition>(inner).unwrap().y - 30.0).abs() < 0.001);
        assert!((world.get::<ScrollPosition>(form).unwrap().y - 30.0).abs() < 0.001);
        world
            .entity_mut(inner)
            .insert(ScrollPosition(Vec2::new(0.0, 200.0)));
        scroll(&mut world, &mut system, 30.0);
        assert!((world.get::<ScrollPosition>(form).unwrap().y - 60.0).abs() < 0.001);
    }
}
