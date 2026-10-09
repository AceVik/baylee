//! The shelf itself: its layout, its spawn, the lifts, and the two strips hung off it.

#[allow(clippy::wildcard_imports)] // the ledge's shared vocabulary
use super::*;

/// Where the shelf put the middle of itself, for the drawer to stand over.
///
/// A resource rather than a second call to
/// [`baylee_client_core::ledge::arrange`], because arranging takes the widths
/// of all three columns and the drawer has no business measuring the shelf's
/// buttons. Two estimates of one number would also be two chances to be
/// wrong about it, and the middle's width is already an estimate —
/// see [`mid_width`].
///
/// Written whenever [`sync_ledge`] rebuilds, which is whenever it can change:
/// the arrangement is a function of the revision and the window.
#[derive(Resource)]
pub struct LedgeLayout {
    /// The centre of the middle column, in logical pixels from the left edge.
    pub(in crate::hud) mid_x: f32,
    /// The window it was measured in.
    pub(in crate::hud) window_w: i32,
}

impl Default for LedgeLayout {
    /// The middle of a window of the width `sync_ledge` assumes before it has
    /// seen one, so a drawer drawn before the first arrangement is centred
    /// rather than thrown to the left edge.
    fn default() -> Self {
        Self {
            mid_x: 600.0,
            window_w: 1200,
        }
    }
}

/// How far the middle of the shelf is from the middle of the window.
///
/// Doubled, because this is paid as *padding on one side*: a centred child in
/// a box padded by `p` on the left has its centre at `(p + w) / 2`, so `p` is
/// twice the slide. Positive means the middle sits right of centre.
pub(in crate::hud) fn mid_shift(mid_x: f32, window_w: i32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let half = window_w as f32 / 2.0;
    2.0 * (mid_x - half)
}

/// That slide as padding, on whichever side has to carry it.
///
/// The one place this arithmetic lives. The middle column and the drawer both
/// stand on the same centre, and a drawer that worked it out for itself would
/// be over the question until the day one of the two was adjusted.
pub(in crate::hud) fn mid_padding(mid_x: f32, window_w: i32) -> UiRect {
    let shift = mid_shift(mid_x, window_w);
    if shift >= 0.0 {
        UiRect::left(px(shift))
    } else {
        UiRect::right(px(-shift))
    }
}

