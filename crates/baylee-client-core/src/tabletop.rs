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
                // Dimmed from 0.55: the wheel is orientation, and it was
                // competing with the cards for the eye.
                alpha = alpha.max(glow * 0.35);
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
pub fn seat_mat(width: u32, height: u32, radius: f32, rim: f32, accent: [f32; 3]) -> Texture {
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
            //
            // Lowered twice, for the same reason each time, and the second
            // time is the one worth remembering. These numbers are an
            // **alpha** on white, and an alpha is linear light: 0.050 of it
            // is not a five-percent tint, it is display 0.24 laid over
            // whatever is beneath. Against a dark green felt that was a
            // modest lift and it was tuned and accepted there. Against the
            // timber the table is cut from now — measured at `(42, 29, 21)`
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
            let base = [0.0135, 0.0105, 0.0080][lane];
            // A hairline *between* lanes, so the rows separate without a
            // border drawn around each one. Measured in pixels from the two
            // boundaries: expressed as a fraction of a lane it comes out
            // under a pixel wide on a mat this shallow and never appears.
            let seam_width = (h * 0.014).max(1.0);
            let seam = [h / 3.0, h * 2.0 / 3.0]
                .iter()
                .map(|edge| (py - edge).abs())
                .fold(f32::MAX, f32::min);
            let seam = (1.0 - seam / seam_width).clamp(0.0, 1.0) * 0.036;

            // The rim: the one part that is meant to be seen from across the
            // table, since it is what carries the seat's colour.
            let falloff = (1.0 - inset / edge).clamp(0.0, 1.0);
            let border = falloff.powf(1.3);
            // And a soft feather so the mat has no jaggies.
            let coverage = (0.5 - outside).clamp(0.0, 1.0);

            let value = base + seam + border * 0.62;
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
            let hue = falloff.powf(0.55);
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
/// only, never the felt (too much of the screen) and never the medallion —
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
/// survivable the moment it fills the channel and runs up against the mat
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
/// still holds: the channel is where no card lies, but bloom does not know
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

/// One seat's ground, as the channel generator needs to see it.
///
/// The resin is defined by what it is *not*: the channel is whatever the mats
/// leave, so the only thing [`channel`] needs to know about a seat is the
/// rectangle it occupies. Deliberately not `SeatSlot` — this module has never
/// depended on the layout, and a shape is all there is to say here.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bank {
    /// Centre of the mat in table space.
    pub center: Vec2,
    /// Half the mat's extent, before the margin is added.
    pub half_extent: Vec2,
    /// The rotation that turns the mat towards its owner.
    pub facing: f32,
}

/// How deep the resin reads at its deepest, in table units.
///
/// The channel between two seats is [`layout::CENTRE_GAP`] across, so half of
/// that is the most distance any point in it can have from a bank — this is
/// what maps that distance onto the shading's full range, and a wider pool at
/// a crowded table simply saturates rather than going on getting darker.
///
/// [`layout::CENTRE_GAP`]: crate::layout::CENTRE_GAP
pub const RESIN_DEPTH: f32 = 1.6;

/// A rectangle's signed distance: negative inside, positive outside.
fn sd_box(p: Vec2, half: Vec2) -> f32 {
    let d = p.abs() - half;
    d.max(Vec2::ZERO).length() + d.x.max(d.y).min(0.0)
}

/// How far inside an ellipse a point is, positive within and negative beyond.
///
/// The implicit function normalised by its own gradient, which is the cheap
/// approximation rather than the exact distance — exact would need a root
/// find per pixel, and this is used to fade a pool's outer edge into wood
/// where nothing measures it.
fn inside_ellipse(p: Vec2, radius: Vec2) -> f32 {
    let scaled = Vec2::new(p.x / radius.x, p.y / radius.y);
    let gradient = Vec2::new(p.x / (radius.x * radius.x), p.y / (radius.y * radius.y)).length();
    if gradient < 1e-6 {
        return radius.min_element();
    }
    (1.0 - scaled.length()) / gradient
}

