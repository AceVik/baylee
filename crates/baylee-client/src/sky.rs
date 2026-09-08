//! The sky behind the table, and the clock that decides which one it is.
//!
//! `baylee_client_core::sky` decides *what* to draw — day, night, or where in
//! a dawn — and this draws it. The split is the usual one, and it is load
//! bearing here for a specific reason: the decision needs a wall clock, and a
//! wall clock is the one thing this crate can read and that one cannot.
//! `std::time::SystemTime::now` panics on `wasm32-unknown-unknown`, so the
//! hour is read with [`web_time`] — which is `std::time` everywhere except a
//! browser, and the browser's own clock there.
//!
//! # The quad
//!
//! One rectangle, parented to the camera, big enough to cover the frustum at
//! the distance it is placed at. It is a **child** rather than a system that
//! moves it, because a sky that was repositioned once a frame would lag the
//! camera by exactly one frame while the player was dragging it, and a sky
//! that swims behind a moving table is worse than no sky.
//!
//! It is drawn in screen space by the shader, so the mesh is nothing but a
//! surface to run a fragment over; its size only has to be *enough*. That is
//! also why it needs no resize system: a window that grows wider is still
//! looking at the middle of a quad cut for a much wider one.
//!
//! Depth does the rest. The quad stands at [`SKY_DISTANCE`], further than
//! anything else in the scene and nearer than the camera's far plane, so the
//! table and every card on it pass the depth test in front of it without a
//! render-layer or a bias being involved.

use baylee_client_core::sky::SkyPhase;
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// How far in front of the camera the sky stands, in table units.
///
/// Well past [`crate::table::MAX_DISTANCE`] so nothing the player can zoom to
/// reaches it, and well inside the default far plane (1000) so it is never
/// clipped away.
pub const SKY_DISTANCE: f32 = 500.0;

/// The widest window the quad is cut for, as width over height.
///
/// Generous: overdraw here is a handful of fragments outside the frustum,
/// which are clipped, and the alternative is a system that re-cuts a mesh on
/// every resize for a surface with nothing on it.
const WIDEST: f32 = 4.0;

/// How fast the sky follows a change of hour, per second.
///
/// Slow, and it has to be: a dawn takes two hours of the player's clock, so
/// the only time this rate is visible at all is the moment a player changes
/// the setting in `settingsui` — and that moment is now the one this number
/// is chosen for. That sentence is only true because [`hang_sky`] seeds the
/// sky at the hour it is; a sky that started at noon would spend these six
/// seconds every launch after dark correcting itself in front of the player.
/// At 2.5 the crossfade was over inside a second and a half,
/// which reads as a cut with a smear on it. At 0.55 it takes about six
/// seconds: long enough that a player watches the stars come out over the
/// table rather than finding that they have, and short enough that nobody
/// waits for the setting they just chose.
///
/// It costs nothing when nobody is watching. The rate only applies while the
/// phase is actually moving, and [`sync_sky`] stops writing the moment it
/// arrives.
const FADE_RATE: f32 = 0.55;

/// Everything the sky shader reads.
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct SkyParams {
    /// 0 is full night, 1 is full day.
    pub day: f32,
    /// 0 at noon and at midnight, 1 in the middle of a dawn or a dusk.
    pub glow: f32,
    /// The clock everything moves on: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL), the same two values the cards and
    /// the table use.
    pub motion: f32,
    /// Padding to the 16-byte boundary a uniform needs.
    pub pad: f32,
}

/// The sky.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct SkyMaterial {
    /// Everything the shader reads. No textures: see `shaders/sky.wgsl`.
    #[uniform(0)]
    pub params: SkyParams,
}

impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/sky.wgsl".into()
    }

    /// Opaque, and the furthest opaque thing there is. Everything else in the
    /// scene is nearer and simply wins the depth test.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

/// Marks the one quad, so the phase can be written to it.
#[derive(Component)]
pub struct Sky {
    /// The phase actually on screen, eased towards the one the clock asks
    /// for. Kept here rather than recomputed because easing towards a target
    /// needs somewhere to keep where it started.
    shown: SkyPhase,
}

