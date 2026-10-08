//! A seat's plate at its mat's edge, its steps at the band's right end, and
//! the turn number in the dial. Ink stays upright while its anchors follow
//! the table projection, including opposing seats.
//!
//! # The plate (the owner's requests of 08.10.2026)
//!
//! *"Make it cleaner … and move it closer to the table edge (the mat's outer
//! edge / the table rim, per arrangement). Clicking it means targeting that
//! player … NO camera control from this plate."* The plate is the seat's
//! name and life on its first line, its hand, library, graveyard, exile and
//! counters on the second, and on a third — only while there is any — the
//! mana floating in its pool (`client_core::seatplate`, which the players'
//! strip reads too). A crown stands before the monarch's name and an ∞ after
//! a hand no maximum size applies to.
//!
//! **Where.** Flush with the seat's battlefield: on the mat's drawn edge on
//! the hearth side, at the seat's own left corner ([`plate_on`]), sliding
//! along that edge only where the window, the HUD's corners, the strips and
//! the hand, another seat's place or a plate already placed are in the way.
//! The steps went to the band's right end, so the middle of the table is the
//! dial's.
//!
//! **The plate is one control.** Everything written on it is
//! `Pickable::IGNORE`, so the plate itself is what the pointer hovers and
//! presses (a label that took the hover would leave the plate cold under the
//! pointer). It is a [`PlateTab`], not a `PlayerTab`: a press chooses the
//! seat while a question can target it and otherwise does nothing — the
//! players' strip is where the camera is moved from.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::seatplate::{Detail, SeatPlate};

/// The plate's width at scale 1, step L.
const PLATE_W: f32 = 268.0;
/// Its padding: across, and above and below.
const PLATE_PAD_X: f32 = 12.0;
const PLATE_PAD_Y: f32 = 8.0;
/// Its three lines' heights, and the gap between two.
const LINE_1: f32 = 22.0;
const LINE_2: f32 = 17.0;
const LINE_3: f32 = 20.0;
const LINE_GAP: f32 = 2.0;
/// The seat colour's spine down the plate's left edge, as on its chip.
const PLATE_SPINE: f32 = 3.0;

/// The plate's size at scale 1: two lines, or three while mana floats.
#[must_use]
pub(crate) const fn plate_size(lines: u8) -> Vec2 {
    let two = 2.0 * PLATE_PAD_Y + LINE_1 + LINE_GAP + LINE_2;
    Vec2::new(
        PLATE_W,
        if lines > 2 {
            two + LINE_GAP + LINE_3
        } else {
            two
        },
    )
}

const HEADER_H: f32 = plate_size(2).y;
const HEADER_W: f32 = PLATE_W;
/// The steps' panel: its height across the band and its length along it.
const TRACK_W: f32 = 60.0;
const TRACK_H: f32 = 520.0;

/// Minimum clear space around the identity and phase groups.
const BAND_GAP: f32 = 32.0;

/// The plate's ground: the players' strip's blue hour, so a seat's chip and
/// its plate read as one thing in two places.
const PLATE_GROUND: Color = Color::srgba(0.075, 0.115, 0.165, 0.90);
/// Its rim at rest.
const PLATE_RIM: Color = Color::srgba(0.40, 0.54, 0.62, 0.45);

/// Which part of the seat's band carries this piece of its information.
#[derive(Component, Clone, Copy, Debug)]
pub enum Panel {
    /// The plate: name, life and details, at the seat's own left.
    Identity,
    /// The twelve steps, at the band's right end.
    Phases,
    /// One numeral inside the central compass.
    Turn,
}

impl Panel {
    pub(crate) fn size(self) -> Vec2 {
        match self {
            Self::Identity => Vec2::new(HEADER_W, HEADER_H),
            Self::Turn => Vec2::new(TURN_CELL * 3.0, TURN_EM * 1.2),
            Self::Phases => Vec2::new(TRACK_H, TRACK_W),
        }
    }
}

/// A seat's plate: a press chooses that seat while a question can target
/// it, and does nothing otherwise (no camera, by the owner's word).
#[derive(Component, Clone, Copy, Debug)]
pub struct PlateTab {
    /// The seat.
    pub player: PlayerId,
}

/// The letters remain upright; their centres and scale follow the table.
///
/// Every write is guarded on what the node already holds, through the
/// `Mut`s themselves: a `Node` merely borrowed mutably reads as changed, and
/// a changed `Node` relays out the whole UI — which this did for every panel
/// on every frame of a table at rest (`docs/perf-baseline.md`, 08.10.2026).
pub(super) fn place(
    duel: &Duel,
    lens: Option<&crate::table::Lens>,
    (player, panel, step): (PlayerId, Panel, f32),
    node: &mut Mut<Node>,
    turn: &mut Mut<UiTransform>,
) {
    let Some((corner, tilt, scale, mat_above)) = pose_facing(duel, lens, player, panel, step)
    else {
        if node.display != Display::None {
            node.display = Display::None;
        }
        return;
    };
    if node.display != Display::Flex {
        node.display = Display::Flex;
    }
    // The steps' names stand on the side away from the battlefield, the
    // tiles on the side that meets it: below the names on my own band,
    // above them on a band whose mat is above it on the screen (the owner,
    // 08.10.2026: the opponent's labels under its bar, mirrored to mine).
    if matches!(panel, Panel::Phases) {
        let flow = if mat_above {
            FlexDirection::ColumnReverse
        } else {
            FlexDirection::Column
        };
        if node.flex_direction != flow {
            node.flex_direction = flow;
        }
    }
    if node.left != px(corner.x) {
        node.left = px(corner.x);
    }
    if node.top != px(corner.y) {
        node.top = px(corner.y);
    }
    let rotation = Rot2::radians(tilt);
    if turn.rotation != rotation {
        turn.rotation = rotation;
    }
    if turn.scale != Vec2::splat(scale) {
        turn.scale = Vec2::splat(scale);
    }
}

