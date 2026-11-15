//! Applies this device's graphics settings (`baylee_client_core::graphics`):
//! how often a frame is drawn, whether it waits for the display, how the
//! table's edges are smoothed and how much the ambient surfaces do — and
//! draws the knobs on the settings screen, with the sound knobs beside them.
//!
//! The decisions are client-core's; this module only reads the window and
//! the settings and hands bevy what they decided. What each knob costs is in
//! `docs/perf-client.md`.
//!
//! # Pacing
//!
//! While active and focused, the client runs at its preset's full rate (60 fps
//! on `Medium`, 120 fps on `High`, unlimited on `Ultra`). When `VSync` is
//! active and the display's refresh rate is at or below the target cap (60 Hz
//! at `Fps60`, 120 Hz `ProMotion` at `Fps120`), `UpdateMode::Continuous` hands
//! pacing directly to hardware `VBlank` (`PresentMode::Fifo` / `AutoVsync`)
//! with zero OS timer beat or dropped ticks; sub-refresh caps use exact
//! `VBlank` multiples in nanoseconds. After `120.0` seconds without user or game
//! activity, menus ease smoothly to 30 fps (and tables to `RestLimit`), waking
//! back to the full active rate immediately on any input or game event. While
//! visible — explicitly including when unfocused — the frame rate never drops
//! below 30 fps. Only when minimised or completely occluded (`WindowOccluded`)
//! does the window pause at 0 fps (`UpdateMode::reactive_low_power(Duration::MAX)`).

use baylee_client_core::graphics::{
    AntiAliasing, DISPLAY_REVERT_SECS, DisplayMode, Graphics, Pace, Showing, VSync,
};
use bevy::anti_alias::fxaa::Fxaa;
use bevy::input::gestures::{DoubleTapGesture, PanGesture, PinchGesture, RotationGesture};
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseButtonInput, MouseMotion, MouseWheel};
use bevy::input::touch::TouchInput;
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::view::Msaa;
use bevy::window::{
    CursorMoved, Monitor, OnMonitor, PresentMode, PrimaryMonitor, PrimaryWindow, WindowOccluded,
};
use bevy::winit::{UpdateMode, WinitSettings};
use std::time::Duration;

use crate::settings::ClientSettings;

/// The graphics settings in force this frame: the device's choice, or the
/// preset its GPU picked when it never chose.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InUse(pub Graphics);

impl Default for InUse {
    fn default() -> Self {
        Self(Graphics::of(Graphics::auto_preset("", MOBILE)))
    }
}

/// Whether ambient clocks pause (`Graphics::rests`): when hidden (minimised or
/// occluded), or on an idle menu with ambient motion already disabled.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resting(pub bool);

/// What the pacer watches: when anything was last touched, and whether the
/// window can be seen.
#[derive(Resource, Default, Debug)]
pub struct Watch {
    last_input: f64,
    hidden: bool,
    was_hidden: bool,
    /// The front door's passage as last seen: a passage on the move counts
    /// as activity, so a flight that outlasts the settle time is not drawn
    /// at the settled rate.
    scene: Option<(crate::vista::Stage, bool, f32)>,
    /// The game at the table as last seen: a change is activity.
    game: Option<TableMotion>,
}

/// A phone: never `Continuous` (iOS stops asking for frames, see
/// `standalone::run`), and its own starting preset.
const MOBILE: bool = cfg!(any(target_os = "android", target_os = "ios"));

/// Installs the pacer and the knobs' effects.
pub struct QualityPlugin;

