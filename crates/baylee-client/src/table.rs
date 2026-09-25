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
use crate::badgemat::BadgeMaterial;
use crate::cardmat::{CardLook, CardMaterial, MOVING, material, motion_of};
use crate::face;
use crate::feltmat::FeltMaterial;
use crate::floormat::{self, FloorMaterial};
use crate::marksmat::MarksMaterial;
use crate::shellmat::{self, Band, ShellKind, ShellLook, ShellMaterial};
use crate::textures::CardTextures;
use baylee_client_core::airborne;
use baylee_client_core::board::KeywordBadge;
use baylee_client_core::card_face::CardFace;
use baylee_client_core::cardcrest;
use baylee_client_core::cardplate::{self, BadgePlace};
use baylee_client_core::cardrail;
use baylee_client_core::combat::Combat;
use baylee_client_core::images::{FinishTreatment, ImageKey};
use baylee_client_core::layout::{
    CARD_HEIGHT, CARD_WIDTH, PILE_JOG, PILE_SLABS, PileKind, STAGE_STEP, SeatSlot, TableLayout,
};
use baylee_client_core::tabletop;
use baylee_client_core::textface;
use baylee_client_core::zones::{self, Place, Tracker};
use baylee_core::color::ColorSet;
use baylee_core::ids::ObjectId;
use baylee_core::ids::PlayerId;
use baylee_core::types::{SubtypeSet, TypeSet};
use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Height of the table surface; cards float a hair above it so they never
/// z-fight with the felt.
pub(crate) const TABLE_Y: f32 = 0.0;
/// Vertical gap between the felt and a card.
pub(crate) const CARD_LIFT: f32 = 0.028;
/// Where a seat's mat sits: above the felt, below everything played on it.
pub(crate) const ZONE_LIFT: f32 = 0.002;
/// Where the glow under a mat sits — below the mat, above the felt.
const GLOW_LIFT: f32 = 0.001;
/// Turns a lift into the order the transparent pass draws it in.
///
/// The ladder above has always carried the comment *"the order of these
/// decides what draws over what; nothing here is depth-sorted"*, and the
/// second half of that was never true. Bevy sorts `Transparent3d` by
/// `rangefinder.distance(mesh_center) + depth_bias`, ascending, with the
/// distance **increasing towards the camera** — so what actually decided the
/// order was which end of the table a thing sat on. A mat on the near half
/// was drawn after a card's contact shadow on the far half and covered it;
/// nobody noticed, because a mat is a veil in the hundredths and a shadow is
/// a small dark ellipse, and because the two are rarely in the same place.
/// The air over the table is what made it worth fixing: it is the width of
/// the whole slab, so it meets every other blended surface at once and would
/// have been over the mats at one end and under them at the other.
///
/// A bias, and not a reordering of the lifts, because the lifts are *right* —
/// they are the heights these things are at, and the depth buffer uses them.
/// The gain only has to be large enough that the smallest rung of the ladder
/// beats the widest table: the rungs are half a thousandth apart and the
/// eight-seat ring reaches about forty units across, so 0.0005 × 400 000 =
/// 200 is a comfortable margin. `the_ladder_decides_what_covers_what` holds
/// both halves of that.
///
/// It touches nothing but the sort. Depth *writing* is off for every blended
/// surface here, so none of them was ever occluding another; what this fixes
/// is purely which one is painted last.
pub(crate) fn sort_bias(lift: f32) -> f32 {
    lift * SORT_GAIN
}

/// See [`sort_bias`].
const SORT_GAIN: f32 = 400_000.0;
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
/// How fast the rail follows a phase change, per second.
///
/// Slower than a card moves. A step boundary is not an event a player has to
/// catch — it is a condition they should notice having changed — and a
/// rail that snapped would flicker through the four steps of combat.
const WASH_RATE: f32 = 3.0;
/// How fast the firewheel follows what is on the battlefield, per second.
///
/// Slower again, and deliberately: a flame reaching its new height over
/// about a second and a half is a fire being fed, and one that jumps the
/// moment a land resolves is a notification in the middle of the table.
const FIRE_RATE: f32 = 0.8;
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
/// it. A reverse-`z` depth buffer resolves about an eight-millionth of the
/// eye's distance: some 4·10⁻⁵ of a unit at [`CameraRig::MAX_DISTANCE`],
/// which #264 took from 120 to 300. An ordinary fan of a dozen puts its
/// cards ten times that apart there and thirty times at the 95 units a
/// four-seat table stands at. A duel-wide lane packed all the way to
/// `MIN_VISIBLE_FRACTION` — past which a row scrolls, never showing more
/// than its lane holds — is some eighty cards and keeps about one step of
/// it at 300, which is an eight-seat table on a phone turned on its side,
/// where a card is drawn four pixels wide.
const LANE_RISE: f32 = 0.004;
// A row that rose further than a card floats would be a staircase, not a row.
const _: () = assert!(LANE_RISE < CARD_LIFT);

/// How far above its card's face the keyword strip lies (#274), as a share
/// of the step between two cards of its row.
///
/// A share of a step, and not a card's thickness, which is what the strip was
/// first planned at. The card laid over this one in a fanned lane is one step
/// higher, and a whole row's rise is [`LANE_RISE`] — 0.004, a quarter of a
/// thousandth a step in a fan of seventeen — so a strip lifted 0.055 would be
/// nearer the camera than the neighbour covering it and would be drawn over
/// that card's art. Under half a step, the neighbour hides the strip the way
/// it hides the rest of this card. What makes it read as an object lying on
/// the card is its contact shadow; a height of a card's thickness would not
/// have shown at this camera in any case.
///
/// A share rather than a length because the step shrinks as the row grows,
/// and half of any step is still that far from both of the faces around it.
const STRIP_STEP_SHARE: f32 = 0.5;
const _: () = assert!(STRIP_STEP_SHARE > 0.0 && STRIP_STEP_SHARE < 1.0);

/// Where the keyword strip sits in the transparent pass: the height it lies
/// at, on a card's face. See [`sort_bias`].
pub(crate) const STRIP_RUNG: f32 = CARD_LIFT + CARD_THICKNESS;
/// Where the offer's light on the felt lies (#298): over the contact
/// shadows, which are at half of [`CARD_LIFT`], and under every card, which
/// is at least all of it — so the card it is for covers its middle and the
/// next card of a fanned lane covers its edge. See [`sort_bias`].
pub(crate) const FLOOR_RUNG: f32 = CARD_LIFT * 0.75;
/// The back of a card: what a card whose art never arrives falls back to,
/// and what fills a slab's window under a pile, where nothing sees it.
const BACK_COLOR: Color = Color::srgb(0.12, 0.14, 0.18);

// A merged pile's cards step out by `layout::PILE_JOG` each, to the left and
// at most `PILE_SLABS` of them ([`pile_slab_transform`]); the row holds the
// room they take (`layout::pile_reach`). A zone pile's slabs alternate
// sides ([`slab_transform`]).
const _: () = assert!(PILE_JOG >= cardrail::MARK / 2.0);
/// The lighter of the two colours a pile's slabs wear in turn (#298): card
/// stock in shadow, so the layers stripe against the back beside them.
///
/// Twenty-odd display levels over [`BACK_COLOR`] and far short of the grey
/// the owner took off with the frame (`a_pile_shows_its_layers` bounds it both
/// ways).
const SLAB_EDGE_COLOR: Color = Color::srgb(0.24, 0.25, 0.28);

/// The colour of the `i`-th slab under a zone pile, counted from 1 as
/// [`slab_transform`] counts them: each side's slabs alternate between the
/// back and [`SLAB_EDGE_COLOR`], so each stands out from the one before it on
/// its side as well as from the felt.
fn slab_color(i: usize) -> Color {
    if i.div_ceil(2) % 2 == 1 {
        SLAB_EDGE_COLOR
    } else {
        BACK_COLOR
    }
}
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
const HOVER_LIFT: f32 = 0.04;
const HOVER_SCALE: f32 = 1.06;
/// Lift and scale for a card chosen for the pending choice (clearly "in").
const SELECTED_LIFT: f32 = 0.08;
const SELECTED_SCALE: f32 = 1.12;

/// The steepest shot the camera ever takes.
///
/// Every bound below divides by the lean, so each of them is hardest to
/// satisfy at the steepest one — and since a wide duel blends towards
/// [`DUEL_LEAN`], checking them at [`CAMERA_LEAN`] would be checking them at
/// the shot a **desktop duel does not use**. They were written when the lean
/// was one number; naming the maximum is what keeps them about the camera
/// rather than about a constant that used to be the whole answer.
const STEEPEST_LEAN: f32 = if DUEL_LEAN > CAMERA_LEAN {
    DUEL_LEAN
} else {
    CAMERA_LEAN
};

/// The most a card may rise, for a given growth, without any part of the
/// footprint it started with leaving the pointer.
///
/// `CARD_WIDTH` rather than `CARD_HEIGHT` because the shift is in *world*
/// space — always straight away from the viewer — while a pod is rotated to
/// face its own seat, so a card's narrow dimension is the one the shift can
/// end up aligned with. Taking the smaller of the two is what makes the bound
/// hold at every seat instead of only at the near one.
const fn covered_lift(scale: f32) -> f32 {
    (CARD_WIDTH / 2.0) * (scale - 1.0) / STEEPEST_LEAN
}
// At `DUEL_LEAN` the cap is exactly `scale - 1`, because a card is one unit
// wide and 0.5 is precisely the lean at which a rise stops being covered by
// its growth. The shipped lifts sat *on* that line — 0.06 against a cap of
// 0.06, 0.12 against 0.12 — which is not a margin, and in `f32` the first of
// them landed on the wrong side of it by one part in a million. So the lifts
// came down a second time, to a fifth of the growth in hand: what a steeper
// shot has to buy is headroom, not another equality.
const _: () = assert!(HOVER_LIFT <= covered_lift(HOVER_SCALE));
const _: () = assert!(SELECTED_LIFT <= covered_lift(SELECTED_SCALE));
// Hover to selected is a rise as well, so the step between them is bound by
// the growth between them and not by either pair on its own.
const _: () = assert!(
    (SELECTED_LIFT - HOVER_LIFT) * STEEPEST_LEAN
        <= (CARD_WIDTH / 2.0) * (SELECTED_SCALE - HOVER_SCALE)
);

// A creature with flying stands higher than any of those, and that is not a
// breach of the bound above — it is a different bound. `covered_lift` is about
// a rise the *pointer* causes: a card that grows under the pointer and rises
// further than it grows slides out from under it, and the flicker that starts
// is one the card is feeding itself. A flier's height is caused by the card
// and not by the pointer, so there is no loop to close; what it has to respect
// is only that its own movement is too small to leave a pointer that is
// sitting still, which is `airborne::SWAY` and is a fiftieth of a card's
// width. The renderer holds it still under the pointer as well — see
// `sync_scene` — so this is the belt to that pair of braces.
const _: () = assert!(airborne::SWAY * STEEPEST_LEAN < CARD_WIDTH / 20.0);
// And a card the player has chosen must never stand higher than a creature in
// the air: two claims about height that mean different things must not be able
// to trade places.
const _: () = assert!(SELECTED_LIFT < airborne::RESTING - airborne::SWAY);

/// The height past which a flier's contact shadow stops spreading.
///
/// The spread is what says "up there", and the shadow texture is shared by
/// every card in the game — so it can be moved and scaled and never dimmed,
/// and past the top of the bob a wider one stops reading as a higher card and
/// starts reading as a heavier one. The cap is that top, which means a
/// *hovered* flier goes on climbing away from a shadow that has stopped
/// growing. That is the right way round: the extra height there is the
/// pointer's claim about the card, not the card's claim about itself.
const FLOAT_SHADOW_CAP: f32 = airborne::RESTING + airborne::SWAY;

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
    /// The firewheel's five eased strengths, packed the way the shader
    /// reads them: `flames` is white, blue, black, red and `tail.x` green.
    ///
    /// Eased for the same reason the lamp is, and on the same clock: a
    /// flame that grows over a second and a half is a fire being fed, and
    /// one that jumps when a land enters play is a notification.
    flames: Vec4,
    tail: Vec4,
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
    /// Distance from the target.
    ///
    /// Written by the framing and by nothing a hand does: there is no zoom
    /// control any more, so this is an output of [`CameraRig::home`] that a
    /// pan carries along rather than a setting.
    pub distance: f32,
    /// Azimuth around the target (0 = behind the local seat).
    pub yaw: f32,
    /// How far the camera stands off vertical, as a tangent.
    ///
    /// Every shot the client takes is [`CAMERA_LEAN`] and no control changes
    /// it: the lean is a measured trade between a card reading as an object
    /// and the far seat's cards shrinking (that constant carries the
    /// numbers), which is a few degrees wide and nothing a hand aims. It
    /// stays a field rather than moving into the transform because the
    /// framing arithmetic reads it off the rig it is solving for. A tangent
    /// and not an angle, because that is what the transform and the shadow
    /// offsets read.
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
    /// As close as the framing may pull the camera in: one seat's lane,
    /// filling the screen.
    pub const MIN_DISTANCE: f32 = 12.0;
    /// As far as the framing may push it out.
    ///
    /// It was a limit on the *player's* zoom and is now the only limit there
    /// is: the zoom went at the owner's word (*„Das Zoom in/out sollte eh
    /// weg!"*), so the pair bounds [`CameraRig::home`] and nothing else —
    /// which is why it has headroom over the furthest table there is. Since
    /// every seat at a ring is handed a duel's board (#264), that is eight
    /// seats: 186 units on a laptop, 223 on a phone held upright and 273 on
    /// one turned on its side, where the furthest table used to ask for 81.
    /// A limit sitting just above that would not stop the shot: it would
    /// silently crop it, because a fit refused is a fit that no longer fits.
    ///
    /// Both ends are distances through [`FOV`], so both moved when it did:
    /// the same shot through half the angle stands twice as far off, and a
    /// pair left where they were would have clamped every table on the way
    /// in and every large one on the way out.
    pub const MAX_DISTANCE: f32 = 300.0;

    // `MIN_LEAN` (0.176, about 10° off plan) and `MAX_LEAN` (1.428, about
    // 55°) stood here and are gone with the tilt control they bounded. Their
    // reasons were real and are now [`CAMERA_LEAN`]'s to carry, because the
    // lean is one number the client picks rather than a range a hand moves
    // through: a card is a slab with a wall and a contact shadow and reads
    // as a decal from straight overhead, and a seat looking along its own
    // board sees the backs of its front row.

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
    /// the hand zone on every screen — a player could not see their own
    /// creatures, which made every later piece of board legibility moot.
    #[must_use]
    pub fn home(layout: &TableLayout, canvas: Canvas) -> Self {
        let Some((min, max)) = layout.extent() else {
            return Self::default();
        };
        // A wide duel can show the table's depth without foreshortening side
        // seats. Blend in as the window grows; rings and small windows keep
        // the readable plan view and its breathing room.
        let framing = if layout.slots.len() == 2 && canvas.aspect() >= 1.4 {
            ((canvas.window.x - 800.0) / 480.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let tilt = CAMERA_LEAN + (DUEL_LEAN - CAMERA_LEAN) * framing;
        let air = AIR - 1.95 * framing;
        let (min, max) = (min - Vec2::splat(air), max + Vec2::splat(air));
        let span = max - min;

        // The free band, as normalised device coordinates: +1 is the top of
        // the window, and the tab strip and the hand zone eat inwards.
        let top = 1.0 - 2.0 * canvas.top / canvas.window.y.max(1.0);
        let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y.max(1.0);
        let right = 1.0 - 2.0 * canvas.right / canvas.window.x.max(1.0);
        let aspect = canvas.window.x / canvas.window.y.max(1.0);

        // Vertically this is exact: `ground` is linear in the eye distance,
        // so the distance at which the table's far edge lands on `top` and
        // its near edge on `bottom` is one division.
        let g_top = ground(top, tilt);
        let g_bottom = ground(bottom, tilt);
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
        let k = tilt / (1.0 + tilt * tilt).sqrt();
        let scale = (half_fov().tan() * aspect).max(1e-3);
        let carry = k.mul_add(mean, 1.0);
        let corners = layout.corners(air);
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
        let lean = (1.0 + tilt * tilt).sqrt();
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
        // is a mat nobody can see, and one under the hand zone is one the
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
            lean: tilt,
        }
    }
}

