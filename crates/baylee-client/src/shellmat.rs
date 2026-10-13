//! The shells round a permanent (#298): what its protection and its
//! summoning sickness look like on the table, now that the frame that
//! carried a mark for them is gone.
//!
//! **A keyword wears a slim border** (the owner, 13.10.2026: "nicht mehr
//! 3D-Kuppel, nur schöner animierter schlanker Rand"). Indestructible,
//! defender, hexproof and shroud each draw one layer of a band standing
//! round the card, from a hair above its face ([`RIM_RISE`]) down to the
//! felt ([`RIM_DROP`]), [`border_margin`] out: nested inside out in that
//! order when a card wears several (shroud swallows hexproof, as the strip's
//! marks do), each told apart by its pattern and its motion as well as its
//! colour, so a player who cannot tell the hues apart still can
//! (`shell_common.wgsl`'s `ward_layer`): a steady golden rim with one glint
//! going round it; a wall of stone blocks; a thin blue line with light
//! flowing along it; a soft violet mist with no edge at all. The band is
//! one mesh and one material per set of keywords ([`Ward`]), so a card
//! wearing three is one draw.
//!
//! **Nothing of a border is drawn over its own print.** A keyword lasts as
//! long as the keyword, so it is not one of the passing states the owner
//! lets lie on the print (13.10.2026, `docs/legal.md` §3). The vertex stage
//! hands the fragment the camera in the shell's own space; the fragment
//! follows the ray from the camera through its point to the card's face
//! plane, and where that ray meets the print its alpha is exactly zero
//! ([`clear_over_print`], which `shell.wgsl` mirrors). It is the real
//! camera and the real ray, so the rule holds at every yaw, lean, screen
//! corner, hover and tap.
//!
//! **Nothing of a shell lies on another card's print.** The mask cannot see
//! a neighbour, so this rule is geometry. [`shell_stands`] follows, from
//! where every card is this frame and where the camera is, where each part
//! of the shell above another card's face lands on that face: further along
//! the camera's ray, away from the camera. A border that would land on a
//! print lies down on the felt instead, the same layers as a flat ring from
//! [`RING_INNER`] out, under every card ([`RING_RUNG`]).
//!
//! The border can stand in a lane at all because it falls to the felt. A
//! card beside it at the same height hides everything of it below its own
//! face, so what can reach that card is only what stands above it: the
//! border's top edge, [`RIM_RISE`] over the face, thrown [`RIM_RISE`] × the
//! ray's tangent past the edge, about 0.013 of a card width; two untapped
//! cards in neighbouring rows of a ring are 0.0185 apart.
//!
//! Summoning sickness (CR 302.6) wears a slow wave of moonlight running out
//! over the card from its middle, a real ripple a hair over its face,
//! resting before the next ([`WAVE_PERIOD`]), beside the moonlight its own
//! print breathes while it lasts (`cardmat::veil::ASLEEP`). It lies over its
//! own print, as the arrival sweep does, and leaves nothing behind (the
//! owner, 25.09), so it is the one shell the mask exempts. It keeps the
//! other rule: it is the same guard over a flat profile ([`WAVE_PROFILE`]),
//! and where it would land on another card's print, under the next card of a
//! fanned row above all, it is not drawn. With motion off it holds still,
//! crest and all.

use baylee_client_core::airborne;
use baylee_client_core::board::{CardGroup, KeywordBadge};
use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH};
use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::table::{CARD_CORNER, CARD_LIFT, CARD_THICKNESS, FLOOR_RUNG, STRIP_RUNG};