impl Plugin for QualityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InUse>()
            .init_resource::<Watch>()
            .init_resource::<Resting>()
            .init_resource::<DisplayTrial>()
            .add_systems(
                Update,
                (try_display_mode, apply_display_mode, show_frame_rate)
                    .chain()
                    .after(resolve),
            )
            .add_systems(
                PreUpdate,
                (note_input, note_hidden, note_motion).after(bevy::input::InputSystems),
            )
            .add_systems(
                Update,
                (resolve, (apply_present_mode, apply_anti_aliasing)).chain(),
            )
            .add_systems(Last, pace);
    }

    /// Every main-world schedule runs on one thread.
    ///
    /// This client's systems are hundreds of small ones, most of them an
    /// early return, and the multi-threaded executor paid more in waking and
    /// parking its pool than it won in parallelism. Measured on the M1 Max at
    /// a busy duel (46 permanents, 60 frames, `/executor` A/B in one
    /// process): process CPU 66 % -> 56 %, energy impact 62 -> 52, and the
    /// main schedules' own time per frame 1.9 -> 1.3 ms. Rendering keeps its
    /// own pipelined thread and the task pools stay for async work. Here, in
    /// `finish`, because every schedule exists only once all plugins have
    /// added their systems.
    fn finish(&self, app: &mut App) {
        let mut schedules = app.world_mut().resource_mut::<Schedules>();
        for (_, schedule) in schedules.iter_mut() {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        }
        // Extraction too, for the same reason: it runs on the main thread
        // between two frames and is mostly copying, and the multi-threaded
        // executor spawned a task per extract system every frame — on a
        // duel at rest about a third of the main thread's allocations, and
        // process CPU 39 % -> 35 % (`docs/perf-baseline.md`, 08.10.2026).
        // The render schedule itself keeps its own executor: some of its
        // systems must reach the window on the main thread, which only the
        // multi-threaded executor arranges (a single-threaded one runs them
        // on the render thread and AppKit aborts).
        if let Some(render) = app.get_sub_app_mut(bevy::render::RenderApp)
            && let Some(mut schedules) = render.world_mut().get_resource_mut::<Schedules>()
            && let Some(extract) = schedules.get_mut(bevy::render::ExtractSchedule)
        {
            extract.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        }
    }
}

/// A display mode on trial (S4-7): the mode it replaced and the seconds
/// left before it is put back, unless Keep is pressed. The settings screen
/// starts it by changing the mode; nothing else does.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct DisplayTrial {
    /// The mode to go back to, while a trial runs.
    pub previous: Option<DisplayMode>,
    /// Seconds left.
    pub left: f32,
    /// The mode being tried, as last seen (a change starts a trial).
    seen: Option<DisplayMode>,
}

impl DisplayTrial {
    /// Keep the mode on trial.
    pub fn keep(&mut self) {
        self.previous = None;
    }

    /// The whole seconds left, for the question's countdown.
    #[must_use]
    pub fn seconds(&self) -> u32 {
        // In 0..=15 by construction.
        self.left.max(0.0).ceil() as u32
    }
}

/// Starts a trial when the chosen mode changes, and puts the old one back
/// when the trial runs out unkept.
fn try_display_mode(
    time: Res<Time<Real>>,
    mut trial: ResMut<DisplayTrial>,
    settings: Option<ResMut<ClientSettings>>,
) {
    let Some(mut settings) = settings else {
        return;
    };
    let now = settings
        .graphics
        .map_or(DisplayMode::Windowed, |g| g.display_mode);
    match trial.seen {
        None => trial.seen = Some(now),
        Some(was) if was != now => {
            trial.seen = Some(now);
            if trial.previous.is_none() {
                trial.previous = Some(was);
            }
            trial.left = DISPLAY_REVERT_SECS;
        }
        Some(_) => {}
    }
    let Some(previous) = trial.previous else {
        return;
    };
    trial.left -= time.delta_secs();
    if trial.left <= 0.0 {
        trial.previous = None;
        trial.seen = Some(previous);
        if let Some(graphics) = settings.graphics.as_mut() {
            graphics.display_mode = previous;
        }
        settings.save();
    }
}

/// The window mode the setting asks for (desktop builds; a browser and a
/// phone own their window).
#[must_use]
pub fn window_mode(mode: DisplayMode) -> bevy::window::WindowMode {
    use bevy::window::{MonitorSelection, VideoModeSelection, WindowMode};
    match mode {
        DisplayMode::Windowed => WindowMode::Windowed,
        DisplayMode::Borderless => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        DisplayMode::Fullscreen => {
            WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
        }
    }
}

fn apply_display_mode(in_use: Res<InUse>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if !DESKTOP_WINDOW || !in_use.is_changed() {
        return;
    }
    let want = window_mode(in_use.0.display_mode);
    for mut window in &mut windows {
        if window.mode != want {
            window.mode = want;
        }
    }
}

/// Whether this build owns its window's mode (a desktop's).
pub const DESKTOP_WINDOW: bool = !cfg!(any(
    target_arch = "wasm32",
    target_os = "android",
    target_os = "ios"
));

/// The frame-rate counter in the top-left corner (Show frame rate).
#[derive(Component)]
pub struct FrameRateCounter;

