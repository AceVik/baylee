//! The table's own artwork, generated rather than shipped.
//!
//! Everything a player sees *under* the cards — the felt, the glow beneath
//! a seat's zone — is computed here
//! into plain RGBA8 buffers. Three reasons it is done this way and not with
//! image files:
//!
//! - **Nothing to license.** `docs/legal.md` §2 rules out `WotC` assets, and a
//!   fantasy table wants exactly the kind of ornament that is easiest to
//!   accidentally borrow. Arithmetic borrows nothing.
//! - **Nothing to ship.** A 1024² felt is a megabyte and a half on disk and
//!   about four milliseconds to compute, and the wasm build already fights
//!   for every byte.
//! - **Everything is testable.** These are pure functions over a pixel
//!   buffer, so the renderer-free crate can hold them and assert what they
//!   produce without a GPU anywhere in sight.
//!
//! The noise is a hashed value-noise fbm with a fixed seed — no `rand`, no
//! clock — so two players at one table see the same grain in the same place,
//! which matters the moment anyone screenshots anything.

use std::f32::consts::{PI, TAU};

use glam::Vec2;

/// A generated image: RGBA8, `width * height * 4` bytes, row-major from the
/// top-left.
#[derive(Clone, Debug)]
pub struct Texture {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Straight (non-premultiplied) RGBA8 samples.
    pub rgba: Vec<u8>,
}

impl Texture {
    /// A transparent image of the given size.
    fn blank(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    /// Writes one pixel. Out-of-range coordinates are ignored, so a caller
    /// may draw shapes that run off the edge without clamping first.
    fn put(&mut self, x: u32, y: u32, rgba: [f32; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let at = ((y as usize) * (self.width as usize) + x as usize) * 4;
        for (channel, value) in rgba.iter().enumerate() {
            self.rgba[at + channel] = to_byte(*value);
        }
    }

    /// Reads one pixel back as floats. Tests use this; the renderer never
    /// does.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> [f32; 4] {
        let at = ((y as usize) * (self.width as usize) + x as usize) * 4;
        [
            f32::from(self.rgba[at]) / 255.0,
            f32::from(self.rgba[at + 1]) / 255.0,
            f32::from(self.rgba[at + 2]) / 255.0,
            f32::from(self.rgba[at + 3]) / 255.0,
        ]
    }
}

/// Clamps and quantises one channel.
fn to_byte(value: f32) -> u8 {
    // `clamp` first: fbm can overshoot slightly and a wrapped cast would
    // turn a highlight into a black speck.
    (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// A deterministic hash of a lattice point to `0.0..1.0`.
///
/// Integer arithmetic on purpose — the same value on every platform, which a
/// float hash built from `sin` is not.
fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d)
        ^ (y as u32).wrapping_mul(0x1656_67b1)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x2974_5c65);
    h ^= h >> 15;
    f32::from(h as u16) / f32::from(u16::MAX)
}

/// Smoothstep, the usual `3t² − 2t³`.
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Value noise on the unit lattice, bilinear with a smoothed interpolant.
fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (smooth(x - x0), smooth(y - y0));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "lattice coordinates are small by construction"
    )]
    let (ix, iy) = (x0 as i32, y0 as i32);
    let c00 = hash(ix, iy, seed);
    let c10 = hash(ix + 1, iy, seed);
    let c01 = hash(ix, iy + 1, seed);
    let c11 = hash(ix + 1, iy + 1, seed);
    let top = c00 + (c10 - c00) * fx;
    let bottom = c01 + (c11 - c01) * fx;
    top + (bottom - top) * fy
}

