//! The 2.5D table: a real 3D stage, with flat cards lying on it.
//!
//! # Why 2.5D and not 2D or 3D
//!
//! Cards are flat objects on a flat surface, so modelling them as textured
//! quads costs nothing and buys three things a 2D renderer cannot do cheaply:
//! tapping is a rotation rather than a swapped sprite, the near seat can be
//! given more screen area than the far ones by perspective alone, and focusing
//! an opponent is a camera move instead of a re-layout.
//!
//! Everything a player *reads* — prompt, hand, stack, life totals — stays in
//! the 2D overlay, where text is crisp and layout is predictable. The rule is
//! simply: if it is a card on a battlefield it is in the world; if it is
//! information about the game it is in the overlay.
//!
//! # Redrawing
//!
//! [`sync_scene`] diffs the board model against the entities that exist and
//! touches only what changed. A board that did not change costs one hash lookup
//! per card and no allocation, which is what keeps a 300-permanent commander
//! table at frame rate on a phone.

use crate::Duel;
use crate::cardmat::{CardLook, CardMaterial, MOVING, material, motion_of};
use crate::face;
use crate::feltmat::FeltMaterial;
use crate::textures::CardTextures;
use baylee_client_core::board::CardGroup;
use baylee_client_core::images::{FinishTreatment, ImageKey};
use baylee_client_core::layout::{
    CARD_HEIGHT, CARD_WIDTH, PileKind, SeatSlot, TableLayout, pack_lane,
};
use baylee_client_core::tabletop;
use baylee_client_core::zones::{self, Place, Tracker};
use baylee_core::color::ColorSet;
use baylee_core::ids::ObjectId;
use baylee_core::ids::PlayerId;
use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Height of the table surface; cards float a hair above it so they never
/// z-fight with the felt.
const TABLE_Y: f32 = 0.0;
/// Vertical gap between the felt and a card.
const CARD_LIFT: f32 = 0.01;
/// Where a seat's mat sits: above the felt, below everything played on it.
const ZONE_LIFT: f32 = 0.002;
/// Where the glow under a mat sits — below the mat, above the felt.
const GLOW_LIFT: f32 = 0.001;
/// Where the centre medallion is inlaid.
const MEDALLION_LIFT: f32 = 0.0015;
/// Table kept around the play area, so no camera angle finds the slab's edge.
///
/// The slab used to be a fixed 60 × 44 whoever was sitting at it — about four
/// times a duel's table, which was harmless while the camera was 29 units up
/// and is not now that it is 17. This is a margin rather than a size because
/// the table is no longer one shape: it is cut to whatever
/// [`TableLayout::extent`] reports, which changes with the seat count, the
/// focus and the window.
///
/// It is [`tabletop::RAIL_WIDTH`] of padded rail plus a unit of felt outside
/// the play area, and it is deliberately **less** than [`AIR`]: the camera
/// frames the layout plus `AIR`, so the difference between the two is a band
/// of sky all the way round the table. A slab cut wider than the frame fills
/// the window edge to edge, and then there is no point drawing anything
/// behind it.
const SLAB_MARGIN: f32 = tabletop::RAIL_WIDTH + 1.0;
const _: () = assert!(SLAB_MARGIN < AIR);

/// How thick the slab is, in table units — near enough one card width.
///
/// A card at this scale is a twentieth of this, and that ratio is roughly
/// right: a gaming table's edge is a couple of inches of timber and a card is
/// two and a half wide. What the thickness is *for* is that the table reads
/// as an object standing in a room rather than as a plane the cards were
/// printed onto — the same argument [`CARD_THICKNESS`] makes one scale down.
/// It is visible from about a third of the way through the lean the player
/// can dial, and `camera_tests::the_table_is_a_slab_and_not_a_sheet` is the
/// bound that keeps it so.
const TABLE_THICKNESS: f32 = 0.9;
/// A card is the reference: a table thinner than the cards lying on it is a
/// sheet of paper, so the claim is worth failing a build for.
const _: () = assert!(TABLE_THICKNESS > CARD_THICKNESS * 4.0);

/// How many segments each of the slab's four corner arcs is drawn with.
///
/// A card gets four; this is a racetrack whose corners are several units
/// across and read as a chamfer at anything under about a dozen.
const SLAB_SEGMENTS: usize = 24;
/// How wide the medallion is inlaid, in table units.
///
/// A written number again, and small enough to sit in the open middle with
/// felt showing on both sides of it —
/// `camera_tests::the_medallion_floats_in_the_open_middle` is that bound. It
/// used to be derived from the lamplight ring, which no longer exists, and a
/// 4.5-unit ring across a 3.4-unit gap would have been the roulette wheel all
/// over again, this time lying across both players' mats.
const MEDALLION_SIZE: f32 = 2.2;
/// How fast the rail follows a phase change, per second.
///
/// Slower than a card moves. A step boundary is not an event a player has to
/// catch — it is a condition they should notice having changed — and a
/// rail that snapped would flicker through the four steps of combat.
const WASH_RATE: f32 = 3.0;
/// Margin around a seat's pod, so its mat is a table the cards sit on rather
/// than a box drawn tight around them.
///
/// It lives in `client-core` now, beside the bands that are fractions of the
/// quad it widens: a renderer-only margin meant the ledge and the lanes were
/// laid out over a rectangle 18.5% shallower than the one they were painted
/// on. Re-exported under the old name because everything else here reads it
/// as a margin around a pod, which is what it still is.
use baylee_client_core::tabletop::MAT_MARGIN as ZONE_MARGIN;
/// How far past the mat the glow beneath it spreads.
const GLOW_SPREAD: f32 = 2.4;
/// How much of a seat's colour the glow beneath its mat spills onto the table.
///
/// It was `0.30`, and that was tuned when a mat was a fifth of the screen and
/// the table under it was a felt of about its own brightness. Both ends of
/// that moved: the mats now fill the screen, and the table under them is a
/// dark cloth. Measured on the same frame, the table came out `(35, 24, 18)`
/// and a mat `(52, 77, 58)` — the mat is a nearly transparent sheet, so what
/// was actually being read was this lamp, twice as bright as the table and
/// warm or green depending on whose seat it was.
///
/// Every reference the design is drawn from does the opposite: the play
/// surface is the quietest thing on the table and the seat is marked at its
/// edge, not lit from beneath. The rim already carries the colour; this is
/// what is left of the glow once the rim is doing its job.
const GLOW_STRENGTH: f32 = 0.085;

/// Extra lift per card in a counted stack, so a stack reads as a stack.
const STACK_LIFT: f32 = 0.006;
/// How far the last card in a row stands above the first.
///
/// Not depth: a row of permanents is meant to read as flat, and the whole
/// rise is smaller than the gap a card already floats above the felt. It is
/// here because two quads at exactly the same height have no order at all —
/// the depth test then decides per pixel, on the last bit of an interpolated
/// float, and the answer changes as the camera moves. A lane fans as soon as
/// it holds more than it has room for, which at a four-seat table is about
/// ten permanents, and from there every card lies partly under its
/// neighbour. What it looked like was bands of the covered card's art
/// crossing the one on top.
///
/// Spread across the row rather than added per card, so a long lane cannot
/// ramp: the step shrinks as the row grows, and the shrinking is what bounds
/// it. A reverse-`z` depth buffer resolves a few millionths of a unit at
/// [`CameraRig::MAX_DISTANCE`]; an ordinary fan of a dozen puts its cards a
/// hundred times that apart, and a lane packed all the way to
/// `MIN_VISIBLE_FRACTION` — a hundred and more cards, which the model groups
/// long before — still keeps an order of magnitude of it.
const LANE_RISE: f32 = 0.004;
// A row that rose further than a card floats would be a staircase, not a row.
const _: () = assert!(LANE_RISE < CARD_LIFT);
/// The back of a card: what a stack behind a counted group is made of, and
/// what a card whose art never arrives falls back to.
const BACK_COLOR: Color = Color::srgb(0.12, 0.14, 0.18);
/// How many slabs a pile is ever built from.
///
/// Fourteen rather than four, and the number is about *continuity* rather
/// than about counting: a card slab is [`CARD_THICKNESS`] thick and the
/// layers are at most a fraction of that apart, so the pile reads as one
/// solid block of cardboard however many are in it. Past this many the same
/// fourteen slabs are simply spread further, which is what stops a
/// ninety-nine-card library from costing a hundred entities per seat.
const MAX_STACK_DEPTH: usize = 14;

/// The tallest a pile is ever drawn, in table units.
///
/// Thirty cards' worth. A real library of ninety-nine is over half a card's
/// width tall, and at this camera that is a tower standing where a player is
/// trying to read the board behind it. The cap is what makes the height a
/// *reading* rather than a measurement: a pile says thin, middling or thick,
/// and the exact number is on the seat's tab where a number belongs.
const MAX_STACK_RISE: f32 = STACK_LIFT * SHOWN_LIBRARY as f32;

/// How much wider a pile's contact shadow grows per unit of its height.
///
/// A shadow that did not grow at all would leave a thick deck looking like
/// a card hovering; one that grew with the full height would put a graveyard
/// in a pool of its own. This is the difference between a card and a
/// thirty-card pile being about a sixth of a card width of extra shadow.
const DECK_SHADOW_SPREAD: f32 = 1.0;

/// The largest library size the table draws a difference for.
///
/// Past it every deck is the same block of cardboard, so a draw changes
/// nothing on screen and nothing has to be rebuilt. Below it a deck visibly
/// thins, which is the whole reason a library has a height at all.
const SHOWN_LIBRARY: u32 = 30;

/// How tall a pile of `under` cards stands.
fn stack_rise(under: usize) -> f32 {
    (under as f32 * STACK_LIFT).min(MAX_STACK_RISE)
}

/// How many slabs that pile is built from.
///
/// One per card while there are few, so a graveyard of three is three cards;
/// capped once the cap is reached, so the slabs spread and keep overlapping
/// rather than multiplying.
fn stack_layers(under: usize) -> usize {
    under.min(MAX_STACK_DEPTH)
}
/// Lift and scale for the card under the cursor (subtle — a glance, not a jump).
///
/// The two numbers are not independent, which is why they are written down
/// together with an assertion under them. Raising a card moves its *footprint*
/// as well: the camera leans by [`CAMERA_LEAN`], so a card lifted by `y`
/// covers a patch of felt shifted by `y * CAMERA_LEAN` away from the viewer.
/// Growing it moves the footprint too — but outwards on every side at once.
///
/// So the growth is self-correcting and the rise is not, and a hover response
/// whose rise outruns its growth slides the card out from under the very
/// pointer that asked for it: `Out` fires, the hover clears, the card falls
/// back, `Over` fires. The values this shipped with missed the bound by
/// nearly double, so the assertions below are the point of the paragraph.
///
/// This is **not** either of the flickers that were reported, though it was
/// fixed while chasing them. One was a pickable preview panel opening over the
/// card it described, on cards nowhere near this bound (the fix is in
/// `hud/overlay.rs`); the other was the metallic coat looping continuously in
/// `card_ui.wgsl`, which moved no card at all (`sheen.rs`). `docs/client.md`
/// ("The pointer only speaks when it moves") has the measurements that tell
/// the three apart, and is normative on which is which.
const HOVER_LIFT: f32 = 0.06;
const HOVER_SCALE: f32 = 1.06;
/// Lift and scale for a card chosen for the pending choice (clearly "in").
const SELECTED_LIFT: f32 = 0.12;
const SELECTED_SCALE: f32 = 1.12;

/// The most a card may rise, for a given growth, without any part of the
/// footprint it started with leaving the pointer.
///
/// `CARD_WIDTH` rather than `CARD_HEIGHT` because the shift is in *world*
/// space — always straight away from the viewer — while a pod is rotated to
/// face its own seat, so a card's narrow dimension is the one the shift can
/// end up aligned with. Taking the smaller of the two is what makes the bound
/// hold at every seat instead of only at the near one.
const fn covered_lift(scale: f32) -> f32 {
    (CARD_WIDTH / 2.0) * (scale - 1.0) / CAMERA_LEAN
}
const _: () = assert!(HOVER_LIFT <= covered_lift(HOVER_SCALE));
const _: () = assert!(SELECTED_LIFT <= covered_lift(SELECTED_SCALE));
// Hover to selected is a rise as well, so the step between them is bound by
// the growth between them and not by either pair on its own.
const _: () = assert!(
    (SELECTED_LIFT - HOVER_LIFT) * CAMERA_LEAN
        <= (CARD_WIDTH / 2.0) * (SELECTED_SCALE - HOVER_SCALE)
);

/// Marks everything spawned for the duel, so closing it is one despawn.
#[derive(Component)]
pub struct DuelStage;

/// The table itself: one slab of baize inside a padded rail.
///
/// It carries the two things that have to survive a frame. `cut` is the size
/// the slab was last made at, so a mesh is only rebuilt when the table
/// actually changes shape. `shown` is the *eased* phase lamp, because easing
/// towards a target needs somewhere to keep where it started.
#[derive(Component)]
pub struct Slab {
    /// The world size the slab was cut to.
    cut: Vec2,
    /// The lamp currently on the rail: `rgb` its colour, `w` its energy.
    shown: Vec4,
    /// Where the lamp was last entering the rail from.
    source: Vec4,
    /// The motion setting the pulse was last running at.
    motion: f32,
}

/// The table camera.
#[derive(Component)]
pub struct TableCamera;

/// The table camera's state: where it looks, from how far, at which
/// azimuth. Input systems move this; [`apply_camera_rig`] turns it into a
/// transform, so navigation (tabs, keys, drag, gestures) all ends up in
/// one place.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct CameraRig {
    /// Look-at point in world space (x/z).
    pub target: Vec2,
    /// Distance from the target (zoom).
    pub distance: f32,
    /// Azimuth around the target (0 = behind the local seat).
    pub yaw: f32,
    /// How far the camera stands off vertical, as a tangent.
    ///
    /// [`CAMERA_LEAN`] is where every shot starts and where the framing
    /// arithmetic is done, because that arithmetic is about which table fits
    /// on which screen and a player tilting the camera has stopped asking
    /// that question. This field is what the player then does to it, and it
    /// is a tangent rather than an angle because that is what the transform
    /// and the shadow offsets read.
    pub lean: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            target: Vec2::ZERO,
            distance: 20.0,
            yaw: 0.0,
            lean: CAMERA_LEAN,
        }
    }
}

impl CameraRig {
    /// As close as the player may pull the camera in: one seat's lane,
    /// filling the screen.
    pub const MIN_DISTANCE: f32 = 12.0;
    /// As far as the player may push it out.
    ///
    /// It is a limit on the *player's* zoom, and [`CameraRig::home`] clamps
    /// itself to the same pair — which is why it has headroom over the
    /// furthest table there is. A five-seat table asks for about 81 units,
    /// and a limit sitting just above that would not stop the shot: it
    /// would silently crop it, because a fit refused is a fit that no
    /// longer fits.
    ///
    /// Both ends are distances through [`FOV`], so both moved when it did:
    /// the same shot through half the angle stands twice as far off, and a
    /// pair left where they were would have clamped every table on the way
    /// in and every large one on the way out.
    pub const MAX_DISTANCE: f32 = 120.0;

    /// As near vertical as the player may tilt: about 10° off plan.
    ///
    /// Not zero, and the card is why. A card is a slab with a thin wall
    /// around its edge and a contact shadow under it, and neither reads from
    /// directly overhead — a table seen straight down is a table of decals.
    pub const MIN_LEAN: f32 = 0.176;
    /// As far over as the player may tilt: about 55° off plan.
    ///
    /// Bounded because a card table is read from above. There *is* a sky
    /// behind it now (`crate::sky`), so the old reason — that a lean this
    /// far pointed the camera at the clear colour — is gone; what is left is
    /// that the cards themselves become unreadable long before the geometry
    /// does, and a seat looking along its own board sees the backs of its
    /// front row and nothing else.
    pub const MAX_LEAN: f32 = 1.428;

    /// Moves the rig so `pod` (a seat's table-space centre) fills the free
    /// canvas area: camera outside the ellipse looking inward, cards
    /// upright with their bottoms toward the screen bottom, the pod
    /// shifted clear of the own-board overlay.
    #[must_use]
    pub fn framing(slot: &SeatSlot, world_center: Vec2) -> Self {
        Self {
            target: world_center * 0.72,
            distance: (slot.half_extent.length() * 2.6).clamp(9.0, Self::MAX_DISTANCE),
            yaw: world_center.y.atan2(world_center.x) + std::f32::consts::FRAC_PI_2,
            lean: CAMERA_LEAN,
        }
    }

    /// The whole table, framed inside the part of the window it is actually
    /// seen through.
    ///
    /// This is the shot a duel opens on and the one `navigate_home` returns
    /// to, and it is computed rather than written down because the thing it
    /// has to fit changes: two seats and eight seats are different tables,
    /// and a phone and a monitor leave different amounts of them uncovered.
    /// The hard-coded 20 units it replaced put the local seat's own mat under
    /// the hand bar on every screen — a player could not see their own
    /// creatures, which made every later piece of board legibility moot.
    #[must_use]
    pub fn home(layout: &TableLayout, canvas: Canvas) -> Self {
        let Some((min, max)) = layout.extent() else {
            return Self::default();
        };
        let (min, max) = (min - Vec2::splat(AIR), max + Vec2::splat(AIR));
        let span = max - min;

        // The free band, as normalised device coordinates: +1 is the top of
        // the window, and the tab strip and the hand bar eat inwards.
        let top = 1.0 - 2.0 * canvas.top / canvas.window.y.max(1.0);
        let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y.max(1.0);
        let right = 1.0 - 2.0 * canvas.right / canvas.window.x.max(1.0);
        let aspect = canvas.window.x / canvas.window.y.max(1.0);

        // Vertically this is exact: `ground` is linear in the eye distance,
        // so the distance at which the table's far edge lands on `top` and
        // its near edge on `bottom` is one division.
        let g_top = ground(top);
        let g_bottom = ground(bottom);
        let deep = span.y / (g_top - g_bottom).max(1e-3);
        // Horizontally, every corner is asked, and each one asks about its own
        // depth. A perspective camera sees less of the felt where the felt is
        // closer, so a mat at the near edge needs more room than the same mat
        // across the table — and the corners of the *box* are not on the
        // table at all, they are the bare felt a ring leaves in its corners.
        // Fitting the box is what left a three-seat shot filling 86% of the
        // width it was given and 81% of the height, binding on neither.
        //
        // With the look point centred (below), a corner's depth is
        // `eye·(1 + k·ḡ) + k·(y − ȳ)` — linear in `eye`, which is what keeps
        // this arithmetic. Two corners fit together when the near band holds
        // both, so each *pair* gives a division and the widest pair wins.
        let mean = f32::midpoint(g_top, g_bottom);
        let middle = f32::midpoint(min.y, max.y);
        let k = CAMERA_LEAN / (1.0 + CAMERA_LEAN * CAMERA_LEAN).sqrt();
        let scale = (half_fov().tan() * aspect).max(1e-3);
        let carry = k.mul_add(mean, 1.0);
        let corners = layout.corners(AIR);
        let mut wide: f32 = 0.0;
        for a in &corners {
            for b in &corners {
                // `a` against the right edge of the band and `b` against the
                // left: the room the two of them need between them, less what
                // their own depths already give, over what a unit of eye buys.
                let held = right * k * (a.y - middle) + k * (b.y - middle);
                wide = wide.max(((a.x - b.x) / scale - held) / (carry * (1.0 + right)));
            }
        }
        // Clamped *before* the look point is derived from it. Aiming for a
        // camera the clamp then moves is the one way this can put the table
        // off screen while every number above is still right: the far edge
        // would be pinned for an eye that is not there, and land above the
        // tab strip. Clamped first, a table too big for `MAX_DISTANCE` keeps
        // its far edge pinned and overflows at the bottom, which is the
        // graceful direction.
        let lean = (1.0 + CAMERA_LEAN * CAMERA_LEAN).sqrt();
        let eye = deep
            .max(wide)
            .clamp(Self::MIN_DISTANCE * lean, Self::MAX_DISTANCE * lean);

        // The table is **centred** in the band, on both axes. Whichever of
        // the two fits binds, the slack the other one has left over is split
        // evenly instead of being pushed to one edge: the far edge used to be
        // pinned under the tab strip and every spare unit opened up in front
        // of the local seat, which on a duel was a fifth of the window of
        // bare felt below the mats and the whole table riding high.
        //
        // Where a centred look point may stand is an interval — far enough
        // back that the far edge clears `top`, far enough forward that the
        // near edge clears `bottom` — and the middle of it is the shot. A
        // table too big for `MAX_DISTANCE` has no such interval, and there
        // the far edge is pinned again and the overflow goes out of the
        // bottom, which is the graceful direction: a mat behind the tab strip
        // is a mat nobody can see, and one under the hand bar is one the
        // player can pull into view.
        //
        // Sideways the span is centred in the band as it stands at the near
        // edge, for the same reason `wide` is measured there: centring on the
        // look plane's band leaves the front row off-centre, and the rail
        // makes the band asymmetric, so being off-centre costs a whole mat on
        // one side.
        let pinned = max.y - eye * g_top;
        let forward = min.y - eye * g_bottom;
        let along = pinned.max(f32::midpoint(pinned, forward));
        // And sideways, the same interval read off the corners themselves:
        // as far right as the leftmost corner allows, as far left as the
        // rightmost one does, and the middle of that.
        let (mut left, mut right_most) = (f32::NEG_INFINITY, f32::INFINITY);
        for c in &corners {
            let band = (eye + k * (c.y - along)).max(1e-3) * scale;
            left = left.max(c.x - right * band);
            right_most = right_most.min(c.x + band);
        }
        let look = Vec2::new(f32::midpoint(left, right_most), along);
        Self {
            // `ground` works from the eye's true distance; the rig stores the
            // height it stands at, which the lean makes shorter.
            distance: eye / lean,
            // Table space to world: `+y` away from the local seat is `-z`.
            target: Vec2::new(look.x, -look.y),
            yaw: 0.0,
            // Every line above was solved at `CAMERA_LEAN`, so this is the
            // one answer it can give. A shot the player has tilted is a shot
            // they have taken over, and `frame_table` has stopped writing
            // here by then.
            lean: CAMERA_LEAN,
        }
    }
}

