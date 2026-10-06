//! Where the table cloth's veins bend: the jitter of every Voronoi cell,
//! computed once when a table is cut instead of on every pixel of every frame.
//!
//! `felt.wgsl` draws its mineral veins as the seams of two cellular fields
//! (`vein_distance`, a trunk scale and a capillary scale). Each pixel visits
//! nine cells per field, and each cell's point used to be two value-noise
//! lookups (`vnoise(seed * 7.13)`, `vnoise(seed * 9.71 + 31.7)`): 36 noise
//! lookups, 144 hashes, per pixel, a third of the whole table's GPU time
//! (3.2 of 10 ms a frame at 3840×2160, `docs/perf-client.md`). The point is
//! a function of the integer cell alone, so it is computed here, for every
//! cell the table can reach, into a small two-channel texture the shader
//! reads with one `textureLoad` per cell.
//!
//! The numbers are the shader's own: [`hash_cell`] is `noise.wgsl`'s
//! integer hash bit for bit and [`vnoise`] is `felt.wgsl`'s value noise, so
//! the veins keep exactly the shape they had. Eight bits per channel move a
//! point by at most 1/510 of a cell, under a third of a pixel at a duel.

/// `noise.wgsl`'s `hash_cell`: the same integer operations in the same
/// order, so the same bits on every GPU and here.
#[must_use]
pub fn hash_cell(x: i32, y: i32) -> f32 {
    #[allow(clippy::cast_sign_loss)]
    let (cx, cy) = (x as u32, y as u32);
    let mut h = cx.wrapping_mul(0x27d4_eb2d) ^ cy.wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x2974_5c65);
    h ^= h >> 15;
    #[allow(clippy::cast_precision_loss)]
    let top = (h >> 8) as f32;
    top * (1.0 / 16_777_216.0)
}

/// `felt.wgsl`'s `vnoise`: value noise with the cubic smoothstep.
#[must_use]
pub fn vnoise(px: f32, py: f32) -> f32 {
    let (ix, iy) = (px.floor(), py.floor());
    let (fx, fy) = (px - ix, py - iy);
    let (ux, uy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    #[allow(clippy::cast_possible_truncation)]
    let (cx, cy) = (ix as i32, iy as i32);
    let a = hash_cell(cx, cy);
    let b = hash_cell(cx.wrapping_add(1), cy);
    let c = hash_cell(cx, cy.wrapping_add(1));
    let d = hash_cell(cx.wrapping_add(1), cy.wrapping_add(1));
    let mix = |p: f32, q: f32, t: f32| p * (1.0 - t) + q * t;
    mix(mix(a, b, ux), mix(c, d, ux), uy)
}

/// A cell's point inside it, as `felt.wgsl`'s `vein_distance` placed it.
#[must_use]
pub fn cell_point(x: i32, y: i32) -> [f32; 2] {
    #[allow(clippy::cast_precision_loss)]
    let (sx, sy) = (x as f32, y as f32);
    [
        vnoise(sx * 7.13, sy * 7.13),
        vnoise(sx * 9.71 + 31.7, sy * 9.71 + 31.7),
    ]
}

/// The two cellular fields `felt.wgsl` draws: scale and offset of the warped
/// table point, as `glass_at` passes them to `vein_distance`.
pub const FIELDS: [(f32, f32); 2] = [(0.28, 0.0), (0.73, 8.3)];

/// How far `glass_at`'s domain warp can push a point (`fbm(…) * 3.4`, fbm in
/// 0..1).
const WARP_REACH: f32 = 3.4;

/// The largest field one region holds. A table at eight seats needs under
/// 180 cells; anything past this reads the region's edge instead of failing.
pub const MAX_REGION: u32 = 512;

/// Every cell point the table can reach, laid out for upload: an `Rg8Unorm`
/// texture with one region per field side by side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VeinTable {
    /// Texels across.
    pub width: u32,
    /// Texels down.
    pub height: u32,
    /// Two bytes per texel, row-major: the point's x then y, `0..=255`.
    pub texels: Vec<u8>,
    /// Per field, what to add to a cell to get its texel: the shader's
    /// `textureLoad(veins, cell + offset)`.
    pub offsets: [[i32; 2]; 2],
}

