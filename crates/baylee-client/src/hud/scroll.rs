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
/// is a design decision. Only a node that says both is scrolled. (A stack
/// entry's sentence box scrolls and is not one of these: it is reached
/// through its row, [`super::stack::StackTextRow`].)
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
/// In the hand the wheel is the hand's (it scrolls the row sideways), with
/// one exception (#289): an upright wheel over the hovered hand card scrolls
/// its preview while that preview's text runs over, which its scrollbar
/// shows. Text that fits leaves the wheel to the hand, so a mouse with one
/// wheel still scrolls the hand over nearly every card, and a sideways
/// wheel is always the hand's.
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

/// A panel that scrolls ([`Scrolls`]): neither a preview's text nor a stack
/// entry's sentence, which the borrow checker needs said.
type Panel = (
    With<Scrolls>,
    Without<crate::face::FaceTextBox>,
    Without<super::stack::StackTextBox>,
);

/// A stack entry's sentence box ([`super::stack::StackTextBox`]): neither a
/// panel nor a preview, which the borrow checker needs said.
type StackText = (
    With<super::stack::StackTextBox>,
    Without<Scrolls>,
    Without<crate::face::FaceTextBox>,
);

/// The stack entries whose sentence a wheel over them may scroll, and those
/// sentences' boxes.
#[derive(bevy::ecs::system::SystemParam)]
pub struct StackTexts<'w, 's> {
    rows: Query<'w, 's, &'static super::stack::StackTextRow>,
    boxes: Query<'w, 's, Scrolled, StackText>,
}

