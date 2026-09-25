//! The shells round a permanent (#298): what its protection looks like on
//! the table, now that the frame that carried a mark for it is gone.
//!
//! The first is indestructible: a rim of darksteel standing round the
//! card, from a hair above its face down to the felt, as if the card were set
//! in a frame. It is the first thing this client draws that stands *higher*
//! than the print it surrounds, so it brings with it the two rules every
//! later shell obeys.
//!
//! **Nothing of a shell is drawn over its own print.** The vertex stage hands
//! the fragment the camera in the shell's own space. The fragment follows the
//! ray from the camera through its point to the card's face plane, and where
//! that ray meets the print its alpha is exactly zero ([`clear_over_print`],
//! which `shell.wgsl` mirrors). It is the real camera and the real ray, so
//! the rule holds at every yaw, lean, screen corner, hover and tap.
//!
//! **Nothing of a shell lies on another card's print.** The mask cannot see a
//! neighbour, so this rule is geometry. [`rim_stands`] measures, from where
//! every card is this frame and where the camera is, how far the rim would
//! throw itself past its card's edge onto each other card, against the gap
//! to that card. A rim that does not fit lies down on the felt instead, as a
//! steel ring from [`RING_INNER`] to [`RING_OUTER`] out, under every card
//! ([`RING_RUNG`]).
//!
//! The rim can stand in a lane at all because it falls to the felt. A card
//! beside it at the same height hides everything of it below its own face, so
//! what can reach that card is only what stands above it: the rim's top edge,
//! [`RIM_RISE`] over the face, thrown [`RIM_RISE`] × the ray's tangent
//! past the edge. That is about 0.013 of a card width. A rim level to its
//! own foot, as first planned, reached 0.055, and two untapped cards in
//! neighbouring rows of a ring are 0.0185 apart.

use baylee_client_core::airborne;
use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH};
use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::table::{CARD_CORNER, CARD_LIFT, CARD_THICKNESS, FLOOR_RUNG, STRIP_RUNG};

/// How far past the card's edge the rim's foot stands, in card widths.
pub const RIM_MARGIN: f32 = 0.04;
/// How far above the card's face the rim's top edge stands.
pub const RIM_RISE: f32 = 0.010;
/// How far below the card's face the rim's foot hangs: to the felt, for a
/// card lying on it.
pub const RIM_DROP: f32 = CARD_THICKNESS + CARD_LIFT;
/// How far past the print the mask takes to open, outward only: exactly zero
/// on the print, whole this far off it.
pub const MASK_FEATHER: f32 = 0.012;
/// Where the ring a rim lies down to starts and ends, past the card's edge:
/// outside the offer's light on the felt ([`crate::floormat::REACH`]), so
/// the two are never one band of mixed colour.
pub const RING_INNER: f32 = 0.10;
/// See [`RING_INNER`].
pub const RING_OUTER: f32 = 0.16;
/// The height the ring lies at: over the offer's light and the contact
/// shadows, under every card.
pub const RING_RUNG: f32 = CARD_LIFT * 0.8;
/// Where the rim sits in the transparent pass: under the strip and the count
/// badge, which hangs over it at the card's top-left corner.
pub const RIM_RUNG: f32 = STRIP_RUNG - 0.001;
/// How much higher a ring's rim must be able to stand than it is before it
/// stands up again. A flier bobs [`airborne::SWAY`] either way, and without
/// this a flier at the edge of fitting would swap rim and ring on every bob:
/// with it, a ring stands up only if its rim would fit at the top of the bob.
pub const STAND_AGAIN: f32 = 2.0 * airborne::SWAY;

const _: () = assert!(RING_INNER >= crate::floormat::REACH && RING_OUTER > RING_INNER);
// Over the offer's light by a rung of the ladder, and under every card.
const _: () = assert!(RING_RUNG >= FLOOR_RUNG + 0.0005 && RING_RUNG < CARD_LIFT);
const _: () = assert!(RIM_RISE > 0.0 && RIM_MARGIN > 0.0);

/// Which shell a material draws, as `shell.wgsl` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ShellKind {
    /// Indestructible's steel rim, standing round the card.
    Rim = 1,
    /// The same steel as a ring on the felt, where the rim has no room.
    Ring = 2,
}

