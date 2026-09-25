//! The front door's music (#296): [`baylee_client_core::music::Tune`],
//! heard on the gateway and sign-in faces and nowhere else.
//!
//! The tune is a [`Decodable`] asset whose decoder is the tune itself, so
//! `bevy_audio` hands it to rodio and the audio thread pulls it a buffer at
//! a time: no frame computes a sample, and no buffer is held. In a browser
//! there is no audio thread, and the pull runs between frames on the one
//! thread there is; the tune is made about three hundred times faster than
//! it plays, so that costs a fraction of a millisecond a frame.
//!
//! One player exists while the music is heard or fading, and none at any
//! other time, so a player at the table synthesises nothing. It fades in
//! over [`FADE_IN`] when the front door opens, and out over [`FADE_OUT`]
//! when a player signs in, a game opens or the music is muted; the volume
//! in [`ClientSettings::music`] is read on every frame, so a slider is heard
//! as it moves. A tune that fades to nothing is let go; the next front door
//! starts it from its first bar.

use std::time::Duration;

use baylee_client_core::lobby::Screen;
use baylee_client_core::music::{self, Tune};
use bevy::audio::{
    AddAudioSource, AudioPlayer, AudioPlugin, AudioSink, AudioSinkPlayback, ChannelCount,
    Decodable, PlaybackSettings, Sample, SampleRate, Source, Volume,
};
use bevy::prelude::*;

use crate::DuelPhase;
use crate::lobby::LobbyState;
use crate::settings::ClientSettings;

/// Seconds from silence to the player's volume.
const FADE_IN: f32 = 2.5;

/// Seconds from the player's volume to silence.
const FADE_OUT: f32 = 0.8;

/// The tune, as an asset `bevy_audio` can play. It holds nothing: every
/// player made from it starts a new [`Tune`] at its first bar.
#[derive(Asset, TypePath, Clone, Copy, Debug, Default)]
pub struct FrontDoorTune;

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

impl Decodable for FrontDoorTune {
    type Decoder = Stream;

    fn decoder(&self) -> Stream {
        Stream(Tune::new())
    }
}

/// The asset every player is made from.
#[derive(Resource)]
struct TuneHandle(Handle<FrontDoorTune>);

/// The one player, and how far it has faded in: from 0, silent, to 1, at
/// the player's volume.
#[derive(Component)]
struct Playing {
    presence: f32,
}

/// Adds the music to the front door, when the app has audio at all: an
/// embedding without bevy's `AudioPlugin` has nothing to play it through.
pub fn install(app: &mut App) {
    if !app.is_plugin_added::<AudioPlugin>() {
        return;
    }
    app.add_audio_source::<FrontDoorTune>();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<FrontDoorTune>>()
        .add(FrontDoorTune);
    app.insert_resource(TuneHandle(handle))
        .add_systems(Update, play_at_the_front_door);
}

/// Whether the music belongs on this screen: the gateway, sign-in and
/// registration faces, which are all [`Screen::SignIn`], and only while no
/// game is open over them.
fn heard(screen: &Screen, duel: DuelPhase) -> bool {
    matches!(screen, Screen::SignIn { .. }) && duel == DuelPhase::Closed
}

/// How far faded in, one frame of `dt` seconds later, moving from `now`
/// towards `target`: up at the fade in's pace, down at the fade out's.
fn fade(now: f32, target: f32, dt: f32) -> f32 {
    if target > now {
        (now + dt / FADE_IN).min(target)
    } else {
        (now - dt / FADE_OUT).max(target)
    }
}

