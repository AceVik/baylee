//! The tearing table's shapes, cut on the CPU along the tear's jagged line
//! (`transition::tear_line`): each piece's top, its share of the rim's wall,
//! its cut face with the drips under it, my piece's weld seam, the void
//! under the table and the floating dial.
//!
//! Real geometry rather than a discard in the shader: an edge that is a
//! triangle's edge is smoothed by multisampling, a discarded one is drawn
//! in stair steps (the coordinator's of 07.10.2026). Both pieces are cut
//! from one list of samples along the line, so their edges, the cut faces
//! and the seam meet exactly — no sliver between them when they weld.
//!
//! Every shape is in the slab mesh's own frame (`x`, `y` table, `z` up) and
//! carries uvs `felt.wgsl` reads as what it is: `u` in 0 to 1 a point of the
//! top (the slab's own mapping), 2 to 3 the cut face, 4 to 5 the rim's wall,
//! 6 to 7 the seam; `v` the top's other coordinate or how far down.

use baylee_client_core::layout::transition::DIAL_THICKNESS;
use bevy::asset::RenderAssetUsages;
use bevy::math::Vec2;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use std::f32::consts::FRAC_PI_2;

/// How far below the slab the spilled veins may hang, in slab thicknesses
/// (`felt.wgsl`'s `DRIP_DEPTH`).
pub const DRIP_DEPTH: f32 = 1.6;

/// The step the pieces are cut at along the line, table units: well under
/// the tear's smallest teeth.
pub const CUT_STEP: f32 = 0.06;

/// The samples on each of the rim's rounded corners.
const ARC_STEPS: usize = 16;

/// How wide the weld seam is either side of the line, table units: a thin
/// line, not a band.
pub const SEAM_HALF: f32 = 0.05;

/// How high the seam lies over the top: over the felt, under every card.
const SEAM_RISE: f32 = 0.004;

/// The radius of the floating dial: everything the firewheel and the clock
/// face draw (`felt.wgsl`'s `FLAME_REACH`).
pub const DIAL_R: f32 = 1.6;

/// A mesh under construction.
#[derive(Default)]
struct Build {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl Build {
    fn vertex(&mut self, at: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // tens of thousands at most
        let index = self.positions.len() as u32;
        self.positions.push(at);
        self.normals.push(normal);
        self.uvs.push(uv);
        index
    }

    /// A quad wound `a → b → c → d`, counter-clockwise as seen from the side
    /// it faces (bevy's front face).
    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.indices.extend_from_slice(&[a, b, c, a, c, d]);
    }

    fn mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(self.indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
    }
}

/// The slab's outline: a rectangle of `span` with corners of radius
/// `corner` (`tabletop::table_corner`), as `rounded_slab_mesh` cuts it.
#[derive(Clone, Copy, Debug)]
pub struct Outline {
    /// The slab's size.
    pub span: Vec2,
    /// Its corners' radius.
    pub corner: f32,
}

impl Outline {
    fn half(self) -> Vec2 {
        self.span * 0.5
    }

    fn radius(self) -> f32 {
        let half = self.half();
        self.corner.clamp(0.0, half.x.min(half.y))
    }

    /// How far the rim stands from the middle line at `x`: the top edge's
    /// height (the bottom edge is its mirror).
    #[must_use]
    pub fn rim(self, x: f32) -> f32 {
        let (half, r) = (self.half(), self.radius());
        let dx = (x.abs() - (half.x - r)).max(0.0);
        half.y - r + (r * r - dx * dx).max(0.0).sqrt()
    }

    /// The outward direction of the top edge at `x` (the bottom edge's is
    /// its mirror).
    fn rim_normal(self, x: f32) -> Vec2 {
        let (half, r) = (self.half(), self.radius());
        let dx = (x.abs() - (half.x - r)).max(0.0);
        if r <= 0.0 || dx <= 0.0 {
            return Vec2::Y;
        }
        Vec2::new(dx.copysign(x), (r * r - dx * dx).max(0.0).sqrt()).normalize_or(Vec2::Y)
    }

