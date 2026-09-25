//! The shells round a permanent (#298): what its protection looks like on
//! the table, now that the frame that carried a mark for it is gone.
//!
//! The first is indestructible: a rim of darksteel standing round the
//! card, from a hair above its face down to the felt, as if the card were set
//! in a frame. It is the first thing this client draws that stands *higher*
//! than the print it surrounds, so it brings with it the two rules every
//! later shell obeys.
//!
//! **Nothing of the rim or the wall is drawn over its own print.** The
//! vertex stage hands the fragment the camera in the shell's own space. The
//! fragment follows the ray from the camera through its point to the card's
//! face plane, and where that ray meets the print its alpha is exactly zero
//! ([`clear_over_print`], which `shell.wgsl` mirrors). It is the real camera
//! and the real ray, so the rule holds at every yaw, lean, screen corner,
//! hover and tap. A dome is the one exception: it is glass over the whole
//! card, the owner's choice (25.09).
//!
//! **Nothing of a shell lies on another card's print.** The mask cannot see a
//! neighbour, so this rule is geometry. [`shell_stands`] follows, from where
//! every card is this frame and where the camera is, where each part of the
//! shell above another card's face lands on that face: further along the
//! camera's ray, away from the camera. A rim that would land on a print
//! lies down on the felt instead, as a steel ring from [`RING_INNER`] to
//! [`RING_OUTER`] out, under every card ([`RING_RUNG`]).
//!
//! The rim can stand in a lane at all because it falls to the felt. A card
//! beside it at the same height hides everything of it below its own face, so
//! what can reach that card is only what stands above it: the rim's top edge,
//! [`RIM_RISE`] over the face, thrown [`RIM_RISE`] × the ray's tangent
//! past the edge. That is about 0.013 of a card width. A rim level to its
//! own foot, as first planned, reached 0.055, and two untapped cards in
//! neighbouring rows of a ring are 0.0185 apart.
//!
//! The second is a dome of glass for hexproof (blue) or shroud (violet,
//! which swallows hexproof): tall over the card, nearly clear at its crown
//! and deepening to its colour at its foot, its crown [`DOME_HEIGHT`]
//! over the face and its foot on the felt [`DOME_MARGIN`] past the card's
//! edge, a quarter ellipse across the whole card between them. A flat top on
//! a steep skirt, as first built, read from above as a frame: a wall seen
//! edge-on glows all the way round. It may lie over its own print, name and
//! artist line included (the owner, 25.09). It is the same guard over a taller
//! profile ([`dome_profile`]), tried at each of [`DOME_STEPS`] in turn: a
//! dome with too little air stands lower, then narrower, its foot drawn in
//! onto its own card, and one with none lies down as a ring in its colour,
//! a band of plates of its own that the offer's soft light on the felt is
//! never taken for. Where the steel and the dome both lie, they share the
//! ring's band inside out, as they stand: steel [`RING_INNER`]..[`RING_SPLIT`],
//! the dome [`RING_SPLIT`]..[`RING_OUTER`] (the PM, 25.09). Which dome is a
//! row ([`Dome::row`]), so ward and protection (#302) are rows too.
//!
//! The third is defender's brick wall: a low rampart standing on the felt
//! past the card's top edge, towards the table's middle and the enemy,
//! slightly curved like a shield. It needs no guard, because it is never
//! taller than any card: at most [`WALL_HEIGHT`] (0.08) off the felt, under
//! the face of every card lying there ([`RIM_DROP`], 0.083), so wherever it
//! meets a print on screen that print is in front of it and hides it. It
//! stands on the felt whatever its card does: a hover or a flier's lift
//! would carry it over its neighbours' faces. Its courses' face towards the
//! card leans back ([`WALL_BATTER`]), because a duel's camera, nearly over
//! it, sees little of a face that stands upright.
//!
//! A standing dome and the wall cast soft shadows on the felt
//! ([`dome_shade_mesh`], [`wall_shade_mesh`]), at [`SHADE_RUNG`], under
//! every card's face for the wall's reason; a dome's, lifted with its card,
//! is faded out before it could rise to one.
//!
//! The fourth is summoning sickness (CR 302.6): a slow wave of moonlight
//! running out over the card from its middle, a real ripple a hair over its
//! face, resting before the next ([`WAVE_PERIOD`]). It lies over its own
//! print, as the arrival sweep does, and leaves nothing behind (the owner,
//! 25.09), so it is the other shell the mask exempts. It keeps the other
//! rule: it is the same guard over a flat profile ([`WAVE_PROFILE`]), and
//! where it would land on another card's print, under the next card of a
//! fanned row above all, it is not drawn; the plate such a card shows
//! writes in moon-grey. With motion off it holds still, crest and all.

