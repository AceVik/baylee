//! Who owns a wheel.
//!
//! The lobby has had this since it had lists ([`crate::lobby::systems`]);
//! the table never did. `Overflow::scroll_y` only *clips* — Bevy moves the
//! content when [`ScrollPosition`] changes and nothing changes it on its own
//! — so the zone browser's card grid carried an overflow, a comment about
//! `Pointer<Scroll>`, and no way to reach the second row of a library. The
//! wheel fell through to `input::camera_controls` and zoomed the
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

/// How far the preview's rules text has been scrolled, and whose it is.
///
/// The preview is a tooltip that follows the hovered card and is never under
/// the pointer itself, so no wheel reaches its text box by the walk in
/// [`scrolls`]. The rule for it (#259): the wheel scrolls what is under the
/// pointer, and on the table a card's readable surface is its preview. So a
/// wheel over the hovered table card scrolls that card's preview. The offset
/// is kept here because the overlay rebuilds the preview on its own schedule,
/// and a rebuilt text box would start at its top.
///
/// The hand keeps its wheel: a hovered hand card's preview does not scroll,
/// and a text that still runs over at ten pixels shows its scrollbar there
/// and is read larger, on the table or in the deckbuilder.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct PreviewScroll {
    /// The card whose preview it is.
    pub object: Option<ObjectId>,
    /// How far it has scrolled, in logical pixels.
    pub offset: f32,
}

/// Starts a preview's text at its top again whenever the hover moves to
/// another card, or off every card.
pub fn follow_the_hover(duel: Res<Duel>, mut preview: ResMut<PreviewScroll>) {
    if preview.object != duel.hovered {
        *preview = PreviewScroll {
            object: duel.hovered,
            offset: 0.0,
        };
    }
}

/// Stands a rebuilt preview's text where it had been scrolled to.
pub fn keep_the_preview_scrolled(
    preview: Res<PreviewScroll>,
    mut boxes: Query<&mut ScrollPosition, Added<crate::face::FaceTextBox>>,
) {
    for mut position in &mut boxes {
        position.y = preview.offset;
    }
}

/// A box a wheel moves: how far it has scrolled, and how big it and its
/// contents are.
type Scrolled = (&'static mut ScrollPosition, &'static ComputedNode);

