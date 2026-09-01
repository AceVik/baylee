//! The table's own artwork, generated rather than shipped.
//!
//! Everything a player sees *under* the cards — the felt, the medallion
//! inlaid at the centre, the glow beneath a seat's zone — is computed here
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

/// The felt: a dark, grained cloth with a woven cross-hatch, worn lighter
/// towards the middle where the game happens and falling into shadow at the
/// edges.
///
/// It is deliberately low-contrast. Everything above it — cards, zone rims,
/// the medallion — has to stay the thing the eye lands on, and a table that
/// competes with its own cards is a table nobody can read.
#[must_use]
pub fn felt(size: u32) -> Texture {
    /// Darkest point, at the corners.
    const DEEP: [f32; 3] = [0.020, 0.030, 0.036];
    /// The cloth's own colour, before wear.
    const CLOTH: [f32; 3] = [0.055, 0.086, 0.082];
    /// Where the table has been leaned on for years.
    const WORN: [f32; 3] = [0.098, 0.130, 0.120];

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
            colour = mix(colour, DEEP, (radius * 1.05).clamp(0.0, 1.0).powf(1.35));
            let lift = (grain - 0.5).mul_add(0.028, (weave - 0.5) * 0.010);
            for channel in &mut colour {
                *channel += lift;
            }
            texture.put(x, y, [colour[0], colour[1], colour[2], 1.0]);
        }
    }
    texture
}

/// The medallion inlaid at the centre of the table: a colour wheel of five
/// glows on a ring of worn gold, transparent everywhere else.
///
/// The wheel is the one piece of ornament that is also information — it is
/// the arrangement every player already has in their head, so it orients the
/// table without a label on it.
#[must_use]
pub fn medallion(size: u32) -> Texture {
    /// Worn gold, for the rings.
    const GILT: [f32; 3] = [0.62, 0.50, 0.26];

    let mut texture = Texture::blank(size, size);
    let extent = size as f32;
    for y in 0..size {
        for x in 0..size {
            let (u, v) = (
                (x as f32 + 0.5) / extent * 2.0 - 1.0,
                (y as f32 + 0.5) / extent * 2.0 - 1.0,
            );
            let radius = (u * u + v * v).sqrt();
            if radius > 1.0 {
                continue;
            }

            let mut colour = [0.0_f32; 3];
            let mut alpha = 0.0_f32;

            // Five glows, one per colour, sitting where the wheel puts them:
            // white at the top, then clockwise. Each is a soft disc rather
            // than a hard wedge, so neighbours bleed into one another the way
            // the pie's allies do.
            for (index, hue) in PIE.iter().enumerate() {
                let at = TAU * (index as f32) / 5.0 - PI / 2.0;
                let (cx, cy) = (at.cos() * 0.62, at.sin() * 0.62);
                let distance = ((u - cx).powi(2) + (v - cy).powi(2)).sqrt();
                let glow = (1.0 - (distance / 0.42)).clamp(0.0, 1.0).powf(2.2);
                colour = mix(colour, *hue, glow.min(1.0));
                alpha = alpha.max(glow * 0.55);
            }

            // Two gilt rings: one around the wheel, one inside it. `ring`
            // peaks on the line and falls away fast on both sides.
            for (at, width, strength) in [(0.94_f32, 0.030_f32, 0.85_f32), (0.30, 0.018, 0.55)] {
                let ring = (1.0 - ((radius - at).abs() / width)).clamp(0.0, 1.0);
                let ring = ring * ring * strength;
                colour = mix(colour, GILT, ring);
                alpha = alpha.max(ring);
            }

            // A faint pool inside the inner ring, so the middle of the table
            // is not a hole.
            let pool = (1.0 - radius / 0.30).clamp(0.0, 1.0).powf(1.5) * 0.18;
            alpha = alpha.max(pool);

            // Age: the gilt is not evenly bright anywhere.
            let tarnish = fbm(u * 6.0, v * 6.0, 0x2b41, 3);
            let fade = 0.75 + tarnish * 0.45;
            texture.put(
                x,
                y,
                [
                    colour[0] * fade,
                    colour[1] * fade,
                    colour[2] * fade,
                    // Halved: the wheel is the room's lighting, not
                    // its subject.
                    alpha * fade.min(1.0) * 0.55,
                ],
            );
        }
    }
    texture
}

