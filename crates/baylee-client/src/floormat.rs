//! The offer's light on the felt round a card (#298).
//!
//! What this client offers to do with a card — amber chasing round one that
//! can be activated, indigo round one it can pay for by tapping lands first,
//! steady gold round an armed one, a slow blue pulse round a land a plan will
//! tap — lit the frame's rim until #298 took the frame away. It is light on
//! the cloth now: a quad under the card, [`REACH`] past it on every side, a
//! child of the card so it follows every glide and tap. The card covers the
//! part of it under the print, and so does the next card of a fanned lane,
//! which the owner accepted for fans and rings.
//!
//! **The material key is the offers and nothing else**, so a table has at
//! most one light material per combination of the four, and the clock is on
//! the material for the reason
//! [`CardParams::motion`](crate::cardmat::CardParams::motion) gives.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::cardmat::glow;

/// How far past the card's edge the light reaches, in card widths:
/// `card_common.wgsl`'s `FLOOR_REACH`, which a test holds this to.
pub const REACH: f32 = 0.10;

/// What the light's shader reads.
///
/// Sixteen bytes, for the reason
/// [`MarksParams`](crate::marksmat::MarksParams) is thirty-two: a uniform
/// block under the GL backend is rounded up to a multiple of sixteen.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct FloorParams {
    /// The offers, [`glow::OFFERS`] of the card's glow word.
    pub glow: u32,
    /// The clock: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    pub motion: f32,
    /// The block's last eight bytes.
    pub pad: UVec2,
}

/// The light under one card.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct FloorMaterial {
    /// The offers and the clock.
    #[uniform(0)]
    pub params: FloorParams,
}

impl FloorMaterial {
    /// The light for the offers in `glow`, on the clock `motion`. Anything
    /// in `glow` that is not an offer is left out: protection is not light
    /// on the felt.
    #[must_use]
    pub fn new(glow: u32, motion: f32) -> Self {
        Self {
            params: FloorParams {
                glow: glow & glow::OFFERS,
                motion,
                pad: UVec2::ZERO,
            },
        }
    }
}

impl Material for FloorMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/floor.wgsl".into()
    }

    /// Light, so it adds to the felt rather than covering it.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    /// Where the light sits in the transparent pass: at the height it lies
    /// at, over the contact shadows. See
    /// [`table::sort_bias`](crate::table::sort_bias).
    fn depth_bias(&self) -> f32 {
        crate::table::sort_bias(crate::table::FLOOR_RUNG)
    }
}

/// The light's quad, `[width, height]` in card widths: the card and
/// [`REACH`] round it.
#[must_use]
pub fn quad_size() -> Vec2 {
    Vec2::new(
        1.0 + 2.0 * REACH,
        baylee_client_core::cardrail::CARD_TALL + 2.0 * REACH,
    )
}

/// Registers the material and ships its shader in the binary, for the reason
/// [`CardMaterialPlugin`](crate::cardmat::CardMaterialPlugin) embeds its own.
/// `card_common.wgsl`, which it imports, is registered there.
pub struct FloorMaterialPlugin;

impl Plugin for FloorMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/floor.wgsl");
        app.add_plugins(MaterialPlugin::<FloorMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardmat::tests::{check_wgsl, wgsl_const};

    /// The light's shader, parsed and validated against the stubs the strip's
    /// is.
    #[test]
    fn the_light_shader_compiles() {
        let prelude = "\
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
struct Globals { time: f32 };
@group(0) @binding(11) var<uniform> globals: Globals;
";
        check_wgsl(
            include_str!("shaders/floor.wgsl"),
            &format!("{prelude}{}", include_str!("shaders/card_common.wgsl")),
        );
    }

    /// One whole `std140` block, which is what the note on [`FloorParams`]
    /// claims and the GL backend needs.
    #[test]
    fn the_light_uniform_is_one_block() {
        assert_eq!(FloorParams::min_size().get(), 16);
    }

    /// The shader declares the uniform in the order Rust lays it out, for
    /// the reason the strip's test gives.
    #[test]
    fn the_shader_reads_the_uniform_rust_writes() {
        let src = include_str!("shaders/floor.wgsl");
        let open = src.find("struct FloorParams {").expect("the uniform");
        let body = &src[open..open + src[open..].find('}').expect("it closes")];
        let fields: Vec<&str> = body
            .lines()
            .map(str::trim)
            .filter(|l| !l.starts_with("//") && l.contains(':'))
            .collect();
        assert_eq!(
            fields,
            ["glow: u32,", "motion: f32,", "pad: vec2<u32>,"],
            "floor.wgsl lays the uniform out differently from `FloorParams`"
        );
    }

    /// The quad reaches as far as the shader lights: a quad shorter than the
    /// reach cuts the light off square at its edge, and a longer one is
    /// fragments spent on nothing.
    #[test]
    fn the_quad_reaches_as_far_as_the_light() {
        let theirs = wgsl_const(include_str!("shaders/card_common.wgsl"), "FLOOR_REACH");
        assert!(
            (theirs - REACH).abs() < 1e-6,
            "FLOOR_REACH: {REACH} here, {theirs} in the shader"
        );
    }

    /// Only the offers make a light: a hexproof card with nothing on offer
    /// lights nothing, and the same offer on a protected card is the same
    /// material as on a plain one.
    #[test]
    fn only_an_offer_is_light_on_the_felt() {
        assert_eq!(FloorMaterial::new(glow::HEXPROOF, 1.0).params.glow, 0);
        assert_eq!(
            FloorMaterial::new(glow::HEXPROOF | glow::ARMED, 1.0).params,
            FloorMaterial::new(glow::ARMED, 1.0).params
        );
    }
}