/// Turns a wheel into scrolling on whatever panel is under the pointer, or on
/// the preview of the table card under it ([`PreviewScroll`]).
#[allow(clippy::too_many_arguments)] // two targets a wheel can have, and their stores
pub fn scrolls(
    mut wheels: MessageReader<Pointer<Scroll>>,
    parents: Query<&ChildOf>,
    mut panels: Query<Scrolled, (With<Scrolls>, Without<crate::face::FaceTextBox>)>,
    hand: Query<(), With<HandScroll>>,
    table: Query<&crate::table::CardVisual>,
    mut previews: Query<Scrolled, With<crate::face::FaceTextBox>>,
    mut duel: ResMut<Duel>,
    mut preview: ResMut<PreviewScroll>,
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
            if let Ok(card) = table.get(entity) {
                // A card on the felt: what a wheel over it scrolls is its
                // preview's text, and only while it is the card previewed.
                // Text that fits is left where it is, and nothing else takes
                // the wheel instead.
                if duel.hovered == Some(card.object) {
                    for (mut position, computed) in &mut previews {
                        position.y = scrolled(
                            position.y,
                            -travel,
                            computed.size().y,
                            computed.content_size().y,
                            computed.inverse_scale_factor(),
                        );
                        *preview = PreviewScroll {
                            object: Some(card.object),
                            offset: position.y,
                        };
                    }
                }
                break;
            }
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
                let axis = if wheel.x.abs() > wheel.y.abs() {
                    wheel.x
                } else {
                    wheel.y
                };
                let delta = match wheel.unit {
                    MouseScrollUnit::Line => axis * HAND_LINE,
                    MouseScrollUnit::Pixel => axis,
                };
                duel.hand_scroll = (duel.hand_scroll - delta).max(0.0);
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

    /// An app with the wheel's systems and the resources they write, in the
    /// client's order.
    fn app() -> App {
        let mut app = App::new();
        app.add_message::<Pointer<Scroll>>();
        app.init_resource::<Duel>();
        app.init_resource::<PreviewScroll>();
        app.add_systems(
            Update,
            (follow_the_hover, scrolls, keep_the_preview_scrolled).chain(),
        );
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

    /// A table card, hovered, and the text box of its preview: `content`
    /// pixels of text in a box 100 deep. Layout never runs in a test, so the
    /// box is told both.
    fn previewed(app: &mut App, content: f32) -> (Entity, Entity) {
        let object = ObjectId::new(7, 0);
        let card = app
            .world_mut()
            .spawn(crate::table::CardVisual { object, count: 1 })
            .id();
        let text = preview_text(app, content);
        app.world_mut().resource_mut::<Duel>().hovered = Some(object);
        // A frame with the preview up before any wheel, as in the client. On
        // its first frame the box is `Added` and `keep_the_preview_scrolled`
        // stands it at the kept offset, which would undo a wheel that moved
        // it without being kept and let a test pass on it.
        app.update();
        (card, text)
    }

    fn preview_text(app: &mut App, content: f32) -> Entity {
        app.world_mut()
            .spawn((
                Node::default(),
                crate::face::FaceTextBox,
                ScrollPosition::default(),
                ComputedNode {
                    size: Vec2::new(200.0, 100.0),
                    content_size: Vec2::new(200.0, content),
                    ..default()
                },
            ))
            .id()
    }

    fn offset(app: &App, text: Entity) -> f32 {
        app.world()
            .entity(text)
            .get::<ScrollPosition>()
            .expect("the box keeps its offset")
            .y
    }

    /// A hand bar with one card in it.
    fn hand_card(app: &mut App) -> Entity {
        let bar = app.world_mut().spawn((Node::default(), HandScroll)).id();
        app.world_mut().spawn((Node::default(), ChildOf(bar))).id()
    }

    /// (a) The preview is never under the pointer, so the card it previews
    /// takes the wheel for it: on the table a card's readable surface is its
    /// preview (#259). The hand does not move.
    #[test]
    fn a_wheel_over_a_previewed_table_card_scrolls_its_text() {
        let mut app = app();
        let (card, text) = previewed(&mut app, 400.0);
        wheel(&mut app, card, -1.0);
        let at = offset(&app, text);
        assert!(at > 0.0, "the preview's text moved");
        assert!(
            (app.world().resource::<PreviewScroll>().offset - at).abs() < f32::EPSILON,
            "and the offset is kept for a rebuilt preview"
        );
        assert!(
            app.world().resource::<Duel>().hand_scroll.abs() < f32::EPSILON,
            "the hand did not move"
        );
    }

    /// (b) The hand keeps its wheel, whatever the preview beside it holds.
    #[test]
    fn a_wheel_over_a_hand_card_scrolls_the_hand_and_not_the_text() {
        let mut app = app();
        let (_, text) = previewed(&mut app, 400.0);
        let card = hand_card(&mut app);
        wheel(&mut app, card, -1.0);
        assert!(app.world().resource::<Duel>().hand_scroll > 0.0);
        assert!(offset(&app, text).abs() < f32::EPSILON, "the text stayed");
    }

    /// (c) A card whose text fits leaves the wheel inert: nothing moves,
    /// and nothing else takes it.
    #[test]
    fn a_wheel_over_a_table_card_whose_text_fits_changes_nothing() {
        let mut app = app();
        let (card, text) = previewed(&mut app, 100.0);
        hand_card(&mut app);
        wheel(&mut app, card, -1.0);
        assert!(offset(&app, text).abs() < f32::EPSILON);
        assert!(app.world().resource::<PreviewScroll>().offset.abs() < f32::EPSILON);
        assert!(app.world().resource::<Duel>().hand_scroll.abs() < f32::EPSILON);
    }

    /// (d) The offset is the previewed card's: a rebuilt preview of the same
    /// card stands where it was scrolled to, and the preview of the next card
    /// starts at its top.
    #[test]
    fn the_offset_resets_when_the_hover_changes() {
        let mut app = app();
        let (card, _) = previewed(&mut app, 400.0);
        wheel(&mut app, card, -1.0);
        let kept = app.world().resource::<PreviewScroll>().offset;
        assert!(kept > 0.0);

        let rebuilt = preview_text(&mut app, 400.0);
        app.update();
        assert!(
            (offset(&app, rebuilt) - kept).abs() < f32::EPSILON,
            "a rebuilt preview of the same card"
        );

        app.world_mut().resource_mut::<Duel>().hovered = Some(ObjectId::new(8, 0));
        app.update();
        assert!(app.world().resource::<PreviewScroll>().offset.abs() < f32::EPSILON);
        let next = preview_text(&mut app, 400.0);
        app.update();
        assert!(offset(&app, next).abs() < f32::EPSILON, "the next card's");
    }

    /// And a card the pointer is on but whose preview is not up — hovered
    /// elsewhere, or not yet — scrolls nothing.
    #[test]
    fn a_wheel_over_a_card_that_is_not_previewed_scrolls_nothing() {
        let mut app = app();
        let (card, text) = previewed(&mut app, 400.0);
        app.world_mut().resource_mut::<Duel>().hovered = None;
        wheel(&mut app, card, -1.0);
        assert!(offset(&app, text).abs() < f32::EPSILON);
    }

    /// The counter-test, and the whole point of the change: a wheel that
    /// reaches nothing the interface owns is left alone here.
    ///
    /// It used to be the camera's, and since 14.09.2026 it is nobody's — the
    /// owner had the general camera movement removed. That makes this test
    /// *more* load-bearing rather than less: the walk has to stop at the last
    /// scrolling ancestor, and a walk that claimed everything would now be
    /// claiming it for a panel that is not under the pointer.
    #[test]
    fn a_wheel_over_the_table_is_claimed_by_nothing() {
        let mut app = app();
        let card_on_the_table = app.world_mut().spawn_empty().id();
        wheel(&mut app, card_on_the_table, -1.0);
        assert!(
            app.world().resource::<Duel>().hand_scroll.abs() < f32::EPSILON,
            "nothing in the interface claimed it"
        );
    }
}