/// How much bare felt is left around the table when it is framed.
///
/// Two things live in this number. The first is not optional: what the layout
/// reports is the box the *cards* stand in, and a seat's mat is drawn
/// [`ZONE_MARGIN`] wider than that on every side — so a shot framed on the
/// reported box crops the mat's own printed border, and at the near edge it
/// crops it under the hand bar.
///
/// The rest is the felt itself. A table framed to the last pixel of the band
/// reads as a photograph someone cropped too tightly, whatever the arithmetic
/// says about it fitting; leaving a couple of units of table showing all round
/// is what makes it look like a table being played at rather than a diagram
/// being displayed. It is also roughly where [`GLOW_SPREAD`] fades out, so the
/// halo under an active seat's mat stays in frame with it.
///
/// It went from 2.0 to this when the sky arrived, and the cost was measured
/// rather than guessed: a card is drawn about nine per cent smaller, and what
/// it buys is a table standing in a room instead of a surface filling the
/// window. Everything past [`SLAB_MARGIN`] is sky.
const AIR: f32 = 3.5;
const _: () = assert!(AIR > ZONE_MARGIN);

/// Half the camera's vertical field of view.
fn half_fov() -> f32 {
    FOV * 0.5
}

/// Where a point on the felt lands on screen, in one axis, per unit of eye
/// distance.
///
/// Take `s` to be table-space distance from the look point along the
/// screen-vertical, positive away from the local seat, and `q` to be
/// normalised device y. With the eye at distance `D` and the lean written as
/// `L` = [`CAMERA_LEAN`], `C = 1/√(1+L²)`, the camera-space depth and height
/// of that point work out to
///
/// ```text
///     depth(s) = D + L·C·s          height(s) = C·s
/// ```
///
/// — the cross terms cancel, which is the whole reason this is arithmetic and
/// not a projection matrix. So `q = height / (depth · tan(fov/2))`, and solved
/// the other way `s = D · ground(q)`. Being *linear in `D`* is what lets
/// [`CameraRig::home`] invert it with a division instead of a search.
fn ground(q: f32) -> f32 {
    let t = half_fov().tan();
    let c = 1.0 / (1.0 + CAMERA_LEAN * CAMERA_LEAN).sqrt();
    q * t / (c * q.mul_add(-CAMERA_LEAN * t, 1.0))
}

/// The part of the window the table is actually seen through.
///
/// The HUD is not beside the battlefield, it is on top of it: the hand bar is
/// an overlay on the same full-window camera. Framing the table against the
/// *window* therefore frames it against a rectangle part of which nobody can
/// see.
///
/// The top used to cost a hundred and ten of those pixels — a strip of seat
/// tabs and a phase rail under it, on every screen and for the whole game.
/// Both are on the table now, written on each seat's own mat, so the top is
/// **zero** and the camera comes in by that much: the framed depth shrinks,
/// so the rig moves closer, so every card is drawn larger. That is the payment
/// the ledge was bought with.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Canvas {
    /// The window, in logical pixels.
    pub window: Vec2,
    /// Covered at the top. Nothing any more — kept because the field is what
    /// a future panel across the top would say so in, and because the framing
    /// arithmetic below is written in terms of all four sides.
    pub top: f32,
    /// Covered at the bottom: the hand bar.
    pub bottom: f32,
    /// Covered on the right. Nothing: the menu pills are a corner rather than
    /// a column, and the stack panel is drawn over the felt on purpose — a
    /// stack lasts a few seconds and the camera must not lurch when one
    /// appears.
    pub right: f32,
}

impl Canvas {
    /// What the duel HUD covers of a window this size.
    #[must_use]
    pub fn hud(window: Vec2) -> Self {
        Self {
            window,
            top: 0.0,
            bottom: crate::hud::HAND_BAR_H,
            right: 0.0,
        }
    }

    /// The shape of what is left once the HUD has taken its share.
    ///
    /// This is what [`TableLayout::new`] is built against, so the table comes
    /// out the shape of the space it has to fit in. On a 1728×1052 window
    /// with this HUD it is about 2.0 — nothing like the window's 1.64, and
    /// nothing like the `16.0 / 9.0` the layout used to assume.
    #[must_use]
    pub fn aspect(&self) -> f32 {
        let width = (self.window.x - self.right).max(1.0);
        let height = (self.window.y - self.top - self.bottom).max(1.0);
        width / height
    }
}

/// Tells the board model the shape of the space it is drawn in.
///
/// A system of its own because [`crate::rebuild_board`] is called from four
/// places, none of which has a window — it answers a view arriving, a
/// preference changing, a focus moving. The window changes on its own
/// schedule, and this is the one place that notices.
pub fn track_canvas(windows: Query<&Window>, mut duel: ResMut<Duel>) {
    let Ok(window) = windows.single() else {
        return;
    };
    let aspect = Canvas::hud(Vec2::new(window.width(), window.height())).aspect();
    // A resize is a rebuild of the whole layout, so the comparison has to be
    // loose enough that a window nudged by a pixel does not do one per frame.
    if duel
        .canvas_aspect
        .is_none_or(|shown| (shown - aspect).abs() > 0.01)
    {
        duel.canvas_aspect = Some(aspect);
        crate::rebuild_board(&mut duel);
    }
}

/// The framing the table currently deserves, and whether the camera is still
/// following it.
///
/// A rig that equals the last framing — or that is still
/// [`CameraRig::default`], which is what the resource starts as and what
/// `navigate_home` asks for, neither of them a place anyone aimed at — is the
/// table's camera and follows the table. One drag, zoom or focus and it is
/// the player's, and a window resize no longer moves it.
#[derive(Resource, Clone, Copy, Default)]
pub struct HomeRig(Option<CameraRig>);

/// Keeps the table framed as seats, focus and window size change.
pub fn frame_table(
    duel: Res<Duel>,
    windows: Query<&Window>,
    mut home: ResMut<HomeRig>,
    mut rig: ResMut<CameraRig>,
) {
    let Some(layout) = duel.layout.as_ref() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let next = CameraRig::home(
        layout,
        Canvas::hud(Vec2::new(window.width(), window.height())),
    );
    let current: CameraRig = *rig;
    let following = home.0.is_none_or(|last| current == last) || current == CameraRig::default();
    if following && current != next {
        *rig = next;
    }
    if home.0 != Some(next) {
        home.0 = Some(next);
    }
}

/// How far the camera stands off to the side, as a fraction of its height.
///
/// The table is read from above, and it has to be: a four-seat pod ring is
/// laid out for a plan view, and the further the camera leans the more a far
/// seat's cards shrink against a near seat's. But a *purely* top-down camera
/// throws away every cue that a card is an object — its edge projects to
/// nothing, its contact shadow hides underneath it, and the board reads as
/// artwork printed into the felt.
///
/// This is the compromise: about 14° off vertical, which is enough that a
/// card's edge and the shadow around it are both visible.
///
/// It was 22°, and the sentence above about a far seat's cards is why it is
/// not any more. Everything the lean costs is paid by the seat furthest from
/// the camera and collected by the seat nearest it, which on a free-for-all
/// is the player's own: three boards laid out exactly 12.0 units wide each
/// were drawn 450, 381 and 378 pixels wide, and the odd one out was the one
/// the player is looking straight at. A board is the thing a player compares
/// most often, so a table where their own is a fifth larger than everybody
/// else's is a table that has misreported the format.
///
/// Two terms make up that error and the lean drives both. A seat's own width
/// axis is turned away from the camera by its facing, and what is left of it
/// is `√(¼ + ¾cos²)` — pure foreshortening, there at any distance. And the
/// near seat stands `lean · cos · radius` closer to the eye than the ring's
/// centre while the far ones stand half that further away, which is
/// perspective and shrinks as the lens lengthens. Halving the lean and
/// halving the field of view together take a free-for-all from 18.9% to 6.3%,
/// and the seats that gave nothing up are the two opposite ones: it is the
/// player's own board that comes back to the size of theirs.
///
/// It has been 0.40, then 0.24, then 0.27, and is **0.36** — and this last
/// step is the one that cost something, so it is written down rather than
/// buried in a number. The owner has now asked three times for more angle on
/// the table, having been told once what it trades against; that is a
/// decision, not a misunderstanding, and it is theirs to make.
///
/// What it costs, measured rather than argued: at 0.36 the widest board on an
/// eight-seat ring is 10.6% wider than the narrowest, against 6.3% at 0.27.
/// The bound in `every_seat_is_drawn_a_board_of_the_same_width` moved from
/// 1.08 to 1.12 to allow exactly that and no more. The number that matters is
/// still the one the complaint was about — 18.9%, where a player reads their
/// own board as a different format — and 10.6% is well under half of it.
///
/// The lens buys nothing here and was left alone. Tried both ways: 0.34 at a
/// `FOV` of 0.36 spreads 8.2%, and so does 0.33 at 0.42 — the perspective
/// term a longer lens shrinks is the smaller of the two, and the
/// foreshortening term, which is the larger, does not care about the lens at
/// all. So the angle is paid for in width and in nothing else, which is the
/// honest way to sell it.
const CAMERA_LEAN: f32 = 0.36;

/// The camera's vertical field of view, in radians.
///
/// Shared with [`CameraRig::home`], which inverts the projection to work out
/// how far back the table has to stand — a framing computed against a
/// different angle from the one the camera is set to is a framing that misses.
///
/// 24° rather than the 40° it was, which is the other half of what
/// [`CAMERA_LEAN`] buys: the same table framed the same way, from twice as
/// far off through half the angle, so every seat sits at nearly the same
/// depth and is drawn at nearly the same size. It is a longer lens than a
/// room is normally seen through, and that is the point — a table is a thing
/// a player reads, not a room they stand in.
pub const FOV: f32 = 0.42;

/// Where the camera actually is, as against where the rig says it should be.
///
/// A second copy rather than smoothing the rig itself, because the rig is
/// *input*: a drag writes it, a zoom writes it, focusing a seat writes it,
/// and every one of those wants to be able to say "there" without having to
/// know that something else is interpolating behind it.
#[derive(Resource, Clone, Copy, Default)]
pub struct ShownRig(Option<CameraRig>);

/// Turns the rig into the camera transform: a near-plan view of the
/// battlefield canvas, leaning by [`CAMERA_LEAN`] so the cards have
/// somewhere to cast a shadow; the rig decides target, zoom, and azimuth.
///
/// The camera follows the rig rather than snapping to it, so tabbing to
/// another seat is a move across the table and not a cut. Yaw is interpolated
/// the short way around, or focusing the seat on your left would spin the
/// table three-quarters of the way round to reach it.
pub fn apply_camera_rig(
    rig: Res<CameraRig>,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut shown: ResMut<ShownRig>,
    mut cams: Query<&mut Transform, With<TableCamera>>,
) {
    let target = *rig;
    let current = match shown.0 {
        // The first frame is a cut by definition: there is nowhere to come
        // from. So is a table a player has asked to hold still.
        None => target,
        Some(_) if prefs.all().reduce_motion => target,
        Some(current) => {
            let t = 1.0 - (-CAMERA_SETTLE * time.delta_secs()).exp();
            let turn = std::f32::consts::TAU;
            let yaw_delta = (target.yaw - current.yaw + std::f32::consts::PI).rem_euclid(turn)
                - std::f32::consts::PI;
            CameraRig {
                target: current.target.lerp(target.target, t),
                distance: current.distance + (target.distance - current.distance) * t,
                yaw: current.yaw + yaw_delta * t,
                lean: current.lean + (target.lean - current.lean) * t,
            }
        }
    };
    // Nothing moved and nothing was asked for: the camera stands still most
    // of the time and should cost nothing then.
    if shown.0 == Some(current) && !rig.is_changed() {
        return;
    }
    shown.0 = Some(current);

    let eye = current.eye();
    for mut transform in &mut cams {
        *transform = eye;
    }
}

impl ShownRig {
    /// Where the camera stands *this frame*, or `None` before the first one.
    #[must_use]
    pub fn rig(self) -> Option<CameraRig> {
        self.0
    }
}

impl CameraRig {
    /// The camera transform this rig asks for.
    ///
    /// Extracted from [`apply_camera_rig`] rather than copied, because
    /// [`Lens`] projects with it and anything placed on the table by
    /// projection has to be placed through the *same* eye the camera was set
    /// from. Two derivations of one camera agree until the day one of them is
    /// edited.
    #[must_use]
    pub fn eye(self) -> Transform {
        let horizontal = self.distance * self.lean;
        let offset = Vec3::new(
            self.yaw.sin() * horizontal,
            self.distance,
            self.yaw.cos() * horizontal,
        );
        let look = Vec3::new(self.target.x, 0.0, self.target.y);
        Transform::from_translation(look + offset).looking_at(look, Vec3::Y)
    }
}

/// Where a point on the felt lands in the window, in logical pixels.
///
/// The seat bars are screen-space ink pinned to a rectangle on the table, so
/// something has to answer "where is that rectangle on screen" every frame.
/// It is written here, from a [`CameraRig`], rather than asked of Bevy's
/// `Camera::world_to_viewport`, for a scheduling reason worth stating: a
/// camera's `GlobalTransform` is propagated in `PostUpdate` **after**
/// `UiSystems::Layout` has already run (`bevy_ui` orders its layout
/// `.before(TransformSystems::Propagate)`), so a placement system reading the
/// propagated transform writes a `Node` position the layout will not look at
/// until the next frame. The ink would swim one frame behind the felt for as
/// long as the camera moved. Reading the rig instead, in `Update` and right
/// after [`apply_camera_rig`] has written it, is exact.
#[derive(Clone, Copy)]
pub struct Lens {
    clip_from_world: Mat4,
    window: Vec2,
}

impl Lens {
    /// The projection this rig gives onto a window of this size.
    #[must_use]
    pub fn new(rig: CameraRig, window: Vec2) -> Self {
        let aspect = (window.x / window.y.max(1.0)).max(1e-3);
        // `perspective_rh` rather than the reverse-infinite matrix Bevy's
        // `PerspectiveProjection` builds: they differ only in how z is
        // mapped, and nothing here reads z. The near and far planes are
        // therefore arbitrary and only have to bracket the table.
        let clip = Mat4::perspective_rh(FOV, aspect, 0.1, 1000.0);
        Self {
            clip_from_world: clip * rig.eye().to_matrix().inverse(),
            window,
        }
    }

    /// Where a point on the felt is drawn, or `None` if it is behind the eye.
    #[must_use]
    pub fn project(&self, table: Vec2) -> Option<Vec2> {
        let clip = self.clip_from_world * to_world(table, TABLE_Y).extend(1.0);
        if clip.w <= 1e-4 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new(
            ndc.x.mul_add(0.5, 0.5) * self.window.x,
            0.5f32.mul_add(-ndc.y, 0.5) * self.window.y,
        ))
    }

    /// Four table points projected, in the order they were given.
    ///
    /// `None` if any of them is behind the eye, because three corners of a
    /// rectangle are not a smaller rectangle — they are a bar drawn somewhere
    /// the shelf is not.
    #[must_use]
    pub fn corners(&self, points: [Vec2; 4]) -> Option<[Vec2; 4]> {
        let mut out = [Vec2::ZERO; 4];
        for (slot, point) in out.iter_mut().zip(points) {
            *slot = self.project(point)?;
        }
        Some(out)
    }
}

/// How quickly a card settles onto its mark, as a fraction of the remaining
/// distance per second.
///
/// Exponential rather than a fixed duration, because the thing being animated
/// is a *correction*: a card whose lane repacked by half a millimetre and a
/// card that just entered the battlefield are the same code path, and the
/// first must not take as long as the second. At 16 the long move reads as a
/// deal and the short one as a settle, which is what a hand on a real table
/// looks like.
const SETTLE: f32 = 16.0;

/// Below this, a card is simply put on its mark: the last hundredth of a
/// millimetre of an exponential curve is not worth a frame of work, and
/// leaving it unfinished is what makes a "still" board quietly never idle.
const SETTLED: f32 = 0.0008;

/// How far above the table a card appears before dropping onto it.
///
/// Direction-agnostic on purpose. A card could fly in from its owner's hand,
/// and at four seats around a ring that means four different directions and a
/// card that flies *across* two other players' boards to get home. Dropping
/// in reads as "this arrived" from every chair.
const ENTRANCE_RISE: f32 = 1.4;

/// How small a card is when it appears, before it settles to full size.
const ENTRANCE_SCALE: f32 = 0.86;

/// How quickly the camera settles, in the same units as [`SETTLE`].
///
/// Faster than the cards: a drag that lags behind the pointer feels broken,
/// while a card that snaps feels cheap. Same mechanism, different answer.
const CAMERA_SETTLE: f32 = 24.0;

/// A drawn card, and the group it stands for.
#[derive(Component)]
pub struct CardVisual {
    /// The object the card represents and that input reports.
    pub object: ObjectId,
    /// How many permanents it stands for.
    pub count: usize,
}

/// Where a card is going.
///
/// The scene diff writes the *target* and never the transform itself, so
/// every source of movement — a lane repacking, a tap, a hover, a card
/// entering play — arrives through one door and animates for free. It also
/// means the animation cannot desynchronise from the board model: there is
/// nothing to keep in step, because the target is recomputed from the model
/// every frame.
#[derive(Component, Clone, Copy)]
pub struct Motion {
    /// The transform the card belongs at right now.
    pub target: Transform,
}

/// Moves every card towards its mark.
///
/// Frame-rate independent: the fraction covered is `1 - e^(-rate · dt)`, so
/// the same motion plays out identically at 30 and at 144 frames per second.
/// A naive `lerp(0.2)` per frame does not — it makes the whole table twice as
/// fast on a better machine, which is the bug this shape exists to avoid.
pub fn glide(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut cards: Query<(&Motion, &mut Transform)>,
) {
    let still = prefs.all().reduce_motion;
    let t = 1.0 - (-SETTLE * time.delta_secs()).exp();
    for (motion, mut transform) in &mut cards {
        let there = transform
            .translation
            .distance_squared(motion.target.translation)
            < SETTLED * SETTLED
            && transform.rotation.angle_between(motion.target.rotation) < SETTLED
            && transform.scale.distance_squared(motion.target.scale) < SETTLED * SETTLED;
        if still || there {
            if *transform != motion.target {
                *transform = motion.target;
            }
            continue;
        }
        transform.translation = transform.translation.lerp(motion.target.translation, t);
        transform.rotation = transform.rotation.slerp(motion.target.rotation, t);
        transform.scale = transform.scale.lerp(motion.target.scale, t);
    }
}

/// Where a card is when it first appears: above its mark, and a little small.
fn entrance(target: &Transform) -> Transform {
    let mut start = *target;
    start.translation.y += ENTRANCE_RISE;
    start.scale *= ENTRANCE_SCALE;
    start
}

/// How long a card that has left the board is kept on the table.
///
/// Long enough for [`glide`] to carry it all but the last thousandth of the
/// way to its exit pose at [`SETTLE`], and short enough that a board being
/// swept does not leave a drift of ghosts behind it. A player who has turned
/// motion off never sees any of it: `glide` puts the card on its mark in one
/// frame, and what is left is a rectangle under a pile or a speck above the
/// felt for half a second.
const EXIT_LIFE: f32 = 0.55;

/// How far under a pile's own card a card joining that pile slides.
///
/// A pile draws exactly one card — its top — so a second card arriving there
/// has to end up *behind* it or the two fight for the same depth. Half a
/// millimetre is enough for that and small enough that the card is hidden
/// rather than merely lower.
const PILE_TUCK: f32 = 0.0005;

/// What a card shrinks to when it leaves for somewhere with no floor.
const VANISH_SCALE: f32 = 0.02;

/// How far a bounced card rises on its way off the table.
///
/// Larger than [`ENTRANCE_RISE`], because this is the entrance played
/// backwards and the card has to be *gone* by the time it is despawned rather
/// than merely high.
const BOUNCE_RISE: f32 = 2.6;

/// A card that has left the board and is playing its way off it.
///
/// It is out of [`SceneIndex::cards`] and has lost its [`CardVisual`] the
/// moment it is marked, so nothing looks it up any more: no hover, no preview,
/// no combat line, no click. All that is left is a [`Motion`] target it is
/// gliding towards and the time it has to get there — the component moves
/// nothing itself, because everything on this table moves through one door.
#[derive(Component, Clone, Copy)]
pub struct Departing {
    /// Seconds left before it is despawned.
    pub left: f32,
}

/// Where the cards of one seat's pile stand, when that pile is drawn.
///
/// `None` for a place that has no pile on this table — the battlefield, the
/// stack, a hand — and for a seat this layout has no slot for. It is the same
/// call [`placements`] makes for a pile's top card, so a card sent here is
/// sent to the pile it is really in and not to an approximation of it.
fn pile_stand(duel: &Duel, place: Option<Place>) -> Option<Transform> {
    let (player, kind) = match place? {
        Place::Graveyard(player) => (player, PileKind::Graveyard),
        Place::Exile(player) => (player, PileKind::Exile),
        // The second commander has a slot of its own, and this sends a
        // partner to the first one. A card is under the pile or despawned by
        // the time it matters, so the two slots are half a card apart for a
        // fraction of a second and never at rest.
        Place::Command(player) => (player, PileKind::Command),
        Place::Battlefield | Place::Stack | Place::Hand => return None,
    };
    let slot = duel.layout.as_ref()?.slot(player)?;
    Some(card_transform(slot, slot.pile_center(kind), false, 0.0))
}

/// Where a card goes as it leaves the table, by where it went.
///
/// Three exits, and which one is taken is decided by `stand` first: a card
/// bound for a pile this table draws glides *to that pile* and slides under
/// its top card, which is what the pile's own top card is doing on the same
/// frame through the update-in-place branch of [`sync_scene`]. Two creatures
/// dying together must not be treated differently for the accident of which
/// of them ends up on top.
///
/// The other two are for zones with no place on the felt. A bounce is the
/// arrival run backwards. A card this seat cannot follow at all — put on the
/// bottom of a library, or into an opponent's hand — gets the neutral shrink,
/// because the only thing that can be said about it is that it is no longer
/// here; see [`baylee_client_core::zones`] for why that is not guessed at.
fn exit(to: Option<Place>, stand: Option<Transform>, from: &Transform) -> Transform {
    if let Some(pile) = stand {
        let mut out = pile;
        out.translation.y -= PILE_TUCK;
        return out;
    }
    let mut out = *from;
    match to {
        Some(Place::Hand) => {
            out.translation.y += BOUNCE_RISE;
            out.scale *= ENTRANCE_SCALE * 0.5;
        }
        _ => out.scale *= VANISH_SCALE,
    }
    out
}