/// How much bare felt is left around the table when it is framed.
///
/// Two things live in this number. The first is not optional: what the layout
/// reports is the box the *cards* stand in, and a seat's mat is drawn
/// [`ZONE_MARGIN`] wider than that on every side — so a shot framed on the
/// reported box crops the mat's own printed border, and at the near edge it
/// crops it under the hand zone.
///
/// The rest is the felt itself. A table framed to the last pixel of the band
/// reads as a photograph someone cropped too tightly, whatever the arithmetic
/// says about it fitting; leaving a couple of units of table showing all round
/// is what makes it look like a table being played at rather than a diagram
/// being displayed. It is also roughly where [`GLOW_SPREAD`] fades out, so the
/// halo under an active seat's mat stays in frame with it.
///
/// Wide duels bring this down to three units, keeping the leather rail and
/// a little sky visible. Smaller windows and rings retain the full margin
/// to preserve their readability. Everything past [`SLAB_MARGIN`] is sky.
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
/// `L`, `C = 1/√(1+L²)`, the camera-space depth and height
/// of that point work out to
///
/// ```text
///     depth(s) = D + L·C·s          height(s) = C·s
/// ```
///
/// — the cross terms cancel, which is the whole reason this is arithmetic and
/// not a projection matrix. So `q = height / (depth · tan(fov/2))`, and solved
/// the other way `s = D · ground(q, L)`. Being *linear in `D`* is what lets
/// [`CameraRig::home`] invert it with a division instead of a search.
fn ground(q: f32, lean: f32) -> f32 {
    let t = half_fov().tan();
    let c = 1.0 / (1.0 + lean * lean).sqrt();
    q * t / (c * q.mul_add(-lean * t, 1.0))
}

/// The part of the window the table is actually seen through.
///
/// The HUD is not beside the battlefield, it is on top of it: the hand zone is
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
    /// Covered at the bottom: the hand zone.
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
            bottom: crate::hud::HAND_ZONE_H,
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

/// Tells the board model what the answer being built now proposes.
///
/// A proposal is part of what a stack is merged on
/// (`baylee_client_core::board::Proposal`): three Soldiers declared out of
/// twelve are a card of their own, and so is the Forest a plan would tap
/// out of five. The answer changes through a click, a key, `Esc`, a row in
/// the prompt bar, an armed spell — more doors than
/// [`crate::rebuild_board`] has callers — so the change is noticed here,
/// once a frame, the way [`track_canvas`] notices the window.
pub fn track_proposals(mut duel: ResMut<Duel>) {
    if crate::proposals(&duel) != duel.proposed {
        crate::rebuild_board(&mut duel);
    }
}

