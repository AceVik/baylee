//! The recorded soundtrack (`MusicTheme::Saga`): nine Ogg-Opus files, cut
//! and levelled from the owner's AI-generated recordings (`docs/legal.md` §5,
//! `art/baylee/README.md`), embedded so the music plays offline.
//!
//! [`Mixer`] is the whole performance, a pure function of the
//! [`ScoreRequest`]s it is handed and the samples it renders: two bed voices
//! that cross-fade with equal power (a scene change ≈ 1.2 s, a loop's head
//! under its own tail by the track's overlap), one stinger voice, and the
//! bed's duck under it. Nothing in it reads a clock.
//!
//! The decoder (`opus-decoder`) allocates per packet, so the mixer never runs
//! on the audio thread: [`feed`] runs it on a worker of its own into a fixed
//! ring, and the audio thread only takes frames out of it. Offline renders
//! and tests run the same mixer inline (`Tune::with_control`).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
// Sample counts below 2^24 and bounded amplitudes.
use super::{RATE, Scene, ScoreRequest};
use opus_decoder::OpusDecoder;
use std::sync::OnceLock;

pub(super) mod feed;
pub(crate) mod ogg;

/// One gain for the whole recorded path, so that switching between the
/// recorded theme and a synthesised one does not jump in loudness: +2.9 dB.
///
/// Measured 18.11.2026 (`the_recorded_path_is_as_loud_as_the_synth`, plain
/// RMS over 30 s after a 5-s settle): the synth's lobby −20.1 dBFS and a
/// mid-tension Dornenkrone table −23.5 dBFS; the recorded lobby bed −25.6 and
/// combat bed −23.8 dBFS at unity gain. The synth's lobby is 3.4 dB louder
/// than its table, the recordings' lobby (levelled to −19 LUFS against
/// combat's −18) 1.8 dB quieter, so no single gain matches both: +2.9 dB
/// splits the 5.2 dB between them, leaving the lobby 2.6 dB quieter and the
/// table 2.6 dB louder than the synth. Matching both would take a trim per
/// bed, which would undo the files' own levelling.
pub(super) const RECORDED_GAIN: f32 = 1.396;

// The recorded path ends in the orchestra's safety knee (`orchestra::knee`).
// The beds stay below it (their sample peaks are −5.2 dBFS and lower, −2.3
// after the gain); the call stinger alone peaks at −2.5 dBFS, +0.4 after the
// gain, so its loudest moments bend there (364 of 432 000 frames in
// `a_stinger_over_the_bed_stays_below_full_scale`).
use super::orchestra::knee;

/// Moves `level` towards `target` by at most `step`, never past it.
fn slew(level: f32, target: f32, step: f32) -> f32 {
    if level < target {
        (level + step).min(target)
    } else {
        (level - step).max(target)
    }
}

/// A scene change's equal-power cross-fade, in seconds.
const SCENE_FADE: f32 = 1.2;
/// An ending's own fade-in: the cuts open with their own short fade and the
/// victory with the fanfare, so they come in quickly while the bed before
/// them still fades over [`SCENE_FADE`].
const ENDING_FADE_IN: f32 = 0.25;
/// A stinger's fade-in; the files carry their own fades.
const STINGER_FADE_IN: f32 = 0.01;
/// How far the bed ducks under a stinger: −7 dB.
const DUCK_FLOOR: f32 = 0.446_683_6;
/// Seconds from full level to the duck's floor, and back.
const DUCK_DOWN: f32 = 0.08;
const DUCK_BACK: f32 = 1.2;

/// The nine recordings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Track {
    Title,
    Lobby,
    Combat,
    Tower,
    Victory,
    Defeat,
    Draw,
    StingerCall,
    StingerEpic,
}

