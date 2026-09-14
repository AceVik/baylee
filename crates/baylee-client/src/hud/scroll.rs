//! Who owns a wheel.
//!
//! The lobby has had this since it had lists ([`crate::lobby::systems`]);
//! the table never did. `Overflow::scroll_y` only *clips* — Bevy moves the
//! content when [`ScrollPosition`] changes and nothing changes it on its own
//! — so the zone browser's card grid carried an overflow, a comment about
//! `Pointer<Scroll>`, and no way to reach the second row of a library. The
//! wheel fell through to [`crate::input::camera_controls`] and zoomed the
//! table instead, which is the owner's report exactly: *"oft möchte ich
//! eigentlich nur irgendwo was scrollen, auf einmal verschiebe ich den
//! Zoom"*.
//!
//! # The node decides, never a rectangle
//!
//! Which gesture belongs to a panel used to be a hard-coded strip at the
//! bottom of the window — the hand zone's height plus twenty pixels — so
//! every other panel was camera by construction and a moved hand zone would
//! have been wrong in silence. It is the **node under the pointer** now: the
//! walk below climbs from whatever the wheel landed on until it meets
//! something that scrolls.
//!
//! # Nothing to arbitrate any more
//!
//! There was a referee beside that walk — `wheel_is_the_interfaces`, which
//! `camera_controls` asked before zooming — and it was not enough. The owner
//! reported the two still fighting (*„mit dem Rad scrollen scheint sich mit
//! dem Kamera Zoom-In/Out zu streiten"*) and decided the argument by taking a
//! side: *„Das Zoom in/out sollte eh weg!"*. So the camera does not zoom, the
//! referee is gone, and a wheel has exactly one possible claimant.
//!
//! One consequence survives the removal and is still deliberate: a list at
//! its end **swallows** the wheel rather than handing it on. Scroll chaining
//! would put a gesture that ran out of rows onto whatever is behind the
//! panel, and a player who has hit the bottom of a library is not asking
//! about anything else.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::{Pointer, Scroll};
use bevy::ui::ScrollPosition;

/// What one line of wheel travel moves a panel, in logical pixels.
///
/// The lobby's number, because the two are the same gesture on the same
/// mouse and a player who scrolls a deck list and then a graveyard should
/// not meet two speeds.
const WHEEL_LINE: f32 = 32.0;

/// What one line of wheel travel moves the hand, in logical pixels.
///
/// Larger than a list's line: the hand scrolls **sideways** past cards a
/// hundred pixels wide, so a list's step would take a dozen notches to reach
/// the next card. This is the number the bottom-of-the-window rectangle used
/// before the node took over, kept so the gesture feels as it did.
const HAND_LINE: f32 = 60.0;

/// A panel that scrolls its own contents.
///
/// The marker is the *statement*, and it sits beside a [`ScrollPosition`]:
/// `Overflow::scroll_y` alone clips, and a node that clips without scrolling
/// is a design decision (the stack panel counts what does not fit rather
/// than scrolling it). Only a node that says both is scrolled.
#[derive(Component, Clone, Copy, Default, Debug)]
pub struct Scrolls;

/// The hand zone, which scrolls sideways and not through [`ScrollPosition`].
///
/// The hand is laid out by hand — `apply_hand_scroll` writes the strip's
/// margin from `Duel::hand_scroll`, because the row also has to lead, trail
/// and follow a hovered card — so the wheel writes that offset and the
/// layout does the rest.
#[derive(Component, Clone, Copy, Default, Debug)]
pub struct HandScroll;

/// Turns a wheel into scrolling on whatever panel is under the pointer.
pub fn scrolls(
    mut wheels: MessageReader<Pointer<Scroll>>,
    parents: Query<&ChildOf>,
    mut panels: Query<(&mut ScrollPosition, &ComputedNode), With<Scrolls>>,
    hand: Query<(), With<HandScroll>>,
    mut duel: ResMut<Duel>,
) {
    for wheel in wheels.read() {
        let travel = match wheel.unit {
            MouseScrollUnit::Line => wheel.y * WHEEL_LINE,
            MouseScrollUnit::Pixel => wheel.y,
        };
        let mut current = Some(wheel.entity);
        // Eight is the lobby's depth and the reason is the same: a row sits
        // a few nodes inside the list it belongs to, and a walk with no
        // bound would follow a cycle if one ever existed.
        for _ in 0..8 {
            let Some(entity) = current else { break };
            if let Ok((mut position, computed)) = panels.get_mut(entity) {
                // A wheel pushed away from the reader moves the content up,
                // which is an *increase* in the offset.
                position.y = scrolled(
                    position.y,
                    -travel,
                    computed.size().y,
                    computed.content_size().y,
                    computed.inverse_scale_factor(),
                );
                break;
            }
            if hand.contains(entity) {
                // Clamped by `apply_hand_scroll` against the layout it just
                // measured, which is the only place the row's real width is
                // known.
                duel.hand_scroll = (duel.hand_scroll - wheel.y * HAND_LINE).max(0.0);
                break;
            }
            current = parents.get(entity).ok().map(ChildOf::parent);
        }
    }
}

