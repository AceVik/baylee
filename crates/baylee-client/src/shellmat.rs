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
//!
//! The second is a dome of light for hexproof (blue) or shroud (violet,
//! which swallows hexproof): a pillow over the card, its plateau
//! [`DOME_HEIGHT`] over the face and its skirt on the felt [`DOME_MARGIN`]
//! past the card's edge. It is the same argument with a taller profile, so it
//! is the same guard ([`shell_stands`] over [`dome_profile`]), tried at each
//! of [`DOME_STEPS`] from the tallest down: a dome with a little air stands
//! lower, and one with none lies down as a ring in its colour. Where the
//! steel and the dome both lie, they share the ring's band inside out, as
//! they stand: steel [`RING_INNER`]..[`RING_SPLIT`], the dome
//! [`RING_SPLIT`]..[`RING_OUTER`] (the PM, 25.09). Which dome is a row
//! ([`Dome::row`]), so ward and protection (#302) are rows too.
//!
//! The third is defender's brick wall: a low rampart standing on the felt
//! past the card's top edge, towards the table's middle and the enemy,
//! slightly curved like a shield. It needs no guard, because it is never
//! taller than any card: at most [`WALL_HEIGHT`] (0.08) off the felt, under
//! the face of every card lying there ([`RIM_DROP`], 0.083), so wherever it
//! meets a print on screen that print is in front of it and hides it. It
//! stands on the felt whatever its card does: a hover or a flier's lift
//! would carry it over its neighbours' faces.

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
/// badge, which stands at the card's top-right corner.
pub const RIM_RUNG: f32 = STRIP_RUNG - 0.001;
/// How much higher a ring's rim must be able to stand than it is before it
/// stands up again. A flier bobs [`airborne::SWAY`] either way, and without
/// this a flier at the edge of fitting would swap rim and ring on every bob:
/// with it, a ring stands up only if its rim would fit at the top of the bob.
pub const STAND_AGAIN: f32 = 2.0 * airborne::SWAY;

/// Where the band a ring lies in is cut in two when the steel and a dome
/// both lie down: the steel inside, the dome outside.
pub const RING_SPLIT: f32 = 0.13;
/// How far past the card's edge a dome's skirt meets the felt: outside the
/// steel rim's foot, inside the band its ring lies in.
pub const DOME_MARGIN: f32 = 0.10;
/// How high a dome's plateau stands over the card's face at full height.
pub const DOME_HEIGHT: f32 = 0.20;
/// How far in from its skirt a dome's plateau begins: the run of the quarter
/// ellipse from the one to the other.
pub const DOME_INSET: f32 = 0.25;
/// How wide the brighter line along a dome's foot is.
pub const DOME_FOOT: f32 = 0.03;
/// The heights a dome may stand at, as shares of its row's full height,
/// tallest first. Below the last it lies down as a ring.
pub const DOME_STEPS: [f32; 4] = [1.0, 0.7, 0.5, 0.3];
/// How many rings a dome's profile is drawn and measured with, from its
/// plateau's edge down to its skirt.
pub const DOME_RINGS: usize = 8;
/// Where a dome sits in the transparent pass: over the rim it stands
/// outside of, under the strip and the count badge.
pub const DOME_RUNG: f32 = RIM_RUNG + 0.0005;
/// How high defender's wall stands off the felt: two courses of brick and
/// a row of merlons, and never as high as a card's face (the PM, 25.09).
pub const WALL_HEIGHT: f32 = 0.08;
/// One course of brick, and one brick along it.
pub const WALL_COURSE: f32 = 0.028;
/// See [`WALL_COURSE`].
pub const WALL_BRICK: f32 = 0.07;
/// How far past the card's top edge the wall's inner face stands at its
/// ends; its middle bulges [`WALL_BULGE`] further out. Far enough that its
/// own card hides little of it (the wall sweep in `table::shell_tests`
/// counts it: one point in a hundred, from every shot).
pub const WALL_NEAR: f32 = 0.12;
/// How far the wall's middle bows out past its ends, like a shield.
pub const WALL_BULGE: f32 = 0.04;
/// How thick the wall is.
pub const WALL_THICK: f32 = 0.025;
/// How far the wall runs past the card's width at each end.
pub const WALL_OVERHANG: f32 = 0.06;
/// How many merlons crown it.
pub const WALL_MERLONS: usize = 5;
/// Where the wall sits in the pass. It writes depth like a solid thing, so
/// it goes first: what is drawn over it after is sorted by that.
pub const WALL_RUNG: f32 = FLOOR_RUNG - 0.0005;