/// Fractal noise: octaves of [`value_noise`] at halving amplitude, normalised
/// back to `0.0..1.0`.
fn fbm(x: f32, y: f32, seed: u32, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut amplitude = 1.0;
    let mut total = 0.0;
    let mut frequency = 1.0;
    for octave in 0..octaves {
        sum += value_noise(x * frequency, y * frequency, seed + octave) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    sum / total
}

/// Linear blend between two colours.
fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// The five colours of the pie, in the order they sit on the wheel.
///
/// Chosen to read as the colours a player already thinks in while staying
/// legible against a dark table: the white is a warm parchment rather than a
/// glare, the black a cold slate rather than a hole.
pub const PIE: [[f32; 3]; 5] = [
    [0.94, 0.91, 0.80], // white — parchment
    [0.29, 0.55, 0.83], // blue  — deep water
    [0.30, 0.27, 0.34], // black — slate
    [0.83, 0.36, 0.28], // red   — ember
    [0.36, 0.66, 0.42], // green — moss
];

/// The mineral cloth in shadow.
pub const FELT_DEEP: [f32; 3] = [0.043, 0.072, 0.080];
/// Midnight mineral cloth: cool, but quieter than a card's colour identity.
pub const FELT_CLOTH: [f32; 3] = [0.120, 0.188, 0.204];
/// Where the table has been leaned on and dealt across for years.
pub const FELT_WORN: [f32; 3] = [0.165, 0.245, 0.258];
/// The aged metal rail, in the shade.
pub const RAIL_HIDE: [f32; 3] = [0.100, 0.084, 0.064];
/// The machined edge, where the light sits on it.
pub const RAIL_LIP: [f32; 3] = [0.300, 0.244, 0.157];
/// The apron: the wall of the slab, below the rail.
///
/// Darker than either, and that is the whole job. The stage carries no light
/// (see the module header on the client's `feltmat`), so a side face cannot
/// be shaded by one — the only thing that says "this table has a thickness"
/// is that its wall is a *different, darker* colour than its top.
pub const APRON: [f32; 3] = [0.040, 0.035, 0.029];

/// How wide the padded rail runs, in table units — under a card width.
///
/// It is exactly `SLAB_MARGIN - AIR` in the client's camera: the framing fits
/// the layout plus `AIR` of table, the slab is cut to the layout plus its own
/// margin, and the ring between the two is this. So the rail is precisely the
/// part of the table the camera keeps outside the play area, which is what a
/// rail is.
pub const RAIL_WIDTH: f32 = 0.55;

/// The corner radius of a table this size: a racetrack, not a rectangle.
///
/// Two jobs. A casino table is an oval, so this is what makes the surface
/// read as one; and the corners it takes away are the only part of the
/// window the felt does not reach, which is where the sky behind the table
/// is seen. A rectangle framed the way this one is fills the screen edge to
/// edge, and a backdrop nothing ever shows is a backdrop worth nothing.
#[must_use]
pub fn table_corner(span: Vec2) -> f32 {
    span.min_element() * 0.065
}

/// The felt: casino baize, rough and woven, worn lighter towards the middle
/// where the game happens and falling into shadow at the rail.
///
/// It is deliberately low-contrast. Everything above it — cards, zone rims,
/// the firewheel — has to stay the thing the eye lands on, and a table that
/// competes with its own cards is a table nobody can read.
///
/// This is the **reference**, not the surface: the table is drawn by
/// `felt.wgsl`, which cannot sample a texture big enough to stay sharp at
/// this camera. The same arithmetic lives here so a test can measure it, and
/// `table::shader_tests` reads the constants above out of the shader and
/// fails when the two drift apart.
#[must_use]
pub fn felt(size: u32) -> Texture {
    const DEEP: [f32; 3] = FELT_DEEP;
    const CLOTH: [f32; 3] = FELT_CLOTH;
    const WORN: [f32; 3] = FELT_WORN;

    let mut texture = Texture::blank(size, size);
    let extent = size as f32;
    for y in 0..size {
        for x in 0..size {
            let (u, v) = (x as f32 / extent, y as f32 / extent);
            // Distance from the centre, 0 in the middle and 1 at the edge of
            // the inscribed circle.
            let radius = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 2.0;

            // Big soft blotches of wear, then fine grain on top.
            let wear = fbm(u * 3.0, v * 3.0, 0x51ed, 4);
            let grain = fbm(u * 140.0, v * 140.0, 0x9a17, 2);
            // The weave: two fine sine ridges crossing at right angles. Kept
            // very shallow — at table distance it should read as texture,
            // never as stripes.
            let weave = ((u * extent * PI * 0.5).sin() * (v * extent * PI * 0.5).sin()) * 0.5 + 0.5;

            let mut colour = mix(CLOTH, WORN, wear.powf(1.6));
            // The vignette starts late and arrives slowly: at `radius * 1.05`
            // it reached DEEP a twentieth of the way inside the inscribed
            // circle, which is most of what a leaning camera actually has on
            // screen — so the table was in shadow everywhere a player looks
            // and lit only in a spot behind the far seat.
            colour = mix(colour, DEEP, (radius * 0.88).clamp(0.0, 1.0).powf(1.6));
            let lift = (grain - 0.5).mul_add(0.028, (weave - 0.5) * 0.010);
            for channel in &mut colour {
                *channel += lift;
            }
            texture.put(x, y, [colour[0], colour[1], colour[2], 1.0]);
        }
    }
    texture
}

/// A sheet of parchment: the surface everything a player *reads* is written
/// on.
///
/// Felt is the ground, parchment is a sheet, brass draws lines, and gold
/// belongs to the local seat — four materials with one job each, so a panel
/// says what kind of thing it is before a word on it has been read. A dialog
/// is a place you work and a sheet is a thing you read, and the prompt is the
/// second: it asks a question and offers the two or three answers to it.
///
/// Stretched rather than tiled, so it is drawn as **one sheet** whatever size
/// the panel is: the blotches are soft enough to survive being pulled about,
/// and a tiled grain would put a seam down the middle of a slip that is four
/// hundred pixels wide and eighty tall.
///
/// Opaque, and that is the point of using a sheet at all. The panels this
/// replaces were 88% black over the table, so a question was read through
/// whatever card happened to be under it.
#[must_use]
pub fn parchment(size: u32) -> Texture {
    /// The sheet where it has been handled least.
    ///
    /// The same parchment the card shader draws a saga's page with
    /// (`shaders/card_common.wgsl`), and the same one
    /// `baylee_client::hud::palette::PARCHMENT` is painted flat with. The
    /// three were three
    /// colours until now: the UI's #EDE3CC and the card's #E0D4B0 for one
    /// material, with the *small* surface the darker of the two — which is
    /// backwards, because a colour field the size of a fingernail already
    /// reads greyer and darker than a sheet of paper does. The card's is
    /// what everything took, so a sheet of parchment is one thing wherever
    /// it is drawn; `the_parchment_is_the_same_paper_in_both_languages`
    /// reads it back out of the WGSL.
    const SHEET: [f32; 3] = [0.880, 0.830, 0.690];
    /// Where it has aged: warmer and a shade down.
    ///
    /// Moved with the sheet by the *ratio* it stood at rather than by the
    /// difference, because a stop is multiplicative: keeping the old offset
    /// would have left the staining nearly as dark against a sheet that had
    /// come down to meet it, and the mottle that stops a flat fill from
    /// reading as a rectangle of paint would have gone with it.
    const AGED: [f32; 3] = [0.795, 0.727, 0.565];
    /// The rim, where a sheet lying on a table loses the light.
    const EDGE: [f32; 3] = [0.669, 0.596, 0.433];

    let mut texture = Texture::blank(size, size);
    let extent = size as f32;
    for y in 0..size {
        for x in 0..size {
            let (u, v) = (x as f32 / extent, y as f32 / extent);
            // Two scales of age — broad staining, then the mottle inside it —
            // and a fine tooth on top, which is what stops a flat fill from
            // reading as a rectangle of paint.
            let stain = fbm(u * 2.4, v * 2.4, 0x7c31, 4);
            let mottle = fbm(u * 9.0, v * 9.0, 0x2ab9, 3);
            let tooth = fbm(u * 190.0, v * 190.0, 0x64d5, 2);
            // Fibres, drawn out along the sheet: the same noise sampled
            // twenty times wider than it is tall.
            let fibre = fbm(u * 3.0, v * 60.0, 0x1f08, 2);

            let mut colour = mix(SHEET, AGED, (stain * 0.72 + mottle * 0.28).powf(1.4));
            // Away from the middle on both axes, and squared so the fall is
            // slow until it is near the edge. `max` rather than a radius: a
            // slip is far wider than it is tall, and a round vignette
            // stretched over one darkens its ends and nothing else.
            //
            // Shallow, and that is a correction: at 0.85 the sheet was most
            // of a stop darker than the flat [`PARCHMENT`] the node behind it
            // is painted with, and a UI image covers a node's *content* box
            // and not its padding — so the slip came out as a pale frame with
            // a visibly darker sheet inside it. A sheet is one piece of
            // paper, so the difference between its middle and its edge has to
            // stay under what a seam would show.
            let off = ((u - 0.5).abs().max((v - 0.5).abs()) * 2.0).clamp(0.0, 1.0);
            colour = mix(colour, EDGE, off.powi(3) * 0.30);
            let lift = (tooth - 0.5).mul_add(0.030, (fibre - 0.5) * 0.014);
            for channel in &mut colour {
                *channel = (*channel + lift).clamp(0.0, 1.0);
            }
            texture.put(x, y, [colour[0], colour[1], colour[2], 1.0]);
        }
    }
    texture
}

/// A seat's mat: how round its corners are, in **table units**.
///
/// A length rather than a fraction, which is what makes it answerable at all.
/// [`seat_mat`] took `radius` as a fraction of the shorter side of a *texture*
/// that was then stretched over a mat, so the only way to say how round a
/// corner came out was to work back through the image's size: 6% of 256
/// texels over a board 13.1 units wide and 6.05 deep is about **0.37 units**.
/// Nobody reading the call could have told you that, and the number the owner
/// was looking at is the one nobody could see.
///
/// Half of it, because a mat is a *playing surface*: the more radius a
/// rectangle carries the more it reads as a control rather than as ground.
/// Enough curve that a corner is not a spike, and no more.
pub const MAT_CORNER: f32 = 0.18;

/// How far in from a mat's edge its coloured rim runs, in table units.
///
/// Held at what it was — the old 1.8% of 256 texels came out at about 0.11
/// units — because the rim is not what was too thick. That was the table's
/// rail, and it is [`RAIL_WIDTH`]. This is the one part of a mat meant to be
/// read from the far side of the table, and it is already a hairline there.
pub const MAT_RIM: f32 = 0.11;

/// The corner has to be wider than the rim, or the rim turns back on itself
/// at every corner and the mat grows four bright blobs.
const _: () = assert!(MAT_CORNER > MAT_RIM);
/// And the rim has to be thick enough to have a colour at all: it is how a
/// seat is named from across the table, and below about a tenth of a unit it
/// is a single pixel at this camera and reads as an artefact.
const _: () = assert!(MAT_RIM > 0.08);

/// How much of white each of a mat's three lanes is veiled with, from the
/// lane nearest the middle of the table outwards.
///
/// An alpha over the felt, and therefore linear light — which is the whole
/// reason these are as small as they are. See the long note inside
/// [`seat_mat`] about the quarter they were cut to.
pub const MAT_LANES: [f32; 3] = [0.0135, 0.0105, 0.0080];

/// The shelf along one long edge of a mat that the seat's bar is written
/// on, in table units — 0.72 of a card's height.
///
/// It is a fourth band on the mat rather than a panel floating over the
/// felt, and that is the whole of the design: the mat's rim runs round the
/// ledge and the lanes together, so the seat's colour and the on-turn breath
/// frame the bar on three sides without a single extra pixel being drawn,
/// and a bar written on a seat's own ground cannot be mistaken for a bar
/// belonging to the table.
///
/// It is added to [`crate::layout::POD_DEPTH`] rather than taken out of it:
/// three lanes are still exactly a card tall each, because the ledge is
/// furniture and a lane is where a card stands.
///
/// It was 0.95, which is one row of ink deep. It is 1.00 because the bar the
/// owner asked for is **two** rows — the twelve steps alone along the mat's
/// top edge and the seat's identity beneath them
/// ([`crate::seatbar::Density::Split`]) — and at the reference window the
/// shallower of a duel's two shelves projected 34.5 px at 0.95 against the
/// 34 of ink two rows draw, which is no margin at all. It projects 36.2 at
/// 1.00. Measured rather than reasoned:
/// `table::camera_tests::a_duel_is_written_on_two_rows` is that number.
///
/// Both of those numbers were **short by a printed border**, and the deeper
/// ledge was bought under them. The shelf a bar is hung on is
/// `MAT_MARGIN + MAT_LEDGE` — see [`LEDGE_FRAC`] — and the same shallower
/// duel shelf projects 55.0 px, not 36.2. So the margin that reading found
/// so tight was never that tight, and this constant would probably have been
/// left at 0.95 had anyone been able to see it. It is not moved back: 1.00 is
/// what the tables above were tuned at, the ring ceiling below is what binds
/// it either way, and a shelf is furniture whose depth is a look and not an
/// arithmetic. What is worth knowing is that the reason it moved was a
/// measurement, and the measurement was of the wrong rectangle.
///
/// Two things about the number are worth writing down, because both are the
/// opposite of what they look like.
///
/// **Deepening the ledge costs a duel no board size.** The pod grows with
/// it, the camera frames the pod, and a duel's shelf projects *longer* at
/// 1.00 than it did at 0.95 (1121 → 1125 px). A duel is framed by its width,
/// and the depth the shelf gained was depth the window had going spare —
/// which is a fact about a duel and not about the constant. Three seats and
/// up sit on a rounder ring where the depth is what the camera is fitting,
/// so there the same 0.85% of extra pod is about that much less board. It is
/// small either way; it is not nothing.
///
/// **What stops it is `layout::MAX_RING_Y`, not the assertion
/// below.** A ring of two *sides* — a 2v2, partners shoulder to shoulder —
/// is the deepest table there is for its width, and it reaches that ceiling
/// at a ledge of about 1.01: past that the ring is clamped, the table comes
/// out 2.23 wide to 1 instead of 1.78, and `layout::tests` finds the two
/// partners overlapping. So the ceiling on this constant is a fact about
/// what the camera can frame, and the shelf never got near the "is it a
/// fourth lane" bound it was written against.
pub const MAT_LEDGE: f32 = 1.00;

/// The shelf has to clear the rim on both sides with something left in the
/// middle, or the bar is written on its own border.
const _: () = assert!(MAT_LEDGE > MAT_RIM * 4.0);

/// And it has to stay a shelf rather than become a fourth lane.
///
/// Measured against a **card** rather than against a lane, which is the
/// tighter and the truer of the two: what makes a band read as a row is that
/// a card would sit on it, not what fraction of the lane beside it the band
/// happens to be. A lane carries a card plus air, so a bound of
/// three-quarters of a *lane* lets the shelf grow to nine tenths of a card.
///
/// The owner authorised moving this fraction to buy the two-row bar, and it
/// did not have to move: the ring ceiling above binds first, at about 1.01,
/// and 0.75 of a card is 1.048. The bound is left where it was because it is
/// still the one that says what a shelf *is*, and a reader who finds it
/// slack should not conclude the rule was abandoned — it was simply not the
/// rule that ran out.
const _: () = assert!(MAT_LEDGE < crate::layout::CARD_HEIGHT * 0.75);

/// How much of white the ledge is veiled with, on the same scale as
/// [`MAT_LANES`].
///
/// One shade *below* `MAT_LANES[2]`, the quietest lane, and deliberately: the
/// ink written on the ledge is the brightest thing on a mat, and a shelf that
/// competed with it would be a panel. `table::shader_tests` has the contrast
/// this leaves, bounded on both sides.
pub const MAT_LEDGE_VALUE: f32 = 0.0060;

/// A shelf brighter than the quietest lane is a panel, and one at zero is a
/// strip of bare table with a rim round it.
const _: () = assert!(MAT_LEDGE_VALUE < MAT_LANES[2] && MAT_LEDGE_VALUE > 0.0);

/// How much wider than the playing extent a mat is drawn, on every side.
///
/// A mat is a table the cards sit on and not a box drawn tight around them,
/// so the quad is [`crate::layout::SeatSlot::half_extent`] plus this on all
/// four sides — a printed border, which is exactly what the margin round a
/// real playmat is.
///
/// It lives here, beside the bands, rather than in the renderer that draws
/// the quad, and that is the whole point of moving it. It used to be
/// `table::ZONE_MARGIN` and nothing outside the renderer could see it, so
/// the bands below were fractions of [`crate::layout::POD_DEPTH`] laid over
/// a quad that is `2 · MAT_MARGIN` deeper — every band stretched by 18.5%,
/// the shelf 0.46 units out of place, and the lane seams a fifth of a unit
/// off the rows they are supposed to fence. Two modules measuring two
/// different rectangles is not a thing a comment can hold together; there is
/// one rectangle now and this is the constant that says how big it is.
pub const MAT_MARGIN: f32 = 0.55;

/// The depth of a mat **as it is drawn**: the playing extent plus its border
/// on both sides.
///
/// Every fraction below is over this and not over
/// [`crate::layout::POD_DEPTH`], because a fraction of the wrong rectangle is
/// a band in the wrong place.
pub const MAT_DRAWN_DEPTH: f32 = crate::layout::POD_DEPTH + MAT_MARGIN * 2.0;

/// How much of the drawn mat the border takes at one end, as a fraction.
pub const MARGIN_FRAC: f32 = MAT_MARGIN / MAT_DRAWN_DEPTH;

/// How much of the drawn mat the shelf takes, as a fraction.
///
/// Which end it takes it from is
/// [`LEDGE_IS_OUTER`](crate::layout::LEDGE_IS_OUTER)'s to say, and the three
/// lanes fill what is left from the centre-facing edge outwards either way.
///
/// The border at the shelf's own end is part of it. The two are contiguous,
/// nothing stands on either, and the alternative is a stripe of creature
/// lane painted outside the shelf at the very edge of the mat — which says
/// a card could stand there. So the shelf a bar is written on is
/// `MAT_MARGIN + MAT_LEDGE` deep, and
/// [`SeatSlot::ledge_corners`](crate::layout::SeatSlot::ledge_corners)
/// returns that same rectangle.
///
/// Derived rather than written down, so the shelf cannot end up a different
/// size in the shader than in the geometry that reserved room for it.
pub const LEDGE_FRAC: f32 = (MAT_MARGIN + MAT_LEDGE) / MAT_DRAWN_DEPTH;

/// How much of the drawn mat one lane takes, as a fraction.
///
/// A lane is exactly [`SeatSlot::lane_height`](crate::layout::SeatSlot) —
/// the playing extent less the shelf, in three — so a lane seam falls on the
/// boundary the layout puts the row of cards against. Splitting *what is
/// left* of the mat in three instead is what drifted them: the border at the
/// far end is not a lane, and dividing it in with them stretched each row by
/// a fifth of a unit more than the last.
pub const LANE_FRAC: f32 =
    (crate::layout::POD_DEPTH - MAT_LEDGE - crate::layout::STAGE_STEP) / (3.0 * MAT_DRAWN_DEPTH);

/// Extra room in front of creatures, reserved for their combat advance.
pub const COMBAT_FRAC: f32 = crate::layout::STAGE_STEP / MAT_DRAWN_DEPTH;

/// The shelf, the three lanes and the border at the far end are the whole
/// mat: a band left over is a band drawn in the wrong place.
const _: () = assert!({
    let sum = LEDGE_FRAC + COMBAT_FRAC + LANE_FRAC * 3.0 + MARGIN_FRAC;
    sum > 1.0 - 1e-6 && sum < 1.0 + 1e-6
});

/// How bright the hairline between two lanes is, on the same scale as
/// [`MAT_LANES`].
pub const MAT_SEAM: f32 = 0.036;

/// How much brighter the seam between the ledge and the creature lane is
/// than the seams between two lanes.
///
/// The lanes are three readings of one surface; the ledge is a different
/// kind of thing standing at its edge, and a seam of the same weight would
/// say it was a fourth lane.
pub const MAT_LEDGE_SEAM: f32 = 1.5;

/// How wide that hairline runs, as a fraction of the mat's depth.
pub const MAT_SEAM_WIDTH: f32 = 0.0075;

/// How much of white a mat's rim carries at its brightest.
pub const MAT_RIM_LIGHT: f32 = 0.62;

/// How hard the rim's opacity falls off across [`MAT_RIM`].
pub const MAT_RIM_FALL: f32 = 1.3;

/// How hard the rim's *hue* falls off across the same distance.
///
/// Shallower than [`MAT_RIM_FALL`] deliberately, so the accent reaches
/// further in than the ink does; the note at the bottom of [`seat_mat`] is
/// where that argument is made and what it looks like when the two are tied
/// together instead.
pub const MAT_HUE_FALL: f32 = 0.55;

/// A seat's mat: the rounded rectangle its permanents are played on.
///
/// **Nothing draws with this any more.** The renderer stopped stretching an
/// image over a mat and started drawing one — `baylee-client`'s
/// `shaders/mat.wgsl`, where the corner is a distance in table units and an
/// edge is one pixel wide however close the camera comes. What this is now is
/// that arithmetic in its readable form and its test bench: every number it
/// works from is a `MAT_*` constant above, and `table::shader_tests` fails if
/// the shader and these drift apart. The three tests below are the ones that
/// say what a mat *means* — only the rim carries the seat's colour, the
/// corners are cut, the seam sits between two lanes and not through one —
/// and none of them can be written against a GPU.
///
/// White, so the renderer can tint one texture per seat; the shape lives in
/// the alpha channel. Three bands run across it, one per lane, brightest at
/// the front where creatures stand — a player reading an opponent's board
/// should be able to see where the rows are without counting cards.
///
/// `radius` and `rim` are fractions of the shorter side. `accent` is the
/// seat's colour in **linear** RGB, and only the rim receives it.
///
/// That last parameter is the whole point of this signature. The mat used to
/// be written white-with-alpha — every pixel `[1, 1, 1]`, the rim told from
/// the field only by being more opaque — and the seat's colour was applied
/// as the material's `base_color`, which multiplies the *entire* texture. So
/// a mat did not have a coloured rim around neutral felt; it was one solid
/// sheet of the seat's colour, brighter at its edge. The local seat's gilt
/// turned its whole ground to brass, and the doc comment on the rim below
/// claimed a separation the code never made. Baking it here is what makes
/// that comment true: the field stays white and picks up only the neutral
/// brightness the material now carries, and the accent lives in the rim
/// alone.
///
/// The rim's colour and the rim's opacity are two curves, not one. Both are
/// driven by the same distance from the edge, but the hue crossfades on a
/// *shallower* exponent than the opacity does, so the accent reaches
/// further in than the ink does. Tying them together is the obvious thing
/// to write and it looks wrong: where a rim is faint it is also barely
/// coloured, so a seat's colour only ever arrives on the handful of pixels
/// that are already nearly opaque, and every rim reads as off-white.
#[must_use]
pub fn seat_mat(
    width: u32,
    height: u32,
    radius: f32,
    rim: f32,
    accent: [f32; 3],
    ledge_outer: bool,
) -> Texture {
    let mut texture = Texture::blank(width, height);
    let (w, h) = (width as f32, height as f32);
    let short = w.min(h);
    let (corner, edge) = (radius * short, (rim * short).max(1.0));
    for y in 0..height {
        for x in 0..width {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // Distance *outside* the rounded rectangle: zero within it, and
            // growing once past an edge or around a corner.
            let dx = (corner - px).max(px - (w - corner)).max(0.0);
            let dy = (corner - py).max(py - (h - corner)).max(0.0);
            let outside = (dx * dx + dy * dy).sqrt() - corner;
            if outside > 0.5 {
                continue;
            }
            // How far in from the rim, in pixels.
            let inset = -outside;

            // `v = 0` is the edge nearest the table centre, and the three
            // lanes always run from there outwards — the creature row first,
            // whichever end the shelf is at. Only the shelf moves: it takes
            // the near end for a seat drawn near the camera and the far one
            // for a seat across the table, so every bar is above the board it
            // describes on the screen somebody is looking at.
            //
            // The border the mat is drawn with is not a fourth band: at the
            // shelf's end it *is* the shelf, and at the other end it is the
            // far lane running out to the rim. So the three lanes start one
            // border in from the centre-facing edge, and one more shelf in
            // when the shelf is standing there.
            let v = py / h;
            let from_shelf = if ledge_outer { 1.0 - v } else { v };
            let first = MARGIN_FRAC
                + if ledge_outer {
                    0.0
                } else {
                    LEDGE_FRAC - MARGIN_FRAC
                };
            let on_ledge = from_shelf < LEDGE_FRAC;
            let below = ((v - first - COMBAT_FRAC) / (LANE_FRAC * 3.0)).clamp(0.0, 1.0);
            #[expect(clippy::cast_possible_truncation, reason = "three lanes")]
            let lane = (below * 3.0).floor().clamp(0.0, 2.0) as usize;
            // Quiet, not absent. The mat's job is to say where a seat's
            // ground ends: everything on it — cards, rims, the glow — has to
            // stay louder, and a mat nobody can see is not quiet, it is
            // missing.
            //
            // Lowered twice, for the same reason each time, and the second
            // time is the one worth remembering. These numbers are an
            // **alpha** on white, and an alpha is linear light: 0.050 of it
            // is not a five-percent tint, it is display 0.24 laid over
            // whatever is beneath. Against a dark green felt that was a
            // modest lift and it was tuned and accepted there. Against the
            // cloth the table is covered in now — measured at `(42, 29, 21)`
            // on the same frame — the same veil put the mat at `(65, 61, 59)`:
            // a pale tray, brighter and greyer than the table it lies on,
            // filling most of the screen because a mat now *is* most of the
            // screen.
            //
            // Every reference this table is drawn from puts the play surface
            // at or below the surround and marks a seat at its edge. So the
            // veil is cut to about a quarter, which lands the field just above
            // the wood — the mat says where a seat's ground is without being
            // the brightest thing in the room.
            let base = if on_ledge {
                MAT_LEDGE_VALUE
            } else {
                MAT_LANES[lane]
            };
            // A hairline *between* lanes, so the rows separate without a
            // border drawn around each one. Measured in pixels from the two
            // boundaries: expressed as a fraction of a lane it comes out
            // under a pixel wide on a mat this shallow and never appears.
            //
            // The ledge's own boundary is a seam too, and a brighter one:
            // it is where the seat's ground stops being a place cards stand
            // and starts being a shelf they are described on.
            let seam_width = (h * MAT_SEAM_WIDTH).max(1.0);
            let lanes = h * (first + COMBAT_FRAC);
            let step = h * LANE_FRAC;
            let fence = h * if ledge_outer {
                1.0 - LEDGE_FRAC
            } else {
                LEDGE_FRAC
            };
            let seam = [lanes + step, lanes + step * 2.0]
                .iter()
                .map(|edge| (py - edge).abs())
                .fold(f32::MAX, f32::min);
            let seam = (1.0 - seam / seam_width).clamp(0.0, 1.0) * MAT_SEAM;
            let ledge_seam =
                (1.0 - (py - fence).abs() / seam_width).clamp(0.0, 1.0) * MAT_SEAM * MAT_LEDGE_SEAM;
            let seam = seam.max(ledge_seam);

            // The rim: the one part that is meant to be seen from across the
            // table, since it is what carries the seat's colour.
            let falloff = (1.0 - inset / edge).clamp(0.0, 1.0);
            let border = falloff.powf(MAT_RIM_FALL);
            // And a soft feather so the mat has no jaggies.
            let coverage = (0.5 - outside).clamp(0.0, 1.0);

            let value = base + seam + border * MAT_RIM_LIGHT;
            // White where the mat is felt, the seat's colour where it is rim.
            //
            // The crossfade is deliberately *not* `border`. Reusing the
            // opacity's curve is the tidy version and it renders a washed-out
            // rim: 1.3 is a steep falloff, so the accent only approaches full
            // strength in the last texel or two, where coverage is feathering
            // it away as well. Composited over the felt that gave a pale
            // beige for gilt and a near-white line for the green seat — four
            // distinguishable places reduced back to one. A shallower
            // exponent spreads the hue across the whole rim while the
            // opacity keeps its own edge, and the seat colours separate.
            let hue = falloff.powf(MAT_HUE_FALL);
            let rgb = [
                (1.0 - hue).mul_add(1.0, hue * accent[0]),
                (1.0 - hue).mul_add(1.0, hue * accent[1]),
                (1.0 - hue).mul_add(1.0, hue * accent[2]),
            ];
            texture.put(
                x,
                y,
                [rgb[0], rgb[1], rgb[2], (value * coverage).clamp(0.0, 1.0)],
            );
        }
    }
    texture
}

/// A soft round glow, white with the falloff in the alpha channel, for
/// tinting under a seat's mat.
#[must_use]
pub fn glow(size: u32) -> Texture {
    let mut texture = Texture::blank(size, size);
    let extent = size as f32;
    for y in 0..size {
        for x in 0..size {
            let (u, v) = (
                (x as f32 + 0.5) / extent * 2.0 - 1.0,
                (y as f32 + 0.5) / extent * 2.0 - 1.0,
            );
            let radius = (u * u + v * v).sqrt();
            let falloff = (1.0 - radius).clamp(0.0, 1.0).powf(2.6);
            texture.put(x, y, [1.0, 1.0, 1.0, falloff]);
        }
    }
    texture
}

/// The soft dark patch a card sits in.
///
/// A card with thickness but no shadow reads as a sticker: the eye takes
/// contact shadow, not the edge, as the cue that an object is *on* something.
/// So this is drawn a little larger than the card and slid underneath it,
/// where only the halo around the edges shows.
///
/// It is a shape, not a cast shadow. A real one would need the table to be
/// lit, and everything down there is unlit on purpose — scene lighting on
/// card art would make colour identity unreadable. A painted halo is honest
/// about what it is, costs one quad, and is correct from every angle the
/// camera can reach.
///
/// `spread` is how much of the texture's short side the falloff takes on each
/// side (the rest is the card's own silhouette); `radius` is the card's
/// corner radius as a fraction of the card's short side, so the shadow's
/// corners match the mesh's.
#[must_use]
pub fn card_shadow(width: u32, height: u32, spread: f32, radius: f32) -> Texture {
    /// How dark the shadow gets right under the card. Well below opaque: a
    /// shadow darker than the felt's own shading reads as a hole in the table.
    const DENSITY: f32 = 0.55;

    let mut texture = Texture::blank(width, height);
    let (w, h) = (width as f32, height as f32);
    let inset = (spread * w.min(h)).max(1.0);
    let (half_w, half_h) = (w.mul_add(0.5, -inset), h.mul_add(0.5, -inset));
    let corner = (radius * (half_w * 2.0).min(half_h * 2.0)).max(1.0);
    for y in 0..height {
        for x in 0..width {
            let px = (x as f32 + 0.5) - w * 0.5;
            let py = (y as f32 + 0.5) - h * 0.5;
            // Distance outside the rounded rectangle the card covers: zero
            // beneath it, growing to `inset` at the texture's own edge.
            let dx = (px.abs() - (half_w - corner)).max(0.0);
            let dy = (py.abs() - (half_h - corner)).max(0.0);
            let outside = dx.hypot(dy) - corner;
            let t = (outside / inset).clamp(0.0, 1.0);
            texture.put(x, y, [0.0, 0.0, 0.0, (1.0 - t).powi(2) * DENSITY]);
        }
    }
    texture
}

/// The mark drawn in the middle of a pile's empty place.
///
/// A zone with nothing in it is a rounded hollow and nothing else, and four
/// identical hollows around a mat say only "something goes here". These say
/// *what*. They are covered the moment a card lies on the pile, which is the
/// whole design: the mark is the empty state, and the card is the answer to
/// it.
///
/// Arithmetic, like everything else on this table — `docs/legal.md` §2. A
/// crown, a headstone, a stack of leaves and a barred ring are about as far
/// from anyone's trade dress as a shape gets, and none of them is a glyph out
/// of a font: the table is 3D and there is no text on it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ZoneMark {
    /// The library: three leaves seen edge on.
    Library,
    /// The graveyard: a headstone.
    Graveyard,
    /// Exile: a ring with a bar across it.
    Exile,
    /// The command zone: a crown.
    Command,
}