/// How many lines `player`'s plate is drawn with in `duel`'s view, without
/// building the plate (this runs every frame).
fn plate_lines(duel: &Duel, player: PlayerId) -> u8 {
    let floating = duel
        .view
        .as_ref()
        .and_then(|v| v.seat(player))
        .is_some_and(|s| !s.mana_pool.is_empty());
    if floating { 3 } else { 2 }
}

/// Where one of `player`'s panels stands — its box's top-left, turn and
/// scale — or `None` where it is not drawn (a tear running, no shelf on
/// screen, or under the hand zone). `step` is the text step's factor, which
/// the plate is drawn at.
pub(crate) fn pose(
    duel: &Duel,
    lens: Option<&crate::table::Lens>,
    player: PlayerId,
    panel: Panel,
    step: f32,
) -> Option<(Vec2, f32, f32)> {
    pose_facing(duel, lens, player, panel, step).map(|(at, tilt, scale, _)| (at, tilt, scale))
}

/// [`pose`], and whether the seat's battlefield lies above the panel as it
/// is drawn (the steps turn their names to the other side).
fn pose_facing(
    duel: &Duel,
    lens: Option<&crate::table::Lens>,
    player: PlayerId,
    panel: Panel,
    step: f32,
) -> Option<(Vec2, f32, f32, bool)> {
    // Ink pinned to a band of felt that is tearing would jump stage by
    // stage ahead of its mat: the bars stand down for the second it takes,
    // and come back on the docked table.
    if duel.tear.is_some() {
        return None;
    }
    let lens = lens?;
    if matches!(panel, Panel::Turn) {
        // The number's size rule (DESIGN-v7 §3.4): a share of the dial's
        // drawn diameter, floored at 16 px and capped at 40, so a turn past
        // 100 reads at eight seats and does not shout on a visit.
        let middle = lens.project(Vec2::ZERO)?;
        let across = crate::dial::dial_px(lens)?;
        return Some((
            middle - panel.size() * 0.5,
            0.0,
            baylee_client_core::dial::number_px(across) / TURN_EM,
            false,
        ));
    }
    let layout = duel.layout.as_ref()?;
    let slot = layout.shown(player)?;
    // The seat's own band, projected: the strip along the rim nearest the
    // middle of the table, taken from the model so the ink and `Shelf` — the
    // density probe and the tiny-overview fallback — describe one rectangle.
    if matches!(panel, Panel::Identity) {
        let top = hand_top(duel, lens);
        return plate_on(layout, lens, player, |p| plate_lines(duel, p), (step, top))
            .map(|(at, tilt, scale)| (at, tilt, scale, false));
    }
    let corners = lens.corners(slot.ledge_corners())?;
    let (at, tilt, scale, mat_above) = steps_on(slot, lens, corners)?;
    (!under_the_hand(at, panel.size(), tilt, scale, hand_top(duel, lens)))
        .then_some((at, tilt, scale, mat_above))
}

/// Where the hand zone's top edge stands in the lens's window: the zone's
/// whole height on a laptop, and on a phone what of it the hand drawer has
/// open (`hand_drawer::drop_at`) — shut, only the actions bar is left there.
pub(crate) fn hand_top(duel: &Duel, lens: &crate::table::Lens) -> f32 {
    let window = lens.window();
    let phone = baylee_client_core::tableview::TableFrame::of(window.x, window.y)
        == baylee_client_core::tableview::TableFrame::Phone;
    let drop = if phone {
        crate::hud::hand_drawer::drop_at(duel.hand_shown)
    } else {
        0.0
    };
    window.y - (crate::hud::HAND_ZONE_H - drop)
}

/// The plate's pose: flush with the seat's battlefield — its edge on the
/// mat's drawn edge on the hearth side, its end at the mat's corner on the
/// seat's own left (the owner, 08.10.2026: *"right at the edge of the
/// player's battlefield (flush)"*), in every arrangement, at every scale and
/// turn. It stands outside the mat, so it covers none of the seat's cards or
/// their badges.
///
/// Where the corner place is not clear — out of the window, on a corner the
/// HUD stands in ([`clear_of_the_hud`]), under the strips and the hand, or on
/// another seat's place — the plate slides along the same edge towards the
/// mat's middle, a quarter of its width at a time, and stays flush. None of
/// those clear: not drawn (a visit that put the seat under the hand). A
/// plate grown to three lines grows away from the mat, so its first line
/// stays where it was.
pub(crate) fn plate_on(
    layout: &baylee_client_core::layout::TableLayout,
    lens: &crate::table::Lens,
    player: PlayerId,
    lines_of: impl Fn(PlayerId) -> u8,
    (step, hand_top): (f32, f32),
) -> Option<(Vec2, f32, f32)> {
    // Seat by seat in the table's order, each plate kept off the ones placed
    // before it: two mats whose edges face across a narrow hearth (a duel's)
    // set their plates at opposite ends instead of on each other. A fixed
    // array, because this runs for every plate on every frame.
    let mut placed = [[Vec2::ZERO; 4]; 8];
    let mut count = 0;
    for slot in layout.on_felt() {
        let pose = lens.corners(slot.ledge_corners()).and_then(|corners| {
            let size = plate_size(lines_of(slot.player));
            plate_at(
                layout,
                (slot, lens, corners),
                size,
                (step, hand_top),
                &placed[..count],
            )
        });
        if slot.player == player {
            return pose.map(|(at, tilt, scale, _)| (at, tilt, scale));
        }
        if let Some((_, _, _, quad)) = pose
            && count < placed.len()
        {
            placed[count] = quad;
            count += 1;
        }
    }
    None
}

