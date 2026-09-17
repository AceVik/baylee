//! Smoked glass over animated elemental strata, with a mechanical compass.
//! Analytic reflections and caustics keep the surface in one opaque pass;
//! card art stays unlit and independent of the eased day/night exposure.

use baylee_client_core::tabletop;
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// Everything the felt shader needs.
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct FeltParams {
    /// The phase lamp: `rgb` its colour, `w` how much of it there is.
    ///
    /// Straight out of [`tabletop::phase_light`], eased rather than snapped —
    /// the rail runs all the way round the table and a step boundary that
    /// changed it in one frame would read as a flash.
    pub wash: Vec4,
    /// Where that light enters the rail: `xy` a point on the active seat's
    /// own edge in table space, `zw` the direction it travels from there.
    ///
    /// This is what keeps the signal honest at more than two seats. "Warm at
    /// one end, cool at the other" cannot be a property of a *ring*; it is a
    /// property of whose turn it is, and it moves.
    pub source: Vec4,
    /// The light the room is in: `rgb` a multiplier on the table's own
    /// colour, `w` how much of it arrives.
    ///
    /// Written by [`crate::sky::light_the_table`] out of the same eased
    /// [`SkyPhase`](baylee_client_core::sky::SkyPhase) the sky itself is
    /// drawn from, which is what makes the day/night change one movement:
    /// nothing here knows a transition is happening, and the table follows
    /// the sky because both read the same number.
    pub ambient: Vec4,
    /// How hard the first four flames of the firewheel burn, 0 to 1: white,
    /// blue, black, red.
    ///
    /// Out of [`baylee_client_core::firewheel::strength`], eased rather than
    /// snapped — a flame that grows over a second and a half is a fire being
    /// fed, and one that jumps is a signal.
    pub flames: Vec4,
    /// `x` green's strength; `yz` **screen-up in table space**; `w` spare.
    ///
    /// Screen-up is one direction for all five and not each flame's own
    /// radius, which is the difference between five candles seen from one
    /// chair and a sun glyph. It is the local seat's inward direction, so it
    /// is right at four seats and at eight as well as at a duel.
    pub flames_tail: Vec4,
    /// The slab's world size, which is how the shader turns a point on the
    /// table into a point in the cloth.
    ///
    /// The shader works from the world position and this, rather than from
    /// the mesh's own uv: a uv origin is a convention of whichever builder
    /// made the mesh, and guessing it wrong mirrors the whole field — which a
    /// duel, symmetric about both axes, would not show.
    pub span: Vec2,
    /// The corner radius the mesh was cut with, so the rail follows the same
    /// racetrack the slab does.
    pub corner: f32,
    /// How wide the padded rail runs, in table units.
    pub rail: f32,
    /// The clock the pulse runs on: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL).
    ///
    /// The same two values the cards use, and deliberately the same
    /// constants: a table that kept moving while every card on it held still
    /// would make the setting look broken.
    pub motion: f32,
    /// How hard the lamp burns at full energy.
    pub gain: f32,
    /// How thick the slab is, so the apron can be shaded down its height.
    pub thickness: f32,
    /// Mechanical compass angle, in radians.
    pub rotation: f32,
    /// Per-duel domain offset, orientation, and scale of the vascular pattern.
    pub pattern: Vec4,
}

/// Random presentation seed, sampled once when a duel is created.
/// Kept on `Duel`, so resizing, reconnecting, and card updates retain the surface.
#[derive(Clone, Copy, Debug)]
pub struct TablePattern(
    /// Bounded shader domain parameters: offset, angle, and scale.
    pub Vec4,
);

impl TablePattern {
    fn from_seed(seed: u64) -> Self {
        let bytes = seed.to_le_bytes();
        let part = |i| f32::from(u16::from_le_bytes([bytes[i], bytes[i + 1]])) / 256.0;
        Self(Vec4::new(part(0), part(2), part(4), part(6)))
    }
}

impl Default for TablePattern {
    fn default() -> Self {
        Self::from_seed(crate::host::fresh_seed())
    }
}

/// How bright the rail burns at the top of combat.
///
/// Above 1.0 deliberately, and what that buys is **saturation, not bloom**.
/// This comment used to say the camera was HDR and a value past white would
/// bloom; it is not and it does not. In bevy 0.19 an HDR intermediate is the
/// opt-in `Hdr` *component*, the table camera carries `Tonemapping::None`
/// and nothing else, and there is no bloom pass anywhere in this client — so
/// everything past 1.0 is clipped by the 8-bit target. The mistake is worth
/// recording rather than quietly deleting, because it is the mechanism the
/// next person designing light on this table will reach for: there is no
/// bloom to reach for, and a glow here has to be *painted* — an additive
/// term on the cloth, which is exactly what `felt.wgsl` does with this
/// number.
///
/// What the overshoot does do is hold the middle of the rail at white while
/// the ends still fall off, which is what a lamp looks like. It is safe here
/// in a way it would not be anywhere else on this table: the cards use their
/// own unlit shader and take no light from the scene, so the glow spreads
/// over the felt and stops at the cardboard.
pub const WASH_GAIN: f32 = 1.80;

/// The slab.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct FeltMaterial {
    /// Everything the shader reads. No textures at all — see the module
    /// header for why the cloth is arithmetic.
    #[uniform(0)]
    pub params: FeltParams,
}

impl Material for FeltMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/felt.wgsl".into()
    }

    /// The slab is the floor of the scene: opaque, and the only thing under
    /// the cards that is. Everything painted on top of it — the mats, the
    /// medallion, the glows — blends over it, which is what makes them read
    /// as lying on a table rather than as being the table.
    ///
    /// It is opaque *and* cut to shape, which is the point of the mesh being
    /// a racetrack: what shows outside the table is the sky, and it shows
    /// because there is no table there, not because the table is see-through.
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

/// Installs the felt material and its shader.
pub struct FeltMaterialPlugin;

impl Plugin for FeltMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/felt.wgsl");
        app.add_plugins(MaterialPlugin::<FeltMaterial>::default());
    }
}

#[cfg(test)]
mod pattern_tests {
    use super::*;

    #[test]
    fn distinct_seeds_produce_stable_bounded_shader_domains() {
        let a = TablePattern::from_seed(0).0;
        let b = TablePattern::from_seed(u64::MAX).0;
        assert_ne!(a, b);
        assert!(b.cmpge(Vec4::ZERO).all() && b.cmplt(Vec4::splat(256.0)).all());
        assert_eq!(a, TablePattern::from_seed(0).0);
        assert_eq!(b, TablePattern::from_seed(u64::MAX).0);
    }
}