/// The shelf itself: the one node in the overlay's retained tree that
/// **outlives a rebuild**.
///
/// [`super::HudRevision`] counts the hover, so `sync_overlay` tears its tree
/// down and builds it again whenever the pointer crosses a card — hundreds of
/// times a turn. Everything in that tree is content that follows the pointer
/// (the preview *is* the hover) or is cheap enough not to care. The shelf is
/// neither: it is the edge the window ends at, it is there at the first frame
/// and never goes, and the buttons on it carry a [`crate::ambience::Feel`]
/// whose warmth is state on the entity — a shelf rebuilt under the pointer
/// snaps the button the player is reaching for back to rest.
///
/// So `sync_overlay` keeps the root and this node across a rebuild and
/// despawns the rest, and the shelf's own contents answer to their own
/// revision. It is **not** a second root, and that is a measurement rather
/// than a preference: a root sorts wholesale against the others, and the
/// shelf has to stand *over* the table veil (`Z_VEIL`, the whole window) and
/// *under* the hover preview (`Z_PREVIEW`, which `hand::beside` may put over
/// the shelf when it describes a card near the bottom of the screen). Both
/// are children of [`HudRoot`], so the shelf has to be one too.
#[derive(Component)]
pub struct LedgeShelf;

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
/// Two more things that look like details and are not. The shelf is **no
/// longer opaque** — §3.3 said it was, and gave a reason ("a translucent edge
/// reads as a veil, not as a shelf"), and the owner reversed it on 14.09.2026:
/// *"Make them a little bit transparent with slow moving shader animation, so
/// it gets a little bit breathing live."* What makes that readable rather
/// than merely dimmer is the other half of the same instruction: the hand
/// zone's cloth now runs **behind** this node, so the shelf is composited over
/// a container and not over the sky. And its overflow is **visible**: the
/// drawer grows up out of it, and so do the two casts below, so a `clip()`
/// copied from the zone would leave a drawer nobody can see and a shelf that
/// stands off nothing.
pub(in crate::hud) fn spawn_ledge(
    commands: &mut Commands,
    cloth: Option<Handle<crate::frontal::FrontalMaterial>>,
) -> Entity {
    let shelf = commands
        .spawn((
            LedgeShelf,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(hand::HAND_ZONE_H - hand::LEDGE_H),
                left: px(0),
                right: px(0),
                height: px(hand::LEDGE_H),
                // The lip is paid for out of the **top** padding, which is why
                // this is not `UiRect::axes`. `BoxSizing::DEFAULT` is
                // `BorderBox`, so `LEDGE_H` is the outside of the shelf and
                // the border eats into it: 6 + 28 + 6 is 40 only if the line
                // is not there, and with it a 28-px button overflows by one.
                // Taking the pixel off the padding rather than adding it to
                // the shelf keeps the rule the whole height budget is built on
                // — every pixel of ledge is a pixel of table — and costs
                // nothing to look at, because 1 + 5 above the button reads as
                // the 6 below it.
                padding: UiRect::new(px(EDGE), px(EDGE), px(LEDGE_PAD_Y - LIP), px(LEDGE_PAD_Y)),
                // The lip is still paid for out of the border box, because the
                // padding arithmetic above is measured against it — but it is
                // *painted* by the cloth (`frontal.wgsl`, the top row of the
                // rail), because a `BorderColor` on a `MaterialNode` is a
                // question and the line is not optional.
                border: UiRect::top(px(LIP)),
                // The two corners the owner asked for on 14.09.2026, cut by
                // the cloth itself and stated here too so the headless
                // fallback is the same shape. It rounds the **top** only; the
                // other three corners of the controls bar are off the frame.
                border_radius: BorderRadius::top(px(crate::frontal::CORNER)),
                overflow: Overflow::visible(),
                ..default()
            },
            // Nothing of its own. The rail is a surface, and it is the same
            // cloth as the zone below — one dye, one pair of clocks, one fold
            // field indexed on the window's x, so the two read as one piece
            // with a rail across the top. Without a render world there is no
            // handle and the flat dye is drawn instead.
            match cloth {
                Some(_) => BackgroundColor(Color::NONE),
                None => BackgroundColor(palette::DOCK_GROUND.with_alpha(RAIL_FALLBACK)),
            },
            ZIndex(Z_LEDGE),
            // The shelf itself answers nothing and must not swallow a click
            // meant for the table — but its children are buttons, and a
            // button in a node the pointer cannot see is a button that cannot
            // be pressed. Hoverable, blocking nothing, exactly as the zone
            // below it is.
            //
            // `should_block_lower: false` does mean a click on the bare shelf
            // reaches whatever 3D geometry is behind it, which is not
            // obviously right. It is harmless today because the only thing
            // down there is the slab's margin and nothing on it is pickable;
            // if the layout ever puts a card under the shelf, this is the
            // line that has to change.
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
        ))
        .id();
    if let Some(handle) = cloth {
        commands
            .entity(shelf)
            .insert((MaterialNode(handle), crate::frontal::Hanging));
    }
    // The two casts, as children outside the shelf's own box rather than as a
    // `BoxShadow` on it. See `LIFT_UP_H` for why they had to stop being one.
    for cast in [lift_up(commands), lift_down(commands)] {
        commands.entity(shelf).add_child(cast);
    }
    shelf
}

/// What [`sync_ledge`] leaves alone: everything the shelf was spawned with.
///
/// One marker rather than two since the mana pool left this node for a strip
/// of its own ([`pool`]). It stays an alias, and stays a one-armed `Or`,
/// because the shelf has spawned something a rebuild must not touch twice
/// already and the next one should be a name added here rather than a filter
/// rewritten at the query.
pub(super) type Retained = Or<(With<LedgeCast>,)>;

/// One of the shelf's two casts, so the rebuild leaves them where they are.
///
/// They are spawned with the shelf and never change: an elevation is a fact
/// about the shelf and not about the question standing on it.
#[derive(Component, Clone, Copy)]
pub struct LedgeCast;

/// The shelf's cast on the table above it.
///
/// A child, `Pickable::IGNORE`, drawn entirely **outside** the shelf's
/// rectangle, which is the whole reason it is not a `ShadowStyle` any more.
///
/// The [`LIP`] in the offset is not a nudge. An absolutely-positioned child's
/// insets are measured from its containing block's **padding** box, and the
/// shelf carries `border: UiRect::top(px(LIP))` — so a bare `-LIFT_UP_H` put
/// this gradient's darkest end one pixel *inside* the bar, on top of the one
/// line the cloth paints. Measured at 3008 x 1630: the lip along the straight
/// top edge composited to (36, 31, 19) where the same lip round the corner,
/// below the cast, came out at `DIALOG_LINE`'s own (55, 48, 31) — the same
/// ratio, 0.65, on all three channels, which is a black veil and not a
/// different colour. A shadow an object casts must not fall on the object.
fn lift_up(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            LedgeCast,
            Node {
                position_type: PositionType::Absolute,
                top: px(-LIFT_UP_H - LIP),
                left: px(0),
                right: px(0),
                height: px(LIFT_UP_H),
                ..default()
            },
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::percent(palette::SHADOW.with_alpha(0.0), 0.0),
                ColorStop::percent(
                    palette::SHADOW.with_alpha(palette::SHADOW.alpha() * 0.18),
                    40.0,
                ),
                ColorStop::percent(
                    palette::SHADOW.with_alpha(palette::SHADOW.alpha() * 0.55),
                    70.0,
                ),
                ColorStop::percent(palette::SHADOW, 100.0),
            ])),
            Pickable::IGNORE,
        ))
        .id()
}