const _: () = assert!(RING_INNER >= crate::floormat::REACH && RING_OUTER > RING_INNER);
const _: () = assert!(RING_SPLIT > RING_INNER && RING_SPLIT < RING_OUTER);
// Over the offer's light by a rung of the ladder, and under every card.
const _: () = assert!(RING_RUNG >= FLOOR_RUNG + 0.0005 && RING_RUNG < CARD_LIFT);
const _: () = assert!(RIM_RISE > 0.0 && RIM_MARGIN > 0.0);
// A dome stands outside the rim and inside its own ring's band.
const _: () = assert!(DOME_MARGIN > RIM_MARGIN && DOME_MARGIN <= RING_INNER);
const _: () = assert!(DOME_RUNG > RIM_RUNG && DOME_RUNG < STRIP_RUNG);
// Under every card's face, which is the whole of the wall's legality.
const _: () = assert!(WALL_HEIGHT <= 0.08 && WALL_HEIGHT < RIM_DROP);
const _: () = assert!(2.0 * WALL_COURSE < WALL_HEIGHT);
// Outside the dome's skirt.
const _: () = assert!(WALL_NEAR > DOME_MARGIN);

/// Which shell a material draws, as `shell.wgsl` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ShellKind {
    /// Indestructible's steel rim, standing round the card.
    Rim = 1,
    /// The same steel as a ring on the felt, where the rim has no room.
    Ring = 2,
    /// A dome of light over the card: hexproof's or shroud's.
    Dome = 3,
    /// A dome lying down as a ring on the felt, in its colour.
    DomeRing = 4,
    /// Defender's brick wall.
    Wall = 5,
}

/// A protection that stands over its card as a dome of light (#298).
///
/// Only a permanent's strongest one is drawn: shroud swallows hexproof, as
/// the strip's own marks do. Ward and protection from a colour (#302) will
/// be rows here, not code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dome {
    /// Blue: only its controller's spells and abilities may target it.
    Hexproof,
    /// Violet: nothing may target it.
    Shroud,
}

/// One dome's shape and colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DomeRow {
    /// How far past the card's edge its skirt meets the felt.
    pub margin: f32,
    /// How high its plateau stands over the card's face, at full height.
    pub height: f32,
    /// Its colour, linear, in the lobby's blue-hour key.
    pub tint: [f32; 3],
}

impl Dome {
    /// Every dome, for building their meshes up front.
    pub const ALL: [Self; 2] = [Self::Hexproof, Self::Shroud];

    /// The dome's row.
    #[must_use]
    pub const fn row(self) -> DomeRow {
        let tint = match self {
            Self::Hexproof => [0.25, 0.55, 1.0],
            Self::Shroud => [0.55, 0.25, 0.95],
        };
        DomeRow {
            margin: DOME_MARGIN,
            height: DOME_HEIGHT,
            tint,
        }
    }

    /// The dome a permanent with these keywords wears, if any.
    #[must_use]
    pub const fn of(hexproof: bool, shroud: bool) -> Option<Self> {
        if shroud {
            Some(Self::Shroud)
        } else if hexproof {
            Some(Self::Hexproof)
        } else {
            None
        }
    }
}

/// Which part of the ring's band a ring lies in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Band {
    /// All of it: the only ring lying round its card.
    Whole,
    /// Its inner half, for the steel under a dome's ring.
    Inner,
    /// Its outer half, for a dome's ring round the steel's.
    Outer,
}

impl Band {
    /// Every band, for building their meshes up front.
    pub const ALL: [Self; 3] = [Self::Whole, Self::Inner, Self::Outer];

