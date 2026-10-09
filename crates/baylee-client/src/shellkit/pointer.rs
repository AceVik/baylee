//! The pointer's shape (owner, 09.10.2026): a hand over everything a click
//! acts on, a text bar over a field, the arrow everywhere else.
//!
//! One system for the whole client, reading bevy's [`HoverMap`] for the
//! mouse: the shape is decided by what lies under the pointer and every
//! ancestor of it, so a control is a hand wherever its label, icon or padding
//! is hovered. What counts as acting on a click is what the client already
//! marks: a lobby [`Press`], a kit hit area ([`Role::Hit`]), anything that
//! answers the pointer with a [`Feel`], the table's buttons, a slider's
//! track, and [`Clickable`] for the rest (a playable card in the hand). A
//! control drawn but off ([`Disabled`]) keeps the arrow: it says why, it
//! does nothing. On the felt a card is a hand while this client offers
//! something for it ([`HoveredOffer`](crate::table::HoveredOffer)).
//!
//! The window's [`CursorIcon`] is written only when the shape changes, so a
//! still pointer writes nothing (`rest_tests`).

use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};

use super::controls::{Disabled, Slider};
use super::role::Role;
use crate::ambience::Feel;

/// Something a click acts on that carries none of the client's other marks.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Clickable;

/// The three shapes the pointer takes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    /// Nothing to click.
    Arrow,
    /// A click acts here.
    Hand,
    /// A text field.
    Text,
}

impl Shape {
    fn icon(self) -> CursorIcon {
        CursorIcon::System(match self {
            Self::Arrow => SystemCursorIcon::Default,
            Self::Hand => SystemCursorIcon::Pointer,
            Self::Text => SystemCursorIcon::Text,
        })
    }
}

/// What a node is to the pointer, read off its components.
#[derive(bevy::ecs::query::QueryData)]
pub(crate) struct Marks {
    role: Option<&'static Role>,
    off: Has<Disabled>,
    press: Option<&'static crate::lobby::Press>,
    feel: Has<Feel>,
    clickable: Has<Clickable>,
    slider: Has<Slider>,
    button: Has<Button>,
}

/// The table's own buttons, which answer through their own components.
pub(crate) type TableButtons = Or<(
    With<crate::hud::MenuButton>,
    With<crate::hud::PromptButton>,
    With<crate::hud::AbilityButton>,
    With<crate::hud::ChoiceButton>,
    With<crate::hud::PlayerTab>,
    With<crate::hud::TrayTab>,
    With<crate::hud::TrayMinimise>,
    With<crate::hud::TrayMaximise>,
    With<crate::hud::TraySort>,
    With<crate::hud::TrayView>,
    With<crate::hud::TrayGear>,
    With<crate::hud::TrayFilter>,
    With<crate::filterui::FilterAct>,
    With<crate::report::DeskPress>,
)>;

impl MarksItem<'_, '_> {
    /// What this node alone says the pointer is over, if anything.
    fn shape(&self) -> Option<Shape> {
        if self.off {
            return Some(Shape::Arrow);
        }
        if matches!(self.role, Some(Role::Field))
            || matches!(
                self.press,
                Some(crate::lobby::Press::Shared(
                    crate::lobby::SharedPress::Focus(_)
                ))
            )
        {
            return Some(Shape::Text);
        }
        let acts = self.press.is_some()
            || matches!(self.role, Some(Role::Hit))
            || self.feel
            || self.clickable
            || self.slider
            || matches!(self.role, Some(Role::Slider))
            || self.button;
        acts.then_some(Shape::Hand)
    }
}

/// The shape for the entities under the pointer: the first of them (nearest
/// the eye first) that is, or stands inside, something a click acts on.
pub(crate) fn shape_under(
    hovered: impl IntoIterator<Item = Entity>,
    marks: &Query<Marks>,
    tables: &Query<(), TableButtons>,
    parents: &Query<&ChildOf>,
) -> Shape {
    for entity in hovered {
        let mut at = Some(entity);
        while let Some(here) = at {
            if let Some(shape) = marks.get(here).ok().and_then(|m| m.shape()) {
                return shape;
            }
            if tables.contains(here) {
                return Shape::Hand;
            }
            at = parents.get(here).ok().map(ChildOf::parent);
        }
    }
    Shape::Arrow
}

