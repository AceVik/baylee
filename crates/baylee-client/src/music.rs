//! One orchestra follows the player through the whole app. The conductor
//! (`client-core::music::direct`, a pure function of the screen and the
//! `PlayerView`) changes future bars; the audio player never restarts at a
//! screen change. Only the user's own volume fades the master, and a
//! priority cue ducks it for a moment.
use crate::{Duel, DuelPhase, lobby::LobbyState, settings::ClientSettings};
use baylee_client_core::lobby::Screen;
use baylee_client_core::music::{self, Ending, Memory, Place, ScoreControl, ScoreRequest, Tune};
use bevy::audio::{
    AddAudioSource, AudioPlayer, AudioPlugin, AudioSink, AudioSinkPlayback, ChannelCount,
    Decodable, PlaybackSettings, Sample, SampleRate, Source, Volume,
};
use bevy::prelude::*;
use std::{sync::Arc, time::Duration};

/// A shared conductor, held by the continuously playing audio asset.
#[derive(Asset, TypePath, Clone, Debug, Default)]
pub struct LobbyTune(Arc<ScoreControl>);
/// A [`Tune`] as rodio pulls it.
pub struct Stream(Tune);

impl Iterator for Stream {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        self.0.next()
    }
}

impl Source for Stream {
    /// Never changes: the same rate and channels for as long as it plays.
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(music::CHANNELS).unwrap_or(ChannelCount::MIN)
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(music::RATE).unwrap_or(SampleRate::MIN)
    }

    /// Endless: the loop plays on until the player is let go.
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for LobbyTune {
    type Decoder = Stream;
    fn decoder(&self) -> Stream {
        Stream(Tune::with_control(self.0.clone()))
    }
}
#[derive(Resource)]
struct Conductor {
    handle: Handle<LobbyTune>,
    control: Arc<ScoreControl>,
}
#[derive(Component)]
struct Playing {
    gain: f32,
}

/// The last request the conductor sent, which `/state` reports as `score`
/// (dev-control): what the drivers decided, beside what a player hears.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct Heard(pub ScoreRequest);

/// Priority cues the music ducks under: `sound.rs` counts each one it plays.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct Duck {
    cues: u32,
}

impl Duck {
    /// A priority cue was played.
    pub fn cue(&mut self) {
        self.cues = self.cues.wrapping_add(1);
    }
}

/// How far the music is ducked, `since` seconds after a priority cue: down
/// 6 dB in 30 ms, held while the cue speaks, back over 400 ms.
fn ducked(since: f32) -> f32 {
    const DOWN: f32 = 0.03;
    const HOLD: f32 = 0.18;
    const BACK: f32 = 0.4;
    const FLOOR: f32 = 0.5;
    if since < DOWN {
        1.0 - (1.0 - FLOOR) * since / DOWN
    } else if since < DOWN + HOLD {
        FLOOR
    } else if since < DOWN + HOLD + BACK {
        FLOOR + (1.0 - FLOOR) * (since - DOWN - HOLD) / BACK
    } else {
        1.0
    }
}

#[derive(Resource)]
struct Installed;

/// Install the orchestra and the screen-independent quick-settings controls.
pub fn install(app: &mut App) {
    if app.world().contains_resource::<Installed>() {
        return;
    }
    app.insert_resource(Installed)
        .init_resource::<Heard>()
        .init_resource::<Duck>();
    app.add_systems(Update, show_level);
    if !app.is_plugin_added::<AudioPlugin>() {
        return;
    }
    music::prepare();
    let control = Arc::new(ScoreControl::default());
    app.add_audio_source::<LobbyTune>();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<LobbyTune>>()
        .add(LobbyTune(control.clone()));
    app.insert_resource(Conductor { handle, control })
        .add_systems(Update, perform);
}

/// Where the player is, as the score's drivers read it: a duel's phase
/// first, else the lobby's screen.
fn place(phase: DuelPhase, screen: Option<&Screen>) -> Place {
    match phase {
        DuelPhase::Opening => Place::Opening,
        DuelPhase::Playing => Place::Table,
        DuelPhase::Finished => Place::Finished,
        DuelPhase::Closed => match screen {
            None | Some(Screen::SignIn { .. }) => Place::FrontDoor,
            Some(Screen::Build) => Place::Build,
            Some(Screen::Table | Screen::Seated(_)) => Place::Lobby,
        },
    }
}