/// Grain lines per table unit: a period of about **1.6 cards**.
///
/// Coarser than the wood in the photographs, and deliberately. This is a
/// period, not a preference — `the_timber_grain_is_coarser_than_a_card` fails
/// below roughly one card, because that is the point where blocking the image
/// at card size averages the figure away, which is exactly what the eye does
/// when a card is sitting on it. Fifteen lines across a table is what is left,
/// and it still reads as wood because a line here is long rather than fine.
pub const GRAIN_LINES: f32 = 0.62;

/// How far a grain line wanders across the table, in units. Enough that the
/// lines are not a corduroy, far too little to make them a pattern.
pub const GRAIN_WANDER: f32 = 1.8;

/// The trough between two grain lines.
pub const WOOD_DEEP: [f32; 3] = [0.085, 0.058, 0.044];
/// The wood's own colour.
pub const WOOD_BASE: [f32; 3] = [0.170, 0.118, 0.084];
/// The lifted edge of a grain line, where a plane has caught the figure.
pub const WOOD_PALE: [f32; 3] = [0.255, 0.185, 0.132];

/// The timber the table is cut from: a dark, warm wood with its grain running
/// the long way.
///
/// Three rules, each of which the rejected granite broke.
///
/// **The grain is coarser than a card.** Granite's speckle was finer, which
/// is what made it read as terrazzo and what made it alias: detail below the
/// size of the thing standing on it competes with that thing and wins,
/// because there is more of it. `span` is therefore the *world* size this
/// texture covers, not a pixel count — the grain keeps its real-world period
/// however large the table is cut or however coarsely the image is sampled.
///
/// **The grain runs parallel to the long axis.** Diagonal grain under rows of
/// cards is noise; grain along the rows is a ruler they line up against.
///
/// **The dark is warm.** Brown-black, never blue-black: a cold black eats the
/// identity of black and blue cards, which are the two the table can least
/// afford to swallow. Every sample here keeps red above blue, and
/// `the_timber_is_a_warm_dark_and_never_a_cold_one` is that as a build
/// failure.
#[must_use]
pub fn timber(width: u32, height: u32, span: Vec2) -> Texture {
    let (deep, wood, pale) = (WOOD_DEEP, WOOD_BASE, WOOD_PALE);

    let mut texture = Texture::blank(width, height);
    for y in 0..height {
        for x in 0..width {
            // Table space, so everything below is in units and not in pixels.
            let u = (x as f32 + 0.5) / width as f32;
            let v = (y as f32 + 0.5) / height as f32;
            let p = Vec2::new((u - 0.5) * span.x, (v - 0.5) * span.y);

            // Stretched hard along x: a feature is many times longer than it
            // is wide, which is the whole difference between grain and noise.
            // Two octaves, not three. The third sits at a period of about a
            // third of a card, which is speckle by the definition above — it
            // buys nothing the eye can resolve past a card standing on it and
            // costs the figure that can be resolved.
            let drift = fbm(p.x * 0.055, p.y * 0.16, 0x51_a3, 2) - 0.5;
            let figure = fbm(p.x * 0.10, p.y * 0.42, 0x77_c1, 2) - 0.5;

            // The lines themselves. `sin` rather than a sawtooth so a line
            // has two soft shoulders instead of one hard step.
            let phase = (p.y + drift * GRAIN_WANDER * 2.0) * GRAIN_LINES * TAU;
            let ring = 0.5 + 0.5 * phase.sin();
            // Sharpened towards the trough: real figure is mostly pale wood
            // with narrow dark lines in it, not an even wave.
            let ring = ring * ring;

            let base = mix(deep, wood, ring);
            let colour = mix(base, pale, (figure + 0.5) * ring * 0.55);
            texture.put(x, y, [colour[0], colour[1], colour[2], 1.0]);
        }
    }
    texture
}

