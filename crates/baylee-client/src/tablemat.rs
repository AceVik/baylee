//! The table's surface: polished granite with runes burning in it.
//!
//! A material of its own rather than a [`StandardMaterial`] with a texture on
//! it, for one reason: the runes *breathe*. Doing that on the CPU would mean
//! regenerating a 1024² texture every frame, or a hundred materials with a
//! hundred tints; doing it in a shader costs one `sin` per pixel of table.
//!
//! Everything else under the cards stays a `StandardMaterial` — the mats, the
//! hearth, the medallion, the glow — because none of it moves, and a second
//! pipeline for a static quad buys nothing.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// Everything the table shader needs that is not a texture.
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct TableParams {
    /// The clock the runes breathe on: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL), from
    /// [`Preferences::reduce_motion`].
    ///
    /// The same two values the cards use, and deliberately the same
    /// constants: a table that kept breathing while every card on it held
    /// still would make the setting look broken.
    ///
    /// [`Preferences::reduce_motion`]: baylee_client_core::prefs::Preferences::reduce_motion
    pub motion: f32,
    /// How bright a rune burns at the top of its breath.
    ///
    /// Held low. The runes are ornament on the one surface that is behind
    /// *everything* — every card, every mat, every counter — and ornament
    /// that competes with the cards is worse than no ornament. This is the
    /// number to turn down first if the table ever starts pulling the eye.
    pub gain: f32,
}

/// How bright the runes burn, out of the box.
pub const RUNE_GAIN: f32 = 0.38;

/// The granite slab.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct TableMaterial {
    /// The stone itself, from [`tabletop::granite`].
    ///
    /// [`tabletop::granite`]: baylee_client_core::tabletop::granite
    #[texture(0)]
    #[sampler(1)]
    pub stone: Handle<Image>,
    /// The sigils, from [`tabletop::runes`] — colour premultiplied by
    /// coverage in `rgb`, the rune's phase in `a`.
    ///
    /// [`tabletop::runes`]: baylee_client_core::tabletop::runes
    #[texture(2)]
    #[sampler(3)]
    pub runes: Handle<Image>,
    /// Everything else.
    #[uniform(4)]
    pub params: TableParams,
}

impl Material for TableMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/table.wgsl".into()
    }

    /// The slab is the floor of the scene: opaque, and the only thing under
    /// the cards that is. Everything painted on top of it — the mats, the
    /// hearth, the glows — blends over it, which is what makes them read as
    /// lying on a table rather than as being the table.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

/// Installs the table material and its shader.
pub struct TableMaterialPlugin;

impl Plugin for TableMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/table.wgsl");
        app.add_plugins(MaterialPlugin::<TableMaterial>::default());
    }
}