/// What the shell's shader reads.
///
/// Sixteen bytes, for the reason [`FloorParams`](crate::floormat::FloorParams)
/// is.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct ShellParams {
    /// A [`ShellKind`].
    pub kind: u32,
    /// The clock: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    pub motion: f32,
    /// The block's last eight bytes.
    pub pad: UVec2,
}

/// One shell's material. The key is the kind and nothing else, so a table
/// has one material per kind of shell however many cards wear it.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ShellMaterial {
    /// The kind and the clock.
    #[uniform(0)]
    pub params: ShellParams,
}

impl ShellMaterial {
    /// The material for `kind` on the clock `motion`.
    #[must_use]
    pub fn new(kind: ShellKind, motion: f32) -> Self {
        Self {
            params: ShellParams {
                kind: kind as u32,
                motion,
                pad: UVec2::ZERO,
            },
        }
    }

    fn kind(&self) -> ShellKind {
        if self.params.kind == ShellKind::Ring as u32 {
            ShellKind::Ring
        } else {
            ShellKind::Rim
        }
    }
}

impl Material for ShellMaterial {
    /// Its own vertex stage, because the mask needs the camera in the
    /// shell's own space, which Bevy's does not hand on.
    fn vertex_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/shell.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/shell.wgsl".into()
    }

    /// Blended, so the mask's zero is nothing at all.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    /// See [`RIM_RUNG`] and [`RING_RUNG`].
    fn depth_bias(&self) -> f32 {
        crate::table::sort_bias(match self.kind() {
            ShellKind::Rim => RIM_RUNG,
            ShellKind::Ring => RING_RUNG,
        })
    }
}

/// Registers the material and ships its shader in the binary, for the reason
/// [`CardMaterialPlugin`](crate::cardmat::CardMaterialPlugin) embeds its own.
pub struct ShellMaterialPlugin;

impl Plugin for ShellMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/shell.wgsl");
        app.add_plugins(MaterialPlugin::<ShellMaterial>::default());
    }
}

/// Signed distance from `p` to the card's rounded rectangle, in the card's
/// own space: negative on the print, positive off it.
#[must_use]
pub fn card_sdf(p: Vec2) -> f32 {
    let half = Vec2::new(CARD_WIDTH, CARD_HEIGHT) * 0.5;
    let q = p.abs() - (half - Vec2::splat(CARD_CORNER));
    q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - CARD_CORNER
}

/// How much of a shell's point `p` may be drawn, seen from `cam`, both in the
/// shell's own space, whose `z = 0` is the card's face: zero where the ray
/// from the camera through `p` meets the face on the print, one once it meets
/// it [`MASK_FEATHER`] off it. `shell.wgsl`'s `clear_over_print`.
///
/// A point above the face is in front of the print where its ray lands; a
/// point below it is behind the card there, which hides it anyway. So the
/// one test holds for both.
#[must_use]
pub fn clear_over_print(cam: Vec3, p: Vec3) -> f32 {
    let t = cam.z / (cam.z - p.z).max(1e-4);
    let hit = cam.truncate() + (p.truncate() - cam.truncate()) * t;
    smoothstep(0.0, MASK_FEATHER, card_sdf(hit))
}

fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How many segments each rounded corner of a band is drawn with. More than
/// the card's own four, because a band's outer edge sweeps a larger radius.
const BAND_SEGMENTS: usize = 6;

/// One vertex of a band round the card: where it is in the card's own space
/// (`x` across, `y` up the card, `z` off the face), the band's normal there
/// and the card's own UV there, running on past the card's edges.
#[derive(Clone, Copy, Debug)]
pub struct BandVertex {
    /// Position.
    pub at: Vec3,
    /// Normal: the band's slope, facing up and out.
    pub normal: Vec3,
    /// The card's UV at this point: `(0, 0)` its top-left corner.
    pub uv: Vec2,
}