/// How much of the well's short side the mark spans, from the middle out.
const MARK_HALF: f32 = 0.30;

/// How much of the lip's colour the mark carries where it is solid.
///
/// Faint on purpose. It is a label on an empty place, not a thing on the
/// table, and a mark that competed with a real card would make an empty
/// graveyard louder than a full one.
const MARK_ALPHA: f32 = 0.30;

/// How many samples across each pixel the mark is measured with.
///
/// Three, so nine per pixel. The shapes are hard-edged rather than distance
/// fields — a crown's zigzag has no closed-form distance worth writing — so
/// the edge is smoothed by counting rather than by a ramp.
const MARK_SAMPLES: u32 = 3;

/// Whether the point `(x, y)` is inside `mark`, in mark space.
///
/// Mark space is `[-1, 1]` on both axes with `y` **down**, the way the
/// texture is written, so a crown's points are at negative `y`.
#[must_use]
fn in_mark(mark: ZoneMark, x: f32, y: f32) -> bool {
    match mark {
        // Three leaves, edge on: the pile a card is drawn off the top of.
        ZoneMark::Library => {
            x.abs() <= 0.80
                && [-0.50_f32, 0.0, 0.50]
                    .iter()
                    .any(|centre| (y - centre).abs() <= 0.13)
        }
        // A headstone: a rectangle with a half-round top.
        ZoneMark::Graveyard => {
            let shoulder = -0.20;
            (x.abs() <= 0.50 && (shoulder..=0.78).contains(&y))
                || (y < shoulder && x.hypot(y - shoulder) <= 0.50)
        }
        // A ring with a bar across it: gone, and not coming back.
        ZoneMark::Exile => {
            let ring = (x.hypot(y) - 0.62).abs() <= 0.135;
            // The bar, in the ring's own diagonal. Rotated by hand rather
            // than with a matrix: one angle, two constants.
            let (s, c) = (
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
            );
            let across = x.mul_add(c, y * s);
            let along = y.mul_add(c, -(x * s));
            ring || (across.abs() <= 0.135 && along.abs() <= 0.755)
        }
        // A crown: a band with three points on it. The top edge is a
        // triangle wave — zero at each point, one at each valley — which is
        // the whole shape in one line.
        ZoneMark::Command => {
            if x.abs() > 0.75 || y > 0.62 {
                return false;
            }
            let step = (x + 0.75) / 0.375;
            let wave = 1.0 - (step.rem_euclid(2.0) - 1.0).abs();
            y >= 0.55f32.mul_add(wave, -0.72)
        }
    }
}