use baylee_client_core::airborne;
use baylee_client_core::board::KeywordBadge;
use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH};
use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::table::{CARD_CORNER, CARD_LIFT, CARD_THICKNESS, FLOOR_RUNG, STRIP_RUNG};

/// How far past the card's edge the rim's foot stands, in card widths.
pub const RIM_MARGIN: f32 = 0.06;
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
/// How high a dome's crown stands over the card's face at full height:
/// a real dome, a third of a card's width (the owner, 25.09).
pub const DOME_HEIGHT: f32 = 0.32;
/// How far in from the card's edge a dome's crown stands: a ridge down the
/// card's middle, as near to it as a corner of [`CAP_ROUND`] lets a ring go.
pub const DOME_CROWN: f32 = CARD_WIDTH / 2.0 - CAP_ROUND;
/// The shapes a dome may stand in, in the order they are tried: full, then
/// lower, then narrower, its foot drawn in from the row's margin onto its
/// own card. Past the last it lies down as a ring.
pub const DOME_STEPS: [DomeStep; 5] = [
    DomeStep {
        share: 1.0,
        inward: 0.0,
    },
    DomeStep {
        share: 0.7,
        inward: 0.0,
    },
    DomeStep {
        share: 0.6,
        inward: DOME_MARGIN,
    },
    DomeStep {
        share: 0.45,
        inward: DOME_MARGIN + 0.1,
    },
    DomeStep {
        share: 0.3,
        inward: DOME_MARGIN + 0.2,
    },
];

/// One shape a dome may stand in: a share of its row's full height, and how
/// far its foot is drawn in from the row's margin. A foot drawn in to the
/// card's edge or past it stands on the card's face rather than the felt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DomeStep {
    /// The share of the row's height.
    pub share: f32,
    /// How far in from the row's margin the foot stands.
    pub inward: f32,
}
/// How many rings a dome's profile is drawn and measured with, from its
/// crown down to its foot.
pub const DOME_RINGS: usize = 10;
/// Where a dome sits in the transparent pass: last of all, over the strip
/// and the count badge, since it is glass over the whole card.
pub const DOME_RUNG: f32 = STRIP_RUNG + 0.0005;
/// How far out past a standing dome's foot its shadow on the felt reaches.
pub const DOME_SHADE: f32 = 0.12;
/// How dark a shadow on the felt is where it is cast: a dome's at its foot,
/// the wall's behind it and at its own foot.
pub const DOME_SHADE_DEPTH: f32 = 0.6;
/// See [`DOME_SHADE_DEPTH`].
pub const WALL_SHADE_DEPTH: f32 = 0.85;
/// See [`DOME_SHADE_DEPTH`].
pub const WALL_FOOT_DEPTH: f32 = 0.5;
/// Where a shell's shadow lies on the felt, and in the pass: over every
/// card's contact shadow, under the offer's light and the rings.
pub const SHADE_RUNG: f32 = CARD_LIFT * 0.6;
/// How high defender's wall stands off the felt: two courses of brick and
/// a row of merlons, and never as high as a card's face (the PM, 25.09).
pub const WALL_HEIGHT: f32 = 0.08;
/// One course of brick, and one brick along it.
pub const WALL_COURSE: f32 = 0.028;
/// See [`WALL_COURSE`].
pub const WALL_BRICK: f32 = 0.07;
/// How far past the card's top edge the foot of the wall's inner face
/// stands at its ends; its middle bulges [`WALL_BULGE`] further out. Far enough that its
/// own card hides little of it (the wall sweep in `table::shell_tests`
/// counts it: one point in a hundred, from every shot).
pub const WALL_NEAR: f32 = 0.12;
/// How far the wall's middle bows out past its ends, like a shield.
pub const WALL_BULGE: f32 = 0.04;
/// How thick the wall is at its top.
pub const WALL_THICK: f32 = 0.04;
/// How far the wall's inner face leans back from its foot to the top of its
/// courses: a battered rampart, so that a duel's camera, nearly over it,
/// sees its courses and not only its top.
pub const WALL_BATTER: f32 = 0.06;
/// How far behind the wall its shadow on the felt reaches, and in front of
/// it the shade at its foot.
pub const WALL_SHADE: f32 = 0.12;
/// See [`WALL_SHADE`].
pub const WALL_FOOT_SHADE: f32 = 0.04;
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
const _: () = assert!(DOME_RUNG > STRIP_RUNG);
// The narrowest dome still curves: its foot stands well outside its crown.
const _: () = assert!(DOME_MARGIN - DOME_STEPS[4].inward + DOME_CROWN >= 0.2);
// Under every card's face, which is the whole of the wall's legality.
const _: () = assert!(WALL_HEIGHT <= 0.08 && WALL_HEIGHT < RIM_DROP);
const _: () = assert!(2.0 * WALL_COURSE < WALL_HEIGHT);
// Outside the dome's skirt.
const _: () = assert!(WALL_NEAR > DOME_MARGIN);
// Over a card's own contact shadow, under the offer's light.
const _: () = assert!(SHADE_RUNG > CARD_LIFT * 0.5 && SHADE_RUNG < FLOOR_RUNG);

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
/// one wave to the next, in seconds: slow, and resting between.
pub const WAVE_TRAVEL: f32 = 3.6;
/// See [`WAVE_TRAVEL`].
pub const WAVE_PERIOD: f32 = 6.0;
/// The wave's sheet's cells, across and down the card.
pub const WAVE_CELLS: (u32, u32) = (18, 26);
/// Where the wave sits in the transparent pass: over the rings, under the
/// rim, the strip, whose label lies over it, and a dome.
pub const WAVE_RUNG: f32 = RIM_RUNG - 0.0005;
/// The wave as the guard measures it ([`shell_stands`]): its sheet with the
/// crest standing everywhere at once, from the card's middle line out to
/// [`WAVE_INSET`] inside its edge.
pub const WAVE_PROFILE: [(f32, f32); 2] = [(-CARD_WIDTH / 2.0, WAVE_TOP), (-WAVE_INSET, WAVE_TOP)];

