//! The front door's scene (#295): a blue-hour landscape in depth, framed by
//! a cleft around the panel, which choosing a gateway walks through.
//!
//! It grows out of [`crate::ambience`]'s field (the same warped noise makes
//! its sky) and, like it, is arithmetic rather than a picture:
//! `docs/legal.md` §2 and §10 for why, and `shaders/vista.wgsl` for the
//! layers.
//!
//! This file schedules what the shader draws. A frame of the scene is a
//! [`Stage`]: the hour, the haze, the glow on the rim, and the two gates of a
//! passage. [`passage`] gives the stage at any point of walking through, from
//! the gateway's side (0) to the far side (1), so the front door's motion
//! only has to say how far it has come; [`waiting`] gives the stage of a
//! wait, which is the passage held open. Where the scene stands (the panel it
//! frames, how far the pointer leans it) is read off the screen each frame.
//!
//! Under `reduce_motion` nothing moves: no time, no pointer, no passage. The
//! hour still turns, as a quarter-second change of colour, so arriving on the
//! far side is still something seen.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::ui::UiGlobalTransform;
use bevy::window::PrimaryWindow;

/// Whether the shader draws what a phone's tiler pays most for: the fine
/// skyline and the river's sparkle.
const QUALITY: f32 = if cfg!(any(target_os = "android", target_os = "ios")) {
    0.0
} else {
    1.0
};

/// How fast the pointer's lean follows the pointer: a low pass at about
/// 4 Hz, so a flick of the mouse is a lean and not a jolt.
const GAZE_RATE: f32 = 25.0;

/// How fast the scene's frame follows its panel when the panel changes size
/// (the account form's two tabs are not the same height).
const FRAME_RATE: f32 = 10.0;

/// How long the scene takes to come or go when the front door does.
const PRESENCE_SECONDS: f32 = 0.4;

/// How long the hour takes to turn when nothing may move.
const STILL_HOUR_SECONDS: f32 = 0.25;

/// How long the hour takes to turn while a wait is held open: the longest
/// wait, taking a seat, lasts up to half a minute.
const WAIT_HOUR_SECONDS: f32 = 40.0;

/// How long a wait's own light takes to come up, so a wait that ends first
/// shows nothing of it.
const WAIT_RISE_SECONDS: f32 = 0.4;

/// How much of a wait's scene there is: nearly all, so the screen it covers
/// stays faintly in sight and a player can tell they have not been thrown
/// back to the beginning.
const WAIT_PRESENCE: f32 = 0.92;

/// What the scene draws at one moment, apart from where it stands.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Stage {
    /// 0 on the gateway's side, 1 on the far side: the same world an hour
    /// later, seen from inside the gate.
    pub hour: f32,
    /// The accent's fog, which hides the hour turning. Never white.
    pub haze: f32,
    /// How bright the rim's glow is, 1 at rest.
    pub glow: f32,
    /// The gate walked through.
    pub before: GateStage,
    /// How far the viewer has walked, 0 to 1: the ridge and the river come
    /// nearer with it, less than the gate does.
    pub dolly: f32,
    /// The gate arrived in.
    pub after: GateStage,
    /// How bright the river runs, 1 at rest.
    pub river: f32,
    /// How many sparks the rim throws, 1 at rest.
    pub sparks: f32,
}

/// One gate of a passage.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GateStage {
    /// How far the cleft stands off the panel: 1 in front of it, wider once
    /// the viewer stands inside it.
    pub opening: f32,
    /// How much of it there is.
    pub alpha: f32,
    /// How near it has come: 1 where it stands, larger as it passes.
    pub zoom: f32,
}

impl GateStage {
    const GONE: Self = Self {
        opening: 1.0,
        alpha: 0.0,
        zoom: 1.0,
    };
}

/// The opening of the gate the viewer stands inside, on the far side.
const INSIDE: f32 = 1.35;

/// The haze's peak on the way in, and on the way back out.
pub const HAZE_IN: f32 = 0.35;
/// See [`HAZE_IN`].
pub const HAZE_OUT: f32 = 0.25;

fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The scene `progress` of the way through the gate, 0 standing before it
/// and 1 arrived, with the haze peaking at `peak`.
///
/// Read as a second of film, which is what the way in takes (the way back
/// takes 0.8 s over the same curves): the rim brightens for a fifth of it,
/// then the viewer walks, the gate passing them from 0.20 to 0.68 while the
/// haze rises and falls once; the hour turns inside the haze (0.40 to 0.72),
/// and the far gate settles round the viewer from 0.55. Both ends are
/// complete stills: one gate, no haze.
#[must_use]
pub fn passage(progress: f32, peak: f32) -> Stage {
    let p = progress.clamp(0.0, 1.0);
    // Up in the first fifth, back to rest over the last half.
    let charge = smoothstep(0.0, 0.20, p) * (1.0 - smoothstep(0.55, 1.0, p));
    let dolly = smoothstep(0.20, 0.68, p);
    let settle = ((p - 0.55) / 0.45).clamp(0.0, 1.0);
    let settle = 1.0 - (1.0 - settle) * (1.0 - settle);
    Stage {
        hour: smoothstep(0.40, 0.72, p),
        haze: (std::f32::consts::PI * ((p - 0.20) / 0.70).clamp(0.0, 1.0)).sin() * peak,
        glow: 1.0 + 1.2 * charge,
        before: GateStage {
            opening: 1.0,
            alpha: 1.0 - smoothstep(0.55, 0.70, p),
            zoom: 1.0 / (1.0 - 0.55 * dolly),
        },
        dolly,
        after: GateStage {
            opening: 3.0 + (INSIDE - 3.0) * settle,
            alpha: smoothstep(0.55, 0.70, p),
            zoom: 1.0,
        },
        river: 1.0 + 0.5 * charge,
        sparks: 1.0 + 2.0 * charge,
    }
}

/// The scene of a wait `age` seconds old: the passage held open, the haze
/// standing like fog rather than passing like a flash, and the hour turning
/// slowly for as long as the wait lasts.
///
/// Lighter than the passage's own peak: a wait is looked at for up to half a
/// minute, and a haze held at the passage's strength washes the whole screen
/// pale.
#[must_use]
pub fn waiting(age: f32) -> Stage {
    let rise = smoothstep(0.0, WAIT_RISE_SECONDS, age);
    Stage {
        hour: (age / WAIT_HOUR_SECONDS).clamp(0.0, 1.0),
        haze: 0.10 * rise,
        glow: 1.0 + 0.4 * rise,
        before: GateStage {
            opening: 1.9,
            alpha: 1.0,
            zoom: 1.0,
        },
        dolly: 0.0,
        after: GateStage::GONE,
        river: 1.0 + 0.25 * rise,
        sparks: 1.0 + rise,
    }
}

/// The parameters of the scene, as the shader reads them.
#[derive(Clone, Copy, ShaderType, Debug, Default)]
pub struct VistaParams {
    /// The panel framed: centre xy, half extent zw, in the field's units.
    pub panel: Vec4,
    /// Pointer xy, aspect, energy.
    pub view: Vec4,
    /// Hour, haze, glow, quality.
    pub hour: Vec4,
    /// The gate walked through: opening, alpha, zoom, dolly.
    pub gate_a: Vec4,
    /// The gate arrived in: opening, alpha, zoom, unused.
    pub gate_b: Vec4,
    /// River brightness, spark rate, the scene's alpha, the river's zoom.
    pub air: Vec4,
}

/// The scene, as a UI material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct VistaMaterial {
    /// Everything the shader reads.
    #[uniform(0)]
    pub params: VistaParams,
}

impl UiMaterial for VistaMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/vista.wgsl".into()
    }
}

/// Which scene a surface draws.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vista {
    /// The front door's, scheduled by the lobby through [`FrontScene`].
    Front,
    /// A wait's, which schedules itself by its age.
    Wait,
}

/// The node a scene of this kind frames: the front door's panels, or the
/// wait's line of text.
#[derive(Component, Clone, Copy)]
pub struct Framed(pub Vista);

/// What the lobby asks of the front door's scene.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct FrontScene {
    /// This frame of the passage.
    pub stage: Stage,
    /// Whether the front door is on screen at all.
    pub shown: bool,
}

impl Default for FrontScene {
    fn default() -> Self {
        Self {
            stage: passage(0.0, HAZE_IN),
            shown: true,
        }
    }
}

/// Where the pointer leans the scene, -1 to 1 each way, low-passed.
#[derive(Resource, Default, Clone, Copy)]
pub struct Gaze(pub Vec2);