/// Where a card is when it first appears, by where it came from.
///
/// A card coming back from a pile starts *on* that pile, so a resurrection
/// and a flicker are the pile-bound exit run backwards. Every other arrival is
/// the one this table has always drawn: a creature cast from hand comes from
/// the stack, a token comes from nowhere at all, and both of them belong
/// dropping onto their mark.
///
/// This is the branch a *buried* card takes. A card that was its pile's top
/// was already on the table and glides home through the update-in-place
/// branch, from the same point this returns — which is the whole reason the
/// two are one call.
fn entrance_from(stand: Option<Transform>, target: &Transform) -> Transform {
    stand.unwrap_or_else(|| entrance(target))
}

/// Puts a departing card's door on the material it is already wearing.
///
/// Answers the handle to wear instead, or `None` for a card that is leaving
/// through no door worth drawing — most of them: a stale group re-keying, a
/// permanent going somewhere this seat cannot see, a card leaving on a table
/// where the player has turned motion off.
///
/// A function of its own rather than six lines inside [`sync_scene`], and the
/// reason is that this is the one frame on which it can happen at all. The
/// card is out of the board model and out of [`SceneIndex::cards`] by the
/// time this runs, so nothing will ever build it a look again — which is why
/// its exit is *written on to* the material it has rather than looked up by a
/// [`CardLook`], and why a version of this that quietly did nothing would be
/// invisible in every test that goes through the cache.
fn dress_the_exit(
    materials: &mut Assets<CardMaterial>,
    worn: &Handle<CardMaterial>,
    step: zones::Move,
    now: f32,
    motion: f32,
) -> Option<Handle<CardMaterial>> {
    let door = step.passage().filter(|door| door.is_departure())?;
    let mut leaving = materials.get(worn).cloned()?;
    // `wear` makes the decision a card holding still makes, in the one place
    // an arrival makes it too: no sweep at all, rather than a sweep on a
    // stopped clock.
    crate::cardmat::wear(
        &mut leaving.params,
        Some(crate::sheen::Sweep::leaving(now, door, EXIT_LIFE)),
        motion,
    );
    Some(materials.add(leaving))
}

/// Despawns cards that have finished leaving.
///
/// A plain countdown rather than [`glide`]'s settled test, because one exit
/// ends at a scale of nearly zero and one ends behind another card: "has it
/// arrived" is the wrong question for a card whose destination is nowhere.
pub fn retire(
    time: Res<Time>,
    mut commands: Commands,
    mut leaving: Query<(Entity, &mut Departing)>,
) {
    for (entity, mut departing) in &mut leaving {
        departing.left -= time.delta_secs();
        if departing.left <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Where every object was in the view before this one, and what has moved
/// since anything last drew.
///
/// A resource of its own rather than a field on [`SceneIndex`], because the
/// two answer different questions and are cleared at different moments: the
/// index is what the scene *is*, this is what the game was. Both are wiped
/// when a table is torn down.
///
/// It holds the moves as well as the tracker because a view produces its
/// batch **once** — `Tracker::observe` answers a view it has already read
/// with nothing — and two systems need that batch: [`crate::sheen`] to say
/// which door an arriving card came in through, and [`sync_scene`] to send a
/// departing one out of the right one. Whichever asks first fills the list;
/// only `sync_scene` empties it, and only once it is past the guards that
/// would otherwise swallow the batch a bailed frame was holding.
#[derive(Resource, Default)]
pub struct ZoneWatch {
    tracker: Tracker,
    undrawn: Vec<zones::Move>,
}

impl ZoneWatch {
    /// Reads a view, if it has not been read, and keeps whatever it said.
    pub fn observe(&mut self, view: &baylee_view::PlayerView) {
        self.undrawn.extend(self.tracker.observe(view));
    }

    /// The moves nothing has drawn yet.
    #[must_use]
    pub fn undrawn(&self) -> &[zones::Move] {
        &self.undrawn
    }

    /// The moves nothing has drawn yet, and they are drawn now.
    pub fn take(&mut self) -> Vec<zones::Move> {
        std::mem::take(&mut self.undrawn)
    }

    /// Forgets everything, for a table that is being torn down.
    pub fn clear(&mut self) {
        self.tracker.clear();
        self.undrawn.clear();
    }
}

/// Entities currently drawn, keyed by the object they represent.
#[derive(Resource, Default)]
pub struct SceneIndex {
    cards: HashMap<ObjectId, Entity>,
    /// One material per *look*, shared by every card wearing it — a board of
    /// forty plain Islands is one material, not forty. A foil Island is a
    /// second, and an Island the rules have made indestructible is a third
    /// until it stops being one: those are the differences the shader draws,
    /// so they are exactly the differences the key carries.
    materials: HashMap<CardLook, Handle<CardMaterial>>,
    quad: Option<Handle<Mesh>>,
    blank: Option<Handle<CardMaterial>>,
    /// Text entities of the constructed face, per card currently showing one,
    /// with the snapshot they were built from.
    ///
    /// Held here rather than found by query because the face comes and goes
    /// with a held key: the entities have to be removed as cheaply as they
    /// were made, and a card that no longer wants one must not keep a stale
    /// line of text glued to it. The sequence number is what rebuilds a face
    /// whose card changed (an anthem, a counter, a clone) without rebuilding
    /// every face every frame.
    faces: HashMap<ObjectId, (u64, Vec<Entity>)>,
    /// One material per colour identity and look, for cards drawing their
    /// own face rather than artwork.
    face_materials: HashMap<CardLook, Handle<CardMaterial>>,
    /// Whether the two caches above were filled to hold still.
    ///
    /// The same trick as [`UiCardMaterials`](crate::cardmat::UiCardMaterials):
    /// `false` is "animated", so the derived `Default` is the right answer,
    /// and the setting lives beside the cache instead of inside its key —
    /// which is the whole point of keeping it here. Were it in the key, every
    /// card on the table would be two entries instead of one and nothing
    /// would ever evict the half no longer wanted.
    ///
    /// A change rewrites the clock on the materials already made rather than
    /// throwing them away. Emptying the maps is the obvious thing to write
    /// and it is strictly worse: the handles are still held by every card
    /// entity, so the discarded materials do not go anywhere — they are
    /// merely rebuilt, once each, on the next frame.
    still: bool,
    /// Whether [`Self::blank`] is wearing the printed card back yet.
    ///
    /// `false` until the picture has arrived, which is also the right answer
    /// for a client that never reaches Scryfall at all: the flat colour is
    /// what it goes on drawing.
    back_dressed: bool,
    /// A seat's zone: the mat and the glow under it, with the mood they were
    /// last drawn in. Held here for the same reason the cards are — so a
    /// frame in which nothing changed costs a lookup and no allocation.
    zones: HashMap<PlayerId, Zone>,
    /// The soft glow every zone shares. It is white with the falloff in its
    /// alpha, so one image serves the whole table and the seat's colour is
    /// the material's tint.
    ///
    /// The mat is *not* here, and used to be. Sharing one image meant the
    /// seat's colour could only be applied as a tint over the whole of it,
    /// which is how a gilt-rimmed board became a sheet of brass; the rim
    /// carries the colour now, so the image is per seat and lives on the
    /// [`Zone`].
    glow_image: Option<Handle<Image>>,
    /// The contact shadow every card sits in: one quad, one material, shared
    /// by the whole table. It is a child of the card, so it follows the tap
    /// rotation and the hover lift with nothing to keep in step.
    shadow_quad: Option<Handle<Mesh>>,
    shadow_material: Option<Handle<StandardMaterial>>,
    /// The empty place a pile stands in, one material per
    /// [`baylee_client_core::PileKind`] in `ALL` order.
    ///
    /// Shared by the whole table, because a well is table furniture and
    /// carries no seat's colour. One material per *kind* rather than one for
    /// all of them because each carries its zone's own mark baked into the
    /// texture — the table is 3D and has no text on it, so the mark is
    /// arithmetic in the well rather than a label over it.
    wells: Vec<Handle<StandardMaterial>>,
}

/// One seat's zone on the table.
struct Zone {
    /// The place this zone was built for.
    ///
    /// Everything below is *geometry*: a mesh cut to the mat's size, a glow
    /// quad under it, and pile places at fixed points beside it. None of it
    /// is a uniform that can be rewritten, so a seat whose slot has moved or
    /// changed size has to be built again — and until this was here it never
    /// was. The layout is legitimately rebuilt more than once (the first one
    /// is drawn against a guessed canvas aspect, and focusing a seat widens
    /// it and shrinks the rest), so a table laid out a second time drew every
    /// mat at the first layout's size while the cards moved to the second.
    /// Measured: the local seat's half width was `9.864` when its zone was
    /// built and `12.841` when its commander was placed, which is a card
    /// standing two and a half card widths outside the mat it belongs to.
    slot: SeatSlot,
    /// The mat itself.
    mat: Entity,
    /// The pool of colour under it.
    glow: Entity,
    /// Their materials, held rather than looked back up off the entities.
    ///
    /// The two used to be found with a `Query<&MeshMaterial3d<_>>`, which was
    /// a system parameter for something this already knows: it spawned both
    /// of them. Now that they are two *different* material types that query
    /// would have had to be two queries, and `sync_zones` is at clippy's
    /// argument budget.
    mat_material: Handle<crate::matmat::MatMaterial>,
    glow_material: Handle<StandardMaterial>,
    /// The pile places beside it, and the face-down library standing on one
    /// of them. Empty when the scene index has no card mesh yet.
    piles: Vec<Entity>,
    /// How tall the library was drawn, in cards.
    ///
    /// Capped at the height a pile is ever drawn to, so a deck that is
    /// visibly shrinking is rebuilt as it shrinks and one that is far past
    /// the cap is not rebuilt at all. It used to be "had the library run
    /// out" alone, which is the one fact the *places* depend on and says
    /// nothing about the deck standing on one of them.
    library_shown: u32,
    /// What the mat was last drawn for.
    mood: Mood,
    /// The seat colour in the mat's rim.
    ///
    /// Kept so a seat whose accent changes is redrawn rather than keeping the
    /// colour it was born with. [`seat_accent`] reads `is_local` and
    /// `ring_index`, both of which are stable while a seat is at the table —
    /// but "stable in practice" is exactly the assumption that put a stale
    /// hover and an over-tall panel on screen this week, and a comparison is
    /// cheaper than being right about it.
    accent: Color,
}

/// What a zone's colour is saying.
///
/// The rim of a seat's mat is the cheapest place to answer "whose turn is
/// it?" and "who is holding everyone up?" — questions a player asks on every
/// single priority pass, and that otherwise cost a trip to the overlay.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Mood {
    /// Whether this is the viewing seat.
    local: bool,
    /// Where the seat stands in the turn.
    standing: Standing,
    /// Whether the turn is this seat's, which is a different question from
    /// [`standing`](Self::standing) and has to be kept beside it.
    ///
    /// `Standing` is a rank and collapses the two: a seat holding priority
    /// reads as `Priority` whether or not the turn is theirs, because what
    /// the *brightness* answers is "who is everybody waiting for". The light
    /// running round a mat's rim answers "whose turn is it", and on every
    /// turn where an opponent responds to something the two have different
    /// answers — so a rim light driven off the rank would leave the active
    /// seat and follow the response, which is precisely backwards.
    on_turn: bool,
}

/// What a seat is doing, in the order the zone cares about it.
///
/// Ordered rather than flagged because these do not stack: a seat holding
/// priority is *also* the active seat nine times out of ten, and drawing both
/// would only mean adding two brightnesses together and hoping.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Standing {
    /// Out of the game.
    Lost,
    /// Holding priority — this is the seat everyone else is waiting for.
    Priority,
    /// Their turn, but not currently holding anyone up.
    Active,
    /// Waiting their turn.
    Waiting,
}

impl Mood {
    /// How a seat's pod reads right now.
    fn of(pod: &baylee_client_core::board::SeatPod) -> Self {
        Self {
            local: pod.is_local,
            standing: if pod.has_lost {
                Standing::Lost
            } else if pod.has_priority {
                Standing::Priority
            } else if pod.is_active {
                Standing::Active
            } else {
                Standing::Waiting
            },
            // A seat that is out of the game is not taking a turn, whatever
            // the view last said about the active player.
            on_turn: pod.is_active && !pod.has_lost,
        }
    }
}

/// A card's corner radius.
///
/// A real card is 63 mm wide with a 3 mm corner, and this is exactly that —
/// 4.76%, the same `PRINTED_CORNER` the two card shaders cut at. The geometry
/// takes the scanner's white corner away and the shader inks the sliver of
/// pixels the mesh edge antialiases through, which only works while both
/// agree: a mesh cut wider than the print leaves the ink nothing to do, and a
/// mesh cut narrower shows white outside it. It used to be 10%, which removed
/// the white by removing a tenth of the card with it and made every permanent
/// read as a token.
pub const CARD_CORNER: f32 = CARD_WIDTH * 0.0476;

/// How thick a card is, in table units.
///
/// A real card at this scale is about a fiftieth of this — it would be a
/// single pixel at any camera distance a player uses. The point of the
/// thickness is not accuracy but that a card reads as an *object lying on*
/// the table rather than a decal printed into it, so it is exaggerated until
/// the edge is visible and stopped well before a card looks like a tile.
pub const CARD_THICKNESS: f32 = CARD_WIDTH * 0.055;

/// How far past the card its contact shadow spreads, as a fraction of the
/// card's width.
const SHADOW_SPREAD: f32 = 0.22;

/// A card: a rounded rectangle with the printed face on top and a thin wall
/// around its edge.
///
/// The face is UV-mapped exactly like Bevy's `Rectangle` (uv.x left→right,
/// uv.y top→bottom of the printed face) and the wall borrows the UV of the
/// face vertex above it — so a card's edge is whatever colour its border is,
/// which for most cards is the black frame and reads as exactly the right
/// thing. There is no bottom face: the camera rig never goes below the table,
/// and two hundred cards is four hundred triangles worth saving.
fn rounded_card_mesh(width: f32, height: f32, radius: f32) -> Mesh {
    rounded_slab_mesh(width, height, radius, CARD_THICKNESS, 0.0, 4)
}