/// Turns a wheel into scrolling on whatever panel is under the pointer, or on
/// the preview of the table card under it ([`PreviewScroll`]).
#[allow(clippy::too_many_arguments)] // two targets a wheel can have, and their stores
pub fn scrolls(
    mut wheels: MessageReader<Pointer<Scroll>>,
    parents: Query<&ChildOf>,
    mut panels: Query<Scrolled, Panel>,
    hand: Query<(), With<HandScroll>>,
    hand_cards: Query<&HandCardVisual>,
    table: Query<&crate::table::CardVisual>,
    bars: Query<&crate::rowbar::RowBar>,
    mut previews: Query<Scrolled, With<crate::face::FaceTextBox>>,
    mut stack: StackTexts,
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
                // A sideways wheel over a card on the felt scrolls its row,
                // if the row scrolls (#298): the upright one is the
                // preview's.
                if wheel.x.abs() > wheel.y.abs() {
                    if let Some(board) = duel.board.as_ref()
                        && let Some((row, _)) =
                            baylee_client_core::rowscroll::row_of(board, card.object)
                    {
                        wheel_the_row(&mut duel, row, wheel.x, wheel.unit);
                    }
                    break;
                }
                // Upright: what a wheel over it scrolls is its preview's
                // text, and only while it is the card previewed. Text that
                // fits is left where it is, and nothing else takes the wheel
                // instead.
                if duel.hovered == Some(card.object) {
                    wheel_the_preview(&mut previews, &mut preview, card.object, travel);
                }
                break;
            }
            // A stack entry whose sentence runs over its box: that sentence
            // (the owner, 08.10.2026). The box is not pickable — it would
            // take the row's hover — so the wheel lands on the row, and the
            // row names its box. Ahead of the hand card's rule below, which
            // the row would otherwise meet first (it is a `HandCardVisual`)
            // and hand the wheel to the preview's text; and a sentence that
            // fits leaves the wheel to that rule and then to the list.
            if wheel.y.abs() >= wheel.x.abs()
                && let Ok(row) = stack.rows.get(entity)
                && let Ok((mut position, computed)) = stack.boxes.get_mut(row.text_box)
                && runs_over(computed)
            {
                position.y = scrolled(
                    position.y,
                    -travel,
                    computed.size().y,
                    computed.content_size().y,
                    computed.inverse_scale_factor(),
                );
                break;
            }
            // The hovered hand card, wheeled upright while its preview's
            // text runs over: that text (#289). Otherwise the walk goes on
            // up to the hand.
            if let Ok(card) = hand_cards.get(entity)
                && wheel.y.abs() >= wheel.x.abs()
                && duel.hovered == Some(card.object)
                && previews.iter().any(|(_, computed)| runs_over(computed))
            {
                wheel_the_preview(&mut previews, &mut preview, card.object, travel);
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
            if let Ok(bar) = bars.get(entity) {
                // A scrolled row's felt or its bar: either way of the
                // wheel scrolls the row.
                let axis = if wheel.x.abs() > wheel.y.abs() {
                    wheel.x
                } else {
                    wheel.y
                };
                wheel_the_row(&mut duel, bar.row, axis, wheel.unit);
                break;
            }
            if hand.contains(entity) {
                // A finished game's veil takes the wheel the way it takes
                // clicks: the hand under it stays where it is (#297).
                if duel.ending().is_some() {
                    break;
                }
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

/// Scrolls the preview of `object`, the card hovered, by `travel`, and keeps
/// the offset for a rebuilt preview. At either end of the text it moves
/// nothing, and nothing else takes the wheel.
fn wheel_the_preview(
    previews: &mut Query<Scrolled, With<crate::face::FaceTextBox>>,
    preview: &mut PreviewScroll,
    object: ObjectId,
    travel: f32,
) {
    for (mut position, computed) in previews {
        position.y = scrolled(
            position.y,
            -travel,
            computed.size().y,
            computed.content_size().y,
            computed.inverse_scale_factor(),
        );
        *preview = PreviewScroll {
            object: Some(object),
            offset: position.y,
        };
    }
}

/// Whether a text box holds more than it shows: the predicate that puts its
/// scrollbar up (`face::thumb`), so the wheel goes to the text exactly when
/// the bar says there is more of it.
fn runs_over(computed: &ComputedNode) -> bool {
    let scale = computed.inverse_scale_factor();
    crate::face::thumb(
        computed.size().y * scale,
        computed.content_size().y * scale,
        0.0,
    )
    .is_some()
}

/// Scrolls a battlefield row by one wheel gesture: a line is a card, and
/// pixels add up to one ([`baylee_client_core::rowscroll::ROW_STEP`]). A wheel
/// pushed away, or to the right, moves the row back towards its first card,
/// the way it moves the hand.
fn wheel_the_row(
    duel: &mut Duel,
    row: baylee_client_core::rowscroll::RowKey,
    axis: f32,
    unit: MouseScrollUnit,
) {
    let pixels = match unit {
        MouseScrollUnit::Line => axis * baylee_client_core::rowscroll::ROW_STEP,
        MouseScrollUnit::Pixel => axis,
    };
    let Duel {
        rows,
        board,
        layout,
        ..
    } = duel;
    if let (Some(board), Some(layout)) = (board.as_ref(), layout.as_ref()) {
        rows.wheel(board, layout, row, -pixels);
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
        aimed_both(entity, 0.0, y)
    }

    /// The same, turned `x` lines sideways as well.
    fn aimed_both(entity: Entity, x: f32, y: f32) -> Pointer<Scroll> {
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
                x,
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

    /// Once the game is over the veil lies over the hand, and a wheel there
    /// moves nothing behind it (#297).
    #[test]
    fn a_wheel_over_a_finished_game_s_hand_moves_nothing() {
        let mut app = app();
        app.world_mut().resource_mut::<Duel>().interaction =
            Some(baylee_client_core::Interaction::new(
                baylee_engine::choice::Pending::GameOver(baylee_engine::win::GameResult {
                    winner: None,
                    reason: baylee_engine::win::EndReason::Draw,
                }),
                baylee_core::ids::PlayerId::new(0),
            ));
        let bar = app.world_mut().spawn((Node::default(), HandScroll)).id();
        let card = app.world_mut().spawn((Node::default(), ChildOf(bar))).id();
        wheel(&mut app, card, -1.0);
        assert!(
            app.world().resource::<Duel>().hand_scroll.abs() < f32::EPSILON,
            "the hand scrolled under a finished game's veil"
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

    /// The hovered hand card, in its bar, with its preview holding `content`
    /// pixels of text in a box 100 deep.
    fn hovered_hand_card(app: &mut App, content: f32) -> (Entity, Entity) {
        let object = ObjectId::new(9, 0);
        let bar = app.world_mut().spawn((Node::default(), HandScroll)).id();
        let card = app
            .world_mut()
            .spawn((Node::default(), HandCardVisual { object }, ChildOf(bar)))
            .id();
        let text = preview_text(app, content);
        app.world_mut().resource_mut::<Duel>().hovered = Some(object);
        app.update();
        (card, text)
    }

    /// (b2, #289) The hovered hand card's preview runs over: an upright
    /// wheel scrolls its text, and the hand stays where it is; a sideways
    /// one is still the hand's.
    #[test]
    fn an_upright_wheel_over_a_hand_card_whose_text_runs_over_scrolls_the_text() {
        let mut app = app();
        let (card, text) = hovered_hand_card(&mut app, 400.0);
        wheel(&mut app, card, -1.0);
        assert!(offset(&app, text) > 0.0, "the preview's text moved");
        assert!(
            app.world().resource::<Duel>().hand_scroll.abs() < f32::EPSILON,
            "the hand did not"
        );
        let kept = offset(&app, text);
        app.world_mut()
            .resource_mut::<Messages<Pointer<Scroll>>>()
            .write(aimed_both(card, -1.0, 0.0));
        app.update();
        assert!(app.world().resource::<Duel>().hand_scroll > 0.0, "sideways");
        assert!((offset(&app, text) - kept).abs() < f32::EPSILON);
    }

    /// (b3, #289) Text that fits leaves the upright wheel to the hand, so a
    /// one-wheel mouse scrolls the hand over nearly every card.
    #[test]
    fn a_wheel_over_a_hand_card_whose_text_fits_scrolls_the_hand() {
        let mut app = app();
        let (card, text) = hovered_hand_card(&mut app, 100.0);
        wheel(&mut app, card, -1.0);
        assert!(app.world().resource::<Duel>().hand_scroll > 0.0);
        assert!(offset(&app, text).abs() < f32::EPSILON);
    }

    /// The stack panel's list, a full row in it standing for `object`, and
    /// the row's sentence box holding `content` pixels of sentence in a box
    /// 63 deep. Hovered, with the row's preview up and running over too: the
    /// rule the box has to win against.
    fn stack_row(app: &mut App, content: f32) -> (Entity, Entity, Entity, Entity) {
        let object = ObjectId::new(30, 0);
        let list = app
            .world_mut()
            .spawn((
                Node::default(),
                Scrolls,
                ScrollPosition::default(),
                ComputedNode {
                    size: Vec2::new(330.0, 300.0),
                    content_size: Vec2::new(330.0, 900.0),
                    ..default()
                },
            ))
            .id();
        let text_box = app
            .world_mut()
            .spawn((
                Node::default(),
                super::super::stack::StackTextBox { object },
                ScrollPosition::default(),
                ComputedNode {
                    size: Vec2::new(220.0, 63.0),
                    content_size: Vec2::new(220.0, content),
                    ..default()
                },
            ))
            .id();
        let row = app
            .world_mut()
            .spawn((
                Node::default(),
                HandCardVisual { object },
                super::super::stack::StackTextRow { text_box },
                ChildOf(list),
            ))
            .id();
        app.world_mut().entity_mut(text_box).insert(ChildOf(row));
        let preview = preview_text(app, 400.0);
        app.world_mut().resource_mut::<Duel>().hovered = Some(object);
        app.update();
        (list, row, text_box, preview)
    }

    /// The owner's (08.10.2026): over a stack entry whose sentence runs over
    /// its box, the wheel scrolls that sentence — not the list it is in,
    /// not the hovered entry's preview, not the hand.
    #[test]
    fn a_wheel_over_a_stack_entry_whose_sentence_runs_over_scrolls_the_sentence() {
        let mut app = app();
        let (list, row, text_box, preview) = stack_row(&mut app, 200.0);
        hand_card(&mut app);
        wheel(&mut app, row, -1.0);
        assert!(offset(&app, text_box) > 0.0, "the sentence moved");
        assert!(offset(&app, list).abs() < f32::EPSILON, "the list did not");
        assert!(
            offset(&app, preview).abs() < f32::EPSILON,
            "nor the preview"
        );
        assert!(app.world().resource::<Duel>().hand_scroll.abs() < f32::EPSILON);
        // And no further than its end: the wheel is swallowed there, as a
        // list's is, rather than handed to the list behind it.
        wheel(&mut app, row, -50.0);
        let end = 200.0 - 63.0;
        assert!((offset(&app, text_box) - end).abs() < 0.01);
        assert!(offset(&app, list).abs() < f32::EPSILON);
    }

    /// A sentence that fits leaves the wheel to what had it before: the
    /// hovered entry's preview while that runs over, and otherwise the list.
    #[test]
    fn a_wheel_over_a_stack_entry_whose_sentence_fits_is_not_the_sentence_s() {
        let mut app = app();
        let (list, row, text_box, preview) = stack_row(&mut app, 40.0);
        wheel(&mut app, row, -1.0);
        assert!(offset(&app, text_box).abs() < f32::EPSILON);
        assert!(offset(&app, preview) > 0.0, "the preview's, as before");
        app.world_mut().resource_mut::<Duel>().hovered = None;
        app.update();
        wheel(&mut app, row, -1.0);
        assert!(offset(&app, text_box).abs() < f32::EPSILON);
        assert!(offset(&app, list) > 0.0, "the list's");
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

    /// Where the local seat's land row of a table of long rows starts.
    fn first_land(app: &App) -> usize {
        use baylee_client_core::layout::LaneKind;
        app.world()
            .resource::<Duel>()
            .rows
            .first((baylee_core::ids::PlayerId::new(0), LaneKind::Lands))
    }

    /// A row that scrolls, under a card of it: the wheel turned sideways
    /// moves the row a card a line, and the upright wheel, which is the
    /// card's preview's (#259), leaves the row alone (#298).
    #[test]
    fn a_sideways_wheel_over_a_card_scrolls_its_row_and_an_upright_one_does_not() {
        let mut app = app();
        app.insert_resource(crate::rowbar::tests::long_rows(2, 80));
        let card = app
            .world_mut()
            .spawn(crate::table::CardVisual {
                object: ObjectId::new(2, 0),
                count: 4,
            })
            .id();
        wheel(&mut app, card, -1.0);
        assert_eq!(first_land(&app), 0, "the upright wheel moved the row");
        app.world_mut()
            .resource_mut::<Messages<Pointer<Scroll>>>()
            .write(aimed_both(card, -2.0, 0.0));
        app.update();
        assert_eq!(first_land(&app), 2, "two lines sideways are two cards");
    }

    /// Over the row's felt or its bar, either way of the wheel scrolls it.
    #[test]
    fn a_wheel_over_a_scrolled_rows_felt_scrolls_it_either_way() {
        use crate::rowbar::{Part, RowBar};
        let mut app = app();
        app.insert_resource(crate::rowbar::tests::long_rows(2, 80));
        let pad = app
            .world_mut()
            .spawn(RowBar {
                row: (
                    baylee_core::ids::PlayerId::new(0),
                    baylee_client_core::layout::LaneKind::Lands,
                ),
                part: Part::Pad,
            })
            .id();
        wheel(&mut app, pad, -1.0);
        assert_eq!(first_land(&app), 1, "upright");
        app.world_mut()
            .resource_mut::<Messages<Pointer<Scroll>>>()
            .write(aimed_both(pad, -1.0, 0.0));
        app.update();
        assert_eq!(first_land(&app), 2, "sideways");
        wheel(&mut app, pad, 5.0);
        assert_eq!(first_land(&app), 0, "back, and no further than the start");
    }
}
