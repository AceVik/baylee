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
use crate::platemat::{PlateMaterial, PlateWords};
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
/// beats the widest table: the rungs are half a thousandth apart, so
/// 0.0005 × 400 000 = 200. That was a comfortable margin while the eight-seat
/// ring reached about forty units across. Since every ring seat is as wide
/// as a duel's (#264), eight seats on a phone turned on its side reach about
/// 190, and 200 still beats it, though no longer by much. A larger gain is
/// no way out: the keys are `f32`, and at a few million two things of one
/// rung a tenth of a unit apart would tie. `the_ladder_decides_what_covers_what`
/// held both halves of that until it went with the air (d368cb56); nothing
/// but this arithmetic holds it now.
///
/// It touches nothing but the sort. Depth *writing* is off for every blended
/// surface here but defender's wall, which is solid and goes first
/// ([`shellmat::WALL_RUNG`]), so none of the rest ever occludes another; what
/// this fixes is purely which one is painted last.
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

mod camera;
mod card_marks;
mod index;
mod mats;
mod motion;
mod piles;
mod placement;
mod scene;
mod shells;
mod stage;

pub use camera::*;
pub use card_marks::*;
pub use index::*;
pub use mats::*;
pub use motion::*;
pub use piles::*;
pub(crate) use placement::*;
pub use scene::*;
pub use shells::*;
pub use stage::*;

#[cfg(test)]
mod camera_tests;

#[cfg(test)]
mod visit_tests;

#[cfg(test)]
mod zone_tests;

#[cfg(test)]
mod tests;

/// `glide` writes nothing once every card is on its mark.
#[cfg(test)]
mod glide_tests;

/// The arrangements as the camera draws them (DESIGN-v8 §3).
#[cfg(test)]
mod arrangement_tests;

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
/// The plate at a card's bottom right, upright under a tapped card.
#[cfg(test)]
mod plate_tests;
#[cfg(test)]
mod shell_tests;
#[cfg(test)]
mod stack_tests;
#[cfg(test)]
mod strip_tests;
#[cfg(test)]
mod tuck_tests;