/// The same slab at any thickness, and with any number of corner segments.
///
/// The table is built by this too, and the two want opposite things from it:
/// a card is 63 mm with a 3 mm corner and four segments is more than the eye
/// can find, while the table is a **racetrack** whose corners are metres
/// across and would read as a chamfer at four. `top` and `bottom` are the two
/// heights the wall runs between, so a card sits *on* the plane it is placed
/// at (`CARD_THICKNESS` to 0) and the table hangs *below* it (0 to
/// `-TABLE_THICKNESS`) — which is what keeps the table's surface exactly
/// where every other thing on the stage is positioned against.
fn rounded_slab_mesh(
    width: f32,
    height: f32,
    radius: f32,
    top: f32,
    bottom: f32,
    segments: usize,
) -> Mesh {
    let segments = segments.max(1);
    let (hw, hh) = (width / 2.0, height / 2.0);
    // A radius past half the short side has no shape left to describe, and
    // would fold the outline through itself exactly the way the bowtie did.
    let r = radius.clamp(0.0, hw.min(hh));
    // Corner arc centres in CCW order with the quarter turn each one sweeps,
    // angles measured the usual way (0° = +x, 90° = +y).
    //
    // Every centre owns the quarter that points *away* from the middle of the
    // card, and it has to: pair a centre with any other quarter and the
    // outline folds back through the centre, so the fan below stitches
    // crossing slivers instead of a card. On a table that reads as a small
    // bright X where a permanent should be.
    let corners: [([f32; 2], f32); 4] = [
        ([hw - r, hh - r], 90.0),
        ([-hw + r, hh - r], 180.0),
        ([-hw + r, -hh + r], 270.0),
        ([hw - r, -hh + r], 360.0),
    ];
    // The outline, with the outward direction at each point — which for an
    // arc point is simply the angle it was drawn at, and is what the wall's
    // normals are.
    let mut outline: Vec<([f32; 2], [f32; 2])> = Vec::new();
    for ([cx, cy], end_deg) in corners {
        let start_deg = end_deg - 90.0;
        for i in 0..=segments {
            #[expect(clippy::cast_precision_loss)] // a handful of segments
            let a = (start_deg + (end_deg - start_deg) * i as f32 / segments as f32).to_radians();
            let (dx, dy) = (a.cos(), a.sin());
            outline.push(([cx + r * dx, cy + r * dy], [dx, dy]));
        }
    }

    // Same mapping as Rectangle: [hw,hh]→[1,0], [-hw,-hh]→[0,1].
    //
    // Clamped, because the outline is built as `centre + r·cos θ`, and at
    // θ = 0 that is `hw - r + r`, which in binary is not always `hw`. A UV a
    // ten-millionth outside the texture samples the wrap or the clamp
    // depending on the backend, so the card would grow a bright thread down
    // one edge on exactly one machine.
    let uv_of = |x: f32, y: f32| {
        [
            f32::midpoint(x / hw, 1.0).clamp(0.0, 1.0),
            ((1.0 - y / hh) * 0.5).clamp(0.0, 1.0),
        ]
    };

    // The face: a centre vertex and the outline, all facing straight up.
    let mut positions: Vec<[f32; 3]> = vec![[0.0, 0.0, top]];
    let mut normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]];
    let mut uvs: Vec<[f32; 2]> = vec![[0.5, 0.5]];
    for ([x, y], _) in &outline {
        positions.push([*x, *y, top]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push(uv_of(*x, *y));
    }
    // A triangle fan from the centre, wound counter-clockwise as seen from
    // +z — the side the printed face is on, and the side the camera is on
    // once the card is laid down. The material does not disable back-face
    // culling, so the other winding is an invisible card.
    let m = outline.len();
    let mut indices = Vec::with_capacity(m * 9);
    for i in 1..m {
        indices.extend_from_slice(&[0, i as u32, i as u32 + 1]);
    }
    indices.extend_from_slice(&[0, m as u32, 1]);

    // The wall: the outline again at both heights, with its own normals, so
    // the face above it keeps a clean flat shade.
    let wall = positions.len() as u32;
    for ([x, y], [nx, ny]) in &outline {
        positions.push([*x, *y, top]);
        normals.push([*nx, *ny, 0.0]);
        uvs.push(uv_of(*x, *y));
    }
    for ([x, y], [nx, ny]) in &outline {
        positions.push([*x, *y, bottom]);
        normals.push([*nx, *ny, 0.0]);
        uvs.push(uv_of(*x, *y));
    }
    for i in 0..m {
        let next = (i + 1) % m;
        let (t0, t1) = (wall + i as u32, wall + next as u32);
        let (b0, b1) = (t0 + m as u32, t1 + m as u32);
        // Down the near edge, along the bottom, back up: that is the order
        // whose normal points away from the card. The other one is a card you
        // can see straight through from the side.
        indices.extend_from_slice(&[t0, b0, b1, t0, b1, t1]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(indices))
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
}

/// Builds the stage: camera, light, and felt.
pub fn spawn_stage(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cards: ResMut<Assets<CardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut index: ResMut<SceneIndex>,
) {
    index.quad = Some(meshes.add(rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER)));

    // The contact shadow: a quad a little larger than a card, carrying a
    // painted halo that is dense under the card and gone by its own edge.
    // Sized from the card so the falloff is the same width on all four sides
    // — a square texture stretched over a card-shaped quad would be wider at
    // the top than at the sides, which is the sort of thing nobody can name
    // but everybody sees.
    let spread = CARD_WIDTH * SHADOW_SPREAD;
    let (shadow_w, shadow_h) = (CARD_WIDTH + 2.0 * spread, CARD_HEIGHT + 2.0 * spread);
    index.shadow_quad = Some(meshes.add(Rectangle::new(shadow_w, shadow_h)));
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "a texture size derived from two positive constants"
    )]
    let shadow_px = (128.0 * shadow_h / shadow_w).round() as u32;
    index.shadow_material = Some(materials.add(StandardMaterial {
        base_color_texture: Some(images.add(image_of(&tabletop::card_shadow(
            128,
            shadow_px,
            spread / shadow_w,
            CARD_CORNER / CARD_WIDTH,
        )))),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    }));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a texture size derived from two positive constants"
    )]
    let well_px = (f32::from(u16::try_from(RECESS_PX).unwrap_or(u16::MAX)) * CARD_HEIGHT
        / CARD_WIDTH)
        .round() as u32;
    index.wells = baylee_client_core::PileKind::ALL
        .into_iter()
        .map(|pile| {
            materials.add(StandardMaterial {
                base_color_texture: Some(images.add(image_of(&tabletop::card_well(
                    RECESS_PX,
                    well_px,
                    CARD_CORNER / CARD_WIDTH,
                    pile.mark(),
                )))),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })
        })
        .collect();
    // The card back: no finish, no glow, and no picture *yet*. It is what a
    // library is drawn as, what the stack behind a counted group is made of,
    // and what a card this seat may not see wears. The printed back is
    // fetched like any other image, so `sync_scene` dresses this material in
    // it the frame it lands; until then it is the flat colour below.
    // A second duel in one session gets a second material, and the flag is
    // about *this* one: left standing, the new material would never be
    // dressed and every hidden card would go back to being a dark rectangle.
    index.back_dressed = false;
    index.blank = Some(cards.add(material(
        CardLook::flat(BACK_COLOR, FinishTreatment::Plain, 0),
        None,
        BACK_COLOR,
        // No finish and no glow: the clock drives the foil sheen and a card
        // back is plain, so there is nothing on this material for it to
        // reach even once the picture is on it. That is why this one is
        // exempt from the cache the two below live in, and why dressing it
        // in the back is a change to the material rather than a new one.
        MOVING,
    )));

    commands.spawn((
        DuelStage,
        TableCamera,
        Camera3d::default(),
        // Looking down the table from behind the local seat (the default
        // rig; apply_camera_rig owns the transform from here on).
        Transform::from_xyz(0.0, 15.0, 13.2).looking_at(Vec3::ZERO, Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV,
            ..default()
        }),
        // No tone mapping. Bevy attaches none to a camera by default, so
        // this is belt and braces rather than a fix — but it is the right
        // thing to say out loud: everything in this scene is unlit and
        // display-referred (a generated texture says what the table should
        // *look* like, and a card's art is the same PNG the hand draws
        // unaltered through the UI pass), so a tone mapper reading those
        // numbers as radiance would be wrong. Naming it here stops a future
        // default from quietly doing that.
        Tonemapping::None,
    ));

    // Nothing below is lit, and nothing above it is either: card art must
    // never be tinted by scene lighting, because a player has to be able to
    // read a card's colour identity at a glance. The table gets its depth
    // from painted-in shading instead — which is what `tabletop` generates,
    // and why this stage has no light in it at all.
    index.glow_image = Some(images.add(image_of(&tabletop::glow(128))));

    // The slab itself — the baize and the rail around it — is not spawned
    // here. It is cut to the layout, and at this point there may not be one:
    // `sync_table` makes it the first time a board arrives and re-cuts it
    // whenever the seating or the window changes.

    // The medallion inlaid at the centre — the colour wheel every player
    // already has in their head, which is what makes it orientation rather
    // than decoration. It sits in the middle of the table, which is the one
    // patch of felt no seat ever plays on.
    commands.spawn((
        DuelStage,
        Mesh3d(meshes.add(Rectangle::new(MEDALLION_SIZE, MEDALLION_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(image_of(&tabletop::medallion(512)))),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, TABLE_Y + MEDALLION_LIFT, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
}

/// Wraps a generated texture in an `Image` the renderer can bind.
pub(crate) fn image_of(texture: &tabletop::Texture) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: texture.width,
            height: texture.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texture.rgba.clone(),
        // sRGB: the generator writes what the table should *look* like, not
        // light values, so the samples are display-referred.
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // The mat and the medallion are stretched over quads much larger than
    // they are; without a linear filter their soft edges come out as stairs.
    image.sampler = bevy::image::ImageSampler::linear();
    image
}

/// The colour a seat's zone is drawn in.
///
/// The viewing seat is gilt, matching the medallion's rings: whatever else is
/// on the table, "mine" is the one edge a player never has to look for. The
/// others take the colours of the pie in ring order, which makes a four-way
/// game four distinguishable places rather than three anonymous opponents.
fn seat_accent(slot: &SeatSlot) -> Color {
    if slot.is_local {
        return Color::srgb(0.78, 0.63, 0.33);
    }
    let hue = tabletop::PIE[(slot.ring_index + 3) % tabletop::PIE.len()];
    Color::srgb(hue[0], hue[1], hue[2])
}

/// One flat thing lying on the table.
///
/// A struct rather than four more parameters, because it already needs a seat
/// and two asset stores and clippy's argument budget is seven.
///
/// It used to describe both halves of a zone. The mat is drawn by its own
/// material now — see [`crate::matmat`] — so what still comes through here is
/// the glow underneath, which is a soft round falloff with no edge in it and
/// therefore the one case a stretched image is actually good at.
struct TableQuad {
    /// Extent on the felt, in table units.
    size: Vec2,
    /// How far above [`TABLE_Y`] it lies. The order of these decides what
    /// draws over what; nothing here is depth-sorted.
    lift: f32,
    /// Multiplied into the texture, so a white texel comes out this colour.
    tint: LinearRgba,
    /// The image, whose alpha is the shape.
    texture: Handle<Image>,
}

/// Spawns one of them, lying flat and facing its seat.
///
/// Returns the material beside the entity, because the caller keeps it: a
/// zone re-tints what it spawned rather than looking it back up through a
/// query on an entity it already has in hand.
fn spawn_table_quad(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    slot: &SeatSlot,
    quad: TableQuad,
) -> (Entity, Handle<StandardMaterial>) {
    let material = materials.add(StandardMaterial {
        base_color: Color::LinearRgba(quad.tint),
        base_color_texture: Some(quad.texture),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let entity = commands
        .spawn((
            DuelStage,
            Mesh3d(meshes.add(Rectangle::new(quad.size.x, quad.size.y))),
            MeshMaterial3d(material.clone()),
            // Ground, glow and medallion are all scenery. Only cards are
            // pointed at, so only cards are pickable.
            Pickable::IGNORE,
            lying_flat(slot, quad.lift),
        ))
        .id();
    (entity, material)
}

/// Where a flat thing lying on a seat's ground stands: on the seat's centre,
/// turned to face it, `lift` above the table.
///
/// One function because the mat and the glow are drawn by two different
/// materials now and would otherwise write the same transform twice — and a
/// mat and the pool of light under it that disagreed by a rotation would be
/// very hard to see and impossible to miss.
fn lying_flat(slot: &SeatSlot, lift: f32) -> Transform {
    Transform {
        translation: to_world(slot.center, TABLE_Y + lift),
        rotation: Quat::from_rotation_y(-slot.facing)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        scale: Vec3::ONE,
    }
}

/// How wide the texture of a pile's empty place is drawn.
///
/// The height follows from the card's aspect, so the lip's corners stay round
/// rather than becoming ellipses.
const RECESS_PX: u32 = 128;

/// How far below a real card a pile's empty place lies.
const RECESS_LIFT: f32 = -CARD_LIFT * 0.5;

/// One seat's four pile places, and the face-down library standing on one.
///
/// Drawn here rather than as placements because neither has an object behind
/// it: a library is face down to everybody, its owner included (CR 401.2),
/// and an empty pile has no card in it at all. What both of them are is a
/// *place* — and a place that appeared and vanished as cards moved through it
/// would make the table rearrange itself in the middle of a game.
fn spawn_piles(
    commands: &mut Commands,
    index: &SceneIndex,
    slot: &SeatSlot,
    piles: &[baylee_client_core::ZonePile],
    library: u32,
) -> Vec<Entity> {
    let Some(quad) = index.quad.clone() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for pile in piles {
        // A kind with no well is a kind added to the model and not to the
        // startup that builds their materials; skipping it draws no place
        // rather than drawing the wrong one.
        let Some(recess) = well_of(index, pile.kind) else {
            continue;
        };
        let at = slot.pile_center(pile.kind);
        out.push(
            commands
                .spawn((
                    DuelStage,
                    Mesh3d(quad.clone()),
                    MeshMaterial3d(recess),
                    card_transform(slot, at, false, RECESS_LIFT),
                    // A click near a pile's empty place means the table, the
                    // same as a click near the edge of a card does.
                    Pickable::IGNORE,
                ))
                .id(),
        );
    }

    // The library, as the deck it is: face down, and as tall as it has cards
    // in it up to [`MAX_STACK_RISE`]. The exact count is on the seat's tab —
    // a physical library does not tell you how many cards are in it either —
    // but its *thickness* is the one thing a real one does say across a
    // table, and a seat playing down to nothing should be able to watch it
    // happen.
    if library > 0
        && piles
            .iter()
            .any(|p| p.kind == baylee_client_core::PileKind::Library)
    {
        let at = slot.pile_center(baylee_client_core::PileKind::Library);
        let under = library as usize;
        let layers = stack_layers(under).max(1);
        let rise = stack_rise(under);
        for i in 0..layers {
            let lift = rise * i as f32 / layers as f32;
            out.push(
                commands
                    .spawn((
                        DuelStage,
                        Mesh3d(quad.clone()),
                        MeshMaterial3d(index.blank.clone().unwrap_or_default()),
                        card_transform(slot, at, false, lift),
                        Pickable::IGNORE,
                    ))
                    .id(),
            );
        }
    }
    out
}

/// The well material for one pile kind.
fn well_of(
    index: &SceneIndex,
    kind: baylee_client_core::PileKind,
) -> Option<Handle<StandardMaterial>> {
    let at = baylee_client_core::PileKind::ALL
        .iter()
        .position(|k| *k == kind)?;
    index.wells.get(at).cloned()
}

/// One seat's mat, as the handful of numbers its shader draws it from.
///
/// The size is the mat's own, in table units, and that is the difference the
/// whole material exists for: the corner and the rim are lengths on this
/// board rather than fractions of an image that was then stretched over it.
fn mat_params(
    accent: Color,
    size: Vec2,
    mood: Mood,
    moving: bool,
    ledge_outer: bool,
) -> crate::matmat::MatParams {
    let rgb = accent.to_linear();
    crate::matmat::MatParams {
        accent: Vec4::new(rgb.red, rgb.green, rgb.blue, zone_brightness(mood)),
        size,
        corner: tabletop::MAT_CORNER,
        rim: tabletop::MAT_RIM,
        on_turn: if mood.on_turn { 1.0 } else { 0.0 },
        motion: if moving {
            crate::cardmat::MOVING
        } else {
            crate::cardmat::STILL
        },
        ledge_outer: if ledge_outer { 1.0 } else { 0.0 },
    }
}

/// How bright a zone's mat is drawn, given what it is saying.
///
/// A seat that has lost fades most of the way out — its permanents are gone
/// and its zone should stop competing for attention — and a seat holding
/// priority is the brightest thing on the felt, because that is the seat
/// everyone else is waiting for.
fn zone_brightness(mood: Mood) -> f32 {
    // Every value here is a multiplier on the mat's **opacity**, so 1.0 is
    // the ceiling and anything past it is not brighter, it is clipped. It has
    // been the ceiling since the accent moved off the material's tint and
    // into the mat itself: at 1.311 and 1.0925 a local seat holding priority
    // and a local seat merely taking its turn would be drawn identically,
    // which is precisely the distinction the mat exists to draw.
    //
    // Opacity rather than colour is what the shader does with it, and that
    // reading is the one the numbers were chosen for anyway: a seat that has
    // lost fades *into* the felt at 0.22, where scaling a colour would have
    // left it drawing a dark grey rim just as visible as everybody else's.
    let standing = match mood.standing {
        Standing::Lost => 0.22,
        Standing::Waiting => 0.62,
        Standing::Active => 0.78,
        Standing::Priority => 1.0,
    };
    // Being the viewing seat is a lift, never a rank. Which mat is mine is
    // answered by the gilt rim and does not need brightness spent on it, and
    // a `local` term big enough to outrank a standing would let my own idle
    // mat outshine the opponent everybody is actually waiting for.
    if mood.local {
        (standing * LOCAL_LIFT).min(1.0)
    } else {
        standing
    }
}

/// How much brighter the viewing seat's own mat is drawn at equal standing.
///
/// Small on purpose: see [`zone_brightness`]. The bound that keeps it honest
/// is `zone_tests::a_standing_always_outranks_being_the_local_seat`.
const LOCAL_LIFT: f32 = 1.10;

/// Cuts the slab to the table and keeps the rail lit by the turn.
///
/// Two jobs in one system because they need the same three things — the
/// layout, the board and the slab entity — and because the second is
/// meaningless without the first having run.
///
/// **Cutting.** The table is no longer a fixed quad. It is made to whatever
/// [`TableLayout::extent`] reports plus [`SLAB_MARGIN`] of table, which
/// changes with the seat count, the focused pod and the window, and it is cut
/// as a **racetrack** with a real thickness rather than as a rectangle one
/// pixel deep — see [`tabletop::table_corner`] for what the corners are for.
/// The cut is guarded on the size, so a table that has not changed shape
/// costs nothing.
///
/// **Lighting.** The step is on the board model, so this needs no engine and
/// no view of its own; the arithmetic — which colour a step is worth — lives
/// in [`tabletop::phase_light`], where it can be argued with in a test rather
/// than looked at in a screenshot. It writes only when the colour actually
/// moves: a lamp that reached its target and kept writing would touch a
/// material every frame for the rest of the game, which is exactly the
/// garbage [`sync_zones`] exists to avoid.
pub fn sync_table(
    mut commands: Commands,
    time: Res<Time>,
    duel: Res<Duel>,
    prefs: Res<crate::prefs::Prefs>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FeltMaterial>>,
    mut slabs: Query<(&mut Slab, &mut Mesh3d, &MeshMaterial3d<FeltMaterial>)>,
) {
    let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.layout.as_ref()) else {
        return;
    };
    let Some((min, max)) = layout.extent() else {
        return;
    };

    // Measured about the origin rather than about the extent's own centre,
    // because the origin is where the medallion is inlaid and where the pool
    // is centred. For every seat count that can actually be played the ring
    // is symmetric and the two agree.
    let reach = min.abs().max(max.abs());
    let span = (reach + Vec2::splat(SLAB_MARGIN)) * 2.0;

    let motion = if prefs.all().reduce_motion {
        crate::cardmat::STILL
    } else {
        crate::cardmat::MOVING
    };

    // Where the light enters: the active seat's own shore, running inward
    // round the rail. At two seats that is the reference photograph —
    // cool at one bank, molten at the other — and at six it is a pool lit
    // from whoever's turn it is, which is the same statement without needing
    // the table to have ends.
    let source = board
        .pods
        .iter()
        .find(|pod| pod.is_active)
        .and_then(|pod| layout.slot(pod.player))
        .map_or(Vec4::new(0.0, -1.0, 0.0, 1.0), |slot| {
            let inward = (-slot.center).try_normalize().unwrap_or(Vec2::Y);
            Vec4::new(slot.center.x, slot.center.y, inward.x, inward.y)
        });

    let want = crate::feltmat::wash_of(board.step);
    let ease = if prefs.all().reduce_motion {
        1.0
    } else {
        1.0 - (-WASH_RATE * time.delta_secs()).exp()
    };

    let Ok((mut slab, mut mesh, handle)) = slabs.single_mut() else {
        // No slab yet. Cut one, and let the next frame light it.
        commands.spawn((
            DuelStage,
            Slab {
                cut: span,
                shown: Vec4::ZERO,
                source,
                motion,
            },
            // The floor of the scene answers no clicks: a pointer on bare
            // cloth means the table, not the thing under it.
            Pickable::IGNORE,
            Mesh3d(meshes.add(slab_mesh(span))),
            MeshMaterial3d(materials.add(FeltMaterial {
                params: crate::feltmat::FeltParams {
                    wash: Vec4::ZERO,
                    source,
                    // No light until the sky has read a clock. The cloth's
                    // own colour is what `a = 0` means, so a table that is
                    // cut before the first `sync_sky` is simply the table.
                    ambient: Vec4::new(1.0, 1.0, 1.0, 0.0),
                    span,
                    corner: tabletop::table_corner(span),
                    rail: tabletop::RAIL_WIDTH,
                    motion,
                    gain: crate::feltmat::WASH_GAIN,
                    thickness: TABLE_THICKNESS,
                },
            })),
            Transform::from_xyz(0.0, TABLE_Y, 0.0)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
        return;
    };

    let recut = (slab.cut - span).abs().max_element() > 1e-3;
    let next = slab.shown + (want - slab.shown) * ease;
    // Below a step this small nothing on screen changes, so stop. A slab that
    // reached its colour and went on writing would touch a material — and so
    // a uniform upload — every frame for the rest of the game.
    let still = (next - slab.shown).abs().max_element() <= 1e-4
        && (slab.source - source).abs().max_element() <= 1e-4
        && (slab.motion - motion).abs() <= f32::EPSILON;
    if still && !recut {
        return;
    }
    slab.shown = next;
    slab.source = source;
    slab.motion = motion;

    if recut {
        slab.cut = span;
        *mesh = Mesh3d(meshes.add(slab_mesh(span)));
    }
    if let Some(mut material) = materials.get_mut(&handle.0) {
        if recut {
            material.params.span = span;
            material.params.corner = tabletop::table_corner(span);
        }
        material.params.wash = next;
        material.params.source = source;
        material.params.motion = motion;
    }
}

/// The slab of a table this size: a racetrack with a real thickness, whose
/// **top face lies exactly at the plane every other thing on the stage is
/// placed against**.
///
/// The body hangs below that plane rather than standing on it, which is the
/// one thing that has to be got right: raise the surface by the thickness and
/// every mat, glow, medallion and card is buried inside the table.
fn slab_mesh(span: Vec2) -> Mesh {
    rounded_slab_mesh(
        span.x,
        span.y,
        tabletop::table_corner(span),
        0.0,
        -TABLE_THICKNESS,
        SLAB_SEGMENTS,
    )
}

/// Keeps one mat and one glow per seat in step with the table.
///
/// Zones are spawned when a seat first appears and only ever re-tinted after
/// that: the layout does not move once a game has begun, and a mat rebuilt
/// every frame would be four meshes and four materials of pure garbage per
/// frame for a table nobody is looking at that hard.
#[allow(clippy::too_many_lines)] // one seat's ground, glow and piles in one pass
pub fn sync_zones(
    mut commands: Commands,
    duel: Res<Duel>,
    prefs: Res<crate::prefs::Prefs>,
    mut index: ResMut<SceneIndex>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<crate::matmat::MatMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.layout.as_ref()) else {
        return;
    };
    let Some(glow_image) = index.glow_image.clone() else {
        return;
    };
    let moving = !prefs.all().reduce_motion;

    let mut seen: HashSet<PlayerId> = HashSet::new();
    for pod in &board.pods {
        let Some(slot) = layout.slot(pod.player) else {
            continue;
        };
        seen.insert(pod.player);
        let mood = Mood::of(pod);
        let accent = seat_accent(slot);
        let size = slot.half_extent * 2.0 + Vec2::splat(ZONE_MARGIN * 2.0);
        let params = mat_params(accent, size, mood, moving, slot.ledge_is_outer());
        // Two different colours, not one with a dimmer on it. The mat carries
        // the seat's colour in its own rim and nowhere else, so a tint over
        // the whole thing would put the accent on the felt as well; what the
        // mat takes from the mood is brightness alone, and it takes it as an
        // alpha. The glow beneath is the opposite case — a white falloff
        // whose entire job is to spill the seat's colour onto the table.
        let glow_tint = accent.to_linear() * zone_brightness(mood) * GLOW_STRENGTH;

        // The one fact about a pile the *places* depend on. A seat whose
        // library has run out loses on its next draw, and the table stops
        // showing a stack there — which is worth a rebuild; how many cards
        // are in a library that still has some is not, and is on the tab.
        let library_shown = pod.library_count.min(SHOWN_LIBRARY);
        // A seat that has moved or changed size is rebuilt rather than
        // updated: its mat is a mesh cut to a width, its glow is a quad and
        // its piles stand at fixed points, and not one of those is something
        // a uniform can carry. See [`Zone::slot`] for what this cost.
        let moved = index
            .zones
            .get(&pod.player)
            .is_some_and(|zone| zone.slot != *slot);
        if moved && let Some(zone) = index.zones.remove(&pod.player) {
            for entity in [zone.mat, zone.glow].into_iter().chain(zone.piles) {
                commands.entity(entity).despawn();
            }
        }
        if let Some(zone) = index.zones.get(&pod.player) {
            let stale_piles = zone.library_shown != library_shown;
            if zone.mood == mood && zone.accent == accent && !stale_piles {
                continue;
            }
            let old_piles = zone.piles.clone();
            // Only the numbers change; the mesh and the transform still hold.
            // The accent moved through here too — it is a uniform now rather
            // than a texture that would have to be generated again.
            if let Some(mut material) = mats.get_mut(&zone.mat_material) {
                material.params = params;
            }
            if let Some(mut material) = materials.get_mut(&zone.glow_material) {
                material.base_color = Color::LinearRgba(glow_tint);
            }
            let fresh = stale_piles.then(|| {
                for entity in old_piles {
                    commands.entity(entity).despawn();
                }
                spawn_piles(&mut commands, &index, slot, &pod.piles, pod.library_count)
            });
            index.zones.entry(pod.player).and_modify(|zone| {
                zone.mood = mood;
                zone.accent = accent;
                zone.library_shown = library_shown;
                if let Some(fresh) = fresh {
                    zone.piles = fresh;
                }
            });
            continue;
        }
        let mat_material = mats.add(crate::matmat::MatMaterial { params });
        let mat = commands
            .spawn((
                DuelStage,
                Mesh3d(meshes.add(Rectangle::new(size.x, size.y))),
                MeshMaterial3d(mat_material.clone()),
                Pickable::IGNORE,
                lying_flat(slot, ZONE_LIFT),
            ))
            .id();
        let (glow, glow_material) = spawn_table_quad(
            &mut commands,
            &mut meshes,
            &mut materials,
            slot,
            TableQuad {
                size: size + Vec2::splat(GLOW_SPREAD * 2.0),
                lift: GLOW_LIFT,
                tint: glow_tint,
                texture: glow_image.clone(),
            },
        );
        let piles = spawn_piles(&mut commands, &index, slot, &pod.piles, pod.library_count);
        index.zones.insert(
            pod.player,
            Zone {
                slot: *slot,
                mat,
                glow,
                mat_material,
                glow_material,
                piles,
                library_shown,
                mood,
                accent,
            },
        );
    }

    // A seat that left the table takes its zone with it.
    index.zones.retain(|player, zone| {
        if seen.contains(player) {
            return true;
        }
        for entity in [zone.mat, zone.glow].into_iter().chain(zone.piles.clone()) {
            commands.entity(entity).despawn();
        }
        false
    });
}

/// Tears the stage down.
pub fn despawn_stage(
    mut commands: Commands,
    stage: Query<Entity, With<DuelStage>>,
    cards: Query<Entity, With<CardVisual>>,
    mut index: ResMut<SceneIndex>,
    mut watch: ResMut<ZoneWatch>,
) {
    for entity in stage.iter().chain(cards.iter()) {
        commands.entity(entity).despawn();
    }
    index.cards.clear();
    index.faces.clear();
    watch.clear();
    // The zones were spawned with `DuelStage`, so they have just gone with
    // it; what is left is the bookkeeping that would otherwise point at
    // entities that no longer exist.
    index.zones.clear();
}

/// Puts the printed back on a material that was built without one.
///
/// Both halves or neither: `has_art` is what the shader reads to decide
/// between sampling the picture and filling with the tint, and it follows the
/// *handle* rather than the look — so a material given a texture and not the
/// flag draws exactly what it drew before, which is a fault that looks like
/// the image never arriving.
fn dress_in_the_back(material: &mut CardMaterial, back: Handle<Image>) {
    material.art = Some(back);
    material.params.has_art = 1.0;
}

/// Places table-space coordinates into the world.
///
/// Table space has `+y` running away from the local seat; the world has the
/// camera on `+z`, so the two are mirrored on that axis.
pub(crate) fn to_world(table: Vec2, height: f32) -> Vec3 {
    Vec3::new(table.x, height, -table.y)
}

/// The transform of one card.
fn card_transform(slot: &SeatSlot, position: Vec2, tapped: bool, lift: f32) -> Transform {
    // Lay the quad flat, then turn it so it faces its owner, then tap it.
    let mut rotation =
        Quat::from_rotation_y(-slot.facing) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    if tapped {
        rotation = Quat::from_rotation_y(-slot.facing)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
            * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    }
    Transform {
        translation: to_world(position, TABLE_Y + CARD_LIFT + lift),
        rotation,
        scale: Vec3::ONE,
    }
}

/// Where every group in the current board model belongs.
struct Placement {
    object: ObjectId,
    slot: SeatSlot,
    position: Vec2,
    /// Where this card stands in its row, as a height — see [`LANE_RISE`].
    lift: f32,
    tapped: bool,
    count: usize,
    art: Option<ImageKey>,
    offer: crate::cardmat::Offer,
    corner: baylee_client_core::cardplate::Corner,
    selected: bool,
}

/// Computes placements for the whole table.
///
/// Pure geometry over the board model, so the ordering is the model's ordering
/// and therefore stable frame to frame — which is what makes the diff below
/// cheap and stops cards from swapping places when nothing happened.
fn placements(duel: &Duel) -> Vec<Placement> {
    let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.layout.as_ref()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for pod in &board.pods {
        let Some(slot) = layout.slot(pod.player) else {
            continue;
        };
        for lane in &pod.lanes {
            let center = slot.lane_center(lane.kind);
            let packing = pack_lane(lane.groups.len(), slot.lane_width());
            // The row's rise, shared out over however many cards are on it.
            let steps = lane.groups.len().saturating_sub(1).max(1) as f32;
            for (i, (group, offset)) in lane.groups.iter().zip(packing.offsets.iter()).enumerate() {
                let along = Vec2::new(slot.facing.cos(), -slot.facing.sin());
                out.push(Placement {
                    object: group.representative,
                    slot: *slot,
                    position: center + along * *offset,
                    // Later in the row is higher, so a fanned lane shingles
                    // the way a hand of cards does — each card over the one
                    // before it, and never in bands of both.
                    lift: LANE_RISE * i as f32 / steps,
                    tapped: group.status.is_tapped(),
                    count: group.count(),
                    art: group.art,
                    // Resolved here rather than in the sync loop, because
                    // here is where the group's *members* are: a plan taps
                    // one particular Forest, and the card drawn for it may
                    // be standing for four.
                    offer: crate::cardmat::Offer::on(
                        duel.armed.as_ref(),
                        &group.members,
                        group.activatable,
                    ),
                    // Power, toughness, marked damage and the counters — the
                    // rules facts printed on every real card and drawn nowhere
                    // in this client on a card showing art. Resolved here for
                    // the same reason the offer is: the group is here.
                    corner: baylee_client_core::cardplate::Corner::of(group),
                    // Chosen for the pending choice — resolved here for the
                    // same reason again, and through `is_selected` rather
                    // than `selected()`. In the two combat modes the answer
                    // being built is a list of *pairs*, and `selected()` is
                    // empty however many attackers have been declared;
                    // `is_selected` is the method that reads the pairs. The
                    // sync loop used to ask the empty list, which is why a
                    // declared attacker lay flat on the table and combat
                    // drew nothing at all.
                    selected: duel
                        .interaction
                        .as_ref()
                        .is_some_and(|i| group.members.iter().any(|member| i.is_selected(*member))),
                });
            }
        }

        // The piles standing beside the ground. A pile whose top card is an
        // object is a placement like any other, and deliberately so:
        // `index.cards` is keyed by `ObjectId`, and an id survives a zone
        // change — so a creature that dies *glides* off its lane and onto the
        // graveyard through the update-in-place branch below, instead of
        // blinking out of one place and into another. It also inherits the
        // material cache, the hover lift, the arming glow and the selection
        // lift, every one of which would have to be written a second time in
        // a renderer of its own. "Nothing on the table is positioned
        // directly" is the rule; a pile is not an exception to it.
        //
        // A library has no object, and neither has an empty pile. Those two
        // are drawn by `sync_zones`, which needs no card behind them.
        for pile in &pod.piles {
            let Some(top) = pile.top else {
                continue;
            };
            out.push(Placement {
                object: top,
                slot: *slot,
                position: slot.pile_center(pile.kind),
                // A pile stands beside the ground and overlaps nothing, so
                // there is nothing here for the row's rise to separate it
                // from.
                lift: 0.0,
                // A card in a graveyard is not a permanent and has no tap
                // state to draw; the same goes for its power and toughness,
                // which is why the corner is the empty one rather than
                // `Corner::of`. Drawing a 4/4 on a card that is no longer a
                // creature would be inventing a fact.
                tapped: false,
                count: usize::try_from(pile.count).unwrap_or(usize::MAX),
                art: pile.art,
                offer: crate::cardmat::Offer::on(duel.armed.as_ref(), &[top], false),
                corner: baylee_client_core::cardplate::Corner::default(),
                selected: duel
                    .interaction
                    .as_ref()
                    .is_some_and(|i| i.is_selected(top)),
            });
        }
    }
    out
}

/// Brings the scene in line with the board model.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // the diff loop is one coherent pass
pub fn sync_scene(
    mut commands: Commands,
    duel: Res<Duel>,
    mut index: ResMut<SceneIndex>,
    mut watch: ResMut<ZoneWatch>,
    mut textures: Option<ResMut<CardTextures>>,
    mut card_materials: ResMut<Assets<CardMaterial>>,
    assets: Res<AssetServer>,
    texts: Res<crate::cardtext::CardTexts>,
    mode: Res<crate::face::FaceMode>,
    settings: Res<crate::settings::ClientSettings>,
    prefs: Res<crate::prefs::Prefs>,
    sheen: Res<crate::sheen::Sheen>,
    fonts: Option<Res<crate::hud::UiFonts>>,
    mut cards: Query<(
        &mut Motion,
        &mut CardVisual,
        &mut MeshMaterial3d<CardMaterial>,
    )>,
) {
    let (Some(statics), Some(textures)) = (duel.statics.as_ref(), textures.as_mut()) else {
        return;
    };
    let Some(quad) = index.quad.clone() else {
        return;
    };
    let blank = index.blank.clone();
    let shadow = index.shadow_quad.clone().zip(index.shadow_material.clone());
    // Read after the guards and not before them: a frame that bails because
    // the quad or the print table has not arrived yet must not swallow the
    // one batch of moves this view will ever produce. `sheen` may have read
    // the view already this frame — it needs the same batch to know which
    // door an arriving card came in through — so the reading and the taking
    // are two calls, and this is the only place that takes.
    if let Some(view) = duel.view.as_ref() {
        watch.observe(view);
    }
    let moves = watch.take();

    // A look carrying a sweep is a key that is asked for on every frame the
    // band is crossing and never again, so it is cached like any other look —
    // otherwise a card arriving would mint a material and a handle sixty
    // times a second — and swept out here once the band is over. Without
    // this the map would gain an entry per permanent per arrival and evict
    // none of them.
    index
        .materials
        .retain(|look, _| look.sweep.is_none_or(|s| sheen.live(s)));

    // The back is a picture and a picture arrives late, so the one material
    // every hidden card wears is dressed in it here rather than built with it
    // in `spawn_stage`. In place, and that is the point: a library stack, the
    // depth behind a counted group and a card this seat may not see all hold
    // *this* handle, some of them spawned once and never visited again, so
    // handing them a new material would mean finding them all. Changing the
    // one they share turns every card over at once.
    if !index.back_dressed
        && textures.card_back_is_printed()
        && let Some(handle) = blank.as_ref()
        && let Some(mut material) = card_materials.get_mut(handle)
    {
        dress_in_the_back(&mut material, textures.card_back());
        index.back_dressed = true;
    }

    // Reduce-motion reaches the cards through their material, so a change to
    // it has to reach every material already made. Compared rather than
    // watched: `Prefs` is written every frame by its own debounce, so change
    // detection on the resource would fire this continuously.
    //
    // Rewritten in place rather than thrown away, which is the difference
    // between the switch taking effect and the switch taking effect *later*:
    // a cleared cache is only refilled by whatever draws the card next, and
    // a table nobody is playing at draws nothing. Rewriting keeps every
    // handle valid, so the cards already on the felt change under the
    // player's eyes on the frame the setting moves.
    let still = prefs.all().reduce_motion;
    let motion = motion_of(still);
    if index.still != still {
        index.still = still;
        let handles: Vec<_> = index
            .materials
            .values()
            .chain(index.face_materials.values())
            .cloned()
            .collect();
        for handle in handles {
            if let Some(mut material) = card_materials.get_mut(&handle) {
                material.params.motion = motion;
            }
        }
    }

    let wanted = placements(&duel);
    let mut live: HashSet<ObjectId> = HashSet::new();

    // The keyboard/mouse cursor. What is *chosen* rides on the placement,
    // because that is where a group's members are.
    let hovered = duel.hovered;

    // The snapshot the faces below were built from: rules text is projected,
    // so a face is only stale when the game state that produced it moved on.
    let seq = duel.view.as_ref().map_or(0, |v| v.seq);

    for placement in &wanted {
        live.insert(placement.object);

        // A card either wears its art or its own text, never both — text on
        // top of artwork is unreadable at any zoom.
        let show_face = face::wants_face(&mode, &settings, textures, placement.art);
        let object = duel
            .view
            .as_ref()
            .and_then(|view| view.object(placement.object));

        // What the card is physically, and what the rules have made it. Both
        // ride on the material, so a foil that gains indestructible becomes a
        // different material and needs no second pass.
        //
        // The finish is a property of the printing, so it comes from the
        // print table — which is per seat, and a printing this seat has not
        // earned reads as plain rather than as a leak.
        let finish = crate::cardmat::finish_of(statics, placement.art);
        // Keywords are what the card is, sickness is what it cannot do this
        // turn, and the offer is what the player could do with it — or has
        // just said they will. All of it rides on the material, so a Forest
        // that becomes tappable becomes a different material and needs no
        // second pass — and stops being one the moment priority moves on.
        let glow = crate::cardmat::glow_of(object, placement.offer);

        let material = if show_face {
            // One material per colour identity, so a mono-green board is one
            // material however many creatures are on it.
            let colors = object.map_or(ColorSet::EMPTY, |o| o.colors);
            let tint = face::table_color(colors);
            let look = CardLook::flat(tint, finish, glow).with_corner(placement.corner);
            if let Some(handle) = index.face_materials.get(&look) {
                handle.clone()
            } else {
                let handle = card_materials.add(material(look, None, tint, motion));
                index.face_materials.insert(look, handle.clone());
                handle
            }
        } else {
            // One material per look, created on first use.
            match placement.art {
                Some(key) => {
                    let look = CardLook::art(key, finish, glow)
                        .with_corner(placement.corner)
                        .with_sweep(sheen.of(placement.object, crate::sheen::Surface::Table));
                    if let Some(handle) = index.materials.get(&look) {
                        handle.clone()
                    } else {
                        let image = textures.get(key, statics, &assets);
                        let handle =
                            card_materials.add(material(look, Some(image), BACK_COLOR, motion));
                        index.materials.insert(look, handle.clone());
                        handle
                    }
                }
                None => blank.clone().unwrap_or_default(),
            }
        };

        // A pile stands on the cards under it. The top card is drawn at the
        // deck's own height and the rest of the deck hangs below it as
        // children, so what a player sees is one block of cardboard with a
        // face on top rather than a card with a fan of cards behind it.
        let deck = stack_rise(placement.count.saturating_sub(1));
        let mut transform = card_transform(
            &placement.slot,
            placement.position,
            placement.tapped,
            placement.lift + deck,
        );
        // Hover (cursor) lifts the card a touch; a chosen card stays raised
        // until the choice is answered, and so does an armed one — a deed
        // waiting on a second tap is a commitment the player has already
        // made, which is the same claim being selected makes and belongs at
        // the same height. Selected wins over both; the pointer moving away
        // must not put an armed card back down.
        if placement.selected || placement.offer.armed {
            transform.translation.y += SELECTED_LIFT;
            transform.scale *= SELECTED_SCALE;
        } else if hovered == Some(placement.object) {
            transform.translation.y += HOVER_LIFT;
            transform.scale *= HOVER_SCALE;
        }

        let entity = if let Some(&entity) = index.cards.get(&placement.object) {
            // Existing card: update in place. Touching only what changed is
            // what keeps a large board cheap.
            if let Ok((mut motion, mut visual, mut current_material)) = cards.get_mut(entity) {
                if motion.target != transform {
                    motion.target = transform;
                }
                if visual.count != placement.count {
                    visual.count = placement.count;
                }
                if current_material.0 != material {
                    current_material.0 = material;
                }
            }
            entity
        } else {
            let entity = commands
                .spawn((
                    DuelStage,
                    CardVisual {
                        object: placement.object,
                        count: placement.count,
                    },
                    Mesh3d(quad.clone()),
                    MeshMaterial3d(material),
                    // Appears wherever it is coming from and settles onto its
                    // mark; `glide` does the rest, and a player who has turned
                    // motion off gets the target on the very first frame.
                    entrance_from(
                        pile_stand(
                            &duel,
                            moves
                                .iter()
                                .find(|m| m.object == placement.object)
                                .and_then(|m| m.from),
                        ),
                        &transform,
                    ),
                    Motion { target: transform },
                ))
                .id();
            index.cards.insert(placement.object, entity);

            // The contact shadow rides along as a child, which is what keeps
            // it under a tapped card without anything having to rotate it,
            // and what makes it grow out from under a card as that card is
            // lifted. It sits between the felt and the card, and is not
            // pickable — a click near a card's edge means the table.
            if let Some((mesh, material)) = shadow.clone() {
                commands.entity(entity).with_child((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    // Under the whole deck rather than under the top card, and
                    // wider the taller the deck is: a thick pile sits in more
                    // shadow than a single card does, which is most of what
                    // makes it read as thick at all.
                    Transform::from_xyz(0.0, 0.0, -(CARD_LIFT * 0.5 + deck)).with_scale(Vec3::new(
                        1.0 + deck * DECK_SHADOW_SPREAD,
                        1.0 + deck * DECK_SHADOW_SPREAD,
                        1.0,
                    )),
                    Pickable::IGNORE,
                ));
            }

            // The rest of the deck, as children hanging below the face. They
            // are children and not loose entities for two reasons: they
            // follow the card through every glide, tap and lift with nothing
            // to keep in step, and they are despawned with it — as loose
            // entities they were never despawned at all, so every card that
            // ever lay on a graveyard left its backing slabs standing there
            // for the rest of the game.
            let layers = stack_layers(placement.count.saturating_sub(1));
            for i in 1..=layers {
                let down = deck * i as f32 / layers as f32;
                commands.entity(entity).with_child((
                    Mesh3d(quad.clone()),
                    MeshMaterial3d(blank.clone().unwrap_or_default()),
                    Transform::from_xyz(0.0, 0.0, -down),
                    // The deck under a card is depth, not cards: the top
                    // card is what a click has to reach.
                    Pickable::IGNORE,
                ));
            }
            entity
        };

        // The text children follow the same decision as the material, and are
        // rebuilt when the snapshot they were made from is no longer current:
        // an anthem, a counter or a clone all change what the face should say.
        let current = index.faces.get(&placement.object).map(|(seq, _)| *seq);
        if show_face && current == Some(seq) {
            continue;
        }
        if let Some((_, previous)) = index.faces.remove(&placement.object) {
            for text in previous {
                commands.entity(text).despawn();
            }
        }
        if !show_face {
            continue;
        }
        let Some((object, fonts)) = object.zip(fonts.as_deref()) else {
            continue;
        };
        let built = face::of_object(object, &texts);
        let spawned = face::spawn_world(&mut commands, entity, &built, fonts);
        index.faces.insert(placement.object, (seq, spawned));
    }

    // Anything no longer on the board leaves the scene.

    let stale: Vec<ObjectId> = index
        .cards
        .keys()
        .copied()
        .filter(|id| !live.contains(id))
        .collect();
    for id in stale {
        if let Some(entity) = index.cards.remove(&id) {
            // Despawning a card takes its text children with it, so the map
            // only has to forget them — and it keeps them for as long as the
            // card is still leaving, which is what makes a named card sink
            // into the graveyard rather than a blank one.
            index.faces.remove(&id);
            // A stale id with no move behind it did not leave anywhere: it is
            // a graveyard's old top card, covered by the one that landed on
            // it this frame, or a group that re-keyed when its lowest-id
            // member went. Nothing about the table changed where it stands,
            // so it goes at once, as it always did — an exit played for one
            // of those would be a card visibly sliding out from under a pile
            // it never left.
            let Some(step) = moves.iter().find(|m| m.object == id).copied() else {
                commands.entity(entity).despawn();
                continue;
            };
            let to = step.to;
            if let Ok((mut motion, _, mut worn)) = cards.get_mut(entity) {
                motion.target = exit(to, pile_stand(&duel, to), &motion.target);
                if let Some(dressed) = dress_the_exit(
                    &mut card_materials,
                    &worn.0,
                    step,
                    sheen.now(),
                    motion_of(still),
                ) {
                    worn.0 = dressed;
                }
            }
            // It stops being a card here. `CardVisual` is what every reader
            // finds a permanent by, so taking it away is what stops a hover,
            // a preview or a combat line from following something that is on
            // its way out of the game.
            commands
                .entity(entity)
                .remove::<CardVisual>()
                .insert((Pickable::IGNORE, Departing { left: EXIT_LIFE }));
        }
    }

    // Tell the cache what is on screen so it can evict the rest.
    let visible: Vec<ImageKey> = wanted.iter().filter_map(|p| p.art).collect();
    textures.retain_visible(&visible);
}

/// How a group should be labelled in the overlay, if at all.
#[must_use]
pub fn stack_badge(group: &CardGroup) -> Option<String> {
    group.is_stack().then(|| format!("×{}", group.count()))
}

#[cfg(test)]
mod camera_tests {
    use super::*;
    use baylee_core::ids::PlayerId;

    /// A laptop's window, in logical pixels.
    const WINDOW: Vec2 = Vec2::new(1728.0, 1052.0);

    fn seats(n: u8) -> Vec<PlayerId> {
        (0..n).map(PlayerId::new).collect()
    }

    /// Where a point on the felt lands, in normalised device coordinates.
    ///
    /// Written out forwards on purpose: [`CameraRig::home`] inverts the same
    /// projection, and a test that reused the inverse would agree with it
    /// however wrong both were.
    fn project(rig: CameraRig, canvas: Canvas, table: Vec2) -> Vec2 {
        let lean = CAMERA_LEAN;
        let eye = rig.distance * (1.0 + lean * lean).sqrt();
        let cos = 1.0 / (1.0 + lean * lean).sqrt();
        let t = (FOV * 0.5).tan();
        let aspect = canvas.window.x / canvas.window.y;
        // The rig stores world x/z; `+y` away from the local seat is `-z`.
        let s = table.y - -rig.target.y;
        let depth = lean.mul_add(cos * s, eye);
        Vec2::new(
            (table.x - rig.target.x) / (depth * t * aspect),
            cos * s / (depth * t),
        )
    }

    /// Every seat's bar fits the shelf it is written on, at every table.
    ///
    /// Two claims, and the second is the one that is easy to lose. The
    /// density chosen for a shelf must not overhang it by more than that
    /// density is allowed — which is what makes the choice a choice rather
    /// than a label. And the shelf has to project **deeper** than the bar is
    /// tall, or the ink is standing on the creature lane behind it rather
    /// than on the ledge; a few pixels of shelf above and below is what makes
    /// the bar sit on the felt instead of floating over it.
    ///
    /// The local seat is in the loop and is not the easy case: its ledge is
    /// the *far* edge of its own mat, so it is the more foreshortened end of
    /// the nearest board.
    ///
    /// Measured through [`Shelf`](crate::hud::Shelf), which is what the
    /// renderer places from, and therefore along each shelf's **own** axis. A
    /// side seat's ledge runs up and down the screen; its bounding box is
    /// sixty pixels wide and the ledge is four hundred long, so a test
    /// measuring the box would report a pip strip on a shelf with room for
    /// every label.
    #[test]
    fn the_bar_fits_its_ledge_at_every_seat_of_an_eight_ring() {
        use crate::hud::Shelf;
        let canvas = Canvas::hud(WINDOW);
        for n in 2..=8 {
            let layout = TableLayout::new(&seats(n), canvas.aspect(), None);
            let rig = CameraRig::home(&layout, canvas);
            let lens = Lens::new(rig, canvas.window);
            for slot in &layout.slots {
                let corners = lens
                    .corners(slot.ledge_corners())
                    .expect("every ledge is in front of the camera");
                let shelf = Shelf::of(corners, false);
                let density = shelf.density;
                let over = density.width(false) - shelf.along;
                assert!(
                    over <= density.width(false) * density.overhang() + 1e-3,
                    "at {n} seats, seat {} takes the {density:?} bar ({} wide) \
                     on a {} shelf — {over} of overhang",
                    slot.ring_index,
                    density.width(false),
                    shelf.along
                );
                assert!(
                    shelf.depth >= density.ink_height(),
                    "at {n} seats, seat {}'s shelf projects {} deep and the \
                     {density:?} bar draws {} of ink — it would stand on the \
                     creature lane",
                    slot.ring_index,
                    shelf.depth,
                    density.ink_height()
                );
                // And no bar is drawn upside-down, whatever its mat is doing:
                // the ink reads in the viewer's order and the ground belongs
                // to the seat.
                assert!(
                    shelf.tilt.abs() <= std::f32::consts::FRAC_PI_2 + 1e-3,
                    "at {n} seats, seat {}'s bar is turned {} radians",
                    slot.ring_index,
                    shelf.tilt
                );
            }
        }
    }

    /// A duel gets the bar its window can hold.
    ///
    /// The claim `baylee-client-core` cannot make on its own: which density
    /// a *real* table's shelf projects to. It used to be made there anyway,
    /// against a shelf modelled as `window.x * 0.635`, and a constant cannot
    /// be wrong about the projection it stands in for — so it passed while
    /// promising the compact bar at a width that actually gets the full one.
    /// Measured here through the same `Lens` the renderer places from.
    #[test]
    fn a_duel_gets_the_bar_its_window_can_hold() {
        use crate::hud::Shelf;
        for (width, wanted) in DUEL_BARS {
            let window = Vec2::new(width, width * WINDOW.y / WINDOW.x);
            let canvas = Canvas::hud(window);
            let layout = TableLayout::new(&seats(2), canvas.aspect(), None);
            let rig = CameraRig::home(&layout, canvas);
            let lens = Lens::new(rig, canvas.window);
            let local = &layout.slots[0];
            let corners = lens
                .corners(local.ledge_corners())
                .expect("the local ledge is in front of the camera");
            let shelf = Shelf::of(corners, false);
            assert_eq!(
                shelf.density, wanted,
                "a {width}-wide duel projects a {:.0} px shelf, which is the \
                 {:?} bar and not the {wanted:?} one",
                shelf.along, shelf.density
            );
        }
    }

    /// A duel is written on two rows, at **both** seats.
    ///
    /// The reason [`crate::tabletop::MAT_LEDGE`] is as deep as it is, and
    /// therefore the test that says what the depth is for. `Density::Split`
    /// is chosen per seat, like every other form, so a ledge deep enough for
    /// only one of a duel's two shelves would draw the near seat a phase line
    /// and the far seat a single crowded row — one table, two designs, and no
    /// test would have noticed.
    ///
    /// Two seats and not more, and what stops it is the shelf's **length**
    /// rather than its depth. Measured at [`WINDOW`]: a duel's shelves
    /// project 1127×61 and 1069×55, and three seats project 372×46, 337×44,
    /// 337×44 — deep enough for the 34 px of ink two rows draw, and nowhere
    /// near the 561 px the two-row bar is wide. It used to be the depth that
    /// ran out first; that was the shelf being measured a printed border
    /// short of the one the mat draws, and the number that moved when they
    /// were reconciled was the depth.
    /// [`the_bar_fits_its_ledge_at_every_seat_of_an_eight_ring`] is what
    /// holds the ladder that takes over there.
    #[test]
    fn a_duel_is_written_on_two_rows() {
        use crate::hud::Shelf;
        use baylee_client_core::seatbar::Density;
        let canvas = Canvas::hud(WINDOW);
        let layout = TableLayout::new(&seats(2), canvas.aspect(), None);
        let rig = CameraRig::home(&layout, canvas);
        let lens = Lens::new(rig, canvas.window);
        let shelves: Vec<Shelf> = layout
            .slots
            .iter()
            .map(|slot| {
                let corners = lens
                    .corners(slot.ledge_corners())
                    .expect("both ledges of a duel are in front of the camera");
                Shelf::of(corners, false)
            })
            .collect();
        // Reported together rather than one at a time, because the number
        // that decides the ledge is the *shallower* of the two and a test
        // that stopped at the near seat would never print it.
        let measured: Vec<String> = shelves
            .iter()
            .map(|s| format!("{:.1}×{:.1} ({:?})", s.along, s.depth, s.density))
            .collect();
        assert!(
            shelves.iter().all(|s| s.density == Density::Split),
            "a duel's shelves project {} and the two-row bar asks for \
             {:.1}×{:.1}",
            measured.join(", "),
            Density::Split.min_length(false),
            Density::Split.ink_height()
        );
    }

    /// What [`a_duel_gets_the_bar_its_window_can_hold`] promises, in one
    /// place, because these are the numbers a reader wants and not the loop
    /// around them.
    ///
    /// Measured, at a window kept the shape of [`WINDOW`]: the local ledge
    /// projects 548, 688, 768 and 1247 pixels long and 21.1, 30.7, 36.2 and
    /// 69.4 deep. The length runs 0.69, 0.67, 0.67 and 0.65 of the window's
    /// width — a band and not a constant, and it narrows as the window grows
    /// because `Canvas::hud` takes a *fixed* hand bar off the bottom, so a
    /// small window is a squarer canvas.
    ///
    /// Every hand-over in the list is a **depth** one, and that is the shape
    /// of the whole ladder now: the two-row bar wants 561 px of length, which
    /// an 800-wide window already has, so what decides the form is whether
    /// the shelf is deep enough to write two rows on — 34 px of ink against
    /// the 21.1, 30.7 and 36.2 above. A window of the shape here gains the
    /// phase line at about 1150, and it is its *height* that buys it.
    ///
    /// This list used to start at the compact bar at 1280 and reach the
    /// two-row one at 1728, and every number in it moved when the shelf
    /// stopped being measured a border short of the one that is drawn: the
    /// same 1728 window that projected a 40.3 px shelf projects 61.1. A duel
    /// is now written on two rows on any laptop, which is what the shelf
    /// could always hold and not a change of mind about what it should.
    ///
    /// The far seat's shelf stays about a tenth shallower than the near one
    /// (55.0 against 61.1 at 1728), so there is still a band — around 1150 to
    /// 1250 — where a duel draws its local bar on two rows and its
    /// opponent's on one. That is the same per-seat answer the ladder gives a
    /// four-seat table, and the list deliberately does not try to pin its
    /// edges: they move with every constant here.
    const DUEL_BARS: [(f32, baylee_client_core::seatbar::Density); 4] = [
        (800.0, baylee_client_core::seatbar::Density::Pip),
        (1024.0, baylee_client_core::seatbar::Density::Compact),
        (1152.0, baylee_client_core::seatbar::Density::Split),
        (1920.0, baylee_client_core::seatbar::Density::Split),
    ];

    /// A free-for-all of three is on the circle it is supposed to be on.
    ///
    /// `layout::ROUND_COST` has always said a table with no allies is
    /// *offered* a circle and takes it when the camera can afford it, and at
    /// three seats a circle is the whole point: an ellipse shaped to a wide
    /// canvas puts the two opponents at 150° and 210°, side by side across
    /// the top, which is the silhouette a 2v1 draws.
    ///
    /// It was not afforded until the rail and the tab strip came off the top
    /// of the window. Measured at 1728×1052 on either canvas: with `top` at
    /// 110 the three-seat ring settled at 12.95 × 4.99 — an ellipse — and
    /// with `top` at nothing it settles at 8.28 × 8.28. That is the one seat
    /// count where the taller canvas made a board *smaller*, 34.9 → 32.0
    /// pixels a table unit, and it is the circle being bought rather than
    /// anything going wrong: 9.3% of reach, against the 30% `ROUND_COST`
    /// allows.
    ///
    /// Written as a test because it is a *silhouette*, and the arithmetic
    /// that produces it turns on a filter that a slightly different window
    /// can flip. Nothing else at the table notices when it does.
    #[test]
    fn three_seats_playing_for_themselves_sit_on_a_circle() {
        let canvas = Canvas::hud(WINDOW);
        let layout = TableLayout::new(&seats(3), canvas.aspect(), None);
        assert!(
            (layout.radius.x - layout.radius.y).abs() < 1e-3,
            "a three-seat free-for-all is on a {:?} ring, not a circle",
            layout.radius
        );
        // And the two opponents are a third of the way round from the local
        // seat and from each other, which is what a circle is for here.
        for slot in &layout.slots {
            let want = std::f32::consts::TAU * slot.ring_index as f32 / 3.0;
            let off = (slot.angle - want).abs();
            assert!(
                off < 0.02,
                "seat {} sits at {} radians, not {want}",
                slot.ring_index,
                slot.angle
            );
        }
    }

    /// [`Lens`] and the projection written out above agree.
    ///
    /// The one is a matrix built from the rig's own eye transform and the
    /// other is the closed form this file has always tested with, derived by
    /// hand from the lean and the lens. They are two independent derivations
    /// of one camera, which is the only reason either is evidence about the
    /// other — and it is what makes `Lens` safe to place the seat bars with,
    /// since a bar pinned to a shelf by a projection nobody has checked is a
    /// bar that lands wherever the arithmetic happens to put it.
    ///
    /// At yaw zero, because the closed form assumes it: it reads `table.y`
    /// straight down the view axis. The general case is `Lens`'s alone, which
    /// is the whole reason it exists — a player may orbit the table.
    #[test]
    fn the_lens_and_the_written_out_projection_agree() {
        let canvas = Canvas::hud(WINDOW);
        let layout = TableLayout::new(&seats(4), canvas.aspect(), None);
        let rig = CameraRig::home(&layout, canvas);
        let lens = Lens::new(rig, canvas.window);
        for slot in &layout.slots {
            for corner in slot.ledge_corners() {
                let theirs = project(rig, canvas, corner) * canvas.window * 0.5;
                // The closed form answers in pixels from the middle of the
                // window with `+y` up; the lens answers from the top-left
                // corner with `+y` down, which is where a `Node` lives.
                let theirs = Vec2::new(
                    theirs.x + canvas.window.x * 0.5,
                    canvas.window.y.mul_add(0.5, -theirs.y),
                );
                let ours = lens.project(corner).expect("the table is in front");
                assert!(
                    ours.distance(theirs) < 0.5,
                    "the lens puts {corner:?} at {ours:?} and the written-out \
                     projection at {theirs:?}"
                );
            }
        }
    }

    /// Every corner of every seat's mat, in table space.
    fn corners(layout: &TableLayout) -> Vec<Vec2> {
        box_corners(layout, |slot| slot.half_extent)
    }

    /// The corners of every seat's whole *place* — the ground and the piles
    /// standing beside it, which is the box the camera actually frames.
    fn places(layout: &TableLayout) -> Vec<Vec2> {
        box_corners(layout, SeatSlot::footprint)
    }

    fn box_corners(layout: &TableLayout, half: fn(&SeatSlot) -> Vec2) -> Vec<Vec2> {
        let mut out = Vec::new();
        for slot in &layout.slots {
            let (sin, cos) = slot.facing.sin_cos();
            for sx in [-1.0_f32, 1.0] {
                for sy in [-1.0_f32, 1.0] {
                    let local = half(slot) * Vec2::new(sx, sy);
                    out.push(
                        slot.center
                            + Vec2::new(
                                cos.mul_add(local.x, sin * local.y),
                                (-sin).mul_add(local.x, cos * local.y),
                            ),
                    );
                }
            }
        }
        out
    }

    /// How wide one seat's board is drawn, in pixels.
    fn drawn_width(rig: CameraRig, canvas: Canvas, slot: &SeatSlot) -> f32 {
        let (sin, cos) = slot.facing.sin_cos();
        let axis = Vec2::new(cos, -sin) * slot.half_extent.x;
        let ends = [slot.center + axis, slot.center - axis]
            .map(|end| project(rig, canvas, end) * canvas.window * 0.5);
        ends[0].distance(ends[1])
    }

    /// Every seat's board is laid out the same width, and is *drawn* nearly
    /// the same width too.
    ///
    /// The layout half of that has its own test in `layout.rs`; this is the
    /// camera half, and the two are different claims. A lean spends the
    /// furthest seat's size on the nearest one, and the nearest one is always
    /// the player's own — so the seat whose board the player compares every
    /// other against was the one drawn wrong. At the lean and the lens this
    /// shipped with it was 18.9% wider than its opponents' at a three-player
    /// free-for-all, which is a difference a player reads as a different
    /// format rather than as a camera.
    ///
    /// Bounded rather than equalised: the remaining few per cent is the
    /// foreshortening of a board turned away from the camera, and squeezing
    /// that out means a lean of zero, which is a table of decals.
    ///
    /// The bound was 1.08, then 1.12, and is 1.13 — a promise being given
    /// back in pieces, so each piece says what bought it. [`CAMERA_LEAN`]
    /// went 0.27 → 0.36 because the owner asked a third time for more angle
    /// after being told what it trades against, and the widest board on an
    /// eight-seat ring went 6.3% → 10.6% with it. Then the tab strip and the
    /// phase rail came off the top of the window, [`Canvas::hud`]'s `top`
    /// dropped from 110 to nothing, and the same ring went 10.3% → 12.1%:
    /// a canvas that is taller is also *squarer*, the ring is laid out
    /// rounder against it, and a rounder ring turns its side seats further
    /// away from the camera. The bound is that measurement plus a hair and
    /// not a round number chosen to be safe: it still fails the shot this
    /// test was written for, which drew one board 18.9% wider than its
    /// neighbours.
    ///
    /// The phone keeps 1.18 and did not move — it was already 16.5% at three
    /// seats for the reason below, and the taller canvas took it to 17.5%.
    #[test]
    fn every_seat_is_drawn_a_board_of_the_same_width() {
        // A phone is allowed a little more. Its ring is nearly a column —
        // 2.5 × 11.2 at three seats — so the near seat stands a far larger
        // fraction of the eye distance closer than it does on a ring that had
        // room to be round, and no lens shortens that.
        for (window, bound) in [
            (WINDOW, 1.13),
            (Vec2::new(1280.0, 800.0), 1.13),
            (Vec2::new(430.0, 932.0), 1.18),
        ] {
            let canvas = Canvas::hud(window);
            for n in 2..=8u8 {
                let layout = TableLayout::new(&seats(n), canvas.aspect(), None);
                let rig = CameraRig::home(&layout, canvas);
                let drawn: Vec<f32> = layout
                    .slots
                    .iter()
                    .map(|slot| drawn_width(rig, canvas, slot))
                    .collect();
                let widest = drawn.iter().copied().fold(0.0_f32, f32::max);
                let narrowest = drawn.iter().copied().fold(f32::INFINITY, f32::min);
                assert!(
                    widest <= narrowest * bound,
                    "{n} seats in {window}: boards laid out {:.2} wide are drawn \
                     {drawn:?} — {:.1}% apart, and the widest is seat {}",
                    layout.slots[0].lane_width(),
                    (widest / narrowest - 1.0) * 100.0,
                    drawn
                        .iter()
                        .position(|w| (w - widest).abs() < 1e-3)
                        .unwrap_or_default()
                );
            }
        }
    }

    /// A seat's mat is wider than the box that seat reports.
    ///
    /// The layout answers where *cards* go, so `half_extent` stops at the
    /// cards; the mat under them is drawn `ZONE_MARGIN` wider on every side,
    /// and that printed border is what makes it a playmat rather than a
    /// rectangle ruled tight around the lanes. Nothing in the layout knows
    /// it exists, so the camera has to, and [`AIR`] is where it is known.
    /// This is the assertion that keeps the two in step: shrink `AIR` back
    /// under `ZONE_MARGIN` and the near seat's border goes under the hand
    /// bar, which is the one edge a player is looking at.
    #[test]
    fn a_seats_printed_border_is_inside_the_band_too() {
        for window in [WINDOW, Vec2::new(1280.0, 800.0), Vec2::new(430.0, 932.0)] {
            let canvas = Canvas::hud(window);
            let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
            let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
            let right = 1.0 - 2.0 * canvas.right / canvas.window.x;
            for n in 2..=8u8 {
                let layout = TableLayout::new(&seats(n), canvas.aspect(), None);
                let rig = CameraRig::home(&layout, canvas);
                for corner in
                    box_corners(&layout, |slot| slot.half_extent + Vec2::splat(ZONE_MARGIN))
                {
                    let at = project(rig, canvas, corner);
                    assert!(
                        at.y >= bottom - 1e-3 && at.y <= top + 1e-3,
                        "{n} seats in {window}: a mat's border lands at y {}, outside \
                         {bottom}..{top}",
                        at.y
                    );
                    assert!(
                        at.x >= -1.0 - 1e-3 && at.x <= right + 1e-3,
                        "{n} seats in {window}: a mat's border lands at x {}, outside -1..{right}",
                        at.x
                    );
                }
            }
        }
    }

    /// The shot is as close as the free band allows — around the felt.
    ///
    /// What the camera frames is the table plus [`AIR`], and *that* is what
    /// has to fill the band: a fit with room to spare on both axes is a fit
    /// that could have come in, and every card at the table is drawn smaller
    /// for it. This used to be the case at every seat count, because the fit
    /// measured the corners of the box around the table, and on a ring those
    /// corners are bare felt — at three seats it filled 86% of the width it
    /// was given and 81% of the height, binding on neither.
    ///
    /// Bounded from below as well, on the bare table this time, because the
    /// two failures look nothing alike and only one of them is arithmetic: a
    /// camera that could have come in wastes the screen, and a camera pushed
    /// out until the table is a coaster in the middle of it has answered a
    /// question nobody asked.
    #[test]
    fn the_shot_is_as_close_as_the_band_allows() {
        let canvas = Canvas::hud(WINDOW);
        let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
        let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
        let right = 1.0 - 2.0 * canvas.right / canvas.window.x;
        let fill = |rig: CameraRig, points: &[Vec2]| {
            let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
            for &corner in points {
                let at = project(rig, canvas, corner);
                lo = lo.min(at);
                hi = hi.max(at);
            }
            Vec2::new(
                (hi.x - lo.x) / (right + 1.0),
                (hi.y - lo.y) / (top - bottom),
            )
        };
        for n in 2..=8u8 {
            let layout = TableLayout::new(&seats(n), 2.01, None);
            let rig = CameraRig::home(&layout, canvas);
            let framed = fill(rig, &layout.corners(AIR));
            assert!(
                framed.x.max(framed.y) > 0.93,
                "{n} seats fills {:.0}% of the band across and {:.0}% along it, \
                 so the camera could have come in",
                framed.x * 100.0,
                framed.y * 100.0
            );
            let table = fill(rig, &places(&layout));
            // 0.7 before there was a sky. [`AIR`] now leaves a deliberate
            // band outside the slab, and the play area gives up nine per
            // cent of its width to it — which is a *decision*, so the bound
            // moves with it rather than the decision being reverted to keep
            // a number. What the bound still catches is the failure it was
            // written for: a camera pushed out until the table is a coaster.
            assert!(
                table.x.max(table.y) > 0.6,
                "{n} seats leaves the table filling {:.0}% across and {:.0}% along, \
                 which is more room than a table needs around it",
                table.x * 100.0,
                table.y * 100.0
            );
        }
    }

    /// And what is left over is left over on both sides of it.
    ///
    /// The far edge used to be pinned under the tab strip and every spare
    /// unit opened up in front of the local seat — on a duel a fifth of the
    /// window of bare felt below the mats, with the whole table riding high.
    /// Measured in table units rather than on screen, because perspective
    /// makes the same span of felt a different height at each end.
    #[test]
    fn the_table_sits_in_the_middle_of_what_can_be_seen() {
        let canvas = Canvas::hud(WINDOW);
        let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
        let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
        for n in 2..=8u8 {
            let layout = TableLayout::new(&seats(n), 2.01, None);
            let rig = CameraRig::home(&layout, canvas);
            let (min, max) = layout.extent().expect("a seated table has an extent");
            let eye = rig.distance * (1.0 + CAMERA_LEAN * CAMERA_LEAN).sqrt();
            // The rig stores world x/z; `+y` away from the local seat is `-z`.
            let along = -rig.target.y;
            let behind = eye * ground(top) - (max.y - along);
            let ahead = (min.y - along) - eye * ground(bottom);
            assert!(
                (behind - ahead).abs() < 0.1,
                "{n} seats: {behind:.2} units of felt behind the far seat \
                 against {ahead:.2} in front of the near one"
            );
        }
    }

    /// The bug this whole framing exists for: the table shipped with a
    /// hard-coded 20-unit camera looking at the middle of the felt, and the
    /// local seat's own mat came out *underneath the hand bar*. A player
    /// could not see their own creatures.
    #[test]
    fn the_local_seats_own_mat_is_not_behind_the_hand_bar() {
        let canvas = Canvas::hud(WINDOW);
        let layout = TableLayout::new(&seats(2), 1.78, None);
        let local = layout.local().copied().expect("a local seat");
        let near = local.center.y - local.half_extent.y;

        let bad = project(CameraRig::default(), canvas, Vec2::new(0.0, near));
        let floor = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
        assert!(
            bad.y < floor,
            "the old framing is supposed to be the broken one: {} vs {floor}",
            bad.y
        );

        let good = project(
            CameraRig::home(&layout, canvas),
            canvas,
            Vec2::new(0.0, near),
        );
        assert!(
            good.y >= floor,
            "the near edge of my own mat is still under the hand bar: {} vs {floor}",
            good.y
        );
    }

    #[test]
    fn every_seats_place_is_inside_the_part_of_the_window_you_can_see() {
        let canvas = Canvas::hud(WINDOW);
        for n in 2..=8 {
            let layout = TableLayout::new(&seats(n), 1.78, None);
            let rig = CameraRig::home(&layout, canvas);
            let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
            let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
            let right = 1.0 - 2.0 * canvas.right / canvas.window.x;
            for corner in places(&layout) {
                let at = project(rig, canvas, corner);
                assert!(
                    at.y >= bottom - 1e-3 && at.y <= top + 1e-3,
                    "{n} seats: {corner} lands at y {} , outside {bottom}..{top}",
                    at.y
                );
                assert!(
                    at.x >= -1.0 - 1e-3 && at.x <= right + 1e-3,
                    "{n} seats: {corner} lands at x {} , outside -1..{right}",
                    at.x
                );
            }
        }
    }

    /// A window is not always a laptop's. The framing has to hold for a phone
    /// held upright, where the HUD covers proportionally far more of it.
    #[test]
    fn the_framing_holds_on_a_tall_narrow_window() {
        let canvas = Canvas::hud(Vec2::new(430.0, 932.0));
        let layout = TableLayout::new(&seats(4), 0.46, None);
        let rig = CameraRig::home(&layout, canvas);
        let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
        let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
        // Both bounds, because only checking the near edge is exactly the
        // hole that let the hand bar bug through in the first place: a shot
        // aimed too far off can satisfy one edge by breaking the other.
        for corner in corners(&layout) {
            let at = project(rig, canvas, corner);
            assert!(
                at.y >= bottom - 1e-3 && at.y <= top + 1e-3,
                "{corner} lands at y {}, outside {bottom}..{top}",
                at.y
            );
        }
        // Sideways this used to be a deliberate gap: a four-seat table did
        // not fit a phone at any honest distance, and the test asserted the
        // overflow so nobody would mistake it for working. It fits now, and
        // not because the camera got cleverer — the ring solve takes the pile
        // strips out of `x` before it fits the span to the canvas, so the
        // whole table is a strip narrower than it was and each seat's ground
        // a strip narrower still. The claim is now the strong one, and it is
        // made about the seat's whole *place*, piles included, because that
        // is the box the camera frames.
        let right = 1.0 - 2.0 * canvas.right / canvas.window.x;
        for corner in places(&layout) {
            let at = project(rig, canvas, corner);
            assert!(
                at.x >= -1.0 - 1e-3 && at.x <= right + 1e-3,
                "{corner} lands at x {}, outside -1..{right}",
                at.x
            );
        }
    }

    #[test]
    fn a_table_with_nobody_at_it_frames_nothing_rather_than_dividing_by_zero() {
        let rig = CameraRig::home(&TableLayout::new(&[], 1.78, None), Canvas::hud(WINDOW));
        assert_eq!(rig, CameraRig::default());
        assert!(rig.distance.is_finite());
    }

    /// `docs/design.md` §1.1: the middle of the table is atmosphere, the mats
    /// are the game. The bound is measurable, so it is measured — and it goes
    /// both ways.
    ///
    /// This replaces the lamplight ring's version of the same rule, which the
    /// felt's own open middle has taken over from. That test is worth remembering
    /// twice: it first compared the ring against `SeatSlot::lane_width`, the
    /// mat's *long* edge, which a ring two and a half times the mat's depth
    /// passes comfortably — the hearth dominated four straight screenshots
    /// while its test agreed it was small. So the medallion is measured
    /// against the gap it actually sits in, and the lower bound is here for
    /// the reason `docs/client.md` gives about the felt's own brightness: a
    /// one-sided assertion only stops the mistake it was written after, and
    /// the opposite mistake ships next.
    /// The cloth is written twice — once in Rust, where a test can block the
    /// image at card size and measure that the tooth survives and that the
    /// baize stays dark enough to read a card against, and once in WGSL,
    /// where the GPU actually draws it. Nothing in either compiler can notice
    /// that they have drifted apart, and the drawing is the one nobody can
    /// assert about directly.
    ///
    /// The two are not pixel-identical and are not meant to be: a lattice
    /// hash on the CPU and a `sin`-based one on the GPU give the same kind of
    /// noise and not the same noise, and the shader works in table units
    /// where the generator works in texels. What has to agree is every colour
    /// a test or a person reasoned about — the three the cloth is mixed from,
    /// and the three the rail and its apron are.
    #[test]
    fn the_shader_and_the_generator_agree_about_the_cloth() {
        let src = include_str!("shaders/felt.wgsl");
        for (name, ours) in [
            ("FELT_DEEP", tabletop::FELT_DEEP),
            ("FELT_CLOTH", tabletop::FELT_CLOTH),
            ("FELT_WORN", tabletop::FELT_WORN),
            ("RAIL_HIDE", tabletop::RAIL_HIDE),
            ("RAIL_LIP", tabletop::RAIL_LIP),
            ("APRON", tabletop::APRON),
        ] {
            let line = src
                .lines()
                .find(|line| line.trim_start().starts_with(&format!("const {name}:")))
                .unwrap_or_else(|| panic!("the shader has no {name}"));
            let Some((inside, _)) = line
                .rsplit_once("vec3<f32>(")
                .and_then(|(_, tail)| tail.split_once(')'))
            else {
                panic!("{name} is not a vec3 literal: {line}")
            };
            let theirs: Vec<f32> = inside
                .split(',')
                .map(|part| part.trim().parse().expect("a number"))
                .collect();
            assert_eq!(theirs.len(), 3, "{name} has {} channels", theirs.len());
            for (channel, (ours, theirs)) in ours.iter().zip(&theirs).enumerate() {
                assert!(
                    (ours - theirs).abs() < 1e-6,
                    "{name} channel {channel}: {ours} here, {theirs} in the shader"
                );
            }
        }
    }

    /// The mat is written twice for the same reason the cloth is: once in
    /// Rust, where `tabletop::seat_mat`'s tests can measure that only the rim
    /// carries the seat's colour and that the seam falls between two lanes,
    /// and once in WGSL, where the GPU actually draws it. Nothing in either
    /// compiler can notice that they have drifted apart.
    ///
    /// Every number here is a shading decision that was argued somewhere —
    /// the lanes are a quarter of what they first were, the rim's hue and its
    /// opacity ride two different exponents on purpose — so a copy of one of
    /// them in the shader that no longer matched would silently undo the
    /// argument. The lengths (`MAT_CORNER`, `MAT_RIM`) are not here: they
    /// travel to the GPU as uniforms, so there is only ever one of each.
    #[test]
    fn the_shader_and_the_generator_agree_about_the_mat() {
        let src = include_str!("shaders/mat.wgsl");
        let read = |name: &str| crate::cardmat::tests::wgsl_const(src, name);
        for (name, ours) in [
            ("LANE_NEAR", tabletop::MAT_LANES[0]),
            ("LANE_MID", tabletop::MAT_LANES[1]),
            ("LANE_FAR", tabletop::MAT_LANES[2]),
            ("LANE_LEDGE", tabletop::MAT_LEDGE_VALUE),
            ("LEDGE_FRAC", tabletop::LEDGE_FRAC),
            ("LANE_FRAC", tabletop::LANE_FRAC),
            ("MARGIN_FRAC", tabletop::MARGIN_FRAC),
            ("LEDGE_SEAM", tabletop::MAT_LEDGE_SEAM),
            ("SEAM", tabletop::MAT_SEAM),
            ("SEAM_W", tabletop::MAT_SEAM_WIDTH),
            ("RIM_LIGHT", tabletop::MAT_RIM_LIGHT),
            ("RIM_FALL", tabletop::MAT_RIM_FALL),
            ("HUE_FALL", tabletop::MAT_HUE_FALL),
        ] {
            let theirs = read(name);
            assert!(
                (ours - theirs).abs() < 1e-6,
                "{name} is {ours} here and {theirs} in the shader"
            );
        }
    }

    /// The mat's WGSL is parsed and validated with the front end wgpu uses.
    ///
    /// `the_shader_and_the_generator_agree_about_the_mat` reads constants out
    /// of this file as text and would go on passing over a shader that does
    /// not compile — and nothing else here ever compiled it, so a typo in the
    /// mat's arithmetic surfaced as a mat that simply did not draw, with the
    /// reason in a browser console.
    #[test]
    fn the_mat_shader_compiles() {
        let prelude = "\
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
struct Globals { time: f32 };
@group(0) @binding(11) var<uniform> globals: Globals;
";
        crate::cardmat::tests::check_wgsl(include_str!("shaders/mat.wgsl"), prelude);
    }

    /// `PILE_REACH` is chosen in the model crate, which cannot see the mat's
    /// printed border — that is `ZONE_MARGIN`, and it lives here. This is the
    /// two of them being made to agree.
    #[test]
    fn a_pile_stands_clear_of_the_mat_it_serves() {
        let spare = baylee_client_core::layout::PILE_REACH - CARD_WIDTH * 0.5 - ZONE_MARGIN;
        assert!(
            spare > 0.0,
            "a pile's near edge falls {} inside the mat's own border — it \
             would be lying on the board it stands beside, not on the table",
            -spare
        );
    }

    #[test]
    fn the_medallion_floats_in_the_open_middle() {
        let gap = baylee_client_core::layout::CENTRE_GAP;
        assert!(
            MEDALLION_SIZE < gap,
            "the colour wheel is {MEDALLION_SIZE} across a gap of {gap} — it \
             would be lying on both players' mats"
        );
        // And with felt visible on both sides of it, or it is not inlaid in
        // anything: it is a lid.
        let bare = (gap - MEDALLION_SIZE) * 0.5;
        assert!(
            bare > 0.4,
            "only {bare} of table shows beside the medallion"
        );

        for n in [2, 4, 6] {
            let layout = TableLayout::new(&seats(n), 2.0, None);
            let local = layout.local().copied().expect("a local seat");
            assert!(
                MEDALLION_SIZE < local.mat_depth(),
                "at {n} seats the eye lands on the medallion, not on the board: \
                 {MEDALLION_SIZE} vs a mat {} deep",
                local.mat_depth()
            );
            assert!(
                MEDALLION_SIZE > local.lane_height(),
                "at {n} seats the medallion has shrunk to nothing: \
                 {MEDALLION_SIZE} vs a lane {} tall",
                local.lane_height()
            );
        }
    }
}

#[cfg(test)]
mod zone_tests {
    use super::*;

    /// Dimmest first. `Standing`'s own declaration order is not this one —
    /// it is written in the order the *reading* code asks its questions —
    /// so the ranking a mat draws is stated here rather than assumed from
    /// the enum.
    const RANKED: [Standing; 4] = [
        Standing::Lost,
        Standing::Waiting,
        Standing::Active,
        Standing::Priority,
    ];

    fn mood(local: bool, standing: Standing) -> Mood {
        Mood {
            local,
            standing,
            // These tests are about brightness, which `on_turn` does not
            // touch: it drives the rim light and nothing else. A fixed
            // `false` keeps them measuring the one thing they measure.
            on_turn: false,
        }
    }

    /// The mat's tint is neutral, so this multiplies white and 1.0 is the
    /// ceiling: two moods above it are one mood as far as a player can see.
    ///
    /// The old scale ran to 1.311 and was safe only because it was applied to
    /// an accent colour first. Moving the accent into the rim's texture — the
    /// fix for a local mat that read as brass — is what made this a bound,
    /// and it is asserted rather than remembered because the two changes are
    /// in different files and nothing else connects them.
    #[test]
    fn no_mood_asks_for_more_light_than_white() {
        for local in [false, true] {
            for standing in RANKED {
                let value = zone_brightness(mood(local, standing));
                assert!(
                    value > 0.0 && value <= 1.0,
                    "a mat drawn at {value} is clipped, not bright"
                );
            }
        }
    }

    #[test]
    fn a_dimmer_standing_is_always_drawn_dimmer() {
        for local in [false, true] {
            for pair in RANKED.windows(2) {
                let (dim, bright) = (
                    zone_brightness(mood(local, pair[0])),
                    zone_brightness(mood(local, pair[1])),
                );
                assert!(
                    dim < bright,
                    "{:?} and {:?} are drawn {dim} and {bright}",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    /// Whose mat is mine is answered by the gilt rim. Brightness answers who
    /// everyone is waiting for, and the two must not compete: a local seat
    /// idling has to stay dimmer than an opponent one rank above it, or the
    /// felt points at the wrong player on every priority pass.
    #[test]
    fn a_standing_always_outranks_being_the_local_seat() {
        for pair in RANKED.windows(2) {
            let mine = zone_brightness(mood(true, pair[0]));
            let theirs = zone_brightness(mood(false, pair[1]));
            assert!(
                mine < theirs,
                "my {:?} mat at {mine} outshines their {:?} at {theirs}",
                pair[0],
                pair[1]
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(facing: f32) -> SeatSlot {
        SeatSlot {
            player: baylee_core::ids::PlayerId::new(0),
            ring_index: 0,
            angle: facing,
            center: Vec2::ZERO,
            facing,
            half_extent: Vec2::new(6.0, 3.0),
            is_local: true,
        }
    }

    /// Every vertex of the card mesh, as (x, y, z).
    fn points(mesh: &Mesh) -> Vec<(f32, f32, f32)> {
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("card mesh has positions")
        };
        p.iter().map(|v| (v[0], v[1], v[2])).collect()
    }

    /// The outline of the printed face, in order, as (x, y) pairs.
    ///
    /// Vertex 0 is the fan's centre and is not part of the outline; the wall
    /// vertices that follow the face are told apart by their normal, which is
    /// the thing that actually distinguishes them.
    fn rim(mesh: &Mesh) -> Vec<(f32, f32)> {
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(n)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("card mesh has normals")
        };
        let face = n.iter().take_while(|v| v[2] > 0.5).count();
        points(mesh)[1..face].iter().map(|v| (v.0, v.1)).collect()
    }

    /// Triangles as index triples.
    fn triangles(mesh: &Mesh) -> Vec<[u32; 3]> {
        let Some(Indices::U32(idx)) = mesh.indices() else {
            panic!("card mesh is indexed")
        };
        idx.as_chunks::<3>().0.to_vec()
    }

    /// Twice the signed area of a triangle, positive when it is wound
    /// counter-clockwise seen from +z.
    fn cross(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    }

    // The card that shipped as a bowtie: every corner arc swept the quarter
    // turn belonging to its neighbour, so the outline crossed itself twice
    // through the middle and a permanent on the battlefield was drawn as a
    // small bright X. `an_untapped_card_lies_flat_on_the_table` passed the
    // whole time — the transform was never the problem — so the mesh needs
    // tests of its own.
    #[test]
    fn the_card_outline_never_folds_through_its_own_middle() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let rim = rim(&mesh);
        // Walking a convex outline turns the same way at every vertex and
        // comes back around exactly once. A bowtie turns back on itself.
        let mut turn = 0.0_f32;
        for i in 0..rim.len() {
            let (a, b, c) = (rim[i], rim[(i + 1) % rim.len()], rim[(i + 2) % rim.len()]);
            assert!(
                cross(a, b, c) >= 0.0,
                "the outline turns back on itself at vertex {i}: {a:?} {b:?} {c:?}"
            );
            let before = (b.1 - a.1).atan2(b.0 - a.0);
            let after = (c.1 - b.1).atan2(c.0 - b.0);
            let mut delta = after - before;
            while delta > std::f32::consts::PI {
                delta -= std::f32::consts::TAU;
            }
            while delta < -std::f32::consts::PI {
                delta += std::f32::consts::TAU;
            }
            turn += delta;
        }
        assert!(
            (turn - std::f32::consts::TAU).abs() < 1e-3,
            "a closed convex outline turns through exactly one full circle, not {turn}"
        );
    }

    #[test]
    fn the_card_mesh_covers_the_card() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let rim = rim(&mesh);
        let area: f32 = (0..rim.len())
            .map(|i| {
                let (a, b) = (rim[i], rim[(i + 1) % rim.len()]);
                a.0.mul_add(b.1, -(b.0 * a.1))
            })
            .sum::<f32>()
            / 2.0;
        // The rounded rectangle, minus what the four corner arcs cut away.
        // The arcs are drawn in segments, so the mesh is a hair under.
        let ideal =
            CARD_WIDTH * CARD_HEIGHT - (4.0 - std::f32::consts::PI) * CARD_CORNER * CARD_CORNER;
        assert!(
            area > ideal * 0.99 && area <= ideal,
            "a card of {CARD_WIDTH}×{CARD_HEIGHT} covers about {ideal}, not {area}"
        );
        // And it stays inside the card: no vertex may stick out past an edge.
        for (x, y) in rim {
            assert!(
                x.abs() <= CARD_WIDTH / 2.0 + 1e-5 && y.abs() <= CARD_HEIGHT / 2.0 + 1e-5,
                "({x}, {y}) is outside the card"
            );
        }
    }

    #[test]
    fn every_face_triangle_faces_the_printed_side() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let points = points(&mesh);
        let outline = rim(&mesh).len();
        // Back-face culling is on, so a triangle wound the other way is an
        // invisible sliver of card.
        for tri in triangles(&mesh) {
            if tri.iter().any(|i| *i as usize > outline) {
                continue; // a wall triangle; checked below
            }
            let flat = |i: u32| (points[i as usize].0, points[i as usize].1);
            let (a, b, c) = (flat(tri[0]), flat(tri[1]), flat(tri[2]));
            assert!(
                cross(a, b, c) > 0.0,
                "face triangle {tri:?} faces away from the camera"
            );
        }
    }

    /// A card is a slab, not a decal: it has a wall around its edge so it
    /// reads as lying *on* the table. Wound the other way, that wall is a
    /// card you can see straight through from the side.
    #[test]
    fn the_card_wall_faces_outwards_and_stands_on_the_table() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let points = points(&mesh);
        let outline = rim(&mesh).len();
        let mut walls = 0;
        for tri in triangles(&mesh) {
            if tri.iter().all(|i| *i as usize <= outline) {
                continue;
            }
            walls += 1;
            let corner = |i: u32| points[i as usize];
            let (first, second, third) = (corner(tri[0]), corner(tri[1]), corner(tri[2]));
            // The triangle's normal, and the direction away from the card's
            // axis at its centroid. A wall faces out when they agree.
            let edge_a = (second.0 - first.0, second.1 - first.1, second.2 - first.2);
            let edge_b = (third.0 - first.0, third.1 - first.1, third.2 - first.2);
            let normal = (
                edge_a.1 * edge_b.2 - edge_a.2 * edge_b.1,
                edge_a.2 * edge_b.0 - edge_a.0 * edge_b.2,
                edge_a.0 * edge_b.1 - edge_a.1 * edge_b.0,
            );
            let out = (
                (first.0 + second.0 + third.0) / 3.0,
                (first.1 + second.1 + third.1) / 3.0,
            );
            assert!(
                normal.0.mul_add(out.0, normal.1 * out.1) > 0.0,
                "wall triangle {tri:?} faces into the card"
            );
            for point in [first, second, third] {
                assert!(
                    point.2 >= -1e-6 && point.2 <= CARD_THICKNESS + 1e-6,
                    "the wall runs past the card's own thickness at {point:?}"
                );
            }
        }
        assert_eq!(
            walls,
            outline * 2,
            "the wall does not close around the card"
        );
    }

    /// The printed face is what a player looks at, and it has to be the
    /// topmost surface — a face level with the wall would z-fight along every
    /// edge of every card on the table.
    #[test]
    fn the_printed_face_sits_on_top_of_the_slab() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let points = points(&mesh);
        let outline = rim(&mesh).len();
        for point in &points[..=outline] {
            assert!(
                (point.2 - CARD_THICKNESS).abs() < 1e-6,
                "a face vertex is not on top: {point:?}"
            );
        }
        assert!(
            points.iter().any(|p| p.2.abs() < 1e-6),
            "nothing touches the table, so the card floats"
        );
    }

    /// The table is a slab with a thickness, and its surface is the plane
    /// everything on the stage is placed against.
    ///
    /// Both halves matter and the second is the one that would break the
    /// board silently: the body hangs *below* zero. Build it standing on zero
    /// instead and every mat, glow, medallion and card is inside the table,
    /// which no test about transforms would notice — the transforms would all
    /// still be right.
    #[test]
    fn the_table_is_a_slab_and_not_a_sheet() {
        let span = Vec2::new(34.0, 26.0);
        let mesh = slab_mesh(span);
        let points = points(&mesh);
        let high = points.iter().fold(f32::NEG_INFINITY, |a, p| a.max(p.2));
        let low = points.iter().fold(f32::INFINITY, |a, p| a.min(p.2));
        assert!(
            high.abs() < 1e-6,
            "the table's surface is not at zero: {high}"
        );
        assert!(
            (low + TABLE_THICKNESS).abs() < 1e-6,
            "the table is {} deep, not {TABLE_THICKNESS}",
            -low
        );
    }

    /// And the table stops before the window does, so there is a sky to see.
    ///
    /// Measured against the window rather than against the table. The camera
    /// frames the layout plus [`AIR`] and the slab is cut to the layout plus
    /// [`SLAB_MARGIN`], so every point of the table's rim has to stand inside
    /// the framed box by the difference. This is the whole reason anything
    /// behind the table is ever visible, and it is one subtraction away from
    /// being false again — `SLAB_MARGIN` is derived from a rail width that a
    /// later change to the table's look could quietly grow.
    #[test]
    fn the_table_stops_before_the_window_does() {
        let span = Vec2::new(34.0, 26.0);
        let rim = rim(&slab_mesh(span));
        let framed = span * 0.5 + Vec2::splat(AIR - SLAB_MARGIN);
        let band = AIR - SLAB_MARGIN;
        assert!(
            band > 0.0,
            "the slab is cut wider than the shot that frames it"
        );
        for &(x, y) in &rim {
            assert!(
                x.abs() <= framed.x - band + 1e-3 && y.abs() <= framed.y - band + 1e-3,
                "the table reaches ({x}, {y}), inside a frame of {framed:?}"
            );
        }
        // And the corner is a corner rather than a bite out of the play area.
        let bite = tabletop::table_corner(span) * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
        assert!(
            bite < SLAB_MARGIN,
            "the corner takes {bite} out of each end, which is play area"
        );
    }

    #[test]
    fn the_card_face_is_mapped_corner_to_corner() {
        let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("card mesh has uvs")
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("card mesh has positions")
        };
        // The printed face runs left→right and top→bottom, so the top-left
        // of the card is (0,0) in the image and the bottom-right is (1,1).
        for (p, uv) in pos.iter().zip(uvs) {
            let want_u = f32::midpoint(p[0] / (CARD_WIDTH / 2.0), 1.0);
            let want_v = (1.0 - p[1] / (CARD_HEIGHT / 2.0)) * 0.5;
            assert!((uv[0] - want_u).abs() < 1e-5 && (uv[1] - want_v).abs() < 1e-5);
            assert!((0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1]));
        }
    }

    /// The geometry and the two shaders have to round the card at the same
    /// radius, and nothing in the compiler can notice that they do: one is a
    /// Rust constant and the others are text in a `.wgsl` file. So the text
    /// is read.
    ///
    /// Cut the mesh wider than the print and the shader's ink has nothing
    /// left to reach; cut it narrower and a white sliver of the scanner bed
    /// survives outside it. Either way the card stops looking like a card,
    /// which is the entire point of cutting the corner at all.
    #[test]
    fn the_mesh_and_the_shaders_round_the_card_alike() {
        let printed = crate::cardmat::tests::wgsl_const(
            include_str!("shaders/card_common.wgsl"),
            "PRINTED_CORNER",
        );
        assert!(
            (CARD_CORNER / CARD_WIDTH - printed).abs() < 1e-6,
            "the mesh rounds at {} of its width, the shaders at {printed}",
            CARD_CORNER / CARD_WIDTH
        );
    }

    #[test]
    fn table_space_maps_away_from_the_seat_into_the_screen() {
        // +y in table space is away from the local seat, which is -z in the
        // world because the camera sits on +z.
        let world = to_world(Vec2::new(2.0, 5.0), 0.0);
        assert!((world.x - 2.0).abs() < 1e-5);
        assert!((world.z + 5.0).abs() < 1e-5);
    }

    #[test]
    fn an_untapped_card_lies_flat_on_the_table() {
        let t = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        // The quad's normal (+z in local space) should point straight up.
        let normal = t.rotation * Vec3::Z;
        assert!((normal.y - 1.0).abs() < 1e-4, "normal was {normal:?}");
    }

    #[test]
    fn tapping_rotates_a_quarter_turn_but_keeps_the_card_on_the_table() {
        let untapped = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        let tapped = card_transform(&slot(0.0), Vec2::ZERO, true, 0.0);
        assert_ne!(untapped.rotation, tapped.rotation);

        // Still flat: only the in-plane orientation changed.
        let normal = tapped.rotation * Vec3::Z;
        assert!((normal.y - 1.0).abs() < 1e-4, "normal was {normal:?}");

        // The card's long axis has swung to the side.
        let up = tapped.rotation * Vec3::Y;
        assert!(
            up.y.abs() < 1e-4,
            "long axis should now lie across the table"
        );
    }

    /// The material every hidden card wears is built before the printed back
    /// has been fetched and dressed in it afterwards, in place — so what
    /// dressing produces has to be the material the picture would have been
    /// built into. The half that is easy to forget is `has_art`: it follows
    /// the handle and not the look, and a material carrying the back's
    /// texture with the flag still at zero goes on drawing the flat colour.
    #[test]
    fn a_dressed_back_is_the_material_the_picture_would_have_built() {
        let picture = Handle::<Image>::default();
        let look = CardLook::flat(BACK_COLOR, FinishTreatment::Plain, 0);

        let mut dressed = material(look, None, BACK_COLOR, MOVING);
        assert!(dressed.art.is_none() && dressed.params.has_art == 0.0);
        dress_in_the_back(&mut dressed, picture.clone());

        let built = material(look, Some(picture), BACK_COLOR, MOVING);
        assert_eq!(dressed.art, built.art);
        assert_eq!(
            format!("{:?}", dressed.params),
            format!("{:?}", built.params),
            "dressing changed something the builder would not have"
        );
    }

    #[test]
    fn cards_float_above_the_felt_so_they_never_z_fight() {
        let t = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        assert!(t.translation.y > TABLE_Y);
    }

    #[test]
    fn a_seat_across_the_table_has_its_cards_turned_to_face_it() {
        let near = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
        let far = card_transform(&slot(std::f32::consts::PI), Vec2::ZERO, false, 0.0);
        // Both flat, but their in-plane orientation is opposite.
        let near_up = near.rotation * Vec3::Y;
        let far_up = far.rotation * Vec3::Y;
        assert!(
            near_up.dot(far_up) < -0.9,
            "far seat should be turned around"
        );
    }

    #[test]
    fn only_counted_groups_get_a_badge() {
        use baylee_client_core::board::{KeywordBadge, Provenance};
        use baylee_view::ObjectStatus;

        let mut group = CardGroup {
            representative: ObjectId::new(1, 0),
            members: vec![ObjectId::new(1, 0)],
            name: "Soldier".into(),
            power: Some(1),
            toughness: Some(1),
            damage: 0,
            loyalty: None,
            status: ObjectStatus::NONE,
            counters: vec![],
            badges: Vec::<KeywordBadge>::new(),
            art: None,
            provenance: Provenance::Token,
            original: None,
            summoning_sick: false,
            activatable: false,
            commander: false,
            individual: None,
        };
        assert_eq!(stack_badge(&group), None);
        group.members.push(ObjectId::new(2, 0));
        group.members.push(ObjectId::new(3, 0));
        assert_eq!(stack_badge(&group).as_deref(), Some("×3"));
    }
    /// The finish is a property of the *printing*, and the print table is per
    /// seat: a printing this seat has not earned reads as plain rather than
    /// as a hole in the hidden-information rule. This pins the lookup that
    /// makes that true, because the alternative — reading a finish off the
    /// card — would be the leak.
    #[test]
    fn a_printing_a_seat_has_not_earned_is_drawn_plain() {
        use baylee_client_core::images::ArtSize;
        use baylee_core::ids::PrintRef;

        let statics = baylee_view::GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: String::new(),
            your_seat: baylee_core::ids::PlayerId::new(0),
            seats: Vec::new(),
            prints: vec![
                Some(baylee_view::PrintEntry {
                    scryfall_id: "11111111-2222-3333-4444-555555555555".to_string(),
                    lang: "en".to_string(),
                    finish: baylee_view::Finish::Foil,
                }),
                // Earned by nobody: the seat has not seen this card.
                None,
            ],
        };
        let look = |slot: u16| {
            let key = ImageKey::new(PrintRef(slot), 0, ArtSize::Normal);
            key.printing()
                .and_then(|p| statics.print(p))
                .map_or(FinishTreatment::Plain, |entry| entry.finish.into())
        };
        assert_eq!(look(0), FinishTreatment::Foil, "its own deck's printing");
        assert_eq!(look(1), FinishTreatment::Plain, "a hole is not a foil");
    }
}

/// The ways off the table and the ways back onto it.
///
/// Geometry, not implementation: what is asserted is that a card bound for a
/// pile ends up *at that pile* and behind the card standing on it, that a card
/// bound nowhere ends up at nothing, and that an arrival from a pile is the
/// exit to it run backwards. Each of them has a counter-arm as well — a
/// version that gave every zone the same pose would pass none of these.
#[cfg(test)]
mod exit_tests {
    use super::*;
    use std::time::Duration;

    fn seat() -> SeatSlot {
        SeatSlot {
            player: baylee_core::ids::PlayerId::new(0),
            ring_index: 0,
            angle: 0.0,
            center: Vec2::ZERO,
            facing: 0.0,
            half_extent: Vec2::new(6.0, 3.0),
            is_local: true,
        }
    }

    /// A card lying flat on the near seat's board.
    fn standing() -> Transform {
        card_transform(&seat(), Vec2::ZERO, false, 0.0)
    }

    /// Where that seat's graveyard stands, as `placements` puts its top card.
    fn grave() -> Transform {
        let slot = seat();
        card_transform(&slot, slot.pile_center(PileKind::Graveyard), false, 0.0)
    }

    /// The fault this whole pass exists to correct. A pile's top card *is* a
    /// placement, so a creature that dies while another dies with it has one
    /// of the two glide to the graveyard and the other marked stale — and the
    /// two must end up in the same place, because which of them is on top is
    /// an accident of sort order and nothing a player can see a reason for.
    #[test]
    fn a_permanent_that_dies_goes_to_the_pile_the_top_card_glides_to() {
        let at = standing();
        let pile = grave();
        let gone = exit(Some(Place::Graveyard(seat().player)), Some(pile), &at);
        assert!(
            (gone.translation.xz() - pile.translation.xz()).length() < 1e-4,
            "it belongs on the pile, not at its lane: {gone:?}"
        );
        assert_eq!(gone.rotation, pile.rotation, "lying as the pile lies");
        // And behind the card already standing there, or the two fight for
        // the same depth and the pile flickers between them.
        assert!(gone.translation.y < pile.translation.y, "tucked under it");
        assert!(
            pile.translation.y - gone.translation.y < CARD_HEIGHT * 0.01,
            "but only just: it is hidden, not dropped"
        );
    }

    #[test]
    fn a_bounced_permanent_leaves_the_way_a_card_arrives() {
        let at = standing();
        let gone = exit(Some(Place::Hand), None, &at);
        let arriving = entrance(&at);
        assert!(gone.translation.y > arriving.translation.y, "further up");
        assert!(gone.scale.length() < arriving.scale.length(), "and smaller");
        assert!(
            gone.translation.y > at.translation.y && gone.scale.length() < at.scale.length(),
            "which is the entrance run backwards"
        );
    }

    /// The honest exit. It must be confusable with neither of the others, or
    /// a card whose fate is unknown would be reported as buried.
    #[test]
    fn a_card_this_seat_cannot_follow_leaves_quietly() {
        let at = standing();
        for unknown in [None, Some(Place::Stack)] {
            let gone = exit(unknown, None, &at);
            assert!(
                (gone.translation - at.translation).length() < 1e-4,
                "it goes nowhere: {unknown:?}"
            );
            assert!(gone.scale.length() < at.scale.length() * 0.1, "{unknown:?}");
            // Exactly, not nearly: `Quat::angle_between` is an `acos` and is
            // worth about 7e-4 of noise on two identical rotations, which is
            // more slack than this assertion has to give.
            assert_eq!(gone.rotation, at.rotation, "and does not turn: {unknown:?}");
        }
    }

    /// A pile place with no pile to send it to is the same unknown fate, and
    /// has to read as one: a layout that has not arrived yet must not make a
    /// death look like a bounce.
    #[test]
    fn a_pile_this_table_is_not_drawing_is_no_destination_at_all() {
        let at = standing();
        let nowhere = exit(Some(Place::Graveyard(seat().player)), None, &at);
        assert_eq!(nowhere, exit(None, None, &at));
    }

    /// Coming back out is going in, run backwards — which is what makes a
    /// resurrection read as one rather than as a fresh card being made.
    #[test]
    fn a_permanent_returning_from_a_pile_comes_off_that_pile() {
        let at = standing();
        let pile = grave();
        let start = entrance_from(Some(pile), &at);
        assert_eq!(start, pile);
        let gone = exit(Some(Place::Graveyard(seat().player)), Some(pile), &at);
        // The tuck apart and no more, compared with a hair of slack: the two
        // differ by exactly `PILE_TUCK`, which an `f32` subtraction does not
        // reproduce to the last bit.
        assert!(
            (start.translation - gone.translation).length() < PILE_TUCK * 1.01,
            "it leaves from where it arrived: {start:?} against {gone:?}"
        );
        assert!(
            (start.scale - at.scale).length() < 1e-4,
            "at full size: it is the card coming back, not a card being made"
        );
    }

    /// A creature cast from hand arrives from the stack; a token arrives from
    /// nowhere. Both of them belong dropping onto their mark, which is what
    /// this table has always done.
    #[test]
    fn every_other_arrival_is_the_one_the_table_has_always_drawn() {
        let at = standing();
        assert_eq!(entrance_from(None, &at), entrance(&at));
    }

    /// The seat is half the answer: two graveyards are two places, and a card
    /// dying under an opponent's control belongs at *their* pile.
    #[test]
    fn two_seats_piles_are_two_different_places() {
        let mut far = seat();
        far.player = baylee_core::ids::PlayerId::new(1);
        far.center = Vec2::new(0.0, -12.0);
        far.facing = std::f32::consts::PI;
        let theirs = card_transform(&far, far.pile_center(PileKind::Graveyard), false, 0.0);
        assert!(
            (theirs.translation - grave().translation).length() > 1.0,
            "both graveyards drawn at the same point"
        );
    }

    /// The exits are only worth anything if the card is still there to play
    /// them, and only harmless if it eventually is not.
    #[test]
    fn a_departing_card_is_despawned_when_its_time_is_up_and_not_before() {
        let mut app = App::new();
        app.init_resource::<Time>().add_systems(Update, retire);
        let card = app.world_mut().spawn(Departing { left: EXIT_LIFE }).id();

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(EXIT_LIFE * 0.5));
        app.update();
        assert!(
            app.world().get_entity(card).is_ok(),
            "half way through it is still leaving"
        );

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(EXIT_LIFE));
        app.update();
        assert!(app.world().get_entity(card).is_err(), "and then it is gone");
    }

    /// A plain card material, as `material` would build one for a card with
    /// nothing happening to it.
    fn plain(materials: &mut Assets<CardMaterial>) -> Handle<CardMaterial> {
        materials.add(crate::cardmat::material(
            CardLook::flat(
                Color::srgb(0.5, 0.5, 0.5),
                baylee_client_core::images::FinishTreatment::Plain,
                0,
            ),
            None,
            Color::srgb(0.5, 0.5, 0.5),
            MOVING,
        ))
    }

    fn leaving_for(to: Option<Place>) -> zones::Move {
        zones::Move {
            object: ObjectId::new(1, 0),
            from: Some(Place::Battlefield),
            to,
        }
    }

    /// The claim item 4 is about, and the one nothing else can make: a card
    /// on its way out wears the door it is going through.
    ///
    /// An outcome and not a call — the handle changes and the params on the
    /// far side of it say which door — because the failure this guards is a
    /// dressing that runs and writes nothing, which looks from every other
    /// angle exactly like a card that left the ordinary way.
    #[test]
    fn a_card_leaving_the_table_wears_the_door_it_goes_through() {
        let mut materials = Assets::<CardMaterial>::default();
        let seat = baylee_core::ids::PlayerId::new(0);
        for (to, want) in [
            (Place::Graveyard(seat), crate::cardmat::door::DESTROYED),
            (Place::Exile(seat), crate::cardmat::door::EXILED),
            (Place::Hand, crate::cardmat::door::BOUNCE),
        ] {
            let worn = plain(&mut materials);
            let dressed =
                dress_the_exit(&mut materials, &worn, leaving_for(Some(to)), 12.0, MOVING)
                    .unwrap_or_else(|| panic!("a card going to {to:?} was dressed in nothing"));
            assert_ne!(dressed, worn, "it kept the material it arrived in");
            let params = materials.get(&dressed).expect("the new material").params;
            assert_eq!(params.sweep_door, want, "the wrong door for {to:?}");
            // And it is a real one-shot on the clock it was given, or the
            // shader draws the door at a phase it never leaves.
            assert!(
                (params.sweep_at - 12.0).abs() < 1e-3,
                "started at {} rather than now",
                params.sweep_at
            );
            assert!(
                (params.sweep_rate - 1.0 / EXIT_LIFE).abs() < 1e-3,
                "it does not last exactly as long as the exit it rides"
            );
        }
    }

    /// The counter-tests, which matter more than the claim: three ways of
    /// leaving that must stay undressed.
    #[test]
    fn a_card_leaving_by_no_door_is_dressed_in_nothing() {
        let mut materials = Assets::<CardMaterial>::default();
        let worn = plain(&mut materials);
        // Somewhere this seat cannot see: an opponent's hand, the bottom of a
        // library. `zones` reports `None` rather than guessing, and a guess
        // here would be a portal drawn over a card that was merely bounced.
        assert!(
            dress_the_exit(&mut materials, &worn, leaving_for(None), 12.0, MOVING).is_none(),
            "a move with one end missing was given a door"
        );
        // The stack. A permanent going there did not leave through any of the
        // five, and a mark on it would be the everywhere-at-once again.
        assert!(
            dress_the_exit(
                &mut materials,
                &worn,
                leaving_for(Some(Place::Stack)),
                12.0,
                MOVING
            )
            .is_none(),
            "leaving for the stack was drawn as a door"
        );
        // And a player who has turned motion off sees none of it — the same
        // decision an arriving card makes, made in the same function.
        let seat = baylee_core::ids::PlayerId::new(0);
        let still = dress_the_exit(
            &mut materials,
            &worn,
            leaving_for(Some(Place::Graveyard(seat))),
            12.0,
            crate::cardmat::STILL,
        )
        .expect("a still card is still dressed, just with nothing happening");
        let params = materials.get(&still).expect("the new material").params;
        assert!(
            params.sweep_rate.abs() < f32::EPSILON,
            "a still card was given a travel"
        );
        assert_eq!(
            params.sweep_door,
            crate::cardmat::door::NONE,
            "a still card was given a door to travel through"
        );
    }
}