    /// The uv of a point of the top: the slab's own mapping.
    fn uv(self, p: Vec2) -> [f32; 2] {
        [p.x / self.span.x + 0.5, 0.5 - p.y / self.span.y]
    }

    /// Where the pieces are cut, left to right: fine along the straight
    /// edges, by angle round the corners, the outermost samples on the
    /// slab's sides exactly.
    #[must_use]
    pub fn columns(self) -> Vec<f32> {
        let (half, r) = (self.half(), self.radius());
        let straight = half.x - r;
        let mut xs = Vec::new();
        #[allow(clippy::cast_precision_loss)]
        let arc = |i: usize| (FRAC_PI_2 * i as f32 / ARC_STEPS as f32).sin();
        for i in (1..=ARC_STEPS).rev() {
            xs.push(-straight - r * arc(i));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = ((2.0 * straight / CUT_STEP).ceil() as usize).max(1);
        for i in 0..=steps {
            #[allow(clippy::cast_precision_loss)]
            xs.push(-straight + 2.0 * straight * i as f32 / steps as f32);
        }
        for i in 1..=ARC_STEPS {
            xs.push(straight + r * arc(i));
        }
        xs
    }

    /// The tear's line at `x`, kept inside the rim.
    fn line(self, x: f32, seed: f32) -> f32 {
        let rim = self.rim(x) - 0.05;
        baylee_client_core::layout::transition::tear_line(x, seed).clamp(-rim, rim)
    }
}

/// One piece of the torn table: `near` mine (below the line), otherwise the
/// far one (above it). Its top, its share of the rim's wall, its cut face
/// down the line facing the gap with the drips under it, and on mine the
/// weld seam over the line.
#[must_use]
#[allow(clippy::too_many_lines)] // one shape, its five parts in order
pub fn piece_mesh(outline: Outline, thickness: f32, seed: f32, near: bool) -> Mesh {
    let xs = outline.columns();
    let line: Vec<f32> = xs.iter().map(|&x| outline.line(x, seed)).collect();
    let rim: Vec<f32> = xs.iter().map(|&x| outline.rim(x)).collect();
    // The piece's far edge from the line: the rim on its own side.
    let edge: Vec<f32> = rim.iter().map(|r| if near { -r } else { *r }).collect();
    let mut out = Build::default();
    let up = [0.0, 0.0, 1.0];

    // The top: one strip per column, from the rim to the line.
    let mut rows = Vec::with_capacity(xs.len());
    for (i, &x) in xs.iter().enumerate() {
        let (lo, hi) = if near {
            (edge[i], line[i])
        } else {
            (line[i], edge[i])
        };
        let a = out.vertex([x, lo, 0.0], up, outline.uv(Vec2::new(x, lo)));
        let c = out.vertex([x, hi, 0.0], up, outline.uv(Vec2::new(x, hi)));
        rows.push((a, c));
    }
    for w in rows.windows(2) {
        let ((lo0, hi0), (lo1, hi1)) = (w[0], w[1]);
        out.quad(lo0, lo1, hi1, hi0);
    }

    // The rim's wall along the piece's own edge, outward, and the two short
    // walls at the slab's sides from the rim to the line.
    let wall_uv = |x: f32, v: f32| [4.0 + x / outline.span.x + 0.5, v];
    let mut walls = Vec::with_capacity(xs.len());
    for (i, &x) in xs.iter().enumerate() {
        let n = outline.rim_normal(x);
        let n = if near {
            [n.x, -n.y, 0.0]
        } else {
            [n.x, n.y, 0.0]
        };
        let top = out.vertex([x, edge[i], 0.0], n, wall_uv(x, 0.0));
        let bottom = out.vertex([x, edge[i], -thickness], n, wall_uv(x, 1.0));
        walls.push((top, bottom));
    }
    for w in walls.windows(2) {
        let ((t0, b0), (t1, b1)) = (w[0], w[1]);
        if near {
            // Seen from below the table's near edge, `x` runs to the right.
            out.quad(t0, b0, b1, t1);
        } else {
            out.quad(t1, b1, b0, t0);
        }
    }
    for (i, side) in [(0, -1.0_f32), (xs.len() - 1, 1.0)] {
        let x = xs[i];
        let (lo, hi) = if near {
            (edge[i], line[i])
        } else {
            (line[i], edge[i])
        };
        let n = [side, 0.0, 0.0];
        let u = if side > 0.0 { 1.0 } else { 0.0 };
        let lo_top = out.vertex([x, lo, 0.0], n, [4.0 + u, 0.0]);
        let lo_bottom = out.vertex([x, lo, -thickness], n, [4.0 + u, 1.0]);
        let hi_top = out.vertex([x, hi, 0.0], n, [4.0 + u, 0.0]);
        let hi_bottom = out.vertex([x, hi, -thickness], n, [4.0 + u, 1.0]);
        if side > 0.0 {
            out.quad(lo_top, lo_bottom, hi_bottom, hi_top);
        } else {
            out.quad(hi_top, hi_bottom, lo_bottom, lo_top);
        }
    }

    // The cut face down the line, facing the gap (`+y` on mine), and the
    // drips hanging under it, seen from both sides.
    let facing: f32 = if near { 1.0 } else { -1.0 };
    let cut_uv = |x: f32, v: f32| [2.0 + x / outline.span.x + 0.5, v];
    let depths = [
        (0.0, 0.0),
        (-thickness, 1.0),
        (-thickness * (1.0 + DRIP_DEPTH), 1.0 + DRIP_DEPTH),
    ];
    let mut cut = Vec::with_capacity(xs.len());
    for (i, &x) in xs.iter().enumerate() {
        let column =
            depths.map(|(z, v)| out.vertex([x, line[i], z], [0.0, facing, 0.0], cut_uv(x, v)));
        cut.push(column);
    }
    for w in cut.windows(2) {
        let (left, right) = (w[0], w[1]);
        for row in 0..2 {
            let (a, bb, c, d) = (left[row], right[row], left[row + 1], right[row + 1]);
            // Counter-clockwise as seen from `facing`: the cross-section is
            // seen from the gap and from nowhere else; the drips hang free
            // and are seen from both sides.
            if facing > 0.0 {
                out.quad(a, bb, d, c);
            } else {
                out.quad(a, c, d, bb);
            }
            if row == 1 {
                if facing > 0.0 {
                    out.quad(a, c, d, bb);
                } else {
                    out.quad(a, bb, d, c);
                }
            }
        }
    }

    // The weld seam, on mine only: a thin strip over the line, offset along
    // the line's own normal so it is as thin on a steep tooth as on a flat.
    if near {
        let point = |i: usize| Vec2::new(xs[i], line[i]);
        let mut strip = Vec::with_capacity(xs.len());
        for i in 0..xs.len() {
            let before = point(i.saturating_sub(1));
            let after = point((i + 1).min(xs.len() - 1));
            let along = (after - before).normalize_or(Vec2::X);
            let normal = Vec2::new(-along.y, along.x);
            let at = point(i);
            let along_u = 6.0 + xs[i] / outline.span.x + 0.5;
            let lo = at - normal * SEAM_HALF;
            let hi = at + normal * SEAM_HALF;
            let below = out.vertex([lo.x, lo.y, SEAM_RISE], up, [along_u, 0.0]);
            let above = out.vertex([hi.x, hi.y, SEAM_RISE], up, [along_u, 1.0]);
            strip.push((below, above));
        }
        for w in strip.windows(2) {
            let ((lo0, hi0), (lo1, hi1)) = (w[0], w[1]);
            out.quad(lo0, lo1, hi1, hi0);
        }
    }
    out.mesh()
}

/// The void under the table (the coordinator's of 07.10.2026: *the gap
/// must read as a hole*): the far part of the slab's outline from a little
/// in front of the tear on, as a floor `felt.wgsl` draws dark. My piece
/// covers the rest from every place the camera stands.
#[must_use]
pub fn void_mesh(outline: Outline, from: f32) -> Mesh {
    let xs = outline.columns();
    let mut out = Build::default();
    let up = [0.0, 0.0, 1.0];
    let mut rows = Vec::with_capacity(xs.len());
    for &x in &xs {
        let rim = outline.rim(x);
        let lo = from.max(-rim);
        let a = out.vertex([x, lo, 0.0], up, outline.uv(Vec2::new(x, lo)));
        let c = out.vertex([x, rim, 0.0], up, outline.uv(Vec2::new(x, rim)));
        rows.push((a, c));
    }
    for w in rows.windows(2) {
        let ((lo0, hi0), (lo1, hi1)) = (w[0], w[1]);
        out.quad(lo0, lo1, hi1, hi0);
    }
    out.mesh()
}

/// The dial lifted whole off the tearing table (the coordinator's of
/// 07.10.2026: *the dial is cut by the tear — keep it whole*): a disc of
/// [`DIAL_R`] at the dial's `scale` (`baylee_client_core::dial::scale_for`)
/// with a thin wall, its top mapped like the slab's so `felt.wgsl` draws the
/// firewheel and the clock face on it exactly as on the table.
#[must_use]
pub fn dial_mesh(outline: Outline, scale: f32) -> Mesh {
    const SEGMENTS: usize = 72;
    let mut out = Build::default();
    let up = [0.0, 0.0, 1.0];
    let centre = out.vertex([0.0, 0.0, 0.0], up, outline.uv(Vec2::ZERO));
    let ring: Vec<Vec2> = (0..SEGMENTS)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let a = std::f32::consts::TAU * i as f32 / SEGMENTS as f32;
            Vec2::new(a.cos(), a.sin())
        })
        .collect();
    #[allow(clippy::cast_possible_truncation)]
    let first = out.positions.len() as u32;
    for d in &ring {
        let p = *d * (DIAL_R * scale);
        out.vertex([p.x, p.y, 0.0], up, outline.uv(p));
    }
    #[allow(clippy::cast_possible_truncation)]
    for i in 0..SEGMENTS as u32 {
        let next = (i + 1) % SEGMENTS as u32;
        out.indices
            .extend_from_slice(&[centre, first + i, first + next]);
    }
    let mut wall = Vec::with_capacity(SEGMENTS);
    for d in &ring {
        let p = *d * (DIAL_R * scale);
        let n = [d.x, d.y, 0.0];
        // The slab's own mapping: `felt.wgsl` shades the disc's edge as the
        // slab's apron, at the point of the table it stands over.
        let uv = outline.uv(p);
        let top = out.vertex([p.x, p.y, 0.0], n, uv);
        let bottom = out.vertex([p.x, p.y, -DIAL_THICKNESS], n, uv);
        wall.push((top, bottom));
    }
    for i in 0..SEGMENTS {
        let ((t0, b0), (t1, b1)) = (wall[i], wall[(i + 1) % SEGMENTS]);
        out.quad(t0, b0, b1, t1);
    }
    out.mesh()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    fn triangles(mesh: &Mesh) -> Vec<[Vec3; 3]> {
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        let Some(Indices::U32(index)) = mesh.indices() else {
            panic!("indices");
        };
        index
            .chunks(3)
            .map(|t| [t[0], t[1], t[2]].map(|i| Vec3::from(p[i as usize])))
            .collect()
    }

    fn uvs(mesh: &Mesh) -> Vec<[f32; 2]> {
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("uvs");
        };
        uv.clone()
    }

    const OUTLINE: Outline = Outline {
        span: Vec2::new(36.0, 18.0),
        corner: 1.2,
    };

    /// Every face of a piece is wound toward the side it is seen from: the
    /// top up, the rim's wall out of the table, the cut face's cross-section
    /// toward the gap (`+y` on mine, `-y` on the far piece). A face wound
    /// the other way is culled from the only side anybody looks at it from
    /// — which is what the first cut face was, measured live.
    #[test]
    fn every_face_of_a_piece_faces_where_it_is_seen_from() {
        for near in [true, false] {
            let mesh = piece_mesh(OUTLINE, 0.9, 3.7, near);
            let facing = if near { 1.0 } else { -1.0 };
            let (mut top, mut cut, mut rim) = (0, 0, 0);
            for [a, b, c] in triangles(&mesh) {
                let n = (b - a).cross(c - a);
                if n.length() < 1e-9 {
                    continue;
                }
                let n = n.normalize();
                let mid = (a + b + c) / 3.0;
                let lowest = a.z.min(b.z).min(c.z);
                if a.z.abs() < 1e-6 && b.z.abs() < 1e-6 && c.z.abs() < 1e-6 {
                    assert!(n.z > 0.99, "near={near}: a top triangle faces down");
                    top += 1;
                } else if mid.z > 1e-3 {
                    assert!(n.z > 0.99, "the seam faces up");
                } else if (mid.y.abs() - OUTLINE.rim(mid.x)).abs() < 0.05
                    || (mid.x.abs() - OUTLINE.span.x * 0.5).abs() < 1e-3
                {
                    let out = Vec3::new(mid.x, mid.y, 0.0);
                    let outward = if (mid.x.abs() - OUTLINE.span.x * 0.5).abs() < 1e-3 {
                        Vec3::new(mid.x.signum(), 0.0, 0.0)
                    } else {
                        Vec3::new(0.0, out.y.signum(), 0.0)
                    };
                    assert!(n.dot(outward) > 0.0, "near={near}: a rim wall faces in");
                    rim += 1;
                } else if lowest >= -0.9 - 1e-4 {
                    assert!(
                        n.y * facing > 0.0,
                        "near={near}: the cross-section faces away from the gap at {mid}"
                    );
                    cut += 1;
                }
            }
            assert!(top > 500 && cut > 500 && rim > 500, "{top} {cut} {rim}");
        }
    }

    /// The two pieces are cut from one line: every point of the line on my
    /// piece's top is a point of the far piece's top, so when they meet
    /// there is no sliver between them — and the seam lies over exactly that
    /// line. A cut taken twice, each piece on its own samples, could leave
    /// one.
    #[test]
    fn the_two_pieces_meet_on_one_line() {
        let edge = |near: bool| -> Vec<(i32, i32)> {
            let mesh = piece_mesh(OUTLINE, 0.9, 5.1, near);
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("positions");
            };
            let uv = uvs(&mesh);
            #[allow(clippy::cast_possible_truncation)]
            let mut points: Vec<(i32, i32)> = p
                .iter()
                .zip(&uv)
                .filter(|(at, uv)| at[2] == 0.0 && (2.0..3.0).contains(&uv[0]))
                .map(|(at, _)| ((at[0] * 1e4) as i32, (at[1] * 1e4) as i32))
                .collect();
            points.sort_unstable();
            points
        };
        let (mine, far) = (edge(true), edge(false));
        assert!(mine.len() > 500);
        assert_eq!(mine, far, "the two cut edges are one line");
    }

    /// The pieces' tops cover the slab's top and nothing else: their areas
    /// add up to the rounded rectangle's, so no strip is missing and none is
    /// drawn twice.
    #[test]
    fn the_two_tops_are_the_whole_table() {
        let area = |mesh: &Mesh| -> f32 {
            triangles(mesh)
                .into_iter()
                .filter(|t| t.iter().all(|v| v.z == 0.0))
                .filter(|[a, b, c]| (b - a).cross(c - a).z > 0.0)
                .map(|[a, b, c]| (b - a).cross(c - a).z * 0.5)
                .sum()
        };
        let (mine, far) = (
            piece_mesh(OUTLINE, 0.9, 2.2, true),
            piece_mesh(OUTLINE, 0.9, 2.2, false),
        );
        // Only the top: the seam lies over it and is not part of it.
        let r = OUTLINE.radius();
        let whole = OUTLINE.span.x * OUTLINE.span.y - (4.0 - std::f32::consts::PI) * r * r;
        let covered = area(&mine) + area(&far);
        assert!(
            (covered - whole).abs() / whole < 2e-3,
            "{covered} of {whole}"
        );
    }
}
