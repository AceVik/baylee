//! The moving surfaces the front door is drawn on, and the buttons that
//! answer the pointer.
//!
//! Two things live here because they are the same argument twice: a screen
//! that never moves reads as a screen that has stopped working. The lobby's
//! backdrop drifts, and a button leans towards the pointer and gives way
//! under a press — both small, both continuous, both switched off entirely by
//! [`baylee_client_core::prefs::Preferences::reduce_motion`].
//!
//! The backdrop is a shader rather than a picture for the reason
//! `docs/legal.md` §2 gives about the table's felt: ornament is the easiest
//! thing to borrow by accident, and arithmetic borrows nothing. It is also
//! the cheaper of the two — one full-screen pass with no texture to load, no
//! bytes in the wasm bundle, and nothing to fail to fetch in a browser.

use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::ui::UiTransform;

use bevy::asset::embedded_asset;

/// How fast a button reaches the state the pointer is asking for.
///
/// Exponential like [`crate::table::glide`] and for the same reason: it is
/// frame-rate independent, and a small correction does not take as long as a
/// large one. 18 is about 120 ms to settle, which is under the ~150 ms where
/// a control starts feeling sluggish and over the ~60 ms where it reads as an
/// instant snap.
const FEEL_RATE: f32 = 18.0;

/// How much a hovered button grows, and how far a pressed one gives way.
const HOVER_SCALE: f32 = 1.025;
const PRESS_SCALE: f32 = 0.975;

/// How far a hovered button is lightened towards white, and a pressed one
/// darkened. Small on purpose: the tone still has to read as the same button.
const HOVER_LIFT: f32 = 0.16;
const PRESS_SINK: f32 = 0.10;

/// The parameters of one ambient surface.
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct AmbienceParams {
    /// The ground colour.
    pub low: Vec4,
    /// What the bands are drawn in.
    pub high: Vec4,
    /// Motion and brightness of the bands. Zero stills the surface.
    pub energy: f32,
    /// Offsets the field, so two surfaces on screen at once do not repeat.
    pub seed: f32,
    /// Width over height of the node, so the field is not stretched.
    pub aspect: f32,
    /// Padding to the 16-byte boundary a uniform needs.
    pub pad: f32,
}

/// A drifting field, as a UI material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct AmbienceMaterial {
    /// Everything the shader reads.
    #[uniform(0)]
    pub params: AmbienceParams,
}

impl UiMaterial for AmbienceMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/ambience.wgsl".into()
    }
}

/// A node carrying an ambient surface, so its aspect can be kept honest and
/// its energy answered to the motion preference.
#[derive(Component, Clone, Copy)]
pub struct Ambient {
    /// The energy this surface has when motion is allowed.
    pub energy: f32,
}

impl Default for Ambient {
    fn default() -> Self {
        Self { energy: 1.0 }
    }
}

/// A button that answers the pointer.
///
/// It carries the tone it was built with rather than reading it back off the
/// node: the animation writes to [`BackgroundColor`] every frame, so after
/// one hover the node's own colour is no longer the colour it started with.
#[derive(Component, Clone, Copy)]
pub struct Feel {
    /// The colour the button rests at.
    pub base: Color,
    /// How far into the hovered state it currently is, 0 to 1. Negative
    /// values are the pressed side, so one number carries both.
    pub warmth: f32,
}

impl Feel {
    /// A button resting at `base`.
    #[must_use]
    pub fn new(base: Color) -> Self {
        Self { base, warmth: 0.0 }
    }
}

/// Registers the material, ships its shader inside the binary, and runs the
/// two animations.
///
/// Embedded rather than loaded from `assets/`, for the same reason
/// [`crate::cardmat::CardMaterialPlugin`] gives: `index.html` copies only the
/// font directory to `dist/`, so a shader on disk would work natively and
/// silently fail to load in a browser.
pub struct AmbiencePlugin;

impl Plugin for AmbiencePlugin {
    fn build(&self, app: &mut App) {
        // The button animation is arithmetic over components and runs
        // anywhere, including in the headless tests that build the lobby's
        // node tree without a renderer.
        app.add_systems(Update, (breathe, feel));
        // The surface is not: embedding a shader needs an asset server, and a
        // UI material needs a render world. A test app has neither, and
        // adding them regardless is a panic in every lobby test rather than
        // an honest "there is nothing to draw on here".
        if !app.world().contains_resource::<AssetServer>() {
            return;
        }
        embedded_asset!(app, "shaders/ambience.wgsl");
        app.add_plugins(UiMaterialPlugin::<AmbienceMaterial>::default());
    }
}

/// Adds [`AmbiencePlugin`] unless another plugin already did.
///
/// The lobby and the duel are separate plugins and either may be the only one
/// present; bevy panics on a duplicate, so "whoever gets there first" has to
/// be said out loud — the same shape [`crate::prefs::install`] uses.
pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<AmbiencePlugin>() {
        app.add_plugins(AmbiencePlugin);
    }
}

/// Keeps every ambient surface's aspect and energy in step with its node.
///
/// The aspect has to be written from here because a UI node's size is not
/// known until layout has run, and a field stretched to a 21:9 window looks
/// like a field that was authored for a different screen.
fn breathe(
    nodes: Query<(&Ambient, &ComputedNode, &MaterialNode<AmbienceMaterial>)>,
    materials: Option<ResMut<Assets<AmbienceMaterial>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    for (ambient, computed, handle) in &nodes {
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        let size = computed.size();
        let aspect = if size.y > 0.0 { size.x / size.y } else { 1.0 };
        let energy = if still { 0.0 } else { ambient.energy };
        material.params.aspect = aspect;
        material.params.energy = energy;
    }
}

