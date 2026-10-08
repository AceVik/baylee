//! The cloth's slow fields, baked once per cut: where the glass is warped,
//! how hot it runs and how much silt lies in it.
//!
//! `felt.wgsl` bends its veins through a domain warp — two four-octave value
//! noises per pixel — and reads a third for the silt and one more lookup for
//! the heat that decides which river a vein carries: thirteen value noises,
//! fifty-two hashes, on every pixel of the slab on every frame, and none of
//! it moves. Taken out of the shader (constants in their place, a duel at six
//! seats, Metal System Trace) the table's GPU time fell by about a quarter.
//! So they are computed here, on a grid over the slab, into a texture the
//! shader samples with one filtered read; the veins themselves stay sharp,
//! because they are still cut per pixel from the cell points
//! (`crate::feltveins`) through this warp.
//!
//! The numbers are the shader's own ([`crate::feltveins::vnoise`] is its
//! value noise bit for bit, and [`fbm`] and [`domain`] repeat its
//! arithmetic), so a texel holds exactly what the shader computed at its
//! centre; between centres the filter interpolates. All four fields are
//! smooth at this grid's spacing (the finest octave has about three cycles
//! a table unit), and `a_sample_between_texels_is_the_field_there` bounds
//! what the interpolation costs.

use crate::feltveins::vnoise;

/// Grid points per table unit, where the table is small enough to afford
/// them ([`MAX_TEXELS`] caps a large one).
pub const TEXELS_PER_UNIT: f32 = 16.0;

/// The most texels along either side: an eight-seat table spreads the same
/// grid thinner, and is seen from further away.
pub const MAX_TEXELS: u32 = 2048;

/// How far past the slab's edge the grid reaches, in table units: a tearing
/// table's cut face reads the field a little beyond its line.
pub const MARGIN: f32 = 2.0;

/// How far the warp can push a point (`felt.wgsl`: `fbm(…) * 3.4`).
pub const WARP_REACH: f32 = 3.4;

/// `felt.wgsl`'s `fbm`: four octaves of [`vnoise`], normalised to `0..1`.
#[must_use]
pub fn fbm(px: f32, py: f32) -> f32 {
    let (mut sum, mut amplitude, mut total) = (0.0, 0.5, 0.0);
    let (mut x, mut y) = (px, py);
    for _ in 0..4 {
        sum += amplitude * vnoise(x, y);
        total += amplitude;
        x *= 2.03;
        y *= 2.03;
        amplitude *= 0.5;
    }
    sum / total
}

/// `felt.wgsl`'s `veins_at` domain: the table point turned by the pattern's
/// angle, scaled by its scale and moved by its offset.
#[must_use]
pub fn domain(p: [f32; 2], pattern: [f32; 4]) -> [f32; 2] {
    let angle = pattern[2] * (std::f32::consts::TAU / 256.0);
    let (sin, cos) = angle.sin_cos();
    let scale = 0.85 + pattern[3] * (0.30 / 256.0);
    [
        (p[0] * cos + p[1] * sin) * scale + pattern[0],
        (-p[0] * sin + p[1] * cos) * scale + pattern[1],
    ]
}

/// The four slow fields at table point `p`: the warp's two noises (the
/// warp is `domain + WARP_REACH · xy`), the heat read at the warped point,
/// and the silt.
#[must_use]
pub fn fields(p: [f32; 2], pattern: [f32; 4]) -> [f32; 4] {
    let d = domain(p, pattern);
    let a = fbm(d[0] * 0.32, d[1] * 0.32);
    let b = fbm(d[0] * 0.32 + 19.4, d[1] * 0.32 + 19.4);
    let warp = [d[0] + a * WARP_REACH, d[1] + b * WARP_REACH];
    let heat = vnoise(warp[0] * 0.19 + 42.0, warp[1] * 0.19 + 42.0);
    let silt = fbm(p[0] * 0.34, p[1] * 0.34);
    [a, b, heat, silt]
}

/// The grid over one slab.
#[derive(Clone, Debug, PartialEq)]
pub struct WarpField {
    /// Texels across.
    pub width: u32,
    /// Texels down.
    pub height: u32,
    /// The table point at the grid's top-left corner (the edge of texel 0,
    /// not its centre): `x` left, `y` the far end (table `y` grows away).
    pub origin: [f32; 2],
    /// The table units the whole grid covers, `x` across and `y` down.
    pub size: [f32; 2],
    /// Four fields per texel ([`fields`]), row-major from the top-left.
    pub texels: Vec<[f32; 4]>,
}