const _: () = assert!(WAVE_INSET > CARD_CORNER);
const _: () = assert!(WAVE_RUNG > RING_RUNG && WAVE_RUNG < RIM_RUNG);
const _: () = assert!(WAVE_TRAVEL < WAVE_PERIOD);

/// Which shell a material draws, as `shell_common.wgsl` numbers it for the
/// table's shader and the preview's.
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
    /// A standing dome's or the wall's shadow on the felt.
    Shade = 6,
    /// Summoning sickness's wave of moonlight over the card.
    Wave = 7,
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
    /// How high its crown stands over the card's face, at full height.
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

/// The shells a permanent wears, from its keywords: the one door the
/// table's rows and the preview both ask ([`crate::shellui`]), so the two
/// never disagree about what a card is protected by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shells {
    /// Indestructible's darksteel rim.
    pub steel: bool,
    /// Hexproof's or shroud's dome, the stronger one.
    pub dome: Option<Dome>,
    /// Defender's wall.
    pub wall: bool,
}

impl Shells {
    /// What a permanent with these keywords wears.
    #[must_use]
    pub fn of(badges: &[KeywordBadge]) -> Self {
        let has = |badge| badges.contains(&badge);
        Self {
            steel: has(KeywordBadge::Indestructible),
            dome: Dome::of(has(KeywordBadge::Hexproof), has(KeywordBadge::Shroud)),
            wall: has(KeywordBadge::Defender),
        }
    }