/// The request for this frame: the duel's view and result, through the pure
/// drivers in client-core — the same for a hosted game and a `LocalHost`
/// game, which fill `Duel::view` alike.
fn direction(
    phase: DuelPhase,
    screen: Option<&Screen>,
    duel: Option<&Duel>,
    theme: music::MusicTheme,
    memory: &mut Memory,
    dt: f32,
) -> ScoreRequest {
    let view = duel.and_then(|duel| duel.view.as_ref());
    let ending = duel.and_then(|duel| {
        let result = duel.ending()?;
        let view = duel.view.as_ref()?;
        Some(Ending::of(
            baylee_client_core::interaction::outcome(result, view.seat, duel.my_team()).won(),
        ))
    });
    music::direct(place(phase, screen), view, ending, theme, memory, dt)
}

#[allow(clippy::too_many_arguments)] // Bevy system: conductor, screen, game, settings and the persistent player
fn perform(
    mut commands: Commands,
    time: Res<Time<Real>>,
    conductor: Res<Conductor>,
    phase: Option<Res<State<DuelPhase>>>,
    duel: Option<Res<Duel>>,
    lobby: Option<Res<LobbyState>>,
    settings: Option<Res<ClientSettings>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut players: Query<(&mut Playing, Option<&mut AudioSink>)>,
    mut heard: ResMut<Heard>,
    duck: Res<Duck>,
    mut memory: Local<Option<Memory>>,
    mut ducking: Local<(u32, f32)>,
) {
    let dt = time.delta_secs();
    // "Rotating" starts somewhere new each run of the client.
    let memory = memory.get_or_insert_with(|| Memory::seeded(seed()));
    let theme = settings
        .as_ref()
        .map_or_else(music::MusicTheme::default, |s| s.music.theme());
    let request = direction(
        phase.map_or(DuelPhase::Closed, |p| *p.get()),
        lobby.as_deref().map(|lobby| lobby.lobby.screen()),
        duel.as_deref(),
        theme,
        memory,
        dt,
    );
    conductor.control.set(request);
    if heard.0 != request {
        heard.0 = request;
    }
    // A priority cue ducks the music from the frame it is heard.
    if ducking.0 == duck.cues {
        ducking.1 += dt;
    } else {
        *ducking = (duck.cues, 0.0);
    }
    let duck = ducked(ducking.1);
    let focused = crate::quality::focused(&windows);
    let target = settings.map_or_else(
        || music::MusicLevel::default().gain(),
        |s| s.music.gain() * s.audio.master_gain(focused),
    );
    let mut any = false;
    for (mut playing, sink) in &mut players {
        any = true;
        playing.gain += (target - playing.gain) * (1.0 - (-dt / 0.10).exp());
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(playing.gain * duck));
            // A silent orchestra stops being rendered: a paused sink stops
            // pulling the score, where one at volume 0 still synthesised
            // every voice on the audio thread (`docs/perf-baseline.md`,
            // 08.10.2026). It resumes where it stood.
            match (sounding(target, playing.gain), sink.is_paused()) {
                (false, false) => sink.pause(),
                (true, true) => sink.play(),
                _ => {}
            }
        }
    }
    if !any {
        commands.spawn((
            AudioPlayer(conductor.handle.clone()),
            PlaybackSettings::ONCE.with_volume(Volume::Linear(0.0)),
            Playing { gain: 0.0 },
        ));
    }
}

/// Where the rotating theme starts: the clock's seconds, so two runs of the
/// client rarely start alike (`web_time`: the browser has no `SystemTime`).
fn seed() -> u8 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |since| (since.as_secs() % 5) as u8)
}

/// Below this gain the music is inaudible: a sink that has faded to it and
/// is asked for nothing louder is paused.
const SILENT_GAIN: f32 = 1e-4;

/// Whether the orchestra has to be rendered: it is asked to be heard, or
/// it is still fading out.
fn sounding(target: f32, gain: f32) -> bool {
    target > 0.0 || gain > SILENT_GAIN
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) enum MusicAction {
    Toggle,
    Adjust(i8),
}
#[derive(Component)]
struct MusicReadout;
#[derive(Component)]
struct MusicSwitch;