/// How much of one texture pixel the mark covers, by counting samples.
fn mark_coverage(mark: ZoneMark, px: f32, py: f32, half: f32, step: f32) -> f32 {
    let mut hits = 0u32;
    for sy in 0..MARK_SAMPLES {
        for sx in 0..MARK_SAMPLES {
            let ox = (f64::from(sx) + 0.5) as f32 / MARK_SAMPLES as f32 - 0.5;
            let oy = (f64::from(sy) + 0.5) as f32 / MARK_SAMPLES as f32 - 0.5;
            let x = px.mul_add(1.0, ox * step) / half;
            let y = py.mul_add(1.0, oy * step) / half;
            if x.abs() <= 1.0 && y.abs() <= 1.0 && in_mark(mark, x, y) {
                hits += 1;
            }
        }
    }
    f32::from(u16::try_from(hits).unwrap_or(u16::MAX)) / (MARK_SAMPLES * MARK_SAMPLES) as f32
}

/// The place a pile stands when nothing is standing there.
///
/// A card-shaped well cut into the table: a thin bright lip where light would
/// catch the cut edge, a shallow darkening inside it, and the zone's own
/// [`ZoneMark`] in the middle. All three are load-bearing, and the lip is the
/// half that was missing first. A darkening alone is what an empty zone used
/// to be, and on this table it is invisible — the wood beside a mat measures
/// `(22, 15, 11)` and the darkened patch on it measured `(15, 9, 6)`, a
/// difference no eye finds at arm's length. Zones a player cannot see are
/// zones a player does not know are clickable; and four wells cut to the same
/// shape are four zones a player has to count round the mat to tell apart,
/// which is what the mark answers.
///
/// The lip's colour stays *under* the seat rim's gilt on purpose. A rim says
/// whose seat this is and which seat everyone is waiting for; a well says
/// only that a pile goes here, and a well that outshone the rim would be
/// answering the louder question with the quieter fact.
///
/// `radius` is the card's corner radius as a fraction of the short side, so
/// the lip follows the same silhouette the card mesh is cut to.
#[must_use]
pub fn card_well(width: u32, height: u32, radius: f32, mark: ZoneMark) -> Texture {
    /// The lip, display-referred. Bone rather than gold: see above.
    const LIP: [f32; 3] = [120.0 / 255.0, 102.0 / 255.0, 78.0 / 255.0];
    /// How opaque the lip gets where it is brightest.
    const LIP_ALPHA: f32 = 0.55;
    /// How far the inside of the well is darkened.
    ///
    /// Lighter than the 0.42 this replaces. The lip carries the reading now,
    /// and a fill dark enough to be seen on its own reads as a hole rather
    /// than a hollow.
    const FILL_ALPHA: f32 = 0.22;
    /// How wide the lip is, as a fraction of the card's short side.
    const LIP_WIDTH: f32 = 0.075;
    /// Where across the lip it is brightest, as a fraction of its width.
    ///
    /// Not at the edge. The mesh antialiases its own silhouette, so a peak
    /// sitting exactly on the boundary is spent on pixels that are half
    /// transparent anyway.
    const LIP_PEAK: f32 = 0.4;

    let mut texture = Texture::blank(width, height);
    let (w, h) = (width as f32, height as f32);
    let short = w.min(h);
    let corner = (radius * short).max(1.0);
    let (half_w, half_h) = (w * 0.5, h * 0.5);
    let lip_px = (LIP_WIDTH * short).max(1.0);
    let mark_half = (MARK_HALF * short).max(1.0);
    for y in 0..height {
        for x in 0..width {
            let px = (x as f32 + 0.5) - half_w;
            let py = (y as f32 + 0.5) - half_h;
            // Signed distance to the rounded rectangle, negative inside.
            let qx = px.abs() - (half_w - corner);
            let qy = py.abs() - (half_h - corner);
            let outside = qx.max(0.0).hypot(qy.max(0.0));
            let inside = qx.max(qy).min(0.0);
            let distance = outside + inside - corner;
            if distance > 0.0 {
                continue;
            }
            // How far in from the lip's outer edge, 0 at the silhouette and
            // 1 a lip's width inside it.
            let across = (-distance / lip_px).clamp(0.0, 1.0);
            // A band that rises to `LIP_PEAK` and falls away again, squared
            // so its shoulders are soft rather than creased.
            let reach = LIP_PEAK.max(1.0 - LIP_PEAK);
            let lip = (1.0 - (across - LIP_PEAK).abs() / reach).clamp(0.0, 1.0);
            let lip = lip * lip;
            // The mark says which zone this place is. It is *added* to the
            // lip's own reading rather than replacing it, so a mark can never
            // eat the hollow it is drawn in — and it is measured against the
            // short side, so the four wells all carry it at one size.
            let ink = lip.max(mark_coverage(mark, px, py, mark_half, 1.0) * MARK_ALPHA);
            texture.put(
                x,
                y,
                [
                    LIP[0] * ink,
                    LIP[1] * ink,
                    LIP[2] * ink,
                    (LIP_ALPHA - FILL_ALPHA).mul_add(ink, FILL_ALPHA),
                ],
            );
        }
    }
    texture
}

/// Where the ring band's inner edge sits, as a fraction of the hearth quad's
/// half-width.
pub const HEARTH_INNER: f32 = 0.46;
/// Where the ring band's outer edge sits. A caller sizing its quad wants this
/// one: the ring a player sees is this fraction of the quad *across*.
pub const HEARTH_OUTER: f32 = 0.60;
/// How many ticks the ring carries.
///
/// Eight, at 45°, so it reads as a compass. It was twenty-four — one every
/// fifteen degrees — which is a clock face, and a clock face in the middle of
/// a card table is the single loudest thing in `docs/design.md` §1.1's
/// numeric read of the board.
pub const HEARTH_TICKS: u16 = 8;

/// The pool of lamplight over the middle of the table, with the arcane ring
/// inlaid in it.
///
/// Two things in one texture because they are one thing to look at: a warm
/// glow that says "this is where the light is", and a set of faint concentric
/// arcs with tick marks that give the felt some structure to read against.
/// A table with nothing between the seat mats reads as an infinite green
/// plane no matter how good the grain is.
///
/// The geometry is arithmetic, not ornament borrowed from anywhere:
/// `docs/legal.md` §2, and rings and ticks are about as far from anyone's
/// trade dress as a shape can get.
///
/// `inner` and `outer` are where the ring band sits, as fractions of the
/// texture's half-width; the light pool fills the whole thing. The renderer
/// passes [`HEARTH_INNER`] and [`HEARTH_OUTER`], which are also what the
/// caller sizes its quad against.
#[must_use]
pub fn hearth(size: u32, inner: f32, outer: f32) -> Texture {
    /// The lamp's colour: candle, not daylight.
    const WARM: [f32; 3] = [1.0, 0.86, 0.62];
    /// The inlay's: old gilt, dim enough to sit under the cards.
    const GILT: [f32; 3] = [0.86, 0.72, 0.40];

    let mut texture = Texture::blank(size, size);
    let extent = size as f32;
    let ticks = f32::from(HEARTH_TICKS);
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / extent * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / extent * 2.0 - 1.0;
            let radius = u.hypot(v);
            if radius > 1.0 {
                continue;
            }
            // The pool: brightest at the middle, gone before the edge, so
            // the quad never shows as a square against the felt. Halved from
            // what it shipped as — candlelight at 0.12 over `felt`'s green
            // made the middle of the table read olive and only its corners
            // read green.
            let pool = (1.0 - radius).clamp(0.0, 1.0).powf(2.2) * 0.06;

            // One hairline, on the outer edge, and the band itself barely
            // lifted — an inlay, not a painted circle. It was two, which with
            // the ticks between them made three concentric lines and a dial.
            let hairline = (1.0 - (radius - outer).abs() / 0.006).clamp(0.0, 1.0) * 0.30;
            let inside_band = radius > inner && radius < outer;
            let band = f32::from(u8::from(inside_band)) * 0.045;

            // Ticks: short radial marks across the band, deterministic and
            // evenly spaced.
            let angle = v.atan2(u);
            let phase = (angle / TAU * ticks).fract().abs();
            let near_tick = phase.min(1.0 - phase);
            let tick = if inside_band {
                (1.0 - near_tick / 0.03).clamp(0.0, 1.0) * 0.22
            } else {
                0.0
            };

            let alpha = pool + hairline + band + tick;
            if alpha <= 0.0 {
                continue;
            }
            // The inlay is gilt, the pool is candlelight; blend by how much
            // of the alpha each contributed, so the ring stays gold where it
            // crosses the bright middle.
            let gold = ((hairline + tick + band) / alpha).clamp(0.0, 1.0);
            let colour = mix(WARM, GILT, gold);
            texture.put(x, y, [colour[0], colour[1], colour[2], alpha.min(1.0)]);
        }
    }
    texture
}

// ------------------------------------------------------------- phase light

/// The colour the middle of the table takes during one step of a turn.
///
/// The table already answers *whose* turn it is, on the felt, through
/// [`seat_mat`]'s rim; this answers *where in the turn we are* — a thing a
/// player currently has to read off the rail, in text, at the edge of the
/// screen. Combat is the case that matters: a board that goes cold-hot as
/// attackers are declared says "something is about to happen to you" faster
/// than a highlighted row ever does.
///
/// Three rules keep it from becoming noise. It washes the **light pool**
/// only, never the felt (too much of the screen) and never the firewheel —
/// that is the colour wheel, and a wheel with a red cast over it would be
/// lying about colour identity, which is the one thing on the table that has
/// to stay literally true. It is desaturated: these are lamps, not filters.
/// And it is a *wash*, blended over what is already there, not a multiplier —
/// multiplying candlelight by a blue gives grey, which is how a tint like
/// this usually fails.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PhaseLight {
    /// The wash's colour, display-referred.
    pub rgb: [f32; 3],
    /// How much of it there is, 0.0 (nothing) to 1.0 (as strong as the
    /// wash ever gets).
    pub energy: f32,
}

/// Dawn: cool and low, for the steps nobody acts in.
///
/// A **teal**, not the blue it reads as. It used to be `[0.42, 0.58, 0.86]`,
/// seven degrees of hue from the pie's own blue — near enough that the two
/// were the same colour with different names. That went unnoticed while the
/// wash was a small pool in the middle of a dark table, and stops being
/// survivable the moment it fills the open middle and runs up against the mat
/// rims: a table that turns blue at untap is a table saying "this is the blue
/// seat's". `the_wash_never_speaks_in_a_colour_of_the_pie` is that rule as a
/// build failure.
pub const COOL: [f32; 3] = [0.24, 0.72, 0.80];

/// The lamp the pool is already generated as — a wash of it changes nothing,
/// which is the point during a main phase.
///
/// The one wash that is allowed to share a hue with another ([`EMBER`] is
/// nine degrees away), because it is not really a signal: it is the absence
/// of one, at the lowest energy of the four, and it is pale where ember is
/// saturated.
pub const CANDLE: [f32; 3] = [1.0, 0.86, 0.62];