    /// Where the band starts and ends past the card's edge.
    #[must_use]
    pub const fn edges(self) -> [(f32, f32); 2] {
        match self {
            Self::Whole => [(RING_INNER, 0.0), (RING_OUTER, 0.0)],
            Self::Inner => [(RING_INNER, 0.0), (RING_SPLIT, 0.0)],
            Self::Outer => [(RING_SPLIT, 0.0), (RING_OUTER, 0.0)],
        }
    }
}

/// Everything a shell's material is told: one material per look, however
/// many cards wear it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShellLook {
    /// What is drawn.
    pub kind: ShellKind,
    /// Whose colour, for a dome and its ring.
    pub dome: Option<Dome>,
    /// Which part of the band, for a ring.
    pub band: Band,
}

impl ShellLook {
    /// A look with no dome, over the whole band.
    #[must_use]
    pub const fn steel(kind: ShellKind) -> Self {
        Self {
            kind,
            dome: None,
            band: Band::Whole,
        }
    }
}

/// What the shell's shader reads.
///
/// Two whole `std140` rows, for the reason
/// [`FloorParams`](crate::floormat::FloorParams) is one.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct ShellParams {
    /// A dome's colour, linear; the steel ignores it.
    pub tint: Vec4,
    /// A [`ShellKind`].
    pub kind: u32,
    /// The clock: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    pub motion: f32,
    /// Where a ring's band, or a dome's foot line, starts and ends past the
    /// card's edge.
    pub inner: f32,
    /// See [`Self::inner`].
    pub outer: f32,
}

/// One shell's material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(ShellKey)]
pub struct ShellMaterial {
    /// The look and the clock.
    #[uniform(0)]
    pub params: ShellParams,
}

/// What a shell's pipeline is specialised on: whether its back faces are
/// drawn, and whether it writes depth.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShellKey {
    /// A dome is light on a surface of glass: from the camera its far side is
    /// seen from within, and that is the half that stands past the card's
    /// far edge, where the mask lets it be seen.
    double_sided: bool,
    /// A wall is solid: its merlons stand in front of its own top, and only
    /// depth says which of its faces is nearer.
    solid: bool,
}

impl From<&ShellMaterial> for ShellKey {
    fn from(material: &ShellMaterial) -> Self {
        Self {
            double_sided: material.kind() == ShellKind::Dome,
            solid: material.kind() == ShellKind::Wall,
        }
    }
}

impl ShellMaterial {
    /// The material for `look` on the clock `motion`.
    #[must_use]
    pub fn new(look: ShellLook, motion: f32) -> Self {
        let tint = look.dome.map_or([0.0; 3], |dome| dome.row().tint);
        let (inner, outer) = if let (ShellKind::Dome, Some(dome)) = (look.kind, look.dome) {
            let margin = dome.row().margin;
            (margin - DOME_FOOT, margin)
        } else {
            let [(inner, _), (outer, _)] = look.band.edges();
            (inner, outer)
        };
        Self {
            params: ShellParams {
                tint: Vec3::from_array(tint).extend(1.0),
                kind: look.kind as u32,
                motion,
                inner,
                outer,
            },
        }
    }

