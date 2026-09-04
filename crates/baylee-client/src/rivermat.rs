//! The table's surface: timber with a channel of resin poured through it.
//!
//! A material of its own rather than a [`StandardMaterial`] with a texture on
//! it, for one reason: the resin *runs*. Doing that on the CPU would mean
//! regenerating a megapixel of table every frame; doing it in a shader costs
//! a handful of noise samples per pixel of channel.
//!
//! It is also **unlit**, like the cards and unlike a `StandardMaterial`. The
//! stage carries no light at all — scene lighting on card art would make
//! colour identity unreadable, which is the one thing this table may not do —
//! so a lit slab would mean introducing a lamp for the benefit of one object
//! and then keeping every other object out of its way. The shimmer here is
//! painted instead, which is the honest way to do it at a camera this close
//! to overhead: a real specular lobe would track the viewer and drag along
//! behind the cards every time the table was turned.
//!
//! Everything else under the cards stays a `StandardMaterial` — the mats, the
//! medallion, the glow — because none of it moves.

use baylee_client_core::tabletop;
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// Everything the river shader needs that is not a texture.
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct RiverParams {
    /// The phase lamp: `rgb` its colour, `w` how much of it there is.
    ///
    /// Straight out of [`tabletop::phase_light`], eased rather than snapped —
    /// the channel is a large part of the screen and a step boundary that
    /// changed it in one frame would read as a flash.
    pub wash: Vec4,
    /// Where that light enters the channel: `xy` a point on the active seat's
    /// shore in uv, `zw` the direction it travels from there.
    ///
    /// This is what keeps the design honest at more than two seats. A river
    /// has two ends and a ring has none, so "cool at one end, hot at the
    /// other" cannot be a property of the *table*; it is a property of whose
    /// turn it is, and it moves.
    pub source: Vec4,
    /// The slab's world size, which is how the shader turns a point on the
    /// table into a point in the two fields.
    ///
    /// The shader works from the world position and this, rather than from
    /// the quad's own uv: a mesh's uv origin is a convention of whichever
    /// primitive built it, and guessing it wrong mirrors the whole field —
    /// which a duel, symmetric about both axes, would not show.
    pub span: Vec2,
    /// The clock the current runs on: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    ///
    /// The same two values the cards use, and deliberately the same
    /// constants: a table that kept flowing while every card on it held still
    /// would make the setting look broken.
    pub motion: f32,
    /// How hard the wash burns at full energy.
    pub gain: f32,
}

/// How bright the channel burns at the top of combat.
///
/// Above 1.0 deliberately: the tone mapper is off and the camera is HDR, so a
/// value past white blooms. That is safe here in a way it would not be
/// anywhere else on this table — the cards use their own unlit shader and
/// take no light from the scene, so the glow spreads over the felt and stops
/// at the cardboard.
pub const WASH_GAIN: f32 = 1.80;

/// The slab.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct RiverMaterial {
    /// The channel field, from [`tabletop::channel`] — how deep, which way
    /// the current runs, and where the shore is.
    ///
    /// The only texture. The timber used to be a second one and is now drawn
    /// in the shader: the slab is thirty-five units across and a card is about
    /// 114 physical pixels at this camera, so sharp wood would want four
    /// thousand texels, and 2048 already costs 1.6 seconds to generate in a
    /// debug build. The channel stays an image because it is the *layout's*
    /// shape rather than a pattern — evaluating eight rotated boxes per pixel
    /// is work that only changes when somebody sits down.
    #[texture(0)]
    #[sampler(1)]
    pub channel: Handle<Image>,
    /// Everything else.
    #[uniform(2)]
    pub params: RiverParams,
}

impl Material for RiverMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/river.wgsl".into()
    }

    /// The slab is the floor of the scene: opaque, and the only thing under
    /// the cards that is. Everything painted on top of it — the mats, the
    /// medallion, the glows — blends over it, which is what makes them read
    /// as lying on a table rather than as being the table.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

/// The lamp a step calls for, packed for the shader.
#[must_use]
pub fn wash_of(step: baylee_view::Step) -> Vec4 {
    let light = tabletop::phase_light(step);
    Vec4::new(light.rgb[0], light.rgb[1], light.rgb[2], light.energy)
}

/// Installs the river material and its shader.
pub struct RiverMaterialPlugin;

impl Plugin for RiverMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/river.wgsl");
        app.add_plugins(MaterialPlugin::<RiverMaterial>::default());
    }
}
