//! The ledge: the top edge of the hand zone, and the place a player answers
//! the game from.
//!
//! It is named for `tabletop::MAT_LEDGE`, the shoulder of a seat's mat that
//! the seat bar is written along. The hand zone gets the same shoulder, at
//! its top — the mana pool at one end, the engine's question and its answers
//! in the middle, the two ways to leave the game at the other. Nothing here
//! floats: the four things that used to hover over the table are one edge.
//!
//! This file holds the shelf. Its three columns arrive in their own steps,
//! and it is deliberately empty until they do — a shelf with nothing on it is
//! still the edge the camera is framed against, and framing it first is what
//! keeps every later measurement against the same picture.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// Spawns the shelf: opaque, full width, [`hand::LEDGE_H`] tall, standing on
/// the top of the hand zone.
///
/// **A sibling of the zone and not a child of it**, which is the one thing
/// about this node that is not free to change. `ZIndex` counts among
/// siblings, and the zone sits at [`Z_HAND`] — under the table veil the zone
/// dialog paints over the whole window at [`Z_VEIL`], deliberately, because
/// the hand cannot answer what that dialog is asking. The question *can*, and
/// a question drawn dimmed is a question the player is being told not to
/// answer. The slip stood above that veil for exactly this reason and the
/// ledge inherits its place.
///
/// Two more things that look like details and are not. The shelf is
/// **opaque**: it is the edge the table ends at, and a translucent one reads
/// as another veil rather than as a shelf. And its overflow is **visible**:
/// the drawer grows up out of it, so a `clip()` copied from the zone below
/// would leave a drawer nobody can see.
pub(super) fn spawn_ledge(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(hand::HAND_ZONE_H - hand::LEDGE_H),
                left: px(0),
                right: px(0),
                height: px(hand::LEDGE_H),
                padding: UiRect::axes(px(EDGE), px(LEDGE_PAD_Y)),
                // The lip: one line along the top and nothing down the sides
                // or under it, because the zone runs on to the window's own
                // edges and an edge has no corners.
                border: UiRect::top(px(1)),
                overflow: Overflow::visible(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            // Downwards, onto the cards. The comment on the zone forbids a
            // shadow there and means it — a shadow is drawn from a node's
            // rectangle, so a transparent node with one lays a hard band
            // across the table. This node is opaque and its rectangle *is*
            // the shelf, so the shadow falls where a shelf's shadow falls and
            // is what turns a card running under it into a card on a shelf
            // rather than a card cut by a line.
            BoxShadow(vec![ShadowStyle {
                color: palette::SHADOW,
                x_offset: px(0.0),
                y_offset: px(4.0),
                spread_radius: px(0.0),
                blur_radius: px(12.0),
            }]),
            ZIndex(Z_LEDGE),
            // The shelf itself answers nothing and must not swallow a click
            // meant for the table — but its children are buttons, and a
            // button in a node the pointer cannot see is a button that cannot
            // be pressed. Hoverable, blocking nothing, exactly as the zone
            // below it is.
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
        ))
        .id()
}

/// The air above and below a button on the shelf.
///
/// Twice this plus a button's 28 is [`hand::LEDGE_H`], and that equation is
/// the only reason either number is what it is.
pub(super) const LEDGE_PAD_Y: f32 = 6.0;