/// Combat: iron heating, not a fire alarm.
///
/// Moved from `[0.90, 0.34, 0.22]`, which sat **two degrees** of hue from the
/// pie's red — the same collision [`COOL`] had with blue, and the worse of
/// the two, because combat is when a player is most likely to be reading whose
/// creature is whose. The warm corridor is narrow (pie red at 9°, [`CANDLE`]
/// at 38°), so the distance is spent where it is needed: twenty degrees from
/// red, and the separation from candlelight is carried by saturation instead.
/// Hotter and more saturated is also the truer picture — metal at temperature
/// goes towards yellow, and only a fire alarm is crimson.
///
/// It stays inside the chroma cap `every_step_is_lit_and_in_range` enforces —
/// a wash is a lamp over a table, not a filter over the cards — which is what
/// picked `0.24` for the blue channel rather than the `0.12` that would have
/// bought another degree of hue. The rule is older than this colour and it
/// still holds: the middle is where no card lies, but bloom does not know
/// that.
pub const EMBER: [f32; 3] = [1.0, 0.61, 0.24];

/// Dusk, for the end of a turn.
///
/// Kept as it was. Violet is the one hue on the table that no card claims —
/// the pie's black is a near-neutral slate, saturated far too little to be
/// confused with it.
pub const DUSK: [f32; 3] = [0.52, 0.40, 0.72];