/// How far above the card's face a border's top edge stands.
pub const RIM_RISE: f32 = 0.010;
/// How far below the card's face a border's foot hangs: to the felt, for a
/// card lying on it.
pub const RIM_DROP: f32 = CARD_THICKNESS + CARD_LIFT;
/// How far past the print the mask takes to open, outward only: exactly zero
/// on the print, whole this far off it.
pub const MASK_FEATHER: f32 = 0.012;
/// How far past the card's edge a border's foot stands with no layer, and
/// how much further out each layer takes it: one layer 0.05, three 0.11.
pub const BORDER_BASE: f32 = 0.02;
/// See [`BORDER_BASE`].
pub const BORDER_LAYER: f32 = 0.03;
/// The most layers a border has: indestructible, defender, and hexproof or
/// shroud.
pub const MAX_LAYERS: u32 = 3;
/// Where the ring a border lies down to starts past the card's edge:
/// outside the offer's light on the felt ([`crate::floormat::REACH`]), so
/// the two are never one band of mixed colour; and how wide each of its
/// layers is.
pub const RING_INNER: f32 = 0.10;
/// See [`RING_INNER`].
pub const RING_LAYER: f32 = 0.025;
/// The height the ring lies at: over the offer's light and the contact
/// shadows, under every card.
pub const RING_RUNG: f32 = CARD_LIFT * 0.8;
/// Where a standing border sits in the transparent pass: under the strip
/// and the count badge, which stands at the card's top-right corner.
pub const BORDER_RUNG: f32 = STRIP_RUNG - 0.001;
/// How much higher a lying border must be able to stand than it is before
/// it stands up again. A flier bobs [`airborne::SWAY`] either way, and
/// without this a flier at the edge of fitting would swap border and ring
/// on every bob: with it, a ring stands up only if its border would fit at
/// the top of the bob.
pub const STAND_AGAIN: f32 = 2.0 * airborne::SWAY;

const _: () = assert!(RING_INNER >= crate::floormat::REACH);
// Over the offer's light by a rung of the ladder, and under every card.
const _: () = assert!(RING_RUNG >= FLOOR_RUNG + 0.0005 && RING_RUNG < CARD_LIFT);
const _: () = assert!(RIM_RISE > 0.0 && BORDER_BASE > 0.0 && BORDER_LAYER > 0.0);

/// How far past the card's edge a border of `layers` layers stands at its
/// foot.
#[must_use]
pub fn border_margin(layers: u32) -> f32 {
    #[expect(clippy::cast_precision_loss)] // three layers at most
    let n = layers.clamp(1, MAX_LAYERS) as f32;
    BORDER_BASE + BORDER_LAYER * n
}

/// A standing border's two edges, as `(distance past the card's edge,
/// height over its face)`: its top at the card's edge, [`RIM_RISE`] over the
/// face, and its foot [`border_margin`] out and [`RIM_DROP`] down.
#[must_use]
pub fn border_edges(layers: u32) -> [(f32, f32); 2] {
    [(0.0, RIM_RISE), (border_margin(layers), -RIM_DROP)]
}

/// The ring a border of `layers` layers lies down to, flat, as its two
/// edges past the card's edge.
#[must_use]
pub fn ring_edges(layers: u32) -> [(f32, f32); 2] {
    #[expect(clippy::cast_precision_loss)] // three layers at most
    let n = layers.clamp(1, MAX_LAYERS) as f32;
    [(RING_INNER, 0.0), (RING_INNER + RING_LAYER * n, 0.0)]
}

/// How far over the card's face summoning sickness's wave lies where no
/// crest lifts it.
pub const WAVE_LIFT: f32 = 0.003;
/// How high the wave's crest rises over that as its front passes.
pub const WAVE_CREST: f32 = 0.009;
/// The highest any point of the wave stands over its card's face.
pub const WAVE_TOP: f32 = WAVE_LIFT + WAVE_CREST;
/// How far inside the card's edge the wave's sheet stops: past the card's
/// rounded corner, so the sheet is a plain rectangle over the print.
pub const WAVE_INSET: f32 = 0.05;
/// How long one wave takes to run out to its sheet's edge, and how long from
/// one wave to the next, in seconds: quick enough to be seen at a glance
/// (the owner, 13.10.2026), resting between.
pub const WAVE_TRAVEL: f32 = 2.8;
/// See [`WAVE_TRAVEL`].
pub const WAVE_PERIOD: f32 = 4.2;
/// The wave's sheet's cells, across and down the card.
pub const WAVE_CELLS: (u32, u32) = (18, 26);
/// Where the wave sits in the transparent pass: over the rings, under the
/// border and the strip, whose label lies over it.
pub const WAVE_RUNG: f32 = BORDER_RUNG - 0.0005;
/// The wave as the guard measures it ([`shell_stands`]): its sheet with the
/// crest standing everywhere at once, from the card's middle line out to
/// [`WAVE_INSET`] inside its edge.
pub const WAVE_PROFILE: [(f32, f32); 2] = [(-CARD_WIDTH / 2.0, WAVE_TOP), (-WAVE_INSET, WAVE_TOP)];