impl Track {
    pub(crate) const ALL: [Self; 9] = [
        Self::Title,
        Self::Lobby,
        Self::Combat,
        Self::Tower,
        Self::Victory,
        Self::Defeat,
        Self::Draw,
        Self::StingerCall,
        Self::StingerEpic,
    ];

    const fn bytes(self) -> &'static [u8] {
        match self {
            Self::Title => include_bytes!("../../../../art/baylee/score/out/title.opus"),
            Self::Lobby => include_bytes!("../../../../art/baylee/score/out/lobby.opus"),
            Self::Combat => include_bytes!("../../../../art/baylee/score/out/combat.opus"),
            Self::Tower => include_bytes!("../../../../art/baylee/score/out/tower.opus"),
            Self::Victory => include_bytes!("../../../../art/baylee/score/out/victory.opus"),
            Self::Defeat => include_bytes!("../../../../art/baylee/score/out/defeat.opus"),
            Self::Draw => include_bytes!("../../../../art/baylee/score/out/draw.opus"),
            Self::StingerCall => {
                include_bytes!("../../../../art/baylee/score/out/stinger_call.opus")
            }
            Self::StingerEpic => {
                include_bytes!("../../../../art/baylee/score/out/stinger_epic.opus")
            }
        }
    }

    /// Seconds its head overlaps its own tail when it loops; `None` plays
    /// once.
    pub(crate) const fn overlap(self) -> Option<f32> {
        match self {
            Self::Title => Some(6.0),
            Self::Lobby | Self::Combat => Some(4.0),
            Self::Tower => Some(5.0),
            _ => None,
        }
    }

    /// The bed a scene plays.
    pub(crate) const fn bed(scene: Scene) -> Option<Self> {
        match scene {
            Scene::FrontDoor => Some(Self::Title),
            Scene::Lobby | Scene::Build => Some(Self::Lobby),
            Scene::Tower => Some(Self::Tower),
            Scene::Table => Some(Self::Combat),
            Scene::Victory => Some(Self::Victory),
            Scene::Defeat => Some(Self::Defeat),
            Scene::Draw => Some(Self::Draw),
            Scene::Opening => None,
        }
    }

    const fn index(self) -> usize {
        self as usize
    }

    /// Its parsed index, built once for the whole process.
    pub(crate) fn stream(self) -> &'static ogg::Stream<'static> {
        static LIBRARY: OnceLock<Vec<ogg::Stream<'static>>> = OnceLock::new();
        let library = LIBRARY.get_or_init(|| {
            Self::ALL
                .iter()
                .map(|track| {
                    // The files are ours and checked by the tests: a failure
                    // here is a broken build, and plays a silent track.
                    ogg::Stream::parse(track.bytes()).unwrap_or_else(|_| ogg::Stream::silent())
                })
                .collect()
        });
        &library[self.index()]
    }
}

/// One decoding voice: a track, its place in it, and its gain envelope.
pub(crate) struct Voice {
    track: Option<Track>,
    decoder: Option<OpusDecoder>,
    pcm: Box<[f32]>,
    pcm_len: usize,
    pcm_at: usize,
    packet: usize,
    skip: usize,
    /// Samples per channel given out since the head.
    played: usize,
    /// The envelope's position, 0 to 1; its amplitude is `sin(level·π/2)`.
    level: f32,
    target: f32,
    step: f32,
    /// This looping voice has already started its own head under its tail.
    handed: bool,
}

impl Voice {
    fn new() -> Self {
        Self {
            track: None,
            decoder: OpusDecoder::new(RATE, 2).ok(),
            pcm: vec![0.0; ogg::MAX_FRAMES * 2].into_boxed_slice(),
            pcm_len: 0,
            pcm_at: 0,
            packet: 0,
            skip: 0,
            played: 0,
            level: 0.0,
            target: 0.0,
            step: 0.0,
            handed: false,
        }
    }