/// What the light does in a given step.
///
/// The arc over a turn is deliberate rather than twelve unrelated colours: a
/// cool quiet beginning, neutral through the main phase (where the pool is
/// simply the lamplight it was generated as), a rise into combat that peaks
/// at damage, and a violet settling at the end. A player who has played two
/// turns knows where they are without reading a word.
#[must_use]
pub fn phase_light(step: baylee_view::Step) -> PhaseLight {
    use baylee_view::Step;
    match step {
        Step::Untap => PhaseLight {
            rgb: COOL,
            energy: 0.30,
        },
        Step::Upkeep => PhaseLight {
            rgb: COOL,
            energy: 0.42,
        },
        Step::Draw => PhaseLight {
            rgb: COOL,
            energy: 0.34,
        },
        Step::Main => PhaseLight {
            rgb: CANDLE,
            energy: 0.16,
        },
        Step::CombatBegin => PhaseLight {
            rgb: EMBER,
            energy: 0.34,
        },
        Step::DeclareAttackers => PhaseLight {
            rgb: EMBER,
            energy: 0.62,
        },
        Step::DeclareBlockers => PhaseLight {
            rgb: EMBER,
            energy: 0.74,
        },
        Step::CombatDamageFirst | Step::CombatDamage => PhaseLight {
            rgb: EMBER,
            energy: 1.0,
        },
        Step::CombatEnd => PhaseLight {
            rgb: EMBER,
            energy: 0.40,
        },
        Step::End => PhaseLight {
            rgb: DUSK,
            energy: 0.36,
        },
        Step::Cleanup => PhaseLight {
            rgb: DUSK,
            energy: 0.22,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::CARD_WIDTH;

    /// sRGB byte to linear light, and back. The GPU blends in linear and the
    /// texture is sRGB, so a claim about what a player *sees* has to make the
    /// same round trip the hardware does — comparing the two byte values
    /// directly is the mistake that let an invisible well ship.
    fn to_linear(byte: f32) -> f32 {
        let c = byte / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    fn to_srgb(linear: f32) -> f32 {
        let c = linear.clamp(0.0, 1.0);
        let v = if c <= 0.003_130_8 {
            c * 12.92
        } else {
            1.055f32.mul_add(c.powf(1.0 / 2.4), -0.055)
        };
        v * 255.0
    }

    /// One texel composited over a background, the way the blend does it.
    fn over(texel: [f32; 4], ground: [f32; 3]) -> [f32; 3] {
        let mut out = [0.0; 3];
        for channel in 0..3 {
            let src = to_linear(texel[channel] * 255.0);
            let dst = to_linear(ground[channel]);
            out[channel] = to_srgb(texel[3].mul_add(src, (1.0 - texel[3]) * dst));
        }
        out
    }

    /// The table beside a seat mat, measured off a screenshot of a live
    /// table at two points: in shadow, and where the lamp reaches.
    const TIMBER_DARK: [f32; 3] = [22.0, 15.0, 11.0];
    const TIMBER_LIT: [f32; 3] = [52.0, 37.0, 26.0];

    /// The brightest texel of a well, and the one in the middle of its fill.
    fn well_lip_and_fill() -> ([f32; 4], [f32; 4]) {
        let well = card_well(128, 179, CARD_CORNER_FRACTION, ZoneMark::Library);
        let lip = pixels(&well)
            .map(|(_, _, rgba)| rgba)
            .max_by(|a, b| a[0].total_cmp(&b[0]))
            .expect("a well has pixels");
        (lip, well.pixel(64, 20))
    }

    /// A card's corner as a fraction of its short side; the renderer passes
    /// the same number, from `CARD_CORNER / CARD_WIDTH`.
    const CARD_CORNER_FRACTION: f32 = 0.0476;

    #[test]
    fn an_empty_place_can_be_seen_on_the_wood_it_is_cut_into() {
        let (lip, fill) = well_lip_and_fill();
        for ground in [TIMBER_DARK, TIMBER_LIT] {
            let drawn = over(lip, ground);
            let apart = (0..3)
                .map(|c| (drawn[c] - ground[c]).abs())
                .fold(0.0f32, f32::max);
            assert!(
                apart > 40.0,
                "the lip is {apart:.0}/255 from {ground:?}, which is a well nobody can find"
            );
        }
        // And the half that could not do it alone. This is not a weaker
        // version of the assertion above: it is the measurement that says
        // *why* there is a lip at all, and it fails if somebody decides the
        // fill can carry the reading by getting darker.
        let alone = over(fill, TIMBER_DARK);
        let apart = (0..3)
            .map(|c| (alone[c] - TIMBER_DARK[c]).abs())
            .fold(0.0f32, f32::max);
        assert!(
            apart < 12.0,
            "a fill {apart:.0}/255 from the table is doing the lip's work"
        );
    }

    #[test]
    fn a_well_stays_quieter_than_the_seat_rim_beside_it() {
        // The gilt of the local seat's rim, measured at the same table.
        const RIM: [f32; 3] = [110.0, 79.0, 42.0];
        let (lip, _) = well_lip_and_fill();
        let drawn = over(lip, TIMBER_DARK);
        let brightness = |c: [f32; 3]| 0.2126f32.mul_add(c[0], 0.7152 * c[1] + 0.0722 * c[2]);
        assert!(
            brightness(drawn) < brightness(RIM),
            "the well draws at {drawn:?}, brighter than the rim at {RIM:?} — \
             a place for a pile must not outshine whose seat it is"
        );
    }

    /// Every zone's mark is in the middle of its well, inside the lip, and
    /// unlike every other zone's.
    ///
    /// The last part is the one worth a test. A mark exists to answer "which
    /// zone is this" at a glance from across a table, so four marks that
    /// covered nearly the same texels would be four wells again with extra
    /// arithmetic — and that is exactly what a small change to one of the
    /// shapes could quietly produce.
    /// A texel back as the bytes it is stored as, so two of them may be
    /// compared for equality without comparing floats.
    fn quantised(rgba: [f32; 4]) -> [u8; 4] {
        rgba.map(|v| (v * 255.0 + 0.5) as u8)
    }

    #[test]
    fn each_zone_wears_its_own_mark_and_wears_it_in_the_middle() {
        let marks = [
            ZoneMark::Library,
            ZoneMark::Graveyard,
            ZoneMark::Exile,
            ZoneMark::Command,
        ];
        let wells: Vec<Texture> = marks
            .iter()
            .map(|&mark| card_well(128, 179, CARD_CORNER_FRACTION, mark))
            .collect();
        // The middle box the mark is allowed to reach: `MARK_HALF` of the
        // short side either way, plus a texel for the samples' own edge.
        let inside = |x: u32, y: u32| {
            let half = MARK_HALF * 128.0 + 1.0;
            (x as f32 - 63.5).abs() <= half && (y as f32 - 89.5).abs() <= half
        };
        // The alpha the well carries with nothing drawn on it, read where no
        // lip reaches and no mark may: past the lip, above the box.
        let plain = wells[0].pixel(64, 20)[3];

        for (mark, well) in marks.iter().zip(&wells) {
            let drawn = (0..179 * 128)
                .filter(|i| {
                    let (x, y) = (i % 128, i / 128);
                    inside(x, y) && well.pixel(x, y)[3] > plain + 0.02
                })
                .count();
            assert!(
                drawn > 400,
                "{mark:?} drew {drawn} texels, which is nothing"
            );
        }

        // Outside that box every well is the same well. A mark that ran into
        // the lip would fail here rather than merely look wrong.
        for (mark, well) in marks.iter().zip(&wells).skip(1) {
            for i in 0..179 * 128 {
                let (x, y) = (i % 128, i / 128);
                if inside(x, y) {
                    continue;
                }
                assert_eq!(
                    quantised(well.pixel(x, y)),
                    quantised(wells[0].pixel(x, y)),
                    "{mark:?} reached ({x}, {y}), outside its own box"
                );
            }
        }

        for (i, a) in marks.iter().enumerate() {
            for (j, b) in marks.iter().enumerate().skip(i + 1) {
                let differ = (0..179 * 128)
                    .filter(|k| {
                        let (x, y) = (k % 128, k / 128);
                        inside(x, y)
                            && quantised(wells[i].pixel(x, y)) != quantised(wells[j].pixel(x, y))
                    })
                    .count();
                assert!(
                    differ > 300,
                    "{a:?} and {b:?} differ on only {differ} texels — they read as one shape"
                );
            }
        }
    }

    #[test]
    fn a_well_is_a_rim_and_not_a_disc() {
        let well = card_well(128, 179, CARD_CORNER_FRACTION, ZoneMark::Library);
        // Across the middle: transparent outside, bright at the lip, and
        // back down to the fill in the centre. A disc would rise once.
        let row: Vec<f32> = (0..128).map(|x| well.pixel(x, 30)[0]).collect();
        let brightest = |slice: &[f32]| {
            slice
                .iter()
                .copied()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .expect("a half-row has pixels")
        };
        // Both halves, because the lip runs all the way round and a test that
        // took the whole row's maximum would only ever find one of them.
        let left = brightest(&row[..64]);
        let right = brightest(&row[64..]);
        assert!(
            left.0 < 32,
            "the left lip peaks at {}, not near its edge",
            left.0
        );
        assert!(
            right.0 + 64 > 95,
            "the right lip peaks at {}, not near its edge",
            right.0 + 64
        );
        assert!(
            row[64] < left.1.min(right.1) * 0.5,
            "the middle is {:.3} against lips of {:.3} and {:.3} — that is a disc",
            row[64],
            left.1,
            right.1
        );
    }

    /// Every pixel of a texture, as `(x, y, rgba)`.
    fn pixels(t: &Texture) -> impl Iterator<Item = (u32, u32, [f32; 4])> + '_ {
        (0..t.height).flat_map(move |y| (0..t.width).map(move |x| (x, y, t.pixel(x, y))))
    }

    /// Hue in degrees and saturation, the two channels a colour is recognised
    /// by across a table. Value is left out deliberately: a rim and a wash at
    /// the same hue read as the same colour whichever is brighter.
    fn hue_sat(rgb: [f32; 3]) -> (f32, f32) {
        let [r, g, b] = rgb;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let chroma = max - min;
        if chroma < 1e-6 || max < 1e-6 {
            return (0.0, 0.0);
        }
        let hue = 60.0
            * if (max - r).abs() < 1e-6 {
                ((g - b) / chroma).rem_euclid(6.0)
            } else if (max - g).abs() < 1e-6 {
                (b - r) / chroma + 2.0
            } else {
                (r - g) / chroma + 4.0
            };
        (hue.rem_euclid(360.0), chroma / max)
    }

    /// The shorter way round the wheel.
    fn hue_gap(a: f32, b: f32) -> f32 {
        let d = (a - b).abs().rem_euclid(360.0);
        d.min(360.0 - d)
    }

    /// Every wash, with the name the failure message needs.
    const WASHES: [(&str, [f32; 3]); 4] = [
        ("COOL", COOL),
        ("CANDLE", CANDLE),
        ("EMBER", EMBER),
        ("DUSK", DUSK),
    ];

    /// Below this a colour has no hue worth confusing — the pie's parchment
    /// white and slate black are both under it, which is why neither appears
    /// in the failures this test can produce.
    const HAS_A_HUE: f32 = 0.35;

    /// Relative luminance, the channel the eye actually judges "how dark" by.
    fn luma(rgba: [f32; 4]) -> f32 {
        0.2126f32.mul_add(rgba[0], 0.7152f32.mul_add(rgba[1], 0.0722 * rgba[2]))
    }

    /// Standard deviation, for the contrast measurements below.
    fn deviation(values: &[f32]) -> f32 {
        let n = values.len() as f32;
        let mean = values.iter().sum::<f32>() / n;
        (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n).sqrt()
    }

    /// The redesign replaces casino green with cool mineral and warm metal.
    /// Both hue and saturation remain bounded; this is not permission for
    /// a neutral black hole or a saturated blue field behind blue cards.
    #[test]
    fn the_baize_is_cool_mineral_under_warm_metal() {
        for cloth in [FELT_DEEP, FELT_CLOTH, FELT_WORN] {
            let (hue, sat) = hue_sat(cloth);
            assert!(
                (185.0..205.0).contains(&hue),
                "{cloth:?} is at {hue}°, outside the mineral palette"
            );
            assert!(
                (0.30..0.50).contains(&sat),
                "{cloth:?} is {sat} saturated — cloth, not a swatch"
            );
        }
        // Warm metal against cool cloth. A rail the same hue as the felt is
        // a table with no rail.
        for hide in [RAIL_HIDE, RAIL_LIP, APRON] {
            let (hue, _) = hue_sat(hide);
            assert!(
                (10.0..45.0).contains(&hue),
                "the rail at {hue}° is not warm metal"
            );
        }
        // The wall has to be darker than the top it hangs from, or the slab
        // has no thickness to see. Nothing lights this stage, so this
        // difference *is* the third dimension.
        assert!(
            luma([APRON[0], APRON[1], APRON[2], 1.0])
                < luma([RAIL_HIDE[0], RAIL_HIDE[1], RAIL_HIDE[2], 1.0]),
            "the apron has to be darker than the rail above it"
        );
    }

    /// And it is *rough*: a weave you can see at the distance a card is read.
    ///
    /// The measurement is at card scale deliberately — a grain that only
    /// exists at texel scale is a grain nobody ever sees, and the same
    /// mistake was caught once already on the wood this replaces.
    #[test]
    fn the_baize_has_a_tooth_at_card_scale() {
        let cloth = felt(256);
        // A card is `CARD_WIDTH` of a table about 30 units across, so it is
        // roughly a thirtieth of this texture: sample a card-sized patch.
        let side = (256.0 * CARD_WIDTH / 30.0).round() as u32;
        assert!(side >= 6, "a card is {side} texels — too few to measure");
        let patch: Vec<f32> = (0..side)
            .flat_map(|y| (0..side).map(move |x| (x + 100, y + 100)))
            .map(|(x, y)| luma(cloth.pixel(x, y)))
            .collect();
        let rough = deviation(&patch);
        assert!(
            rough > 0.002,
            "the cloth is flat at card scale ({rough}) — that is paint, not felt"
        );
        assert!(
            rough < 0.030,
            "the cloth is louder than the cards on it ({rough})"
        );
    }

    /// A table has rounded corners, and neither a chamfer nor a stadium.
    ///
    /// It was a racetrack at 0.42, chosen when the corners were the *only*
    /// place the sky could be seen. `AIR` now leaves a band all the way
    /// round, so the corner is free to be what a table's corner is — and the
    /// owner's word on the racetrack was "less border corner". Both bounds
    /// are here because both failures are real: a corner too small is a
    /// rectangle with the edges filed off, and one too large eats the play
    /// area at the ends of the mats.
    #[test]
    fn a_table_has_a_corner_and_not_a_chamfer() {
        let span = Vec2::new(34.0, 26.0);
        let r = table_corner(span);
        assert!(
            r > span.min_element() * 0.04,
            "a {r} corner on a {span:?} table reads as a bevel, not a corner"
        );
        assert!(
            r < span.min_element() * 0.10,
            "a {r} corner is a racetrack again"
        );
        // And it is drawn as a curve, not as one flat cut: the deepest point
        // of the arc has to sit clear of the chord across it by more than the
        // rail is wide, or the rail follows a straight line round the bend.
        let sag = r * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
        assert!(
            sag > RAIL_WIDTH * 0.5,
            "the arc sags {sag} across a {RAIL_WIDTH} rail — that is a chamfer"
        );
    }

    /// A mat's corner and rim are lengths now, so they can be checked against
    /// the mat the layout actually hands out.
    ///
    /// The shallowest one there is: [`crate::layout::POD_DEPTH`] plus
    /// [`MAT_MARGIN`] on either side. The bound is taken against the playing
    /// extent alone anyway, because that is the stricter of the two
    /// rectangles — but it is a choice now rather than the only rectangle
    /// this crate can reach, which is what it was while the border was
    /// `ZONE_MARGIN` in the renderer and no constant here. Both sides again,
    /// and each is a mistake that has
    /// been made on this table: too round and a board reads as a button, too
    /// square and the mat has spikes at the corners where the rim doubles
    /// back on itself.
    #[test]
    fn a_mat_is_a_playing_surface_and_not_a_button() {
        let depth = crate::layout::POD_DEPTH;
        assert!(
            MAT_CORNER < depth * 0.05,
            "a {MAT_CORNER} corner on a mat {depth} deep is a lozenge"
        );
        // The rim has to survive a lane: a border as deep as the row of
        // creatures behind it is a frame, not an edge. The two bounds that
        // need no layout at all — a corner wider than the rim, a rim thick
        // enough to carry a hue — are `const _` assertions beside the
        // constants themselves, where they fail at compile time.
        // The rim has to survive a **lane**, and a lane is what the mat has
        // left once the ledge has taken its share — measuring against the
        // whole depth would let the rim grow every time the shelf did.
        let lane = (depth - MAT_LEDGE) / 3.0;
        assert!(
            MAT_RIM < lane * 0.15,
            "a {MAT_RIM} rim against a {lane} lane is a frame"
        );
        // That the ledge clears the rim, and that it stays a shelf rather
        // than becoming a fourth lane, are both `const _` assertions beside
        // `MAT_LEDGE` itself — `layout` is the same crate, so the card the
        // second one measures against is reachable at compile time.
    }

    /// Lane dividers read as fine markings, not soft bands across the mat.
    #[test]
    fn lane_seams_are_fine_but_visible() {
        for height in [256, 512] {
            for ledge_outer in [false, true] {
                let mat = seat_mat(64, height, 0.02, 0.01, [1.0; 3], ledge_outer);
                let first = if ledge_outer { MARGIN_FRAC } else { LEDGE_FRAC };
                for lane in [1.0, 2.0] {
                    let centre = ((first + COMBAT_FRAC + LANE_FRAC * lane) * height as f32) as u32;
                    let bright = (centre - 10..=centre + 10)
                        .filter(|&y| mat.pixel(32, y)[3] > MAT_LANES[0] + MAT_SEAM * 0.5)
                        .count();
                    assert!(
                        (1..=height as usize / 128).contains(&bright),
                        "{bright} bright pixels at height {height}, outer ledge {ledge_outer}"
                    );
                }
            }
        }
    }

    /// The ink on the ledge has to be readable *as composited*, which is a
    /// different question from how bright the veil is.
    ///
    /// The veil is an alpha on white and the felt underneath it is not the
    /// `FELT_CLOTH` constant — the shader's lamp and the sky's tint are both
    /// multiplies this module cannot see. So the ground is the same one every
    /// other composited bound here uses: a **screenshot**, read off a live
    /// duel. Reasoning about the constant instead gives a felt three times
    /// too bright and a contrast that agrees with nothing on screen.
    ///
    /// Bounded on both sides for the reason
    /// `the_felt_is_dark_enough_to_read_cards_against` is: a one-sided check
    /// only stops the mistake it was written after. Too dark and the bar's
    /// secondary glyphs go under 4.5:1; too bright and the shelf has become
    /// the loudest band on a seat's ground, which is a panel, which is what
    /// the ledge exists not to be.
    #[test]
    fn the_ledge_is_dark_enough_to_read_ink_against() {
        // Bare felt beside a mat, measured off `off_a.png` at a duel framing.
        const FELT_ON_SCREEN: [f32; 3] = [21.0, 63.0, 40.0];
        // `PARCHMENT` #E0D4B0 and `PARCHMENT_EDGE` #AA986E — the bar's
        // numerals and its glyphs, the second being the worst case. Both came
        // down when the UI's parchment took the card shader's value; they
        // measure 7.9 and 4.1 against the 9.1 and 4.7 they had.
        const PARCHMENT: [f32; 3] = [224.0, 212.0, 176.0];
        const PARCHMENT_EDGE: [f32; 3] = [171.0, 152.0, 110.0];

        let ledge = over([1.0, 1.0, 1.0, MAT_LEDGE_VALUE], FELT_ON_SCREEN);
        // Two floors, because the bar writes two kinds of thing on this
        // shelf. Its numerals and its name are text and take WCAG's 4.5:1
        // for text, with room to spare; its glyphs are 10 px marks, which is
        // a graphical object and takes 3:1. They measure 8.7 and 4.5, so the
        // bounds are where the *categories* put them rather than a hair under
        // what today's numbers happen to be.
        for (what, ink, floor) in [
            ("parchment", PARCHMENT, 7.0),
            ("its glyphs", PARCHMENT_EDGE, 3.0),
        ] {
            let ratio = contrast(display_luma(ink), display_luma(ledge));
            assert!(
                ratio >= floor,
                "{what} on the ledge is {ratio:.2}:1 — the seat bar is not \
                 readable on its own shelf"
            );
        }
        // That the shelf is quieter than the quietest lane and louder than
        // bare felt — one shade below the mat rather than a hole cut through
        // it — needs no compositing and is a `const _` beside the constant.
    }

    /// WCAG contrast between two colours given as 0–255 display triples.
    fn display_luma(rgb: [f32; 3]) -> f32 {
        luma([to_linear(rgb[0]), to_linear(rgb[1]), to_linear(rgb[2]), 1.0])
    }

    fn contrast(a: f32, b: f32) -> f32 {
        let (hi, lo) = if a > b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn the_wash_never_speaks_in_a_colour_of_the_pie() {
        // The two signals on this table that a player reads without words:
        // the pie says *whose* and *which colour*, the wash says *where in
        // the turn*. They may not be the same colour. This was broken from
        // the day both existed — ember sat 1.9 degrees from the pie's red and
        // cool 7.1 from its blue — and it went unseen because the wash was a
        // small pool over a dark middle. It stops being survivable when the
        // wash fills the middle and runs up against the mat rims.
        //
        // Fifteen degrees is not a round number chosen in advance: the
        // colours were placed first and the worst surviving pair measures
        // 20.6, so this is the bound with a little air under it.
        const APART: f32 = 15.0;
        for (wname, wash) in WASHES {
            let (wh, ws) = hue_sat(wash);
            if ws < HAS_A_HUE {
                continue;
            }
            for (i, pie) in PIE.iter().enumerate() {
                let (ph, ps) = hue_sat(*pie);
                if ps < HAS_A_HUE {
                    continue;
                }
                let gap = hue_gap(wh, ph);
                assert!(
                    gap >= APART,
                    "{wname} {wash:?} is {gap:.1}° from pie colour {i} {pie:?} — \
                     the turn and the colour wheel would be saying the same thing"
                );
            }
        }
    }

    #[test]
    fn no_two_washes_read_as_the_same_light() {
        // Separating the wash from the pie is only half of it: four washes
        // that collapsed into two would leave a player unable to tell combat
        // from a main phase. A pair may share a hue *only* if one of them is
        // obviously the paler — which is the licence candlelight needs and
        // the only one it gets, because it is the absence of a wash rather
        // than a signal of its own.
        const BY_HUE: f32 = 25.0;
        const BY_SATURATION: f32 = 0.30;
        for (i, (aname, a)) in WASHES.iter().enumerate() {
            for (bname, b) in WASHES.iter().skip(i + 1) {
                let ((ah, asat), (bh, bsat)) = (hue_sat(*a), hue_sat(*b));
                let (hue, sat) = (hue_gap(ah, bh), (asat - bsat).abs());
                assert!(
                    hue >= BY_HUE || sat >= BY_SATURATION,
                    "{aname} and {bname} are {hue:.1}° apart at {sat:.2} of saturation — \
                     two steps of the turn that look alike"
                );
            }
        }
    }

    #[test]
    fn the_table_looks_the_same_to_everyone_at_it() {
        // No clock, no rng: two runs are the same table. Players screenshot
        // these, and a grain that moved would be the first thing anyone
        // noticed.
        assert_eq!(felt(64).rgba, felt(64).rgba);
        assert_eq!(glow(32).rgba, glow(32).rgba);
    }

    #[test]
    fn the_felt_is_dark_enough_to_read_cards_against() {
        let cloth = felt(96);
        assert_eq!(cloth.rgba.len(), 96 * 96 * 4);
        let mut brightest = 0.0_f32;
        for (_, _, px) in pixels(&cloth) {
            let luma = px[1].mul_add(0.72, px[0].mul_add(0.21, px[2] * 0.07));
            brightest = brightest.max(luma);
            assert!((px[3] - 1.0).abs() < 1e-6, "the felt is opaque");
        }
        assert!(
            brightest < 0.30,
            "a card has to be the brightest thing on the table, not the felt ({brightest})"
        );
        // And the other end of it, which is the half this test was missing
        // the first time the table was drawn: the felt shipped at a third of
        // this and read on screen as a hole in the world. A one-sided bound
        // on "dark enough" is how that passed every run.
        assert!(
            brightest > 0.16,
            "the felt has to be visible cloth, not a black hole ({brightest})"
        );
    }

    #[test]
    fn the_felt_falls_into_shadow_at_its_edges() {
        let cloth = felt(128);
        let luma = |x: u32, y: u32| {
            let p = cloth.pixel(x, y);
            p[1].mul_add(0.72, p[0].mul_add(0.21, p[2] * 0.07))
        };
        let middle = luma(64, 64);
        let corner = luma(2, 2);
        assert!(
            middle > corner * 1.5,
            "the middle of the table should be the lit part: {middle} vs {corner}"
        );
    }

    /// A seat colour with all three channels far apart, so a test can tell
    /// which of them a pixel actually got.
    const ACCENT: [f32; 3] = [0.90, 0.20, 0.05];

    #[test]
    fn only_the_rim_of_a_mat_carries_the_seats_colour() {
        let mat = seat_mat(128, 64, 0.18, 0.05, ACCENT, false);
        // The field is the seat's *ground*, not the seat's colour: it stays
        // white so the material's neutral brightness leaves it felt, and a
        // player reads a coloured border around their board rather than a
        // solid sheet of gold with their cards lying on it.
        let field = mat.pixel(64, 32);
        for (channel, got) in field.iter().take(3).enumerate() {
            assert!(
                (got - 1.0).abs() < 1e-3,
                "channel {channel} of the field is {got}, not white"
            );
        }
        // And the rim is the accent — stated as "nearer the accent than the
        // white it is mixed with" rather than as an equality, because the
        // rim is a gradient and no single pixel of it is the pure colour.
        // The property that matters is that a seat is *nameable* from across
        // the table, and a mix that landed on white's side of halfway would
        // not be.
        let rim = mat.pixel(64, 0);
        for (channel, want) in ACCENT.iter().enumerate() {
            let got = rim[channel];
            let halfway = f32::midpoint(1.0, *want);
            assert!(
                got < halfway,
                "channel {channel} of the rim is {got}, nearer white than \
                 the accent's {want}"
            );
        }
        // The accent's own shape survives the mix: this seat's colour is
        // red-dominant and must not come back grey.
        assert!(
            rim[0] > rim[1] && rim[1] > rim[2],
            "the rim lost the accent's ordering: {rim:?}"
        );
    }

    #[test]
    fn a_seat_mat_is_a_rounded_rectangle_with_a_rim() {
        let mat = seat_mat(128, 64, 0.18, 0.05, ACCENT, false);
        // Corners are cut away, so a mat never reads as a plain box.
        assert!(mat.pixel(0, 0)[3] < 1e-6, "the corner is rounded off");
        assert!(mat.pixel(127, 63)[3] < 1e-6, "and so is the opposite one");
        // The rim is brighter than the field it encloses.
        let edge = mat.pixel(64, 1)[3];
        let field = mat.pixel(64, 32)[3];
        assert!(
            edge > field,
            "the rim should draw the zone's boundary: {edge} vs {field}"
        );
        assert!(field > 0.0, "the mat itself is visible, not just its rim");
    }

    #[test]
    fn a_seat_mat_shows_where_its_lanes_are() {
        // Tall for the reason `the_mat_fences_its_bands_where_the_layout_put_them`
        // gives below, which this test was the one to be *caught* by: it read
        // 96 rows while `MAT_SEAM_WIDTH` was 0.014, so a hairline was 1.3 rows
        // and the shelf's fence came out one level of 255 above a lane seam.
        // Halving the width to 0.0075 took the fence to `max(h · w, 1.0)` — a
        // single row, the same row the seam gets — and the two arrived at
        // 12/255 apiece. Nothing about the mat had stopped being true; the
        // measurement had stopped being able to see it, which is worth one
        // line of comment because the first instinct was to loosen the claim.
        const H: u32 = 512;
        let mat = seat_mat(256, H, 0.1, 0.03, ACCENT, false);
        // The seam belongs *on* the boundary between two lanes, not in the
        // middle of one. Drawn mid-lane it splits every row down its own
        // centre and tells a player the opposite of the truth about where
        // their creatures end.
        //
        // The boundaries are computed rather than written down, because the
        // ledge moved them: three lanes share what the shelf leaves, so a
        // test that sampled thirds was reading the middle of the near lane
        // and passing on a mat with no seams drawn at all.
        #[expect(clippy::cast_possible_truncation, reason = "a row of a texture")]
        let row = |v: f32| (v * H as f32) as u32;
        let lanes = LEDGE_FRAC + COMBAT_FRAC;
        let step = LANE_FRAC;
        let seam = mat.pixel(128, row(lanes + step))[3];
        let mid_lane = mat.pixel(128, row(lanes + step * 0.5))[3];
        assert!(
            seam > mid_lane,
            "the lane seam should be visible: {seam} vs {mid_lane}"
        );
        // And it is a hairline: a few rows apart it is already gone.
        let past = mat.pixel(128, row(lanes + step) + 4)[3];
        assert!(
            past < seam,
            "the seam should be a line, not a band: {seam} then {past}"
        );
        // The ledge is a band of its own at the centre-facing edge, quieter
        // than every lane behind it and fenced off by the brightest seam on
        // the mat.
        let ledge = mat.pixel(128, row(lanes * 0.5))[3];
        assert!(
            ledge < mid_lane,
            "the ledge should be dimmer than the creature lane: {ledge} vs \
             {mid_lane}"
        );
        let fence = mat.pixel(128, row(LEDGE_FRAC))[3];
        assert!(
            fence > seam,
            "the ledge's own seam should be the plainer of the two: {fence} \
             vs {seam}"
        );
    }

    /// The shelf changes ends; the lanes do not.
    ///
    /// A seat across the table has its board drawn upside-down, so its bar
    /// goes at the far end of the mat and is above its creatures on the one
    /// screen there is. What must *not* move with it is the lane order —
    /// creatures nearest the middle of the table at every seat — because
    /// that is where the cards stand and no card turns round when the ink
    /// does. The two are one flag apart in the shader and it would be a
    /// one-character mistake to flip the uv instead, which is exactly what
    /// this catches: it reads the same three rows out of both mats and asks
    /// that the veils match end to end while the shelf does not.
    #[test]
    fn a_flipped_shelf_takes_the_other_end_and_leaves_the_lanes_alone() {
        const H: u32 = 96;
        let inner = seat_mat(256, H, 0.1, 0.03, ACCENT, false);
        let outer = seat_mat(256, H, 0.1, 0.03, ACCENT, true);
        #[expect(clippy::cast_possible_truncation, reason = "a row of a texture")]
        let row = |v: f32| (v * H as f32) as u32;
        let span = 1.0 - LEDGE_FRAC;
        // Where the shelf *ends* rather than what it is veiled with, because
        // a `Texture` is eight bits a channel and the shelf's own veil and
        // the quietest lane's are 0.0060 and 0.0080 — both 2/255, and the
        // same pixel. The fence between the shelf and the lanes is the
        // brightest seam on a mat and lands at 12/255 on a mat this shallow,
        // so *that* is what a test can read: it is at one end of one mat and
        // at the other end of the other. What it may **not** be read against
        // here is a lane seam, which arrives at the same 12: that comparison
        // needs the rows `a_seat_mat_shows_where_its_lanes_are` spends.
        let fence = |mat: &Texture, at: f32| mat.pixel(128, row(at))[3];
        assert!(
            fence(&inner, LEDGE_FRAC) > fence(&inner, span),
            "the unflipped shelf is fenced off at the centre-facing end"
        );
        assert!(
            fence(&outer, span) > fence(&outer, LEDGE_FRAC),
            "the flipped shelf is fenced off at the outer end"
        );
        // And the creature lane is nearest the middle of the table on both,
        // which is the half that must *not* move: on the flipped mat it
        // starts at the very edge, where the unflipped one has its shelf.
        let creature = MARGIN_FRAC + LANE_FRAC * 0.5;
        assert!(
            outer.pixel(128, row(creature))[3] > inner.pixel(128, row(creature))[3],
            "the flipped mat plays creatures where the unflipped one writes"
        );
        // Read outwards, the three lanes never brighten. Stated as "never
        // brighter" with one strict step across the whole run, because two
        // neighbouring lanes are 0.0135 and 0.0105 and eight bits cannot
        // always tell them apart either.
        for (mat, first, name) in [
            (&inner, LEDGE_FRAC, "inner"),
            (&outer, MARGIN_FRAC, "outer"),
        ] {
            let lane =
                |i: f32| mat.pixel(128, row((i + 0.5).mul_add(LANE_FRAC, first + COMBAT_FRAC)))[3];
            let (near, mid, far) = (lane(0.0), lane(1.0), lane(2.0));
            assert!(
                near >= mid && mid >= far && near > far,
                "the {name} mat's lanes should dim outwards: {near}, {mid}, {far}"
            );
        }
    }

    /// The bands the mat draws are the bands the layout laid out.
    ///
    /// Both halves of this existed and neither could see the other. A mat is
    /// banded in fractions of its own depth, and the depth it is *drawn* at
    /// is the playing extent plus a printed border only the renderer knew
    /// about — so every fraction was taken over a rectangle 18.5% too
    /// shallow. The shelf came out 0.46 units from where the geometry had
    /// reserved it, with the seat's bar following the geometry faithfully off
    /// its own ledge and onto the creature lane; the two lane seams came out
    /// 0.06 and 0.25 units from the rows of cards they are there to fence.
    /// Nothing said so, because nothing had ever measured the drawn mat
    /// against the layout in the same unit.
    ///
    /// This does, and off the **texture** rather than off the constants the
    /// texture was built from: a band written down twice is the mistake, so
    /// reading the number back out of one of the copies would only ask
    /// whether it equalled itself.
    #[test]
    fn the_mat_fences_its_bands_where_the_layout_put_them() {
        const H: u32 = 512;
        // Tall, because a fence is a hairline: `MAT_SEAM_WIDTH` of a mat
        // 96 rows deep is one row, and one row cannot be told from its
        // neighbour.
        let lane = (crate::layout::POD_DEPTH - MAT_LEDGE - crate::layout::STAGE_STEP) / 3.0;
        #[expect(clippy::cast_possible_truncation, reason = "a row of a texture")]
        let row = |t: f32| (t / MAT_DRAWN_DEPTH * H as f32) as u32;
        // Measured from the mat's centre-facing edge, in table units, which
        // is the unit `SeatSlot::ledge_corners` and `lane_center` answer in.
        // The shelf is the border plus `MAT_LEDGE`, and the three lanes are
        // each exactly `SeatSlot::lane_height` — the same rectangle a card is
        // placed against.
        //
        // Both mats, because the shelf changes ends and the lanes do not: a
        // near seat spends a border *and* a shelf before its creature row
        // starts, a far seat spends only the border and meets its shelf at
        // the other end. Sampling one of them leaves the other's arithmetic
        // held by nothing — and the mutant that reads the lanes' start as a
        // plain `LEDGE_FRAC`-or-zero is algebraically right on the near mat,
        // so it is exactly the one a single-mat test lets through.
        for ledge_outer in [false, true] {
            let mat = seat_mat(256, H, 0.1, 0.02, ACCENT, ledge_outer);
            let lanes = MAT_MARGIN + if ledge_outer { 0.0 } else { MAT_LEDGE };
            let fence = if ledge_outer {
                lanes + crate::layout::STAGE_STEP + lane * 3.0
            } else {
                lanes
            };
            for (what, at) in [
                (
                    "the first lane seam",
                    lanes + crate::layout::STAGE_STEP + lane,
                ),
                (
                    "the second lane seam",
                    lanes + crate::layout::STAGE_STEP + lane * 2.0,
                ),
                ("the shelf's own fence", fence),
            ] {
                let here = mat.pixel(128, row(at))[3];
                for away in [-12, 12] {
                    let there = mat.pixel(128, row(at).saturating_add_signed(away))[3];
                    assert!(
                        here > there,
                        "on the {} seat's mat {what} belongs {at} table units \
                         in from the centre-facing edge, which is row {}: it \
                         reads {here} there and {there} a dozen rows {}",
                        if ledge_outer { "far" } else { "near" },
                        row(at),
                        if away < 0 { "before" } else { "after" }
                    );
                }
            }
        }
    }

    #[test]
    fn the_glow_fades_to_nothing_at_its_edge() {
        let g = glow(64);
        assert!(g.pixel(32, 32)[3] > 0.9, "brightest in the middle");
        assert!(g.pixel(0, 32)[3] < 1e-6, "and gone by the edge");
        for (_, _, px) in pixels(&g) {
            assert!((px[0] - 1.0).abs() < 1e-6, "white, so it can be tinted");
        }
    }

    #[test]
    fn a_card_shadow_is_densest_where_the_card_touches_the_table() {
        let s = card_shadow(128, 160, 0.15, 0.1);
        assert!(s.pixel(64, 80)[3] > 0.5, "no shadow under the card");
        // Out through the falloff: fading, and monotonically.
        let mut previous = 1.0;
        for y in (0..19).rev() {
            let alpha = s.pixel(64, y)[3];
            assert!(
                alpha <= previous,
                "the falloff runs the wrong way at row {y}"
            );
            previous = alpha;
        }
    }

    /// A quad whose texture is still dark at its own edge draws as a square,
    /// and a square shadow under a rounded card is the most visible bug there
    /// is.
    #[test]
    fn a_card_shadow_reaches_nothing_before_its_own_edge() {
        let s = card_shadow(128, 160, 0.15, 0.1);
        for x in 0..128 {
            assert!(
                s.pixel(x, 0)[3] < 1e-6 && s.pixel(x, 159)[3] < 1e-6,
                "the shadow is still dark at column {x} of its own edge"
            );
        }
        for y in 0..160 {
            assert!(s.pixel(0, y)[3] < 1e-6 && s.pixel(127, y)[3] < 1e-6);
        }
    }

    #[test]
    fn the_hearth_is_brightest_in_the_middle_and_gone_at_its_edge() {
        let h = hearth(256, HEARTH_INNER, HEARTH_OUTER);
        // Faint on purpose: the pool is half what it shipped as, because at
        // 0.12 the candle warmth over `felt`'s green made the middle of the
        // table read olive. What is asserted is that the lamp is *there*.
        assert!(h.pixel(128, 128)[3] > 0.03, "no light in the middle");
        for x in 0..256 {
            assert!(h.pixel(x, 0)[3] < 1e-6, "still lit at the texture's edge");
            assert!(h.pixel(0, x)[3] < 1e-6);
        }
    }

    #[test]
    fn the_arcane_ring_is_where_it_was_asked_for() {
        let h = hearth(512, HEARTH_INNER, HEARTH_OUTER);
        // Sampled *between* two ticks: on a tick every radius in the band is
        // bright, and the hairline would have nothing to stand out from.
        let at = |r: f32| {
            let angle = (360.0 / f32::from(HEARTH_TICKS) / 2.0).to_radians();
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (x, y) = (
                (256.0 + r * 256.0 * angle.cos()) as u32,
                (256.0 + r * 256.0 * angle.sin()) as u32,
            );
            h.pixel(x.min(511), y.min(511))[3]
        };
        // The hairline is a local maximum, which is what a line *is*. There
        // is one, on the outer edge: two of them with ticks in between drew
        // three concentric circles and read as a dial.
        let on = at(HEARTH_OUTER);
        assert!(
            on > at(HEARTH_OUTER - 0.04) && on > at(HEARTH_OUTER + 0.04),
            "no hairline at the ring's edge: {} / {on} / {}",
            at(HEARTH_OUTER - 0.04),
            at(HEARTH_OUTER + 0.04)
        );
        // And the band inside it is lifted above the felt outside it, at
        // radii where the pool alone would have it the other way round.
        let (inside, outside) = (at(HEARTH_INNER + 0.04), at(HEARTH_OUTER + 0.1));
        assert!(
            inside > outside,
            "the band is not there: {inside} vs {outside}"
        );
    }

    #[test]
    fn the_ring_carries_its_ticks_all_the_way_round() {
        let h = hearth(512, HEARTH_INNER, HEARTH_OUTER);
        let mid = f32::midpoint(HEARTH_INNER, HEARTH_OUTER);
        let at = |degrees: f32| {
            let angle = degrees.to_radians();
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (x, y) = (
                (256.0 + mid * 256.0 * angle.cos()) as u32,
                (256.0 + mid * 256.0 * angle.sin()) as u32,
            );
            h.pixel(x.min(511), y.min(511))[3]
        };
        // Eight of them — a compass, not a clock — so every forty-fifth
        // degree is a mark and the angle halfway between two is not.
        let step = 360.0 / f32::from(HEARTH_TICKS);
        for mark in 0..HEARTH_TICKS {
            let on = at(f32::from(mark) * step);
            let off = at(f32::from(mark).mul_add(step, step * 0.5));
            assert!(on > off, "tick {mark} is missing: {on} vs {off}");
        }
    }

    /// The pool is candlelight and the inlay is gilt. Neither may go blue:
    /// a cold light over a green table makes every card's colour identity a
    /// guess, which is the one thing the whole unlit design exists to avoid.
    #[test]
    fn nothing_in_the_hearth_is_a_cold_colour() {
        for (_, _, px) in pixels(&hearth(64, HEARTH_INNER, HEARTH_OUTER)) {
            if px[3] < 1e-6 {
                continue;
            }
            assert!(px[0] >= px[1] && px[1] >= px[2], "a cold pixel: {px:?}");
        }
    }

    #[test]
    fn a_card_shadow_is_black_so_it_only_ever_darkens_the_table() {
        for (_, _, px) in pixels(&card_shadow(32, 40, 0.15, 0.1)) {
            assert!(px[0] < 1e-6 && px[1] < 1e-6 && px[2] < 1e-6);
            assert!(px[3] <= 0.56, "dense enough to read as a hole in the table");
        }
    }

    /// Combat is the loudest thing the table says, and damage is its peak.
    /// A rise that did not peak there would be decoration rather than a
    /// warning.
    #[test]
    fn the_light_rises_into_combat_and_peaks_at_damage() {
        use baylee_view::Step;
        let energy = |step| phase_light(step).energy;
        assert!(energy(Step::Main) < energy(Step::CombatBegin));
        assert!(energy(Step::CombatBegin) < energy(Step::DeclareAttackers));
        assert!(energy(Step::DeclareAttackers) < energy(Step::DeclareBlockers));
        assert!(energy(Step::DeclareBlockers) < energy(Step::CombatDamage));
        assert!(
            (energy(Step::CombatDamage) - 1.0).abs() < f32::EPSILON,
            "damage is the top of the scale, so nothing above it is wasted"
        );
        assert!(energy(Step::CombatEnd) < energy(Step::DeclareAttackers));
    }

    /// A main phase is where a player reads their own board, so the table
    /// stays the colour it was generated as and gets out of the way. The
    /// wash is not switched off — that would make the main phase a visible
    /// gap in the arc — but it is the lamp's own colour and nearly nothing.
    #[test]
    fn a_main_phase_barely_washes_at_all() {
        let main = phase_light(baylee_view::Step::Main);
        assert!(main.energy < 0.2, "not {}", main.energy);
        assert!(
            main.rgb[0] > main.rgb[2],
            "still lamplight, not a colour laid over it"
        );
    }

    /// Every step has a light and none of them is out of range. The match is
    /// exhaustive by construction; this is about the numbers in it, which are
    /// hand-written and easy to fat-finger.
    #[test]
    fn every_step_is_lit_and_in_range() {
        use baylee_view::Step;
        for step in [
            Step::Untap,
            Step::Upkeep,
            Step::Draw,
            Step::Main,
            Step::CombatBegin,
            Step::DeclareAttackers,
            Step::DeclareBlockers,
            Step::CombatDamageFirst,
            Step::CombatDamage,
            Step::CombatEnd,
            Step::End,
            Step::Cleanup,
        ] {
            let light = phase_light(step);
            assert!(
                (0.0..=1.0).contains(&light.energy),
                "{step:?} has energy {}",
                light.energy
            );
            for channel in light.rgb {
                assert!((0.0..=1.0).contains(&channel), "{step:?} is out of gamut");
            }
            // Desaturated on purpose: these are lamps over a table, and a
            // fully saturated wash over card art makes colour identity harder
            // to read, which is the one thing the table must not do.
            let low = light.rgb.iter().copied().fold(f32::MAX, f32::min);
            let high = light.rgb.iter().copied().fold(0.0_f32, f32::max);
            assert!(
                high - low < 0.8,
                "{step:?} is a filter, not a lamp: {:?}",
                light.rgb
            );
        }
    }

    /// The beginning and the end of a turn must not read as the same moment.
    #[test]
    fn the_turn_does_not_start_and_finish_in_one_colour() {
        let dawn = phase_light(baylee_view::Step::Upkeep).rgb;
        let dusk = phase_light(baylee_view::Step::End).rgb;
        let apart: f32 = (0..3).map(|i| (dawn[i] - dusk[i]).abs()).sum();
        assert!(apart > 0.3, "{dawn:?} and {dusk:?} are the same lamp");
    }

    /// Parchment is a sheet, not a rectangle of paint.
    ///
    /// Bounded on both sides, which is the lesson the felt taught: it was
    /// authored four times too dark and a one-sided "dark enough" assertion
    /// let it through. A sheet has to be light enough to take ink and not so
    /// light that it glares beside the cards, it has to lose the light at its
    /// rim, and the grain has to be there without being a pattern.
    #[test]
    fn a_sheet_of_parchment_is_warm_lit_and_grained() {
        let sheet = parchment(256);
        let middle = sheet.pixel(128, 128);
        assert!(
            (0.72..0.94).contains(&middle[0]),
            "the middle of the sheet is {middle:?}, which is not parchment"
        );
        assert!(
            middle[0] > middle[1] && middle[1] > middle[2],
            "the sheet is {middle:?} — parchment is warm, and that is not"
        );
        assert!(
            (middle[3] - 1.0).abs() < 1e-6,
            "the sheet is {:.2} opaque, and a question read through a card is \
             a question nobody answers",
            middle[3]
        );

        let rim = sheet.pixel(2, 128);
        // Both ends of it, and both were paid for. The upper bound is the
        // original: a sheet that goes dark at its edge is a hole, not paper.
        // The lower one was 0.05 and had to come down, because that much
        // fall-off is *visible as a seam* — a UI image covers a node's
        // content box and not its padding, so the darkening showed up as a
        // rectangle inside the slip. It still has to be there: a sheet with
        // no edge at all reads as a flat fill.
        assert!(
            rim[0] < middle[0] - 0.02 && rim[0] > middle[0] - 0.30,
            "the rim is {:.3} against the middle's {:.3} — a sheet loses the \
             light at its edge, and only a little",
            rim[0],
            middle[0]
        );

        // Grain: neighbouring pixels differ, and never by so much that the
        // sheet reads as noise.
        let mut most: f32 = 0.0;
        let mut moved = 0_u32;
        for x in 100..156 {
            let step = (sheet.pixel(x, 128)[0] - sheet.pixel(x + 1, 128)[0]).abs();
            most = most.max(step);
            moved += u32::from(step > 1.0 / 255.0);
        }
        assert!(
            moved > 20,
            "only {moved} of 56 steps across the sheet moved"
        );
        assert!(most < 0.06, "the grain jumps {most:.3} between neighbours");
    }
}
