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
//! events do not wake it early while the window is in front). A menu nobody
//! has touched for two seconds eases to thirty frames, after half a minute to
//! the background limit; behind other windows the background limit holds,
//! and a hidden window draws one frame a second. Every reduced pace wakes on
//! input at once (`baylee_client_core::graphics::Graphics::pace`).

use baylee_client_core::graphics::{
    AntiAliasing, BackgroundLimit, Effects, FrameLimit, Graphics, Pace, Preset, Showing, VSync,
};
use baylee_client_core::i18n::{Lang, Phrase};
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
            .add_systems(
                PreUpdate,
                (note_input, note_hidden, note_motion).after(bevy::input::InputSystems),
            )
            .add_systems(
                Update,
                (
                    resolve,
                    (apply_present_mode, apply_anti_aliasing, show_knobs),
                )
                    .chain(),
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
    }
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

/// A screen changing, or the front door's passage moving, is activity: the
/// settle clock starts again, as if the player had touched something.
fn note_motion(
    time: Res<Time<Real>>,
    mut watch: ResMut<Watch>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    front: Option<Res<crate::vista::FrontScene>>,
) {
    let changed_screen = phase.is_some_and(|p| p.is_changed());
    let scene = front.map(|f| (f.stage, f.entering, f.portal));
    let moving = scene.is_some() && scene != watch.scene;
    watch.scene = scene;
    if changed_screen || moving {
        watch.last_input = time.elapsed_secs_f64();
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

// ---- the knobs on the settings screen

/// One knob a row cycles through.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Knob {
    Preset,
    AntiAliasing,
    VSync,
    FrameLimit,
    Background,
    Effects,
    Master(i8),
    GameSounds(i8),
    MuteInBackground,
}

/// The text that shows a knob's value.
#[derive(Component, Clone, Copy)]
struct Readout(Knob);

fn next<T: Copy + PartialEq>(all: &[T], now: T) -> T {
    let at = all.iter().position(|v| *v == now).unwrap_or(0);
    all[(at + 1) % all.len()]
}

/// What pressing `knob` does to `settings`.
pub(crate) fn turn(knob: Knob, settings: &mut ClientSettings, in_use: Graphics) {
    let mut graphics = settings.graphics.unwrap_or(in_use);
    match knob {
        Knob::Preset => {
            let preset = match graphics.preset {
                Preset::Custom => Preset::Low,
                now => next(&Preset::NAMED, now),
            };
            graphics = Graphics::of(preset);
        }
        Knob::AntiAliasing => {
            graphics.adjust(|g| g.anti_aliasing = next(&AntiAliasing::ALL, g.anti_aliasing));
        }
        Knob::VSync => graphics.adjust(|g| g.vsync = next(&VSync::ALL, g.vsync)),
        Knob::FrameLimit => {
            graphics.adjust(|g| g.frame_limit = next(&FrameLimit::ALL, g.frame_limit));
        }
        Knob::Background => {
            graphics.adjust(|g| {
                g.background_limit = next(&BackgroundLimit::ALL, g.background_limit);
            });
        }
        Knob::Effects => graphics.adjust(|g| g.effects = next(&Effects::ALL, g.effects)),
        Knob::Master(step) => {
            let volume = settings.audio.master() + f32::from(step) * 0.1;
            settings.audio.set_master(volume);
            return;
        }
        Knob::GameSounds(step) => {
            let volume = settings.audio.effects() + f32::from(step) * 0.1;
            settings.audio.set_effects(volume);
            return;
        }
        Knob::MuteInBackground => {
            settings.audio.mute_in_background = !settings.audio.mute_in_background;
            return;
        }
    }
    settings.graphics = Some(graphics);
}

fn quality_name(preset: Preset) -> Phrase {
    match preset {
        Preset::Low => Phrase::QualityLow,
        Preset::Medium => Phrase::QualityMedium,
        Preset::High => Phrase::QualityHigh,
        Preset::Ultra => Phrase::QualityUltra,
        Preset::Custom => Phrase::QualityCustom,
    }
}

/// What a knob's readout says.
pub(crate) fn reading(
    knob: Knob,
    settings: &ClientSettings,
    in_use: Graphics,
    lang: Lang,
) -> String {
    let g = settings.graphics.unwrap_or(in_use);
    let percent = |v: f32| format!("{:.0} %", v * 100.0);
    match knob {
        Knob::Preset => quality_name(g.preset).text(lang).to_string(),
        Knob::AntiAliasing => match g.anti_aliasing {
            AntiAliasing::Off => Phrase::SwitchOff.text(lang).to_string(),
            AntiAliasing::Fxaa => "FXAA".to_string(),
            AntiAliasing::Msaa2 => "MSAA 2×".to_string(),
            AntiAliasing::Msaa4 => "MSAA 4×".to_string(),
        },
        Knob::VSync => match g.vsync {
            VSync::On => Phrase::SwitchOn,
            VSync::Adaptive => Phrase::VSyncAdaptive,
            VSync::Off => Phrase::SwitchOff,
        }
        .text(lang)
        .to_string(),
        Knob::FrameLimit => g.frame_limit.fps().map_or_else(
            || Phrase::Unlimited.text(lang).to_string(),
            |fps| format!("{fps} fps"),
        ),
        Knob::Background => format!("{} fps", g.background_limit.fps()),
        Knob::Effects => match g.effects {
            Effects::Low => Phrase::QualityLow,
            Effects::Medium => Phrase::QualityMedium,
            Effects::High => Phrase::QualityHigh,
        }
        .text(lang)
        .to_string(),
        Knob::Master(_) => percent(settings.audio.master()),
        Knob::GameSounds(_) => percent(settings.audio.effects()),
        Knob::MuteInBackground => if settings.audio.mute_in_background {
            Phrase::SwitchOn
        } else {
            Phrase::SwitchOff
        }
        .text(lang)
        .to_string(),
    }
}

/// A heading and its rows, each row a label and the knobs it turns: one
/// knob is a button showing its value, two are a − and a + around it.
type Section = (Phrase, &'static [(Phrase, &'static [Knob])]);