/// Shows the counter while the setting asks for it: the frame time and the
/// rate, averaged over about half a second.
fn show_frame_rate(
    mut commands: Commands,
    time: Res<Time<Real>>,
    in_use: Res<InUse>,
    fonts: Option<Res<crate::hud::UiFonts>>,
    mut counters: Query<(Entity, &mut Text, &mut Node), With<FrameRateCounter>>,
    mut average: Local<(f32, f32)>,
    tiles: Query<(
        &crate::lobby::front::door::CornerTile,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
) {
    let want = in_use.0.show_frame_rate;
    if !want {
        for (entity, ..) in &counters {
            commands.entity(entity).try_despawn();
        }
        return;
    }
    let dt = time.delta_secs().max(1e-4);
    let (mean, since) = &mut *average;
    *mean = if *mean == 0.0 {
        dt
    } else {
        *mean * 0.9 + dt * 0.1
    };
    *since += dt;
    let said = format!("{:.1} ms \u{b7} {:.0} fps", *mean * 1000.0, 1.0 / *mean);
    // Under the front door's top-left tile, never on it (beta.6: the
    // counter sat on the source tile).
    let top = tiles
        .iter()
        .filter(|(tile, ..)| tile.corner == crate::lobby::front::door::Corner::Left)
        .map(|(_, node, at)| frame_rate_top(node, at))
        .fold(4.0_f32, f32::max);
    if let Ok((_, mut text, mut node)) = counters.single_mut() {
        if node.top != Val::Px(top) {
            node.top = Val::Px(top);
        }
        if *since >= 0.5 && text.0 != said {
            *since = 0.0;
            text.0 = said;
        }
        return;
    }
    let Some(fonts) = fonts else {
        return;
    };
    commands.spawn((
        FrameRateCounter,
        Text::new(said),
        crate::hud::tf(&fonts, 11.0),
        TextColor(crate::hud::palette::INK),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(4.0),
            top: Val::Px(top),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
            ..default()
        },
        GlobalZIndex(crate::shellkit::tokens::z::VEIL + 5),
        Pickable::IGNORE,
    ));
}

/// The counter's top below a tile: the tile's bottom edge in logical
/// pixels, plus a little air.
fn frame_rate_top(node: &ComputedNode, at: &UiGlobalTransform) -> f32 {
    let scale = node.inverse_scale_factor;
    at.translation.y * scale + node.size().y * scale / 2.0 + 4.0
}

/// Whether ambient surfaces stand still: the player asked for no motion, or
/// the device's ambient effects are at `Low`. Gameplay motion reads only the
/// first; the front door's world, the cloth and the sky read this.
#[must_use]
pub fn ambient_still(reduce_motion: bool, in_use: Option<&InUse>) -> bool {
    reduce_motion || in_use.is_some_and(|q| !q.0.effects.ambient_motion())
}

/// The detail flag the ambient shaders branch on (`Effects::detail`).
#[must_use]
pub fn ambient_detail(in_use: Option<&InUse>) -> f32 {
    in_use.map_or_else(
        || InUse::default().0.effects.detail(),
        |q| q.0.effects.detail(),
    )
}

/// Whether the primary window has the focus; `true` without one (a test, an
/// embedding host), which keeps everything audible and paced as in front.
pub fn focused(windows: &Query<&Window, With<PrimaryWindow>>) -> bool {
    windows.single().map_or(true, |w| w.focused)
}

#[allow(clippy::too_many_arguments)] // Bevy system: every input stream that wakes an idle window
fn note_input(
    time: Res<Time<Real>>,
    mut watch: ResMut<Watch>,
    mut keys: MessageReader<KeyboardInput>,
    mut buttons: MessageReader<MouseButtonInput>,
    mut moves: MessageReader<CursorMoved>,
    mut motions: MessageReader<MouseMotion>,
    mut wheels: MessageReader<MouseWheel>,
    mut touches: MessageReader<TouchInput>,
    mut gestures: (
        MessageReader<PinchGesture>,
        MessageReader<RotationGesture>,
        MessageReader<DoubleTapGesture>,
        MessageReader<PanGesture>,
    ),
) {
    let touched = keys.read().count()
        + buttons.read().count()
        + moves.read().count()
        + motions.read().count()
        + wheels.read().count()
        + touches.read().count()
        + gestures.0.read().count()
        + gestures.1.read().count()
        + gestures.2.read().count()
        + gestures.3.read().count();
    if touched > 0 {
        watch.last_input = time.elapsed_secs_f64();
    }
}

/// A screen changing, the front door's passage moving, or the game at the
/// table moving (a new view or log line, a tear running) is
/// activity: the settle clock starts again, as if the player had touched
/// something. That is what brings a table at rest back to the full rate for
/// the cards an opponent's move deals.
fn note_motion(
    time: Res<Time<Real>>,
    mut watch: ResMut<Watch>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    front: Option<Res<crate::vista::FrontScene>>,
    duel: Option<Res<crate::Duel>>,
) {
    let changed_screen = phase.is_some_and(|p| p.is_changed());
    let scene = front.map(|f| (f.stage, f.entering, f.portal));
    let moving = scene.is_some() && scene != watch.scene;
    watch.scene = scene;
    let game = duel.as_deref().map(table_motion);
    let playing = game.is_some_and(|g| g.tearing) || game != watch.game;
    watch.game = game;
    if changed_screen || moving || playing {
        watch.last_input = time.elapsed_secs_f64();
    }
}

