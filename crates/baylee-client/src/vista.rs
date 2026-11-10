//! Baylee's sunlit world: a distant garden city under a wide cloud horizon.
//! The approved sleeping portrait is an independent interactive UI material
//! (`lobby::memorial`); this surface supplies atmosphere, reading contrast and
//! the existing login passage. Provenance: `art/baylee/README.md`.
//! All ambient movement stops under reduced motion or low ambient quality.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::ui::UiGlobalTransform;
use bevy::window::PrimaryWindow;

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
/// back to the beginning. Faintly: over a bright sky a form showing through
/// at a twelfth reads as clutter, not as a place.
const WAIT_PRESENCE: f32 = 0.96;

/// What the scene draws at one moment, apart from where it stands.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Stage {
    /// 0 on the gateway's side, 1 on the far side: the same world with the
    /// light broadened, seen from beyond the arch.
    pub hour: f32,
    /// The accent's fog, which hides the hour turning. Never white.
    pub haze: f32,
    /// How bright the rim's glow is, 1 at rest.
    pub glow: f32,
    /// The frame walked through.
    pub before: GateStage,
    /// How far the viewer has walked, 0 to 1: the ridge and the floor come
    /// nearer with it, less than the frame does.
    pub dolly: f32,
    /// The frame arrived in.
    pub after: GateStage,
    /// How bright the light's path on the floor runs, 1 at rest.
    pub river: f32,
    /// Strength of the inward dust stream, 1 at rest.
    pub sparks: f32,
}

/// One frame of a passage.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GateStage {
    /// How far the arch stands off the panel: 1 in front of it, wider once
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

/// The opening of the arch the viewer stands inside, on the far side.
const INSIDE: f32 = 1.35;

/// The haze's peak on the way in, and on the way back out.
pub const HAZE_IN: f32 = 0.55;
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

/// Successful authentication: gather light, accelerate through the current
/// arch, then reveal the already-loaded lobby from the centre outward.
#[must_use]
pub fn arrival(progress: f32) -> Stage {
    let p = progress.clamp(0.0, 1.0);
    let travel = smoothstep(0.10, 0.88, p);
    let charge = smoothstep(0.0, 0.22, p) * (1.0 - smoothstep(0.70, 1.0, p));
    Stage {
        hour: 1.0,
        haze: 0.16 * charge,
        glow: 1.0 + 1.8 * charge,
        before: GateStage {
            opening: INSIDE,
            alpha: 1.0 - smoothstep(0.65, 0.9, p),
            zoom: 1.0 / (1.0 - 0.94 * travel),
        },
        after: GateStage::GONE,
        dolly: travel,
        river: 1.0 + charge,
        sparks: 1.0 + 4.0 * charge,
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
#[derive(Clone, Copy, PartialEq, ShaderType, Debug, Default)]
pub struct VistaParams {
    /// The panel framed: centre xy, half extent zw, in the field's units.
    pub panel: Vec4,
    /// Baylee's screen-wide home: foot xy and height, independent of the panel.
    pub home: Vec4,
    /// Pointer xy, aspect, energy.
    pub view: Vec4,
    /// Hour, haze, glow, quality.
    pub hour: Vec4,
    /// The frame walked through: opening, alpha, zoom, dolly.
    pub gate_a: Vec4,
    /// The frame arrived in: opening, alpha, zoom, interior reading mode.
    pub gate_b: Vec4,
    /// The path's brightness, the ember rate, the scene's alpha, the floor's
    /// zoom.
    pub air: Vec4,
    /// Login flight progress, active flag, virtual seconds, logical height.
    pub portal: Vec4,
}

/// The scene, as a UI material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct VistaMaterial {
    /// Everything the shader reads.
    #[uniform(0)]
    pub params: VistaParams,
    /// Original sanctuary matte painting, decoded in sRGB.
    #[texture(1)]
    #[sampler(2)]
    pub sanctuary: Handle<Image>,
}

impl UiMaterial for VistaMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/vista.wesl".into()
    }
}

/// Which scene a surface draws.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vista {
    /// The front door's, scheduled by the lobby through [`FrontScene`].
    Front,
    /// The same garden, quietly lit behind the lobby and deck editor.
    Interior,
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
    /// The authenticated flight draws over the lobby while it is revealed.
    pub entering: bool,
    /// Successful login progress (0..1), zero before authentication.
    pub portal: f32,
}