const _: () = assert!(WAVE_INSET > CARD_CORNER);
const _: () = assert!(WAVE_RUNG > RING_RUNG && WAVE_RUNG < BORDER_RUNG);
const _: () = assert!(WAVE_TRAVEL < WAVE_PERIOD);

/// Which shell a material draws, as `shell_common.wgsl` numbers it for the
/// table's shader and the preview's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ShellKind {
    /// A keyword's border, standing round the card.
    Border = 1,
    /// The same border as a ring on the felt, where it has no room.
    Ring = 2,
    /// Summoning sickness's wave of moonlight over the card.
    Wave = 3,
}

/// The keywords a permanent's border says, as `shell_common.wgsl`'s `WARD_*`
/// bits. Shroud swallows hexproof: a permanent with both may be targeted by
/// nobody, which is shroud, so it wears one layer and not two.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ward(pub u32);

impl Ward {
    /// No keyword: no border.
    pub const NONE: Self = Self(0);
    /// Indestructible: a steady golden rim, the innermost layer.
    pub const INDESTRUCTIBLE: u32 = 1;
    /// Defender: a wall of stone blocks.
    pub const DEFENDER: u32 = 2;
    /// Hexproof: a thin blue line with light flowing along it.
    pub const HEXPROOF: u32 = 4;
    /// Shroud: a soft violet mist with no edge.
    pub const SHROUD: u32 = 8;

    /// The border of a permanent with the keywords `bits` names
    /// ([`Self::INDESTRUCTIBLE`] and the rest): shroud swallows hexproof.
    #[must_use]
    pub const fn of(bits: u32) -> Self {
        if bits & Self::SHROUD == 0 {
            Self(bits)
        } else {
            Self(bits & !Self::HEXPROOF)
        }
    }

    /// How many layers the border has.
    #[must_use]
    pub const fn layers(self) -> u32 {
        self.0.count_ones()
    }

    /// Whether there is no border at all.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// The shells a permanent wears, from its keywords and its summoning
/// sickness: the one door the table's rows and the preview both ask
/// ([`crate::shellui`]), so the two never disagree about what a card wears.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shells {
    /// Its keywords' border.
    pub ward: Ward,
    /// Summoning sickness's wave, over the card's own face.
    pub wave: bool,
}

impl Shells {
    /// What the permanents of `group` wear. Only a creature is ever
    /// modelled asleep ([`baylee_client_core::board::asleep`]), and a
    /// phased-out permanent wears nothing: it is treated as though it does
    /// not exist (CR 702.26b), and its grey says so.
    #[must_use]
    pub fn of(group: &CardGroup) -> Self {
        if group.status.is_phased_out() {
            return Self::default();
        }
        let has = |badge| group.badges.contains(&badge);
        Self {
            ward: Ward::of(
                [
                    (KeywordBadge::Indestructible, Ward::INDESTRUCTIBLE),
                    (KeywordBadge::Defender, Ward::DEFENDER),
                    (KeywordBadge::Hexproof, Ward::HEXPROOF),
                    (KeywordBadge::Shroud, Ward::SHROUD),
                ]
                .into_iter()
                .filter(|&(badge, _)| has(badge))
                .fold(0, |bits, (_, bit)| bits | bit),
            ),
            wave: group.summoning_sick,
        }
    }

    /// Whether any of them reaches past the card's edge, as a border does
    /// and the wave does not.
    #[must_use]
    pub fn reach_off_the_card(self) -> bool {
        !self.ward.is_empty()
    }

