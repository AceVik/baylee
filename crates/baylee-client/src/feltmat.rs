//! The table's surface: midnight mineral cloth inside an aged champagne rail,
//! on a slab with a thickness to it. It was casino baize until the September
//! 2026 redesign; `tabletop::FELT_CLOTH` is the colour, not this sentence.
//!
//! A material of its own rather than a [`StandardMaterial`] with a texture on
//! it, for one reason: the table is thirty-five units across and a card is
//! about 114 physical pixels at this camera, so cloth sharp enough to read
//! would want four thousand texels — and generating 2048 already costs 1.6
//! seconds in a debug build every time the table is re-cut. Arithmetic has no
//! resolution.
//!
//! It is also **unlit**, like the cards and unlike a `StandardMaterial`. The
//! stage carries no light at all — scene lighting on card art would make
//! colour identity unreadable, which is the one thing this table may not do —
//! so a lit slab would mean introducing a lamp for the benefit of one object
//! and then keeping every other object out of its way. The rail's roll and
//! the apron's fall-off are painted instead, which is the honest way to do it
//! at a camera this close to overhead: a real specular lobe would track the
//! viewer and drag along behind the cards every time the table was turned.
//!
//! Everything else under the cards stays a `StandardMaterial` — the mats, the
//! medallion, the glow — because none of it moves.
//!
//! # What this replaced
//!
//! A slab of dark timber with a channel of resin poured through it, and the
//! reason it is gone is not that it was badly drawn: it was ornament that
//! nobody at a card table expects to find, and it put a bright moving surface
//! down the middle of the board. The phase lamp it carried is the part worth
//! keeping, and it is kept — it runs round the rail now, which is the one
//! part of the table no card is ever laid on.

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
    /// The weather's own tint: `rgb` a second multiplier on the table's
    /// colour, `w` unused.
    ///
    /// Straight out of
    /// [`Weather::grade`](baylee_client_core::atmosphere::Weather::grade),
    /// and a field of its own rather than folded into [`Self::ambient`]
    /// because the two are scaled by different things — the sky's light
    /// arrives at a strength that depends on the hour, and the weather's does
    /// not. Multiplied together, a forest would stop being green at noon.
    ///
    /// A multiply and not a mix, for the reason this file keeps repeating:
    /// there is no light in this scene and there cannot be one. `(1, 1, 1)`
    /// is a table with nothing in the air over it, and is what the slab is
    /// cut with.
    pub weather: Vec4,
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
