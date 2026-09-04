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
            // These were three times higher, and were tuned while the whole
            // texture was tinted by the seat's accent — a dark, saturated
            // multiplier that held the field down. Against the neutral
            // brightness the material carries now the same numbers rendered
            // a mat at three times the felt's luminance: a pale concrete
            // slab that the cards on it had to compete with, which is the
            // rule above backwards. Measured against felt at `(23, 38, 28)`,
            // these put the field at about `(52, 60, 54)`.
            let base = [0.050, 0.040, 0.031][lane];
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
    /// Dawn: cool and low, for the steps nobody acts in.
    const COOL: [f32; 3] = [0.42, 0.58, 0.86];
    /// The lamp the pool is already generated as — a wash of it changes
    /// nothing, which is the point during a main phase.
    const CANDLE: [f32; 3] = [1.0, 0.86, 0.62];
    /// Combat: iron heating, not a fire alarm.
    const EMBER: [f32; 3] = [0.90, 0.34, 0.22];
    /// Dusk, for the end of a turn.
    const DUSK: [f32; 3] = [0.52, 0.40, 0.72];
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

// ------------------------------------------------------------------ the slab

/// The slab the game is played on: dark granite, polished to a glass sheen.
///
/// Opaque, and the only thing under the table that is. Everything else here
/// paints *onto* it.
///
/// Three grains and no image. The stone between them is near black and
/// cooled slightly blue, because a neutral dark grey photographs as brown
/// beside warm card art and a warm one competes with it. Through that run
/// pale feldspar, rarer warm orthoclase — the grain that stops polished
/// granite reading as poured concrete — and a few quartz veins. The polish
/// is a single broad highlight raked across the surface: a real specular
/// sweep needs a light and a normal, and this stage has neither on purpose
/// (`docs/client.md` §"The table itself"), so the sheen is painted where a
/// light would have put it.
///
/// Low contrast, for the reason the felt it replaces was: the cards have to
/// stay the thing the eye lands on.
#[must_use]
pub fn granite(width: u32, height: u32) -> Texture {
    /// The stone between the grains.
    const STONE: [f32; 3] = [0.078, 0.083, 0.098];
    /// Its darkest patches, and the edge it falls off to.
    const DEEP: [f32; 3] = [0.034, 0.037, 0.047];
    /// Feldspar: the pale grains scattered through it.
    const FELDSPAR: [f32; 3] = [0.29, 0.29, 0.32];
    /// The warm grains, sparser than the pale ones.
    const ORTHOCLASE: [f32; 3] = [0.30, 0.23, 0.22];
    /// Quartz, in the veins.
    const QUARTZ: [f32; 3] = [0.38, 0.42, 0.49];

    let mut texture = Texture::blank(width, height);
    let (w, h) = (width as f32, height as f32);
    // Noise is sampled in a square frame so the grain does not stretch with
    // the slab: a 60×44 table wearing a square texture would have grains
    // half again as wide as they are tall.
    let aspect = h / w;
    for y in 0..height {
        for x in 0..width {
            let (u, v) = (x as f32 / w, y as f32 / h);
            let (su, sv) = (u, v * aspect);

            // Big slow patches, so the slab is not uniform at arm's length.
            let patch = fbm(su * 2.6, sv * 2.6, 0x3f11, 4);
            let mut colour = mix(STONE, DEEP, (patch - 0.5).mul_add(0.9, 0.45));

            // The grains. A high-frequency field pushed through a hard knee
            // rather than added: granite's speckle is *discrete*, and noise
            // added smoothly gives fog. The knee is what makes a grain a
            // grain.
            // Frequency and texture size move together: a grain whose period
            // is under about four pixels stops being a grain and becomes
            // aliasing, which reads as a shimmer rather than as stone.
            let pale = grain_at(su, sv, 300.0, 0x7c05, 0.63, 0.20);
            let warm = grain_at(su, sv, 215.0, 0x21bd, 0.74, 0.16);
            colour = mix(colour, FELDSPAR, pale * 0.62);
            colour = mix(colour, ORTHOCLASE, warm * 0.55);

            // Veins: ridged noise on a stretched frame, so they run long and
            // thin rather than blotch. Raised to a high power because a vein
            // is a thin
            // bright line and everything short of its crest is stone.
            let field = fbm(sv.mul_add(0.8, su * 2.0), sv * 7.5, 0x58ea, 4);
            let ridge = 1.0 - (2.0f32.mul_add(field, -1.0)).abs();
            colour = mix(colour, QUARTZ, ridge.powf(15.0) * 0.34);

            // The polish: one broad rake of light across the slab.
            let along = 0.62f32.mul_add(u, 0.38 * v);
            let sheen = (-((along - 0.46) * (along - 0.46)) / 0.052).exp();
            for channel in &mut colour {
                *channel += sheen * 0.016;
            }

            // A gentle fall-off to the rim, so the slab has a body rather
            // than being a flat sheet. Gentler than the felt's vignette was:
            // this table is meant to read as lit from above, not as sitting
            // in a dark room.
            let radius = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 1.9;
            colour = mix(colour, DEEP, radius.clamp(0.0, 1.0).powf(2.4) * 0.40);

            texture.put(x, y, [colour[0], colour[1], colour[2], 1.0]);
        }
    }
    texture
}