    /// What the preview draws, standing, as two runs in the order each is
    /// painted: those under the card's own objects (the strip, the plate and
    /// the count badge), which lie under them on the table too, and those
    /// over them, a dome being glass over the whole card ([`DOME_RUNG`]).
    /// A new shell is a look here and an arm in `shell_ui.wgsl`.
    #[must_use]
    pub fn standing(self) -> [Vec<ShellLook>; 2] {
        let under = [
            self.wall.then_some(ShellLook::steel(ShellKind::Wall)),
            self.steel.then_some(ShellLook::steel(ShellKind::Rim)),
        ];
        let over = [self.dome.map(|dome| ShellLook {
            kind: ShellKind::Dome,
            dome: Some(dome),
            band: Band::Whole,
        })];
        [
            under.into_iter().flatten().collect(),
            over.into_iter().flatten().collect(),
        ]
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
    /// Where a ring's band starts and ends past the card's edge.
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
        let [(inner, _), (outer, _)] = look.band.edges();
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
            6 => ShellKind::Shade,
            7 => ShellKind::Wave,
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

    /// See [`RIM_RUNG`], [`RING_RUNG`], [`DOME_RUNG`], [`WALL_RUNG`],
    /// [`SHADE_RUNG`] and [`WAVE_RUNG`].
    fn depth_bias(&self) -> f32 {
        crate::table::sort_bias(match self.kind() {
            ShellKind::Rim => RIM_RUNG,
            ShellKind::Ring | ShellKind::DomeRing => RING_RUNG,
            ShellKind::Dome => DOME_RUNG,
            ShellKind::Wall => WALL_RUNG,
            ShellKind::Shade => SHADE_RUNG,
            ShellKind::Wave => WAVE_RUNG,
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

/// The rim's two edges: its top at the card's edge, [`RIM_RISE`] over the
/// face, and its foot [`RIM_MARGIN`] out and [`RIM_DROP`] down.
pub const RIM_EDGES: [(f32, f32); 2] = [(0.0, RIM_RISE), (RIM_MARGIN, -RIM_DROP)];
/// The ring's two edges, flat.
pub const RING_EDGES: [(f32, f32); 2] = [(RING_INNER, 0.0), (RING_OUTER, 0.0)];

/// A dome's profile standing at `step`: a quarter ellipse from its crown,
/// [`DOME_CROWN`] in from the card's edge, down to its foot, on the felt
/// past the card's edge or on the card's face where the step draws it in
/// that far, as `(distance past the card's edge, height over its face)`.
#[must_use]
pub fn dome_profile(row: DomeRow, step: DomeStep) -> [(f32, f32); DOME_RINGS] {
    let (margin, base, run, rise) = dome_ellipse(row, step);
    std::array::from_fn(|k| {
        let t = ring_angle(k);
        (margin - run + run * t.sin(), base + rise * t.cos())
    })
}

/// A dome's quarter ellipse at `step`: where its foot stands past the card's
/// edge and over its face, and its run and rise from there to its crown.
fn dome_ellipse(row: DomeRow, step: DomeStep) -> (f32, f32, f32, f32) {
    let margin = row.margin - step.inward;
    let base = if margin > 0.0 { -RIM_DROP } else { 0.0 };
    (
        margin,
        base,
        margin + DOME_CROWN,
        row.height * step.share - base,
    )
}

/// How far round its quarter ellipse a dome's `k`-th ring stands: none at
/// the crown, a right angle at the foot.
fn ring_angle(k: usize) -> f32 {
    #[expect(clippy::cast_precision_loss)] // ten rings
    let share = k as f32 / (DOME_RINGS - 1) as f32;
    share * std::f32::consts::FRAC_PI_2
}

/// The point `d` past the card's edge in the direction `angle` (degrees) out
/// of the corner `centre` stands on, and that direction.
///
/// Inside the card's edge a dome's ring rounds its corners more the further
/// in it stands, [`DOME_ROUNDING`] for each unit, until its ends are half
/// circles, as a dome's are: its crown is a ridge with round ends,
/// [`CAP_ROUND`] across. A rounder corner lies inside the true inset, so no
/// point of a dome is further past the edge than its profile says.
fn outline(centre: Vec2, angle: f32, d: f32) -> (Vec2, Vec2) {
    let round = (CARD_CORNER + d).max((CARD_CORNER - DOME_ROUNDING * d).min(CARD_WIDTH / 2.0 + d));
    let pull = CARD_CORNER + d - round;
    let dir = Vec2::from_angle(angle.to_radians());
    (centre + centre.signum() * pull + dir * round, dir)
}

/// The smallest corner a ring of a dome keeps: its crown's.
pub(crate) const CAP_ROUND: f32 = 0.03;
/// How much rounder a dome's ring's corners are for each unit it stands
/// inside the card's edge.
pub(crate) const DOME_ROUNDING: f32 = 1.5;

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
/// profile, joined, and its crown closed by a fan. Its `uv.x` is how far out
/// from its crown towards its foot a point is, seen from above (0 at the
/// crown, 1 at the foot), which is where `shell.wgsl` lays the glass of its
/// foot; nothing reads a dome's card UV.
///
/// # Panics
///
/// Never: a dome has a few hundred vertices.
#[must_use]
pub fn dome_mesh(row: DomeRow, step: DomeStep) -> Mesh {
    let profile = dome_profile(row, step);
    let (_, _, run, rise) = dome_ellipse(row, step);
    let mut at = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for (k, &(d, z)) in profile.iter().enumerate() {
        // The ellipse's own normal: straight up at the crown, straight out
        // at the foot.
        let t = ring_angle(k);
        let (out, up) = (rise * t.sin(), run * t.cos());
        for (centre, start) in corners() {
            for i in 0..=BAND_SEGMENTS {
                #[expect(clippy::cast_precision_loss)] // a handful of segments
                let angle = start + 90.0 * i as f32 / BAND_SEGMENTS as f32;
                let (p, dir) = outline(centre, angle, d);
                at.push(p.extend(z).to_array());
                normals.push(
                    (dir.extend(0.0) * out + Vec3::Z * up)
                        .normalize_or(Vec3::Z)
                        .to_array(),
                );
                uvs.push([t.sin(), 0.0]);
            }
        }
    }
    let around = u32::try_from(4 * (BAND_SEGMENTS + 1)).expect("a few dozen");
    let mut indices = Vec::new();
    for ring in 0..u32::try_from(DOME_RINGS - 1).expect("ten rings") {
        for i in 0..around {
            let next = (i + 1) % around;
            let (inner, outer) = (ring * around + i, (ring + 1) * around + i);
            let (inner_next, outer_next) = (ring * around + next, (ring + 1) * around + next);
            indices.extend_from_slice(&[inner, outer, outer_next, inner, outer_next, inner_next]);
        }
    }
    // The crown: a fan from the middle of the card over the first ring, a
    // ridge twice [`CAP_ROUND`] wide.
    let middle = u32::try_from(at.len()).expect("a few hundred");
    at.push([0.0, 0.0, profile[0].1]);
    normals.push([0.0, 0.0, 1.0]);
    uvs.push([0.0, 0.0]);
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

/// The shadow a dome standing at `step` casts on the felt round its foot,
/// [`DOME_SHADE`] out from it: none where its foot stands on its own card's
/// face. In the dome's own space, [`SHADE_RUNG`] over the felt.
#[must_use]
pub fn dome_shade_mesh(row: DomeRow, step: DomeStep) -> Option<Mesh> {
    let (margin, base, _, _) = dome_ellipse(row, step);
    if margin <= 0.0 {
        return None;
    }
    let z = base + SHADE_RUNG;
    let pairs = band((margin, z), (margin + DOME_SHADE, z))
        .chunks(2)
        .map(|pair| {
            (
                pair[0].at.truncate(),
                pair[1].at.truncate(),
                DOME_SHADE_DEPTH,
            )
        })
        .collect();
    Some(shade_mesh(&[pairs], z, true))
}

/// Defender's wall's shadow on the felt, in the wall's own space
/// ([`wall_mesh`]): behind it, [`WALL_SHADE`] deep, where the light over the
/// player's shoulder throws it, and in front of it the shade at its foot,
/// [`WALL_FOOT_SHADE`] deep; both fading out at the wall's two ends.
#[must_use]
pub fn wall_shade_mesh() -> Mesh {
    const PIECES: usize = 24;
    let half = WALL_THICK / 2.0;
    let arc: Vec<(Vec2, Vec2, f32)> = (0..=PIECES)
        .map(|i| {
            #[expect(clippy::cast_precision_loss)] // a few dozen pieces
            let u = i as f32 / PIECES as f32;
            let (centre, out, _) = wall_arc(u);
            let end = if i == 0 || i == PIECES { 0.0 } else { 1.0 };
            (centre, out, end)
        })
        .collect();
    let behind = arc
        .iter()
        .map(|&(centre, out, end)| {
            let foot = centre + out * half;
            (foot, foot + out * WALL_SHADE, WALL_SHADE_DEPTH * end)
        })
        .collect();
    let front = arc
        .iter()
        .map(|&(centre, out, end)| {
            let foot = centre - out * (half + WALL_BATTER);
            (foot, foot - out * WALL_FOOT_SHADE, WALL_FOOT_DEPTH * end)
        })
        .collect();
    shade_mesh(&[behind, front], SHADE_RUNG - RIM_DROP, false)
}

/// A shadow on the felt as a mesh, `z` off the face: strips of `(where it
/// is cast, where it is gone, how dark it is cast there)`, each pair joined
/// to the next and, `closed`, the last to the first. `uv.x` is how far
/// across its strip a point is, `uv.y` how dark; every triangle faces up.
///
/// # Panics
///
/// Never: a shadow has a few hundred vertices.
fn shade_mesh(strips: &[Vec<(Vec2, Vec2, f32)>], z: f32, closed: bool) -> Mesh {
    let mut at: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for strip in strips {
        let first = u32::try_from(at.len()).expect("a few hundred");
        let pairs = u32::try_from(strip.len()).expect("a few dozen");
        for &(cast, gone, depth) in strip {
            at.push(cast.extend(z).to_array());
            uvs.push([0.0, depth]);
            at.push(gone.extend(z).to_array());
            uvs.push([1.0, depth]);
        }
        let joins = if closed { pairs } else { pairs - 1 };
        for pair in 0..joins {
            let next = (pair + 1) % pairs;
            let quad = [
                first + 2 * pair,
                first + 2 * pair + 1,
                first + 2 * next + 1,
                first + 2 * next,
            ];
            let [a, b, c, _] = quad.map(|i| Vec3::from_array(at[i as usize]));
            let order = if (b - a).cross(c - a).z >= 0.0 {
                [0, 1, 2, 0, 2, 3]
            } else {
                [0, 2, 1, 0, 3, 2]
            };
            indices.extend(order.map(|k| quad[k]));
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

/// Defender's wall as a mesh, in the card's own space with its face at
/// `z = 0` and the felt [`RIM_DROP`] under it: two courses of brick along a
/// shallow arc past the card's top edge, their face towards the card
/// leaning back [`WALL_BATTER`], and [`WALL_MERLONS`] merlons on them.
/// Every face faces out of the solid, and `uv.x` is the distance along the
/// arc, which the bricks are laid by.
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
    // A length of wall from `from` to `to` along it, `low` to `high` off
    // the face, its face towards the card's foot `batter` further in than
    // its top.
    let mut slab = |from: f32, to: f32, low: f32, high: f32, batter: f32, pieces: usize| {
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
            let (f0, f1) = (i0 - n0 * batter, i1 - n1 * batter);
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
                    (f0.extend(low), s0),
                    (f1.extend(low), s1),
                    (i1.extend(high), s1),
                    (i0.extend(high), s0),
                ],
                (Vec3::Z * batter - n * (high - low)).normalize(),
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
                    ((c - n * (half + batter)).extend(low), s),
                    ((c + n * half).extend(low), s),
                    ((c + n * half).extend(high), s),
                    ((c - n * half).extend(high), s),
                ],
                along.extend(0.0),
            );
        }
    };
    slab(0.0, 1.0, foot, crenels, WALL_BATTER, 24);
    let parts = 2 * WALL_MERLONS - 1;
    for k in 0..WALL_MERLONS {
        #[expect(clippy::cast_precision_loss)] // five merlons
        let (from, to) = (
            (2 * k) as f32 / parts as f32,
            (2 * k + 1) as f32 / parts as f32,
        );
        slab(from, to, crenels, top, 0.0, 3);
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

/// The middle line of the wall's top at `u` along it (0 at its left end, 1
/// at its right, seen from the card): where it is, which way is out
/// (towards the table's middle), and how far along the arc that is.
#[must_use]
pub fn wall_arc(u: f32) -> (Vec2, Vec2, f32) {
    let chord = CARD_WIDTH + 2.0 * WALL_OVERHANG;
    let x = (u - 0.5) * chord;
    let across = 2.0 * x / chord;
    let near = CARD_HEIGHT / 2.0 + WALL_NEAR + WALL_BATTER + WALL_THICK / 2.0;
    let y = near + WALL_BULGE * (1.0 - across * across);
    let slope = -8.0 * WALL_BULGE * x / (chord * chord);
    let out = Vec2::new(-slope, 1.0).normalize();
    // The arc is shallow enough that its length is its chord's to within a
    // brick's thousandth; bricks are laid by it, not measured against it.
    (Vec2::new(x, y), out, x + chord / 2.0)
}

/// Which of [`DOME_STEPS`] a dome of `row` stands at round the card at `me`,
/// or `None` for its ring on the felt: the first that lands on no other
/// card's print ([`shell_stands`]).
///
/// `standing` is the step it stands at now. A step before that, or any step
/// at all for a dome lying down, must fit [`STAND_AGAIN`] higher than it is,
/// for the reason a ring stands up again only with that headroom.
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

/// Whether a rim can stand round the card at `me` without landing on the
/// print of any card in `others`, seen from a camera at `eye`, if it stood
/// `headroom` higher than it does: [`shell_stands`] over the rim's profile.
#[must_use]
pub fn rim_stands(
    me: &Footprint,
    others: impl IntoIterator<Item = Footprint>,
    eye: Vec3,
    headroom: f32,
) -> bool {
    shell_stands(&RIM_EDGES, me, others, eye, headroom)
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
    /// What it shares with the preview's shells: the card, the kinds, and
    /// how steel, glass and brick are lit.
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
                "tint: vec4<f32>,",
                "kind: u32,",
                "motion: f32,",
                "inner: f32,",
                "outer: f32,"
            ],
            "shell.wgsl lays the uniform out differently from `ShellParams`"
        );
    }

    /// Every number the shaders and this file share is the same number in
    /// both: a mask cut to a different card than the rim is built round is a
    /// mask with a gap in it. The card and the kinds are `shell_common.wgsl`'s,
    /// which the table's shells and the preview's both draw with.
    #[test]
    fn the_shader_measures_the_card_this_file_does() {
        for (source, name, ours) in [
            (COMMON, "CARD_HALF_W", CARD_WIDTH / 2.0),
            (COMMON, "CARD_HALF_H", CARD_HEIGHT / 2.0),
            (COMMON, "CARD_ROUND", CARD_CORNER),
            (COMMON, "MASK_FEATHER", MASK_FEATHER),
            (COMMON, "RIM_RISE", RIM_RISE),
            (COMMON, "RIM_DROP", RIM_DROP),
            (COMMON, "WALL_COURSE", WALL_COURSE),
            (COMMON, "WALL_BRICK", WALL_BRICK),
            (COMMON, "WALL_HEIGHT", WALL_HEIGHT),
            (SHADER, "SHADE_FLOOR", crate::table::TABLE_Y + SHADE_RUNG),
            (SHADER, "WAVE_LIFT", WAVE_LIFT),
            (SHADER, "WAVE_CREST", WAVE_CREST),
            (SHADER, "WAVE_INSET", WAVE_INSET),
            (SHADER, "WAVE_TRAVEL", WAVE_TRAVEL),
            (SHADER, "WAVE_PERIOD", WAVE_PERIOD),
        ] {
            let theirs = wgsl_const(source, name);
            assert!(
                (theirs - ours).abs() < 1e-4,
                "{name}: {ours} here, {theirs} in the shader"
            );
        }
        for (name, kind) in [
            ("SHELL_RIM", ShellKind::Rim),
            ("SHELL_RING", ShellKind::Ring),
            ("SHELL_DOME", ShellKind::Dome),
            ("SHELL_DOME_RING", ShellKind::DomeRing),
            ("SHELL_WALL", ShellKind::Wall),
            ("SHELL_SHADE", ShellKind::Shade),
            ("SHELL_WAVE", ShellKind::Wave),
        ] {
            #[expect(clippy::cast_precision_loss)] // a small enum
            let ours = kind as u32 as f32;
            assert!(
                (wgsl_const(COMMON, name) - ours).abs() < f32::EPSILON,
                "{name}"
            );
        }
    }

    /// Every colour the fragment stage returns but the dome's and the wave's
    /// has the mask as the last factor of its alpha: a `return` that forgot
    /// it would be a rim or a ring drawn over its own print from whichever
    /// side the camera happened to be on.
    #[test]
    fn every_colour_but_the_domes_and_the_waves_carries_the_mask() {
        let open = SHADER.find("fn fragment(").expect("the fragment stage");
        let body = &SHADER[open..];
        // The two branches the owner let lie over their own print (25.09),
        // the dome's glass and the wave's passing light: each returns once,
        // and nothing else goes unmasked.
        let mut exempt = Vec::new();
        for (branch, colour) in [
            (
                "if params.kind == SHELL_DOME {",
                "return vec4<f32>(colour, glass);",
            ),
            (
                "if params.kind == SHELL_WAVE {",
                "return vec4<f32>(MOONLIGHT, glow);",
            ),
        ] {
            let start = body.find(branch).expect("the branch");
            let end = start + body[start..].find("\n    }").expect("it closes");
            let own = &body[start..end];
            assert_eq!(own.matches("return").count(), 1, "`{branch}` returns once");
            assert!(own.contains(colour), "`{branch}` returns `{colour}`");
            exempt.push(start..end);
        }
        let mut at = 0;
        let mut returns = Vec::new();
        for line in body.split_inclusive('\n') {
            let here = at;
            at += line.len();
            if !exempt.iter().any(|range| range.contains(&here))
                && line.trim().starts_with("return")
            {
                returns.push(line.trim());
            }
        }
        assert!(returns.len() >= 4, "the fragment stage returns too little");
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

    /// Defender's wall stands where its constants say: on the felt, no
    /// higher than [`WALL_HEIGHT`], its foot [`WALL_NEAR`] past the card's
    /// top edge or further, its face towards the card leaning back
    /// [`WALL_BATTER`] to the top of its courses. And every face of it faces out of the solid, so the back
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
            let across = (p.truncate() - centre).dot(out);
            let lean = WALL_BATTER * ((crenels - p.z) / (crenels + RIM_DROP)).max(0.0);
            if across > WALL_THICK / 2.0 || across < -WALL_THICK / 2.0 - lean {
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

    /// Every point of a dome's mesh is where its profile says or further in:
    /// no ring stands further past the card's edge than its point of the
    /// profile, which is the distance the guard measures its throw from, and
    /// each stands at that point's height.
    #[test]
    fn a_domes_mesh_stands_no_further_out_than_its_profile() {
        let row = Dome::Hexproof.row();
        for step in DOME_STEPS {
            let profile = dome_profile(row, step);
            assert!(
                (profile[0].1 - row.height * step.share).abs() < 1e-6,
                "crown"
            );
            let (foot, base) = profile[DOME_RINGS - 1];
            assert!((foot - (row.margin - step.inward)).abs() < 1e-6, "skirt");
            let wanted = if foot > 0.0 { -RIM_DROP } else { 0.0 };
            assert!(
                (base - wanted).abs() < 1e-6,
                "on the felt past the edge, on the face within it"
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
            for (i, p) in at.iter().enumerate().take(DOME_RINGS * around) {
                let d = profile[i / around].0;
                if d >= 0.0 {
                    assert!((card_sdf(Vec2::new(p[0], p[1])) - d).abs() < 1e-4);
                }
            }
        }
    }

    /// A shadow lies flat on the felt, facing up, and only outside its own
    /// card: a dome's from its foot [`DOME_SHADE`] out, and only at the
    /// steps whose foot stands on the felt; the wall's behind it and before
    /// its foot. Lying there it is under every card's face, and one lifted
    /// with its card is gone before it rises to one: the shader fades it out
    /// by `SHADE_GONE` over [`SHADE_RUNG`].
    #[test]
    #[allow(clippy::float_cmp)] // exactly one edge or the other is the claim
    fn a_shadow_lies_on_the_felt_outside_its_card() {
        let felt = SHADE_RUNG - RIM_DROP;
        let lies = |mesh: &Mesh, what: &str| -> Vec<Vec3> {
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(at)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("{what} has positions");
            };
            let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
                mesh.attribute(Mesh::ATTRIBUTE_UV_0)
            else {
                panic!("{what} has uvs");
            };
            let Some(Indices::U32(indices)) = mesh.indices() else {
                panic!("{what} is indexed");
            };
            let at: Vec<Vec3> = at.iter().map(|p| Vec3::from_array(*p)).collect();
            for p in &at {
                assert!((p.z - felt).abs() < 1e-6, "{what} at {p}, off the felt");
            }
            for uv in uvs {
                assert!(uv[0] == 0.0 || uv[0] == 1.0, "{what}: across {uv:?}");
                assert!((0.0..=1.0).contains(&uv[1]), "{what}: depth {uv:?}");
            }
            for tri in indices.chunks(3) {
                let [a, b, c] = [0, 1, 2].map(|k| at[tri[k] as usize]);
                assert!((b - a).cross(c - a).z > 0.0, "{what} faces down at {a}");
            }
            at
        };
        let row = Dome::Hexproof.row();
        let mut shaded = 0;
        for step in DOME_STEPS {
            let margin = row.margin - step.inward;
            let Some(mesh) = dome_shade_mesh(row, step) else {
                assert!(margin <= 0.0, "a dome on the felt casts no shadow");
                continue;
            };
            shaded += 1;
            assert!(margin > 0.0, "a dome on its card's face casts one");
            for p in lies(&mesh, "a dome's shadow") {
                let past = card_sdf(p.truncate());
                assert!(
                    past > margin - 1e-4 && past < margin + DOME_SHADE + 1e-4,
                    "a dome's shadow {past} past its card"
                );
            }
        }
        assert_eq!(shaded, 2, "full height and lower stand on the felt");
        for p in lies(&wall_shade_mesh(), "the wall's shadow") {
            assert!(
                p.y > CARD_HEIGHT / 2.0 + WALL_NEAR - WALL_FOOT_SHADE - 1e-4,
                "the wall's shadow comes to {p}"
            );
        }
        let gone = wgsl_const(SHADER, "SHADE_FLOOR") + wgsl_const(SHADER, "SHADE_GONE");
        assert!(
            gone < RIM_DROP,
            "a lifted shadow is still drawn at {gone}, over a card's face at {RIM_DROP}"
        );
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

    /// A dome stands as tall and as wide as its air allows, and what it
    /// throws goes to the far side of it from the camera: at full height
    /// alone and with a card at the edge of its foot on the camera's side,
    /// not with one as close on the far side; narrower with a card a ring's gap
    /// beside it, under its skirt; lying down only under a card that
    /// overlaps it. And a dome lying down, or at a later step, comes back
    /// only with a flier's bob of headroom.
    #[test]
    fn a_dome_stands_as_tall_as_its_air_allows() {
        let row = Dome::Shroud.row();
        let eye = Vec3::new(0.0, 20.0, 8.0);
        let me = flat(0.0, 0.0, 0.083);
        let step = |standing, others: &[Footprint]| {
            dome_step(row, standing, &me, others.iter().copied(), eye)
        };
        // At the edge of its foot, which a crowned dome throws no further
        // than on the far side.
        let past_skirt = CARD_HEIGHT + 0.09;
        assert_eq!(step(Some(0), &[]), Some(0), "alone, at full height");
        assert_eq!(step(None, &[]), Some(0), "alone, a lying dome stands up");
        let near = [flat(0.0, past_skirt, 0.083)];
        assert_eq!(
            step(Some(0), &near),
            Some(0),
            "past its skirt, nearer the camera"
        );
        let far = [flat(0.0, -past_skirt, 0.083)];
        assert!(
            step(Some(0), &far).is_some_and(|s| s > 0),
            "past its skirt, beyond it: {:?}",
            step(Some(0), &far)
        );
        let beside = [flat(1.0185, 0.0, 0.083)];
        assert!(
            step(Some(0), &beside).is_some_and(|s| DOME_STEPS[s].inward > 0.0),
            "a ring's gap beside it, narrower: {:?}",
            step(Some(0), &beside)
        );
        let fanned = [flat(0.35, 0.0, 0.084)];
        assert_eq!(step(Some(0), &fanned), None, "under the next card of a fan");
        // Hysteresis: from lying down, or from a later step, it needs more
        // air than to stay.
        let at_edge = (0..2500)
            .map(|i| 0.6 + 0.0002 * i as f32)
            .find(|&x| step(Some(DOME_STEPS.len() - 1), &[flat(x, 0.0, 0.084)]).is_some())
            .expect("the last step fits somewhere");
        assert!(
            step(None, &[flat(at_edge, 0.0, 0.084)]).is_none(),
            "a ring at the edge of fitting stood up without headroom"
        );
    }
}