/// And its cast on the cards below, the lighter of the two.
///
/// The [`LIP`] comes off this one for the reason it goes onto [`lift_up`]:
/// both are measured from the padding box, so `LEDGE_H` alone would start the
/// cast a pixel below the shelf and leave an unshadowed line under it.
fn lift_down(commands: &mut Commands) -> Entity {
    let share = |f: f32| palette::SHADOW.with_alpha(palette::SHADOW.alpha() * LIFT_DOWN_SHARE * f);
    commands
        .spawn((
            LedgeCast,
            Node {
                position_type: PositionType::Absolute,
                top: px(hand::LEDGE_H - LIP),
                left: px(0),
                right: px(0),
                height: px(LIFT_DOWN_H),
                ..default()
            },
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::percent(share(1.0), 0.0),
                ColorStop::percent(share(0.48), 35.0),
                ColorStop::percent(share(0.12), 70.0),
                ColorStop::percent(share(0.0), 100.0),
            ])),
            Pickable::IGNORE,
        ))
        .id()
}

/// How far the shelf stands off the table above it, and how soft that cast is.
///
/// §3.3 asked for one shadow, downwards, and gave the reason: a card running
/// under an opaque bar reads as a card on a shelf rather than a card cut by a
/// line. The owner asked for the other one on 14.09.2026 — *"zum Tisch hin
/// als auch zur Hand hin (zur Hand etwas leichter)"* — and it is the same
/// argument turned round. A slab that casts one way is a lid; one that casts
/// both ways is standing off both, which is what a shelf at the edge of a
/// table is doing.
///
/// The table's side is the deeper of the two because it is the side that can
/// afford it. Above the shelf is open felt with nothing on it to compete
/// with; below it is a row of cards each carrying a glow that is a *rules*
/// statement, and a heavy cast there argues with the one light on this screen
/// a player is meant to read as information.
///
/// **Why these are heights and not offsets and blurs.** They were a
/// `BoxShadow` with two `ShadowStyle`s, and a `BoxShadow` is the node's own
/// rectangle offset and blurred — so the up-cast's rectangle is this shelf
/// shifted five pixels up and covers nearly the whole of its interior at
/// `SHADOW`'s full 0.55, and the down-cast covers it again from about ten
/// pixels down. All of that was *behind an opaque node and never seen*, which
/// was true right up until the owner asked for the shelf to be translucent on
/// 14.09.2026. On a translucent one it is roughly two thirds black across the
/// whole width, which is the removed panel arriving by the back door — the
/// same failure `frontal.rs`'s hem paid for once ("a hem at 0.97 under a cast
/// at 0.25 is 0.99 over the first sixteen pixels").
///
/// So each cast is a gradient child lying entirely outside the shelf's box:
/// nothing of either falls on the shelf, the up-cast still lands on the felt
/// and the down-cast still lands on the cards, and both still ride at
/// [`Z_LEDGE`] because they are children of the node that carries it. The
/// depths are what the old blurs reached — about fourteen pixels up and
/// twelve down — so the picture is the one §3.3 and AV1 measured.
const LIFT_UP_H: f32 = 14.0;
const LIFT_DOWN_H: f32 = 12.0;

/// What the hand's side of the cast is worth against the table's.
///
/// A share and not a second colour, so "somewhat lighter" cannot quietly stop
/// being true when [`palette::SHADOW`] is retuned — which is what the assert
/// below is for.
///
/// It was 0.45 while the two casts were one blurred rectangle each: the
/// up-cast's own lower edge was blurred too and about eleven pixels of it
/// landed on the hand, so the hand's side was being paid for twice and a
/// share of 0.70 measured *deeper* there than the single cast it replaced.
/// Neither cast reaches the other's side now, so the share is the plain
/// statement it was always meant to be and goes back to the 0.70 AV1 asked
/// for.
const LIFT_DOWN_SHARE: f32 = 0.70;
const _: () = assert!(LIFT_DOWN_SHARE < 1.0 && LIFT_DOWN_SHARE > 0.0);

/// How dense the shelf is where there is no render world to draw the cloth.
///
/// The cloth's own density at the rail, **read** from [`crate::frontal::RAIL`]
/// rather than restated, for the reason `hand::GROUND` gives: a fallback that
/// put the ground somewhere else would move every headless measurement with
/// it.
const RAIL_FALLBACK: f32 = crate::frontal::RAIL;