/// Sets the window's pointer from what lies under the mouse.
pub(crate) fn follow_the_pointer(
    mut commands: Commands,
    hovers: Option<Res<HoverMap>>,
    marks: Query<Marks>,
    tables: Query<(), TableButtons>,
    parents: Query<&ChildOf>,
    offer: Option<Res<crate::table::HoveredOffer>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    windows: Query<(Entity, Option<&CursorIcon>), With<PrimaryWindow>>,
) {
    let Ok((window, now)) = windows.single() else {
        return;
    };
    let mut shape = hovers
        .as_deref()
        .and_then(|h| h.get(&PointerId::Mouse))
        .map_or(Shape::Arrow, |over| {
            // Nearest the eye first: a button over a card is the button.
            let mut hits: Vec<(Entity, f32)> =
                over.iter().map(|(e, hit)| (*e, hit.depth)).collect();
            hits.sort_by(|a, b| a.1.total_cmp(&b.1));
            shape_under(hits.into_iter().map(|(e, _)| e), &marks, &tables, &parents)
        });
    let at_table = phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed);
    if shape == Shape::Arrow && at_table && offer.is_some_and(|o| o.0) {
        shape = Shape::Hand;
    }
    let want = shape.icon();
    if now != Some(&want) {
        commands.entity(window).insert(want);
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<crate::table::HoveredOffer>()
        .add_systems(PostUpdate, follow_the_pointer);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lobby::{Press, SharedPress};

    fn shape_of(app: &mut App, entity: Entity) -> Shape {
        let mut state: bevy::ecs::system::SystemState<(
            Query<Marks>,
            Query<(), TableButtons>,
            Query<&ChildOf>,
        )> = bevy::ecs::system::SystemState::new(app.world_mut());
        let (marks, tables, parents) = state.get(app.world()).expect("the queries");
        shape_under([entity], &marks, &tables, &parents)
    }

    /// Every clickable role the kit draws shows the hand, wherever on it the
    /// pointer rests (a label inside a face inside a hit area); a field shows
    /// the text bar; a control drawn but off, and plain words, the arrow.
    #[test]
    fn every_clickable_role_shows_the_hand() {
        let mut app = App::new();
        let world = app.world_mut();
        let wrapped = |world: &mut World, role: Role, extra: Option<Press>| {
            let label = world.spawn(Text::new("x")).id();
            let face = world.spawn(role).add_child(label).id();
            let mut hit = world.spawn(Role::Hit);
            if let Some(press) = extra {
                hit.insert(press);
            }
            hit.add_child(face);
            label
        };
        let roles = [
            Role::Button,
            Role::Chip,
            Role::Tab,
            Role::Nav,
            Role::Pill,
            Role::Segment,
            Role::Toggle,
            Role::MenuItem,
        ];
        let labels: Vec<Entity> = roles
            .iter()
            .map(|role| wrapped(world, *role, None))
            .collect();
        let tile = world.spawn((Role::Tile, Feel::new(Color::NONE))).id();
        let row_label = world.spawn(Text::new("a row")).id();
        world
            .spawn((Role::Row, Press::Shared(SharedPress::PickerNothing)))
            .add_child(row_label);
        let field = world
            .spawn((
                Role::Field,
                Press::Shared(SharedPress::Focus(
                    baylee_client_core::lobby::Field::Username,
                )),
            ))
            .id();
        let dead_label = world.spawn(Text::new("off")).id();
        let dead_face = world.spawn(Role::Button).add_child(dead_label).id();
        world.spawn((Role::Hit, Disabled)).add_child(dead_face);
        let words = world.spawn(Text::new("just words")).id();
        let card = world.spawn(Clickable).id();
        let track = world.spawn(Role::Slider).id();
        for (role, label) in roles.iter().zip(labels) {
            assert_eq!(shape_of(&mut app, label), Shape::Hand, "{role:?}");
        }
        assert_eq!(shape_of(&mut app, tile), Shape::Hand, "a tile");
        assert_eq!(shape_of(&mut app, row_label), Shape::Hand, "a row's words");
        assert_eq!(shape_of(&mut app, card), Shape::Hand, "a playable card");
        assert_eq!(shape_of(&mut app, track), Shape::Hand, "a slider's track");
        assert_eq!(shape_of(&mut app, field), Shape::Text, "a field");
        assert_eq!(
            shape_of(&mut app, dead_label),
            Shape::Arrow,
            "a control that is off"
        );
        assert_eq!(shape_of(&mut app, words), Shape::Arrow, "words");
    }

    /// The window's pointer is written when the shape changes and not again
    /// while it stays (a still pointer writes nothing).
    #[test]
    fn the_window_is_written_only_when_the_shape_changes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<HoverMap>()
            .init_resource::<crate::table::HoveredOffer>()
            .add_systems(Update, follow_the_pointer);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let button = app.world_mut().spawn(Role::Hit).id();
        app.update();
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&Shape::Arrow.icon())
        );
        let mut over = bevy::ecs::entity::EntityHashMap::default();
        over.insert(
            button,
            bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        );
        app.world_mut()
            .resource_mut::<HoverMap>()
            .insert(PointerId::Mouse, over);
        app.update();
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&Shape::Hand.icon())
        );
        let tick = app
            .world()
            .entity(window)
            .get_ref::<CursorIcon>()
            .map(|r| r.last_changed());
        app.update();
        app.update();
        let again = app
            .world()
            .entity(window)
            .get_ref::<CursorIcon>()
            .map(|r| r.last_changed());
        assert_eq!(tick, again, "a still pointer wrote the window");
    }
}