/// The vertices of a band round the card, the rounded rectangle's outline
/// pushed out: from `inner` = `(distance past the edge, height off the
/// face)` to `outer`, in pairs, inner first, going round the card
/// anticlockwise seen from above.
#[must_use]
pub fn band(inner: (f32, f32), outer: (f32, f32)) -> Vec<BandVertex> {
    let (hw, hh) = (CARD_WIDTH / 2.0, CARD_HEIGHT / 2.0);
    let r = CARD_CORNER;
    let corners = [
        (Vec2::new(hw - r, hh - r), 0.0_f32),
        (Vec2::new(-hw + r, hh - r), 90.0),
        (Vec2::new(-hw + r, -hh + r), 180.0),
        (Vec2::new(hw - r, -hh + r), 270.0),
    ];
    let (d0, z0) = inner;
    let (d1, z1) = outer;
    let mut out = Vec::new();
    for (centre, start) in corners {
        for i in 0..=BAND_SEGMENTS {
            #[expect(clippy::cast_precision_loss)] // a handful of segments
            let a = (start + 90.0 * i as f32 / BAND_SEGMENTS as f32).to_radians();
            let dir = Vec2::new(a.cos(), a.sin());
            // The slope's normal: up and out, perpendicular to the run from
            // the inner edge to the outer one.
            let normal = (dir.extend(0.0) * (z0 - z1) + Vec3::Z * (d1 - d0)).normalize_or(Vec3::Z);
            for (d, z) in [(d0, z0), (d1, z1)] {
                let p = centre + dir * (r + d);
                out.push(BandVertex {
                    at: p.extend(z),
                    normal,
                    uv: Vec2::new(p.x / CARD_WIDTH + 0.5, 0.5 - p.y / CARD_HEIGHT),
                });
            }
        }
    }
    out
}