impl WarpField {
    /// The grid for a slab `span` across, centred on the table's middle,
    /// under the cloth's `pattern`.
    #[must_use]
    pub fn for_table(span: [f32; 2], pattern: [f32; 4]) -> Self {
        Self::at_density(span, pattern, Self::density(span))
    }

    /// Grid points per table unit for a slab `span` across.
    #[must_use]
    pub fn density(span: [f32; 2]) -> f32 {
        let longest = span[0].max(span[1]) + 2.0 * MARGIN;
        #[allow(clippy::cast_precision_loss)]
        let capped = MAX_TEXELS as f32 / longest;
        TEXELS_PER_UNIT.min(capped)
    }

    /// The grid over a slab `span` across at `density` points a unit.
    #[must_use]
    pub fn at_density(span: [f32; 2], pattern: [f32; 4], density: f32) -> Self {
        let size = [span[0] + 2.0 * MARGIN, span[1] + 2.0 * MARGIN];
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let side = |units: f32| ((units * density).ceil() as u32).clamp(2, MAX_TEXELS);
        let (width, height) = (side(size[0]), side(size[1]));
        let origin = [-size[0] * 0.5, size[1] * 0.5];
        let mut texels = Vec::with_capacity((width * height) as usize);
        for row in 0..height {
            for col in 0..width {
                texels.push(fields(
                    Self::centre(origin, size, width, height, col, row),
                    pattern,
                ));
            }
        }
        Self {
            width,
            height,
            origin,
            size,
            texels,
        }
    }

    /// The table point at the centre of texel (`col`, `row`).
    #[allow(clippy::cast_precision_loss)]
    fn centre(
        origin: [f32; 2],
        size: [f32; 2],
        width: u32,
        height: u32,
        col: u32,
        row: u32,
    ) -> [f32; 2] {
        [
            origin[0] + (col as f32 + 0.5) / width as f32 * size[0],
            origin[1] - (row as f32 + 0.5) / height as f32 * size[1],
        ]
    }