/// What one surface has settled on so far: the things that follow their
/// targets rather than jump to them.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Settled {
    panel: Option<Vec4>,
    presence: f32,
    hour: f32,
    age: f32,
}

/// Registers the material, ships its shader inside the binary, and keeps
/// every surface's uniforms in step with the screen.
pub struct VistaPlugin;

impl Plugin for VistaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrontScene>()
            .init_resource::<Gaze>()
            .add_systems(Update, (gaze, paint).chain());
        // As for the ambient field: a headless test has no asset server and
        // no render world, and there is nothing to draw on there.
        if !app.world().contains_resource::<AssetServer>() {
            return;
        }
        embedded_asset!(app, "shaders/vista.wgsl");
        app.add_plugins(UiMaterialPlugin::<VistaMaterial>::default());
    }
}

/// Adds [`VistaPlugin`] unless another plugin already did: the lobby and the
/// wait's veil both draw a scene, and either may be the only one present.
pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<VistaPlugin>() {
        app.add_plugins(VistaPlugin);
    }
}

/// Spawns a full-bleed scene of `kind`, returning it for the caller to place
/// (under the screen, or inside a veil, as [`crate::ambience::backdrop`]).
pub fn surface(
    commands: &mut Commands,
    materials: &mut Assets<VistaMaterial>,
    kind: Vista,
) -> Entity {
    let handle = materials.add(VistaMaterial {
        params: VistaParams::default(),
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
            kind,
            Settled::default(),
            Pickable::IGNORE,
        ))
        .id()
}

/// Follows the pointer. A window with no pointer in it (a phone between
/// touches) lets the scene drift back to the middle.
fn gaze(
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut gaze: ResMut<Gaze>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let target = windows
        .single()
        .ok()
        .and_then(|window| {
            let at = window.cursor_position()?;
            let size = window.size();
            (size.x > 0.0 && size.y > 0.0).then(|| (at / size) * 2.0 - Vec2::ONE)
        })
        .unwrap_or(Vec2::ZERO);
    let next = if still {
        Vec2::ZERO
    } else {
        gaze.0 + (target - gaze.0) * (1.0 - (-GAZE_RATE * time.delta_secs()).exp())
    };
    if gaze.0 != next {
        gaze.0 = next;
    }
}

/// Where a framed node stands, in the field's units of a surface `size`
/// (physical pixels, like the node's own).
fn frame_of(computed: &ComputedNode, place: &UiGlobalTransform, size: Vec2) -> Vec4 {
    let centre = place.translation / size.y;
    let half = computed.size() * 0.5 / size.y;
    Vec4::new(centre.x, centre.y, half.x, half.y)
}

/// A frame for a scene with nothing in it to frame yet: a panel of about the
/// front door's size in the middle.
fn unframed(aspect: f32) -> Vec4 {
    Vec4::new(aspect * 0.5, 0.5, (aspect * 0.4).min(0.3), 0.3)
}

/// Moves `from` towards `to` by `step` of the way.
fn towards(from: f32, to: f32, step: f32) -> f32 {
    if (to - from).abs() <= step {
        to
    } else {
        from + (to - from).signum() * step
    }
}