    /// What the preview draws, standing, in the order each is painted: the
    /// wave, then the border, both under the card's own objects (the strip,
    /// the plate and the count badge), as on the table. A new shell is a
    /// look here and an arm in `shell_ui.wgsl`.
    #[must_use]
    pub fn standing(self) -> Vec<ShellLook> {
        [
            self.wave.then_some(ShellLook::wave()),
            (!self.ward.is_empty()).then_some(ShellLook::border(self.ward)),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// Everything a shell's material is told: one material per look, however
/// many cards wear it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShellLook {
    /// What is drawn.
    pub kind: ShellKind,
    /// The keywords, for a border and its ring.
    pub ward: Ward,
}

impl ShellLook {
    /// Summoning sickness's wave.
    #[must_use]
    pub const fn wave() -> Self {
        Self {
            kind: ShellKind::Wave,
            ward: Ward::NONE,
        }
    }

    /// The border of `ward`, standing.
    #[must_use]
    pub const fn border(ward: Ward) -> Self {
        Self {
            kind: ShellKind::Border,
            ward,
        }
    }

    /// The border of `ward`, lying on the felt.
    #[must_use]
    pub const fn ring(ward: Ward) -> Self {
        Self {
            kind: ShellKind::Ring,
            ward,
        }
    }
}

/// What the shell's shader reads.
///
/// Two whole `std140` rows, for the reason
/// [`FloorParams`](crate::floormat::FloorParams) is one.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct ShellParams {
    /// A [`ShellKind`].
    pub kind: u32,
    /// The clock: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    pub motion: f32,
    /// Where a ring's band starts and ends past the card's edge.
    pub inner: f32,
    /// See [`Self::inner`].
    pub outer: f32,
    /// The keywords ([`Ward`]).
    pub ward: u32,
    /// How many layers they make.
    pub layers: u32,
    /// Nothing: the second row's rest.
    pub spare: Vec2,
}

impl ShellParams {
    /// What the shader is told to draw `look` on the clock `motion`.
    #[must_use]
    pub fn new(look: ShellLook, motion: f32) -> Self {
        let layers = look.ward.layers();
        let [(inner, _), (outer, _)] = match look.kind {
            ShellKind::Ring => ring_edges(layers),
            ShellKind::Border | ShellKind::Wave => {
                let [(_, _), (outer, _)] = border_edges(layers);
                [(0.0, 0.0), (outer, 0.0)]
            }
        };
        Self {
            kind: look.kind as u32,
            motion,
            inner,
            outer,
            ward: look.ward.0,
            layers,
            spare: Vec2::ZERO,
        }
    }
}

/// One shell's material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ShellMaterial {
    /// The look and the clock.
    #[uniform(0)]
    pub params: ShellParams,
}

impl ShellMaterial {
    /// The material for `look` on the clock `motion`.
    #[must_use]
    pub fn new(look: ShellLook, motion: f32) -> Self {
        Self {
            params: ShellParams::new(look, motion),
        }
    }

