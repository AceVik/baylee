//! The cloth that hangs from the actions row over the hand.
//!
//! The shader is `shaders/frontal.wgsl` and the whole argument is written
//! there. This is the half that has to exist in Rust: the material, the one
//! handle it is ever minted on, and the system that keeps the surface's idea
//! of its own size and of the motion preference in step with the node.
//!
//! It is shaped like [`crate::ambience`] on purpose — the same `UiMaterial`,
//! the same guard on there being a render world at all, the same
//! aspect-written-from-`ComputedNode`. What it does **not** share is the
//! field: a full-screen aurora with a vignette in its middle is authored for
//! a screen, and this node is 150 pixels tall and up to 3200 wide.
//!
//! # Why the handle lives in a resource
//!
//! [`crate::hud::sync_overlay`] rebuilds the whole hand zone whenever
//! [`crate::hud::HudRevision`] changes, and that counter **counts the hover**
//! — so the node this material is on is respawned on every pointer move
//! across a card. A material minted in the builder would be a fresh asset per
//! mouse move: a leak, and a pipeline specialisation each time. So it is
//! minted once into [`Cloth`] and handed out by clone, which is what
//! `UiCardMaterials` does for every card in the hand for the same reason.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::ui::ComputedNode;

/// Everything the cloth reads.
///
/// The colours are **linear**, because that is what the UI material pipeline
/// hands to the target — `ambience` passes `LinearRgba` for the same reason,
/// and mixing two dyes in linear space is also the only mix that means
/// anything.
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct FrontalParams {
    /// The gathered hem at the shelf; `w` is how dense the cloth is there.
    pub hem: Vec4,
    /// The sheer body; `w` is how dense it is at the window's edge.
    pub body: Vec4,
    /// The gathering stitch at each tack; `w` is how far it shows.
    pub thread: Vec4,
    /// How far the two movements travel. Zero leaves the cloth hanging still.
    pub energy: f32,
    /// Width over height of the node.
    pub aspect: f32,
    /// The node's height in logical pixels.
    pub height: f32,
    /// How dense the body is where it leaves the band.
    pub shoulder: f32,
}

/// The cloth, as a UI material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct FrontalMaterial {
    /// Everything the shader reads.
    #[uniform(0)]
    pub params: FrontalParams,
}

impl UiMaterial for FrontalMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/frontal.wgsl".into()
    }
}

/// The node wearing the cloth, so its size can be written back into the
/// material.
#[derive(Component, Clone, Copy, Default)]
pub struct Hanging;

/// The one handle the cloth is ever minted on.
#[derive(Resource, Default)]
pub struct Cloth(Option<Handle<FrontalMaterial>>);

impl Cloth {
    /// The handle, minted on the first call and cloned on every one after.
    ///
    /// `None` when there is no render world — a headless app has no
    /// `Assets<FrontalMaterial>`, and the hand zone falls back to the
    /// gradient this surface replaced rather than growing a second code path.
    pub fn get(
        &mut self,
        assets: Option<&mut Assets<FrontalMaterial>>,
    ) -> Option<Handle<FrontalMaterial>> {
        let assets = assets?;
        if let Some(handle) = self.0.as_ref() {
            return Some(handle.clone());
        }
        let handle = assets.add(FrontalMaterial {
            params: FrontalParams {
                hem: LinearRgba::from(HEM).to_f32_array().into(),
                body: ground(),
                thread: LinearRgba::from(THREAD).to_f32_array().into(),
                energy: 1.0,
                // Both are written by `hang` on the first frame the node has
                // a size; until then a square node is the least wrong guess.
                aspect: 1.0,
                height: crate::hud::HAND_ZONE_H - crate::hud::LEDGE_H,
                shoulder: SHOULDER,
            },
        });
        self.0 = Some(handle.clone());
        Some(handle)
    }
}

/// The gathered hem, and how dense the cloth is at the shelf.
///
/// The dye is `palette::DIALOG` taken down to 65 %, so the hem is the shelf's
/// own colour in shadow rather than a second material meeting it at a line.
/// The alpha is the number the whole transition is bought with, and it is not
/// the 0.97 the design asked for: the shelf casts a `BoxShadow` **onto** this
/// node — the owner asked for that cast on 14.09.2026 — and a hem at 0.97
/// under a cast at 0.25 is 0.99 over the first sixteen pixels, which is a
/// black stripe and is the panel scar reopening. The cast is the shelf's and
/// stays; the cloth gives way under it.
const HEM: Color = Color::srgba(0.072, 0.064, 0.049, 0.88);