/// A [`band`] as a mesh, with its triangles facing up and out.
///
/// # Panics
///
/// Never: a band has a few dozen vertices.
#[must_use]
pub fn band_mesh(inner: (f32, f32), outer: (f32, f32)) -> Mesh {
    let vertices = band(inner, outer);
    let pairs = u32::try_from(vertices.len() / 2).expect("a few dozen pairs");
    let mut indices = Vec::new();
    for pair in 0..pairs {
        let next = (pair + 1) % pairs;
        // Inner then outer of this pair and the next: two triangles each
        // anticlockwise seen from above, so they face up and out.
        let (inner, outer) = (2 * pair, 2 * pair + 1);
        let (inner_next, outer_next) = (2 * next, 2 * next + 1);
        indices.extend_from_slice(&[inner, outer, outer_next, inner, outer_next, inner_next]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vertices.iter().map(|v| v.at.to_array()).collect::<Vec<_>>(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vertices
            .iter()
            .map(|v| v.normal.to_array())
            .collect::<Vec<_>>(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vertices.iter().map(|v| v.uv.to_array()).collect::<Vec<_>>(),
    )
    .with_inserted_indices(Indices::U32(indices))
}

/// The rim's two edges: its top at the card's edge, [`RIM_RISE`] over the
/// face, and its foot [`RIM_MARGIN`] out and [`RIM_DROP`] down.
pub const RIM_EDGES: [(f32, f32); 2] = [(0.0, RIM_RISE), (RIM_MARGIN, -RIM_DROP)];
/// The ring's two edges, flat.
pub const RING_EDGES: [(f32, f32); 2] = [(RING_INNER, 0.0), (RING_OUTER, 0.0)];

/// A card's face this frame, as the other cards and the camera see it: its
/// corners laid flat on the table (`x`, `z`), the lowest and the highest any
/// of them stands, and how much the card has grown.
#[derive(Clone, Copy, Debug)]
pub struct Footprint {
    /// The face's corners, flat, in order round the card.
    pub corners: [Vec2; 4],
    /// The lowest corner's height.
    pub low: f32,
    /// The highest corner's height.
    pub high: f32,
    /// The card's scale: a hover grows it.
    pub scale: f32,
}

impl Footprint {
    /// The face of a card standing at `transform`.
    #[must_use]
    pub fn of(transform: &Transform) -> Self {
        let (hw, hh) = (CARD_WIDTH / 2.0, CARD_HEIGHT / 2.0);
        let corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
            .map(|(x, y)| transform.transform_point(Vec3::new(x, y, CARD_THICKNESS)));
        Self {
            corners: corners.map(|c| Vec2::new(c.x, c.z)),
            low: corners.iter().map(|c| c.y).fold(f32::INFINITY, f32::min),
            high: corners
                .iter()
                .map(|c| c.y)
                .fold(f32::NEG_INFINITY, f32::max),
            scale: transform.scale.max_element(),
        }
    }

    fn centre(&self) -> Vec2 {
        self.corners.iter().copied().sum::<Vec2>() / 4.0
    }

    fn radius(&self) -> f32 {
        let c = self.centre();
        self.corners
            .iter()
            .map(|p| p.distance(c))
            .fold(0.0, f32::max)
    }
}

/// How far past its card's edge a rim throws itself onto a face `over`
/// below the rim's top edge, seen along rays of tangent `tangent`, on a
/// card grown by `scale`.
///
/// The rim's profile runs from its top edge at the card's edge down to its
/// foot, and a face hides every part of it that is below that face. What is
/// above lands `height × tangent` further out, so of the part above the face
/// the furthest landing is at one of its ends: the top edge, or the point
/// where the profile goes under the face (or the foot, if it never does).
#[must_use]
pub fn rim_reach(over: f32, scale: f32, tangent: f32) -> f32 {
    if over <= 0.0 {
        return 0.0;
    }
    let (margin, fall) = (RIM_MARGIN * scale, (RIM_RISE + RIM_DROP) * scale);
    let under = if over >= fall {
        margin
    } else {
        margin * over / fall
    };
    let foot = under + (over - fall * under / margin).max(0.0) * tangent;
    (over * tangent).max(foot)
}

/// Whether a rim can stand round the card at `me` without landing on the
/// face of any card in `others`, seen from a camera at `eye`, if it stood
/// `headroom` higher than it does.
///
/// A face higher than the rim's top edge hides the rim wherever the two meet
/// on screen, so only the faces below it count. For each of those the reach
/// is measured along the steepest ray the camera sends to any corner of this
/// card, and in every direction. That is the conservative half: a raised
/// point only ever lands *away* from the camera.
#[must_use]
pub fn rim_stands(
    me: &Footprint,
    others: impl IntoIterator<Item = Footprint>,
    eye: Vec3,
    headroom: f32,
) -> bool {
    let top = me.high + RIM_RISE * me.scale + headroom;
    let tangent = me
        .corners
        .iter()
        .map(|c| c.distance(Vec2::new(eye.x, eye.z)) / (eye.y - top).max(1e-3))
        .fold(0.0, f32::max);
    let (centre, radius) = (me.centre(), me.radius());
    others.into_iter().all(|other| {
        let reach = rim_reach(top - other.low, me.scale, tangent);
        if reach <= 0.0 {
            return true;
        }
        // Too far off to be reached at all: no need to measure closer.
        if centre.distance(other.centre()) - radius - other.radius() > reach {
            return true;
        }
        gap(&me.corners, &other.corners) >= reach
    })
}

/// The distance between two convex quadrilaterals, and a negative number
/// when they overlap.
#[must_use]
pub fn gap(a: &[Vec2; 4], b: &[Vec2; 4]) -> f32 {
    if overlap(a, b) {
        return -1.0;
    }
    let mut best = f32::INFINITY;
    for (points, edges) in [(a, b), (b, a)] {
        for p in points {
            for i in 0..4 {
                best = best.min(to_segment(*p, edges[i], edges[(i + 1) % 4]));
            }
        }
    }
    best
}

/// Whether two convex quadrilaterals overlap: no edge of either separates
/// them.
fn overlap(a: &[Vec2; 4], b: &[Vec2; 4]) -> bool {
    for poly in [a, b] {
        for i in 0..4 {
            let edge = poly[(i + 1) % 4] - poly[i];
            let axis = Vec2::new(-edge.y, edge.x);
            let span = |p: &[Vec2; 4]| {
                p.iter()
                    .map(|v| v.dot(axis))
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), x| {
                        (lo.min(x), hi.max(x))
                    })
            };
            let ((a0, a1), (b0, b1)) = (span(a), span(b));
            if a1 < b0 || b1 < a0 {
                return false;
            }
        }
    }
    true
}

fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardmat::tests::{check_wgsl, wgsl_const};

    const SHADER: &str = include_str!("shaders/shell.wgsl");

    /// The shell's shader, parsed and validated against stubs of what Bevy
    /// hands a custom vertex stage: the mesh's matrices, the view and the
    /// clock.
    #[test]
    fn the_shell_shader_compiles() {
        let prelude = "\
struct View { world_position: vec3<f32> };
@group(0) @binding(0) var<uniform> view: View;
struct Globals { time: f32 };
@group(0) @binding(11) var<uniform> globals: Globals;
fn get_world_from_local(i: u32) -> mat4x4<f32> { return mat4x4<f32>(); }
fn get_local_from_world(i: u32) -> mat4x4<f32> { return mat4x4<f32>(); }
fn mesh_position_local_to_world(m: mat4x4<f32>, p: vec4<f32>) -> vec4<f32> { return m * p; }
fn mesh_normal_local_to_world(n: vec3<f32>, i: u32) -> vec3<f32> { return n; }
fn position_world_to_clip(p: vec3<f32>) -> vec4<f32> { return vec4<f32>(p, 1.0); }
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
";
        check_wgsl(
            SHADER,
            &format!("{prelude}{}", include_str!("shaders/card_common.wgsl")),
        );
    }

    /// One whole `std140` block.
    #[test]
    fn the_shell_uniform_is_one_block() {
        assert_eq!(ShellParams::min_size().get(), 16);
    }

    /// The shader declares the uniform in the order Rust lays it out.
    #[test]
    fn the_shader_reads_the_uniform_rust_writes() {
        let open = SHADER.find("struct ShellParams {").expect("the uniform");
        let body = &SHADER[open..open + SHADER[open..].find('}').expect("it closes")];
        let fields: Vec<&str> = body
            .lines()
            .map(str::trim)
            .filter(|l| !l.starts_with("//") && l.contains(':'))
            .collect();
        assert_eq!(
            fields,
            ["kind: u32,", "motion: f32,", "pad: vec2<u32>,"],
            "shell.wgsl lays the uniform out differently from `ShellParams`"
        );
    }

    /// Every number the shader and this file share is the same number in
    /// both: a mask cut to a different card than the rim is built round is a
    /// mask with a gap in it.
    #[test]
    fn the_shader_measures_the_card_this_file_does() {
        for (name, ours) in [
            ("CARD_HALF_W", CARD_WIDTH / 2.0),
            ("CARD_HALF_H", CARD_HEIGHT / 2.0),
            ("CARD_ROUND", CARD_CORNER),
            ("MASK_FEATHER", MASK_FEATHER),
            ("RING_INNER", RING_INNER),
            ("RING_OUTER", RING_OUTER),
            ("RIM_RISE", RIM_RISE),
            ("RIM_DROP", RIM_DROP),
        ] {
            let theirs = wgsl_const(SHADER, name);
            assert!(
                (theirs - ours).abs() < 1e-4,
                "{name}: {ours} here, {theirs} in shell.wgsl"
            );
        }
        for (name, kind) in [
            ("SHELL_RIM", ShellKind::Rim),
            ("SHELL_RING", ShellKind::Ring),
        ] {
            #[expect(clippy::cast_precision_loss)] // a small enum
            let ours = kind as u32 as f32;
            assert!(
                (wgsl_const(SHADER, name) - ours).abs() < f32::EPSILON,
                "{name}"
            );
        }
    }

    /// Every colour the fragment stage returns has the mask as the last
    /// factor of its alpha: a `return` that forgot it would be a shell drawn
    /// over its own print from whichever side the camera happened to be on.
    #[test]
    fn every_colour_a_shell_returns_carries_the_mask() {
        let open = SHADER.find("fn fragment(").expect("the fragment stage");
        let body = &SHADER[open..];
        let returns: Vec<&str> = body
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("return"))
            .collect();
        assert!(!returns.is_empty(), "the fragment stage returns nothing");
        for line in returns {
            let alpha = line
                .strip_suffix(");")
                .and_then(|l| l.rsplit_once(','))
                .map(|(_, alpha)| alpha.trim());
            assert!(
                alpha.is_some_and(|a| a == "clear" || a.ends_with("* clear")),
                "`{line}` returns an alpha the mask has not touched"
            );
        }
    }

    /// The mask, by hand: the camera straight over the card's middle sees
    /// every point above the print on the print, and a point beside it off
    /// it; from low and to one side, a point standing over the near edge is
    /// thrown onto the print and one over the far edge off it.
    #[test]
    #[allow(clippy::float_cmp)] // exactly zero is the claim, not nearly
    fn the_mask_is_exactly_zero_over_the_print() {
        let above = Vec3::new(0.0, 0.0, 20.0);
        for p in [
            Vec3::new(0.0, 0.0, 0.3),
            // In the corner, inside its round.
            Vec3::new(0.47, 0.68, 0.01),
            Vec3::new(-0.3, 0.2, 0.0),
        ] {
            assert_eq!(clear_over_print(above, p), 0.0, "{p} over the print");
        }
        assert_eq!(clear_over_print(above, Vec3::new(0.52, 0.0, 0.01)), 1.0);
        assert!(clear_over_print(above, Vec3::new(0.505, 0.0, 0.01)) < 1.0);

        // A camera off to the right at a 45° ray: a rim point over the right
        // edge (near the camera) lands inward, one over the left edge
        // (away from it) lands outward.
        let side = Vec3::new(10.5, 0.0, 10.0);
        let near = Vec3::new(0.505, 0.0, 0.01);
        let far = Vec3::new(-0.505, 0.0, 0.01);
        assert_eq!(clear_over_print(side, near), 0.0, "the near edge's top");
        assert!(clear_over_print(side, far) > 0.9, "the far edge's top");
        // Below the face, the same geometry the other way round: the card
        // stands between the camera and the far foot.
        let far_foot = Vec3::new(-0.52, 0.0, -0.05);
        assert_eq!(clear_over_print(side, far_foot), 0.0, "behind the card");
    }

    /// The reach, by hand. Level with the rim's top edge a face is hidden
    /// behind nothing and takes nothing; a face the rim's top stands 0.014
    /// over takes 0.014 × the tangent from the top edge; a flier's rim,
    /// wholly above its neighbour, reaches its foot's margin and more.
    #[test]
    #[allow(clippy::float_cmp)] // nothing reached is a literal zero
    fn a_rim_reaches_only_as_far_as_what_stands_above_the_face() {
        assert_eq!(rim_reach(0.0, 1.0, 1.2), 0.0);
        assert_eq!(rim_reach(-0.1, 1.0, 1.2), 0.0);
        let at_rest = rim_reach(0.014, 1.0, 1.2);
        assert!((at_rest - 0.014 * 1.2).abs() < 1e-6, "{at_rest}");
        // From straight overhead the top edge throws nothing, and the
        // furthest point above the face is where the slope goes under it.
        let overhead = rim_reach(0.014, 1.0, 0.0);
        assert!((overhead - RIM_MARGIN * 0.014 / (RIM_RISE + RIM_DROP)).abs() < 1e-6);
        let flier = rim_reach(0.2, 1.0, 1.0);
        assert!((flier - 0.2).abs() < 1e-6, "{flier}");
        let over_foot = rim_reach(0.2, 1.0, 0.1);
        assert!(
            (over_foot - (RIM_MARGIN + (0.2 - RIM_RISE - RIM_DROP) * 0.1)).abs() < 1e-6,
            "{over_foot}"
        );
    }

    /// The gap between two cards, by hand: side by side, overlapping, and
    /// corner to corner.
    #[test]
    fn the_gap_between_two_cards_is_measured_to_the_nearest_point() {
        let square = |x: f32, y: f32| {
            [
                Vec2::new(x, y),
                Vec2::new(x + 1.0, y),
                Vec2::new(x + 1.0, y + 1.0),
                Vec2::new(x, y + 1.0),
            ]
        };
        assert!((gap(&square(0.0, 0.0), &square(1.25, 0.0)) - 0.25).abs() < 1e-6);
        assert!(gap(&square(0.0, 0.0), &square(0.5, 0.5)) < 0.0);
        assert!((gap(&square(0.0, 0.0), &square(1.3, 1.4)) - 0.5).abs() < 1e-6);
    }
}