    fn kind(&self) -> ShellKind {
        match self.params.kind {
            2 => ShellKind::Ring,
            3 => ShellKind::Wave,
            _ => ShellKind::Border,
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

    /// Blended, so the mask's zero is nothing at all. Not `Add`: Bevy draws
    /// `Add` premultiplied, where a colour with alpha zero is still added,
    /// and the mask is an alpha.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    /// See [`BORDER_RUNG`], [`RING_RUNG`] and [`WAVE_RUNG`].
    fn depth_bias(&self) -> f32 {
        crate::table::sort_bias(match self.kind() {
            ShellKind::Border => BORDER_RUNG,
            ShellKind::Ring => RING_RUNG,
            ShellKind::Wave => WAVE_RUNG,
        })
    }
}

/// Registers the material and ships its shaders in the binary, for the reason
/// [`CardMaterialPlugin`](crate::cardmat::CardMaterialPlugin) embeds its own:
/// `shell.wgsl`, and `shell_common.wgsl`, which the preview's shells
/// ([`crate::shellui`]) import as well.
pub struct ShellMaterialPlugin;

impl Plugin for ShellMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/shell_common.wgsl");
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

/// Summoning sickness's wave as a mesh, flat, in the card's own space with
/// its face at `z = 0`: a grid of [`WAVE_CELLS`] over the card,
/// [`WAVE_INSET`] in from its edge. `shell.wgsl`'s vertex stage lifts it.
///
/// # Panics
///
/// Never: a wave has a few hundred vertices.
#[must_use]
pub fn wave_mesh() -> Mesh {
    let (cols, rows) = WAVE_CELLS;
    let (hw, hh) = (
        CARD_WIDTH / 2.0 - WAVE_INSET,
        CARD_HEIGHT / 2.0 - WAVE_INSET,
    );
    let mut at = Vec::new();
    let mut uvs = Vec::new();
    for j in 0..=rows {
        for i in 0..=cols {
            #[expect(clippy::cast_precision_loss)] // a few dozen cells
            let (u, v) = (i as f32 / cols as f32, j as f32 / rows as f32);
            at.push([-hw + 2.0 * hw * u, hh - 2.0 * hh * v, 0.0]);
            uvs.push([u, v]);
        }
    }
    let mut indices = Vec::new();
    for j in 0..rows {
        for i in 0..cols {
            let a = j * (cols + 1) + i;
            let (b, c, d) = (a + 1, a + cols + 1, a + cols + 2);
            // Anticlockwise seen from over the face.
            indices.extend([a, c, b, b, c, d]);
        }
    }
    let normals = vec![[0.0, 0.0, 1.0]; at.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, at)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
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

/// Whether a border of `layers` layers can stand round the card at `me`
/// without landing on the print of any card in `others`, seen from a camera
/// at `eye`, if it stood `headroom` higher than it does: [`shell_stands`]
/// over the border's profile.
#[must_use]
pub fn border_stands(
    layers: u32,
    me: &Footprint,
    others: impl IntoIterator<Item = Footprint>,
    eye: Vec3,
    headroom: f32,
) -> bool {
    shell_stands(&border_edges(layers), me, others, eye, headroom)
}

/// Whether summoning sickness's wave may be drawn over the card at `me`
/// this frame, seen from `eye`, `headroom` higher than it stands: the one
/// guard over [`WAVE_PROFILE`].
#[must_use]
pub fn wave_stands(
    me: &Footprint,
    others: impl IntoIterator<Item = Footprint>,
    eye: Vec3,
    headroom: f32,
) -> bool {
    shell_stands(&WAVE_PROFILE, me, others, eye, headroom)
}

/// Whether a shell of this profile (`(distance past the card's edge, height
/// over its face)` points, joined by straight runs as its mesh joins them)
/// can stand round the card at `me` without
/// landing on the print of any card in `others`, seen from a camera at
/// `eye`, if it stood `headroom` higher than it does.
///
/// A point of the shell above another card's face lands on that face
/// further along the camera's ray, away from the camera: its height over
/// that face × the ray's tangent. So each run of the profile, clipped to
/// the part above that face, lands inside the hull of four rectangles: the
/// card's own grown by the run's two ends' distance past its edge, each
/// carried away from the camera by the least and the most any of its rays
/// carries it, and grown by how far the rays across the card fan out from
/// the one through its middle. A shell stands if no hull meets another
/// card's face ([`overlap`]).
///
/// Its own card's print hides what lies under it: a run whose every ray
/// crosses the card's own face inside its edge cannot land on a card lower
/// than that face, and is only measured against faces level with it or
/// above it. A face
/// higher than the shell's top hides the shell.
#[must_use]
pub fn shell_stands(
    profile: &[(f32, f32)],
    me: &Footprint,
    others: impl IntoIterator<Item = Footprint>,
    eye: Vec3,
    headroom: f32,
) -> bool {
    let scale = me.scale;
    let (low_z, high_z) = profile
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &(_, z)| {
            (lo.min(z), hi.max(z))
        });
    let widest = profile.iter().map(|&(d, _)| d).fold(0.0, f32::max) * scale;
    let (top, bottom) = (me.high + high_z * scale + headroom, me.low + low_z * scale);
    let flat = Vec2::new(eye.x, eye.z);
    let centre = me.centre();
    let radius = me.radius() + widest * std::f32::consts::SQRT_2;
    let far = centre.distance(flat);
    let (near_run, far_run) = ((far - radius).max(0.0), far + radius);
    // The least and the most a ray through any point of the shell is
    // carried out per unit of height it drops.
    let least = near_run / (eye.y - bottom).max(1e-3);
    let most = far_run / (eye.y - top).max(1e-3);
    let away = (centre - flat).normalize_or_zero();
    let fan = (radius / far.max(1e-3)).min(1.0);
    let sides = [me.corners[1] - me.corners[0], me.corners[3] - me.corners[0]];
    let axes = sides.map(Vec2::normalize_or_zero);
    let halves = sides.map(|side| side.length() / 2.0);
    let signs = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    // The card's face grown by `grow` (shrunk, when it is negative, no
    // further than to its middle line), carried `by` away from the camera.
    let rect = |grow: f32, by: f32| -> [Vec2; 4] {
        let (gx, gy) = (grow.max(-halves[0]), grow.max(-halves[1]));
        std::array::from_fn(|k| {
            let (sx, sy) = signs[k];
            me.corners[k] + axes[0] * (sx * gx) + axes[1] * (sy * gy) + away * by
        })
    };
    // Where one point of the profile, `h` over a face, may land on it.
    let lands = |d: f32, h: f32, out: &mut Vec<Vec2>| {
        let spread = h * most * fan;
        out.extend(rect(d * scale + spread, h * least));
        out.extend(rect(d * scale + spread, h * most));
    };
    // Whether every ray through the profile's point `(d, z)` crosses the
    // card's own face inside its edge.
    let hidden_by_own =
        |(d, z): (f32, f32)| d * scale + z.abs() * scale * most * (1.0 + fan) <= 0.0;
    others.into_iter().all(|other| {
        // A face over the whole shell hides it.
        if other.low >= top {
            return true;
        }
        let lift = me.high + headroom - other.low;
        let reach = (lift + high_z * scale).max(0.0) * most;
        if centre.distance(other.centre()) > radius + reach + other.radius() {
            return true;
        }
        // Strictly: of two faces level with each other, nothing says whose
        // print is on top where they overlap.
        let below_me = other.high < me.low;
        let (there, other_radius) = (other.centre(), other.radius());
        let mut landed = Vec::with_capacity(24);
        for pair in profile.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if below_me && hidden_by_own(a) && hidden_by_own(b) {
                continue;
            }
            let (ha, hb) = (lift + a.1 * scale, lift + b.1 * scale);
            if ha <= 0.0 && hb <= 0.0 {
                continue;
            }
            // Too far off to meet: a circle round everything the run can
            // land on, against one round the other card.
            let (h, d) = (ha.max(hb).max(0.0), a.0.max(b.0) * scale);
            let middle = centre + away * (h * (least + most) / 2.0);
            let round = me.radius()
                + (d + h * most * fan).max(0.0) * std::f32::consts::SQRT_2
                + h * (most - least) / 2.0;
            if middle.distance(there) > round + other_radius {
                continue;
            }
            landed.clear();
            for ((d, _), h) in [(a, ha), (b, hb)] {
                if h > 0.0 {
                    lands(d, h, &mut landed);
                }
            }
            // One end under the face: the run goes under it where it
            // crosses it, and lands there.
            if (ha > 0.0) != (hb > 0.0) {
                let d = a.0 + (b.0 - a.0) * ha / (ha - hb);
                lands(d, 0.0, &mut landed);
            }
            if overlap(&landed, &other.corners) {
                return false;
            }
        }
        true
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

/// Whether the convex hull of the points `a` meets the convex polygon `b`
/// (its corners in order round it): no edge of either separates them.
fn overlap(a: &[Vec2], b: &[Vec2]) -> bool {
    let a = hull(a);
    for poly in [a.as_slice(), b] {
        for i in 0..poly.len() {
            let edge = poly[(i + 1) % poly.len()] - poly[i];
            let axis = Vec2::new(-edge.y, edge.x);
            let span = |p: &[Vec2]| {
                p.iter()
                    .map(|v| v.dot(axis))
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), x| {
                        (lo.min(x), hi.max(x))
                    })
            };
            let ((a0, a1), (b0, b1)) = (span(&a), span(b));
            if a1 < b0 || b1 < a0 {
                return false;
            }
        }
    }
    true
}

/// The convex hull of `points`, anticlockwise (Andrew's monotone chain).
fn hull(points: &[Vec2]) -> Vec<Vec2> {
    let mut sorted = points.to_vec();
    sorted.sort_by(|p, q| p.x.total_cmp(&q.x).then(p.y.total_cmp(&q.y)));
    sorted.dedup();
    if sorted.len() < 3 {
        return sorted;
    }
    let turn = |o: Vec2, a: Vec2, b: Vec2| (a - o).perp_dot(b - o);
    let mut out: Vec<Vec2> = Vec::with_capacity(sorted.len() + 1);
    for pass in [sorted.clone(), sorted.into_iter().rev().collect()] {
        let start = out.len();
        for p in pass {
            while out.len() >= start + 2 && turn(out[out.len() - 2], out[out.len() - 1], p) <= 0.0 {
                out.pop();
            }
            out.push(p);
        }
        out.pop();
    }
    out
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
    /// What it shares with the preview's shells: the card, the kinds, the
    /// border's layers and the wave.
    const COMMON: &str = include_str!("shaders/shell_common.wgsl");

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
            &format!(
                "{prelude}{}{COMMON}",
                include_str!("shaders/card_common.wgsl")
            ),
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
                "kind: u32,",
                "motion: f32,",
                "inner: f32,",
                "outer: f32,",
                "ward: u32,",
                "layers: u32,",
                "spare: vec2<f32>,"
            ],
            "shell.wgsl lays the uniform out differently from `ShellParams`"
        );
    }

    /// Every number the shaders and this file share is the same number in
    /// both: a mask cut to a different card than the border is built round
    /// is a mask with a gap in it, and a keyword bit read as another is a
    /// card wearing the wrong border.
    #[test]
    fn the_shader_measures_the_card_this_file_does() {
        for (name, ours) in [
            ("CARD_HALF_W", CARD_WIDTH / 2.0),
            ("CARD_HALF_H", CARD_HEIGHT / 2.0),
            ("CARD_ROUND", CARD_CORNER),
            ("MASK_FEATHER", MASK_FEATHER),
            ("RIM_RISE", RIM_RISE),
            ("RIM_DROP", RIM_DROP),
            ("WAVE_LIFT", WAVE_LIFT),
            ("WAVE_CREST", WAVE_CREST),
            ("WAVE_INSET", WAVE_INSET),
            ("WAVE_TRAVEL", WAVE_TRAVEL),
            ("WAVE_PERIOD", WAVE_PERIOD),
        ] {
            let theirs = wgsl_const(COMMON, name);
            assert!(
                (theirs - ours).abs() < 1e-4,
                "{name}: {ours} here, {theirs} in the shader"
            );
        }
        #[expect(clippy::cast_precision_loss)] // small numbers
        for (name, ours) in [
            ("SHELL_BORDER", ShellKind::Border as u32),
            ("SHELL_RING", ShellKind::Ring as u32),
            ("SHELL_WAVE", ShellKind::Wave as u32),
            ("WARD_INDESTRUCTIBLE", Ward::INDESTRUCTIBLE),
            ("WARD_DEFENDER", Ward::DEFENDER),
            ("WARD_HEXPROOF", Ward::HEXPROOF),
            ("WARD_SHROUD", Ward::SHROUD),
        ] {
            assert!(
                (wgsl_const(COMMON, name) - ours as f32).abs() < f32::EPSILON,
                "{name}"
            );
        }
    }

    /// Every colour the fragment stage returns but the wave's has the mask
    /// as the last factor of its alpha: a `return` that forgot it would be a
    /// border or a ring drawn over its own print from whichever side the
    /// camera happened to be on. A keyword lasts, so its border never lies
    /// on the print; only the wave, which passes, may (the owner, 25.09 and
    /// 13.10.2026).
    #[test]
    fn every_colour_but_the_waves_carries_the_mask() {
        let open = SHADER.find("fn fragment(").expect("the fragment stage");
        let body = &SHADER[open..];
        let branch = "if params.kind == SHELL_WAVE {";
        let start = body.find(branch).expect("the branch");
        let end = start + body[start..].find("\n    }").expect("it closes");
        let own = &body[start..end];
        assert_eq!(own.matches("return").count(), 1, "`{branch}` returns once");
        assert!(own.contains("return vec4<f32>(MOONLIGHT, glow);"));
        let mut at = 0;
        let mut returns = Vec::new();
        for line in body.split_inclusive('\n') {
            let here = at;
            at += line.len();
            if !(start..end).contains(&here) && line.trim().starts_with("return") {
                returns.push(line.trim());
            }
        }
        assert!(!returns.is_empty(), "the fragment stage returns too little");
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

    /// The wave's sheet lies wholly over the print, [`WAVE_INSET`] inside the
    /// card's edge all the way round, and flat on the face until the vertex
    /// stage lifts it.
    #[test]
    #[allow(clippy::float_cmp)] // exactly flat is the claim
    fn the_waves_sheet_lies_over_the_print() {
        let mesh = wave_mesh();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(at)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("a wave has positions");
        };
        let (cols, rows) = WAVE_CELLS;
        assert_eq!(at.len(), ((cols + 1) * (rows + 1)) as usize);
        let outmost = at
            .iter()
            .map(|p| card_sdf(Vec2::new(p[0], p[1])))
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            (outmost + WAVE_INSET).abs() < 1e-4,
            "the sheet reaches {outmost} past the card's edge"
        );
        assert!(at.iter().all(|p| p[2] == 0.0), "the sheet is flat");
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