/// What of the game at the table tells the pacer something happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TableMotion {
    /// The view's snapshot.
    seq: Option<u64>,
    /// How many log lines have arrived (a frame may carry lines and repeat
    /// the view).
    log: usize,
    /// A tear is running (`Duel::tear`): it moves for as long as it lasts.
    tearing: bool,
}

fn table_motion(duel: &crate::Duel) -> TableMotion {
    TableMotion {
        seq: duel.view.as_ref().map(|v| v.seq),
        log: duel.log.entries().len(),
        tearing: duel.tear.is_some(),
    }
}

/// Whether the primary window is minimised or hidden: checks OS-level
/// `winit::Window::is_minimized` on native targets alongside `!window.visible`
/// and `0 × 0` physical resolution (Windows resizes minimised windows to `0 × 0`
/// without sending `WindowOccluded`).
fn window_minimised(entity: Entity, window: &Window) -> bool {
    if !window.visible
        || window.resolution.physical_width() == 0
        || window.resolution.physical_height() == 0
    {
        return true;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        bevy::winit::WINIT_WINDOWS.with_borrow(|ww| {
            ww.get_window(entity)
                .and_then(|win| win.is_minimized())
                .unwrap_or(false)
        })
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = entity;
        false
    }
}

fn note_hidden(mut watch: ResMut<Watch>, mut occluded: MessageReader<WindowOccluded>) {
    if let Some(last) = occluded.read().last() {
        watch.hidden = last.occluded && !watched_from_behind();
    }
}

/// A dev-control window opened passive (`BAYLEE_DEV_PASSIVE=1`) is *meant* to
/// stand behind everything else while a harness drives it, so being covered
/// is not being hidden: at 0 fps a click or screenshot would never advance.
/// Never in a shipped build.
fn watched_from_behind() -> bool {
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    {
        crate::devctl::passive()
    }
    #[cfg(not(all(feature = "dev-control", not(target_arch = "wasm32"))))]
    {
        false
    }
}

/// The settings in force: the device's choice, else its GPU's preset.
fn resolve(
    settings: Option<Res<ClientSettings>>,
    adapter: Option<Res<RenderAdapterInfo>>,
    mut in_use: ResMut<InUse>,
) {
    let chosen = settings.as_deref().and_then(|s| s.graphics);
    let want = chosen.unwrap_or_else(|| {
        let name = adapter.as_deref().map_or("", |info| info.name.as_str());
        Graphics::of(Graphics::auto_preset(name, MOBILE))
    });
    in_use.set_if_neq(InUse(want));
}

/// The present mode `vsync` asks for. Each falls back by itself where the
/// platform lacks it (a browser has only `Fifo`).
#[must_use]
pub const fn present_mode(vsync: VSync) -> PresentMode {
    match vsync {
        VSync::On => PresentMode::Fifo,
        VSync::Adaptive => PresentMode::AutoVsync,
        VSync::Off => PresentMode::AutoNoVsync,
    }
}

fn apply_present_mode(
    in_use: Res<InUse>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    added: Query<(), Added<PrimaryWindow>>,
) {
    if !in_use.is_changed() && added.is_empty() {
        return;
    }
    let want = present_mode(in_use.0.vsync);
    for mut window in &mut windows {
        if window.present_mode != want {
            window.present_mode = want;
        }
    }
}

/// The table camera's multisampling and FXAA, whenever the setting changes
/// or a table camera appears. A driver that cannot multisample keeps `Off`
/// whatever is asked (`crate::gpu::msaa`).
#[allow(clippy::type_complexity)]
fn apply_anti_aliasing(
    mut commands: Commands,
    in_use: Res<InUse>,
    adapter: Option<Res<RenderAdapterInfo>>,
    mut cameras: Query<(Entity, &mut Msaa, Has<Fxaa>), With<Camera3d>>,
    added: Query<(), Added<Camera3d>>,
) {
    if !in_use.is_changed() && added.is_empty() {
        return;
    }
    let aa = in_use.0.anti_aliasing;
    let able = crate::gpu::msaa(adapter.as_deref()) != Msaa::Off;
    let want = match aa.samples() {
        _ if !able => Msaa::Off,
        2 => Msaa::Sample2,
        4 => Msaa::Sample4,
        _ => Msaa::Off,
    };
    for (entity, mut msaa, has_fxaa) in &mut cameras {
        if *msaa != want {
            *msaa = want;
        }
        match (aa == AntiAliasing::Fxaa, has_fxaa) {
            (true, false) => {
                commands.entity(entity).try_insert(Fxaa::default());
            }
            (false, true) => {
                commands.entity(entity).try_remove::<Fxaa>();
            }
            _ => {}
        }
    }
}