    /// What a bilinear, edge-clamped read of the grid gives at table point
    /// `p` — the shader's `textureSampleLevel` on a linear sampler, here to
    /// be held against [`fields`].
    #[must_use]
    pub fn sample(&self, point: [f32; 2]) -> [f32; 4] {
        #[allow(clippy::cast_precision_loss)]
        let (across, down) = (self.width as f32, self.height as f32);
        let col = (point[0] - self.origin[0]) / self.size[0] * across - 0.5;
        let row = (self.origin[1] - point[1]) / self.size[1] * down - 0.5;
        let (col0, row0) = (col.floor(), row.floor());
        let (frac_col, frac_row) = (col - col0, row - row0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let texel = |x: f32, y: f32| {
            let col = (x as i32).clamp(0, self.width as i32 - 1);
            let row = (y as i32).clamp(0, self.height as i32 - 1);
            #[allow(clippy::cast_sign_loss)]
            self.texels[(row as u32 * self.width + col as u32) as usize]
        };
        let (top_left, top_right, bottom_left, bottom_right) = (
            texel(col0, row0),
            texel(col0 + 1.0, row0),
            texel(col0, row0 + 1.0),
            texel(col0 + 1.0, row0 + 1.0),
        );
        std::array::from_fn(|i| {
            let top = top_left[i] + (top_right[i] - top_left[i]) * frac_col;
            let bottom = bottom_left[i] + (bottom_right[i] - bottom_left[i]) * frac_col;
            top + (bottom - top) * frac_row
        })
    }

    /// The uniform the shader reads the grid through: the origin in `xy`
    /// and the reciprocal of the size in `zw` (so `uv = (p - origin) · zw`,
    /// `y` flipped); all zero means no grid and the shader computes.
    #[must_use]
    pub fn rect(&self) -> [f32; 4] {
        [
            self.origin[0],
            self.origin[1],
            1.0 / self.size[0],
            1.0 / self.size[1],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATTERNS: [[f32; 4]; 3] = [
        [0.0, 0.0, 0.0, 0.0],
        [131.5, 17.25, 64.0, 128.0],
        [255.9, 255.9, 200.0, 255.9],
    ];

    /// A texel holds the fields at its own centre, exactly; the grid covers
    /// the slab and its margin; the uniform maps the grid's corners to 0
    /// and 1.
    #[test]
    fn a_texel_is_the_field_at_its_centre() {
        let span = [12.0, 8.0];
        for pattern in PATTERNS {
            let field = WarpField::for_table(span, pattern);
            assert_eq!(field.texels.len(), (field.width * field.height) as usize);
            assert!(field.size[0] >= span[0] + 2.0 * MARGIN - 1e-4);
            for (col, row) in [(0, 0), (field.width - 1, field.height - 1), (37, 21)] {
                let at = WarpField::centre(
                    field.origin,
                    field.size,
                    field.width,
                    field.height,
                    col,
                    row,
                );
                let stored = field.texels[(row * field.width + col) as usize];
                assert_eq!(stored, fields(at, pattern), "texel {col},{row}");
                // And a read at the centre gives the texel back.
                let read = field.sample(at);
                for i in 0..4 {
                    assert!((read[i] - stored[i]).abs() < 1e-4);
                }
            }
            let rect = field.rect();
            let far_left = [field.origin[0], field.origin[1]];
            let near_right = [
                field.origin[0] + field.size[0],
                field.origin[1] - field.size[1],
            ];
            for (p, uv) in [(far_left, [0.0, 0.0]), (near_right, [1.0, 1.0])] {
                let u = (p[0] - rect[0]) * rect[2];
                let v = (rect[1] - p[1]) * rect[3];
                assert!((u - uv[0]).abs() < 1e-5 && (v - uv[1]).abs() < 1e-5);
            }
        }
    }

    /// Between texel centres the filtered read stays near the field: the
    /// warp within about a hundredth of a table unit (a vein is about a
    /// tenth wide; measured 0.006 at the full grid, 0.014 at an eight-seat
    /// table's), the heat and the silt within half a hundredth (0.003,
    /// 0.006).
    #[test]
    fn a_sample_between_texels_is_the_field_there() {
        // The eight-seat slab's grid is as sparse as its own size makes it,
        // measured on a duel-sized piece so the test stays quick.
        let sparse = WarpField::density([190.0, 76.0]);
        for (density, warp_bound, rest_bound) in
            [(TEXELS_PER_UNIT, 0.012, 0.005), (sparse, 0.025, 0.01)]
        {
            let (span, pattern) = ([30.0, 20.0], PATTERNS[1]);
            let field = WarpField::at_density(span, pattern, density);
            let (mut worst_warp, mut worst_rest) = (0.0f32, 0.0f32);
            // A fixed spread of points, off the grid's centres.
            for i in 0..4000u32 {
                #[allow(clippy::cast_precision_loss)]
                let (fx, fy) = (
                    (i as f32 * 0.618_034).fract(),
                    (i as f32 * 0.754_877_7).fract(),
                );
                let p = [(fx - 0.5) * span[0], (fy - 0.5) * span[1]];
                let exact = fields(p, pattern);
                let read = field.sample(p);
                worst_warp = worst_warp
                    .max((exact[0] - read[0]).abs() * WARP_REACH)
                    .max((exact[1] - read[1]).abs() * WARP_REACH);
                worst_rest = worst_rest
                    .max((exact[2] - read[2]).abs())
                    .max((exact[3] - read[3]).abs());
            }
            assert!(
                worst_warp < warp_bound,
                "{density}/unit: warp off by {worst_warp}"
            );
            assert!(
                worst_rest < rest_bound,
                "{density}/unit: heat or silt off by {worst_rest}"
            );
            // And the bound is not vacuous: the fields do vary.
            assert!(
                field
                    .texels
                    .iter()
                    .any(|t| (t[0] - field.texels[0][0]).abs() > 0.1)
            );
        }
    }

    /// The grid never exceeds its cap, and a small table keeps the full
    /// density.
    #[test]
    fn the_grid_is_capped_and_a_small_table_keeps_its_density() {
        let density = WarpField::density([400.0, 120.0]);
        #[allow(clippy::cast_precision_loss)]
        let longest = (400.0 + 2.0 * MARGIN) * density;
        assert!(longest <= MAX_TEXELS as f32 + 1e-3);
        let small = WarpField::for_table([20.0, 10.0], PATTERNS[0]);
        #[allow(clippy::cast_precision_loss)]
        let density = small.width as f32 / small.size[0];
        assert!((density - TEXELS_PER_UNIT).abs() < 0.5);
    }
}