/// Combat, as the table draws it.
///
/// The board here is built by hand rather than through a view: what is under
/// test is the *bridge* between the interaction state and the placements, and
/// a `PlayerView` in the middle would put the whole projection between the
/// thing being asserted and the thing being set.
#[cfg(test)]
mod combat_tests {
    use super::*;
    use baylee_client_core::board::{BoardModel, Lane, Provenance, SeatPod};
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::layout::LaneKind;
    use baylee_core::ids::Defender;
    use baylee_engine::choice::Pending;

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// One creature, standing for itself.
    fn creature(slot: u32) -> CardGroup {
        CardGroup {
            representative: obj(slot),
            members: vec![obj(slot)],
            name: format!("Creature {slot}"),
            power: Some(2),
            toughness: Some(2),
            damage: 0,
            loyalty: None,
            status: baylee_view::ObjectStatus::NONE,
            counters: Vec::new(),
            badges: Vec::new(),
            art: None,
            provenance: Provenance::Token,
            original: None,
            summoning_sick: false,
            activatable: false,
            commander: false,
            individual: None,
        }
    }

    /// A one-seat board holding `groups` in the creature lane.
    fn board(groups: Vec<CardGroup>) -> BoardModel {
        BoardModel {
            seq: 1,
            local: PlayerId::new(0),
            turn: 1,
            step: baylee_view::Step::DeclareAttackers,
            pods: vec![SeatPod {
                player: PlayerId::new(0),
                life: 20,
                poison: 0,
                energy: 0,
                hand_count: 0,
                library_count: 40,
                graveyard_count: 0,
                has_lost: false,
                is_local: true,
                is_active: true,
                has_priority: true,
                lanes: vec![Lane {
                    kind: LaneKind::Creatures,
                    groups,
                    overflowing: false,
                }],
                piles: baylee_client_core::PileKind::ALL
                    .into_iter()
                    .map(baylee_client_core::ZonePile::empty)
                    .collect(),
                tokens: Vec::new(),
                threat: baylee_client_core::ThreatSummary::default(),
            }],
            stack: Vec::new(),
            hand: Vec::new(),
        }
    }