/// One seat's plate, as [`plate_on`] tries it: flush at the corner, then
/// sliding along the edge, clear of the HUD, other seats' places and the
/// plates in `placed`. With the quad it is drawn as.
fn plate_at(
    layout: &baylee_client_core::layout::TableLayout,
    (slot, lens, corners): (
        &baylee_client_core::layout::SeatSlot,
        &crate::table::Lens,
        [Vec2; 4],
    ),
    size: Vec2,
    (step, hand_top): (f32, f32),
    placed: &[[Vec2; 4]],
) -> Option<(Vec2, f32, f32, [Vec2; 4])> {
    let (_, _, band_scale) = pose_on(corners, Panel::Identity);
    let scale = band_scale * step;
    let MatEdge {
        left: corner,
        along,
        away,
        ..
    } = mat_edge(slot, lens)?;
    // Turned with the edge itself, folded upright as every panel is.
    let tilt = upright(along.y.atan2(along.x));
    let window = lens.window();
    let half = size * scale * 0.5;
    let [.., far_b, far_a] = corners;
    let room = far_a.distance(far_b) - size.x * scale;
    // Every seat's steps: its own stand on the same shelf at its right end,
    // and across a narrow hearth another seat's can reach this edge.
    let mut steps = [[Vec2::ZERO; 4]; 8];
    let mut count = 0;
    for other in layout.on_felt() {
        if let Some(corners) = lens.corners(other.ledge_corners())
            && count < steps.len()
            && let Some((at, tilt, scale, _)) = steps_on(other, lens, corners)
        {
            steps[count] = drawn_quad(at, Panel::Phases.size(), tilt, scale);
            count += 1;
        }
    }
    let steps = &steps[..count];
    (0..=12u8)
        .map(|k| f32::from(k) * half.x * 0.5)
        .take_while(|slide| *slide <= room.max(0.0))
        .find_map(|slide| {
            let middle = corner + along * (half.x + slide) + away * (half.y + BAND_AIR);
            let at = middle - size * 0.5;
            let quad = drawn_quad(at, size, tilt, scale);
            (clear_of_the_hud(&quad, window, hand_top)
                && !on_another_seat(&quad, layout, slot.player, lens)
                && !placed.iter().any(|other| overlaps(&quad, other))
                && !steps.iter().any(|other| overlaps(&quad, other)))
            .then_some((at, tilt, scale, quad))
        })
}

/// Between the battlefield's drawn edge and what hangs on it — the plate's
/// edge and the steps' tiles alike, one constant for both (the owner,
/// 08.10.2026: *"the plate's bottom line and the phases bar's bottom line at
/// the same distance from the battlefield"*): a hairline, so they read as
/// touching and the mat's rim is not painted over.
pub(crate) const BAND_AIR: f32 = 1.0;

/// The battlefield's edge on the hearth side, as [`mat_edge`] finds it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MatEdge {
    /// The drawn corner on the seat's own left.
    pub(crate) left: Vec2,
    /// The drawn corner on the seat's own right.
    pub(crate) right: Vec2,
    /// Along the edge, towards the seat's right.
    pub(crate) along: Vec2,
    /// Square to the edge, away from the battlefield (towards the hearth).
    pub(crate) away: Vec2,
}

/// The battlefield's edge on the hearth side, as drawn: its two corners,
/// the direction along the edge (towards the seat's right) and the one away
/// from the battlefield (towards the hearth).
///
/// The battlefield is the framed field of lanes, which the shelf (the band
/// the seat's ink is written on) borders on the hearth side. Its edge is
/// taken where `mat.wgsl` draws it, not where the layout's shelf ends: the
/// shader crops the field at `LEDGE_FRAC` of the mat's drawn depth, and a
/// duel's mat is deeper than `MAT_DRAWN_DEPTH` (`layout`'s roomy boards),
/// so its drawn shelf is deeper than `ledge_corners`' by a quarter of a
/// unit — the ten pixels the owner still saw between plate and frame. The
/// edge runs the drawn mat's whole width (`MAT_MARGIN` past the playing
/// extent at each end), projected through `lens`; `None` if any of it is
/// behind the eye.
pub(crate) fn mat_edge(
    slot: &baylee_client_core::layout::SeatSlot,
    lens: &crate::table::Lens,
) -> Option<MatEdge> {
    use baylee_client_core::tabletop::{LEDGE_FRAC, MAT_MARGIN};
    let towards = Vec2::new(slot.facing.sin(), slot.facing.cos());
    let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
    let reach = if baylee_client_core::layout::LEDGE_IS_OUTER {
        -1.0
    } else {
        1.0
    };
    // The drawn mat's half depth, and its shelf as the shader cuts it.
    let half_depth = slot.half_extent.y + MAT_MARGIN * slot.scale;
    let shelf = LEDGE_FRAC * half_depth * 2.0;
    let line = slot.center + towards * (reach * (half_depth - shelf));
    let out = side * (slot.half_extent.x + MAT_MARGIN * slot.scale);
    let left = lens.project(line - out)?;
    let right = lens.project(line + out)?;
    // A point on the shelf, to know which side of the line the hearth is.
    let hearth = lens.project(line + towards * (reach * shelf * 0.5))?;
    let along = (right - left).normalize_or_zero();
    // Square to the edge, on the side away from the field: under
    // perspective the mat's own depth axis leans, and a plate set off
    // along it would stand into the field at one end.
    let normal = Vec2::new(-along.y, along.x);
    let away = if normal.dot(hearth - left.midpoint(right)) < 0.0 {
        -normal
    } else {
        normal
    };
    Some(MatEdge {
        left,
        right,
        along,
        away,
    })
}