fn play_at_the_front_door(
    mut commands: Commands,
    time: Res<Time<Real>>,
    tune: Res<TuneHandle>,
    lobby: Option<Res<LobbyState>>,
    duel: Option<Res<State<DuelPhase>>>,
    settings: Option<Res<ClientSettings>>,
    mut players: Query<(Entity, &mut Playing, Option<&mut AudioSink>)>,
) {
    let phase = duel.map_or(DuelPhase::Closed, |duel| *duel.get());
    let level = settings.map(|settings| settings.music).unwrap_or_default();
    let wanted =
        lobby.is_some_and(|lobby| heard(lobby.lobby.screen(), phase)) && level.gain() > 0.0;
    let target = if wanted { 1.0 } else { 0.0 };
    let dt = time.delta_secs();
    let mut any = false;
    for (entity, mut playing, sink) in &mut players {
        any = true;
        playing.presence = fade(playing.presence, target, dt);
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(playing.presence * level.loudness()));
        }
        if playing.presence <= 0.0 && !wanted {
            commands.entity(entity).despawn();
        }
    }
    if !any && wanted {
        commands.spawn((
            AudioPlayer(tune.0.clone()),
            PlaybackSettings::ONCE.with_volume(Volume::Linear(0.0)),
            Playing { presence: 0.0 },
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Heard on the faces before sign-in, and nowhere once a player is in
    /// or a game is open, the table's above all.
    #[test]
    fn the_music_is_heard_at_the_front_door_and_nowhere_else() {
        let signing_in = Screen::SignIn { registering: false };
        let registering = Screen::SignIn { registering: true };
        assert!(heard(&signing_in, DuelPhase::Closed));
        assert!(heard(&registering, DuelPhase::Closed));
        for phase in [DuelPhase::Opening, DuelPhase::Playing, DuelPhase::Finished] {
            assert!(!heard(&signing_in, phase), "{phase:?}");
        }
        assert!(!heard(&Screen::Table, DuelPhase::Closed));
        assert!(!heard(&Screen::Build, DuelPhase::Closed));
    }

    /// The fades take their own time each way, whatever the volume, and
    /// stop where they are going.
    #[test]
    fn a_fade_takes_its_time_and_stops_there() {
        let frame = 1.0 / 60.0;
        let count = |from: f32, to: f32| {
            let (mut now, mut frames) = (from, 0_u32);
            while (now - to).abs() > f32::EPSILON {
                now = fade(now, to, frame);
                frames += 1;
                assert!((0.0..=1.0).contains(&now), "{now}");
            }
            frames
        };
        let up = count(0.0, 1.0);
        let down = count(1.0, 0.0);
        assert!(
            up.abs_diff(150) <= 1,
            "in over {up} frames, not {FADE_IN} s"
        );
        assert!(
            down.abs_diff(48) <= 1,
            "out over {down} frames, not {FADE_OUT} s"
        );
    }

    /// An app with the lobby's state, a device's settings and the music's
    /// system, whose clock moves a tenth of a second a frame.
    fn front_door() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<DuelPhase>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_millis(100),
            ))
            .insert_resource(LobbyState::new())
            .insert_resource(ClientSettings::default())
            .insert_resource(TuneHandle(Handle::default()))
            .add_systems(Update, play_at_the_front_door);
        app.update();
        app
    }

    /// How far the one player has faded in, or `None` when there is none.
    fn playing(app: &mut App) -> Option<f32> {
        let mut query = app.world_mut().query::<&Playing>();
        let found: Vec<f32> = query.iter(app.world()).map(|p| p.presence).collect();
        assert!(found.len() <= 1, "one player at most: {found:?}");
        found.first().copied()
    }

    fn run(app: &mut App, seconds: f32) {
        for _ in 0..(seconds * 10.0).ceil() as usize {
            app.update();
        }
    }

    /// The player's whole life: made at the front door and faded all the
    /// way in, faded out and let go when a game opens over it, made again
    /// from the first bar when the front door is back, and faded out and let
    /// go when muted.
    #[test]
    fn the_player_comes_and_goes_with_the_front_door() {
        let mut app = front_door();
        run(&mut app, FADE_IN / 2.0);
        let rising = playing(&mut app).expect("the front door plays");
        assert!(rising > 0.0 && rising < 1.0, "fading in: {rising}");
        run(&mut app, FADE_IN);
        let gain = playing(&mut app).expect("still playing");
        assert!((gain - 1.0).abs() < f32::EPSILON, "faded in: {gain}");

        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Playing);
        run(&mut app, FADE_OUT / 2.0);
        let fading = playing(&mut app).expect("still fading");
        assert!(fading < gain && fading > 0.0, "{fading}");
        run(&mut app, FADE_OUT);
        assert_eq!(playing(&mut app), None, "let go at the table");
        run(&mut app, 3.0);
        assert_eq!(playing(&mut app), None, "and never made again there");

        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Closed);
        run(&mut app, 0.2);
        assert!(playing(&mut app).is_some(), "back at the front door");
        run(&mut app, FADE_IN);

        app.world_mut()
            .resource_mut::<ClientSettings>()
            .music
            .set_muted(true);
        run(&mut app, 0.3);
        assert!(
            playing(&mut app).is_some(),
            "muting fades, it does not click"
        );
        run(&mut app, FADE_OUT);
        assert_eq!(playing(&mut app), None, "muted is let go, not played at 0");
    }

    /// Rodio is told the truth about the stream: two channels at the tune's
    /// rate, and no end.
    #[test]
    fn the_stream_says_what_it_is() {
        let stream = FrontDoorTune.decoder();
        assert_eq!(stream.channels().get(), music::CHANNELS);
        assert_eq!(stream.sample_rate().get(), music::RATE);
        assert_eq!(stream.total_duration(), None);
        assert_eq!(stream.take(10_000).count(), 10_000);
    }
}