    fn kind(&self) -> ShellKind {
        match self.params.kind {
            2 => ShellKind::Ring,
            3 => ShellKind::Dome,
            4 => ShellKind::DomeRing,
            5 => ShellKind::Wall,
            _ => ShellKind::Rim,
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

    /// Blended, so the mask's zero is nothing at all. Not `Add`, which a
    /// dome's light would suggest: Bevy draws `Add` premultiplied, where a
    /// colour with alpha zero is still added, and the mask is an alpha.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    /// See [`RIM_RUNG`], [`RING_RUNG`] and [`DOME_RUNG`].
    fn depth_bias(&self) -> f32 {
        crate::table::sort_bias(match self.kind() {
            ShellKind::Rim => RIM_RUNG,
            ShellKind::Ring | ShellKind::DomeRing => RING_RUNG,
            ShellKind::Dome => DOME_RUNG,
            ShellKind::Wall => WALL_RUNG,
        })
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        if key.bind_group_data.double_sided {
            descriptor.primitive.cull_mode = None;
        }
        if key.bind_group_data.solid
            && let Some(depth) = descriptor.depth_stencil.as_mut()
        {
            depth.depth_write_enabled = Some(true);
        }
        Ok(())
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

/// A dome's profile standing at `step` of its row's full height: from its
/// plateau's edge down a quarter ellipse to its skirt on the felt, as
/// `(distance past the card's edge, height over its face)`.
#[must_use]
pub fn dome_profile(row: DomeRow, step: f32) -> [(f32, f32); DOME_RINGS] {
    let (run, rise) = (DOME_INSET, row.height * step + RIM_DROP);
    std::array::from_fn(|k| {
        #[expect(clippy::cast_precision_loss)] // eight rings
        let t = k as f32 / (DOME_RINGS - 1) as f32 * std::f32::consts::FRAC_PI_2;
        (row.margin - run + run * t.sin(), -RIM_DROP + rise * t.cos())
    })
}

/// The point `d` past the card's edge in the direction `angle` (degrees) out
/// of the corner `centre` stands on, and that direction.
///
/// Inside the card's edge a corner cannot keep a negative radius: it keeps
/// [`CAP_ROUND`] instead, which lies inside the true inset, so no point of a
/// dome is further past the edge than its profile says.
fn outline(centre: Vec2, angle: f32, d: f32) -> (Vec2, Vec2) {
    let round = (CARD_CORNER + d).max(CAP_ROUND);
    let pull = CARD_CORNER + d - round;
    let dir = Vec2::from_angle(angle.to_radians());
    (centre + centre.signum() * pull + dir * round, dir)
}

/// The smallest corner a dome's plateau keeps.
const CAP_ROUND: f32 = 0.03;

/// The four corners a card's outline turns round, and the angle each turn
/// starts at, anticlockwise seen from above.
fn corners() -> [(Vec2, f32); 4] {
    let (hw, hh, r) = (CARD_WIDTH / 2.0, CARD_HEIGHT / 2.0, CARD_CORNER);
    [
        (Vec2::new(hw - r, hh - r), 0.0),
        (Vec2::new(-hw + r, hh - r), 90.0),
        (Vec2::new(-hw + r, -hh + r), 180.0),
        (Vec2::new(hw - r, -hh + r), 270.0),
    ]
}

/// A dome as a mesh: a ring of the card's outline at each point of its
/// profile, joined, and its plateau closed over the card by a fan.
///
/// # Panics
///
/// Never: a dome has a few hundred vertices.
#[must_use]
pub fn dome_mesh(row: DomeRow, step: f32) -> Mesh {
    let profile = dome_profile(row, step);
    let mut at = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let uv = |p: Vec2| [p.x / CARD_WIDTH + 0.5, 0.5 - p.y / CARD_HEIGHT];
    for (k, &(d, z)) in profile.iter().enumerate() {
        // The slope's normal, from its neighbours on the profile: straight
        // up at the plateau's edge, straight out at the skirt.
        let (d0, z0) = profile[k.saturating_sub(1)];
        let (d1, z1) = profile[(k + 1).min(DOME_RINGS - 1)];
        for (centre, start) in corners() {
            for i in 0..=BAND_SEGMENTS {
                #[expect(clippy::cast_precision_loss)] // a handful of segments
                let angle = start + 90.0 * i as f32 / BAND_SEGMENTS as f32;
                let (p, dir) = outline(centre, angle, d);
                at.push(p.extend(z).to_array());
                normals.push(
                    (dir.extend(0.0) * (z0 - z1) + Vec3::Z * (d1 - d0))
                        .normalize_or(Vec3::Z)
                        .to_array(),
                );
                uvs.push(uv(p));
            }
        }
    }
    let around = u32::try_from(4 * (BAND_SEGMENTS + 1)).expect("a few dozen");
    let mut indices = Vec::new();
    for ring in 0..u32::try_from(DOME_RINGS - 1).expect("eight rings") {
        for i in 0..around {
            let next = (i + 1) % around;
            let (inner, outer) = (ring * around + i, (ring + 1) * around + i);
            let (inner_next, outer_next) = (ring * around + next, (ring + 1) * around + next);
            indices.extend_from_slice(&[inner, outer, outer_next, inner, outer_next, inner_next]);
        }
    }
    // The plateau: a fan from the middle of the card over the first ring.
    let middle = u32::try_from(at.len()).expect("a few hundred");
    at.push([0.0, 0.0, profile[0].1]);
    normals.push([0.0, 0.0, 1.0]);
    uvs.push([0.5, 0.5]);
    for i in 0..around {
        indices.extend_from_slice(&[middle, i, (i + 1) % around]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, at)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Defender's wall as a mesh, in the card's own space with its face at
/// `z = 0` and the felt [`RIM_DROP`] under it: two courses of brick along a
/// shallow arc past the card's top edge, and [`WALL_MERLONS`] merlons on
/// them. Every face faces out of the solid, and `uv.x` is the distance
/// along the arc, which the bricks are laid by.
///
/// # Panics
///
/// Never: a wall has a few hundred vertices.
#[must_use]
pub fn wall_mesh() -> Mesh {
    let mut at: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut quad = |corners: [(Vec3, f32); 4], normal: Vec3| {
        let base = u32::try_from(at.len()).expect("a few hundred");
        for (p, along) in corners {
            at.push(p.to_array());
            normals.push(normal.to_array());
            uvs.push([along, 0.0]);
        }
        let [a, b, c, _] = corners.map(|(p, _)| p);
        let order = if (b - a).cross(c - a).dot(normal) >= 0.0 {
            [0, 1, 2, 0, 2, 3]
        } else {
            [0, 2, 1, 0, 3, 2]
        };
        indices.extend(order.map(|i| base + i));
    };
    let foot = -RIM_DROP;
    let crenels = foot + 2.0 * WALL_COURSE;
    let top = foot + WALL_HEIGHT;
    let mut slab = |from: f32, to: f32, low: f32, high: f32, pieces: usize| {
        let arc: Vec<(Vec2, Vec2, f32)> = (0..=pieces)
            .map(|i| {
                #[expect(clippy::cast_precision_loss)] // a few dozen pieces
                let u = from + (to - from) * i as f32 / pieces as f32;
                wall_arc(u)
            })
            .collect();
        for pair in arc.windows(2) {
            let [(c0, n0, s0), (c1, n1, s1)] = [pair[0], pair[1]];
            let half = WALL_THICK / 2.0;
            let (o0, o1) = (c0 + n0 * half, c1 + n1 * half);
            let (i0, i1) = (c0 - n0 * half, c1 - n1 * half);
            let n = (n0 + n1).normalize().extend(0.0);
            quad(
                [
                    (o0.extend(low), s0),
                    (o1.extend(low), s1),
                    (o1.extend(high), s1),
                    (o0.extend(high), s0),
                ],
                n,
            );
            quad(
                [
                    (i0.extend(low), s0),
                    (i1.extend(low), s1),
                    (i1.extend(high), s1),
                    (i0.extend(high), s0),
                ],
                -n,
            );
            quad(
                [
                    (i0.extend(high), s0),
                    (o0.extend(high), s0),
                    (o1.extend(high), s1),
                    (i1.extend(high), s1),
                ],
                Vec3::Z,
            );
        }
        for (end, outward) in [(arc[0], -1.0), (arc[arc.len() - 1], 1.0)] {
            let (c, n, s) = end;
            let half = WALL_THICK / 2.0;
            let along = Vec2::new(n.y, -n.x) * outward;
            quad(
                [
                    ((c - n * half).extend(low), s),
                    ((c + n * half).extend(low), s),
                    ((c + n * half).extend(high), s),
                    ((c - n * half).extend(high), s),
                ],
                along.extend(0.0),
            );
        }
    };
    slab(0.0, 1.0, foot, crenels, 24);
    let parts = 2 * WALL_MERLONS - 1;
    for k in 0..WALL_MERLONS {
        #[expect(clippy::cast_precision_loss)] // five merlons
        let (from, to) = (
            (2 * k) as f32 / parts as f32,
            (2 * k + 1) as f32 / parts as f32,
        );
        slab(from, to, crenels, top, 3);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, at)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// The wall's middle line at `u` along it (0 at its left end, 1 at its
/// right, seen from the card): where it is, which way is out (towards the
/// table's middle), and how far along the arc that is.
#[must_use]
pub fn wall_arc(u: f32) -> (Vec2, Vec2, f32) {
    let chord = CARD_WIDTH + 2.0 * WALL_OVERHANG;
    let x = (u - 0.5) * chord;
    let across = 2.0 * x / chord;
    let near = CARD_HEIGHT / 2.0 + WALL_NEAR + WALL_THICK / 2.0;
    let y = near + WALL_BULGE * (1.0 - across * across);
    let slope = -8.0 * WALL_BULGE * x / (chord * chord);
    let out = Vec2::new(-slope, 1.0).normalize();
    // The arc is shallow enough that its length is its chord's to within a
    // brick's thousandth; bricks are laid by it, not measured against it.
    (Vec2::new(x, y), out, x + chord / 2.0)
}

/// Which of [`DOME_STEPS`] a dome of `row` stands at round the card at `me`,
/// or `None` for its ring on the felt: the tallest that lands on no other
/// card's print ([`shell_stands`]).
///
/// `standing` is the step it stands at now. A step taller than that, or any
/// step at all for a dome lying down, must fit [`STAND_AGAIN`] higher than it
/// is, for the reason a ring stands up again only with that headroom.
#[must_use]
pub fn dome_step(
    row: DomeRow,
    standing: Option<usize>,
    me: &Footprint,
    others: impl Iterator<Item = Footprint> + Clone,
    eye: Vec3,
) -> Option<usize> {
    (0..DOME_STEPS.len()).find(|&step| {
        let headroom = if standing.is_some_and(|now| now <= step) {
            0.0
        } else {
            STAND_AGAIN
        };
        shell_stands(
            &dome_profile(row, DOME_STEPS[step]),
            me,
            others.clone(),
            eye,
            headroom,
        )
    })
}

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
    profile_reach(&RIM_EDGES, over - RIM_RISE * scale, scale, tangent).max(0.0)
}

/// How far past its card's edge a shell of this profile throws itself onto a
/// face that the card's own face stands `lift` above, seen along rays of
/// tangent `tangent`, on a card grown by `scale`.
///
/// The profile is a run of `(distance past the card's edge, height over its
/// face)` points, joined by straight lines as its mesh joins them. A face
/// hides every part of the shell below it, and a point above it lands its
/// height over it × the tangent further out. Along a straight line that
/// landing is straight too, so the furthest one is at a point of the profile
/// or where a line goes under the face.
///
/// Negative when everything above the face lands inside the card's own
/// edge, as a dome's plateau does; `f32::NEG_INFINITY` when nothing of the
/// shell stands above the face at all.
#[must_use]
pub fn profile_reach(profile: &[(f32, f32)], lift: f32, scale: f32, tangent: f32) -> f32 {
    let mut reach = f32::NEG_INFINITY;
    let mut take = |d: f32, h: f32| reach = reach.max(d + h * tangent);
    let at = |(d, z): (f32, f32)| (d * scale, lift + z * scale);
    for (i, &point) in profile.iter().enumerate() {
        let (d, h) = at(point);
        if h >= 0.0 {
            take(d, h);
        }
        if let Some(&next) = profile.get(i + 1) {
            let (d1, h1) = at(next);
            // One above the face and one not: never level, so never a
            // division by zero.
            if (h > 0.0) != (h1 > 0.0) {
                take(d + (d1 - d) * h / (h - h1), 0.0);
            }
        }
    }
    reach
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
    shell_stands(&RIM_EDGES, me, others, eye, headroom)
}

/// [`rim_stands`] for a shell of any profile ([`profile_reach`]): whether it
/// can stand round the card at `me` without landing on another card's print.
#[must_use]
pub fn shell_stands(
    profile: &[(f32, f32)],
    me: &Footprint,
    others: impl IntoIterator<Item = Footprint>,
    eye: Vec3,
    headroom: f32,
) -> bool {
    let crest = profile
        .iter()
        .map(|&(_, z)| z)
        .fold(f32::NEG_INFINITY, f32::max);
    let top = me.high + crest * me.scale + headroom;
    let tangent = me
        .corners
        .iter()
        .map(|c| c.distance(Vec2::new(eye.x, eye.z)) / (eye.y - top).max(1e-3))
        .fold(0.0, f32::max);
    let (centre, radius) = (me.centre(), me.radius());
    others.into_iter().all(|other| {
        let reach = profile_reach(profile, me.high + headroom - other.low, me.scale, tangent);
        // All of it under that face, which hides it wherever the two meet.
        if reach == f32::NEG_INFINITY {
            return true;
        }
        // What lands inside the card's own edge can still land on a card
        // lying across that edge: a flier over its neighbour, under a dome's
        // plateau. So a card that overlaps this one takes any reach at all.
        let reach = reach.max(0.0);
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

    /// Two whole `std140` rows.
    #[test]
    fn the_shell_uniform_is_two_whole_rows() {
        assert_eq!(ShellParams::min_size().get(), 32);
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
            [
                "tint: vec4<f32>,",
                "kind: u32,",
                "motion: f32,",
                "inner: f32,",
                "outer: f32,"
            ],
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
            ("RIM_RISE", RIM_RISE),
            ("RIM_DROP", RIM_DROP),
            ("WALL_COURSE", WALL_COURSE),
            ("WALL_BRICK", WALL_BRICK),
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
            ("SHELL_DOME", ShellKind::Dome),
            ("SHELL_DOME_RING", ShellKind::DomeRing),
            ("SHELL_WALL", ShellKind::Wall),
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

    /// Every point of a dome's mesh is where its profile says or further in:
    /// no ring stands further past the card's edge than its point of the
    /// profile, which is the distance the guard measures its throw from, and
    /// each stands at that point's height.
    /// Defender's wall stands where its constants say: on the felt, no
    /// higher than [`WALL_HEIGHT`], [`WALL_NEAR`] past the card's top edge
    /// or further. And every face of it faces out of the solid, so the back
    /// faces the pipeline culls are the ones inside it: a face whose front
    /// is in the wall and whose back is out of it is drawn inside out.
    #[test]
    fn the_wall_stands_where_its_constants_say_and_faces_out() {
        let mesh = wall_mesh();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(at)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("a wall has positions");
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("a wall has normals");
        };
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("a wall is indexed");
        };
        let at: Vec<Vec3> = at.iter().map(|p| Vec3::from_array(*p)).collect();
        let (low, high) = at.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.z), hi.max(p.z))
        });
        assert!((low + RIM_DROP).abs() < 1e-6, "its foot is at {low}");
        assert!(
            (high - (WALL_HEIGHT - RIM_DROP)).abs() < 1e-6,
            "its top is at {high}"
        );
        let nearest = at.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        assert!(
            nearest >= CARD_HEIGHT / 2.0 + WALL_NEAR - 1e-5,
            "it comes to {nearest}"
        );
        let crenels = 2.0 * WALL_COURSE - RIM_DROP;
        let inside = |p: Vec3| {
            let u = p.x / (CARD_WIDTH + 2.0 * WALL_OVERHANG) + 0.5;
            if !(0.0..=1.0).contains(&u) || p.z < -RIM_DROP || p.z > WALL_HEIGHT - RIM_DROP {
                return false;
            }
            let (centre, out, _) = wall_arc(u);
            if (p.truncate() - centre).dot(out).abs() > WALL_THICK / 2.0 {
                return false;
            }
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=9
            let part = (u * (2 * WALL_MERLONS - 1) as f32) as usize;
            p.z <= crenels || part.is_multiple_of(2)
        };
        let mut out = 0;
        for tri in indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|k| at[tri[k] as usize]);
            let n = Vec3::from_array(normals[tri[0] as usize]);
            assert!(
                (b - a).cross(c - a).dot(n) > 0.0,
                "a face wound against its normal at {a}"
            );
            let middle = (a + b + c) / 3.0;
            let (front, back) = (inside(middle + n * 1e-3), inside(middle - n * 1e-3));
            assert!(!front || back, "a face at {middle} faces into the wall");
            if back && !front {
                out += 1;
            }
        }
        assert!(out > 200, "only {out} faces face out of the wall");
    }

    #[test]
    fn a_domes_mesh_stands_no_further_out_than_its_profile() {
        let row = Dome::Hexproof.row();
        for step in DOME_STEPS {
            let profile = dome_profile(row, step);
            assert!((profile[0].1 - row.height * step).abs() < 1e-6, "plateau");
            assert!(
                (profile[DOME_RINGS - 1].0 - row.margin).abs() < 1e-6,
                "skirt"
            );
            assert!(
                (profile[DOME_RINGS - 1].1 + RIM_DROP).abs() < 1e-6,
                "on the felt"
            );
            let mesh = dome_mesh(row, step);
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(at)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("a dome has positions");
            };
            let around = 4 * (BAND_SEGMENTS + 1);
            assert_eq!(at.len(), DOME_RINGS * around + 1, "rings and the middle");
            for (i, p) in at.iter().enumerate().take(DOME_RINGS * around) {
                let (d, z) = profile[i / around];
                let past = card_sdf(Vec2::new(p[0], p[1]));
                assert!(
                    past <= d + 1e-4,
                    "ring {} at {past}, profile {d}",
                    i / around
                );
                assert!((p[2] - z).abs() < 1e-6);
            }
            // At and outside the card's edge the rings are the true outline.
            for (i, p) in at.iter().enumerate().skip(4 * around).take(4 * around) {
                let d = profile[i / around].0;
                if d >= 0.0 {
                    assert!((card_sdf(Vec2::new(p[0], p[1])) - d).abs() < 1e-4);
                }
            }
        }
    }

