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
//! Bevy's default is to draw every frame the display offers while focused —
//! 120 a second on a 120 Hz laptop display — and 60 behind other windows. Every
//! one of those frames pays every shader again, which on an M1 was the fan
//! the owner heard. A frame limit here is a `Reactive` update mode with the
//! limit's interval as its wait: winit sleeps until the next frame is due
//! instead of spinning, and the cap holds whatever the pointer does (window
//! events do not wake it early while the window is in front). A table where
//! nothing has been touched and nothing has happened for two seconds eases to
//! thirty frames and stays there; a menu nobody has touched for two seconds
//! eases to thirty frames, after half a minute to
//! the background limit; behind other windows the background limit holds,
//! and a hidden window draws one frame a second. Every reduced pace wakes on
//! input at once (`baylee_client_core::graphics::Graphics::pace`).

use baylee_client_core::graphics::{
    AntiAliasing, DISPLAY_REVERT_SECS, DisplayMode, Graphics, Pace, Showing, VSync,
};
use bevy::anti_alias::fxaa::Fxaa;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::input::touch::TouchInput;
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::view::Msaa;
use bevy::window::{CursorMoved, PresentMode, PrimaryWindow, WindowOccluded};
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

/// Whether a menu has come to rest (`Graphics::rests`): its ambient world
/// holds still where it stands until something happens.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resting(pub bool);