/// The steps' pose: the tiles' edge [`BAND_AIR`] off the battlefield's
/// drawn edge, as the plate's is, and the panel's end on the seat's right
/// at the battlefield's corner there (the owner, 08.10.2026: *"the phases
/// bar's right edge on the same line as the battlefield's right edge"* —
/// for a seat across the table that corner is on the screen's left). At the
/// band's scale, turned with the edge and folded upright; and whether the
/// battlefield lies above the panel as drawn, which turns its names to the
/// other side ([`place`]).
pub(crate) fn steps_on(
    slot: &baylee_client_core::layout::SeatSlot,
    lens: &crate::table::Lens,
    corners: [Vec2; 4],
) -> Option<(Vec2, f32, f32, bool)> {
    let (_, _, scale) = pose_on(corners, Panel::Identity);
    let edge = mat_edge(slot, lens)?;
    let size = Panel::Phases.size();
    let tilt = upright(edge.along.y.atan2(edge.along.x));
    let half = size * scale * 0.5;
    let middle = edge.right - edge.along * half.x + edge.away * (half.y + BAND_AIR);
    // `away` as the turned panel sees it: pointing up its own box, the
    // battlefield is below it.
    let mat_above = (Rot2::radians(-tilt) * edge.away).y > 0.0;
    Some((middle - size * 0.5, tilt, scale, mat_above))
}

/// The four corners a box posed like this is drawn at: `corner` is the
/// top-left of the un-rotated box and `size` its size before `scale`; the
/// turn and the scale are about the box's own middle.
pub(crate) fn drawn_quad(corner: Vec2, size: Vec2, tilt: f32, scale: f32) -> [Vec2; 4] {
    let middle = corner + size * 0.5;
    let half = size * scale * 0.5;
    let spin = Rot2::radians(tilt);
    [
        middle + spin * Vec2::new(-half.x, -half.y),
        middle + spin * Vec2::new(half.x, -half.y),
        middle + spin * Vec2::new(half.x, half.y),
        middle + spin * Vec2::new(-half.x, half.y),
    ]
}

/// Whether a drawn quad lies inside the window and clear of what the HUD
/// stands over the table: the two top corners (the arrangement pill and the
/// report button, with the square beside it) and the strips and the hand
/// along the bottom (the hand zone's top is `hand_top`). Bars are drawn
/// under the HUD (`GlobalZIndex(-1)`), so a plate there would be read
/// half-covered.
pub(crate) fn clear_of_the_hud(quad: &[Vec2; 4], window: Vec2, hand_top: f32) -> bool {
    const MARGIN: f32 = crate::hud::EDGE;
    let inside = quad.iter().all(|p| {
        p.x >= MARGIN
            && p.x <= window.x - MARGIN
            && p.y >= MARGIN
            && p.y <= hand_top - crate::hud::STRIPS_H
    });
    inside
        && crate::hud::hud_corners(window)
            .iter()
            .all(|corner| !overlaps(quad, &rect_quad(*corner)))
}

/// A rectangle's corners, round the loop.
fn rect_quad(rect: Rect) -> [Vec2; 4] {
    [
        rect.min,
        Vec2::new(rect.max.x, rect.min.y),
        rect.max,
        Vec2::new(rect.min.x, rect.max.y),
    ]
}

/// Whether two convex quads overlap: a separating-axis test on their edges'
/// normals.
pub(crate) fn overlaps(a: &[Vec2; 4], b: &[Vec2; 4]) -> bool {
    let span = |points: &[Vec2; 4], axis: Vec2| {
        points
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.dot(axis)), hi.max(p.dot(axis)))
            })
    };
    let apart = |shape: &[Vec2; 4]| {
        (0..4).any(|i| {
            let edge = shape[(i + 1) % 4] - shape[i];
            let axis = Vec2::new(-edge.y, edge.x);
            let (a0, a1) = span(a, axis);
            let (b0, b1) = span(b, axis);
            a1 < b0 || b1 < a0
        })
    };
    !(apart(a) || apart(b))
}

/// Whether a drawn quad covers any other seat's place — its mat and its piles,
/// as the camera draws them.
fn on_another_seat(
    quad: &[Vec2; 4],
    layout: &baylee_client_core::layout::TableLayout,
    player: PlayerId,
    lens: &crate::table::Lens,
) -> bool {
    layout
        .on_felt()
        .filter(|other| other.player != player)
        .any(|other| {
            let (sin, cos) = other.facing.sin_cos();
            // Out to the mat as it is drawn, `MAT_MARGIN` past the playing
            // extent, which is where its band and the ink on it stand too.
            let half = other.footprint()
                + Vec2::splat(baylee_client_core::tabletop::MAT_MARGIN * other.scale);
            let at = |sx: f32, sy: f32| {
                let local = half * Vec2::new(sx, sy);
                other.footprint_center()
                    + Vec2::new(
                        cos.mul_add(local.x, sin * local.y),
                        (-sin).mul_add(local.x, cos * local.y),
                    )
            };
            lens.corners([at(-1.0, -1.0), at(1.0, -1.0), at(1.0, 1.0), at(-1.0, 1.0)])
                .is_some_and(|place| overlaps(quad, &place))
        })
}

