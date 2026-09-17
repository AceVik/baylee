//! A seat's mat: the rounded ground its permanents are played on, drawn
//! rather than stretched.
//!
//! The same argument as [`crate::feltmat`], one step smaller. A mat was a
//! 512 × 256 image laid over a board thirteen units wide, which at this
//! camera is about four texels to the physical pixel — so its rim, its lane
//! seams and its corners were all soft, and the owner's word for the whole
//! table was "unsharp". Arithmetic has no resolution: a signed distance and
//! `fwidth` give an edge one pixel wide from any distance, and the corner
//! stops being a fraction of an image and becomes a length in table units,
//! which is the only form in which "less border radius" is a thing anyone can
//! ask for or check.
//!
//! It also buys the thing a texture could not have: the mat of the seat whose
//! turn it is carries a light running round its rim. That is one uniform, not
//! a re-generated image per frame.
//!
//! The **glow** underneath stays a `StandardMaterial` on
//! [`tabletop::glow`](baylee_client_core::tabletop::glow). It is a soft round
//! falloff with no edge in it at all, which is exactly the case a stretched
//! image is good at and a distance field would only make more expensive.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// Everything the mat shader reads.
#[derive(Clone, Copy, ShaderType, Debug, PartialEq)]
pub struct MatParams {
    /// The seat's colour in linear RGB, and in `w` how brightly the mat is
    /// drawn at all — `table::zone_brightness` in one number.
    ///
    /// The two used to be a texture and a material tint. They are one vector
    /// now, and the meaning of the brightness moved with them: it scales the
    /// mat's **alpha**, so a seat that has lost fades into the felt instead
    /// of drawing a dark rim over it.
    pub accent: Vec4,
    /// The mat's world size. Every other length here is in the same units,
    /// which is the point of the whole material.
    pub size: Vec2,
    /// The corner radius, in table units:
    /// [`tabletop::MAT_CORNER`](baylee_client_core::tabletop::MAT_CORNER).
    pub corner: f32,
    /// How far in from the edge the coloured rim runs:
    /// [`tabletop::MAT_RIM`](baylee_client_core::tabletop::MAT_RIM).
    pub rim: f32,
    /// 1 while this is the seat whose turn it is, 0 otherwise.
    ///
    /// **The turn, not priority.** `Standing` collapses the two — a seat
    /// holding priority outranks the seat whose turn it is, because the mat's
    /// brightness answers "who is everybody waiting for". This answers the
    /// other question, and on any turn where an opponent responds to
    /// something the two have different answers.
    pub on_turn: f32,
    /// The clock the travelling light runs on: [`MOVING`](crate::cardmat::MOVING)
    /// or [`STILL`](crate::cardmat::STILL).
    pub motion: f32,
    /// 1 when this seat's shelf is on the mat's outer edge, 0 when it is on
    /// the centre-facing one:
    /// [`LEDGE_IS_OUTER`](baylee_client_core::layout::LEDGE_IS_OUTER).
    ///
    /// A flag rather than a flipped uv, because only the shelf changes ends.
    /// The lanes run from the centre-facing edge outwards at every seat —
    /// creatures nearest the middle of the table — since that is where the
    /// cards stand, and cards do not turn round because the ink did.
    pub ledge_outer: f32,
}

/// One seat's mat.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct MatMaterial {
    /// Everything the shader reads. No textures: see the module header.
    #[uniform(0)]
    pub params: MatParams,
}

impl Material for MatMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/mat.wgsl".into()
    }

    /// A mat lies *on* the felt and is mostly a veil over it — the lanes are
    /// alphas in the hundredths — so it blends. The shape is the distance
    /// field's, and the fragments outside it are discarded rather than drawn
    /// transparent, so nothing outside the rounded rectangle reaches the
    /// blend at all.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    /// Where a mat sits in the transparent pass: at its own rung of the
    /// table's ladder, so it is painted before the weather over it and before
    /// the contact shadow of any card lying on it — wherever on the table
    /// that card happens to be, which is the half of it the sort used to get
    /// wrong. See [`table::sort_bias`](crate::table::sort_bias).
    fn depth_bias(&self) -> f32 {
        crate::table::sort_bias(crate::table::ZONE_LIFT)
    }
}

/// Installs the mat material and its shader.
pub struct MatMaterialPlugin;

impl Plugin for MatMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/mat.wgsl");
        app.add_plugins(MaterialPlugin::<MatMaterial>::default());
    }
}