/// The sheer body, and how dense it is at the window's edge.
///
/// **Read from the gradient it replaced rather than restated.** The ground a
/// card's glow is seen against was measured at that alpha — sky 136 down to
/// about 94 at the bottom edge — and it is the whole reason the strip under
/// the cards is dark at all. A prettier surface that quietly moved it would
/// be the reported "my own mana glow is gone" a second time, so the two are
/// the same two constants and not a test that they agree.
fn ground() -> Vec4 {
    let dye = LinearRgba::from(crate::hud::VEIL);
    Vec4::new(dye.red, dye.green, dye.blue, crate::hud::VEIL_ALPHA)
}

/// The gathering stitch. `DIALOG_LINE`, which is the shelf's own lip — the
/// cloth is tacked to the shelf, so it is tacked with the shelf's line.
const THREAD: Color = Color::srgba(0.216, 0.188, 0.122, 0.35);

/// How dense the body is where it leaves the band.
///
/// The one number in this file that is neither the shelf's nor the veil's. It
/// is what the strip between the band and the card bottoms reads at, which is
/// most of what a player with an empty hand is looking at, and it is not the
/// gradient's near-nothing because a sheer cloth that reaches zero has
/// stopped being cloth — the swag would then be a dark scallop hanging in
/// mid-air rather than the gathered top of something.
///
/// Measured at 1280 against this window's own sky, which reads 148 to 167
/// along the bottom and not the 136 the design was drawn against: the
/// gradient left the strip at y = 40 reading 163 and 0.33 took it to 150,
/// which is a cloth nobody can see. This is what makes the body read as the
/// same material as the band without closing the gap to it.
const SHOULDER: f32 = 0.38;

/// Keeps the cloth's idea of its own size, and of the motion preference, in
/// step with the node it is on.
///
/// The same job `ambience::breathe` does and the same trap: a UI node has no
/// size until layout has run, so the first frame after a respawn reports
/// zero — and this node is respawned on every pointer move. A division there
/// is a NaN aspect for one frame, every time.
fn hang(
    nodes: Query<(&ComputedNode, &MaterialNode<FrontalMaterial>), With<Hanging>>,
    materials: Option<ResMut<Assets<FrontalMaterial>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    for (computed, handle) in &nodes {
        let size = computed.size() * computed.inverse_scale_factor;
        if size.y <= 0.0 {
            continue;
        }
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        material.params.aspect = size.x / size.y;
        material.params.height = size.y;
        material.params.energy = f32::from(u8::from(!still));
    }
}

/// Installs the cloth, if there is anything to draw it on.
pub struct FrontalPlugin;