/// Exact wait interval for `fps`, snapped to an integer multiple of the
/// display's `VBlank` period when `monitor_mhz` divides cleanly.
fn frame_wait(fps: u32, monitor_mhz: Option<u32>) -> Duration {
    let fps = u64::from(fps.max(1));
    if let Some(mhz) = monitor_mhz.filter(|&m| m > 0) {
        let mhz = u64::from(mhz);
        let target_mhz = fps * 1000;
        let div = (mhz + target_mhz / 2) / target_mhz;
        if div >= 2 {
            let approx = div * target_mhz;
            let diff = approx.abs_diff(mhz);
            // Within 2% of an exact VBlank harmonic (e.g. 60/30 on 120 Hz or
            // 59.94/119.88 Hz): lock to exact `div` hardware VBlanks.
            if diff * 50 <= mhz {
                return Duration::from_nanos(div * 1_000_000_000_000 / mhz);
            }
        }
    }
    Duration::from_nanos(1_000_000_000 / fps)
}

/// Display-aware update mode:
/// - [`Pace::Paused`]: `UpdateMode::reactive_low_power(Duration::MAX)` (0 fps
///   while minimised or occluded; wakes immediately on window restore/unocclude).
/// - [`Pace::Unlimited`]: `UpdateMode::Continuous` on desktop.
/// - [`Pace::Fps`]: when `VSync` is enabled on desktop and the display refresh
///   rate is at or below `fps` (60 Hz at `Fps60`, 120 Hz `ProMotion` at `Fps120`),
///   uses `UpdateMode::Continuous` so `CAMetalLayer` / hardware `VBlank` paces
///   every frame without `CFRunLoopTimer` beat or dropped ticks; otherwise uses
///   `UpdateMode::Reactive` with exact nanosecond / `VBlank`-harmonic intervals.
#[must_use]
pub fn update_mode_for_display(
    pace: Pace,
    waking: bool,
    vsync: VSync,
    monitor_mhz: Option<u32>,
) -> UpdateMode {
    match pace {
        Pace::Paused => UpdateMode::reactive_low_power(Duration::MAX),
        Pace::Unlimited if !MOBILE => UpdateMode::Continuous,
        Pace::Unlimited => UpdateMode::reactive(Duration::from_nanos(8_333_333)),
        Pace::Fps(fps) => {
            let vsync_caps = !MOBILE
                && vsync != VSync::Off
                && monitor_mhz
                    .filter(|&m| m > 0)
                    .is_some_and(|mhz| u64::from(fps) * 1000 + 500 >= u64::from(mhz));
            if vsync_caps {
                UpdateMode::Continuous
            } else {
                UpdateMode::Reactive {
                    wait: frame_wait(fps, monitor_mhz),
                    react_to_device_events: false,
                    react_to_user_events: true,
                    react_to_window_events: waking,
                }
            }
        }
    }
}

/// The update mode for a pace without monitor refresh-rate metadata.
#[must_use]
pub fn update_mode(pace: Pace, waking: bool) -> UpdateMode {
    update_mode_for_display(pace, waking, VSync::On, None)
}