/// One grain field: high-frequency noise through a hard knee.
///
/// `floor` is where a grain starts and `width` how quickly it reaches full
/// strength; the square at the end is what keeps them sparse, since most of
/// the field sits just over the floor and would otherwise haze the whole
/// slab.
fn grain_at(u: f32, v: f32, frequency: f32, seed: u32, floor: f32, width: f32) -> f32 {
    let n = fbm(u * frequency, v * frequency, seed, 2);
    let t = ((n - floor) / width).clamp(0.0, 1.0);
    t * t
}

// ----------------------------------------------------------------- the runes

/// Distance from `p` to the segment `a`–`b`.
fn seg_dist(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (px, py) = (p[0] - a[0], p[1] - a[1]);
    let (bx, by) = (b[0] - a[0], b[1] - a[1]);
    let len2 = bx.mul_add(bx, by * by);
    let t = if len2 <= f32::EPSILON {
        0.0
    } else {
        (px.mul_add(bx, py * by) / len2).clamp(0.0, 1.0)
    };
    (px - bx * t).hypot(py - by * t)
}

/// Distance from `p` to the arc of radius `r` about the origin, running from
/// `from` to `to` radians counter-clockwise.
///
/// Outside the sweep it is the distance to the nearer end *cap*, not to the
/// whole circle — an arc has ends, and rounding them is what stops a broken
/// ring from looking like a ring with a bite taken out of it.
fn arc_dist(p: [f32; 2], r: f32, from: f32, to: f32) -> f32 {
    let len = p[0].hypot(p[1]);
    let mut angle = p[1].atan2(p[0]);
    while angle < from {
        angle += TAU;
    }
    if angle <= to {
        return (len - r).abs();
    }
    let cap = |a: f32| (p[0] - r * a.cos()).hypot(p[1] - r * a.sin());
    cap(from).min(cap(to))
}

/// How many sigils the alphabet has.
const SIGILS: u32 = 6;

/// The distance from a point in a sigil's own square — `-1.0..1.0` on both
/// axes — to the strokes of sigil `which`.
///
/// Six shapes built from arcs and line segments, and *geometric on purpose*.
/// `docs/legal.md` §2 rules out borrowed ornament, and a fantasy rune is
/// exactly the thing that gets borrowed by accident: anything that reads as
/// Elder Futhark, as a Tolkien alphabet, or as a real sigil from a real
/// grimoire is out, however arcane it looks. A ring with a bar through it is
/// a ring with a bar through it.
fn sigil_dist(which: u32, p: [f32; 2]) -> f32 {
    match which % SIGILS {
        // A ring with a bar across it.
        0 => arc_dist(p, 0.62, 0.0, TAU).min(seg_dist(p, [-0.62, 0.0], [0.62, 0.0])),
        // A stem with two branches, one either side.
        1 => seg_dist(p, [0.0, -0.78], [0.0, 0.78])
            .min(seg_dist(p, [0.0, 0.16], [0.54, 0.62]))
            .min(seg_dist(p, [0.0, -0.16], [-0.54, -0.62])),
        // A triangle.
        2 => seg_dist(p, [0.0, 0.72], [0.64, -0.44])
            .min(seg_dist(p, [0.64, -0.44], [-0.64, -0.44]))
            .min(seg_dist(p, [-0.64, -0.44], [0.0, 0.72])),
        // A broken ring with a filled dot standing in its gap.
        3 => arc_dist(p, 0.60, 0.60, TAU - 0.60)
            .min(p[0].hypot(p[1] - 0.60) - 0.13),
        // A diamond on a stem.
        4 => seg_dist(p, [0.0, 0.70], [0.50, 0.10])
            .min(seg_dist(p, [0.50, 0.10], [0.0, -0.50]))
            .min(seg_dist(p, [0.0, -0.50], [-0.50, 0.10]))
            .min(seg_dist(p, [-0.50, 0.10], [0.0, 0.70]))
            .min(seg_dist(p, [0.0, -0.50], [0.0, -0.82])),
        // Three prongs off one stem.
        _ => seg_dist(p, [0.0, -0.78], [0.0, 0.78])
            .min(seg_dist(p, [-0.52, 0.30], [-0.52, 0.78]))
            .min(seg_dist(p, [0.52, 0.30], [0.52, 0.78]))
            .min(seg_dist(p, [-0.52, 0.30], [0.52, 0.30])),
    }
}