/// Leans a hovered button towards the pointer and lets a pressed one give
/// way.
///
/// One system for every button in the client, driven by
/// [`PickingInteraction`] — which bevy maintains for anything the pointer can
/// hit, so a button gets this by carrying [`Feel`] and nothing else. Doing it
/// per screen would mean every new screen forgetting it once.
fn feel(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut buttons: Query<(
        &mut Feel,
        Option<&PickingInteraction>,
        &mut BackgroundColor,
        &mut UiTransform,
    )>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let step = if still {
        1.0
    } else {
        1.0 - (-FEEL_RATE * time.delta_secs()).exp()
    };
    for (mut feel, interaction, mut colour, mut transform) in &mut buttons {
        let target = match interaction {
            Some(PickingInteraction::Pressed) => -1.0,
            Some(PickingInteraction::Hovered) => 1.0,
            _ => 0.0,
        };
        feel.warmth += (target - feel.warmth) * step;
        if (feel.warmth - target).abs() < 0.001 {
            feel.warmth = target;
        }

        let hot = feel.warmth.max(0.0);
        let cold = (-feel.warmth).max(0.0);
        let scale = 1.0 + (HOVER_SCALE - 1.0) * hot + (PRESS_SCALE - 1.0) * cold;
        transform.scale = Vec2::splat(scale);
        colour.0 = shade(feel.base, HOVER_LIFT * hot - PRESS_SINK * cold);
    }
}

/// Lightens (positive) or darkens (negative) a colour, keeping its alpha.
///
/// Mixing towards white rather than scaling the channels: scaling a colour
/// that is already at 1.0 in one channel shifts its hue, and the accent green
/// turning yellow under the pointer is exactly the kind of thing nobody can
/// name but everybody sees.
fn shade(base: Color, by: f32) -> Color {
    let mut rgba = base.to_srgba();
    let towards = if by >= 0.0 { 1.0 } else { 0.0 };
    let amount = by.abs();
    rgba.red += (towards - rgba.red) * amount;
    rgba.green += (towards - rgba.green) * amount;
    rgba.blue += (towards - rgba.blue) * amount;
    rgba.into()
}

/// Spawns a full-bleed ambient surface, returning it for the caller to place.
///
/// Positioned absolutely and marked [`Pickable::IGNORE`]: it is the ground,
/// and a backdrop that swallowed clicks would make every button on top of it
/// unreachable.
///
/// Where it sits in the stack is the caller's business, and deliberately not
/// decided here. A surface spawned at the root needs a negative
/// [`GlobalZIndex`] to sit under the screen; the same call *inside* an overlay
/// must not have one, or it sinks below the screen it was meant to cover —
/// which is exactly what the loading veil did on its first attempt.
pub fn backdrop(
    commands: &mut Commands,
    materials: &mut Assets<AmbienceMaterial>,
    low: Color,
    high: Color,
    energy: f32,
    seed: f32,
) -> Entity {
    let handle = materials.add(AmbienceMaterial {
        params: AmbienceParams {
            low: LinearRgba::from(low).to_f32_array().into(),
            high: LinearRgba::from(high).to_f32_array().into(),
            energy,
            seed,
            aspect: 1.0,
            pad: 0.0,
        },
    });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            MaterialNode(handle),
            Ambient { energy },
            Pickable::IGNORE,
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lightening must not move the hue. The accent is a saturated green with
    /// a channel near 1.0, which is exactly where a multiplicative highlight
    /// goes wrong.
    #[test]
    fn a_highlight_keeps_the_colour_it_highlights() {
        let accent = Color::srgb(0.30, 0.78, 0.70);
        let lit = shade(accent, 0.2).to_srgba();
        let base = accent.to_srgba();
        assert!(lit.red > base.red && lit.green > base.green && lit.blue > base.blue);
        // Every channel moves a fifth of the way to white, so the ordering of
        // the channels — which is what hue is — cannot change.
        assert!(lit.green > lit.blue && lit.blue > lit.red);
        let sunk = shade(accent, -0.2).to_srgba();
        assert!(sunk.green < base.green && sunk.green > lit.red * 0.0);
    }

    /// Zero is the resting state and has to leave the colour untouched: the
    /// system writes it every frame to every button on screen.
    #[test]
    fn an_unhovered_button_is_the_colour_it_was_built_with() {
        let tone = Color::srgb(0.11, 0.13, 0.16);
        let rest = shade(tone, 0.0).to_srgba();
        let base = tone.to_srgba();
        assert!((rest.red - base.red).abs() < 1e-6);
        assert!((rest.green - base.green).abs() < 1e-6);
        assert!((rest.blue - base.blue).abs() < 1e-6);
    }

    /// The WGSL is parsed and validated with the same front end wgpu uses.
    /// The alternative is finding a typo when a pipeline is built — in a
    /// browser, where there is no filesystem to look at.
    #[test]
    fn the_ambience_shader_compiles() {
        let prelude = "\
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
struct Globals { time: f32 };
";
        crate::cardmat::tests::check_wgsl(include_str!("shaders/ambience.wgsl"), prelude);
    }
}