/// A seat's mat: the rounded rectangle its permanents are played on.
///
/// White, so the renderer can tint one texture per seat; the shape lives in
/// the alpha channel. Three bands run across it, one per lane, brightest at
/// the front where creatures stand — a player reading an opponent's board
/// should be able to see where the rows are without counting cards.
///
/// `radius` and `rim` are fractions of the shorter side.
#[must_use]
pub fn seat_mat(width: u32, height: u32, radius: f32, rim: f32) -> Texture {
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

            // Three lanes across the mat's depth. The band nearest the table
            // centre (v = 0) is the creature row.
            let v = py / h;
            #[expect(clippy::cast_possible_truncation, reason = "three lanes")]
            let lane = (v * 3.0).floor().clamp(0.0, 2.0) as usize;
            // Barely there. The mat's job is to say where a seat's ground
            // ends, not to be looked at: everything on it — cards, rims,
            // the glow — has to stay louder than the ground it stands on.
            let base = [0.052, 0.038, 0.026][lane];
            // A hairline *between* lanes, so the rows separate without a
            // border drawn around each one. Measured in pixels from the two
            // boundaries: expressed as a fraction of a lane it comes out
            // under a pixel wide on a mat this shallow and never appears.
            let seam_width = (h * 0.014).max(1.0);
            let seam = [h / 3.0, h * 2.0 / 3.0]
                .iter()
                .map(|edge| (py - edge).abs())
                .fold(f32::MAX, f32::min);
            let seam = (1.0 - seam / seam_width).clamp(0.0, 1.0) * 0.07;

            // The rim: the one part that is meant to be seen from across the
            // table, since it is what carries the seat's colour.
            let border = (1.0 - inset / edge).clamp(0.0, 1.0).powf(1.3);
            // And a soft feather so the mat has no jaggies.
            let coverage = (0.5 - outside).clamp(0.0, 1.0);

            let value = base + seam + border * 0.62;
            texture.put(x, y, [1.0, 1.0, 1.0, (value * coverage).clamp(0.0, 1.0)]);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every pixel of a texture, as `(x, y, rgba)`.
    fn pixels(t: &Texture) -> impl Iterator<Item = (u32, u32, [f32; 4])> + '_ {
        (0..t.height).flat_map(move |y| (0..t.width).map(move |x| (x, y, t.pixel(x, y))))
    }

    #[test]
    fn the_table_looks_the_same_to_everyone_at_it() {
        // No clock, no rng: two runs are the same table. Players screenshot
        // these, and a grain that moved would be the first thing anyone
        // noticed.
        assert_eq!(felt(64).rgba, felt(64).rgba);
        assert_eq!(medallion(64).rgba, medallion(64).rgba);
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

    #[test]
    fn the_medallion_is_a_disc_and_leaves_the_rest_of_the_table_alone() {
        let disc = medallion(128);
        // Outside the inscribed circle nothing is drawn at all, so the felt
        // shows through the corners of the quad it is mapped onto.
        assert!(disc.pixel(0, 0)[3] < 1e-6, "the top-left corner is clear");
        assert!(
            disc.pixel(127, 0)[3] < 1e-6,
            "the top-right corner is clear"
        );
        assert!(
            disc.pixel(0, 127)[3] < 1e-6,
            "the bottom-left corner is clear"
        );
        assert!(
            disc.pixel(127, 127)[3] < 1e-6,
            "the bottom-right corner is clear"
        );
        // And the outer gilt ring is genuinely there. Scanned rather than
        // sampled at a guessed pixel: the ring is a few pixels wide and where
        // exactly it lands is the renderer's business, not the test's.
        let rim = (0..24)
            .map(|y| disc.pixel(64, y)[3])
            .fold(0.0_f32, f32::max);
        assert!(rim > 0.2, "the medallion has a rim to sit in, not {rim}");
        assert!(
            rim < 0.55,
            "and the rim is atmosphere, not the subject of the table: {rim}"
        );
    }

    #[test]
    fn the_wheel_carries_all_five_colours() {
        let disc = medallion(256);
        // Each colour should dominate somewhere on the wheel: sample the ring
        // the glows sit on, at the angle each one was placed.
        for (index, hue) in PIE.iter().enumerate() {
            let at = TAU * (index as f32) / 5.0 - PI / 2.0;
            #[expect(clippy::cast_possible_truncation, reason = "inside a 256px image")]
            let x = (128.0 + at.cos() * 0.62 * 128.0) as u32;
            #[expect(clippy::cast_possible_truncation, reason = "inside a 256px image")]
            let y = (128.0 + at.sin() * 0.62 * 128.0) as u32;
            let px = disc.pixel(x, y);
            let brightest = hue
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(c, _)| c)
                .unwrap_or_default();
            let drawn = px[..3]
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(c, _)| c)
                .unwrap_or_default();
            assert!(px[3] > 0.2, "colour {index} is on the wheel");
            assert_eq!(
                drawn, brightest,
                "colour {index} came out with the wrong channel on top"
            );
        }
    }

    #[test]
    fn a_seat_mat_is_a_rounded_rectangle_with_a_rim() {
        let mat = seat_mat(128, 64, 0.18, 0.05);
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
        let mat = seat_mat(256, 96, 0.1, 0.03);
        // The seam belongs *on* the boundary between two lanes (a third of
        // the way down), not in the middle of one. Drawn mid-lane it splits
        // every row down its own centre and tells a player the opposite of
        // the truth about where their creatures end.
        let seam = mat.pixel(128, 32)[3];
        let mid_lane = mat.pixel(128, 16)[3];
        assert!(
            seam > mid_lane,
            "the lane seam should be visible: {seam} vs {mid_lane}"
        );
        // And it is a hairline: two rows apart it is already gone.
        let past = mat.pixel(128, 40)[3];
        assert!(
            past < seam,
            "the seam should be a line, not a band: {seam} then {past}"
        );
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
}