/// How wide a stroke is, as a fraction of a sigil's half-width.
const RUNE_STROKE: f32 = 0.095;

/// Nothing is drawn inside this radius of the middle, measured the way the
/// vignettes are. The medallion and the hearth own the centre of the table.
const RUNE_CLEAR: f32 = 0.13;

/// Sigils across the slab's long axis.
const RUNE_COLUMNS: u32 = 16;

/// The runes cut into the slab, in the five colours of the pie.
///
/// **The channels are not what they usually are here**, and this is the one
/// function in the module that departs from [`Texture`]'s straight-RGBA
/// contract. `rgb` is the rune's colour already multiplied by its coverage,
/// so it is black wherever there is no rune; `a` is that rune's **phase**,
/// `0.0..1.0`. The shader animates the glow as `sin(t + a·τ)`, which is how
/// two runes side by side breathe out of step without a second texture, a
/// second draw, or per-rune uniforms — of which `WebGL2` has very few to
/// spare. Alpha is the only channel a sampler never gamma-decodes, which is
/// why the phase rides there and not in a colour channel.
///
/// The layout is a jittered grid that thins towards the middle: the centre of
/// the table is the medallion's and the edges are the part no seat ever plays
/// on, so that is where ornament can be bright without ever sitting under a
/// card.
#[must_use]
pub fn runes(width: u32, height: u32) -> Texture {
    let mut texture = Texture::blank(width, height);
    let (w, h) = (width as f32, height as f32);
    // Square cells, counted along the long axis, so a sigil is round on a
    // slab that is not.
    let cell = w / RUNE_COLUMNS as f32;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a row count derived from two positive sizes"
    )]
    let rows = (h / cell).ceil() as u32;

    // Coverage already written at each pixel, so two sigils whose jitter
    // brought them together keep the nearer one's stroke instead of the
    // later one's.
    let mut covered = vec![0.0f32; (width as usize) * (height as usize)];

    for row in 0..rows {
        for column in 0..RUNE_COLUMNS {
            let (cx, cy) = (column, row);
            // Where the cell's sigil sits, jittered off the lattice so the
            // grid never reads as a grid.
            let jx = hash(cx as i32, cy as i32, 0x1a2b).mul_add(0.44, -0.22);
            let jy = hash(cx as i32, cy as i32, 0x77c3).mul_add(0.44, -0.22);
            let px = (column as f32 + 0.5 + jx) * cell;
            let py = (row as f32 + 0.5 + jy) * cell;
            let (u, v) = (px / w, py / h);

            // Thinner towards the middle, and nothing at all in the medallion's
            // patch.
            let radius = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt();
            if radius < RUNE_CLEAR {
                continue;
            }
            // `keep` is uniform, so the bound *is* the chance of a rune
            // landing in this cell. It wants to be small: runes are what a
            // slab has a few of, and at anything like a half the stone reads
            // as patterned paper rather than as stone. Denser towards the
            // edges, where no seat ever plays.
            let keep = hash(cx as i32, cy as i32, 0x5e09);
            if keep > (radius * 0.72).clamp(0.05, 0.30) {
                continue;
            }

            let which = (hash(cx as i32, cy as i32, 0x9d41) * 97.0) as u32;
            let colour = PIE[(hash(cx as i32, cy as i32, 0x3b70) * 4.999) as usize % 5];
            let phase = hash(cx as i32, cy as i32, 0xc417);
            let turn = hash(cx as i32, cy as i32, 0x0f52) * TAU;
            let scale = hash(cx as i32, cy as i32, 0x6ba8).mul_add(0.26, 0.30) * cell;
            let (sin, cos) = turn.sin_cos();

            // Only the pixels the sigil can reach. `1.15` is the sigil's own
            // reach (0.82 at the furthest stroke) plus its stroke and a
            // pixel of anti-aliasing.
            let reach = scale * 1.15;
            let (lo_x, hi_x) = span(px - reach, px + reach, width);
            let (lo_y, hi_y) = span(py - reach, py + reach, height);
            for y in lo_y..hi_y {
                for x in lo_x..hi_x {
                    let dx = (x as f32 + 0.5 - px) / scale;
                    let dy = (y as f32 + 0.5 - py) / scale;
                    // Into the sigil's own frame.
                    let local = [cos.mul_add(dx, sin * dy), (-sin).mul_add(dx, cos * dy)];
                    let d = sigil_dist(which, local);
                    // One pixel of anti-aliasing, in the sigil's units.
                    let aa = 1.0 / scale;
                    let m = 1.0 - smooth(((d - RUNE_STROKE) / aa).clamp(0.0, 1.0));
                    if m <= 0.0 {
                        continue;
                    }
                    let at = (y as usize) * (width as usize) + x as usize;
                    if m <= covered[at] {
                        continue;
                    }
                    covered[at] = m;
                    texture.put(
                        x,
                        y,
                        [colour[0] * m, colour[1] * m, colour[2] * m, phase],
                    );
                }
            }
        }
    }
    texture
}