impl VeinTable {
    /// The table for a slab `span` across, under the cloth's `pattern`
    /// (`FeltParams::pattern`: offset xy, rotation in 256ths of a turn,
    /// scale in 256ths of 0.30 over 0.85).
    #[must_use]
    pub fn for_table(span: [f32; 2], pattern: [f32; 4]) -> Self {
        // Every point on the slab lies within its half-diagonal of the
        // centre, whatever the pattern's rotation; the scale only grows it.
        let reach = (span[0] * span[0] + span[1] * span[1]).sqrt() * 0.5;
        let scale = 0.85 + pattern[3] * (0.30 / 256.0);
        let radius = reach * scale;
        let mut regions = [(0i32, 0i32, 0u32, 0u32); 2];
        for (region, (k, b)) in regions.iter_mut().zip(FIELDS) {
            // A point's warp lies in `domain + [0, WARP_REACH]`, and a pixel
            // visits the cells one either side of its own.
            let lo = |c: f32| ((c - radius) * k + b).floor() - 1.0;
            let hi = |c: f32| ((c + radius + WARP_REACH) * k + b).floor() + 1.0;
            #[allow(clippy::cast_possible_truncation)]
            let (x0, y0) = (lo(pattern[0]) as i32, lo(pattern[1]) as i32);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (w, h) = (
                ((hi(pattern[0]) as i32 - x0 + 1) as u32).min(MAX_REGION),
                ((hi(pattern[1]) as i32 - y0 + 1) as u32).min(MAX_REGION),
            );
            *region = (x0, y0, w, h);
        }
        let width = regions[0].2 + regions[1].2;
        let height = regions[0].3.max(regions[1].3);
        let mut texels = vec![0u8; (width * height * 2) as usize];
        let mut left = 0u32;
        let mut offsets = [[0i32; 2]; 2];
        for (i, &(x0, y0, w, h)) in regions.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            {
                offsets[i] = [left as i32 - x0, -y0];
            }
            for row in 0..h {
                for col in 0..w {
                    #[allow(clippy::cast_possible_wrap)]
                    let point = cell_point(x0 + col as i32, y0 + row as i32);
                    let at = ((row * width + left + col) * 2) as usize;
                    texels[at] = quantise(point[0]);
                    texels[at + 1] = quantise(point[1]);
                }
            }
            left += w;
        }
        Self {
            width,
            height,
            texels,
            offsets,
        }
    }

    /// The point the shader reads for `cell` of field `field`, clamped to
    /// the texture's edge as `textureLoad` is fed.
    #[must_use]
    pub fn point(&self, field: usize, cell: [i32; 2]) -> [f32; 2] {
        #[allow(clippy::cast_possible_wrap)]
        let x = (cell[0] + self.offsets[field][0]).clamp(0, self.width as i32 - 1);
        #[allow(clippy::cast_possible_wrap)]
        let y = (cell[1] + self.offsets[field][1]).clamp(0, self.height as i32 - 1);
        #[allow(clippy::cast_sign_loss)]
        let at = ((y as u32 * self.width + x as u32) * 2) as usize;
        [
            f32::from(self.texels[at]) / 255.0,
            f32::from(self.texels[at + 1]) / 255.0,
        ]
    }
}

fn quantise(value: f32) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    byte
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hash is `noise.wgsl`'s: these are values its integer arithmetic
    /// gives, worked out by hand from the constants, and a lattice point of
    /// the general `hash2` (place zero) is the same function.
    #[test]
    fn the_hash_is_the_shaders_integer_hash() {
        // h = 0 at the origin: every step keeps zero.
        assert!(hash_cell(0, 0).abs() < f32::EPSILON);
        let one = {
            let mut h: u32 = 0x27d4_eb2d;
            h ^= h >> 15;
            h = h.wrapping_mul(0x2c1b_3c6d);
            h ^= h >> 12;
            h = h.wrapping_mul(0x2974_5c65);
            h ^= h >> 15;
            f64::from(h >> 8) / 16_777_216.0
        };
        assert!((f64::from(hash_cell(1, 0)) - one).abs() < 1e-9);
        // Negative cells hash their two's complement, as `bitcast` does.
        assert_ne!(hash_cell(-1, 0), hash_cell(1, 0));
        for x in -50..50 {
            let v = hash_cell(x, 3 * x);
            assert!((0.0..1.0).contains(&v));
        }
    }

    /// Value noise meets every lattice corner exactly and stays inside it.
    #[test]
    fn value_noise_is_its_corners_at_the_lattice() {
        for (x, y) in [(0, 0), (3, -7), (-12, 40)] {
            #[allow(clippy::cast_precision_loss)]
            let at = vnoise(x as f32, y as f32);
            assert!((at - hash_cell(x, y)).abs() < 1e-6);
        }
        let mid = vnoise(0.5, 0.5);
        let corners = [
            hash_cell(0, 0),
            hash_cell(1, 0),
            hash_cell(0, 1),
            hash_cell(1, 1),
        ];
        let (lo, hi) = corners
            .iter()
            .fold((1.0f32, 0.0f32), |(l, h), c| (l.min(*c), h.max(*c)));
        assert!(mid >= lo && mid <= hi);
    }

    /// Every cell a pixel of the slab can visit is in the table — for a duel
    /// and for eight seats, at every pattern corner — and reads back the
    /// point the shader used to compute, to within one 8-bit step.
    #[test]
    fn every_reachable_cell_is_in_the_table_and_reads_back_its_point() {
        for span in [[38.0, 24.0], [190.0, 76.0]] {
            for pattern in [[0.0, 0.0, 0.0, 0.0], [255.9, 255.9, 128.0, 255.9]] {
                let table = VeinTable::for_table(span, pattern);
                assert!(table.width <= 2 * MAX_REGION && table.height <= MAX_REGION);
                let scale = 0.85 + pattern[3] * (0.30 / 256.0);
                let reach = (span[0].hypot(span[1])) * 0.5 * scale;
                for (field, (k, b)) in FIELDS.into_iter().enumerate() {
                    // The extreme warped points, and the neighbours of their cells.
                    for (dx, dy) in [(-reach, -reach), (reach + 3.4, reach + 3.4)] {
                        #[allow(clippy::cast_possible_truncation)]
                        let cx = ((pattern[0] + dx) * k + b).floor() as i32;
                        #[allow(clippy::cast_possible_truncation)]
                        let cy = ((pattern[1] + dy) * k + b).floor() as i32;
                        for (ox, oy) in [(-1, -1), (1, 1), (0, 0)] {
                            let cell = [cx + ox, cy + oy];
                            #[allow(clippy::cast_possible_wrap)]
                            let x = cell[0] + table.offsets[field][0];
                            let y = cell[1] + table.offsets[field][1];
                            #[allow(clippy::cast_possible_wrap)]
                            let inside = x >= 0
                                && y >= 0
                                && x < table.width as i32
                                && y < table.height as i32;
                            assert!(inside, "{span:?} {pattern:?} field {field} cell {cell:?}");
                            let read = table.point(field, cell);
                            let exact = cell_point(cell[0], cell[1]);
                            for c in 0..2 {
                                assert!((read[c] - exact[c]).abs() <= 0.5 / 255.0 + 1e-6);
                            }
                        }
                    }
                }
            }
        }
    }
}