impl Plugin for FrontalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Cloth>();
        // The same guard `ambience` carries: embedding a shader needs an
        // asset server and a UI material needs a render world, and a headless
        // test app has neither. `Cloth::get` then answers `None` and the hand
        // zone draws the gradient it always did.
        if !app.world().contains_resource::<AssetServer>() {
            return;
        }
        embedded_asset!(app, "shaders/noise.wgsl");
        embedded_asset!(app, "shaders/frontal.wgsl");
        app.add_plugins(UiMaterialPlugin::<FrontalMaterial>::default())
            .add_systems(Update, hang);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The WGSL is parsed and validated with the same front end wgpu uses.
    ///
    /// The alternative is finding a typo when a pipeline is built — in a
    /// browser, where there is no filesystem to look at. The grain is pasted
    /// in ahead of it because `check_wgsl` strips `#import` lines, so an
    /// imported function is an unknown identifier to it.
    #[test]
    fn the_frontal_shader_compiles() {
        let prelude = format!(
            "\
struct UiVertexOutput {{
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
}};
struct Globals {{ time: f32 }};
{}",
            include_str!("shaders/noise.wgsl")
        );
        crate::cardmat::tests::check_wgsl(include_str!("shaders/frontal.wgsl"), &prelude);
    }

    /// The hem is dense, and still gives way under the shelf's own cast.
    ///
    /// Both ends stated, as §3.2 states every pair: under about 0.8 the seam
    /// at the shelf's edge comes back (the gradient started at zero and the
    /// step was 84 levels on one pixel row), and at 0.97 — the figure the
    /// design asked for — the hem plus the shelf's downward cast is 0.99 over
    /// the first sixteen pixels, which is the opaque strip this whole node
    /// exists to not be.
    #[test]
    fn the_hem_is_dense_enough_to_be_a_seam_and_sheer_enough_to_be_cloth() {
        assert!(
            HEM.alpha() >= 0.80,
            "a hem at {} leaves a step at the shelf's edge",
            HEM.alpha()
        );
        assert!(
            HEM.alpha() <= 0.92,
            "a hem at {} plus the shelf's cast is the panel again",
            HEM.alpha()
        );
    }

    /// The cloth speaks the shelf's register and the ground's, and nothing a
    /// card says.
    ///
    /// The shelf has `ledge.rs::the_ledge_speaks_only_dialog` for this; the
    /// cloth hangs off the shelf and is read in the same glance, so it is
    /// held to the same list. `ACTIVATABLE`, `ARMED` and `WILL_TAP` are rules
    /// statements on cards and nothing in the interface may borrow them.
    #[test]
    fn the_cloth_borrows_no_light_that_means_something() {
        let source = include_str!("frontal.rs")
            .split_once("#[cfg(test)]")
            .expect("the tests are still where they were")
            .0;
        let shader = include_str!("shaders/frontal.wgsl");
        for forbidden in [
            "ACTIVATABLE",
            "WILL_TAP",
            "palette::ARMED",
            "palette::BRASS",
            "palette::ACCENT",
            "palette::PARCHMENT",
            "palette::CANDLE",
        ] {
            assert!(
                !source.contains(forbidden),
                "the cloth writes with `{forbidden}`, which belongs to a card"
            );
            assert!(
                !shader.contains(forbidden),
                "the cloth's shader names `{forbidden}`"
            );
        }
    }

    /// A swag is always centred on the window, whatever the window is.
    ///
    /// The owner asked for *symmetric* curves, and the symmetry is arithmetic
    /// rather than a number that happens to divide. The shader's swag
    /// coordinate carries a half, which puts a tack at every half-pitch from
    /// the middle, so `x` and `-x` land at mirrored places inside mirrored
    /// swags. This is that same arithmetic, read back out of the shader so
    /// the two cannot drift — the pattern `camera_tests` uses on the felt.
    #[test]
    fn the_chain_of_swags_is_mirrored_about_the_middle() {
        let shader = include_str!("shaders/frontal.wgsl");
        let pitch: f32 = number(shader, "const PITCH: f32 = ");
        let shape: f32 = number(shader, "const CATENARY: f32 = ");
        assert!(
            shader.contains("let s = x / PITCH + 0.5;"),
            "the half is what centres a swag on the window"
        );

        let bow = |x: f32| {
            let s = x / pitch + 0.5;
            let u = s - s.floor();
            1.0 - ((shape * (2.0 * u - 1.0)).cosh() - 1.0) / (shape.cosh() - 1.0)
        };
        for x in [0.0, 17.0, 119.0, 121.0, 240.0, 613.0, 1599.0] {
            let (left, right) = (bow(-x), bow(x));
            assert!(
                (left - right).abs() < 1e-4,
                "at ±{x} the cloth hangs {left} on one side and {right} on the other"
            );
        }
        assert!(
            bow(0.0) > 0.99,
            "the middle of the window is a swag's middle"
        );
        assert!(
            bow(pitch / 2.0) < 0.01,
            "and half a pitch out is a tack, where the cloth is fixed"
        );
    }

    /// A card's step and the cloth's pitch must not line up.
    ///
    /// `hand_layout` caps the step at `HAND_CARD_W + 8`, and a swag whose
    /// tacks fell on the gaps between cards would read as scaffolding the
    /// cards were hung on rather than as cloth they are held in front of.
    #[test]
    fn the_swags_do_not_line_up_with_the_cards() {
        let pitch: f32 = number(include_str!("shaders/frontal.wgsl"), "const PITCH: f32 = ");
        let step = crate::hud::HAND_CARD_W + 8.0;
        let ratio = pitch / step;
        assert!(
            (ratio - ratio.round()).abs() > 0.15,
            "the pitch is {pitch} against a card step of {step}, which is \
             {ratio} steps — near enough to a whole one to read as a grid"
        );
    }

    /// Reads a `const` back out of the shader.
    fn number(shader: &str, name: &str) -> f32 {
        shader
            .split_once(name)
            .unwrap_or_else(|| panic!("`{name}` is no longer in the shader"))
            .1
            .split(';')
            .next()
            .expect("a const ends in a semicolon")
            .trim()
            .parse()
            .expect("and is a number")
    }
}