/// What the pacer watches: when anything was last touched, and whether the
/// window can be seen.
#[derive(Resource, Default, Debug)]
pub struct Watch {
    last_input: f64,
    hidden: bool,
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
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let whole = self.left.max(0.0).ceil() as u32;
        whole
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
    mut counters: Query<(Entity, &mut Text), With<FrameRateCounter>>,
    mut average: Local<(f32, f32)>,
) {
    let want = in_use.0.show_frame_rate;
    if !want {
        for (entity, _) in &counters {
            commands.entity(entity).despawn();
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
    if let Ok((_, mut text)) = counters.single_mut() {
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
            top: Val::Px(4.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
            ..default()
        },
        GlobalZIndex(crate::shellkit::tokens::z::VEIL + 5),
        Pickable::IGNORE,
    ));
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

fn note_input(
    time: Res<Time<Real>>,
    mut watch: ResMut<Watch>,
    mut keys: MessageReader<KeyboardInput>,
    mut buttons: MessageReader<MouseButtonInput>,
    mut moves: MessageReader<CursorMoved>,
    mut wheels: MessageReader<MouseWheel>,
    mut touches: MessageReader<TouchInput>,
) {
    let touched = keys.read().count()
        + buttons.read().count()
        + moves.read().count()
        + wheels.read().count()
        + touches.read().count();
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

fn note_hidden(mut watch: ResMut<Watch>, mut occluded: MessageReader<WindowOccluded>) {
    if let Some(last) = occluded.read().last() {
        watch.hidden = last.occluded;
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
                commands.entity(entity).insert(Fxaa::default());
            }
            (false, true) => {
                commands.entity(entity).remove::<Fxaa>();
            }
            _ => {}
        }
    }
}

/// The update mode for a pace. `waking`: input wakes a frame at once,
/// which a background or idle pace wants and a focused cap must not.
#[must_use]
pub fn update_mode(pace: Pace, waking: bool) -> UpdateMode {
    match pace.interval_secs() {
        None if !MOBILE => UpdateMode::Continuous,
        // A phone's "no limit" is its display's own rate, asked for on a
        // timer rather than by spinning.
        None => UpdateMode::reactive(Duration::from_secs_f32(1.0 / 120.0)),
        Some(interval) => UpdateMode::Reactive {
            wait: Duration::from_secs_f32(interval),
            react_to_device_events: false,
            react_to_user_events: true,
            react_to_window_events: waking,
        },
    }
}

#[allow(clippy::too_many_arguments)] // Bevy system: the clocks, the window and the settings it reads
fn pace(
    time: Res<Time<Real>>,
    watch: Res<Watch>,
    in_use: Res<InUse>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    winit: Option<ResMut<WinitSettings>>,
    mut resting: ResMut<Resting>,
) {
    #[allow(clippy::cast_possible_truncation)] // seconds, far inside f32
    let untouched_secs = (time.elapsed_secs_f64() - watch.last_input) as f32;
    let showing = Showing {
        focused: focused(&windows),
        hidden: watch.hidden,
        menu: phase.is_none_or(|p| *p.get() == crate::DuelPhase::Closed),
        untouched_secs,
        still: ambient_still(prefs.is_some_and(|p| p.all().reduce_motion), Some(&in_use)),
    };
    resting.set_if_neq(Resting(in_use.0.rests(showing)));
    let Some(mut winit) = winit else {
        return;
    };
    let (pace, waking) = in_use.0.pace(showing);
    let mode = update_mode(pace, waking);
    if winit.focused_mode != mode || winit.unfocused_mode != mode {
        winit.focused_mode = mode;
        winit.unfocused_mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A focused cap holds against the pointer; a background or idle pace
    /// wakes on it; no limit is continuous on a desktop.
    #[test]
    fn a_pace_becomes_an_update_mode() {
        match update_mode(Pace::Fps(60), false) {
            UpdateMode::Reactive {
                wait,
                react_to_window_events,
                react_to_device_events,
                ..
            } => {
                assert!((wait.as_secs_f32() - 1.0 / 60.0).abs() < 1e-4);
                assert!(!react_to_window_events && !react_to_device_events);
            }
            UpdateMode::Continuous => panic!("a cap is reactive"),
        }
        assert!(matches!(
            update_mode(Pace::Fps(15), true),
            UpdateMode::Reactive {
                react_to_window_events: true,
                ..
            }
        ));
        if !MOBILE {
            assert_eq!(update_mode(Pace::Unlimited, false), UpdateMode::Continuous);
        }
    }

    /// The pacer in a running app: focused at the table it caps at the
    /// limit; behind another window it drops to the background rate; a
    /// hidden window draws one frame a second. Proved by removing the focus.
    #[test]
    fn the_pacer_writes_the_winit_settings() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::input::InputPlugin)
            .add_message::<CursorMoved>()
            .add_message::<WindowOccluded>()
            .add_message::<TouchInput>()
            .insert_resource(WinitSettings::game())
            .insert_resource(ClientSettings::default())
            .add_plugins(QualityPlugin);
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
        ));
        app.update();
        let wait = |app: &App| match app.world().resource::<WinitSettings>().focused_mode {
            UpdateMode::Reactive { wait, .. } => Some(wait.as_secs_f32()),
            UpdateMode::Continuous => None,
        };
        let medium = 1.0 / 60.0;
        // No `DuelPhase` state: the front door, but touched just now (the
        // clock starts at zero), so not idle.
        assert!((wait(&app).expect("capped") - medium).abs() < 1e-4);
        let mut windows = app.world_mut().query::<&mut Window>();
        windows.single_mut(app.world_mut()).expect("one").focused = false;
        app.update();
        assert!((wait(&app).expect("capped") - 1.0 / 15.0).abs() < 1e-4);
        app.world_mut().write_message(WindowOccluded {
            window: Entity::PLACEHOLDER,
            occluded: true,
        });
        app.update();
        assert!((wait(&app).expect("capped") - 1.0).abs() < 1e-4);
    }

    /// A table nobody touches and where nothing happens comes to rest at
    /// thirty frames; the game moving — a new view, a tear — brings the full
    /// rate back on the next frame, untouched as the window still is.
    /// Proved both ways: without the new view the table stays at rest.
    #[test]
    fn a_table_rests_until_its_game_moves() {
        use baylee_client_core::graphics::{TABLE_REST_FPS, TABLE_SETTLE_SECS};
        use baylee_client_core::test_support::ViewBuilder;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .add_plugins(bevy::input::InputPlugin)
            .add_message::<CursorMoved>()
            .add_message::<WindowOccluded>()
            .add_message::<TouchInput>()
            .init_state::<crate::DuelPhase>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs_f32(TABLE_SETTLE_SECS / 4.0),
            ))
            .insert_resource(WinitSettings::game())
            .insert_resource(ClientSettings::default())
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
        let wait = |app: &App| match app.world().resource::<WinitSettings>().focused_mode {
            UpdateMode::Reactive { wait, .. } => wait.as_secs_f32(),
            UpdateMode::Continuous => 0.0,
        };
        let full = 1.0 / 60.0;
        #[allow(clippy::cast_precision_loss)]
        let rest = 1.0 / TABLE_REST_FPS as f32;
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