/// The channel: where the resin is, how deep, and which way it runs.
///
/// This is the **negative form of the layout** and nothing else — the resin
/// is wherever a card never lies. At two seats that is a band between two
/// banks and reads as a river; at five it is a pool with five shores. One
/// rule, one shape, no second design, which is what makes the ring survivable
/// at all: a river needs two ends and a ring has none.
///
/// The result is a field rather than a picture, and the shader reads all four
/// channels:
///
/// - **red** — how deep, `0.0` at a bank and `1.0` at [`RESIN_DEPTH`]. Depth
///   is what carries the illusion of a pour with a bottom to it.
/// - **green, blue** — which way the current runs, as a unit vector biased
///   into `0.0..1.0`. It is the *tangent* of the distance field, so the flow
///   follows the channel: along a duel's band, round a crowded table's pool,
///   and never in some fixed screen direction that would swim against the
///   shape it is in.
/// - **alpha** — resin or wood, with a soft edge a pixel or two wide so the
///   shore does not stair-step.
///
/// `span` is the world size of the whole slab and `pool` the semi-axes of the
/// water's outer edge — which is the *table*, not the slab. The two differ:
/// the slab runs well past the seating so no camera angle finds its edge, and
/// every unit of that overhang has to be dry timber. Deriving one from the
/// other is how a river becomes a flood.
///
/// `margin` is the clear table kept between a mat and the water — the same
/// margin the seat's own rim is drawn at, so the resin starts where the mat's
/// ground visibly stops rather than under its edge.
#[must_use]
pub fn channel(
    width: u32,
    height: u32,
    span: Vec2,
    pool: Vec2,
    banks: &[Bank],
    margin: f32,
) -> Texture {
    let cells = (width as usize) * (height as usize);
    let at = |x: u32, y: u32| -> Vec2 {
        let u = (x as f32 + 0.5) / width as f32;
        let v = (y as f32 + 0.5) / height as f32;
        Vec2::new((u - 0.5) * span.x, (v - 0.5) * span.y)
    };

    // The field first, whole, and the flow read back off it afterwards. The
    // obvious thing — sampling the field five times per pixel for a central
    // difference — is five times the work for the same numbers, and this
    // generator runs again every time the table is resized.
    let mut depth = vec![0.0_f32; cells];
    for y in 0..height {
        for x in 0..width {
            let p = at(x, y);
            let mut d = inside_ellipse(p, pool);
            for bank in banks {
                let (sin, cos) = bank.facing.sin_cos();
                let local = p - bank.center;
                let turned = Vec2::new(
                    cos.mul_add(local.x, -(sin * local.y)),
                    sin.mul_add(local.x, cos * local.y),
                );
                d = d.min(sd_box(turned, bank.half_extent + Vec2::splat(margin)));
            }
            depth[(y as usize) * (width as usize) + x as usize] = d;
        }
    }

    let step = Vec2::new(span.x / width as f32, span.y / height as f32);
    let (last_x, last_y) = (width as usize - 1, height as usize - 1);
    let read = |x: usize, y: usize| -> f32 { depth[y * (width as usize) + x] };

    let mut texture = Texture::blank(width, height);
    for y in 0..height {
        for x in 0..width {
            let (cx, cy) = (x as usize, y as usize);
            let d = read(cx, cy);
            // The field's gradient points from the nearest bank towards open
            // water, so a quarter turn from it points *along* the channel.
            let slope = Vec2::new(
                (read((cx + 1).min(last_x), cy) - read(cx.saturating_sub(1), cy)) / (2.0 * step.x),
                (read(cx, (cy + 1).min(last_y)) - read(cx, cy.saturating_sub(1))) / (2.0 * step.y),
            );
            let flow = if slope.length_squared() < 1e-8 {
                Vec2::X
            } else {
                let n = slope.normalize();
                Vec2::new(-n.y, n.x)
            };

            // A shore two texels wide, so the waterline is a line and not a
            // staircase. Anything narrower aliases at this resolution.
            let feather = step.x.max(step.y) * 2.0;
            let coverage = (d / feather).clamp(0.0, 1.0);
            texture.put(
                x,
                y,
                [
                    (d / RESIN_DEPTH).clamp(0.0, 1.0),
                    flow.x.mul_add(0.5, 0.5),
                    flow.y.mul_add(0.5, 0.5),
                    coverage,
                ],
            );
        }
    }
    texture
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::CARD_WIDTH;

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

    /// A duel's two mats, near enough to the real geometry to test against.
    fn duel_banks() -> (Vec2, [Bank; 2]) {
        let span = Vec2::new(26.6, 13.3);
        let half = Vec2::new(13.29, 2.47);
        (
            span,
            [
                Bank {
                    center: Vec2::new(0.0, -4.17),
                    half_extent: half,
                    facing: 0.0,
                },
                Bank {
                    center: Vec2::new(0.0, 4.17),
                    half_extent: half,
                    facing: PI,
                },
            ],
        )
    }

    /// Where a point of table lands in a generated field.
    fn sample(t: &Texture, span: Vec2, p: Vec2) -> [f32; 4] {
        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let x = (((p.x / span.x + 0.5) * t.width as f32) as u32).min(t.width - 1);
        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let y = (((p.y / span.y + 0.5) * t.height as f32) as u32).min(t.height - 1);
        t.pixel(x, y)
    }

    #[test]
    #[ignore = "a measurement, not an assertion; run with --ignored"]
    fn timber_at_screen_resolution_is_not_a_freeze() {
        let span = Vec2::new(34.6, 21.3);
        for w in [512u32, 1024, 1536, 2048] {
            let h = (w as f32 * span.y / span.x) as u32;
            let t = std::time::Instant::now();
            let _ = timber(w, h, span);
            let a = t.elapsed();
            let t = std::time::Instant::now();
            let _ = channel(w, h, span, span * 0.5, &duel_banks().1, 0.55);
            println!("{w}x{h}: timber {a:?}  channel {:?}", t.elapsed());
        }
    }

    #[test]
    fn the_timber_is_a_warm_dark_and_never_a_cold_one() {
        // Warm, because a cold black eats the identity of black and blue
        // cards — the two colours the table can least afford to swallow.
        // Every sample, not the average: an average stays warm while
        // individual grain lines go blue.
        let wood = timber(256, 128, Vec2::new(26.6, 13.3));
        let mut sum = 0.0;
        for (x, y, rgba) in pixels(&wood) {
            assert!(
                rgba[0] > rgba[2],
                "({x},{y}) is {rgba:?} — blue at or above red is a cold black"
            );
            sum += luma(rgba);
        }
        let mean = sum / (wood.width * wood.height) as f32;

        // Both bounds. The felt shipped four times too dark past a one-sided
        // "dark enough" assertion, and a resin table is the darker design of
        // the two — what rescues the photograph is studio light on gloss, and
        // a camera looking straight down has none of it.
        assert!(
            (0.09..=0.20).contains(&mean),
            "the timber's mean luminance is {mean}, outside the band it has to sit in"
        );
    }

    #[test]
    fn the_timber_grain_is_coarser_than_a_card() {
        // The rule the granite broke. Detail finer than the thing standing on
        // it competes with that thing and wins, because there is more of it —
        // that is what read as terrazzo, and what aliased.
        //
        // Measured rather than asserted by eye: block the image into squares
        // one card wide and compare how much contrast survives. Speckle
        // averages away to nothing at that size; long grain does not.
        let span = Vec2::new(26.6, 13.3);
        let wood = timber(256, 128, span);
        let card_px = (CARD_WIDTH / span.x * wood.width as f32).round().max(2.0);
        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let block = card_px as u32;

        let all: Vec<f32> = pixels(&wood).map(|(_, _, rgba)| luma(rgba)).collect();
        let fine = deviation(&all);

        let mut coarse = Vec::new();
        for by in (0..wood.height).step_by(block as usize) {
            for bx in (0..wood.width).step_by(block as usize) {
                let mut sum = 0.0;
                let mut count = 0.0;
                for y in by..(by + block).min(wood.height) {
                    for x in bx..(bx + block).min(wood.width) {
                        sum += luma(wood.pixel(x, y));
                        count += 1.0;
                    }
                }
                coarse.push(sum / count);
            }
        }
        // The bound is arithmetic, not taste. Averaging a wave of period `P`
        // over a box of width `W` scales it by `sinc(W/P)`, so even a perfect
        // grain at 1.6 cards keeps only `sinc(0.625) ≈ 0.47` — a threshold of
        // one half would have been unreachable by construction, which is what
        // the first draft of this test asserted. Speckle at a third of a card
        // keeps `sinc(3) ≈ 0.1`. Anywhere between is a real distinction, and
        // this sits well clear of the speckle end.
        let survives = deviation(&coarse) / fine;
        assert!(
            survives > 0.35,
            "only {survives} of the timber's contrast survives being averaged over \
             a card — that is speckle, not grain"
        );
    }

    #[test]
    fn the_channel_is_exactly_what_the_mats_leave() {
        let (span, banks) = duel_banks();
        let field = channel(256, 128, span, span * 0.5, &banks, 0.55);

        // Under a mat there is no water. This is the whole placement rule:
        // orange light under a card would falsify its colour identity, which
        // is the one thing this table may not do.
        for bank in banks {
            let a = sample(&field, span, bank.center)[3];
            assert!(
                a < 0.01,
                "a mat at {:?} is standing in water ({a})",
                bank.center
            );
        }

        // And between them there is, at its deepest in the middle.
        let middle = sample(&field, span, Vec2::ZERO);
        assert!(
            middle[3] > 0.99,
            "the channel is dry in the middle: {middle:?}"
        );
        let shore = sample(&field, span, Vec2::new(0.0, -1.6));
        assert!(
            middle[0] > shore[0],
            "the middle {} is no deeper than the shore {}",
            middle[0],
            shore[0]
        );
    }

    #[test]
    fn the_current_runs_along_the_channel_and_not_across_it() {
        // A river that flowed into its own bank would be a texture scrolling
        // in a fixed direction, which is what this field exists to avoid: the
        // flow is the *tangent* of the distance to the nearest shore, so it
        // follows whatever shape the layout leaves.
        let (span, banks) = duel_banks();
        let field = channel(256, 128, span, span * 0.5, &banks, 0.55);
        for x in [-8.0, -3.0, 0.0, 4.0, 9.0] {
            let f = sample(&field, span, Vec2::new(x, 0.0));
            let flow = Vec2::new(f[1].mul_add(2.0, -1.0), f[2].mul_add(2.0, -1.0));
            assert!(
                flow.x.abs() > flow.y.abs() * 3.0,
                "at x={x} the current runs {flow:?}, across a channel that lies along x"
            );
        }
    }

    /// Standard deviation, for the two contrast measurements above.
    fn deviation(values: &[f32]) -> f32 {
        let n = values.len() as f32;
        let mean = values.iter().sum::<f32>() / n;
        (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n).sqrt()
    }

    #[test]
    fn the_wash_never_speaks_in_a_colour_of_the_pie() {
        // The two signals on this table that a player reads without words:
        // the pie says *whose* and *which colour*, the wash says *where in
        // the turn*. They may not be the same colour. This was broken from
        // the day both existed — ember sat 1.9 degrees from the pie's red and
        // cool 7.1 from its blue — and it went unseen because the wash was a
        // small pool over a dark middle. It stops being survivable when the
        // wash fills the channel and runs up against the mat rims.
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
            // A low floor: the wheel was dimmed deliberately (see the glow's
            // 0.35), and what this asserts is that each colour is *drawn* —
            // how loudly is §1.1's business, not this test's.
            assert!(px[3] > 0.1, "colour {index} is on the wheel");
            assert_eq!(
                drawn, brightest,
                "colour {index} came out with the wrong channel on top"
            );
        }
    }

    /// A seat colour with all three channels far apart, so a test can tell
    /// which of them a pixel actually got.
    const ACCENT: [f32; 3] = [0.90, 0.20, 0.05];

    #[test]
    fn only_the_rim_of_a_mat_carries_the_seats_colour() {
        let mat = seat_mat(128, 64, 0.18, 0.05, ACCENT);
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
        let mat = seat_mat(128, 64, 0.18, 0.05, ACCENT);
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
        let mat = seat_mat(256, 96, 0.1, 0.03, ACCENT);
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
}