    /// Shroud swallows hexproof: one dome, never two colours.
    #[test]
    fn shroud_swallows_hexproof() {
        assert_eq!(Dome::of(true, true), Some(Dome::Shroud));
        assert_eq!(Dome::of(false, true), Some(Dome::Shroud));
        assert_eq!(Dome::of(true, false), Some(Dome::Hexproof));
        assert_eq!(Dome::of(false, false), None);
    }

    /// A card lying flat at `(x, z)`, its face `y` over the table.
    fn flat(x: f32, z: f32, y: f32) -> Footprint {
        let (hw, hh) = (CARD_WIDTH / 2.0, CARD_HEIGHT / 2.0);
        Footprint {
            corners: [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
                .map(|(dx, dz)| Vec2::new(x + dx, z + dz)),
            low: y,
            high: y,
            scale: 1.0,
        }
    }

    /// A dome stands as tall as its air allows: at full height alone, lower
    /// with a neighbour a little way off, lying down beside one a ring's row
    /// away; and a dome lying down, or standing lower, rises only with a
    /// flier's bob of headroom.
    #[test]
    fn a_dome_stands_as_tall_as_its_air_allows() {
        let row = Dome::Shroud.row();
        let eye = Vec3::new(0.0, 20.0, 8.0);
        let me = flat(0.0, 0.0, 0.083);
        let step = |standing, others: &[Footprint]| {
            dome_step(row, standing, &me, others.iter().copied(), eye)
        };
        assert_eq!(step(Some(0), &[]), Some(0), "alone, at full height");
        assert_eq!(step(None, &[]), Some(0), "alone, a lying dome stands up");
        let tight = [flat(1.0185, 0.0, 0.083)];
        assert_eq!(step(Some(0), &tight), None, "a ring's row away");
        // Somewhere between, a lower step fits where the full one does not.
        let lower = (0..200).map(|i| 1.0 + 0.002 * i as f32).find_map(|x| {
            let beside = [flat(x, 0.0, 0.083)];
            match step(Some(0), &beside) {
                Some(s) if s > 0 => Some((x, s)),
                _ => None,
            }
        });
        let (x, s) = lower.expect("some gap stands a dome lower than full height");
        // Hysteresis: from lying down, or from lower, it needs more air.
        let beside = [flat(x, 0.0, 0.083)];
        assert!(
            step(None, &beside).is_none_or(|up| up >= s),
            "a lying dome stood up taller than a standing one would"
        );
        let at_edge = (0..400)
            .map(|i| 1.0 + 0.001 * i as f32)
            .find(|&x| step(Some(DOME_STEPS.len() - 1), &[flat(x, 0.0, 0.083)]).is_some())
            .expect("the lowest step fits somewhere");
        assert!(
            step(None, &[flat(at_edge, 0.0, 0.083)]).is_none(),
            "a ring at the edge of fitting stood up without headroom"
        );
    }
}