/// Where a list ends up after a gesture.
///
/// Bevy clamps what it *draws* but leaves [`ScrollPosition`] alone, so an
/// offset past the end would have to be unwound before the list moved again —
/// a swipe that ran off the bottom would then need the same distance back
/// before anything happened. The two sizes are physical pixels and the offset
/// is logical, which is what `scale` (a `ComputedNode`'s inverse scale factor)
/// converts between.
///
/// It was the lobby's, in `lobby::systems`, back when the lobby was the only
/// screen with a list in it. It is here because the table has one now, and
/// two clamps would be two answers to "did that swipe reach the end".
pub(crate) fn scrolled(from: f32, by: f32, view: f32, content: f32, scale: f32) -> f32 {
    let room = (content - view).max(0.0) * scale;
    (from + by).clamp(0.0, room)
}

// There was a referee here — `wheel_is_the_interfaces`, answering whose wheel
// this frame's was by asking whether it had landed on a UI node. It is gone
// with the thing it refereed against: the camera does not zoom any more, so a
// wheel has one possible claimant and nothing to arbitrate.

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::picking::pointer::{Location, PointerId};

    /// A wheel aimed at one entity. The location is required of the message
    /// and read by nothing here.
    fn aimed(entity: Entity, y: f32) -> Pointer<Scroll> {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::window::WindowRef;
        Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::Window(
                    WindowRef::Primary
                        .normalize(Some(Entity::PLACEHOLDER))
                        .expect("a window reference"),
                ),
                position: Vec2::ZERO,
            },
            Scroll {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                // What a mouse always sends (`Scroll::phase`'s own doc).
                phase: bevy::input::touch::TouchPhase::Moved,
            },
            entity,
        )
    }

    /// An app with the one system and the one resource it writes.
    fn app() -> App {
        let mut app = App::new();
        app.add_message::<Pointer<Scroll>>();
        app.init_resource::<Duel>();
        app.add_systems(Update, scrolls);
        app
    }

    fn wheel(app: &mut App, entity: Entity, y: f32) {
        app.world_mut()
            .resource_mut::<Messages<Pointer<Scroll>>>()
            .write(aimed(entity, y));
        app.update();
    }

    /// The zone browser's own report: a library of sixty cards ended at the
    /// bottom of the sheet, and the wheel that should have reached the rest
    /// zoomed the table instead. Layout never runs in a test, so the panel
    /// is told how big it and its contents are.
    #[test]
    fn a_wheel_over_a_panel_scrolls_the_panel() {
        let mut app = app();
        let panel = app
            .world_mut()
            .spawn((
                Node::default(),
                Scrolls,
                ScrollPosition::default(),
                ComputedNode {
                    size: Vec2::new(300.0, 300.0),
                    content_size: Vec2::new(300.0, 900.0),
                    ..default()
                },
            ))
            .id();
        wheel(&mut app, panel, -1.0);
        let at = app
            .world()
            .entity(panel)
            .get::<ScrollPosition>()
            .expect("the panel keeps its offset")
            .y;
        assert!(at > 0.0, "a wheel pulled towards the reader moved the list");
    }

    /// The gesture lands on a *row*, not on the list: the walk up the
    /// ancestry is what makes a wheel over a card scroll the grid the card
    /// is in.
    #[test]
    fn a_wheel_over_a_row_scrolls_the_list_the_row_is_in() {
        let mut app = app();
        let panel = app
            .world_mut()
            .spawn((
                Node::default(),
                Scrolls,
                ScrollPosition::default(),
                ComputedNode {
                    size: Vec2::new(300.0, 300.0),
                    content_size: Vec2::new(300.0, 900.0),
                    ..default()
                },
            ))
            .id();
        let row = app
            .world_mut()
            .spawn((Node::default(), ChildOf(panel)))
            .id();
        wheel(&mut app, row, -1.0);
        assert!(
            app.world()
                .entity(panel)
                .get::<ScrollPosition>()
                .expect("the panel keeps its offset")
                .y
                > 0.0
        );
    }

    /// The hand is the other half, and it does not go through
    /// [`ScrollPosition`]: `apply_hand_scroll` lays the row out from
    /// `Duel::hand_scroll`, so the wheel writes that.
    #[test]
    fn a_wheel_over_the_hand_bar_scrolls_the_hand() {
        let mut app = app();
        let bar = app.world_mut().spawn((Node::default(), HandScroll)).id();
        let card = app.world_mut().spawn((Node::default(), ChildOf(bar))).id();
        wheel(&mut app, card, -1.0);
        assert!(
            app.world().resource::<Duel>().hand_scroll > 0.0,
            "the card's own bar took the wheel"
        );
    }

    /// The counter-test, and the whole point of the change: a wheel that
    /// reaches nothing the interface owns is left alone here — it is the
    /// camera's, and `camera_controls` is what will read it.
    #[test]
    fn a_wheel_over_the_table_is_left_for_the_camera() {
        let mut app = app();
        let card_on_the_table = app.world_mut().spawn_empty().id();
        wheel(&mut app, card_on_the_table, -1.0);
        assert!(
            app.world().resource::<Duel>().hand_scroll.abs() < f32::EPSILON,
            "nothing in the interface claimed it"
        );
    }
}
