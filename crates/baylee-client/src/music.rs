//! One orchestra follows the player through the whole app. The conductor
//! changes future bars from public game activity; the audio player never
//! restarts at a screen change. Only the user's own volume fades the master.
use crate::{Duel, DuelPhase, settings::ClientSettings};
use baylee_client_core::music::{self, Mood, ScoreControl, Tune};
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
#[derive(Default)]
struct Activity {
    seq: Option<u64>,
    life: i32,
    objects: usize,
    stack: usize,
    energy: f32,
}

#[derive(Resource)]
struct Installed;

/// Install the orchestra and the screen-independent quick-settings controls.
pub fn install(app: &mut App) {
    if app.world().contains_resource::<Installed>() {
        return;
    }
    app.insert_resource(Installed);
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

fn direction(
    phase: DuelPhase,
    duel: Option<&Duel>,
    activity: &mut Activity,
    dt: f32,
) -> (Mood, f32) {
    activity.energy *= (-dt / 9.0).exp();
    if phase == DuelPhase::Closed {
        activity.seq = None;
        return (Mood::Sanctuary, 0.0);
    }
    if let Some(duel) = duel
        && let Some(view) = duel.view.as_ref()
    {
        if let Some(result) = duel.ending() {
            return (
                match baylee_client_core::interaction::outcome(result, view.seat, duel.my_team())
                    .won()
                {
                    Some(true) => Mood::Victory,
                    Some(false) => Mood::Defeat,
                    None => Mood::Draw,
                },
                0.0,
            );
        }
        if activity.seq != Some(view.seq) {
            let life = view.seats.iter().map(|seat| seat.life).sum::<i32>();
            if activity.seq.is_some() {
                #[allow(clippy::cast_precision_loss)] // small visible board counts
                let burst = (activity.life - life).max(0) as f32 * 0.065
                    + activity.objects.abs_diff(view.battlefield.len()) as f32 * 0.06
                    + activity.stack.abs_diff(view.stack.len()) as f32 * 0.13;
                activity.energy = (activity.energy + burst).min(1.0);
            }
            activity.seq = Some(view.seq);
            activity.life = life;
            activity.objects = view.battlefield.len();
            activity.stack = view.stack.len();
        }
        #[allow(clippy::cast_precision_loss)] // capped after conversion
        let pressure =
            (view.combat.attackers.len() as f32 * 0.09 + view.stack.len() as f32 * 0.10).min(0.65);
        return (Mood::Battle, (0.12 + pressure + activity.energy).min(1.0));
    }
    (Mood::Battle, 0.15)
}

#[allow(clippy::too_many_arguments)] // Bevy system: conductor, screen, game, settings and the persistent player
fn perform(
    mut commands: Commands,
    time: Res<Time<Real>>,
    conductor: Res<Conductor>,
    phase: Option<Res<State<DuelPhase>>>,
    duel: Option<Res<Duel>>,
    settings: Option<Res<ClientSettings>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut players: Query<(&mut Playing, Option<&mut AudioSink>)>,
    mut activity: Local<Activity>,
) {
    let dt = time.delta_secs();
    let (mood, energy) = direction(
        phase.map_or(DuelPhase::Closed, |p| *p.get()),
        duel.as_deref(),
        &mut activity,
        dt,
    );
    conductor.control.set(mood, energy);
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
            sink.set_volume(Volume::Linear(playing.gain));
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
        crate::lobby::Press::PickerNothing,
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
        assert!(app.world().get::<Playing>(entity).unwrap().gain < 0.0001);
        app.world_mut()
            .resource_mut::<ClientSettings>()
            .music
            .set_muted(false);
        for _ in 0..20 {
            app.update();
        }
        assert!(app.world().get::<Playing>(entity).unwrap().gain > 0.24);
    }
    #[test]
    fn action_energy_decays_and_does_not_count_a_repeated_snapshot() {
        use baylee_client_core::test_support::ViewBuilder;
        let mut duel = Duel {
            view: Some(ViewBuilder::new(2).build()),
            ..Default::default()
        };
        let mut activity = Activity::default();
        direction(DuelPhase::Playing, Some(&duel), &mut activity, 0.1);
        let v = duel.view.as_mut().unwrap();
        v.seq += 1;
        v.seats[0].life -= 7;
        let (_, peak) = direction(DuelPhase::Playing, Some(&duel), &mut activity, 0.1);
        let (_, later) = direction(DuelPhase::Playing, Some(&duel), &mut activity, 5.0);
        assert!(peak > 0.5 && later < peak);
        assert_eq!(
            direction(DuelPhase::Closed, Some(&duel), &mut activity, 0.1).0,
            Mood::Sanctuary
        );
    }
    #[test]
    fn results_choose_the_matching_cadence() {
        use baylee_client_core::{Interaction, test_support::ViewBuilder};
        use baylee_core::ids::PlayerId;
        use baylee_engine::{
            choice::Pending,
            win::{EndReason, GameResult, Victor},
        };
        for (winner, expected) in [
            (Some(Victor::Player(PlayerId::new(0))), Mood::Victory),
            (Some(Victor::Player(PlayerId::new(1))), Mood::Defeat),
            (None, Mood::Draw),
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
            assert_eq!(
                direction(
                    DuelPhase::Finished,
                    Some(&duel),
                    &mut Activity::default(),
                    0.1
                )
                .0,
                expected
            );
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