    fn duel(board: BoardModel, interaction: Option<Interaction>) -> Duel {
        Duel {
            board: Some(board),
            layout: Some(TableLayout::new(&[PlayerId::new(0)], 1.78, None)),
            interaction,
            ..Duel::default()
        }
    }

    /// The defect `docs/design.md` §2.2 calls "unreadable": both combat
    /// prompts were fully operable and drew nothing, because the sync loop
    /// asked `selected()` — a list that stays empty in the two combat modes,
    /// where the answer being built is a list of *pairs*.
    #[test]
    fn a_declared_attacker_is_marked_chosen_and_an_undeclared_one_is_not() {
        let mut i = Interaction::new(
            Pending::ChooseAttackers {
                player: PlayerId::new(0),
                attackers: vec![obj(1), obj(2)],
                defenders: vec![Defender::Player(PlayerId::new(1))],
            },
            PlayerId::new(0),
        );
        assert!(
            i.declare_attacker(obj(1), Defender::Player(PlayerId::new(1))),
            "the choice offered this attacker"
        );

        let duel = duel(board(vec![creature(1), creature(2)]), Some(i));
        let placed = placements(&duel);
        let chosen = |id: ObjectId| {
            placed
                .iter()
                .find(|p| p.object == id)
                .unwrap_or_else(|| panic!("{id:?} is on the table"))
                .selected
        };
        assert!(chosen(obj(1)), "the declared attacker lies flat");
        assert!(
            !chosen(obj(2)),
            "a creature held back is drawn as attacking"
        );
    }

