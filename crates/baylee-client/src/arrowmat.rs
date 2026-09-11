//! The material a combat arrow is drawn with.
//!
//! One quad per arrow, and everything about the arrow — where its ends are,
//! how far it bows, how wide its head is, where the current has got to — is
//! in this uniform. The alternative was a chain of little quads along the
//! curve, which costs a dozen entities per arrow, has no antialiasing worth
//! the name, and would have had to be rebuilt on every frame because both
//! ends are welded to cards that glide.
//!
//! A material per arrow, rather than a handful shared like
//! [`crate::matmat`]'s: an arrow's geometry *is* its uniform, so two arrows
//! are never the same material. They are written in place and only when
//! something about them changed — the current runs off `globals.time` in the
//! shader, so a standing arrow at rest uploads nothing at all.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// Everything the arrow shader reads.
///
/// The field order is the WGSL struct's, member for member, because the two
/// are one buffer.
#[derive(Clone, Copy, ShaderType, Debug, PartialEq)]
pub struct ArrowParams {
    /// The arrow's colour in linear RGB, and in `w` its opacity.
    pub color: Vec4,
    /// The quad's size in table units, which is the space every other length
    /// here is measured in.
    pub box_size: Vec2,
    /// The straight-line distance between the arrow's two ends.
    pub chord: f32,
    /// How far the curve's apex stands off that chord, signed.
    pub bulge: f32,
    /// Half the width of the shaft.
    pub width: f32,
    /// Half the width of the head at its base.
    pub head_width: f32,
    /// How much of the curve the head takes, as a fraction of it.
    pub head_frac: f32,
    /// How many dashes the current is cut into over the whole curve.
    pub dashes: f32,
    /// How many of those dashes pass a fixed point each second.
    pub speed: f32,
    /// The clock: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    pub motion: f32,
}

/// One combat arrow.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ArrowMaterial {
    /// Everything the shader reads. No textures: see the module header.
    #[uniform(0)]
    pub params: ArrowParams,
}

impl Material for ArrowMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/arrow.wgsl".into()
    }

    /// An arrow lies over the felt and is a stroke on a quad that is mostly
    /// empty, so it blends — and the empty part is `discard`ed rather than
    /// drawn transparent, so the blend never sees it.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

/// Installs the arrow material and its shader.
pub struct ArrowMaterialPlugin;

impl Plugin for ArrowMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/arrow.wgsl");
        app.add_plugins(MaterialPlugin::<ArrowMaterial>::default());
    }
}