/// A pixel range clamped to `0..limit`, as a half-open span.
fn span(from: f32, to: f32, limit: u32) -> (u32, u32) {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to the texture before the cast"
    )]
    let lo = from.floor().clamp(0.0, limit as f32) as u32;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to the texture before the cast"
    )]
    let hi = to.ceil().clamp(0.0, limit as f32) as u32;
    (lo, hi)
}

// ------------------------------------------------------------- the world under

/// The land the table floats over: forest seen from high above, in sunlight.
///
/// Opaque, and drawn on a plane well below the slab so it parallaxes when the
/// camera moves — which is the whole reason it is a separate surface and not
/// painted into a backdrop.
///
/// The hard part of this texture is not making it look like a forest, it is
/// making it *stay out of the way*. It is the brightest thing on screen and
/// it surrounds every card, so two rules hold it back: everything is lifted
/// towards a pale warm haze, the way distance actually behaves, and the whole
/// image is low-frequency. A crisp, saturated landscape would win every
/// staring contest with a card, and the card has to win.
#[must_use]
pub fn vista(width: u32, height: u32) -> Texture {
    /// The forest floor, in shadow between the crowns.
    const UNDER: [f32; 3] = [0.10, 0.17, 0.12];
    /// A canopy in shade.
    const CANOPY: [f32; 3] = [0.22, 0.36, 0.21];
    /// A crown with the sun on it.
    const SUNLIT: [f32; 3] = [0.53, 0.63, 0.33];
    /// Open ground: dry grass in a clearing.
    const MEADOW: [f32; 3] = [0.62, 0.60, 0.36];
    /// Water.
    const WATER: [f32; 3] = [0.34, 0.47, 0.56];
    /// The haze everything fades into with distance.
    const HAZE: [f32; 3] = [0.78, 0.82, 0.86];

    let mut texture = Texture::blank(width, height);
    let (w, h) = (width as f32, height as f32);
    let aspect = h / w;
    for y in 0..height {
        for x in 0..width {
            let (u, v) = (x as f32 / w, y as f32 / h);
            let (su, sv) = (u, v * aspect);

            // Where the land is open and where it is wooded.
            let cover = fbm(su * 3.4, sv * 3.4, 0x2c8f, 4);
            let mut colour = mix(MEADOW, CANOPY, ((cover - 0.34) * 3.2).clamp(0.0, 1.0));

            // Individual crowns, and the shadow between them. Two fields at
            // different scales, because a forest from above is big masses
            // with smaller ones inside them, not one grain size.
            let crowns = fbm(su * 26.0, sv * 26.0, 0x84b1, 3);
            let inner = fbm(su * 64.0, sv * 64.0, 0x1d47, 2);
            let relief = crowns.mul_add(0.72, inner * 0.28);
            colour = mix(colour, UNDER, ((0.46 - relief) * 2.6).clamp(0.0, 1.0) * 0.85);

            // The sun comes from the near-left, so crowns facing it catch it.
            // A directional term over a height field, which is the cheapest
            // honest imitation of a light there is — and it is *painted*, so
            // it lights nothing else in the scene.
            let dx = fbm(su.mul_add(26.0, 0.6), sv * 26.0, 0x84b1, 3) - crowns;
            let dy = fbm(su * 26.0, sv.mul_add(26.0, 0.6), 0x84b1, 3) - crowns;
            let facing = (-(dx + dy) * 7.0).clamp(0.0, 1.0);
            colour = mix(colour, SUNLIT, facing * 0.75);

            // A river, from the same ridged trick the granite's veins use.
            let field = fbm(su * 1.7, sv.mul_add(2.4, su * 1.1), 0x6f30, 4);
            let ridge = 1.0 - (2.0f32.mul_add(field, -1.0)).abs();
            colour = mix(colour, WATER, ridge.powf(22.0));

            // Broad daylight across the whole thing, brightest towards the
            // sun.
            let day = 0.22f32.mul_add(-(u + v), 1.06);
            for channel in &mut colour {
                *channel *= day;
            }

            // Aerial perspective. Strong: this is a long way down, and the
            // haze is what keeps a forest from out-shouting a card.
            colour = mix(colour, HAZE, 0.26);

            texture.put(x, y, [colour[0], colour[1], colour[2], 1.0]);
        }
    }
    texture
}