    /// A card standing for four is chosen when *any* of the four is: the
    /// representative is a drawing decision, and a plan or a declaration
    /// names one particular permanent.
    #[test]
    fn a_stack_of_identical_creatures_is_chosen_by_any_of_its_members() {
        let mut group = creature(1);
        group.members = vec![obj(1), obj(2), obj(3)];
        let mut i = Interaction::new(
            Pending::ChooseAttackers {
                player: PlayerId::new(0),
                attackers: vec![obj(1), obj(2), obj(3)],
                defenders: vec![Defender::Player(PlayerId::new(1))],
            },
            PlayerId::new(0),
        );
        // Not the representative — that is the whole point.
        assert!(i.declare_attacker(obj(3), Defender::Player(PlayerId::new(1))));

        let duel = duel(board(vec![group]), Some(i));
        let placed = placements(&duel);
        assert_eq!(placed.len(), 1, "one card stands for the three");
        assert!(
            placed[0].selected,
            "the card drawn for the stack ignores a declaration by a member \
             that is not its representative"
        );
    }

    /// The same rule outside combat, where `selected()` *is* the answer being
    /// built. Reading the raw list was wrong here too, just less visibly: a
    /// target chosen from a stack of four is one particular permanent, and
    /// the card drawn for the stack is the only thing on the table that can
    /// show it has been chosen.
    #[test]
    fn a_target_chosen_from_a_stack_lifts_the_card_that_stands_for_it() {
        let mut group = creature(1);
        group.members = vec![obj(1), obj(2), obj(3)];
        let mut i = Interaction::new(
            Pending::ChooseTargets {
                player: PlayerId::new(0),
                options: vec![obj(1), obj(2), obj(3)],
                player_options: vec![],
                min: 1,
                max: 1,
                reason: baylee_engine::choice::TargetPrompt::Targets,
            },
            PlayerId::new(0),
        );
        // Again not the representative.
        assert_eq!(
            i.toggle(obj(3)),
            baylee_client_core::interaction::SelectionOutcome::Added
        );

        let duel = duel(board(vec![group]), Some(i));
        assert!(
            placements(&duel)[0].selected,
            "the stack was targeted and the card drawn for it sits flat"
        );
    }