    /// Starts a track from its head, fading in over `fade` seconds.
    fn start(&mut self, track: Track, fade: f32) {
        if let Some(decoder) = &mut self.decoder {
            decoder.reset();
        }
        self.track = Some(track);
        self.pcm_len = 0;
        self.pcm_at = 0;
        self.packet = 0;
        self.skip = track.stream().pre_skip;
        self.played = 0;
        self.level = 0.0;
        self.handed = false;
        self.fade(1.0, fade);
    }

    /// Moves the envelope towards `target` over `seconds` for a full swing.
    fn fade(&mut self, target: f32, seconds: f32) {
        self.target = target;
        self.step = 1.0 / (seconds.max(1e-3) * RATE as f32);
    }

    const fn free(&self) -> bool {
        self.track.is_none()
    }

    fn stop(&mut self) {
        self.track = None;
        self.level = 0.0;
        self.target = 0.0;
    }

    /// The next decoded frame of its track, `None` at its end.
    fn next(&mut self) -> Option<[f32; 2]> {
        let stream = self.track?.stream();
        loop {
            if self.played >= stream.length {
                return None;
            }
            if self.pcm_at < self.pcm_len {
                let at = self.pcm_at * 2;
                self.pcm_at += 1;
                if self.skip > 0 {
                    self.skip -= 1;
                    continue;
                }
                self.played += 1;
                return Some([self.pcm[at] * stream.gain, self.pcm[at + 1] * stream.gain]);
            }
            let packet = stream.packet(self.packet)?;
            self.packet += 1;
            let decoder = self.decoder.as_mut()?;
            self.pcm_len = decoder.decode_float(packet, &mut self.pcm, false).ok()?;
            self.pcm_at = 0;
        }
    }

    /// Adds this voice, enveloped, to `out`.
    fn render(&mut self, out: &mut [[f32; 2]], weight: impl Fn(usize) -> f32) {
        if self.free() {
            return;
        }
        for (i, frame) in out.iter_mut().enumerate() {
            self.level = slew(self.level, self.target, self.step);
            let Some(sample) = self.next() else {
                self.stop();
                return;
            };
            let gain = (self.level * std::f32::consts::FRAC_PI_2).sin() * weight(i);
            frame[0] += sample[0] * gain;
            frame[1] += sample[1] * gain;
            if self.level <= 0.0 && self.target <= 0.0 {
                self.stop();
                return;
            }
        }
    }
}

/// The recorded performance: two bed voices, one stinger voice and the duck.
pub(crate) struct Mixer {
    beds: [Voice; 2],
    stinger: Voice,
    /// The bed the last request asked for.
    wanted: Option<Track>,
    /// A bed waiting for a voice to fall silent.
    pending: Option<Track>,
    /// The stinger counter last seen; `None` before the first request.
    stingers: Option<u8>,
    /// The bed's duck under a stinger, 1 at rest, per sample.
    duck: Box<[f32]>,
    duck_level: f32,
}

impl Default for Mixer {
    fn default() -> Self {
        Self::new()
    }
}

impl Mixer {
    /// Silent, with every voice and buffer allocated.
    pub(crate) fn new() -> Self {
        // Index every file now rather than at the first request.
        for track in Track::ALL {
            let _ = track.stream();
        }
        Self {
            beds: [Voice::new(), Voice::new()],
            stinger: Voice::new(),
            wanted: None,
            pending: None,
            stingers: None,
            duck: vec![1.0; super::orchestra::BLOCK].into_boxed_slice(),
            duck_level: 1.0,
        }
    }

    /// Nothing sounds and nothing waits to.
    pub(crate) fn silent(&self) -> bool {
        self.beds.iter().all(Voice::free) && self.stinger.free() && self.pending.is_none()
    }

    /// Renders the next frames for `request` into `out` (overwritten),
    /// at most [`super::orchestra::BLOCK`] at a time.
    pub(crate) fn render(&mut self, request: ScoreRequest, out: &mut [[f32; 2]]) {
        for chunk in out.chunks_mut(super::orchestra::BLOCK) {
            self.block(request, chunk);
        }
    }