/// The air above and below a button on the shelf.
///
/// Twice this plus a button's [`BUTTON_H`] is [`hand::LEDGE_H`], and that
/// equation is the only reason either number is what it is. The [`LIP`] comes
/// out of the top of it rather than out of the shelf; the node says why.
pub(in crate::hud) const LEDGE_PAD_Y: f32 = 6.0;

/// The line along the top of the shelf, where the table stops.
pub(crate) const LIP: f32 = 1.0;

/// How tall anything a player presses on the shelf is.
///
/// A keycap with the button's own air around it: [`KEYCAP_SIDE`] is 1.9 and
/// [`CAP_PT`] is 10.5, so the cap is a 20-pixel square and `20 + 2 · 4` is
/// this. Written out rather than computed all the same, because the equation
/// below is the one that actually holds it — a cap drawn at another size
/// would have to leave the shelf's height alone, not move it.
///
/// Not the 44 a phone would ask for: this is a desktop table, the shelf is
/// 40 px of a window that would rather be table, and a 44-px target would take
/// another 16 px off the hand. A deliberate refusal, written down as one.
pub(in crate::hud) const BUTTON_H: f32 = 28.0;

/// The shelf's arithmetic, as one statement rather than four comments.
const _: () = assert!(LEDGE_PAD_Y * 2.0 + BUTTON_H == hand::LEDGE_H);

// ------------------------------------------------------ the two attachments
//
// [`players`] hangs off the shelf's left end and [`pool`] off its right
// (#264). The pair were the tray and the pool until the owner put the tray's
// doors into the bar on 25.09.2026, and the owner had asked for the pool in
// terms of the tray: *"Es soll symetrisch zum Tray aussehen nur auf der
// linken Seite"* (19.09.2026). Symmetry is a property of two things, so the
// numbers that decide the shape of a strip are here rather than in either of
// them — a copy in the second file would be symmetric on the day it was
// typed and only then. They were [`tray`]'s own until the second strip
// existed, which is why their reasons are written in the tray's terms.

/// A strip's height.
///
/// *"kleiner von der Höhe her"* than the shelf, and this is what that means
/// arithmetically: a [`BUTTON_H`] button loses the four pixels the shelf
/// spends on breathing room above and below its own row, and the strip is
/// that button plus a pixel of padding on each side. It comes out shorter
/// than [`hand::LEDGE_H`].
pub(super) const STRIP_H: f32 = BUTTON_H - 4.0 + 2.0 * STRIP_PAD;

/// A strip's own padding, inside its border.
const STRIP_PAD: f32 = 3.0;

/// How far a strip hangs *into* the shelf below it.
///
/// One pixel, so the strip's bottom border and the shelf's lip are one line
/// rather than two — the drawer overlaps by exactly the same amount and for
/// the same reason. It is a constant rather than a literal because the game
/// menu's panel hangs off the same edge ([`menu::root_node`]), and a `- 1.0`
/// written twice is a line that is right until somebody changes one of them.
pub(super) const STRIP_LIP: f32 = 1.0;

/// A strip's corner.
///
/// Rounded on the **top two** corners only and square at the bottom, which is
/// the rule the drawer's panel and the shelf itself already obey: a thing
/// growing out of the shelf is continuous with it at the join and finished
/// everywhere else.
pub(super) const STRIP_R: f32 = 4.0;

/// Which end of the shelf a strip hangs off.
///
/// Since #264 (the owner, 25.09.2026) the players stand at the left end and
/// the mana pool at the right, where the tray hung before its two doors went
/// into the bar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum StripSide {
    /// The players' ([`players`]).
    Left,
    /// The mana pool's.
    Right,
}

/// Where a strip hangs off the shelf, and how far in from the window's edge.
///
/// The one node both attachments spawn, so the two cannot drift apart in the
/// four ways that would be visible: a different height, a different overlap
/// with the lip, a different corner, or a different inset. The inset is
/// [`EDGE`] on **both** sides and is therefore not a parameter — that is the
/// whole of the symmetry, and a strip that could be given its own would be a
/// strip that could stop being symmetric without anybody editing this line.
pub(super) fn strip_node(side: StripSide) -> Node {
    let inset = px(EDGE);
    Node {
        position_type: PositionType::Absolute,
        // The drawer's overlap, and it has to be the drawer's exactly: the
        // three hang off one edge and a pixel between them would draw as a
        // step in the shelf's lip.
        bottom: px(hand::HAND_ZONE_H - STRIP_LIP),
        left: if side == StripSide::Left {
            inset
        } else {
            Val::Auto
        },
        right: if side == StripSide::Right {
            inset
        } else {
            Val::Auto
        },
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        height: px(STRIP_H),
        padding: UiRect::all(px(STRIP_PAD)),
        border: UiRect::new(px(1), px(1), px(1), px(0)),
        border_radius: BorderRadius::top(px(STRIP_R)),
        ..default()
    }
}