/// Keeps the table framed as seats, focus and window size change.
///
/// Whether it may is [`Duel::camera_held`], and that used to be a float
/// comparison: this system kept the framing it had last computed and followed
/// the table only while the rig still *equalled* it. Anything that moved the
/// rig by any amount at all — one pixel of the left-drag orbit that has since
/// been deleted, during an ordinary click on a card — switched the automatic
/// framing off for the rest of the session, silently, and the table then
/// stayed wherever the accident left it through a resize and through a seat
/// joining. Nothing said so and nothing could put it back except a key nobody
/// knew to press.
///
/// Two things put the camera back in the table's hands, and both are about a
/// table rather than about a rig. The **seat count** changing is a different
/// table, so a player aiming at the old one is not aiming at this one. And
/// [`CameraRig::default`] is the one rig that means "nobody aimed this" —
/// [`crate::input::navigate_home`] asks for exactly it, which is how a key, a
/// tab and anything else that cannot reach the flag still comes home.
pub fn frame_table(
    mut duel: ResMut<Duel>,
    windows: Query<&Window>,
    mut seats: Local<usize>,
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
    if *seats != layout.slots.len() {
        *seats = layout.slots.len();
        duel.camera_held = false;
    }
    if *rig == CameraRig::default() {
        duel.camera_held = false;
    }
    if duel.camera_held {
        return;
    }
    if *rig != next {
        *rig = next;
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

/// About 32° off vertical for a wide duel; rings retain [`CAMERA_LEAN`].
const DUEL_LEAN: f32 = 0.62;

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
/// *input*: a drag writes it, the framing writes it, focusing a seat writes it,
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

    /// The window this projects onto, in logical pixels.
    #[must_use]
    pub const fn window(&self) -> Vec2 {
        self.window
    }

    /// Where a point on the felt is drawn, or `None` if it is behind the eye.
    #[must_use]
    pub fn project(&self, table: Vec2) -> Option<Vec2> {
        self.project_world(to_world(table, TABLE_Y))
    }

    /// Where a point in the world is drawn, or `None` if it is behind the eye.
    ///
    /// The felt is the common case and [`Self::project`] is the name for it.
    /// This is for what is not a point on the felt: a card is a quad standing
    /// `CARD_LIFT` above it and turned by its own rotation, and its corners
    /// are world points with no table-space spelling. The lift itself turns
    /// out to be worth almost nothing on screen — 0.14 px at a duel, measured
    /// while writing `devctl::card_rect` — because the camera is near enough
    /// overhead that raising something a hundredth of a card barely moves it.
    /// It is the rotation that this is needed for.
    #[must_use]
    pub fn project_world(&self, world: Vec3) -> Option<Vec2> {
        let clip = self.clip_from_world * world.extend(1.0);
        if clip.w <= 1e-4 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new(
            ndc.x.mul_add(0.5, 0.5) * self.window.x,
            0.5f32.mul_add(-ndc.y, 0.5) * self.window.y,
        ))
    }

    /// Intersects a screen point with a horizontal plane in the table scene.
    pub(crate) fn on_plane(&self, screen: Vec2, height: f32) -> Option<Vec3> {
        let ndc = Vec2::new(
            screen.x / self.window.x * 2.0 - 1.0,
            1.0 - screen.y / self.window.y * 2.0,
        );
        // Double precision avoids cancellation between the near/far ray points.
        let inverse = self.clip_from_world.as_dmat4().inverse();
        let ndc = ndc.as_dvec2();
        let near = inverse.project_point3(ndc.extend(-1.0));
        let far = inverse.project_point3(ndc.extend(1.0));
        let direction = far - near;
        if direction.y.abs() < 1e-6 {
            return None;
        }
        let t = (f64::from(height) - near.y) / direction.y;
        (t >= 0.0).then_some((near + direction * t).as_vec3())
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

/// The screen box one card covers, as its centre and its size in logical
/// pixels.
///
/// The centre is the card's own projected origin rather than the middle of
/// the box: under perspective those differ, and the one worth clicking is the
/// card's.
///
/// It lives here rather than in [`crate::devctl`], where it was written, for
/// the reason the ability sheet needs it: a sheet anchored to a permanent has
/// to be put where that permanent is *drawn*, and the harness had already
/// worked out that a card is four corners turned by its own transform and not
/// an axis-aligned rectangle around its origin.
#[must_use]
pub fn card_box(lens: &Lens, at: &Transform) -> Option<(Vec2, Vec2)> {
    let half = Vec2::new(
        baylee_client_core::layout::CARD_WIDTH,
        baylee_client_core::layout::CARD_HEIGHT,
    ) / 2.0;
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for corner in [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        Vec2::new(half.x, half.y),
        Vec2::new(-half.x, half.y),
    ] {
        let drawn = lens.project_world(at.transform_point(corner.extend(0.0)))?;
        min = min.min(drawn);
        max = max.max(drawn);
    }
    Some((lens.project_world(at.translation)?, max - min))
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

/// Where a card stands when nothing is touching it.
///
/// [`sync_scene`] builds a card's pose in two stages — its place on the felt,
/// and then what the pointer or an armed deed is doing to it ([`HOVER_LIFT`]
/// with [`HOVER_SCALE`], [`SELECTED_LIFT`] with [`SELECTED_SCALE`]) — and this
/// is the first stage kept on its own.
///
/// It exists because something finally had to **point** at a card rather than
/// be one. The ability sheet stands beside a permanent for as long as a player
/// is reading it, and anchored to the live pose it was dragged about by the
/// [`HOVER_LIFT`] and [`HOVER_SCALE`] the pointer applies to a card: a sheet
/// that jumped whenever the hand moved across the thing it was describing.
///
/// Nothing *draws* from it, which is what keeps it from being a second opinion
/// about where a card is — [`Motion::target`] is still the only one.
#[derive(Component)]
pub struct CardRest(pub Transform);

/// Everything [`sync_scene`] writes on a card that is already on the table.
///
/// Named because it is five terms long and appears in three places in that
/// one function, not because it is a concept: a card on the felt is its pose,
/// what it stands for, what it is made of, whether it is in the air, and
/// where it would be with nothing touching it.
type DrawnCard = (
    &'static mut Motion,
    &'static mut CardVisual,
    &'static mut MeshMaterial3d<CardMaterial>,
    Has<Floating>,
    &'static mut CardRest,
    &'static mut Visibility,
);

/// A card's visibility: its parent's while it is shown, none while a
/// scrolled row leaves it out.
fn shown_as(shown: bool) -> Visibility {
    if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

/// A card that is off the felt because what it stands for has flying.
///
/// A marker and nothing more: the height is written into [`Motion::target`]
/// by [`sync_scene`] like every other reason a card is where it is, so there
/// is no second number here that could disagree with it. What the marker is
/// for is the *shadow* — [`ground_the_shadows`] asks which cards' shadows
/// have to be left behind on the table, and this is the answer.
#[derive(Component)]
pub struct Floating;

/// The contact shadow under a card, as a child of that card.
///
/// It was unmarked while it was only ever set once at spawn. It has to be
/// found again now, because a card that leaves the ground has to leave it
/// behind.
#[derive(Component)]
pub struct CardShadow;

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
    mut cards: Query<(&Motion, &mut Transform, Option<&crate::combatfx::Recoil>)>,
) {
    let still = prefs.all().reduce_motion;
    let t = 1.0 - (-SETTLE * time.delta_secs()).exp();
    for (motion, mut transform, recoil) in &mut cards {
        let mut target = motion.target;
        if !still && let Some(recoil) = recoil {
            target.translation += recoil.offset(time.elapsed_secs());
        }
        let there = transform.translation.distance_squared(target.translation) < SETTLED * SETTLED
            && transform.rotation.angle_between(target.rotation) < SETTLED
            && transform.scale.distance_squared(target.scale) < SETTLED * SETTLED;
        if still || there {
            if *transform != target {
                *transform = target;
            }
            continue;
        }
        transform.translation = transform.translation.lerp(target.translation, t);
        transform.rotation = transform.rotation.slerp(target.rotation, t);
        transform.scale = transform.scale.lerp(target.scale, t);
    }
}

/// Only live cards own grounded shadows; departing cards keep their exit pose.
/// The exclusion makes the card and shadow transform queries disjoint.
type ShadowOwners = (With<CardVisual>, Without<CardShadow>);

/// The materials of the objects lying on and round a card: its strip, its
/// count badge, the offer's light on the felt round it and its shell.
type CardCompanions<'w> = (
    ResMut<'w, Assets<MarksMaterial>>,
    ResMut<'w, Assets<BadgeMaterial>>,
    ResMut<'w, Assets<FloorMaterial>>,
    ResMut<'w, Assets<ShellMaterial>>,
);

/// Ground flying shadows using the card's live pose, preserving its tapped
/// heading while removing its bank. Spread grows with height up to a fixed cap.
/// Remember the original child pose so losing flying restores a contact shadow;
/// ordinary cards and pile fans retain their existing shadows.
pub fn ground_the_shadows(
    cards: Query<(&Transform, Has<Floating>), ShadowOwners>,
    mut shadows: Query<(Entity, &ChildOf, &mut Transform), With<CardShadow>>,
    mut resting: Local<HashMap<Entity, Transform>>,
) {
    resting.retain(|entity, _| shadows.get(*entity).is_ok());
    for (entity, parent, mut at) in &mut shadows {
        let Ok((card, flying)) = cards.get(parent.parent()) else {
            resting.remove(&entity);
            continue;
        };
        if !flying {
            if let Some(original) = resting.remove(&entity) {
                *at = original;
            }
            continue;
        }
        resting.entry(entity).or_insert(*at);
        // How far off the felt the card itself is. `CARD_LIFT` is the hair
        // every card is given so it does not z-fight the cloth, so a card
        // lying down measures zero here and its shadow keeps the placement it
        // was spawned with.
        let height = (card.translation.y - TABLE_Y - CARD_LIFT).max(0.0);
        let spread = 1.0 + height.min(FLOAT_SHADOW_CAP) * DECK_SHADOW_SPREAD;
        // Undo the parent bank so the shadow remains flat on the table.
        let ground = Vec3::new(
            card.translation.x + height * 0.12,
            TABLE_Y + CARD_LIFT * 0.5,
            card.translation.z + height * 0.18,
        );
        let right = card.rotation * Vec3::X;
        let yaw = (-right.z).atan2(right.x);
        let wanted = Transform {
            translation: card.to_matrix().inverse().transform_point3(ground),
            rotation: card.rotation.inverse()
                * Quat::from_rotation_y(yaw)
                * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            scale: Vec3::new(spread, spread, 1.0),
        };
        if *at != wanted {
            *at = wanted;
        }
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

/// A pile that is a *place* rather than a card — which is to say a library.
///
/// Every other pile is drawn through its top card, and a top card is an object
/// wearing a [`CardVisual`], so the pointer finding it is the ordinary hover
/// that finds any permanent. A library has no top card at all: it is face down
/// to everyone, its owner included (CR 401.2), and what stands there is a
/// stack of blank slabs. This is what lets the pointer find *those* — carried
/// by the topmost slab and by the backs a hover fans out of it, because the
/// rest of the deck is depth rather than cards.
#[derive(Component, Clone, Copy)]
pub struct PileVisual {
    /// Whose pile.
    pub player: PlayerId,
    /// Which pile.
    pub kind: PileKind,
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

/// The text a card's face is showing on the table, and what it was made
/// from ([`SceneIndex::faces`]).
struct ShownFace {
    /// The snapshot it was built from.
    seq: u64,
    /// Whether its lines were fitted by the font's widths and not the
    /// average's ([`face::Widths`]).
    measured: bool,
    /// How many lines its name took: its name bar's depth, which its
    /// material draws.
    lines: usize,
    /// The `Text2d` children.
    texts: Vec<Entity>,
}

impl ShownFace {
    /// Whether it is still the face to show: built from this snapshot, and
    /// measured the way the table can measure now.
    fn is(&self, seq: u64, measured: bool) -> bool {
        self.seq == seq && self.measured == measured
    }
}

/// What a card's text face is this frame (#259).
enum FaceNow {
    /// The face on the card is still the one to show; its name is on this
    /// many lines.
    Kept(usize),
    /// A face fitted afresh: the old one's text comes off and this one's
    /// goes on.
    Fitted(Box<(CardFace, face::WorldFit)>),
    /// None shown, and nothing yet to build one from.
    Unbuilt,
}

impl FaceNow {
    /// How many lines its name takes: its name bar's depth.
    fn lines(&self) -> usize {
        match self {
            Self::Kept(lines) => *lines,
            Self::Fitted(fitted) => fitted.1.lines(),
            Self::Unbuilt => 1,
        }
    }
}

/// Whether the face `shown` on a card is still the one to show, and if not,
/// the one to put there instead, built by `build` and fitted by `widths`.
fn face_now(
    shown: Option<&ShownFace>,
    seq: u64,
    widths: &face::Widths<'_>,
    build: impl FnOnce() -> Option<CardFace>,
) -> FaceNow {
    if let Some(shown) = shown.filter(|shown| shown.is(seq, widths.measured())) {
        return FaceNow::Kept(shown.lines);
    }
    build().map_or(FaceNow::Unbuilt, |built| {
        let fit = face::WorldFit::of(&built, widths);
        FaceNow::Fitted(Box::new((built, fit)))
    })
}

/// The look of a card standing its text face in the window: its colours,
/// what it is, and how deep its name bar is ([`textface::face_word`]), over
/// the flat colour its identity used to be drawn in.
fn face_look(
    object: Option<&baylee_view::PublicObject>,
    lines: usize,
    finish: FinishTreatment,
) -> CardLook {
    let colors = object.map_or(ColorSet::EMPTY, |o| o.colors);
    CardLook::flat(face::table_color(colors), finish).with_face(face_word(object, lines))
}

/// The face word of `object` with its name on `lines` lines: what the
/// material draws, and what the text standing on it is inked for.
fn face_word(object: Option<&baylee_view::PublicObject>, lines: usize) -> u32 {
    textface::face_word(
        object.map_or(ColorSet::EMPTY, |o| o.colors),
        object.map_or(TypeSet::EMPTY, |o| o.types),
        object.map_or(SubtypeSet::EMPTY, |o| o.subtypes),
        textface::Depths::table(lines),
    )
}

/// Entities currently drawn, keyed by the object they represent.
#[derive(Resource, Default)]
pub struct SceneIndex {
    cards: HashMap<ObjectId, Entity>,
    /// One material per *look*, shared by every card wearing it — a board of
    /// forty plain Islands is one material, not forty. A foil Island is a
    /// second: that is a difference the shader draws, and since #298 the
    /// print, its finish and the light passing over it are all it draws.
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
    faces: HashMap<ObjectId, ShownFace>,
    /// The strip lying on each card with something to say (#274, #298): what
    /// it says and the row step it was put on for, and the strip itself.
    ///
    /// Held here for the reason [`Self::faces`] is: a strip comes and goes
    /// with what the rules do to the card, and has to be taken off as cheaply
    /// as it was put on.
    marks: HashMap<ObjectId, (cardrail::Strip, f32, Entity)>,
    /// One strip material per thing a strip says, shared by every card
    /// saying it — a lane of twelve Soldiers with the same keywords is one.
    marks_materials: HashMap<cardrail::Strip, Handle<MarksMaterial>>,
    /// The quad every strip is drawn on: [`cardrail::quad_rect`], one mesh
    /// for the whole table, since the shader sizes the strip inside it.
    marks_quad: Option<Handle<Mesh>>,
    /// The count badge at each merged card's corner (#261): what it was put
    /// on for (the count, the row step, where it stands and whether its card
    /// is tapped) and the badge itself. Held for the strip's reason.
    badges: HashMap<ObjectId, (BadgeKey, Entity)>,
    /// One badge material per count and place, shared by every card saying
    /// it.
    badge_materials: HashMap<(u32, BadgePlace), Handle<BadgeMaterial>>,
    /// The quad every badge is drawn on: [`cardplate::badge_quad_rect`], one
    /// mesh for the whole table, since the shader sizes the body inside it.
    badge_quad: Option<Handle<Mesh>>,
    /// The offer's light on the felt under each card this client is offering
    /// something for (#298): the offers and the depth it was put at, and the
    /// light itself. Held for the strip's reason, and it comes and goes with
    /// priority, which is far more often.
    floors: HashMap<ObjectId, (u32, f32, Entity)>,
    /// One light material per combination of offers: at most sixteen.
    floor_materials: HashMap<u32, Handle<FloorMaterial>>,
    /// The quad every light is drawn on: [`floormat::quad_size`], one mesh
    /// for the whole table.
    floor_quad: Option<Handle<Mesh>>,
    /// The shells round each protected permanent: indestructible's steel
    /// and hexproof's or shroud's dome, each with the ring it lies down to,
    /// all children of the card, and how each stands. [`fit_the_shells`]
    /// decides that every frame, from where every card is.
    shells: HashMap<ObjectId, Shell>,
    /// One material per look of shell.
    shell_materials: HashMap<ShellLook, Handle<ShellMaterial>>,
    /// The rim's mesh, one for the whole table.
    rim_mesh: Option<Handle<Mesh>>,
    /// A ring's mesh for each part of its band.
    ring_meshes: HashMap<Band, Handle<Mesh>>,
    /// Each dome's mesh at each of its steps.
    dome_meshes: HashMap<(shellmat::Dome, usize), Handle<Mesh>>,
    /// Defender's wall, one for the whole table.
    wall_mesh: Option<Handle<Mesh>>,
    /// What stands under each card on the table: the slabs of its deck and
    /// its contact shadow, with the count they were built for (#261). Held for the strip's reason, and because a group grows under
    /// the same top card: a deck built once at spawn kept one slab under a
    /// card that had risen to stand on eleven.
    stacks: HashMap<ObjectId, Stack>,
    /// What stood in a pile's hover fan on the **previous** frame, and which
    /// pile each card came out of.
    ///
    /// Previous and not current, which is the whole reason it is kept at all.
    /// A fan closes by leaving the board model, so the frame that has to send
    /// its cards home is a frame on which they are already gone — and a card
    /// that leaves with no zone change behind it is otherwise despawned where
    /// it stands, which for seven cards in the air is seven cards blinking
    /// out of it.
    fanned: HashMap<ObjectId, Place>,
    /// Whose library is standing open.
    ///
    /// A library's fan is blank slabs rather than placements — see
    /// [`sync_library_fan`] — so it is the one part of the scene that is not
    /// keyed by `ObjectId` and has to be remembered by hand.
    library_fan: Option<PlayerId>,
    /// The slabs of that fan, in order from the top of the deck.
    library_fan_cards: Vec<Entity>,
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    /// Whether nobody is sitting in this chair — the house is answering for a
    /// player who has gone ([`SeatRole::Away`]).
    ///
    /// On the `Mood` and not passed beside it, which is the whole reason it
    /// is here: `sync_zones` skips a seat whose `mood` and `accent` are both
    /// unchanged, so a flag outside this struct would be written once when
    /// the mat was built and never again. A chair handed to the house
    /// mid-game would keep a solid rim until something else about the seat
    /// happened to move.
    held: bool,
}

/// What a seat is doing, in the order the zone cares about it.
///
/// Ordered rather than flagged because these do not stack: the seat being
/// asked is *also* the active seat nine times out of ten, and drawing both
/// would only mean adding two brightnesses together and hoping.
///
/// `Asked` rather than `SeatPod::is_awaited`'s own word, although the two are
/// fed from one field. A pod mirrors its feed and this names a rung, and the
/// rung sits one line from `Waiting` — which means the opposite and would be
/// a word away from it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Standing {
    /// Out of the game.
    Lost,
    /// The seat the engine is waiting on an answer from — a priority pass,
    /// but equally a block to declare or a card to discard.
    Asked,
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
            } else if pod.is_awaited {
                Standing::Asked
            } else if pod.is_active {
                Standing::Active
            } else {
                Standing::Waiting
            },
            // A seat that is out of the game is not taking a turn, whatever
            // the view last said about the active player.
            on_turn: pod.is_active && !pod.has_lost,
            // `Away` and not `answered_by_the_house`: an AI chair was always
            // an AI chair and there is nothing provisional about it, while
            // this one is a player's seat being covered until they come back.
            held: pod.role == baylee_client_core::board::SeatRole::Away,
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
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
) {
    index.quad = Some(meshes.add(rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER)));
    let strip = crate::marksmat::quad_size();
    index.marks_quad = Some(meshes.add(Rectangle::new(
        strip.x * CARD_WIDTH,
        strip.y * DOWN_THE_CARD,
    )));
    let badge = crate::badgemat::quad_size();
    index.badge_quad = Some(meshes.add(Rectangle::new(
        badge.x * CARD_WIDTH,
        badge.y * DOWN_THE_CARD,
    )));
    let floor = floormat::quad_size();
    index.floor_quad = Some(meshes.add(Rectangle::new(
        floor.x * CARD_WIDTH,
        floor.y * DOWN_THE_CARD,
    )));
    let [inner, outer] = shellmat::RIM_EDGES;
    index.rim_mesh = Some(meshes.add(shellmat::band_mesh(inner, outer)));
    for band in Band::ALL {
        let [inner, outer] = band.edges();
        index
            .ring_meshes
            .insert(band, meshes.add(shellmat::band_mesh(inner, outer)));
    }
    for dome in shellmat::Dome::ALL {
        for (step, share) in shellmat::DOME_STEPS.into_iter().enumerate() {
            index.dome_meshes.insert(
                (dome, step),
                meshes.add(shellmat::dome_mesh(dome.row(), share)),
            );
        }
    }
    index.wall_mesh = Some(meshes.add(shellmat::wall_mesh()));

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
        // A card's shadow is the highest thing the table's ground carries,
        // and it is what the air is kept underneath.
        depth_bias: sort_bias(CARD_LIFT * 0.5),
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
    // library is drawn as, slab by slab (CR 401.2: face down), and what a
    // card this seat may not see wears. The slabs under a card on the table
    // are not backs but frames (`sync_stack`). The printed back is
    // fetched like any other image, so `sync_scene` dresses this material in
    // it the frame it lands; until then it is the flat colour below.
    // A second duel in one session gets a second material, and the flag is
    // about *this* one: left standing, the new material would never be
    // dressed and every hidden card would go back to being a dark rectangle.
    index.back_dressed = false;
    index.blank = Some(cards.add(material(
        CardLook::flat(BACK_COLOR, FinishTreatment::Plain),
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
        // The same answer the lobby's camera gets, from the same place:
        // `Msaa` is a component in bevy 0.19, so a driver workaround has to
        // be repeated on every camera rather than set once. Both need it
        // rather than only the one the defect was caught on — it is in the
        // tiler's multisample resolve, not in anything the lobby does.
        crate::gpu::msaa(adapter.as_deref()),
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

    // Nothing is spawned for the middle of the table either. The colour
    // wheel used to be a quad here with a 512-texel medallion on it — five
    // soft discs of the pie on two rings of worn gold — and it is five
    // flames painted into `felt.wgsl` now, because the light a fire throws
    // has to be *added to the cloth* to have the weave show through it, and
    // a quad blended over the felt can only ever cover it.
    // `baylee_client_core::firewheel` is normative.
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
    // A mat is stretched over a quad much larger than the texture is;
    // without a linear filter its soft edges come out as stairs.
    image.sampler = bevy::image::ImageSampler::linear();
    image
}

/// The colour a seat's zone is drawn in.
///
/// The viewing seat is gilt, matching the firewheel's rings: whatever else is
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
    /// draws over what, through [`sort_bias`] — which is newer than this
    /// comment, and is what finally made the sentence true.
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
        depth_bias: sort_bias(quad.lift),
        ..default()
    });
    let entity = commands
        .spawn((
            DuelStage,
            Mesh3d(meshes.add(Rectangle::new(quad.size.x, quad.size.y))),
            MeshMaterial3d(material.clone()),
            // Ground and glow are both scenery. Only cards are
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
            let mut slab = commands.spawn((
                DuelStage,
                Mesh3d(quad.clone()),
                MeshMaterial3d(index.blank.clone().unwrap_or_default()),
                card_transform(slot, at, false, lift),
            ));
            // The deck under the top slab is depth, not cards, so it is as
            // unpickable as the backing under a graveyard's top card. The
            // slab on top is the one thing here a pointer can mean, and it
            // has to be findable: a library is the one pile with no top card
            // to hover, and the fan it opens is drawn out of *this*.
            if i + 1 == layers {
                slab.insert(PileVisual {
                    player: slot.player,
                    kind: baylee_client_core::PileKind::Library,
                });
            } else {
                slab.insert(Pickable::IGNORE);
            }
            out.push(slab.id());
        }
    }
    out
}

/// The backs a hover spreads out of a library.
///
/// A system of its own, and it is the one place the fan is not a placement.
/// Every other pile fans *objects* — cards with an `ObjectId`, which
/// `sync_scene` already knows how to cache, glide, light and pick — and a
/// library has none: it is face down to everybody, its owner included
/// (CR 401.2), so `PlayerView` carries it as a count and `ZonePile::fan` is
/// empty there by construction. What is drawn here is therefore blank slabs,
/// as many as [`ZonePile::fan_len`] says, wearing the one material every
/// hidden card in the game wears.
///
/// It is the same *shape* as the fan beside it — the same poses from the same
/// [`SeatSlot::fan_pose`] — which is the point: a library that answered a
/// hover differently from a graveyard would teach a player that some piles
/// are worth pointing at and leave them guessing which. What it says is the
/// count, up to seven, which is the one thing the pile's own thickness cannot
/// say once it has reached its cap.
pub fn sync_library_fan(mut commands: Commands, duel: Res<Duel>, mut index: ResMut<SceneIndex>) {
    let wanted = duel
        .hovered_pile
        .filter(|(_, kind)| *kind == baylee_client_core::PileKind::Library)
        .and_then(|(player, kind)| {
            let board = duel.board.as_ref()?;
            let slot = duel.layout.as_ref()?.slot(player)?;
            let pile = board
                .pod(player)?
                .piles
                .iter()
                .find(|pile| pile.kind == kind)?;
            (pile.fan_len() > 0).then(|| (player, *slot, pile.fan_len()))
        });

    // Nothing to draw, or a different pile than the one standing open: the
    // backs go home the way a fanned card does, on the pile-bound exit with
    // no door on it. `retire` despawns them once the glide has had its time.
    if index.library_fan != wanted.map(|(player, _, _)| player) {
        for entity in std::mem::take(&mut index.library_fan_cards) {
            if let Ok(mut card) = commands.get_entity(entity) {
                card.remove::<PileVisual>()
                    .insert((Pickable::IGNORE, Departing { left: EXIT_LIFE }));
            }
        }
        index.library_fan = None;
    }

    let Some((player, slot, len)) = wanted else {
        return;
    };
    let (Some(quad), Some(blank)) = (index.quad.clone(), index.blank.clone()) else {
        return;
    };

    // Born on the pile and gliding out of it, so the fan grows rather than
    // appearing — `glide` does the moving, as it does for everything else on
    // this table, and a player who has turned motion off gets the pose on the
    // first frame.
    if index.library_fan.is_none() {
        let home = card_transform(
            &slot,
            slot.pile_center(baylee_client_core::PileKind::Library),
            false,
            stack_rise(len),
        );
        index.library_fan_cards = (0..len)
            .map(|i| {
                let pose = slot.fan_pose(baylee_client_core::PileKind::Library, i, len, false);
                let mut at = card_transform(&slot, pose.at, false, pose.lift);
                at.rotation = fan_rotation(&slot, pose);
                commands
                    .spawn((
                        DuelStage,
                        Mesh3d(quad.clone()),
                        MeshMaterial3d(blank.clone()),
                        home,
                        Motion { target: at },
                        // Pickable, and wearing the pile it came out of: once
                        // the fan is open the pointer is on a back and no
                        // longer on the library, and a fan that only the
                        // library itself held open would shut on the first
                        // pixel of travel.
                        PileVisual {
                            player,
                            kind: baylee_client_core::PileKind::Library,
                        },
                    ))
                    .id()
            })
            .collect();
        index.library_fan = Some(player);
    }
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
        awaited: if mood.standing == Standing::Asked {
            1.0
        } else {
            0.0
        },
        held: if mood.held { 1.0 } else { 0.0 },
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
/// and its zone should stop competing for attention — and the seat being
/// asked is the brightest thing on the felt, because that is the seat
/// everyone else is waiting for.
fn zone_brightness(mood: Mood) -> f32 {
    // Every value here is a multiplier on the mat's **opacity**, so 1.0 is
    // the ceiling and anything past it is not brighter, it is clipped. It has
    // been the ceiling since the accent moved off the material's tint and
    // into the mat itself: at 1.311 and 1.0925 a local seat being asked
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
        Standing::Asked => 1.0,
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

/// What the firewheel should be burning at, packed the way the shader reads
/// it: `(white, blue, black, red)` and `(green, up.x, up.y, spare)`.
///
/// The strengths come from what can make coloured mana on the **whole
/// table** — every seat's, not this one's — because the wheel in the middle
/// belongs to the table rather than to a chair. `up` is screen-up in table
/// space: the local seat's own inward direction, which is one direction for
/// all five flames and is what makes them five candles seen from one chair
/// instead of a sun glyph.
fn firewheel_of(
    duel: &Duel,
    board: &baylee_client_core::board::BoardModel,
    layout: &TableLayout,
) -> (Vec4, Vec4) {
    let seats = board.pods.len().max(1);
    let burn = duel.view.as_ref().map_or([0.0; 5], |view| {
        baylee_client_core::firewheel::strength(crate::manasources::table_mana(view), seats)
    });
    let up = layout
        .local()
        .and_then(|slot| (-slot.center).try_normalize())
        .unwrap_or(Vec2::Y);
    (
        Vec4::new(burn[0], burn[1], burn[2], burn[3]),
        Vec4::new(burn[4], up.x, up.y, 0.0),
    )
}

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
#[allow(clippy::too_many_lines)] // slab creation and incremental material update share one state
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
    // because the origin is where the firewheel burns and where the pool
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

    let (want_flames, want_tail) = firewheel_of(&duel, board, layout);
    let up = want_tail.yz();

    let want = crate::feltmat::wash_of(board.step);
    // Two rates, one decision: with reduce-motion on, both arrive at once.
    let (ease, fire_ease) = if prefs.all().reduce_motion {
        (1.0, 1.0)
    } else {
        let step = time.delta_secs();
        (
            1.0 - (-WASH_RATE * step).exp(),
            1.0 - (-FIRE_RATE * step).exp(),
        )
    };

    let Ok((mut slab, mut mesh, handle)) = slabs.single_mut() else {
        // No slab yet. Cut one, and let the next frame light it.
        commands.spawn((
            DuelStage,
            crate::compass::Compass::default(),
            Slab {
                cut: span,
                shown: Vec4::ZERO,
                source,
                // Cut dark and let the next frame light it, like the lamp
                // above: five flames at the pilot light is what zero means,
                // and a table cut before a board has arrived is simply the
                // table with its wheel banked down.
                flames: Vec4::ZERO,
                tail: Vec4::new(0.0, up.x, up.y, 0.0),
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
                    flames: Vec4::ZERO,
                    flames_tail: Vec4::new(0.0, up.x, up.y, 0.0),
                    span,
                    corner: tabletop::table_corner(span),
                    rail: tabletop::RAIL_WIDTH,
                    motion,
                    gain: crate::feltmat::WASH_GAIN,
                    thickness: TABLE_THICKNESS,
                    rotation: 0.0,
                    pattern: duel.table_pattern.0,
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
    // The flames ease on their own rate, slower than the lamp: the lamp is
    // a step changing and the wheel is a board filling up.
    let next_flames = slab.flames + (want_flames - slab.flames) * fire_ease;
    let next_tail = slab.tail + (want_tail - slab.tail) * fire_ease;
    // The flicker itself runs on `globals.time` inside the shader and needs
    // no upload at all, so a wheel that has reached its heights stops
    // touching the material and goes on burning.
    let still = (next - slab.shown).abs().max_element() <= 1e-4
        && (next_flames - slab.flames).abs().max_element() <= 1e-4
        && (next_tail - slab.tail).abs().max_element() <= 1e-4
        && (slab.source - source).abs().max_element() <= 1e-4
        && (slab.motion - motion).abs() <= f32::EPSILON;
    if still && !recut {
        return;
    }
    slab.shown = next;
    slab.flames = next_flames;
    slab.tail = next_tail;
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
        material.params.flames = next_flames;
        material.params.flames_tail = next_tail;
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
/// every mat, glow and card is buried inside the table.
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
        let params = mat_params(
            accent,
            size,
            mood,
            moving,
            baylee_client_core::layout::LEDGE_IS_OUTER,
        );
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
    index.materials.clear();
    index.face_materials.clear();
    index.faces.clear();
    index.marks.clear();
    index.marks_materials.clear();
    index.badges.clear();
    index.badge_materials.clear();
    index.floors.clear();
    index.floor_materials.clear();
    index.shells.clear();
    index.shell_materials.clear();
    index.stacks.clear();
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

/// One card width down the card, in table units.
///
/// The mesh is [`CARD_HEIGHT`] tall and the shaders measure a card as
/// 1/[`cardrail::CARD_ASPECT`] card widths, and the two differ in the fourth
/// place. Anything placed on the card in the shaders' units goes down it by
/// this, so the strip's bottom edge lands on the seam the card shader draws
/// and not a ten-thousandth beside it.
pub(crate) const DOWN_THE_CARD: f32 = CARD_HEIGHT * cardrail::CARD_ASPECT;

/// The keyword strip lying on a card (#274): a marker, so a strip can be
/// found and counted without being taken for the card or its shadow.
#[derive(Component)]
pub struct KeywordStrip;

/// Where a card's keyword strip lies, in the card's own space: at
/// [`cardrail::quad_rect`], a share of its row's step over the face.
fn strip_transform(rung: f32) -> Transform {
    let [x0, y0, x1, y1] = cardrail::quad_rect();
    Transform::from_xyz(
        (f32::midpoint(x0, x1) - 0.5) * CARD_WIDTH,
        CARD_HEIGHT * 0.5 - f32::midpoint(y0, y1) * DOWN_THE_CARD,
        CARD_THICKNESS + rung * STRIP_STEP_SHARE,
    )
}

/// Puts the strip on a card, changes it, or takes it off (#274, #298).
///
/// A diff like the rest of [`sync_scene`]: a card whose strip and row step
/// have not moved costs one lookup. The strip is a child of the card, so it
/// follows every glide, tap, lift and exit with nothing to keep in step, and
/// goes when the card does. It is not a [`CardShadow`] —
/// [`ground_the_shadows`] must not flatten it onto the felt — and it is not
/// pickable: a click on a mark is a click on the card.
fn sync_strip(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<MarksMaterial>,
    card: Entity,
    (object, strip, rung): (ObjectId, cardrail::Strip, f32),
    motion: f32,
) {
    let current = index.marks.get(&object).copied();
    if current.is_some_and(|(said, at, _)| said == strip && at.to_bits() == rung.to_bits()) {
        return;
    }
    if strip.is_empty() {
        if let Some((.., entity)) = index.marks.remove(&object) {
            commands.entity(entity).despawn();
        }
        return;
    }
    let Some(quad) = index.marks_quad.clone() else {
        return;
    };
    let material = index
        .marks_materials
        .entry(strip)
        .or_insert_with(|| materials.add(MarksMaterial::new(strip, motion)))
        .clone();
    let transform = strip_transform(rung);
    let entity = if let Some((.., entity)) = current {
        commands
            .entity(entity)
            .try_insert((MeshMaterial3d(material), transform));
        entity
    } else {
        let entity = commands
            .spawn((
                KeywordStrip,
                Mesh3d(quad),
                MeshMaterial3d(material),
                transform,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(card).add_child(entity);
        entity
    };
    index.marks.insert(object, (strip, rung, entity));
}

/// The count badge at a merged card's corner (#261): a marker, so a badge
/// can be found and counted without being taken for the card or its strip.
#[derive(Component)]
pub struct CountBadge;

/// What a card's badge was put on for: the count, the row step, where it
/// stands and whether its card is tapped.
type BadgeKey = (u32, f32, BadgePlace, bool);

/// A badge standing over its card ([`BadgePlace::Above`]) stays upright when
/// the card taps (the owner, 25.09): it is where it would be on the card
/// untapped, clear of the row, and tapping moves nothing else in a row
/// either. It is still the card's child, so it glides and goes with it;
/// [`keep_badges_upright`] turns it back by as much as the card has turned.
#[derive(Component, Clone, Copy, Debug)]
pub struct Upright {
    /// The card's rotation untapped.
    base: Quat,
    /// The badge's transform on the card untapped.
    at: Transform,
}

impl Upright {
    /// The badge's transform on a card turned `rotation`: its place on the
    /// card untapped, turned back by as much as the card is turned from
    /// untapped.
    fn on(&self, rotation: Quat) -> Transform {
        let back = rotation.inverse() * self.base;
        Transform {
            translation: back * self.at.translation,
            rotation: back * self.at.rotation,
            scale: self.at.scale,
        }
    }
}

/// Where a card's count badge lies, in the card's own space, untapped: at
/// [`cardplate::badge_quad_rect`] for `place`, at the strip's share of its
/// row's step over the face.
///
/// The strip's height and for the strip's reason: a badge lifted further
/// would stand over the card laid on this one.
fn badge_transform(rung: f32, place: BadgePlace) -> Transform {
    let [x0, y0, x1, y1] = cardplate::badge_quad_rect(place);
    Transform::from_xyz(
        (f32::midpoint(x0, x1) - 0.5) * CARD_WIDTH,
        CARD_HEIGHT * 0.5 - f32::midpoint(y0, y1) * DOWN_THE_CARD,
        CARD_THICKNESS + rung * STRIP_STEP_SHARE,
    )
}

/// Keeps every badge standing over its card upright while its card turns
/// (#298): a tap glides, and the badge is the card's child, so it is laid
/// again from where the glide has the card this frame.
pub fn keep_badges_upright(
    cards: Query<&Transform, Without<CountBadge>>,
    mut badges: Query<(&ChildOf, &Upright, &mut Transform), With<CountBadge>>,
) {
    for (parent, upright, mut local) in &mut badges {
        if let Ok(card) = cards.get(parent.parent()) {
            local.set_if_neq(upright.on(card.rotation));
        }
    }
}

/// Puts the count badge on a card, changes it, or takes it off (#261).
///
/// [`sync_strip`]'s diff, for [`sync_strip`]'s reasons: a child of the card,
/// so it follows every glide and goes with the card; not a [`CardShadow`];
/// not pickable, since a click on the count is a click on the card. Where it
/// stands is its seat's rows' ([`SeatSlot::badge_place`]): over the card it
/// is [`Upright`], beside it it turns with a tapped card.
fn sync_badge(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<BadgeMaterial>,
    card: Entity,
    placement: &Placement,
) {
    let place = placement.slot.badge_place();
    let key: BadgeKey = (placement.badge, placement.rung, place, placement.tapped);
    let current = index.badges.get(&placement.object).copied();
    if current.is_some_and(|((count, rung, at, tapped), _)| {
        (count, at, tapped) == (key.0, key.2, key.3) && rung.to_bits() == key.1.to_bits()
    }) {
        return;
    }
    if placement.badge == 0 {
        if let Some((_, badge)) = index.badges.remove(&placement.object) {
            commands.entity(badge).despawn();
        }
        return;
    }
    let Some(quad) = index.badge_quad.clone() else {
        return;
    };
    let material = index
        .badge_materials
        .entry((placement.badge, place))
        .or_insert_with(|| materials.add(BadgeMaterial::new(placement.badge, place)))
        .clone();
    let at = badge_transform(placement.rung, place);
    let upright = (place == BadgePlace::Above).then(|| Upright {
        base: card_transform(&placement.slot, placement.position, false, placement.lift).rotation,
        at,
    });
    // Laid for where the card will come to rest; `keep_badges_upright`
    // keeps an upright one so on the way there.
    let transform = upright.map_or(at, |upright| {
        upright.on(card_transform(
            &placement.slot,
            placement.position,
            placement.tapped,
            placement.lift,
        )
        .rotation)
    });
    let badge = if let Some((_, badge)) = current {
        commands
            .entity(badge)
            .try_insert((MeshMaterial3d(material), transform));
        badge
    } else {
        let badge = commands
            .spawn((
                CountBadge,
                Mesh3d(quad),
                MeshMaterial3d(material),
                transform,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(card).add_child(badge);
        badge
    };
    match upright {
        Some(upright) => {
            commands.entity(badge).try_insert(upright);
        }
        None => {
            commands.entity(badge).try_remove::<Upright>();
        }
    }
    index.badges.insert(placement.object, (key, badge));
}

/// The offer's light on the felt under a card (#298): a marker, so a light
/// can be found and counted without being taken for the card, its shadow or
/// its strip.
#[derive(Component)]
pub struct FloorLight;

/// Where a card's light lies, in the card's own space: `depth` under the
/// card's own lift, so on the felt at [`FLOOR_RUNG`] however high the card's
/// row and deck have put it.
fn floor_transform(depth: f32) -> Transform {
    Transform::from_xyz(0.0, 0.0, -(CARD_LIFT - FLOOR_RUNG + depth))
}

/// Puts the offer's light under a card, changes it, or takes it away (#298).
///
/// [`sync_strip`]'s diff, for [`sync_strip`]'s reasons: a child of the card,
/// so it follows every glide and tap and goes with the card; not a
/// [`CardShadow`]; not pickable, since a click on the light round a card
/// means the table. It lies at the felt under the row's rise and the deck,
/// and rides a card lifted by a hover or by flying, whose light it still is.
/// A card standing in a pile's hover fan is tipped up in the air, so its
/// light lies just under it instead, as a halo round the card.
fn sync_floor(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<FloorMaterial>,
    card: Entity,
    placement: &Placement,
    glow: u32,
    motion: f32,
) {
    let offers = glow & crate::cardmat::glow::OFFERS;
    let depth = if placement.fan.is_some() {
        0.0
    } else {
        placement.lift + stack_rise(placement.count.saturating_sub(1))
    };
    let current = index.floors.get(&placement.object).copied();
    if current.is_some_and(|(said, at, _)| said == offers && at.to_bits() == depth.to_bits()) {
        return;
    }
    if offers == 0 {
        if let Some((.., light)) = index.floors.remove(&placement.object) {
            commands.entity(light).despawn();
        }
        return;
    }
    let Some(quad) = index.floor_quad.clone() else {
        return;
    };
    let material = index
        .floor_materials
        .entry(offers)
        .or_insert_with(|| materials.add(FloorMaterial::new(offers, motion)))
        .clone();
    let transform = floor_transform(depth);
    let light = if let Some((.., light)) = current {
        commands
            .entity(light)
            .try_insert((MeshMaterial3d(material), transform));
        light
    } else {
        let light = commands
            .spawn((
                FloorLight,
                Mesh3d(quad),
                MeshMaterial3d(material),
                transform,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(card).add_child(light);
        light
    };
    index
        .floors
        .insert(placement.object, (offers, depth, light));
}

/// Which part of a permanent's shell an entity is ([`shellmat`]).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellPart {
    /// The rim standing round the card.
    Rim,
    /// The ring on the felt the rim lies down to where it has no room.
    Ring,
    /// A dome standing over the card.
    Dome,
    /// The ring on the felt a dome lies down to.
    DomeRing,
    /// Defender's wall, on the felt past the card's top edge.
    Wall,
}

/// One shell round a card: what stands, the ring it lies down to, and how
/// it stands as [`fit_the_shells`] last found: its step of
/// [`shellmat::DOME_STEPS`] (a rim has only the first), or `None` lying.
#[derive(Clone, Copy, Debug)]
struct Layer {
    stand: Entity,
    lie: Entity,
    step: Option<usize>,
}

/// The shells round one card, inside out: see [`SceneIndex::shells`].
#[derive(Clone, Copy, Debug, Default)]
struct Shell {
    /// Indestructible's steel.
    steel: Option<Layer>,
    /// Hexproof's or shroud's dome, and which.
    dome: Option<(shellmat::Dome, Layer)>,
    /// Defender's wall, which always stands.
    wall: Option<Entity>,
}

impl Shell {
    fn layers(self) -> impl Iterator<Item = Layer> {
        self.steel
            .into_iter()
            .chain(self.dome.map(|(_, layer)| layer))
    }

    fn despawn(self, commands: &mut Commands) {
        for layer in self.layers() {
            commands.entity(layer.stand).despawn();
            commands.entity(layer.lie).despawn();
        }
        if let Some(wall) = self.wall {
            commands.entity(wall).despawn();
        }
    }

    fn is_empty(self) -> bool {
        self.layers().next().is_none() && self.wall.is_none()
    }
}

/// The material for `look`, made the first time a table asks for it.
fn shell_material(
    index: &mut SceneIndex,
    materials: &mut Assets<ShellMaterial>,
    look: ShellLook,
    motion: f32,
) -> Handle<ShellMaterial> {
    index
        .shell_materials
        .entry(look)
        .or_insert_with(|| materials.add(ShellMaterial::new(look, motion)))
        .clone()
}

/// Spawns one shell round `card`, hidden: what stands, at `stand_at` in the
/// card's space, and the ring it lies down to.
fn spawn_layer(
    commands: &mut Commands,
    card: Entity,
    stand: (ShellPart, Handle<Mesh>, Handle<ShellMaterial>),
    lie: (ShellPart, Handle<Mesh>, Handle<ShellMaterial>),
    step: Option<usize>,
) -> Layer {
    let [stand, lie] = [(stand, true), (lie, false)].map(|((part, mesh, material), up)| {
        commands
            .spawn((
                part,
                Mesh3d(mesh),
                MeshMaterial3d(material),
                // What stands has its origin on the card's face, which is the
                // plane its mask measures the print on; a ring is laid on
                // the felt by `fit_the_shells`.
                if up {
                    Transform::from_xyz(0.0, 0.0, CARD_THICKNESS)
                } else {
                    Transform::default()
                },
                Visibility::Hidden,
                Pickable::IGNORE,
            ))
            .id()
    });
    commands.entity(card).add_children(&[stand, lie]);
    Layer { stand, lie, step }
}

/// Puts a protected permanent's shells round it, or takes them away:
/// indestructible's steel, hexproof's or shroud's dome, and defender's
/// wall.
///
/// Each is spawned hidden, as children of the card, and [`fit_the_shells`]
/// shows it standing or lying on the same frame: which depends on where
/// every card is once they have all moved, which is not known here. A card
/// standing in a pile's hover fan has none: it is not on the battlefield.
fn sync_shell(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<ShellMaterial>,
    card: Entity,
    placement: &Placement,
    motion: f32,
) {
    let on_table = placement.fan.is_none();
    let steel = placement.indestructible && on_table;
    let dome = placement.dome.filter(|_| on_table);
    let wall = placement.defender && on_table;
    let mut shell = index
        .shells
        .get(&placement.object)
        .copied()
        .unwrap_or_default();
    if shell.steel.is_some() == steel
        && shell.dome.map(|(d, _)| d) == dome
        && shell.wall.is_some() == wall
    {
        return;
    }
    let (Some(rim_mesh), Some(ring_mesh)) = (
        index.rim_mesh.clone(),
        index.ring_meshes.get(&Band::Whole).cloned(),
    ) else {
        return;
    };
    if shell.steel.is_some() != steel {
        if let Some(layer) = shell.steel.take() {
            Shell {
                steel: Some(layer),
                ..Shell::default()
            }
            .despawn(commands);
        } else {
            let rim = shell_material(index, materials, ShellLook::steel(ShellKind::Rim), motion);
            let ring = shell_material(index, materials, ShellLook::steel(ShellKind::Ring), motion);
            // The half of the band it takes under a dome's ring, ready for
            // the frame both lie down.
            let mut inner = ShellLook::steel(ShellKind::Ring);
            inner.band = Band::Inner;
            shell_material(index, materials, inner, motion);
            shell.steel = Some(spawn_layer(
                commands,
                card,
                (ShellPart::Rim, rim_mesh, rim),
                (ShellPart::Ring, ring_mesh.clone(), ring),
                None,
            ));
        }
    }
    if shell.dome.map(|(d, _)| d) != dome {
        if let Some(gone) = shell.dome.take() {
            Shell {
                dome: Some(gone),
                ..Shell::default()
            }
            .despawn(commands);
        }
        if let Some(which) = dome {
            let look = |kind, band| ShellLook {
                kind,
                dome: Some(which),
                band,
            };
            let standing =
                shell_material(index, materials, look(ShellKind::Dome, Band::Whole), motion);
            let lying = shell_material(
                index,
                materials,
                look(ShellKind::DomeRing, Band::Whole),
                motion,
            );
            shell_material(
                index,
                materials,
                look(ShellKind::DomeRing, Band::Outer),
                motion,
            );
            if let Some(mesh) = index.dome_meshes.get(&(which, 0)).cloned() {
                shell.dome = Some((
                    which,
                    spawn_layer(
                        commands,
                        card,
                        (ShellPart::Dome, mesh, standing),
                        (ShellPart::DomeRing, ring_mesh, lying),
                        None,
                    ),
                ));
            }
        }
    }
    if shell.wall.is_some() != wall {
        shell.wall = sync_wall(commands, index, materials, card, shell.wall, motion);
    }
    if shell.is_empty() {
        index.shells.remove(&placement.object);
    } else {
        index.shells.insert(placement.object, shell);
    }
}

/// Takes defender's wall away if `had` one, or builds one, hidden, as a
/// child of `card`: [`fit_the_shells`] stands it on the felt.
fn sync_wall(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<ShellMaterial>,
    card: Entity,
    had: Option<Entity>,
    motion: f32,
) -> Option<Entity> {
    if let Some(gone) = had {
        commands.entity(gone).despawn();
        return None;
    }
    let mesh = index.wall_mesh.clone()?;
    let material = shell_material(index, materials, ShellLook::steel(ShellKind::Wall), motion);
    let wall = commands
        .spawn((
            ShellPart::Wall,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_child(wall);
    Some(wall)
}

/// Where a ring lies under the card at `at`, in the card's own space: flat on
/// the felt at [`shellmat::RING_RUNG`] under the card's middle, unbanked,
/// the way [`ground_the_shadows`] holds a flier's shadow. It grows with the
/// card, since it is drawn round it.
fn on_the_felt(at: &Transform) -> Transform {
    let ground = Vec3::new(
        at.translation.x,
        TABLE_Y + shellmat::RING_RUNG,
        at.translation.z,
    );
    let right = at.rotation * Vec3::X;
    let yaw = (-right.z).atan2(right.x);
    Transform {
        translation: at.to_matrix().inverse().transform_point3(ground),
        rotation: at.rotation.inverse()
            * Quat::from_rotation_y(yaw)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        scale: Vec3::ONE,
    }
}

/// Where defender's wall stands for the card at `at`, in the card's own
/// space: on the felt under the card's middle, turned as the card is and
/// unbanked, at the height a resting card's face has, which is the plane
/// its mesh and its mask are drawn from ([`shellmat::wall_mesh`]). Never
/// lifted or grown with the card, which would carry it over its
/// neighbours' faces.
pub fn wall_pose(at: &Transform) -> Transform {
    let mut pose = on_the_felt(at);
    let foot = Vec3::new(
        at.translation.x,
        TABLE_Y + shellmat::RIM_DROP,
        at.translation.z,
    );
    pose.translation = at.to_matrix().inverse().transform_point3(foot);
    pose.scale = Vec3::ONE / at.scale;
    pose
}

/// Stands each protected permanent's shells, or lays them on the felt as
/// rings, and keeps the rings flat on the felt under their card.
///
/// After the glide, because the question is about where every card *is* this
/// frame: a card gliding in, a hover lifting one, a flier on its bob. It asks
/// [`shellmat::rim_stands`] for the steel and [`shellmat::dome_step`] for the
/// dome, against every card on the table, departing ones included, from
/// where the camera is. A dome stands at the tallest step that fits. Where
/// the steel and the dome both lie, the steel takes the band's inner half
/// and the dome its outer, nested as they stand. Defender's wall always
/// stands, on the felt ([`wall_pose`]).
#[allow(clippy::type_complexity)] // four disjoint views of one table
#[allow(clippy::too_many_lines)] // three shells fitted in one pass over the table
pub fn fit_the_shells(
    mut index: ResMut<SceneIndex>,
    cards: Query<
        (Entity, &Transform, &Visibility),
        (Or<(With<CardVisual>, With<Departing>)>, Without<ShellPart>),
    >,
    camera: Query<&Transform, (With<TableCamera>, Without<ShellPart>)>,
    mut parts: Query<
        (
            &mut Visibility,
            &mut Transform,
            &mut Mesh3d,
            &mut MeshMaterial3d<ShellMaterial>,
        ),
        (With<ShellPart>, Without<TableCamera>),
    >,
) {
    if index.shells.is_empty() {
        return;
    }
    let Ok(eye) = camera.single() else {
        return;
    };
    let eye = eye.translation;
    // Only the cards that are drawn: a scrolled row's hidden cards stand
    // past the lane's end, where nothing is to be kept clear of them.
    let faces: Vec<(Entity, shellmat::Footprint)> = cards
        .iter()
        .filter(|(_, _, seen)| **seen != Visibility::Hidden)
        .map(|(entity, at, _)| (entity, shellmat::Footprint::of(at)))
        .collect();
    let SceneIndex {
        shells,
        cards: drawn,
        ring_meshes,
        dome_meshes,
        shell_materials,
        ..
    } = &mut *index;
    for (object, shell) in shells.iter_mut() {
        let Some(&card) = drawn.get(object) else {
            continue;
        };
        let Ok((_, at, _)) = cards.get(card) else {
            continue;
        };
        if let Some(wall) = shell.wall
            && let Ok((mut shown, mut transform, _, _)) = parts.get_mut(wall)
        {
            shown.set_if_neq(Visibility::Inherited);
            transform.set_if_neq(wall_pose(at));
        }
        let me = shellmat::Footprint::of(at);
        let others = faces
            .iter()
            .filter(move |(entity, _)| *entity != card)
            .map(|(_, face)| *face);
        if let Some(steel) = &mut shell.steel {
            let headroom = if steel.step.is_some() {
                0.0
            } else {
                shellmat::STAND_AGAIN
            };
            steel.step = shellmat::rim_stands(&me, others.clone(), eye, headroom).then_some(0);
        }
        if let Some((dome, layer)) = &mut shell.dome {
            layer.step = shellmat::dome_step(dome.row(), layer.step, &me, others, eye);
        }
        let both_lie =
            shell.layers().all(|layer| layer.step.is_none()) && shell.layers().count() == 2;
        let lying = on_the_felt(at);
        let mut show = |layer: Layer, stand_mesh: Option<Handle<Mesh>>, ring: ShellLook| {
            if let Ok((mut shown, _, mut mesh, _)) = parts.get_mut(layer.stand) {
                shown.set_if_neq(if layer.step.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
                if let Some(wanted) = stand_mesh
                    && mesh.0 != wanted
                {
                    mesh.0 = wanted;
                }
            }
            if let Ok((mut shown, mut transform, mut mesh, mut material)) = parts.get_mut(layer.lie)
            {
                shown.set_if_neq(if layer.step.is_some() {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                });
                transform.set_if_neq(lying);
                if let Some(wanted) = ring_meshes.get(&ring.band)
                    && mesh.0 != *wanted
                {
                    mesh.0 = wanted.clone();
                }
                if let Some(wanted) = shell_materials.get(&ring)
                    && material.0 != *wanted
                {
                    material.0 = wanted.clone();
                }
            }
        };
        if let Some(steel) = shell.steel {
            let mut ring = ShellLook::steel(ShellKind::Ring);
            if both_lie {
                ring.band = Band::Inner;
            }
            show(steel, None, ring);
        }
        if let Some((dome, layer)) = shell.dome {
            let ring = ShellLook {
                kind: ShellKind::DomeRing,
                dome: Some(dome),
                band: if both_lie { Band::Outer } else { Band::Whole },
            };
            let mesh = layer
                .step
                .and_then(|step| dome_meshes.get(&(dome, step)).cloned());
            show(layer, mesh, ring);
        }
    }
}

/// One slab of the deck under a card: depth, not a card. A marker, so the
/// slabs can be found and counted without being taken for the card.
#[derive(Component)]
pub struct StackSlab;

/// What stands under one card: see [`SceneIndex::stacks`].
#[derive(Default)]
struct Stack {
    /// The count the deck was built for.
    count: usize,
    /// Whether the pile was laid tapped: a merged pile's cards step out to
    /// its seat's left, which a tapped card's own space names differently.
    tapped: bool,
    /// The slabs, top first.
    slabs: Vec<Entity>,
    /// The contact shadow under the whole deck.
    shadow: Option<Entity>,
}

/// Where the `i`-th of `layers` cards under a merged pile hangs, the pile
/// standing `deck` high (#263): that share of the deck down, and a further
/// [`PILE_JOG`] out to the left of the seat for each, whether or not the
/// pile is tapped (the owner, 25.09). A tapped card is turned a quarter
/// about its face, so its seat's left is its own −y, and the offset is laid
/// there.
fn pile_slab_transform(i: usize, layers: usize, deck: f32, tapped: bool) -> Transform {
    #[allow(clippy::cast_precision_loss)] // at most `PILE_SLABS`
    let (step, share) = (i as f32, i as f32 / layers as f32);
    let out = -step * PILE_JOG * CARD_WIDTH;
    let (x, y) = if tapped { (0.0, out) } else { (out, 0.0) };
    Transform::from_xyz(x, y, -deck * share)
}

/// The colour of the `i`-th card under a merged pile: the back and
/// [`SLAB_EDGE_COLOR`] in turn, so each edge of the staircase stands out
/// from the one above it.
fn pile_slab_color(i: usize) -> Color {
    if i % 2 == 1 {
        SLAB_EDGE_COLOR
    } else {
        BACK_COLOR
    }
}

/// Where the `i`-th of `layers` slabs hangs under a zone pile standing
/// `deck` high, in the card's own space: that share of the deck down, to the right
/// for an odd slab and to the left for an even one, and further out the
/// deeper it lies — from half of [`PILE_JOG`] towards all of it — so every
/// slab's edge shows past the one above it on its side, not only the first.
fn slab_transform(i: usize, layers: usize, deck: f32) -> Transform {
    let side = if i % 2 == 1 { 1.0 } else { -1.0 };
    let share = i as f32 / layers as f32;
    Transform::from_xyz(
        side * PILE_JOG * CARD_WIDTH * (0.5 + 0.5 * share),
        0.0,
        -deck * share,
    )
}

/// Builds what stands under a card — the slabs of its deck and its contact
/// shadow — and rebuilds it when the count changes (#261).
///
/// A pile stands on the cards under it: the top card is drawn at the deck's
/// own height and the rest hangs below it as children, so what a player sees
/// is one block of cardboard with a face on top. The slabs are cards with no
/// print, jogged ([`PILE_JOG`]) so their edges show, in two colours in turn
/// ([`pile_slab_color`], [`slab_color`]) so the layers do: a merged pile's
/// to the left, at most [`PILE_SLABS`] of them, a zone pile's to either
/// side. Children and not loose
/// entities, for the strip's reasons and one more: as loose entities they
/// were never despawned at all, and every card that ever lay on a graveyard
/// left its slabs standing there for the rest of the game.
///
/// The shadow is rebuilt only with the count, and rebuilt rather than moved:
/// [`ground_the_shadows`] remembers a flier's resting shadow by entity, and a
/// moved one would be put back where the smaller deck had it when the card
/// lands. It sits under the whole deck and wider the taller the deck is — a
/// thick pile sits in more shadow than a single card does, which is most of
/// what makes it read as thick at all.
fn sync_stack(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<CardMaterial>,
    card: Entity,
    placement: &Placement,
    motion: f32,
) {
    // A merged card on the battlefield, which the badge is only ever on.
    let pile = placement.badge > 0;
    let current = index.stacks.get(&placement.object);
    if current.is_some_and(|stack| {
        stack.count == placement.count && (!pile || stack.tapped == placement.tapped)
    }) {
        return;
    }
    let Some(quad) = index.quad.clone() else {
        return;
    };
    let recount = current.is_none_or(|stack| stack.count != placement.count);
    let mut stack = index.stacks.remove(&placement.object).unwrap_or_default();
    for slab in stack.slabs.drain(..) {
        commands.entity(slab).despawn();
    }
    let under = placement.count.saturating_sub(1);
    // A pile shows at most `PILE_SLABS` cards under it and stands as high
    // as they do: the badge says how many there are.
    let layers = if pile {
        under.min(PILE_SLABS)
    } else {
        stack_layers(under)
    };
    let deck = stack_rise(if pile { layers } else { under });
    if recount {
        if let Some(shadow) = stack.shadow.take() {
            commands.entity(shadow).despawn();
        }
        if let Some((mesh, material)) = index.shadow_quad.clone().zip(index.shadow_material.clone())
        {
            let shadow = commands
                .spawn((
                    CardShadow,
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0.0, 0.0, -(CARD_LIFT * 0.5 + deck)).with_scale(Vec3::new(
                        1.0 + deck * DECK_SHADOW_SPREAD,
                        1.0 + deck * DECK_SHADOW_SPREAD,
                        1.0,
                    )),
                    // Between the felt and the card, and a click near a
                    // card's edge means the table.
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(card).add_child(shadow);
            stack.shadow = Some(shadow);
        }
    }
    for i in 1..=layers {
        // Nothing sees more of a slab than its jog and its wall: the card on
        // top covers the rest.
        let color = if pile {
            pile_slab_color(i)
        } else {
            slab_color(i)
        };
        let look = CardLook::flat(color, FinishTreatment::Plain);
        let material = index
            .face_materials
            .entry(look)
            .or_insert_with(|| materials.add(material(look, None, color, motion)))
            .clone();
        let slab = commands
            .spawn((
                StackSlab,
                Mesh3d(quad.clone()),
                MeshMaterial3d(material),
                if pile {
                    pile_slab_transform(i, layers, deck, placement.tapped)
                } else {
                    slab_transform(i, layers, deck)
                },
                // The deck under a card is depth, not cards: the top card is
                // what a click has to reach.
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(card).add_child(slab);
        stack.slabs.push(slab);
    }
    stack.count = placement.count;
    stack.tapped = placement.tapped;
    index.stacks.insert(placement.object, stack);
}

/// How one card of a pile's hover fan is turned.
///
/// A rotation of its own rather than two more arguments to
/// [`card_transform`], because the two are never both true: a card lifted out
/// of a graveyard is not a permanent and has no tap state to compose with.
///
/// The order is the one a card is already built by — lay the quad flat, then
/// turn it to face its owner — with the lying-flat quarter turn short of
/// square by the pose's tilt, so the edge furthest from the owner rises. Which
/// edge that is on *screen* is the pose's business and not this function's;
/// see [`baylee_client_core::FanPose::tilt`].
fn fan_rotation(slot: &SeatSlot, pose: baylee_client_core::FanPose) -> Quat {
    Quat::from_rotation_y(-slot.facing)
        * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2 + pose.tilt)
        * Quat::from_rotation_z(pose.yaw)
}

/// Where every group in the current board model belongs.
#[allow(clippy::struct_excessive_bools)] // five facts about one card, each read on its own
struct Placement {
    object: ObjectId,
    slot: SeatSlot,
    position: Vec2,
    /// Where this card stands in its row, as a height — see [`LANE_RISE`].
    lift: f32,
    tapped: bool,
    /// Whether this card has flying, and therefore stands off the felt — see
    /// [`airborne`].
    ///
    /// Read off the group's badges, which is where the client is told: the
    /// engine sends a keyword bitset and `KeywordBadge::from_bits` is what
    /// turns it into facts. Resolved here for the reason the offer and the
    /// corner are — this is where the group's members are.
    flying: bool,
    /// Whether this card is an indestructible permanent, and so stands in
    /// its steel rim ([`shellmat`]). Read off the group's badges as flying
    /// is; false for every pile, where indestructible means nothing.
    indestructible: bool,
    /// The dome this card stands under, if it has hexproof or shroud
    /// ([`shellmat::Dome::of`]); read and meaningful as `indestructible` is.
    dome: Option<shellmat::Dome>,
    /// Whether this card has defender, and so stands behind its wall
    /// ([`shellmat::wall_mesh`]); read and meaningful as `indestructible` is.
    defender: bool,
    count: usize,
    /// What the count badge says ([`cardplate::count_word`]): how many
    /// permanents this card stands for when it is a merged group on the
    /// battlefield, and zero for a lone card, a pile and a fanned card — a
    /// pile's size is `count` above and is drawn as the deck under it.
    badge: u32,
    art: Option<ImageKey>,
    offer: crate::cardmat::Offer,
    corner: baylee_client_core::cardplate::Corner,
    selected: bool,
    /// Set while this card stands in a pile's hover fan, and then it carries
    /// the pose and the pile the card was lifted out of.
    ///
    /// The pile is here for the way *back*. A fan closes by leaving the board
    /// model, and a card that leaves with no zone change behind it is
    /// despawned where it stands — right for a graveyard's old top card,
    /// covered by the one that landed on it, and wrong for seven cards that
    /// have to drop back into the pile they came out of.
    fan: Option<(baylee_client_core::FanPose, PileKind)>,
    /// The keyword strip's word ([`cardrail::badge_bits`]): zero for a card
    /// wearing no marks, which is every card that is not a permanent.
    marks: u32,
    /// Whether the strip wears the moon: a creature that cannot attack or
    /// tap this turn (CR 302.6, `CardGroup::summoning_sick`).
    sick: bool,
    /// The identity crests at the strip's end ([`cardcrest::marks`]).
    crests: [Option<usize>; cardcrest::MAX_CRESTS],
    /// Whether the card laid after this one in its row covers its lower
    /// right, where the print writes its power and toughness — a fanned
    /// lane — so the strip has to say them (`Corner::shows_plate`).
    covered: bool,
    /// How much higher the next card of this card's row stands, which the
    /// strip lies a share of: see [`STRIP_STEP_SHARE`].
    rung: f32,
    /// Whether the card is drawn: false for the cards of a scrolled row
    /// outside the run it shows (`LanePacking::window`).
    shown: bool,
}

/// The place a pile stands for, for the zone machinery that speaks in places.
///
/// `None` for a library: nothing ever moves *to* one that this table can
/// draw, and nothing is ever lifted out of one that has an object behind it.
fn place_of(kind: PileKind, player: PlayerId) -> Option<Place> {
    match kind {
        PileKind::Graveyard => Some(Place::Graveyard(player)),
        PileKind::Exile => Some(Place::Exile(player)),
        PileKind::Command | PileKind::Command2 => Some(Place::Command(player)),
        PileKind::Library => None,
    }
}

/// How far above its row a card stands because of what the card *is*.
///
/// Zero for everything but a creature with flying, which has a height of its
/// own and a slow bob around it ([`airborne`]). The height is returned rather
/// than applied: it joins the row's rise and the deck under the card in the
/// one lift [`card_transform`] is given, so it travels through
/// [`Motion`]/[`glide`] like every other reason a card is where it is. An
/// offset added *after* the glide would be pulled back by the glide on the
/// next frame and would compound.
///
/// `held` freezes the bob at its resting height — the height stays, only the
/// movement stops. It is true while the pointer is on the card, while the
/// card is chosen or armed, and while the player has motion turned off. The
/// first three are the same reading: a card someone is looking at should hold
/// still, and a card bobbing under a pointer that is not moving is the one
/// way this could take a click away from a player.
fn float_of(placement: &Placement, held: bool, elapsed: f32) -> f32 {
    if !placement.flying {
        0.0
    } else if held {
        airborne::RESTING
    } else {
        airborne::height(elapsed, airborne::phase(placement.object))
    }
}

/// Computes placements for the whole table.
///
/// Pure geometry over the board model, so the ordering is the model's ordering
/// and therefore stable frame to frame — which is what makes the diff below
/// cheap and stops cards from swapping places when nothing happened.
#[allow(clippy::too_many_lines)] // one walk of the model, in the model's order
fn placements(duel: &Duel) -> Vec<Placement> {
    let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.layout.as_ref()) else {
        return Vec::new();
    };
    // Which pile, if any, the pointer has spread open. Read once for the
    // whole table: at most one pile is ever open, because the pointer is over
    // at most one card.
    let fanned = board.fanned_pile(duel.hovered);
    // Who is in the fight. The same `Combat::read` the lines are drawn from,
    // so a card that has stepped out of its row and the line leaving it can
    // never disagree about whether the declaration exists — they are two
    // readings of one answer rather than two answers.
    let combat = duel
        .view
        .as_ref()
        .map(|view| Combat::read(view, duel.interaction.as_ref()));
    let mut out = Vec::new();
    for pod in &board.pods {
        let Some(slot) = layout.slot(pod.player) else {
            continue;
        };
        for lane in &pod.lanes {
            let center = slot.lane_center(lane.kind);
            // A merged card holds its cell whole, so its badge lies on no
            // neighbour; a row that cannot hold them and still fan legibly
            // shows a run of whole cards and scrolls (the owner, 25.09).
            let packing = lane.pack(slot);
            let window = packing.window(duel.rows.first((pod.player, lane.kind)));
            // The row's rise, shared out over however many cards are on it.
            let steps = lane.groups.len().saturating_sub(1).max(1) as f32;
            for (i, (group, offset)) in lane.groups.iter().zip(packing.offsets.iter()).enumerate() {
                let along = Vec2::new(slot.facing.cos(), -slot.facing.sin());
                // A group is one card standing for several, and combat is
                // declared per creature — so the step is asked of the members
                // and not of the representative. It cannot normally differ:
                // a declared attacker, sent or only proposed, is taken out of
                // its group by the board model for exactly this reason
                // (`board::Proposal`). `any` rather than `all` because if
                // that ever stops being true, a fighting card stepping
                // forward is the better failure.
                let staged = combat
                    .as_ref()
                    .is_some_and(|c| group.members.iter().any(|m| c.staged(*m)));
                let stage = if staged { STAGE_STEP } else { 0.0 };
                out.push(Placement {
                    object: group.representative,
                    slot: *slot,
                    position: center + along * (*offset + window.shift) + slot.forward() * stage,
                    // Later in the row is higher, so a fanned lane shingles
                    // the way a hand of cards does — each card over the one
                    // before it, and never in bands of both.
                    lift: LANE_RISE * i as f32 / steps,
                    marks: cardrail::badge_bits(&group.badges),
                    sick: group.summoning_sick,
                    crests: cardcrest::marks(group.provenance, group.commander),
                    // The last card shown has nothing laid over it, and
                    // nothing lies over a merged card's cell.
                    covered: packing.covered(i, &window),
                    shown: window.shown.contains(&i),
                    rung: LANE_RISE / steps,
                    tapped: group.status.is_tapped(),
                    // A group is one card standing for several and every
                    // member of it has the same keywords — `ObjectSummaryKey`
                    // carries them, so two Serra Angels of which one has lost
                    // flying are two groups.
                    flying: group.badges.contains(&KeywordBadge::Flying),
                    indestructible: group.badges.contains(&KeywordBadge::Indestructible),
                    dome: shellmat::Dome::of(
                        group.badges.contains(&KeywordBadge::Hexproof),
                        group.badges.contains(&KeywordBadge::Shroud),
                    ),
                    defender: group.badges.contains(&KeywordBadge::Defender),
                    count: group.count(),
                    badge: cardplate::count_word(group.count()),
                    art: group.art,
                    // Resolved here rather than in the sync loop, because
                    // here is where the group's *members* are: a plan taps
                    // one particular Forest, and the card drawn for it may
                    // be standing for four.
                    offer: crate::cardmat::Offer::on(
                        duel.proposing(),
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
                    fan: None,
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
            // The hover fan. It replaces the pile's own top card rather than
            // standing beside it — the top card *is* the first card of the
            // fan, and drawing both would put one card in two places and give
            // `index.cards` two entities for one id.
            //
            // The pile's thickness goes on the **lowest** card of the fan,
            // which is the one still standing over the pile's own place: it
            // is the rest of the cards, and carrying it up with the top card
            // instead would lift the whole deck into the air with it.
            if fanned == Some((pod.player, pile.kind)) && !pile.fan.is_empty() {
                let len = pile.fan.len();
                let under = usize::try_from(pile.count).unwrap_or(usize::MAX);
                for (i, card) in pile.fan.iter().enumerate() {
                    let pose = slot.fan_pose(pile.kind, i, len, duel.hovered == Some(card.object));
                    out.push(Placement {
                        object: card.object,
                        slot: *slot,
                        position: pose.at,
                        lift: pose.lift,
                        tapped: false,
                        // A card the pointer has lifted out of a graveyard is
                        // not a permanent and has no keywords to draw, the
                        // same reason its corner is the empty one.
                        flying: false,
                        indestructible: false,
                        dome: None,
                        defender: false,
                        count: if i + 1 == len {
                            under.saturating_sub(len - 1).max(1)
                        } else {
                            1
                        },
                        badge: 0,
                        art: card.art,
                        offer: pile_offer(duel, card.object),
                        corner: baylee_client_core::cardplate::Corner::default(),
                        selected: duel
                            .interaction
                            .as_ref()
                            .is_some_and(|i| i.is_selected(card.object)),
                        fan: Some((pose, pile.kind)),
                        marks: 0,
                        sick: false,
                        crests: [None; cardcrest::MAX_CRESTS],
                        covered: false,
                        shown: true,
                        rung: 0.0,
                    });
                }
                continue;
            }

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
                flying: false,
                indestructible: false,
                dome: None,
                defender: false,
                count: usize::try_from(pile.count).unwrap_or(usize::MAX),
                badge: 0,
                art: pile.art,
                offer: pile_offer(duel, top),
                corner: baylee_client_core::cardplate::Corner::default(),
                selected: duel
                    .interaction
                    .as_ref()
                    .is_some_and(|i| i.is_selected(top)),
                fan: None,
                marks: 0,
                sick: false,
                crests: [None; cardcrest::MAX_CRESTS],
                covered: false,
                shown: true,
                rung: 0.0,
            });
        }
    }
    out
}

/// The offer drawn on a card lying in a pile, or lifted out of one by a
/// hover: [`crate::Duel::reach_of`], in the two lights it answers with.
///
/// Until #242 both pile sites passed `false` here, so nothing in a pile was
/// ever lit — not the Opt Snapcaster Mage had just made castable, and not a
/// commander standing in the command zone with the lands to pay for it.
fn pile_offer(duel: &Duel, object: ObjectId) -> crate::cardmat::Offer {
    let reach = duel.reach_of(object);
    crate::cardmat::Offer::on(
        duel.proposing(),
        &[object],
        reach == Some(crate::Reach::Offered),
    )
    .reaching(reach == Some(crate::Reach::Taps))
}

/// Brings the scene in line with the board model.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // the diff loop is one coherent pass
pub fn sync_scene(
    mut commands: Commands,
    time: Res<Time>,
    duel: Res<Duel>,
    mut index: ResMut<SceneIndex>,
    mut watch: ResMut<ZoneWatch>,
    mut textures: Option<ResMut<CardTextures>>,
    mut card_materials: ResMut<Assets<CardMaterial>>,
    // One parameter for the objects lying on and under a card: a system
    // takes sixteen.
    (mut strip_materials, mut badge_materials, mut floor_materials, mut shell_materials): CardCompanions<'_>,
    assets: Res<AssetServer>,
    texts: Res<crate::cardtext::CardTexts>,
    mode: Res<crate::face::FaceMode>,
    settings: Res<crate::settings::ClientSettings>,
    prefs: Res<crate::prefs::Prefs>,
    sheen: Res<crate::sheen::Sheen>,
    // The fonts, and what has arrived of them: a face is measured in one.
    (fonts, font_assets): (Option<Res<crate::hud::UiFonts>>, Option<Res<Assets<Font>>>),
    mut cards: Query<DrawnCard>,
) {
    let (Some(statics), Some(textures)) = (duel.statics.as_ref(), textures.as_mut()) else {
        return;
    };
    let Some(quad) = index.quad.clone() else {
        return;
    };
    let blank = index.blank.clone();
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

    // One original sleeve for the library, its fan and every hidden card.
    // Dress the resident material in place: the library's slabs spawn once
    // and are never visited again. Downloaded printing art cannot replace it.
    if !index.back_dressed
        && let Some(handle) = blank.as_ref()
        && let Some(mut material) = card_materials.get_mut(handle)
    {
        dress_in_the_back(&mut material, textures.procedural_back());
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
        for handle in index.marks_materials.values() {
            if let Some(mut material) = strip_materials.get_mut(handle) {
                material.params.motion = motion;
            }
        }
        for handle in index.floor_materials.values() {
            if let Some(mut material) = floor_materials.get_mut(handle) {
                material.params.motion = motion;
            }
        }
        for handle in index.shell_materials.values() {
            if let Some(mut material) = shell_materials.get_mut(handle) {
                material.params.motion = motion;
            }
        }
    }

    let wanted = placements(&duel);
    let mut live: HashSet<ObjectId> = HashSet::new();
    // What is standing in a fan this frame, and which pile it came out of.
    // Rebuilt every frame rather than kept, because a fan is open for exactly
    // as long as the pointer is on it and the answer is never carried over.
    let mut fanned: HashMap<ObjectId, Place> = HashMap::new();

    // The keyboard/mouse cursor. What is *chosen* rides on the placement,
    // because that is where a group's members are.
    let hovered = duel.hovered;

    // The snapshot the faces below were built from: rules text is projected,
    // so a face is only stale when the game state that produced it moved on.
    let seq = duel.view.as_ref().map_or(0, |v| v.seq);
    // What the faces below are measured with: the font they are set in, once
    // it has arrived.
    let widths = face::Widths::of(
        fonts
            .as_deref()
            .zip(font_assets.as_deref())
            .and_then(|(fonts, assets)| assets.get(&fonts.text)),
    );

    for placement in &wanted {
        live.insert(placement.object);

        // A card either wears its art or its own text, never both — text on
        // top of artwork is unreadable at any zoom.
        let show_face = face::wants_face(&mode, &settings, textures, placement.art);
        let object = duel
            .view
            .as_ref()
            .and_then(|view| view.object(placement.object));

        // What the card is physically: the finish is a property of the
        // printing, so it comes from the print table — which is per seat, and
        // a printing this seat has not earned reads as plain rather than as
        // a leak.
        let finish = crate::cardmat::finish_of(statics, placement.art);
        // The offer is what the player could do with the card — or has just
        // said they will — and is light on the felt round it; a Forest that
        // becomes tappable is lit, and stops being lit the moment priority
        // moves on.
        let glow = crate::cardmat::glow_of(object, placement.offer);

        // The face is fitted before the material is chosen: its name's lines
        // are the name bar's depth, which the material draws (#259), so the
        // two come out of one fitting. A face fitted before the font arrived
        // is fitted again when it does — the average's widths are a
        // stand-in, and the font's may put the same name on the other number
        // of lines.
        let face_now = if show_face {
            face_now(index.faces.get(&placement.object), seq, &widths, || {
                object.map(|object| face::of_object(object, None, &texts))
            })
        } else {
            FaceNow::Unbuilt
        };

        let material = if show_face {
            // One material per colour identity and name depth, so a
            // mono-green board is one material however many creatures are on
            // it.
            let look = face_look(object, face_now.lines(), finish);
            if let Some(handle) = index.face_materials.get(&look) {
                handle.clone()
            } else {
                let tint = face::table_color(object.map_or(ColorSet::EMPTY, |o| o.colors));
                let handle = card_materials.add(material(look, None, tint, motion));
                index.face_materials.insert(look, handle.clone());
                handle
            }
        } else {
            // One material per look, created on first use.
            match placement.art {
                Some(key) => {
                    let look = CardLook::art(key, finish)
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
        // A creature with flying stands off the felt. It goes in *here*, with
        // the row's rise and the deck under it, so it reaches the card
        // through `Motion` and `glide` like every other reason a card is
        // where it is — an offset added to the transform after the glide
        // would be fought by the glide on the next frame and compound.
        //
        let float = float_of(
            placement,
            placement.selected
                || placement.offer.armed
                || hovered == Some(placement.object)
                || still,
            time.elapsed_secs_wrapped(),
        );
        let mut transform = card_transform(
            &placement.slot,
            placement.position,
            placement.tapped,
            placement.lift + deck + float,
        );
        if placement.flying
            && !still
            && !placement.selected
            && !placement.offer.armed
            && hovered != Some(placement.object)
        {
            let phase = airborne::phase(placement.object) * std::f32::consts::TAU;
            let t = time.elapsed_secs_wrapped();
            // Subtle bank and pitch, through the regular glide.
            transform.rotation *= Quat::from_rotation_x((t * 1.15 + phase).sin() * 0.012)
                * Quat::from_rotation_y((t * 0.83 + phase).cos() * 0.016);
        }
        // A card in a fan is tipped up and turned; everything else about it —
        // where it stands, the deck under it, the hover lift below — is the
        // same arithmetic every other card gets.
        if let Some((pose, kind)) = placement.fan {
            transform.rotation = fan_rotation(&placement.slot, pose);
            if let Some(place) = place_of(kind, placement.slot.player) {
                fanned.insert(placement.object, place);
            }
        }
        // Hover (cursor) lifts the card a touch; a chosen card stays raised
        // until the choice is answered, and so does an armed one — a deed
        // waiting on a second tap is a commitment the player has already
        // made, which is the same claim being selected makes and belongs at
        // the same height. Selected wins over both; the pointer moving away
        // must not put an armed card back down.
        // Where the card stands with nothing touching it, kept before the two
        // branches below add what is. See [`CardRest`].
        let resting = transform;
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
            if let Ok((
                mut motion,
                mut visual,
                mut current_material,
                airborne,
                mut rest,
                mut seen,
            )) = cards.get_mut(entity)
            {
                // A card a scrolled row does not show is not drawn, and
                // everything lying on it or under it goes with it.
                seen.set_if_neq(shown_as(placement.shown));
                if motion.target != transform {
                    motion.target = transform;
                }
                if rest.0 != resting {
                    rest.0 = resting;
                }
                if visual.count != placement.count {
                    visual.count = placement.count;
                }
                if current_material.0 != material {
                    current_material.0 = material;
                }
                // A creature can gain flying and lose it again — an anthem
                // resolving, an aura leaving — so this is a diff like the
                // others and not a property of the entity. Compared first:
                // an unconditional insert every frame would be an archetype
                // move every frame for every flier on the table.
                if airborne != placement.flying {
                    if placement.flying {
                        commands.entity(entity).insert(Floating);
                    } else {
                        commands.entity(entity).remove::<Floating>();
                    }
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
                            // A card the fan is lifting comes out of its own
                            // pile, which is where it has been lying all
                            // along. Asked first, because `moves` has nothing
                            // to say about a card that has not moved and the
                            // generic entrance would drop it out of the air
                            // onto a mark it is supposed to have risen to.
                            placement
                                .fan
                                .and_then(|(_, kind)| place_of(kind, placement.slot.player))
                                .or_else(|| {
                                    moves
                                        .iter()
                                        .find(|m| m.object == placement.object)
                                        .and_then(|m| m.from)
                                }),
                        ),
                        &transform,
                    ),
                    Motion { target: transform },
                    CardRest(resting),
                    shown_as(placement.shown),
                ))
                .id();
            index.cards.insert(placement.object, entity);
            if placement.flying {
                commands.entity(entity).insert(Floating);
            }

            entity
        };

        // What the strip says. The plate goes on it only where the print
        // cannot say it (`Corner::shows_plate`): a card showing its text
        // face has no printed box, and one showing its art has, unless the
        // next card of the row lies over it.
        let print = !show_face && placement.art.is_some();
        let corner = placement.corner;
        let strip = cardrail::Strip::new(
            placement.marks,
            corner
                .shows_plate(print, placement.covered)
                .then_some(corner),
            placement.sick,
            placement.crests,
        );
        sync_strip(
            &mut commands,
            &mut index,
            &mut strip_materials,
            entity,
            (placement.object, strip, placement.rung),
            motion,
        );
        sync_badge(
            &mut commands,
            &mut index,
            &mut badge_materials,
            entity,
            placement,
        );
        sync_floor(
            &mut commands,
            &mut index,
            &mut floor_materials,
            entity,
            placement,
            glow,
            motion,
        );
        sync_stack(
            &mut commands,
            &mut index,
            &mut card_materials,
            entity,
            placement,
            motion,
        );
        sync_shell(
            &mut commands,
            &mut index,
            &mut shell_materials,
            entity,
            placement,
            motion,
        );

        // The text children follow the same decision as the material, and are
        // rebuilt when the snapshot they were made from is no longer current:
        // an anthem, a counter or a clone all change what the face should say.
        if let FaceNow::Kept(_) = face_now {
            continue;
        }
        if let Some(previous) = index.faces.remove(&placement.object) {
            for text in previous.texts {
                commands.entity(text).despawn();
            }
        }
        let (FaceNow::Fitted(fitted), Some(fonts)) = (face_now, fonts.as_deref()) else {
            continue;
        };
        let (built, fit) = *fitted;
        let spawned = face::spawn_world(
            &mut commands,
            entity,
            &built,
            &fit,
            face_word(object, fit.lines()),
            placement.corner.plate,
            fonts,
        );
        index.faces.insert(
            placement.object,
            ShownFace {
                seq,
                measured: fit.measured,
                lines: fit.lines(),
                texts: spawned,
            },
        );
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
            // into the graveyard rather than a blank one. The strip, the
            // badge and the light on the felt are children in the same way.
            index.faces.remove(&id);
            index.marks.remove(&id);
            index.badges.remove(&id);
            index.floors.remove(&id);
            index.stacks.remove(&id);
            // Not the shell: indestructible means nothing off the
            // battlefield, and a rim flying off with its card would no
            // longer be fitted to anything on the way.
            if let Some(shell) = index.shells.remove(&id) {
                shell.despawn(&mut commands);
            }
            // A stale id with no move behind it did not leave anywhere: it is
            // a graveyard's old top card, covered by the one that landed on
            // it this frame, or a group that re-keyed when its lowest-id
            // member went. Nothing about the table changed where it stands,
            // so it goes at once, as it always did — an exit played for one
            // of those would be a card visibly sliding out from under a pile
            // it never left.
            let Some(step) = moves.iter().find(|m| m.object == id).copied() else {
                // Unless it is a card the fan had in the air a frame ago. It
                // has not left anywhere — the pointer left *it* — so it takes
                // the pile-bound exit with no door on it, which is the glide
                // that puts it back under the pile's top card. A card that
                // moved zones on the same frame never reaches here: the move
                // below is the truer answer and takes precedence.
                if let Some(&home) = index.fanned.get(&id) {
                    if let Ok((mut motion, ..)) = cards.get_mut(entity) {
                        motion.target =
                            exit(Some(home), pile_stand(&duel, Some(home)), &motion.target);
                    }
                    commands
                        .entity(entity)
                        .remove::<CardVisual>()
                        .insert((Pickable::IGNORE, Departing { left: EXIT_LIFE }));
                    continue;
                }
                commands.entity(entity).despawn();
                continue;
            };
            let to = step.to;
            if let Ok((mut motion, _, mut worn, ..)) = cards.get_mut(entity) {
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

    // After the stale pass and not before it: what this frame fanned is next
    // frame's answer to "was that card in the air".
    index.fanned = fanned;

    // Tell the cache what is on screen so the next fetch evicts something
    // else. The placements are the table's own cards; `required_images` is
    // the board model's whole answer — the hand, the stack and what it points
    // at, a pile's top card and the fan a hover spreads out of it, the
    // cardboard under a copy — and none of those is on the table, so none of
    // them was being touched at all. The zone dialog's rows are the one thing
    // neither list holds, and `hud::tray` touches those itself.
    let mut visible: Vec<ImageKey> = wanted.iter().filter_map(|p| p.art).collect();
    if let Some(board) = duel.board.as_ref() {
        visible.extend(board.required_images());
    }
    textures.touch_visible(&visible);
}

#[cfg(test)]
mod camera_tests;

#[cfg(test)]
mod zone_tests;

#[cfg(test)]
mod tests;

/// The ways off the table and the ways back onto it.
///
/// Geometry, not implementation: what is asserted is that a card bound for a
/// pile ends up *at that pile* and behind the card standing on it, that a card
/// bound nowhere ends up at nothing, and that an arrival from a pile is the
/// exit to it run backwards. Each of them has a counter-arm as well — a
/// version that gave every zone the same pose would pass none of these.
#[cfg(test)]
mod exit_tests;

/// Combat, as the table draws it.
///
/// The board here is built by hand rather than through a view: what is under
/// test is the *bridge* between the interaction state and the placements, and
/// a `PlayerView` in the middle would put the whole projection between the
/// thing being asserted and the thing being set.
#[cfg(test)]
mod combat_tests;

/// What the fan is worth on the screen it is drawn on.
///
/// Every other test about the fan is in table space, where it is easy to be
/// right and wrong at the same time: the first version of this fan rose 1.5
/// units and stepped 0.72, which looks generous written down and drew seven
/// cards inside forty-two pixels. Height and distance **cancel** at this
/// camera — raising a card moves it up the screen and bringing it towards the
/// viewer moves it down — so the only honest measure is the projection, and
/// this is the only test that takes it.
#[cfg(test)]
mod fan_screen_tests;

/// The one fan that is not made of cards.
///
/// [`sync_library_fan`] is *run* here rather than called, because the thing
/// most likely to be wrong about it is not its arithmetic but whether it is
/// wired at all — a system that is written and never scheduled draws nothing
/// and fails no test that only calls its parts.
#[cfg(test)]
mod library_fan_tests;

/// The offer drawn on a card, which is the join nothing crossed: `Offer::on`
/// is unit-tested in `cardmat` and `owed_plan` in `owed_tests`, and the line
/// that hands one to the other is here.
#[cfg(test)]
mod offer_tests;

/// The camera is the table's, and the one thing that takes it is a player
/// asking to look at a single seat.
///
/// These run the real system in an `App` rather than calling the arithmetic,
/// because the bug they are about was never in the arithmetic:
/// [`frame_table`] computed the right shot every time and had stopped being
/// allowed to write it.
///
/// What the harness registers is what can still reach the rig from the input
/// set, and that is now **nothing**. `input::camera_controls` was deleted on
/// 14.09.2026 at the owner's word — *„Generelles Camera Movement kann weg
/// (also nicht nur die Maus Controls, sondern auch die Keyboard Controls)"* —
/// so every gesture below is written into an app with no reader for it, and
/// what these tests hold is the other half: the framing keeps the shot through
/// all of it, and gives it up only where it is asked to.
#[cfg(test)]
mod framing_tests;

#[cfg(test)]
mod badge_tests;
#[cfg(test)]
mod face_tests;
/// What flying does to a card on the table, and to the shadow under it.
///
/// The height itself is [`baylee_client_core::airborne`]'s and is tested
/// there. What is tested here is the two halves the renderer owns: that the
/// height is asked for at all and only for the right cards, and that the
/// shadow is left behind on the felt when the card takes off.
#[cfg(test)]
mod flying_tests;
/// The shell round an indestructible permanent, on real tables and from the
/// real camera ([`shellmat`]).
#[cfg(test)]
mod shell_tests;
#[cfg(test)]
mod stack_tests;
#[cfg(test)]
mod strip_tests;