    fn block(&mut self, request: ScoreRequest, out: &mut [[f32; 2]]) {
        out.fill([0.0; 2]);
        let wanted = if request.recorded {
            Track::bed(request.scene)
        } else {
            None
        };
        if wanted != self.wanted {
            self.change(wanted);
        }
        self.start_pending();
        self.hand_over();
        self.cue(request);
        // The duck: down fast while a stinger sounds, back slowly after.
        let ducking = !self.stinger.free() && self.stinger.target > 0.0;
        let (target, step) = if ducking {
            (DUCK_FLOOR, (1.0 - DUCK_FLOOR) / (DUCK_DOWN * RATE as f32))
        } else {
            (1.0, (1.0 - DUCK_FLOOR) / (DUCK_BACK * RATE as f32))
        };
        for duck in &mut self.duck[..out.len()] {
            self.duck_level = slew(self.duck_level, target, step);
            *duck = self.duck_level;
        }
        let duck = &self.duck;
        for bed in &mut self.beds {
            bed.render(out, |i| duck[i]);
        }
        self.stinger.render(out, |_| 1.0);
        for frame in out.iter_mut() {
            for sample in frame {
                *sample = knee(*sample * RECORDED_GAIN);
            }
        }
    }

    /// A new bed is wanted: the old fades out, and the new one fades in, or
    /// fades back up from where it stands if it is still fading out.
    fn change(&mut self, wanted: Option<Track>) {
        self.wanted = wanted;
        self.pending = None;
        let back = wanted.and_then(|track| {
            self.beds
                .iter()
                .position(|bed| bed.track == Some(track) && !bed.handed)
        });
        for (i, bed) in self.beds.iter_mut().enumerate() {
            if Some(i) == back {
                bed.fade(1.0, SCENE_FADE);
            } else if !bed.free() {
                bed.fade(0.0, SCENE_FADE);
            }
        }
        if back.is_none() {
            self.pending = wanted;
        }
        if wanted != Some(Track::Combat) && !self.stinger.free() {
            self.stinger.fade(0.0, SCENE_FADE);
        }
    }

    /// Starts the waiting bed on the first silent voice.
    fn start_pending(&mut self) {
        let Some(track) = self.pending else {
            return;
        };
        if let Some(bed) = self.beds.iter_mut().find(|bed| bed.free()) {
            let fade = if track.overlap().is_some() {
                SCENE_FADE
            } else {
                ENDING_FADE_IN
            };
            bed.start(track, fade);
            self.pending = None;
        }
    }

    /// A looping bed near its end starts its own head on the other voice.
    fn hand_over(&mut self) {
        for lead in 0..2 {
            let bed = &self.beds[lead];
            let Some(track) = bed.track else {
                continue;
            };
            let Some(overlap) = track.overlap() else {
                continue;
            };
            let length = track.stream().length;
            let overlap_frames = (overlap * RATE as f32) as usize;
            if bed.handed || bed.target <= 0.0 || bed.played + overlap_frames < length {
                continue;
            }
            let other = 1 - lead;
            if !self.beds[other].free() {
                continue;
            }
            self.beds[other].start(track, overlap);
            self.beds[lead].fade(0.0, overlap);
            self.beds[lead].handed = true;
        }
    }

    /// A new stinger in the request plays over the table's bed.
    fn cue(&mut self, request: ScoreRequest) {
        let seen = self.stingers.replace(request.stingers);
        if seen.is_none_or(|seen| seen == request.stingers) {
            return;
        }
        if request.recorded && request.scene == Scene::Table {
            let track = if request.stinger_epic {
                Track::StingerEpic
            } else {
                Track::StingerCall
            };
            self.stinger.start(track, STINGER_FADE_IN);
        }
    }
}

#[cfg(test)]
mod tests;