/// The graphics and sound knobs, for the settings screen: a label and a
/// button per knob, the button cycling the value it shows. Kept to rows of
/// the existing widgets on purpose — the screen's own redesign binds to
/// [`Knob`], [`turn`] and [`reading`], not to this layout.
pub(crate) fn controls(
    commands: &mut Commands,
    fonts: &crate::hud::UiFonts,
    metrics: crate::lobby::Metrics,
    lang: Lang,
) -> Entity {
    use crate::hud::{palette, tf};
    use bevy::ui::{percent, px};
    let root = commands
        .spawn((
            Node {
                width: percent(100),
                max_width: px(420),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let sections: [Section; 2] = [
        (
            Phrase::Graphics,
            &[
                (Phrase::GraphicsPreset, &[Knob::Preset]),
                (Phrase::FrameLimit, &[Knob::FrameLimit]),
                (Phrase::BackgroundFrames, &[Knob::Background]),
                (Phrase::AmbientEffects, &[Knob::Effects]),
                (Phrase::AntiAliasing, &[Knob::AntiAliasing]),
                (Phrase::VSync, &[Knob::VSync]),
            ],
        ),
        (
            Phrase::Audio,
            &[
                (Phrase::MasterVolume, &[Knob::Master(-1), Knob::Master(1)]),
                (
                    Phrase::EffectsVolume,
                    &[Knob::GameSounds(-1), Knob::GameSounds(1)],
                ),
                (Phrase::MuteInBackground, &[Knob::MuteInBackground]),
            ],
        ),
    ];
    for (title, rows) in sections {
        let head = commands
            .spawn((
                Text::new(title.text(lang)),
                tf(fonts, metrics.text),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(root).add_child(head);
        for (label, knobs) in rows {
            let line = knob_row(commands, fonts, metrics, label.text(lang), knobs);
            commands.entity(root).add_child(line);
        }
    }
    root
}

/// One row: its label, and the button (or − value +) that turns its knob.
fn knob_row(
    commands: &mut Commands,
    fonts: &crate::hud::UiFonts,
    metrics: crate::lobby::Metrics,
    label: &str,
    knobs: &[Knob],
) -> Entity {
    use crate::hud::{palette, tf};
    use bevy::ui::px;
    let line = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = commands
        .spawn((
            Text::new(label.to_string()),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_child(name);
    let readout = |commands: &mut Commands, knob: Knob| {
        commands
            .spawn((
                Text::new(""),
                tf(fonts, metrics.text),
                TextColor(palette::INK),
                Readout(knob),
                Pickable::IGNORE,
            ))
            .id()
    };
    match *knobs {
        [down, up] => {
            let minus = knob_button(commands, fonts, metrics, "−", down);
            let value = readout(commands, down);
            let plus = knob_button(commands, fonts, metrics, "+", up);
            let group = commands
                .spawn((
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(6),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(group).add_children(&[minus, value, plus]);
            commands.entity(line).add_child(group);
        }
        [knob] => {
            let button = knob_button(commands, fonts, metrics, "", knob);
            let value = readout(commands, knob);
            commands.entity(button).add_child(value);
            commands.entity(line).add_child(button);
        }
        _ => {}
    }
    line
}

fn knob_button(
    commands: &mut Commands,
    fonts: &crate::hud::UiFonts,
    metrics: crate::lobby::Metrics,
    label: &str,
    knob: Knob,
) -> Entity {
    let id = crate::lobby::button(
        commands,
        fonts,
        metrics,
        label,
        crate::lobby::Press::Shared(crate::lobby::SharedPress::PickerNothing),
        crate::hud::palette::PANEL,
        true,
    );
    commands
        .entity(id)
        .entry::<Node>()
        .and_modify(move |mut node| node.min_width = bevy::ui::px(metrics.tap));
    commands
        .entity(id)
        .remove::<crate::lobby::Press>()
        .insert((Button, knob))
        .observe(
            |mut click: On<Pointer<Click>>,
             knobs: Query<&Knob>,
             in_use: Res<InUse>,
             mut settings: ResMut<ClientSettings>| {
                let Ok(knob) = knobs.get(click.entity) else {
                    return;
                };
                click.propagate(false);
                turn(*knob, &mut settings, in_use.0);
                settings.save();
            },
        );
    id
}

fn show_knobs(
    settings: Option<Res<ClientSettings>>,
    in_use: Res<InUse>,
    mut readouts: Query<(&mut Text, &Readout)>,
    added: Query<(), Added<Readout>>,
) {
    let Some(settings) = settings else {
        return;
    };
    if !settings.is_changed() && !in_use.is_changed() && added.is_empty() {
        return;
    }
    let lang = Lang::of(&settings.lang);
    for (mut text, readout) in &mut readouts {
        let value = reading(readout.0, &settings, in_use.0, lang);
        if **text != value {
            **text = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Turning the preset knob walks the named presets and wraps; turning
    /// any other knob makes the device's choice `Custom` and is shown.
    #[test]
    fn the_knobs_turn_the_settings_and_read_back() {
        let mut settings = ClientSettings::default();
        let medium = Graphics::of(Preset::Medium);
        turn(Knob::Preset, &mut settings, medium);
        assert_eq!(settings.graphics.map(|g| g.preset), Some(Preset::High));
        for _ in 0..3 {
            turn(Knob::Preset, &mut settings, medium);
        }
        assert_eq!(settings.graphics.map(|g| g.preset), Some(Preset::Medium));
        turn(Knob::FrameLimit, &mut settings, medium);
        let now = settings.graphics.expect("chosen");
        assert_eq!(now.frame_limit, FrameLimit::Fps120);
        assert_eq!(now.preset, Preset::Custom);
        assert_eq!(reading(Knob::Preset, &settings, medium, Lang::En), "Custom");
        assert_eq!(
            reading(Knob::FrameLimit, &settings, medium, Lang::De),
            "120 fps"
        );
        // From Custom the preset knob starts over at Low.
        turn(Knob::Preset, &mut settings, medium);
        assert_eq!(settings.graphics.map(|g| g.preset), Some(Preset::Low));
        turn(Knob::Master(-1), &mut settings, medium);
        assert_eq!(
            reading(Knob::Master(-1), &settings, medium, Lang::En),
            "90 %"
        );
        turn(Knob::MuteInBackground, &mut settings, medium);
        assert!(settings.audio.mute_in_background);
    }

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
}
