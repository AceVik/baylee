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
    const DEEP: [f32; 3] = [0.035, 0.058, 0.045];
    /// The cloth's own colour, before wear.
    const CLOTH: [f32; 3] = [0.098, 0.165, 0.122];
    /// Where the table has been leaned on for years.
    const WORN: [f32; 3] = [0.140, 0.215, 0.162];

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
            // Quiet, not absent. The mat's job is to say where a seat's
            // ground ends: everything on it — cards, rims, the glow — has to
            // stay louder, and a mat nobody can see is not quiet, it is
            // missing.
            let base = [0.150, 0.120, 0.095][lane];
            // A hairline *between* lanes, so the rows separate without a
            // border drawn around each one. Measured in pixels from the two
            // boundaries: expressed as a fraction of a lane it comes out
            // under a pixel wide on a mat this shallow and never appears.
            let seam_width = (h * 0.014).max(1.0);
            let seam = [h / 3.0, h * 2.0 / 3.0]
                .iter()
                .map(|edge| (py - edge).abs())
                .fold(f32::MAX, f32::min);
            let seam = (1.0 - seam / seam_width).clamp(0.0, 1.0) * 0.08;

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
/// texture's half-width; the light pool fills the whole thing.
#[must_use]
pub fn hearth(size: u32, inner: f32, outer: f32) -> Texture {
    /// The lamp's colour: candle, not daylight.
    const WARM: [f32; 3] = [1.0, 0.86, 0.62];
    /// The inlay's: old gilt, dim enough to sit under the cards.
    const GILT: [f32; 3] = [0.86, 0.72, 0.40];

    let mut texture = Texture::blank(size, size);
    let extent = size as f32;
    // One tick every 15°, so the ring reads as a dial rather than as a
    // circle somebody drew twice.
    let ticks = 24.0;
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / extent * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / extent * 2.0 - 1.0;
            let radius = u.hypot(v);
            if radius > 1.0 {
                continue;
            }
            // The pool: brightest at the middle, gone before the edge, so
            // the quad never shows as a square against the felt.
            let pool = (1.0 - radius).clamp(0.0, 1.0).powf(2.2) * 0.12;

            // Two hairlines bounding the band, and the band itself barely
            // lifted — an inlay, not a painted circle.
            let line = [inner, outer]
                .iter()
                .map(|edge| (radius - edge).abs())
                .fold(f32::MAX, f32::min);
            let hairline = (1.0 - line / 0.006).clamp(0.0, 1.0) * 0.30;
            let inside_band = radius > inner && radius < outer;
            let band = f32::from(u8::from(inside_band)) * 0.045;

            // Ticks: short radial marks across the band, deterministic and
            // evenly spaced.
            let angle = v.atan2(u);
            let phase = (angle / TAU * ticks).fract().abs();
            let near_tick = phase.min(1.0 - phase);
            let tick = if inside_band {
                (1.0 - near_tick / 0.06).clamp(0.0, 1.0) * 0.22
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
        let h = hearth(256, 0.55, 0.72);
        assert!(h.pixel(128, 128)[3] > 0.1, "no light in the middle");
        for x in 0..256 {
            assert!(h.pixel(x, 0)[3] < 1e-6, "still lit at the texture's edge");
            assert!(h.pixel(0, x)[3] < 1e-6);
        }
    }

    #[test]
    fn the_arcane_ring_is_where_it_was_asked_for() {
        let h = hearth(512, 0.55, 0.72);
        // Sampled *between* two ticks: on a tick every radius in the band is
        // bright, and the hairlines would have nothing to stand out from.
        let at = |r: f32| {
            let angle = (360.0f32 / 24.0 / 2.0).to_radians();
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (x, y) = (
                (256.0 + r * 256.0 * angle.cos()) as u32,
                (256.0 + r * 256.0 * angle.sin()) as u32,
            );
            h.pixel(x.min(511), y.min(511))[3]
        };
        // Each hairline is a local maximum, which is what a line *is*.
        for edge in [0.55_f32, 0.72] {
            let on = at(edge);
            assert!(
                on > at(edge - 0.04) && on > at(edge + 0.04),
                "no hairline at {edge}: {} / {on} / {}",
                at(edge - 0.04),
                at(edge + 0.04)
            );
        }
        // And the band between them is lifted above the felt outside it, at
        // radii where the pool alone would have it the other way round.
        assert!(
            at(0.66) > at(0.78),
            "the band is not there: {} vs {}",
            at(0.66),
            at(0.78)
        );
    }

    #[test]
    fn the_ring_carries_its_ticks_all_the_way_round() {
        let h = hearth(512, 0.55, 0.72);
        let at = |degrees: f32| {
            let angle = degrees.to_radians();
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (x, y) = (
                (256.0 + 0.635 * 256.0 * angle.cos()) as u32,
                (256.0 + 0.635 * 256.0 * angle.sin()) as u32,
            );
            h.pixel(x.min(511), y.min(511))[3]
        };
        // Twenty-four of them, so every fifteenth degree is a mark and every
        // seventh-and-a-half is not.
        for step in 0..24 {
            let on = at(step as f32 * 15.0);
            let off = at(step as f32 * 15.0 + 7.5);
            assert!(on > off, "tick {step} is missing: {on} vs {off}");
        }
    }

    /// The pool is candlelight and the inlay is gilt. Neither may go blue:
    /// a cold light over a green table makes every card's colour identity a
    /// guess, which is the one thing the whole unlit design exists to avoid.
    #[test]
    fn nothing_in_the_hearth_is_a_cold_colour() {
        for (_, _, px) in pixels(&hearth(64, 0.55, 0.72)) {
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
}