/// The same accessible controls in the login gear, settings and table menu.
pub(crate) fn controls(
    commands: &mut Commands,
    fonts: &crate::hud::UiFonts,
    metrics: crate::lobby::Metrics,
    lang: baylee_client_core::i18n::Lang,
) -> Entity {
    use crate::hud::{palette, tf};
    use baylee_client_core::i18n::Phrase;
    use bevy::ui::{percent, px};
    let root = commands
        .spawn((
            Node {
                width: percent(100),
                max_width: px(320),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let toggle = control_button(
        commands,
        fonts,
        metrics,
        Phrase::MusicPlaying.text(lang),
        MusicAction::Toggle,
    );
    commands.entity(toggle).with_child((
        Text::new(Phrase::MusicPlaying.text(lang)),
        tf(fonts, metrics.text),
        TextColor(palette::INK),
        MusicSwitch,
        Pickable::IGNORE,
    ));
    let row = commands
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
    let down = control_button(commands, fonts, metrics, "−", MusicAction::Adjust(-1));
    let up = control_button(commands, fonts, metrics, "+", MusicAction::Adjust(1));
    let label = commands
        .spawn((
            Text::new("50 %"),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            MusicReadout,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(&[down, label, up]);
    commands.entity(root).add_children(&[toggle, row]);
    root
}
fn control_button(
    commands: &mut Commands,
    fonts: &crate::hud::UiFonts,
    metrics: crate::lobby::Metrics,
    label: &str,
    action: MusicAction,
) -> Entity {
    let id = crate::lobby::button(
        commands,
        fonts,
        metrics,
        if matches!(action, MusicAction::Toggle) {
            ""
        } else {
            label
        },
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
        .insert((Button, action))
        .observe(
            |mut click: On<Pointer<Click>>,
             actions: Query<&MusicAction>,
             mut settings: ResMut<ClientSettings>| {
                let Ok(action) = actions.get(click.entity) else {
                    return;
                };
                click.propagate(false);
                match action {
                    MusicAction::Toggle => settings.music.toggle(),
                    MusicAction::Adjust(step) => {
                        let volume = settings.music.volume() + f32::from(*step) * 0.1;
                        settings.music.set_volume(volume);
                        if *step > 0 {
                            settings.music.set_muted(false);
                        }
                    }
                }
                settings.save();
            },
        );
    id
}
#[allow(clippy::type_complexity)] // two marker types share a text update query
fn show_level(
    settings: Option<Res<ClientSettings>>,
    mut labels: Query<
        (&mut Text, Has<MusicReadout>, Has<MusicSwitch>),
        Or<(With<MusicReadout>, With<MusicSwitch>)>,
    >,
) {
    use baylee_client_core::i18n::{Lang, Phrase};
    let Some(settings) = settings else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    for (mut text, volume, switch) in &mut labels {
        let value = if volume {
            format!("{:.0} %", settings.music.volume() * 100.0)
        } else if switch {
            if settings.music.gain() > 0.0 {
                Phrase::MusicPlaying
            } else {
                Phrase::MusicSilent
            }
            .text(lang)
            .to_owned()
        } else {
            continue;
        };
        if **text != value {
            **text = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_player_survives_tables_endings_and_mute() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<DuelPhase>()
            .insert_resource(ClientSettings::default())
            .insert_resource(Conductor {
                handle: Handle::default(),
                control: Arc::new(ScoreControl::default()),
            })
            .init_resource::<Heard>()
            .init_resource::<Duck>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_millis(100),
            ))
            .add_systems(Update, perform);
        app.update();
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<Playing>>()
            .single(app.world())
            .unwrap();
        for phase in [
            DuelPhase::Opening,
            DuelPhase::Playing,
            DuelPhase::Finished,
            DuelPhase::Closed,
        ] {
            app.world_mut()
                .resource_mut::<NextState<DuelPhase>>()
                .set(phase);
            for _ in 0..20 {
                app.update();
            }
            assert_eq!(
                app.world_mut()
                    .query_filtered::<Entity, With<Playing>>()
                    .single(app.world())
                    .unwrap(),
                entity
            );
        }
        app.world_mut()
            .resource_mut::<ClientSettings>()
            .music
            .set_muted(true);
        for _ in 0..20 {
            app.update();
        }
        let faded = app.world().get::<Playing>(entity).unwrap().gain;
        assert!(faded < 0.0001);
        assert!(
            !sounding(0.0, faded),
            "a muted orchestra that has faded out is paused, not rendered at volume 0"
        );
        app.world_mut()
            .resource_mut::<ClientSettings>()
            .music
            .set_muted(false);
        for _ in 0..20 {
            app.update();
        }
        assert!(app.world().get::<Playing>(entity).unwrap().gain > 0.24);
        // The theme is a setting: changed, the very next request carries it,
        // and the one player plays on (the score takes it at a bar line).
        for theme in music::MusicTheme::ALL {
            app.world_mut()
                .resource_mut::<ClientSettings>()
                .music
                .set_theme(theme);
            app.update();
            let heard = app.world().resource::<Heard>().0.theme;
            if theme != music::MusicTheme::Rotating {
                assert_eq!(heard, theme.pick(0), "{theme:?}");
            }
            assert_eq!(
                app.world_mut()
                    .query_filtered::<Entity, With<Playing>>()
                    .single(app.world())
                    .unwrap(),
                entity,
                "no restart for a theme"
            );
        }
    }
    /// Muting fades out first and pauses once silent; any gain asked for,
    /// however small, renders again.
    #[test]
    fn the_orchestra_is_rendered_only_while_it_can_be_heard() {
        assert!(sounding(0.5, 0.0), "unmuted: rendered from the first frame");
        assert!(sounding(0.0, 0.2), "muted but still fading out");
        assert!(!sounding(0.0, SILENT_GAIN * 0.5), "faded out: paused");
        assert!(!sounding(0.0, 0.0));
        assert!(sounding(0.01, 0.0), "a quiet volume is still a volume");
    }

    /// The screens and the phases name their places: a duel's phase wins
    /// over the lobby's screen.
    #[test]
    fn the_screens_and_phases_are_places() {
        let sign_in = Screen::SignIn { registering: false };
        assert_eq!(place(DuelPhase::Closed, None), Place::FrontDoor);
        assert_eq!(place(DuelPhase::Closed, Some(&sign_in)), Place::FrontDoor);
        assert_eq!(place(DuelPhase::Closed, Some(&Screen::Table)), Place::Lobby);
        assert_eq!(place(DuelPhase::Closed, Some(&Screen::Build)), Place::Build);
        assert_eq!(
            place(DuelPhase::Opening, Some(&Screen::Build)),
            Place::Opening
        );
        assert_eq!(place(DuelPhase::Playing, None), Place::Table);
        assert_eq!(place(DuelPhase::Finished, None), Place::Finished);
    }

    /// A priority cue ducks the music 6 dB within 30 ms and gives it back
    /// within half a second.
    #[test]
    fn a_priority_cue_ducks_the_music() {
        assert!((ducked(0.0) - 1.0).abs() < 1e-6);
        assert!((ducked(0.03) - 0.5).abs() < 1e-6);
        assert!((ducked(0.1) - 0.5).abs() < 1e-6);
        assert!(ducked(0.4) > 0.5 && ducked(0.4) < 1.0);
        assert!((ducked(0.7) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn results_choose_the_matching_ending() {
        use baylee_client_core::music::Scene;
        use baylee_client_core::{Interaction, test_support::ViewBuilder};
        use baylee_core::ids::PlayerId;
        use baylee_engine::{
            choice::Pending,
            win::{EndReason, GameResult, Victor},
        };
        for (winner, expected) in [
            (Some(Victor::Player(PlayerId::new(0))), Scene::Victory),
            (Some(Victor::Player(PlayerId::new(1))), Scene::Defeat),
            (None, Scene::Draw),
        ] {
            let duel = Duel {
                view: Some(ViewBuilder::new(2).build()),
                interaction: Some(Interaction::new(
                    Pending::GameOver(GameResult {
                        winner,
                        reason: if winner.is_some() {
                            EndReason::LastPlayerStanding
                        } else {
                            EndReason::Draw
                        },
                    }),
                    PlayerId::new(0),
                )),
                ..Default::default()
            };
            let request = direction(
                DuelPhase::Finished,
                None,
                Some(&duel),
                music::MusicTheme::Thorn,
                &mut Memory::default(),
                0.1,
            );
            assert_eq!(request.scene, expected);
        }
    }

    #[test]
    fn the_stream_remains_endless_stereo() {
        let stream = LobbyTune::default().decoder();
        assert_eq!(stream.channels().get(), music::CHANNELS);
        assert_eq!(stream.sample_rate().get(), music::RATE);
        assert_eq!(stream.total_duration(), None);
    }
}