/// The cloud deck between the table and the land below it.
///
/// Transparent where there is no cloud, so the vista shows through the gaps
/// and the two planes read as two heights rather than one picture. Seen from
/// above, which is why the tops are bright and the shading is a thin cool
/// edge rather than the dark underside a cloud shows from a lawn.
///
/// The sun itself is here rather than in the sky, because there is no sky:
/// the camera looks down. It is a warm patch of glare in one corner with the
/// cloud around it burnt out — which is what a sun looks like when you are
/// above the weather and cannot see the disc.
#[must_use]
pub fn clouds(width: u32, height: u32) -> Texture {
    /// A cloud top in full sun.
    const TOP: [f32; 3] = [0.98, 0.97, 0.95];
    /// The cool edge where a cloud thins out.
    const EDGE: [f32; 3] = [0.72, 0.78, 0.87];
    /// The glare where the sun is.
    const GLARE: [f32; 3] = [1.0, 0.96, 0.84];
    /// Where the sun sits, in texture coordinates.
    const SUN: [f32; 2] = [0.22, 0.24];

    let mut texture = Texture::blank(width, height);
    let (w, h) = (width as f32, height as f32);
    let aspect = h / w;
    for y in 0..height {
        for x in 0..width {
            let (u, v) = (x as f32 / w, y as f32 / h);
            let (su, sv) = (u, v * aspect);

            // Masses first, then the billows inside them: the same two-scale
            // rule the forest follows, because a cloud field has the same
            // shape of structure.
            let mass = fbm(su * 2.3, sv * 2.3, 0x4a19, 4);
            let billow = fbm(su * 7.0, sv * 7.0, 0xb2e6, 3);
            let density = mass.mul_add(0.74, billow * 0.26);

            // A soft threshold, so a cloud has an edge and the gaps are
            // genuinely open. `0.52` leaves rather more sky than cloud.
            let alpha = ((density - 0.44) * 3.0).clamp(0.0, 1.0);
            if alpha <= 0.0 {
                texture.put(x, y, [0.0, 0.0, 0.0, 0.0]);
                continue;
            }

            // Thin cloud is the cool colour, thick cloud the sunlit top.
            let mut colour = mix(EDGE, TOP, alpha.powf(0.7));

            // The glare, which reaches beyond the cloud it sits on.
            let sun = (u - SUN[0]).hypot((v - SUN[1]) * aspect);
            let bloom = (-(sun * sun) / 0.085).exp();
            colour = mix(colour, GLARE, bloom.clamp(0.0, 1.0));

            texture.put(
                x,
                y,
                [
                    colour[0],
                    colour[1],
                    colour[2],
                    // The glare burns its own cloud opaque, so the sun does
                    // not show the forest through it.
                    (alpha + bloom * 0.85).clamp(0.0, 1.0),
                ],
            );
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