fn current_monitor_mhz(
    on_monitor: Option<&OnMonitor>,
    monitors: &Query<(&Monitor, Has<PrimaryMonitor>)>,
) -> Option<u32> {
    if let Some(on) = on_monitor
        && let Ok((monitor, _)) = monitors.get(on.0)
        && let Some(mhz) = monitor.refresh_rate_millihertz
    {
        return Some(mhz);
    }
    monitors
        .iter()
        .find_map(|(m, primary)| primary.then_some(m.refresh_rate_millihertz).flatten())
        .or_else(|| monitors.iter().find_map(|(m, _)| m.refresh_rate_millihertz))
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)] // Bevy system: the clocks, the window and the settings it reads
fn pace(
    time: Res<Time<Real>>,
    mut watch: ResMut<Watch>,
    in_use: Res<InUse>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    windows: Query<(Entity, &Window, Option<&OnMonitor>), With<PrimaryWindow>>,
    monitors: Query<(&Monitor, Has<PrimaryMonitor>)>,
    winit: Option<ResMut<WinitSettings>>,
    film: Option<Res<crate::vista::Moving>>,
    mut resting: ResMut<Resting>,
) {
    let passive = watched_from_behind();
    let menu = phase.is_none_or(|p| *p.get() == crate::DuelPhase::Closed);
    let still = ambient_still(prefs.is_some_and(|p| p.all().reduce_motion), Some(&in_use));
    let (is_focused, is_min, monitor_mhz) = match windows.single() {
        Ok((entity, w, on_mon)) => (
            w.focused,
            window_minimised(entity, w),
            current_monitor_mhz(on_mon, &monitors),
        ),
        Err(_) => (true, false, current_monitor_mhz(None, &monitors)),
    };
    let hidden = !passive && (watch.hidden || is_min);
    if watch.was_hidden != hidden {
        let waking_from_hidden = watch.was_hidden && !hidden;
        watch.was_hidden = hidden;
        if waking_from_hidden {
            watch.last_input = time.elapsed_secs_f64();
        }
    }
    let untouched_secs = if passive {
        0.0
    } else {
        (time.elapsed_secs_f64() - watch.last_input).max(0.0) as f32
    };
    let showing = Showing {
        focused: is_focused || passive,
        hidden,
        menu,
        untouched_secs,
        still,
        picture: film.is_some_and(|film| film.0),
    };
    resting.set_if_neq(Resting(in_use.0.rests(showing)));
    let Some(mut winit) = winit else {
        return;
    };
    let (pace, waking) = in_use.0.pace(showing);
    let mode = update_mode_for_display(pace, waking, in_use.0.vsync, monitor_mhz);
    if winit.focused_mode != mode || winit.unfocused_mode != mode {
        winit.focused_mode = mode;
        winit.unfocused_mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Focused caps, reduced paces, `VBlank`-locked `Continuous` modes on 60 Hz
    /// and 120 Hz `ProMotion`, and `Duration::MAX` 0-fps pause when hidden.
    #[test]
    fn a_pace_becomes_an_update_mode() {
        match update_mode(Pace::Fps(60), false) {
            UpdateMode::Reactive {
                wait,
                react_to_window_events,
                react_to_device_events,
                ..
            } => {
                assert_eq!(wait, Duration::from_nanos(16_666_666));
                assert!(!react_to_window_events && !react_to_device_events);
            }
            UpdateMode::Continuous => panic!("headless cap without monitor is reactive"),
        }
        assert!(matches!(
            update_mode(Pace::Fps(30), true),
            UpdateMode::Reactive {
                react_to_window_events: true,
                ..
            }
        ));
        assert_eq!(
            update_mode(Pace::Paused, true),
            UpdateMode::reactive_low_power(Duration::MAX)
        );
        if !MOBILE {
            assert_eq!(update_mode(Pace::Unlimited, false), UpdateMode::Continuous);
            // 60 fps on 60 Hz / 59.94 Hz and 120 fps on 120 Hz ProMotion use
            // VSync-locked `Continuous` to avoid `CFRunLoopTimer` beat judder.
            assert_eq!(
                update_mode_for_display(Pace::Fps(60), false, VSync::On, Some(60_000)),
                UpdateMode::Continuous
            );
            assert_eq!(
                update_mode_for_display(Pace::Fps(60), false, VSync::On, Some(59_940)),
                UpdateMode::Continuous
            );
            assert_eq!(
                update_mode_for_display(Pace::Fps(120), false, VSync::On, Some(120_000)),
                UpdateMode::Continuous
            );
            // With VSync off, explicit caps stay timer-enforced instead of spinning.
            assert!(matches!(
                update_mode_for_display(Pace::Fps(60), false, VSync::Off, Some(60_000)),
                UpdateMode::Reactive { .. }
            ));
            // 60 fps and 30 fps on 120 Hz snap to exact 2- and 4-VBlank multiples.
            match update_mode_for_display(Pace::Fps(60), false, VSync::On, Some(120_000)) {
                UpdateMode::Reactive { wait, .. } => {
                    assert_eq!(wait, Duration::from_nanos(16_666_666));
                }
                UpdateMode::Continuous => panic!("60 on 120 Hz caps every 2 VBlanks"),
            }
            match update_mode_for_display(Pace::Fps(30), true, VSync::On, Some(120_000)) {
                UpdateMode::Reactive { wait, .. } => {
                    assert_eq!(wait, Duration::from_nanos(33_333_333));
                }
                UpdateMode::Continuous => panic!("30 on 120 Hz caps every 4 VBlanks"),
            }
        }
    }

    /// Focused active window runs at 60 fps; unfocused visible window stays at
    /// `>= 30` fps (never 15 fps!); occluded window pauses at 0 fps
    /// (`Duration::MAX`) and wakes back to 60 fps when un-occluded.
    #[test]
    fn the_pacer_writes_the_winit_settings() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::input::InputPlugin)
            .add_message::<CursorMoved>()
            .add_message::<WindowOccluded>()
            .insert_resource(WinitSettings::game())
            .insert_resource(ClientSettings::default())
            .add_plugins(QualityPlugin);
        let win = app
            .world_mut()
            .spawn((
                Window {
                    focused: true,
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        app.update();
        let wait = |app: &App| match app.world().resource::<WinitSettings>().focused_mode {
            UpdateMode::Reactive { wait, .. } => Some(wait),
            UpdateMode::Continuous => None,
        };
        assert_eq!(wait(&app), Some(Duration::from_nanos(16_666_666)));
        assert_eq!(*app.world().resource::<Resting>(), Resting(false));

        // Unfocused but visible: hard >= 30 fps floor (`33_333_333 ns`).
        app.world_mut()
            .entity_mut(win)
            .get_mut::<Window>()
            .unwrap()
            .focused = false;
        app.update();
        assert_eq!(wait(&app), Some(Duration::from_nanos(33_333_333)));
        assert_eq!(*app.world().resource::<Resting>(), Resting(false));

        // Truly occluded: 0 fps (`Duration::MAX`) + `Resting(true)`.
        app.world_mut().write_message(WindowOccluded {
            window: win,
            occluded: true,
        });
        app.update();
        assert_eq!(wait(&app), Some(Duration::MAX));
        assert_eq!(*app.world().resource::<Resting>(), Resting(true));

        // Visible and focused again: immediately back to 60 fps + `Resting(false)`.
        app.world_mut()
            .entity_mut(win)
            .get_mut::<Window>()
            .unwrap()
            .focused = true;
        app.world_mut().write_message(WindowOccluded {
            window: win,
            occluded: false,
        });
        app.update();
        assert_eq!(wait(&app), Some(Duration::from_nanos(16_666_666)));
        assert_eq!(*app.world().resource::<Resting>(), Resting(false));
    }

    /// A minimised window (`0 × 0` physical pixels or `!visible`) pauses at
    /// 0 fps (`Duration::MAX`) and resumes at 60 fps when restored.
    #[test]
    fn a_minimised_window_draws_no_frames() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::input::InputPlugin)
            .add_message::<CursorMoved>()
            .add_message::<WindowOccluded>()
            .insert_resource(WinitSettings::game())
            .insert_resource(ClientSettings::default())
            .add_plugins(QualityPlugin);
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        let wait = |app: &App| match app.world().resource::<WinitSettings>().focused_mode {
            UpdateMode::Reactive { wait, .. } => wait,
            UpdateMode::Continuous => Duration::ZERO,
        };
        let resize = |app: &mut App, width: u32, height: u32| {
            let mut windows = app.world_mut().query::<&mut Window>();
            windows
                .single_mut(app.world_mut())
                .expect("one")
                .resolution
                .set_physical_resolution(width, height);
        };
        resize(&mut app, 0, 0);
        app.update();
        assert_eq!(wait(&app), Duration::MAX);
        assert_eq!(*app.world().resource::<Resting>(), Resting(true));

        resize(&mut app, 1280, 720);
        app.update();
        assert_eq!(wait(&app), Duration::from_nanos(16_666_666));
        assert_eq!(*app.world().resource::<Resting>(), Resting(false));
    }

    /// Menu pacing policy: 60 fps for a full 120.0 seconds, smooth drop to
    /// 30 fps at 120.0 seconds (never freezing ambient motion), and immediate
    /// return to 60 fps on the very next frame after any user input.
    #[test]
    fn a_menu_holds_sixty_for_two_minutes_eases_to_thirty_and_wakes_on_input() {
        use baylee_client_core::graphics::IDLE_AFTER_SECS;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::input::InputPlugin)
            .add_message::<CursorMoved>()
            .add_message::<WindowOccluded>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs(30),
            ))
            .insert_resource(WinitSettings::game())
            .insert_resource(ClientSettings::default())
            .add_plugins(QualityPlugin);
        let win = app
            .world_mut()
            .spawn((
                Window {
                    focused: true,
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        // First update initializes `Time<Real>` (elapsed = 0s): 60 fps.
        app.update();
        assert!((focused_wait(&app) - 1.0 / 60.0).abs() < 1e-4);
        // +30s, +60s, +90s (< 120.0s): still 60 fps!
        for step in 1..=3 {
            app.update();
            assert!(
                (focused_wait(&app) - 1.0 / 60.0).abs() < 1e-4,
                "at {}s (< {IDLE_AFTER_SECS}s): {}",
                step * 30,
                focused_wait(&app)
            );
        }
        // +120s (`IDLE_AFTER_SECS`): eases smoothly to 30 fps without resting!
        app.update();
        assert!((focused_wait(&app) - 1.0 / 30.0).abs() < 1e-4);
        assert_eq!(*app.world().resource::<Resting>(), Resting(false));
        // Any user action (`CursorMoved`) returns immediately to 60 fps!
        app.world_mut().write_message(CursorMoved {
            window: win,
            position: Vec2::new(42.0, 24.0),
            delta: Some(Vec2::ONE),
        });
        app.update();
        assert!((focused_wait(&app) - 1.0 / 60.0).abs() < 1e-4);
    }

    /// A table under `settings`, a duel on it, its clock stepping a quarter
    /// of the settle time a frame.
    fn table_app(settings: ClientSettings) -> App {
        use baylee_client_core::graphics::TABLE_SETTLE_SECS;
        use baylee_client_core::test_support::ViewBuilder;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .add_plugins(bevy::input::InputPlugin)
            .add_message::<CursorMoved>()
            .add_message::<WindowOccluded>()
            .init_state::<crate::DuelPhase>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs_f32(TABLE_SETTLE_SECS / 4.0),
            ))
            .insert_resource(WinitSettings::game())
            .insert_resource(settings)
            .insert_resource(crate::Duel {
                view: Some(ViewBuilder::new(2).build()),
                ..default()
            })
            .add_plugins(QualityPlugin);
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
        ));
        app.world_mut()
            .resource_mut::<NextState<crate::DuelPhase>>()
            .set(crate::DuelPhase::Playing);
        app
    }

    /// The focused update mode's wait, in seconds (0 for continuous).
    fn focused_wait(app: &App) -> f32 {
        match app.world().resource::<WinitSettings>().focused_mode {
            UpdateMode::Reactive { wait, .. } => wait.as_secs_f32(),
            UpdateMode::Continuous => 0.0,
        }
    }

    /// At "Bildrate in Ruhe" 60 under Medium's cap of 60 the table never
    /// eases down, untouched for as long as it likes, and input does not
    /// wake it — the pacing from before the setting. Red if the setting is
    /// ignored: the default rests at thirty.
    #[test]
    fn at_a_rest_rate_of_sixty_the_table_never_eases_down() {
        use baylee_client_core::graphics::{Graphics, Preset, RestLimit};
        let mut graphics = Graphics::of(Preset::Medium);
        graphics.adjust(|g| g.rest_limit = RestLimit::Fps60);
        let mut app = table_app(ClientSettings {
            graphics: Some(graphics),
            ..default()
        });
        for frame in 0..40 {
            app.update();
            assert!(
                (focused_wait(&app) - 1.0 / 60.0).abs() < 1e-4,
                "frame {frame}: eased to {}",
                focused_wait(&app)
            );
            assert!(matches!(
                app.world().resource::<WinitSettings>().focused_mode,
                UpdateMode::Reactive {
                    react_to_window_events: false,
                    ..
                }
            ));
        }
    }

    /// A table nobody touches and where nothing happens comes to rest at
    /// thirty frames; the game moving — a new view, a tear — brings the full
    /// rate back on the next frame, untouched as the window still is.
    /// Proved both ways: without the new view the table stays at rest.
    #[test]
    fn a_table_rests_until_its_game_moves() {
        use baylee_client_core::graphics::RestLimit;
        let mut app = table_app(ClientSettings::default());
        let wait = focused_wait;
        let full = 1.0 / 60.0;
        #[allow(clippy::cast_precision_loss)]
        let rest = 1.0 / RestLimit::Fps30.fps() as f32;
        app.update();
        assert!((wait(&app) - full).abs() < 1e-4, "a table just opened");
        for _ in 0..6 {
            app.update();
        }
        assert!((wait(&app) - rest).abs() < 1e-4, "at rest");
        // Nothing happens: still at rest.
        app.update();
        assert!((wait(&app) - rest).abs() < 1e-4, "still at rest");
        // The opponent moves: a new view arrives.
        app.world_mut()
            .resource_mut::<crate::Duel>()
            .view
            .as_mut()
            .expect("a view")
            .seq += 1;
        app.update();
        assert!((wait(&app) - full).abs() < 1e-4, "the game moved");
        for _ in 0..6 {
            app.update();
        }
        assert!((wait(&app) - rest).abs() < 1e-4, "at rest again");
    }
}