/// Where one panel sits on a seat's projected band, and how big.
///
/// Split out of [`place`] because it is the whole of the arithmetic and none
/// of the Bevy: `corners` is
/// [`SeatSlot::ledge_corners`](baylee_client_core::layout::SeatSlot::ledge_corners)'
/// loop already run through [`crate::table::Lens`], in the order that model
/// promises — the two on the rim first, then the two that meet the lane
/// behind it.
///
/// Returns the **top-left of the un-rotated box**, which is what a `Node`'s
/// `left`/`top` want, together with the turn and the scale
/// [`UiTransform`] applies about the box's own middle. The identity is at
/// the band's left end (the plate's last resort, [`plate_on`]) and the steps
/// at its right end (the owner, 08.10.2026: *"further to the right
/// corner"*), which leaves the middle to the dial.
pub(crate) fn pose_on(corners: [Vec2; 4], panel: Panel) -> (Vec2, f32, f32) {
    let [near_a, near_b, far_b, far_a] = corners;
    // Preserve the owner's left/right order, even for the opposing seat.
    // Only the text rotation is folded upright below.
    let end_a = near_a.midpoint(far_a);
    let end_b = near_b.midpoint(far_b);
    let (left, right) = (end_a, end_b);
    let axis = (right - left).normalize_or_zero();
    let width = left.distance(right);
    let depth = near_a.midpoint(near_b).distance(far_a.midpoint(far_b));
    // A shared scale keeps identity and phases separated as the band narrows.
    let scale = (width / (HEADER_W + TRACK_H + BAND_GAP * 3.0))
        .min(depth * 0.85 / HEADER_H.max(TRACK_W))
        .min(1.0);
    let middle = match panel {
        Panel::Identity => left + axis * (HEADER_W * scale * 0.5 + BAND_GAP * scale),
        Panel::Turn => left.midpoint(right),
        Panel::Phases => right - axis * (TRACK_H * scale * 0.5 + BAND_GAP * scale),
    };
    (
        middle - panel.size() * 0.5,
        upright(axis.y.atan2(axis.x)),
        scale.max(0.0),
    )
}

/// An angle folded into a half-turn, so ink is never drawn upside-down: the
/// seat across the table has its band turned 180° and its ink the right way
/// up.
fn upright(angle: f32) -> f32 {
    if angle > std::f32::consts::FRAC_PI_2 {
        angle - std::f32::consts::PI
    } else if angle < -std::f32::consts::FRAC_PI_2 {
        angle + std::f32::consts::PI
    } else {
        angle
    }
}

/// Whether any of a box posed like this lies under the hand zone (#303) or
/// the strips standing on it.
///
/// `corner` is the top-left of the un-rotated box and `size` its size before
/// `scale`, which is what [`pose_on`] hands over; the turn and the scale are
/// about the box's own middle.
///
/// The bars stand under the whole HUD (`GlobalZIndex(-1)`), and the hand
/// zone's skirt is a veil over the table and not a lid on it
/// (`hud::hand::spawn_hand_zone`'s own note). So a bar the camera has put
/// behind the hand is read through it: aimed at an opponent, the camera
/// stands over the local seat's own mat, and that seat's name, life and
/// phases showed faintly under the cards. The felt under the veil is meant
/// to be seen; ink is not, because it is the one thing there that reads as
/// writing, and the seat's name and life are on the players' strip above the
/// bar anyway. A box any part of which would be covered is not drawn at all,
/// since half a name is worse than none.
pub(crate) fn under_the_hand(
    corner: Vec2,
    size: Vec2,
    tilt: f32,
    scale: f32,
    hand_top: f32,
) -> bool {
    let (sin, cos) = tilt.sin_cos();
    let half = size * scale * 0.5;
    let reach = sin.abs().mul_add(half.x, cos.abs() * half.y);
    corner.y + size.y * 0.5 + reach > hand_top
}

/// Where `player`'s plate is drawn this frame, for whatever stands beside
/// it (the decision clock's lane, 08.10.2026): read it rather than
/// repeating the plate's placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlateBeside {
    /// The plate's four drawn corners: its top-left, top-right,
    /// bottom-right and bottom-left as it reads.
    pub quad: [Vec2; 4],
    /// Its turn, radians, as its `UiTransform` has it.
    pub tilt: f32,
    /// Its scale (the band's and the text step's), as its `UiTransform`
    /// has it.
    pub scale: f32,
    /// Along the mat's edge towards the seat's right: the plate's free
    /// side, since it hangs at the mat's left corner and slides right.
    pub along: Vec2,
    /// Square to the mat's edge, away from the battlefield.
    pub away: Vec2,
}

/// [`PlateBeside`] for `player` in `duel` through `lens` at text step
/// `step`, or `None` where the plate is not drawn.
pub fn plate_beside(
    duel: &Duel,
    lens: &crate::table::Lens,
    player: PlayerId,
    step: f32,
) -> Option<PlateBeside> {
    let (corner, tilt, scale) = pose(duel, Some(lens), player, Panel::Identity, step)?;
    let slot = duel.layout.as_ref()?.shown(player)?;
    let edge = mat_edge(slot, lens)?;
    Some(PlateBeside {
        quad: drawn_quad(corner, plate_size(plate_lines(duel, player)), tilt, scale),
        tilt,
        scale,
        along: edge.along,
        away: edge.away,
    })
}

/// Screen-space centre of the life value on `player`'s plate, wherever the
/// plate stands: what an attack's arrow points at.
pub(crate) fn life_anchor(
    duel: &Duel,
    player: PlayerId,
    lens: &crate::table::Lens,
    step: f32,
) -> Option<Vec2> {
    let (corner, angle, scale) = pose(duel, Some(lens), player, Panel::Identity, step)?;
    let size = plate_size(plate_lines(duel, player));
    let centre = corner + size * 0.5;
    let life = Vec2::new(PLATE_W - PLATE_PAD_X - 34.0, PLATE_PAD_Y + LINE_1 * 0.5);
    Some(centre + Rot2::radians(angle) * (life - size * 0.5) * scale)
}