/// Writes each surface's uniforms for this frame.
#[allow(clippy::type_complexity)]
fn paint(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    gaze: Res<Gaze>,
    front: Res<FrontScene>,
    frames: Query<(&Framed, &ComputedNode, &UiGlobalTransform)>,
    mut surfaces: Query<(
        &Vista,
        &mut Settled,
        &ComputedNode,
        &MaterialNode<VistaMaterial>,
        &mut Visibility,
    )>,
    materials: Option<ResMut<Assets<VistaMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let dt = time.delta_secs();
    for (kind, mut settled, computed, handle, mut visibility) in &mut surfaces {
        let size = computed.size();
        if size.y <= 0.0 {
            continue;
        }
        let aspect = size.x / size.y;
        settled.age += dt;
        let (stage, presence) = match kind {
            Vista::Front => (front.stage, if front.shown { 1.0 } else { 0.0 }),
            Vista::Wait => (waiting(settled.age), WAIT_PRESENCE),
        };
        settled.presence = if still {
            presence
        } else {
            towards(settled.presence, presence, dt / PRESENCE_SECONDS)
        };
        settled.hour = if still {
            towards(settled.hour, stage.hour, dt / STILL_HOUR_SECONDS)
        } else {
            stage.hour
        };
        let target = frames
            .iter()
            .find(|(framed, computed, _)| framed.0 == *kind && computed.size().y > 0.0)
            .map_or_else(
                || unframed(aspect),
                |(_, node, place)| frame_of(node, place, size),
            );
        let panel = match settled.panel {
            Some(panel) if !still => panel + (target - panel) * (1.0 - (-FRAME_RATE * dt).exp()),
            _ => target,
        };
        settled.panel = Some(panel);

        let shown = settled.presence > 0.0;
        let wanted = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        visibility.set_if_neq(wanted);
        if !shown {
            continue;
        }
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        let energy = if still { 0.0 } else { 1.0 };
        material.params = VistaParams {
            panel,
            view: Vec4::new(gaze.0.x, gaze.0.y, aspect, energy),
            hour: Vec4::new(settled.hour, stage.haze, stage.glow, QUALITY),
            gate_a: Vec4::new(
                stage.before.opening,
                stage.before.alpha,
                stage.before.zoom,
                stage.dolly,
            ),
            gate_b: Vec4::new(
                stage.after.opening,
                stage.after.alpha,
                stage.after.zoom,
                0.0,
            ),
            air: Vec4::new(
                stage.river,
                stage.sparks,
                settled.presence,
                1.0 / (1.0 - 0.35 * stage.dolly),
            ),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both ends of the passage are stills: one gate standing, no haze, and
    /// the rim at rest. Anything else would be a scene that is still moving
    /// when nothing is.
    #[test]
    #[allow(clippy::float_cmp)] // the ends are exact: `smoothstep` clamps to 0 and 1
    fn a_passage_starts_and_ends_on_a_still() {
        for peak in [HAZE_IN, HAZE_OUT] {
            let before = passage(0.0, peak);
            assert_eq!(before.hour, 0.0);
            assert!(before.haze.abs() < 1e-6, "{before:?}");
            assert_eq!((before.before.alpha, before.before.zoom), (1.0, 1.0));
            assert_eq!(before.after.alpha, 0.0);
            assert!((before.glow - 1.0).abs() < 1e-6);

            let after = passage(1.0, peak);
            assert_eq!(after.hour, 1.0);
            assert!(after.haze.abs() < 1e-6, "{after:?}");
            assert_eq!(after.before.alpha, 0.0);
            assert_eq!(after.after.alpha, 1.0);
            assert!((after.after.opening - INSIDE).abs() < 1e-6);
            assert!((after.glow - 1.0).abs() < 1e-6);
        }
    }

    /// The hour turns only behind the haze, so the ridge changing its
    /// skyline is never seen happening, and the haze never passes its peak,
    /// which is the accent and never white.
    #[test]
    fn the_hour_turns_only_inside_the_haze() {
        for step in 0..=1000 {
            let p = step as f32 / 1000.0;
            let stage = passage(p, HAZE_IN);
            if stage.hour > 0.0 && stage.hour < 1.0 {
                assert!(stage.haze >= 0.15, "at {p}: {stage:?}");
            }
            assert!(stage.haze <= HAZE_IN + 1e-6);
            assert!(passage(p, HAZE_OUT).haze <= HAZE_OUT + 1e-6);
        }
    }

    /// A wait holds the gate open and turns the hour over the longest wait,
    /// and a wait too short to be read shows none of its own light.
    #[test]
    #[allow(clippy::float_cmp)] // the ends are exact: `smoothstep` clamps to 0 and 1
    fn a_wait_is_the_passage_held_open() {
        let fresh = waiting(0.0);
        assert_eq!((fresh.haze, fresh.glow, fresh.hour), (0.0, 1.0, 0.0));
        let held = waiting(10.0);
        assert!(held.before.alpha == 1.0 && held.before.opening > 1.0);
        assert!(held.hour > 0.2 && held.hour < 0.3, "{held:?}");
        assert_eq!(waiting(60.0).hour, 1.0);
    }

    /// The WGSL is parsed and validated with the same front end wgpu uses,
    /// as the ambient field's is.
    #[test]
    fn the_vista_shader_compiles() {
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
        let prelude = format!("{prelude}{}", include_str!("shaders/noise.wgsl"));
        crate::cardmat::tests::check_wgsl(include_str!("shaders/vista.wgsl"), &prelude);
    }
}