impl Default for FrontScene {
    fn default() -> Self {
        Self {
            stage: passage(0.0, HAZE_IN),
            shown: true,
            entering: false,
            portal: 0.0,
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

/// Keep the front door's art resident, including while a restored session
/// skips its form. The arrival curtain waits for these handles before it lifts.
#[derive(Resource)]
pub(crate) struct VistaArt([Handle<Image>; 4]);

impl VistaArt {
    pub(crate) fn ready(&self, assets: &AssetServer) -> Result<bool, ()> {
        let mut ready = true;
        for image in &self.0 {
            if matches!(
                assets.get_load_state(image.id()),
                Some(bevy::asset::LoadState::Failed(_))
            ) {
                return Err(());
            }
            ready &= assets.is_loaded_with_dependencies(image.id());
        }
        Ok(ready)
    }
}

impl Plugin for VistaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrontScene>()
            .init_resource::<Gaze>()
            .add_systems(Update, (gaze, paint, step_vista_videos).chain());
        // As for the ambient field: a headless test has no asset server and
        // no render world, and there is nothing to draw on there.
        if !app.world().contains_resource::<AssetServer>() {
            return;
        }
        let assets = app.world().resource::<AssetServer>();
        let world_image = assets.load("scenes/baylees-world.png");
        let tavern_image = assets.load("scenes/tavern-lobby.png");
        #[cfg(not(target_arch = "wasm32"))]
        let world_id = world_image.id();
        #[cfg(not(target_arch = "wasm32"))]
        let tavern_id = tavern_image.id();
        let art = VistaArt([
            world_image,
            assets.load("scenes/baylee-cloud.png"),
            assets.load("brand/baylee-logo.png"),
            tavern_image,
        ]);
        app.insert_resource(art);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut streams = Vec::with_capacity(2);
            if let Some(s) = VideoStream::spawn(
                Vista::Front,
                world_id,
                "scenes/world-loop.mp4",
                "world-video-stream",
            ) {
                streams.push(s);
            }
            if let Some(s) = VideoStream::spawn(
                Vista::Interior,
                tavern_id,
                "scenes/tavern-loop.mp4",
                "tavern-video-stream",
            ) {
                streams.push(s);
            }
            if !streams.is_empty() {
                let videos = VistaVideos(std::sync::Arc::from(streams));
                app.insert_resource(videos.clone());
                if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
                    render_app.insert_resource(videos).add_systems(
                        bevy::render::Render,
                        upload_vista_frames
                            .in_set(bevy::render::RenderSystems::PrepareAssets)
                            .after(
                                bevy::render::render_asset::prepare_assets::<
                                    bevy::render::texture::GpuImage,
                                >,
                            ),
                    );
                }
            }
        }
        embedded_asset!(app, "shaders/vista.wesl");
        embedded_asset!(app, "shaders/memorial.wesl");
        embedded_asset!(app, "shaders/memorial_paper.wesl");
        embedded_asset!(app, "shaders/memorial_preview.wesl");
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
    assets: Option<&AssetServer>,
) -> Entity {
    let scene_path = match kind {
        Vista::Interior => "scenes/tavern-lobby.png",
        Vista::Front | Vista::Wait => "scenes/baylees-world.png",
    };
    let handle = materials.add(VistaMaterial {
        params: VistaParams::default(),
        sanctuary: assets.map_or_else(Handle::default, |a| a.load(scene_path)),
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

/// Baylee's home belongs to the window, not to whichever panel a scene draws.
fn home_anchor(aspect: f32) -> Vec4 {
    Vec4::new(aspect * 0.235, 0.968, 0.30, 0.0)
}

/// Moves `from` towards `to` by `step` of the way.
fn towards(from: f32, to: f32, step: f32) -> f32 {
    if (to - from).abs() <= step {
        to
    } else {
        from + (to - from).signum() * step
    }
}

/// The painted world's own clock, one for every surface that draws it, so the
/// front door and the lobby behind its aperture show one world at one moment:
/// seconds it has moved, stopped while a menu rests (`quality::Resting`) and
/// going on from there, never jumping.
#[derive(Default)]
pub(crate) struct WorldClock(f32);

/// What the device's settings ask of the world this frame: whether it stands
/// still (ambient `Low` freezes its own movement, never a transition), its
/// detail, and how far its clock moves (none while a menu rests).
fn ambience(
    still: bool,
    quality: Option<&crate::quality::InUse>,
    resting: Option<&crate::quality::Resting>,
    dt: f32,
) -> (bool, f32, f32, f32) {
    (
        crate::quality::ambient_still(still, quality),
        crate::quality::ambient_detail(quality),
        // The backdrop (WP5): how much of the painting is drawn; none at
        // all on Plain, whose surfaces are not drawn.
        quality.map_or(1.0, |q| q.0.backdrop.share()),
        if resting.is_some_and(|r| r.0) {
            0.0
        } else {
            dt
        },
    )
}

/// Writes each surface's uniforms for this frame.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)] // Bevy system: the stage, the screen and the settings it reads
#[allow(clippy::too_many_lines)] // one pass over the surfaces; its steps share the frame's numbers
pub(crate) fn paint(
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
        Option<&mut GlobalZIndex>,
    )>,
    materials: Option<ResMut<Assets<VistaMaterial>>>,
    quality: Option<Res<crate::quality::InUse>>,
    resting: Option<Res<crate::quality::Resting>>,
    mut clock: Local<WorldClock>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let dt = time.delta_secs();
    let (frozen, detail, backdrop, world_dt) =
        ambience(still, quality.as_deref(), resting.as_deref(), dt);
    clock.0 += world_dt;
    for (kind, mut settled, computed, handle, mut visibility, z) in &mut surfaces {
        let size = computed.size();
        if size.y <= 0.0 {
            continue;
        }
        let aspect = size.x / size.y;
        settled.age += dt;
        let (stage, presence) = match kind {
            Vista::Front => (front.stage, if front.shown { 1.0 } else { 0.0 }),
            Vista::Interior => (
                passage(1.0, HAZE_IN),
                if !front.shown || front.entering {
                    1.0
                } else {
                    0.0
                },
            ),
            Vista::Wait => (waiting(settled.age), WAIT_PRESENCE),
        };
        let entering = *kind == Vista::Front && front.entering && !still;
        if *kind == Vista::Front
            && let Some(mut z) = z
        {
            z.set_if_neq(GlobalZIndex(if entering { 50 } else { -1 }));
        }
        settled.presence = if still || (*kind == Vista::Front && front.portal > 0.0 && !front.shown)
        {
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
            // The form has already gone; keep its aperture throughout the
            // flight instead of chasing the fallback panel behind the lobby.
            Some(panel) if entering => panel,
            Some(panel) if !still => panel + (target - panel) * (1.0 - (-FRAME_RATE * dt).exp()),
            _ => target,
        };
        settled.panel = Some(panel);

        let shown = settled.presence > 0.0 && backdrop > 0.0;
        let wanted = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        visibility.set_if_neq(wanted);
        if !shown {
            continue;
        }
        let energy = if frozen { 0.0 } else { 1.0 };
        let next_params = VistaParams {
            panel,
            home: home_anchor(aspect),
            view: Vec4::new(gaze.0.x, gaze.0.y, aspect, energy),
            hour: Vec4::new(settled.hour, stage.haze, stage.glow, detail),
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
                if *kind == Vista::Interior { 1.0 } else { 0.0 },
            ),
            portal: Vec4::new(
                if entering { front.portal } else { 0.0 },
                if entering { 1.0 } else { 0.0 },
                clock.0 * energy,
                size.y * computed.inverse_scale_factor(),
            ),
            air: Vec4::new(
                stage.river,
                stage.sparks,
                settled.presence * backdrop,
                1.0 / (1.0 - 0.35 * stage.dolly),
            ),
        };
        if materials
            .get(&handle.0)
            .is_some_and(|m| m.params != next_params)
            && let Some(mut material) = materials.get_mut(&handle.0)
        {
            material.params = next_params;
        }
    }
}

/// Streams the pre-built 4K 60fps seamless video loops (`scenes/world-loop.mp4` for
/// [`Vista::Front`] and `scenes/tavern-loop.mp4` for [`Vista::Interior`]) via hardware
/// decoding into double-buffered staging slots, uploading directly into the GPU textures
/// in [`bevy::render::RenderApp`] via `RenderQueue::write_texture` without mutating
/// `Assets<Image>` or reallocating `MTLTexture`s. Inactive streams automatically reap
/// their `ffmpeg` child and park with zero CPU/GPU cost.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Clone)]
struct VistaVideos(std::sync::Arc<[std::sync::Arc<VideoStream>]>);

#[cfg(not(target_arch = "wasm32"))]
struct VideoStream {
    kind: Vista,
    image_id: AssetId<Image>,
    wanted_tick: std::sync::atomic::AtomicU64,
    active: std::sync::atomic::AtomicBool,
    stop: std::sync::atomic::AtomicBool,
    ready: std::sync::Mutex<ReadyFrame>,
    child: std::sync::Mutex<Option<std::process::Child>>,
    reader: std::sync::OnceLock<std::thread::Thread>,
}

#[cfg(not(target_arch = "wasm32"))]
struct ReadyFrame {
    seq: u64,
    uploaded_seq: u64,
    buf: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for VideoStream {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut guard) = self.child.lock()
            && let Some(mut child) = guard.take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(t) = self.reader.get() {
            t.unpark();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl VideoStream {
    const WIDTH: u32 = 3840;
    const HEIGHT: u32 = 2160;
    const FRAME_BYTES: usize = (Self::WIDTH as usize) * (Self::HEIGHT as usize) * 4;
    const FPS: f32 = 60.0;

    /// A decoder that failed this many times in a row is given up for the
    /// session: the still picture stays, and nothing respawns at frame rate.
    const MOST_FAILURES: u32 = 5;

    /// The pause after the `n`th failure in a row: 0.25 s, doubling, at most 8 s.
    fn backoff(failures: u32) -> std::time::Duration {
        std::time::Duration::from_millis(250 << failures.min(5))
    }

    fn spawn_ffmpeg(mp4_path: &std::path::Path) -> Option<std::process::Child> {
        use std::process::{Command, Stdio};
        Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-hwaccel",
                "videotoolbox",
                "-stream_loop",
                "-1",
                "-i",
            ])
            .arg(mp4_path)
            .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()
    }

    fn spawn(
        kind: Vista,
        image_id: AssetId<Image>,
        rel_path: &'static str,
        thread_name: &'static str,
    ) -> Option<std::sync::Arc<Self>> {
        use std::io::Read;
        use std::sync::{
            Arc, Mutex, OnceLock,
            atomic::{AtomicBool, AtomicU64, Ordering},
        };

        // VideoToolbox and a player's own ffmpeg are a Mac's; elsewhere the
        // still picture stands, as it does before a loop is found.
        if !cfg!(target_os = "macos") {
            return None;
        }
        // Where the asset server reads `assets/` from: `BEVY_ASSET_ROOT`, the
        // crate under `cargo run`, else beside the executable (a package). Not
        // a path baked in at build time: that names the build machine's tree.
        let mp4_path = bevy::asset::io::file::FileAssetReader::get_base_path()
            .join("assets")
            .join(rel_path);
        let stream = Arc::new(Self {
            kind,
            image_id,
            wanted_tick: AtomicU64::new(0),
            active: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            ready: Mutex::new(ReadyFrame {
                seq: 0,
                uploaded_seq: 0,
                // Filled on the first frame: a client whose video never
                // plays (no ffmpeg, no file) holds no 33 MB frames.
                buf: Vec::new(),
            }),
            child: Mutex::new(None),
            reader: OnceLock::new(),
        });
        let stream_bg = Arc::clone(&stream);
        let handle = std::thread::Builder::new()
            .name(thread_name.to_string())
            .spawn(move || {
                let mut work_buf = Vec::new();
                let mut failures: u32 = 0;
                // When the next try may spawn a decoder: the reader is unparked
                // every frame the vista shows, so a pause alone would not hold.
                let mut retry_at: Option<std::time::Instant> = None;
                let mut last_tick = u64::MAX;
                let mut stdout: Option<std::process::ChildStdout> = None;
                let mut idle_polls: u32 = 0;

                while !stream_bg.stop.load(Ordering::Relaxed) {
                    if !stream_bg.active.load(Ordering::Relaxed) {
                        idle_polls = idle_polls.saturating_add(1);
                        if idle_polls > 20
                            && stdout.take().is_some()
                            && let Ok(mut guard) = stream_bg.child.lock()
                            && let Some(mut child) = guard.take()
                        {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                        if stdout.is_none() {
                            std::thread::park();
                        } else {
                            std::thread::park_timeout(std::time::Duration::from_millis(25));
                        }
                        continue;
                    }
                    idle_polls = 0;
                    let tick = stream_bg.wanted_tick.load(Ordering::Relaxed);
                    if tick == last_tick {
                        std::thread::park_timeout(std::time::Duration::from_millis(8));
                        continue;
                    }
                    if stdout.is_none() {
                        if failures >= Self::MOST_FAILURES || !mp4_path.is_file() {
                            // Given up, or nothing to play: wait for `stop`.
                            std::thread::park();
                            continue;
                        }
                        if let Some(at) = retry_at {
                            let now = std::time::Instant::now();
                            if now < at {
                                std::thread::park_timeout(at - now);
                                continue;
                            }
                        }
                        if let Some(mut proc) = Self::spawn_ffmpeg(&mp4_path) {
                            work_buf.resize(Self::FRAME_BYTES, 0);
                            stdout = proc.stdout.take();
                            if let Ok(mut guard) = stream_bg.child.lock()
                                && let Some(mut old) = guard.replace(proc)
                            {
                                let _ = old.kill();
                                let _ = old.wait();
                            }
                        } else {
                            // No ffmpeg on this machine.
                            failures = Self::MOST_FAILURES;
                            continue;
                        }
                    }
                    if let Some(pipe) = stdout.as_mut() {
                        if pipe.read_exact(&mut work_buf).is_ok() {
                            failures = 0;
                            last_tick = tick;
                            if let Ok(mut guard) = stream_bg.ready.lock() {
                                std::mem::swap(&mut work_buf, &mut guard.buf);
                                guard.seq = guard.seq.wrapping_add(1);
                            }
                        } else {
                            // The decoder ended or could not decode: reap it and
                            // wait before the next try, so a failing ffmpeg is not
                            // respawned at frame rate.
                            stdout = None;
                            if let Ok(mut guard) = stream_bg.child.lock()
                                && let Some(mut child) = guard.take()
                            {
                                let _ = child.kill();
                                let _ = child.wait();
                            }
                            failures += 1;
                            retry_at = Some(std::time::Instant::now() + Self::backoff(failures));
                        }
                    }
                }
            })
            .ok()?;
        let _ = stream.reader.set(handle.thread().clone());
        Some(stream)
    }
}

#[allow(unused_variables)]
fn step_vista_videos(
    surfaces: Query<(&Vista, &MaterialNode<VistaMaterial>, &Visibility)>,
    materials: Option<Res<Assets<VistaMaterial>>>,
    #[cfg(not(target_arch = "wasm32"))] videos: Option<Res<VistaVideos>>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::atomic::Ordering;
        let (Some(materials), Some(videos)) = (materials, videos) else {
            return;
        };
        for stream in videos.0.iter() {
            let mut active = false;
            for (kind, handle, vis) in &surfaces {
                if *kind != stream.kind || *vis == Visibility::Hidden {
                    continue;
                }
                let Some((t, energy)) = materials
                    .get(&handle.0)
                    .map(|m| (m.params.portal.z, m.params.view.w))
                else {
                    continue;
                };
                if energy <= 0.0 {
                    continue;
                }
                active = true;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let tick = (t * VideoStream::FPS) as u64;
                let prev = stream.wanted_tick.swap(tick, Ordering::Relaxed);
                if prev != tick
                    && let Some(reader) = stream.reader.get()
                {
                    reader.unpark();
                }
                break;
            }
            let was_active = stream.active.swap(active, Ordering::Relaxed);
            if active != was_active
                && let Some(reader) = stream.reader.get()
            {
                reader.unpark();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn upload_vista_frames(
    videos: Option<Res<VistaVideos>>,
    gpu_images: Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
    render_queue: Res<bevy::render::renderer::RenderQueue>,
) {
    let Some(videos) = videos else {
        return;
    };
    for stream in videos.0.iter() {
        if !stream.active.load(std::sync::atomic::Ordering::Relaxed) {
            continue;
        }
        let Some(gpu_image) = gpu_images.get(stream.image_id) else {
            continue;
        };
        // The frames are 8-bit RGBA at 3840 × 2160: a still picture of
        // another size or depth would make the copy invalid.
        let size = gpu_image.texture_descriptor.size;
        if size.width != VideoStream::WIDTH
            || size.height != VideoStream::HEIGHT
            || !matches!(
                gpu_image.texture_descriptor.format,
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
                    | bevy::render::render_resource::TextureFormat::Rgba8Unorm
            )
        {
            continue;
        }
        let Ok(mut guard) = stream.ready.try_lock() else {
            continue;
        };
        if guard.seq == guard.uploaded_seq || guard.buf.len() != VideoStream::FRAME_BYTES {
            continue;
        }
        render_queue.write_texture(
            gpu_image.texture.as_image_copy(),
            &guard.buf,
            bevy::render::render_resource::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(VideoStream::WIDTH * 4),
                rows_per_image: None,
            },
            size,
        );
        guard.uploaded_seq = guard.seq;
    }
}

#[cfg(test)]
mod tests;