fn frame(commands: &mut Commands, root: Entity, player: PlayerId, panel: Panel) -> Entity {
    let size = panel.size();
    let entity = commands
        .spawn((
            SeatBar {
                player,
                placed: None,
            },
            panel,
            UiTransform::default(),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                width: px(size.x),
                height: px(size.y),
                // One direction for both panels: the band runs along the rim
                // and everything written on it runs with it.
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                border: UiRect::ZERO,
                border_radius: BorderRadius::all(px(2)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.35)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(entity);
    entity
}

#[allow(clippy::too_many_arguments)] // reuses the existing typed seat cells
pub(super) fn spawn(
    commands: &mut Commands,
    root: Entity,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    role: baylee_client_core::board::SeatRole,
    orders: &PhaseOrders,
    fonts: &UiFonts,
) {
    spawn_identity(commands, root, lang, view, statics, seat, role, fonts);
    spawn_phases(commands, root, lang, view, statics, seat, orders, fonts);
}

/// The plate: spine, then three lines (`client_core::seatplate`).
#[allow(clippy::too_many_arguments)]
fn spawn_identity(
    commands: &mut Commands,
    root: Entity,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    role: baylee_client_core::board::SeatRole,
    fonts: &UiFonts,
) {
    let Some(plate) = SeatPlate::of(view, seat.player) else {
        return;
    };
    let size = plate_size(plate.lines());
    let called = super::called(lang, view, statics, seat.player, role);
    let identity = frame(commands, root, seat.player, Panel::Identity);
    commands.entity(identity).insert((
        PlateTab {
            player: seat.player,
        },
        crate::hud::Hint(plate.describe(lang, &called)),
        crate::hud::HintSeat(seat.player),
        // The plate is the one thing here the pointer may land on.
        Pickable::default(),
        bevy::picking::hover::PickingInteraction::default(),
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            width: px(size.x),
            height: px(size.y),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Stretch,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(PLATE_GROUND),
        BorderColor::all(PLATE_RIM),
    ));
    let lost = plate.lost;
    let away = role == baylee_client_core::board::SeatRole::Away;
    let ink = if lost {
        palette::DEAD
    } else if away {
        palette::LEDGE_DEAD
    } else {
        palette::DIALOG_INK
    };
    let soft = if lost || away {
        ink
    } else {
        palette::LEDGE_SOFT
    };
    let spine = commands
        .spawn((
            Node {
                width: px(PLATE_SPINE),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(if lost {
                palette::DEAD
            } else {
                seat_colour(view.seat, statics, seat.player)
            }),
            Pickable::IGNORE,
        ))
        .id();
    let column = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: px(LINE_GAP),
                padding: UiRect::axes(px(PLATE_PAD_X), px(PLATE_PAD_Y)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let first = plate_first_line(commands, fonts, &plate, seat, view, &called, role, ink);
    commands.entity(column).add_child(first);
    if !lost {
        let second = plate_details(commands, fonts, &plate, ink, soft);
        commands.entity(column).add_child(second);
        if !plate.pool.is_empty() {
            let third = spawn_seat_pool(commands, fonts, &plate.pool);
            commands.entity(third).insert(PlateMark {
                player: plate.player,
                kind: PlateMarkKind::Pool,
            });
            commands.entity(column).add_child(third);
        }
    }
    commands.entity(identity).add_children(&[spine, column]);
    top_lines(commands, identity, seat.player);
}

/// The turn's and the wait's lines along a plate's top edge, as on the chip.
fn top_lines(commands: &mut Commands, plate: Entity, player: PlayerId) {
    for kind in [crate::hud::TagKind::Turn, crate::hud::TagKind::Priority] {
        let line = commands
            .spawn(crate::hud::ledge::players::line(player, kind, true))
            .id();
        commands.entity(plate).add_child(line);
    }
}

/// The plate's first line: crown, mark, name; then life.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one line, one flat build
fn plate_first_line(
    commands: &mut Commands,
    fonts: &UiFonts,
    plate: &SeatPlate,
    seat: &SeatView,
    view: &PlayerView,
    called: &str,
    role: baylee_client_core::board::SeatRole,
    ink: Color,
) -> Entity {
    let line = commands
        .spawn((
            Node {
                height: px(LINE_1),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(5),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut parts = Vec::new();
    if plate.monarch {
        parts.push(plate_icon(
            commands,
            fonts,
            glyph::CROWN,
            12.0,
            palette::CANDLE,
            PlateMark {
                player: plate.player,
                kind: PlateMarkKind::Crown,
            },
        ));
    }
    let mark = if plate.lost {
        Some(glyph::SKULL)
    } else if role == baylee_client_core::board::SeatRole::Away {
        Some(glyph::AWAY)
    } else if role == baylee_client_core::board::SeatRole::House {
        Some(glyph::HOUSE)
    } else {
        None
    };
    if let Some(mark) = mark {
        parts.push(plate_icon(
            commands,
            fonts,
            mark,
            10.0,
            palette::LEDGE_SOFT,
            (),
        ));
    }
    let name_ink = if plate.lost {
        ink
    } else if view.active == seat.player {
        palette::CANDLE
    } else if seat.player == view.seat {
        palette::ACTIVE
    } else {
        ink
    };
    parts.push(
        commands
            .spawn((
                Text::new(called.to_string()),
                tf_bold(fonts, 13.0),
                TextColor(name_ink),
                TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                // The name gives way, and has to be told to: a `NoWrap`
                // text's automatic minimum is the whole string.
                Node {
                    flex_shrink: 1.0,
                    min_width: px(0),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id(),
    );
    // Life at the line's right end.
    let spring = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    parts.push(spring);
    if !plate.lost {
        parts.push(plate_life(commands, fonts, plate, ink));
    }
    commands.entity(line).add_children(&parts);
    line
}

/// The plate's life: a heart and the number, both in danger at five.
fn plate_life(commands: &mut Commands, fonts: &UiFonts, plate: &SeatPlate, ink: Color) -> Entity {
    let low = plate.life <= 5;
    commands
        .spawn((
            LifeCell {
                player: plate.player,
            },
            Node {
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Pickable::IGNORE,
            children![
                (
                    Text::new(glyph::HEART.to_string()),
                    icon_tf(fonts, 11.0),
                    TextColor(if low {
                        palette::DANGER
                    } else {
                        palette::LEDGE_SOFT
                    }),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(plate.life.to_string()),
                    tf_bold(fonts, 16.0),
                    TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                    TextColor(if low { palette::DANGER } else { ink }),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id()
}

/// One of the things a plate says only sometimes, marked for
/// `/state.plates` and the tests: the crown, the ∞, the pool's line.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlateMark {
    /// Whose plate it stands on.
    pub player: PlayerId,
    /// Which.
    pub kind: PlateMarkKind,
}

/// What a [`PlateMark`] marks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlateMarkKind {
    /// The crown before the monarch's name.
    Crown,
    /// The ∞ beside a hand no maximum size applies to.
    Unlimited,
    /// The third line: mana floating in the seat's pool.
    Pool,
}

/// An icon on the plate, pointer-transparent, with whatever marker it wears.
fn plate_icon(
    commands: &mut Commands,
    fonts: &UiFonts,
    mark: char,
    size: f32,
    ink: Color,
    marker: impl Bundle,
) -> Entity {
    commands
        .spawn((
            Text::new(mark.to_string()),
            table_icon_tf(fonts, mark, size),
            TextColor(ink),
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
            marker,
        ))
        .id()
}

/// The glyph a detail is drawn with: the audited zone set for the four
/// zones (`client_core::tableicons`), and the counters' own.
pub(crate) const fn detail_glyph(detail: Detail) -> char {
    use baylee_client_core::tableicons::{ENERGY, POISON, ZONES};
    match detail {
        Detail::Hand { .. } => ZONES[0],
        Detail::Library(_) => ZONES[1],
        Detail::Graveyard(_) => ZONES[2],
        Detail::Exile(_) => ZONES[3],
        Detail::Poison(_) => POISON,
        Detail::Energy(_) => ENERGY,
        Detail::Commander(_) => glyph::COMMANDER_DAMAGE,
    }
}

/// The plate's second line: an icon and a number per detail.
fn plate_details(
    commands: &mut Commands,
    fonts: &UiFonts,
    plate: &SeatPlate,
    ink: Color,
    soft: Color,
) -> Entity {
    let line = commands
        .spawn((
            Node {
                height: px(LINE_2),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for detail in &plate.details {
        let loud = if detail.dangerous() {
            palette::DANGER
        } else {
            ink
        };
        let group = commands
            .spawn((
                Node {
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: px(3),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let mark = detail_glyph(*detail);
        let icon = plate_icon(
            commands,
            fonts,
            mark,
            10.0,
            if detail.dangerous() { loud } else { soft },
            (),
        );
        let number = commands
            .spawn((
                Text::new(detail.number()),
                tf(fonts, 11.5),
                TextColor(loud),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(group).add_children(&[icon, number]);
        if let Detail::Hand {
            unlimited: true, ..
        } = detail
        {
            let infinity = plate_icon(
                commands,
                fonts,
                glyph::INFINITY,
                10.0,
                palette::CANDLE,
                PlateMark {
                    player: plate.player,
                    kind: PlateMarkKind::Unlimited,
                },
            );
            commands.entity(group).add_child(infinity);
        }
        commands.entity(line).add_child(group);
    }
    line
}

#[allow(clippy::too_many_arguments)]
fn spawn_phases(
    commands: &mut Commands,
    root: Entity,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    orders: &PhaseOrders,
    fonts: &UiFonts,
) {
    let track = frame(commands, root, seat.player, Panel::Phases);
    commands.entity(track).insert(Node {
        display: Display::None,
        position_type: PositionType::Absolute,
        width: px(TRACK_H),
        height: px(TRACK_W),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        // The tiles at the end that meets the battlefield, whichever way
        // `place` turns the column.
        justify_content: JustifyContent::FlexEnd,
        row_gap: px(6),
        ..default()
    });
    let current = RailRow::current(view.phase, view.step);
    let caption = spawn_phase_caption(commands, fonts, lang, current, seat.player, view.active);
    let timeline = commands
        .spawn((
            Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_children(&[caption, timeline]);
    let side = if same_team(statics, seat.player, view.seat) {
        RailSide::Mine
    } else {
        RailSide::Theirs
    };
    for phase in baylee_client_core::automation::RAIL_PHASES {
        let group = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(1),
                    ..default()
                },
                BackgroundColor(palette::DOCK_GROUND.with_alpha(0.65)),
                Pickable::IGNORE,
            ))
            .id();
        for row in phase.rows().iter().copied() {
            let tile = spawn_tile(
                commands,
                fonts,
                Density::Full,
                38.0,
                TileState {
                    side,
                    row,
                    skipped: orders
                        .rows_for(side)
                        .any(|(r, skipped)| r == row && skipped),
                    live: row.grants_priority(),
                    now: row == current,
                    gold: view.active == seat.player,
                    selected: orders.selected() == Some((side, row)),
                    lost: seat.has_lost(),
                },
            );
            commands.entity(tile).insert(PhaseHint {
                player: seat.player,
                row,
            });
            commands.entity(tile).insert(Node {
                width: px(38),
                height: px(34),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            });
            commands.entity(group).add_child(tile);
        }
        commands.entity(timeline).add_child(group);
    }
}

fn spawn_phase_caption(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    current: RailRow,
    player: PlayerId,
    active: PlayerId,
) -> Entity {
    commands
        .spawn((
            PhaseCaption {
                player,
                current,
                lang,
            },
            Text::new(current.name().text(lang)),
            tf_bold(fonts, 12.0),
            TextColor(if active == player {
                palette::CANDLE
            } else {
                palette::DOCK_INK.with_alpha(0.45)
            }),
            Pickable::IGNORE,
        ))
        .id()
}

/// The plate's third line: every seat's floating mana is public. The same
/// colour/count and restricted-mana convention the owed strip's pips use,
/// in the Mana font as everywhere a symbol is drawn.
fn spawn_seat_pool(
    commands: &mut Commands,
    fonts: &UiFonts,
    pool: &[baylee_client_core::manapool::Floating],
) -> Entity {
    let row = commands
        .spawn((
            Node {
                height: px(LINE_3),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(4),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for mana in pool {
        let group = commands
            .spawn((
                Node {
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: px(2),
                    padding: UiRect::axes(px(1), px(0)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BorderColor::all(if mana.restricted {
                    palette::DIALOG_SOFT
                } else {
                    Color::NONE
                }),
                Pickable::IGNORE,
            ))
            .id();
        let pip = crate::manaui::spawn_pip(commands, fonts, mana.pip, 12.0);
        commands.entity(pip).insert(Pickable::IGNORE);
        let count = commands
            .spawn((
                Text::new(format!("\u{00d7}{}", mana.count)),
                tf_bold(fonts, 10.5),
                TextColor(palette::DIALOG_INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(group).add_children(&[pip, count]);
        commands.entity(row).add_child(group);
    }
    row
}

#[derive(Component)]
pub(crate) struct PhaseHint {
    player: PlayerId,
    row: RailRow,
}

#[derive(Component)]
pub(crate) struct PhaseCaption {
    player: PlayerId,
    current: RailRow,
    lang: Lang,
}

/// Hover explains a glyph in the track's existing caption, without a popup.
pub(crate) fn describe_phase(
    tiles: Query<(&PhaseHint, &bevy::picking::hover::PickingInteraction)>,
    mut captions: Query<(&PhaseCaption, &mut Text)>,
) {
    for (caption, mut text) in &mut captions {
        let row = tiles
            .iter()
            .find_map(|(hint, interaction)| {
                (hint.player == caption.player
                    && *interaction != bevy::picking::hover::PickingInteraction::None)
                    .then_some(hint.row)
            })
            .unwrap_or(caption.current);
        let label = row.name().text(caption.lang);
        if text.0 != label {
            text.0 = label.to_string();
        }
    }
}

/// How a plate is lit: offered to a target question, aimed at by the
/// keyboard, under the pointer, chosen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PlateLight {
    /// Nothing asks about this seat.
    Rest,
    /// A question can target it.
    Offered,
    /// Offered, and the pointer or the keyboard's aim is on it.
    Aimed,
    /// Chosen as a target.
    Chosen,
}

impl PlateLight {
    /// The light `player`'s plate wears in `duel`, `hovered` or not.
    pub(crate) fn of(duel: &Duel, player: PlayerId, hovered: bool) -> Self {
        let Some(i) = duel.interaction.as_ref().filter(|i| i.is_mine()) else {
            return Self::Rest;
        };
        if i.is_seat_selected(player) {
            return Self::Chosen;
        }
        let offered = matches!(
            i.pending(),
            baylee_engine::choice::Pending::ChooseTargets { player_options, .. }
                if player_options.contains(&player)
        );
        if !offered {
            return Self::Rest;
        }
        let aimed = i.aim() == Some(baylee_client_core::interaction::Pick::Seat(player));
        if hovered || aimed {
            Self::Aimed
        } else {
            Self::Offered
        }
    }

    /// The plate's ground and rim in this light.
    fn paint(self) -> (Color, Color) {
        match self {
            Self::Rest => (PLATE_GROUND, PLATE_RIM),
            Self::Offered => (PLATE_GROUND, palette::CANDLE.with_alpha(0.55)),
            Self::Aimed => (
                mix(PLATE_GROUND, palette::CANDLE, 0.16),
                palette::CANDLE.with_alpha(0.90),
            ),
            Self::Chosen => (mix(PLATE_GROUND, palette::CANDLE, 0.26), palette::CANDLE),
        }
    }
}

/// `a` to `b` by `t`, keeping `a`'s alpha.
fn mix(a: Color, b: Color, t: f32) -> Color {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    Color::srgba(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        a.alpha,
    )
}

/// Lights each plate for the target question in hand; a write only where
/// the light changed.
pub(crate) fn highlight_player(
    duel: Res<Duel>,
    mut plates: Query<(
        &PlateTab,
        &bevy::picking::hover::PickingInteraction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (tab, hover, mut background, mut border) in &mut plates {
        let hovered = *hover != bevy::picking::hover::PickingInteraction::None;
        let (ground, rim) = PlateLight::of(&duel, tab.player, hovered).paint();
        if background.0 != ground {
            background.0 = ground;
        }
        let rim = BorderColor::all(rim);
        if *border != rim {
            *border = rim;
        }
    }
}

/// The turn number's drawn em at scale 1, in logical pixels; [`place`]
/// scales the panel to the dial's size rule from here.
const TURN_EM: f32 = 40.0;

/// One digit's cell: [`baylee_client_core::dial::CELL_EM`] of the em, wide
/// enough for Faustina's widest figure at weight 800.
const TURN_CELL: f32 = baylee_client_core::dial::CELL_EM * TURN_EM;

/// Kept in the same retained tree as the seats, rebuilt only on game changes.
///
/// Each digit stands in a fixed cell of its own rather than in one run of
/// text: Faustina's figures are proportional (a `1` is 444/1000 em, a `0`
/// 616), so 99 → 100 would shuffle the digits and 7, 77, 777 would each sit
/// off the hub's centre by a different amount. Cells make the number
/// tabular without asking the font for `tnum` (DESIGN-v7 §3.4).
pub(super) fn spawn_turn(
    commands: &mut Commands,
    root: Entity,
    view: &PlayerView,
    fonts: &UiFonts,
) {
    let panel = frame(commands, root, view.seat, Panel::Turn);
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for digit in view.turn.to_string().chars() {
        let cell = commands
            .spawn((
                Node {
                    width: px(TURN_CELL),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
                children![(
                    Text::new(digit.to_string()),
                    tf_serif(fonts, TURN_EM / crate::hud::SERIF_SCALE, 800),
                    TextLayout::justify(Justify::Center),
                    bevy::text::LineHeight::Px(TURN_EM),
                    TextColor(palette::CANDLE),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        commands.entity(row).add_child(cell);
    }
    commands.entity(panel).add_child(row);
}

#[cfg(test)]
mod tests;
