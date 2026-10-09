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

/// The lists that remember where they were left.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum List {
    /// The searchable card pool.
    Pool,
    /// The deck being built.
    Deck,
    /// The tables and decks on the lobby screen.
    Table,
    /// The saved gateways on the front door.
    Gateways,
    Games,
    RoomCards,
    Library,
    PickerSets,
    PickerPanel,
    Settings,
    /// The import or export dialog.
    Transfer,
    /// The terms sheet's text (WP1).
    Terms,
    /// The About sheet's text (WP1).
    About,
    /// A language-model profile's sheet (WP5).
    ProfileSheet,
    /// The settings screen's section list (WP5).
    SettingsNav,
}

/// Where each list was left, across rebuilds of the node tree.
///
/// Deliberately not part of [`LobbyState`]: the tree is rebuilt whenever that
/// changes, so keeping the offsets there would rebuild sixty rows on every
/// notch of the wheel. Kept apart, adding a card rebuilds the list *and*
/// leaves it where the player was reading — which is the only reason they
/// scrolled there.
#[derive(Resource, Default)]
pub(crate) struct Scrolled {
    pool: f32,
    deck: f32,
    table: f32,
    gateways: f32,
    games: f32,
    library: f32,
    picker_panel: f32,
    settings: f32,
    transfer: f32,
    terms: f32,
    about: f32,
    profile_sheet: f32,
    settings_nav: f32,
}

impl Scrolled {
    pub(crate) fn get(&self, list: List) -> f32 {
        match list {
            List::Pool => self.pool,
            List::Deck => self.deck,
            List::Table => self.table,
            List::Gateways => self.gateways,
            List::Games => self.games,
            List::RoomCards | List::PickerSets => 0.0,
            List::Library => self.library,
            List::PickerPanel => self.picker_panel,
            List::Settings => self.settings,
            List::Transfer => self.transfer,
            List::Terms => self.terms,
            List::About => self.about,
            List::ProfileSheet => self.profile_sheet,
            List::SettingsNav => self.settings_nav,
        }
    }

    pub(crate) fn set(&mut self, list: List, at: f32) {
        match list {
            List::Pool => self.pool = at,
            List::Deck => self.deck = at,
            List::Table => self.table = at,
            List::Gateways => self.gateways = at,
            List::Games => self.games = at,
            List::RoomCards | List::PickerSets => {}
            List::Library => self.library = at,
            List::PickerPanel => self.picker_panel = at,
            List::Settings => self.settings = at,
            List::Transfer => self.transfer = at,
            List::Terms => self.terms = at,
            List::About => self.about = at,
            List::ProfileSheet => self.profile_sheet = at,
            List::SettingsNav => self.settings_nav = at,
        }
    }
}

/// Turns a wheel or a swipe into scrolling on the list under the pointer.
pub(super) fn scrolls(
    mut wheels: MessageReader<Pointer<Scroll>>,
    mut drags: MessageReader<Pointer<Drag>>,
    parents: Query<&ChildOf>,
    mut lists: Query<(&mut ScrollPosition, &ComputedNode, &Scrollable)>,
    mut memory: ResMut<Scrolled>,
    entrance: Res<super::entrance::Entrance>,
    journey: Option<Res<crate::arrival::Journey>>,
) {
    if journey.as_ref().is_some_and(|j| j.active()) || entrance.active() {
        wheels.clear();
        drags.clear();
        return;
    }
    for wheel in wheels.read() {
        let travel = match wheel.unit {
            MouseScrollUnit::Line => wheel.y * WHEEL_LINE,
            MouseScrollUnit::Pixel => wheel.y,
        };
        // A wheel pushed away from the reader moves the content up, which is
        // an *increase* in the scroll offset.
        scroll_lineage(wheel.entity, -travel, &parents, &mut lists, &mut memory);
    }
    for drag in drags.read() {
        // A finger drags the content itself, so it goes the other way again.
        scroll_lineage(
            drag.entity,
            -drag.delta.y,
            &parents,
            &mut lists,
            &mut memory,
        );
    }
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