    /// Reported from a four-player game: the cards on the table flickered
    /// against each other in bands.
    ///
    /// A lane fans once it holds more than fits, and a fan is overlap by
    /// definition — so the two quads sharing a patch of felt were at exactly
    /// the same height, and which of them a pixel belongs to was decided by
    /// the last bit of an interpolated depth. Every card on a lane got
    /// `0.0`.
    #[test]
    fn cards_that_overlap_in_a_row_do_not_lie_at_the_same_height() {
        let seats: Vec<_> = (0..4).map(PlayerId::new).collect();
        let duel = Duel {
            // Enough of them that the row fans however wide a four-seat pod
            // is: twelve stopped fanning the day `MIN_POD_WIDTH` went up, and
            // a test that quietly stops testing is worse than one that fails
            // — which is what the assertion at the bottom is for.
            board: Some(board((1..=24).map(creature).collect())),
            layout: Some(TableLayout::new(&seats, 2.01, None)),
            ..Duel::default()
        };
        let placed = placements(&duel);

        let mut overlaps = 0;
        for pair in placed.windows(2) {
            if pair[0].position.distance(pair[1].position) >= CARD_WIDTH {
                continue;
            }
            overlaps += 1;
            assert!(
                pair[1].lift > pair[0].lift,
                "two cards {} apart — closer than a card is wide — both sit at {}",
                pair[0].position.distance(pair[1].position),
                pair[0].lift
            );
        }
        assert!(
            overlaps > 0,
            "the row did not fan, so nothing about overlapping cards was tested"
        );

        // And it stays a row: the whole rise is smaller than the gap a card
        // already floats above the felt, so this is order for the depth
        // buffer and not a staircase for the eye.
        let top = placed.iter().map(|p| p.lift).fold(0.0_f32, f32::max);
        assert!(
            top < CARD_LIFT,
            "a row of twelve climbs {top} off the table"
        );
    }
}