/// Hangs the sky on the camera, the first time there is a camera to hang it
/// on.
///
/// A system rather than part of `spawn_stage` because the sky belongs to the
/// camera and `spawn_stage` is where the camera is *made*: adding a child in
/// the same command batch that creates the parent works, and reads as if the
/// order were an accident. This runs after, sees the camera, and is a no-op
/// on every frame after the first.
///
/// It reads the clock itself rather than leaving that to [`sync_sky`],
/// because the two would then disagree for six seconds. A sky hung at full
/// day and left to ease towards the truth means a player opening the game at
/// eleven at night watches a sunset they did not ask for — and a picture that
/// visibly corrects itself on launch reads as a bug, not as weather.
/// [`FADE_RATE`] is for a change of *setting*, and on the first frame there
/// is nothing to change from.
pub fn hang_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SkyMaterial>>,
    prefs: Res<crate::prefs::Prefs>,
    cameras: Query<Entity, With<crate::table::TableCamera>>,
    hung: Query<(), With<Sky>>,
) {
    if !hung.is_empty() {
        return;
    }
    let Ok(camera) = cameras.single() else {
        return;
    };
    // Half the frustum's height at the sky's distance, from the same `FOV`
    // the camera is built with. Doubled for the mesh, and widened by
    // `WIDEST` sideways.
    let half = SKY_DISTANCE * (crate::table::FOV * 0.5).tan();
    let quad = meshes.add(Rectangle::new(half * 2.0 * WIDEST, half * 2.0));
    // The hour it actually is, at the setting the player actually keeps —
    // the same call `sync_sky` makes every frame, made once here so the first
    // frame is already right.
    let start = baylee_client_core::sky::phase(prefs.all().sky, local_hour());
    let sky = commands
        .spawn((
            Sky { shown: start },
            Mesh3d(quad),
            MeshMaterial3d(materials.add(SkyMaterial {
                params: SkyParams {
                    day: start.day,
                    glow: start.glow,
                    motion: crate::cardmat::MOVING,
                    pad: 0.0,
                },
            })),
            // The camera looks down its own −z, so the sky stands there,
            // facing back at it.
            Transform::from_xyz(0.0, 0.0, -SKY_DISTANCE),
            // A pointer on the sky means nothing: it is not in the world.
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(camera).add_child(sky);
}

/// The light the table stands in, as the felt shader wants it: `xyz` a
/// multiplier on the table's own colour, `w` how much of it arrives.
///
/// A resource rather than a second read of the clock, because the light has
/// to follow the sky through the *eased* phase and not the one the clock
/// asks for. That is what makes the change from day to night one movement:
/// the sky crossfades, and the table crossfades with it because both are
/// driven from the same number on the same frame.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TableLight(pub Vec4);

impl Default for TableLight {
    /// The table's own colour, unlit — what it looks like before a clock has
    /// been read.
    fn default() -> Self {
        Self(Vec4::new(1.0, 1.0, 1.0, 0.0))
    }
}

/// Puts the sky's light on the table.
///
/// Split from [`sync_sky`] because it writes a different asset, and kept out
/// of `table::sync_table` because that system is already at clippy's argument
/// budget and this needs neither the layout nor the board. What it does need
/// is to run *after* the phase has been eased, which the system order in
/// `lib.rs` gives it.
///
/// Writes only when the light moves, for the reason the whole file keeps
/// repeating: a material touched every frame is a uniform uploaded every
/// frame for a table that has not changed.
pub fn light_the_table(
    light: Res<TableLight>,
    mut materials: ResMut<Assets<crate::feltmat::FeltMaterial>>,
    slabs: Query<&MeshMaterial3d<crate::feltmat::FeltMaterial>>,
) {
    let Ok(handle) = slabs.single() else {
        return;
    };
    let Some(mut material) = materials.get_mut(&handle.0) else {
        return;
    };
    if (material.params.ambient - light.0).abs().max_element() <= 1e-4 {
        return;
    }
    material.params.ambient = light.0;
}

/// Keeps the sky at the hour it is, and at the setting the player asked for.
///
/// Writes only when the phase actually moves. A sky that reached its target
/// and went on writing would touch a material — and so a uniform upload —
/// every frame for the rest of the game, which is the same garbage
/// `sync_zones` exists to avoid.
pub fn sync_sky(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut light: ResMut<TableLight>,
    mut materials: ResMut<Assets<SkyMaterial>>,
    mut sky: Query<(&mut Sky, &MeshMaterial3d<SkyMaterial>)>,
) {
    let Ok((mut sky, handle)) = sky.single_mut() else {
        return;
    };
    let settings = prefs.all();
    let want = baylee_client_core::sky::phase(settings.sky, local_hour());
    let motion = if settings.reduce_motion {
        crate::cardmat::STILL
    } else {
        crate::cardmat::MOVING
    };
    let ease = if settings.reduce_motion {
        1.0
    } else {
        1.0 - (-FADE_RATE * time.delta_secs()).exp()
    };

    let next = SkyPhase {
        day: sky.shown.day + (want.day - sky.shown.day) * ease,
        glow: sky.shown.glow + (want.glow - sky.shown.glow) * ease,
    };
    // The table's light comes off the *eased* phase, and is written before
    // the early return below rather than after it: the sky stops writing when
    // it arrives, and the table has to have arrived with it.
    let lit = baylee_client_core::sky::table_light(next);
    light.0 = Vec4::new(lit.rgb[0], lit.rgb[1], lit.rgb[2], lit.strength);

    let Some(mut material) = materials.get_mut(&handle.0) else {
        return;
    };
    let still = (next.day - sky.shown.day).abs() <= 1e-4
        && (next.glow - sky.shown.glow).abs() <= 1e-4
        && (material.params.motion - motion).abs() <= f32::EPSILON;
    if still {
        return;
    }
    sky.shown = next;
    material.params.day = next.day;
    material.params.glow = next.glow;
    material.params.motion = motion;
}

/// The player's own local hour, with its minutes as a fraction.
///
/// `web_time` rather than `std::time` for one reason: this crate compiles for
/// `wasm32-unknown-unknown`, where `SystemTime::now` panics. There it is the
/// browser's clock, and everywhere else it *is* `std::time`.
///
/// The offset from UTC is not asked for. `chrono`/`time` with a timezone
/// database is a large dependency for one number, and a browser will not hand
/// out the zone database anyway — so the hour is UTC, and the sky is
/// therefore right on the prime meridian and up to half a day out at the
/// ends of the world. That is a real limitation and it is written down in
/// `docs/observed-faults.md` 43 rather than hidden: a player it bothers has
/// `Day` and `Night` to pin, which is the reason those two modes exist at all.
fn local_hour() -> f32 {
    let secs = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(43_200, |since| since.as_secs() % 86_400);
    // `u32` first: 86 400 is exact in an `f32` and the seconds of a day are
    // well inside what one can count.
    #[expect(clippy::cast_precision_loss)] // bounded by a day, exact in f32
    let hour = (secs % 86_400) as f32 / 3600.0;
    hour
}

/// Installs the sky material, its shader and the two systems.
pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/sky.wgsl");
        app.init_resource::<TableLight>()
            .add_plugins(MaterialPlugin::<SkyMaterial>::default());
    }
}
