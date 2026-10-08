//! Baylee's score: one pitch set around B♭, performed live by one sampler and
//! one musical clock (`docs/client.md` §"One orchestra follows the player").
//!
//! Every scene lives on the B♭ set (B♭ C D E♭ F G A); where the drone stands
//! decides the mode, and the drone moves only at a bar line, after a pivot bar
//! whose bass already plays the new final's fifth and whose melody rests. E
//! is the one movable degree: E♮ over a B♭ drone is B♭ Lydian, the front
//! door's and the lobby's wonder, and one bar of it lights a big spell.
//!
//! | Texture | Final, mode | Metre |
//! |---|---|---|
//! | front door, lobby | B♭ Lydian | 6/8 |
//! | deck building, arrival | B♭ Ionian | 6/8 |
//! | the table at rest | C Dorian | 6/8 |
//! | rising tension (dark: D Phrygian at phrase ends) | G Aeolian | 7/8 (3+2+2) |
//! | the hunt (attackers declared; the horn call) | F Mixolydian | 6/8 jig |
//! | the climax | G Aeolian | 6/8 jig |
//! | victory / draw / defeat | B♭ / F–C open fifth / G Aeolian | 6/8, 3/4, 3/4 |
//!
//! Deterministic: what plays is a function of the bar, the tick and the
//! requests admitted at bar lines, so a replayed game sounds the same.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)] // bounded MIDI/tick/sample conversions; t, b, k, l, d: tick, bar, ending bar, layers, drums
use super::{
    RATE,
    bank::{self, at},
    direct::{Scene, ScoreRequest},
    orchestra::{BLOCK, Family, Orchestra, Touch},
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

mod melodies;
mod orchestral;
mod textures;
mod themes;
#[allow(clippy::wildcard_imports)] // the score's note names and melodies, read as notation
use melodies::*;

pub use themes::Theme;
use themes::Voicing;

/// Lock-free message from the game to the conductor: a [`ScoreRequest`] in
/// one word, an atomic snapshot also on wasm without threads.
#[derive(Debug, Default)]
pub struct ScoreControl(AtomicU64);
impl ScoreControl {
    /// Asks for this. A tension that is no number is no tension.
    pub fn set(&self, request: ScoreRequest) {
        self.0.store(request.pack(), Ordering::Relaxed);
    }
    /// The latest request.
    #[must_use]
    pub fn get(&self) -> ScoreRequest {
        ScoreRequest::unpack(self.0.load(Ordering::Relaxed))
    }
}

/// What the score is playing, decided at bar lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Texture {
    FrontDoor,
    Lobby,
    Build,
    Arrival,
    Calm,
    Tension,
    Hunt,
    Climax,
    Victory,
    Draw,
    Defeat,
}

impl Texture {
    /// The final's pitch class.
    const fn final_class(self) -> u8 {
        match self {
            Self::FrontDoor | Self::Lobby | Self::Build | Self::Arrival | Self::Victory => BB,
            Self::Calm => C,
            Self::Tension | Self::Climax | Self::Defeat => G,
            Self::Hunt | Self::Draw => F,
        }
    }
    /// Whether E is natural (B♭ Lydian) rather than flat.
    const fn lydian(self) -> bool {
        matches!(self, Self::FrontDoor | Self::Lobby)
    }
    /// Sixteenths in a bar: 6/8 and 3/4 hold twelve, 7/8 fourteen.
    const fn ticks(self) -> u8 {
        if matches!(self, Self::Tension) {
            14
        } else {
            12
        }
    }
    const fn ending(self) -> bool {
        matches!(self, Self::Victory | Self::Draw | Self::Defeat)
    }
    /// The eighth's length this texture settles at, in seconds.
    fn eighth(self, tension: f32) -> f64 {
        match self {
            // Owner, 08.10.2026: noticeably faster, the table and the lobby
            // most; the hunt and the climax stay the fastest.
            Self::FrontDoor => 0.29,
            Self::Lobby | Self::Build | Self::Arrival => 0.27,
            // The table's dance: a dotted quarter of 78.
            Self::Calm => 0.255,
            Self::Tension => 0.166 - 0.018 * f64::from(tension),
            Self::Hunt => 0.152,
            Self::Climax => 60.0 / 134.0 / 3.0,
            Self::Victory => 0.19,
            Self::Draw => 0.21,
            Self::Defeat => 0.22,
        }
    }
    /// The texture's overall level, set by measurement so each scene sits at
    /// the loudness the design asks for (RMS at master 1.0: calm −23 dBFS,
    /// tension −20, the hunt −19, the climax −17, the endings −19 to −21;
    /// `every_scene_is_finite_audible_and_within_its_loudness`).
    const fn level(self) -> f32 {
        match self {
            Self::FrontDoor => 0.56,
            Self::Lobby => 0.60,
            Self::Build => 0.84,
            Self::Arrival => 0.70,
            Self::Calm => 0.52,
            Self::Tension => 0.92,
            Self::Hunt | Self::Victory => 1.26,
            Self::Climax => 1.30,
            Self::Draw => 0.95,
            Self::Defeat => 0.85,
        }
    }
    /// The textures a table plays.
    const fn table(self) -> bool {
        matches!(self, Self::Calm | Self::Tension | Self::Hunt | Self::Climax)
    }
}

/// How many bars an ending's cadence takes before it holds its last chord.
const ENDING_BARS: u32 = 8;
/// The fewest bars between two horn calls: a hunt is an event, not a habit.
const HORN_REST: u64 = 16;
/// A climax longer than this takes a breath of tension texture.
const CLIMAX_RUN: u32 = 64;
/// How long the breath is.
const BREATH: u32 = 8;

/// The five layers and the pipe's two, each a gain slewed per bar.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Layers {
    drone: f32,
    bass: f32,
    ostinato: f32,
    melody: f32,
    drums: f32,
    pipe: f32,
    chanter: f32,
    /// The orchestral body: the string sections' held chord, the spiccato
    /// drive, the brass, the big drums.
    strings: f32,
    drive: f32,
    brass: f32,
    perc: f32,
}

impl Layers {
    /// One bar's step towards `target`: up over two bars, down over three.
    fn toward(&mut self, target: Self) {
        let step = |now: &mut f32, want: f32| {
            *now += (want - *now).clamp(-0.34, 0.5);
        };
        step(&mut self.drone, target.drone);
        step(&mut self.bass, target.bass);
        step(&mut self.ostinato, target.ostinato);
        step(&mut self.melody, target.melody);
        step(&mut self.drums, target.drums);
        step(&mut self.pipe, target.pipe);
        step(&mut self.chanter, target.chanter);
        step(&mut self.strings, target.strings);
        step(&mut self.drive, target.drive);
        step(&mut self.brass, target.brass);
        step(&mut self.perc, target.perc);
    }
}

/// The pipe's drones grow from a tension of 0.25 to full at 0.7.
fn pipe_level(tension: f32) -> f32 {
    ((tension - 0.25) / 0.45).clamp(0.0, 1.0)
}
/// The chanter joins from 0.55.
const CHANTER_FROM: f32 = 0.55;

/// Voice tags: a held bed is let go by its tag, not by waiting for it.
const PIPES: u8 = 1;
const BED: u8 = 2;

/// A note as the tests hear it: recording, pitch, texture, lit, bar.
#[cfg(test)]
type Played = (usize, u8, Texture, bool, u64);

/// Endless stereo performance. Its transport, voices and room outlive every
/// screen and game; only newly scheduled notes change the orchestration.
pub struct Tune {
    control: Arc<ScoreControl>,
    orchestra: Orchestra,
    texture: Texture,
    /// The bar now playing hands over to this texture at its end.
    pivot: Option<Texture>,
    /// Bars since the performance began, and since this texture began.
    bar: u64,
    here: u32,
    tick: u8,
    ticks: u8,
    until_tick: f64,
    eighth: f64,
    eighth_target: f64,
    /// Seconds over which the tempo approaches its target.
    inertia: f64,
    request: ScoreRequest,
    heard: ScoreRequest,
    layers: Layers,
    hot_bars: u32,
    climax_bars: u32,
    breath: u32,
    /// The horn: the bar of the last call, how many calls, and the call now
    /// sounding (its bar within the call and whether it is this seat's).
    horn_last: Option<u64>,
    horn_calls: u32,
    horn: Option<(u8, bool)>,
    /// This bar is lit by a big spell (B♭ Lydian), or rung by the monarch.
    light: bool,
    monarch: bool,
    /// Turns the melody instrument on, by the monarch changing.
    rotation: u8,
    /// The eighth an ending started from, for its ritardando.
    ending_from: f64,
    from: Texture,
    /// What the held beds are sounding, so they are struck again only when
    /// that changes: pitch and level.
    pipes_held: Option<(u8, f32)>,
    bed_held: Option<u8>,
    strings_held: Option<(u8, f32)>,
    low_held: Option<u8>,
    /// The theme the score sings, and whether this bar is the breath a
    /// change of theme takes before the new one sings.
    theme: Theme,
    theme_rest: bool,
    /// The bar's root, as the scene's bass line sets it: what the orchestral
    /// body voices its chord on.
    root: u8,
    #[cfg(test)]
    played: Vec<Played>,
    /// Frames rendered ahead for the sample iterator, and how many of them
    /// it has handed out (`Iterator for Tune`).
    ahead: Vec<[f32; 2]>,
    scratch: Box<[f32; BLOCK]>,
    ahead_len: usize,
    ahead_read: usize,
    right: Option<f32>,
}

impl Default for Tune {
    fn default() -> Self {
        Self::new()
    }
}

impl Tune {
    /// Start at the front door with a private conductor.
    #[must_use]
    pub fn new() -> Self {
        Self::with_control(Arc::new(ScoreControl::default()))
    }
    /// Start a performance whose future bars follow this conductor.
    #[must_use]
    pub fn with_control(control: Arc<ScoreControl>) -> Self {
        Self {
            control,
            orchestra: Orchestra::default(),
            texture: Texture::FrontDoor,
            pivot: None,
            bar: 0,
            here: 0,
            tick: 0,
            ticks: 12,
            until_tick: 0.0,
            eighth: Texture::FrontDoor.eighth(0.0),
            eighth_target: Texture::FrontDoor.eighth(0.0),
            inertia: 6.0,
            request: ScoreRequest::default(),
            heard: ScoreRequest::default(),
            layers: Layers::default(),
            hot_bars: 0,
            climax_bars: 0,
            breath: 0,
            horn_last: None,
            horn_calls: 0,
            horn: None,
            light: false,
            monarch: false,
            rotation: 0,
            ending_from: 0.19,
            from: Texture::FrontDoor,
            pipes_held: None,
            bed_held: None,
            strings_held: None,
            low_held: None,
            theme: Theme::default(),
            theme_rest: false,
            root: BB1,
            #[cfg(test)]
            played: Vec::new(),
            ahead: vec![[0.0; 2]; BLOCK],
            scratch: Box::new([0.0; BLOCK]),
            ahead_len: 0,
            ahead_read: 0,
            right: None,
        }
    }

    /// Render the next stereo frame, without allocation or I/O.
    ///
    /// The reference: [`Tune::render`] gives the same frames in runs, and is
    /// what the audio thread plays (`a_rendered_block_is_the_frames_it_replaces`).
    pub fn frame(&mut self) -> [f32; 2] {
        self.transport();
        self.advance();
        self.orchestra.frame()
    }

    /// The bar and tick clock at the start of a frame: a new tick schedules
    /// its notes.
    fn transport(&mut self) {
        if self.until_tick <= 0.0 {
            if self.tick == 0 {
                self.conduct();
            }
            self.schedule();
            self.tick += 1;
            if self.tick >= self.ticks {
                self.tick = 0;
                self.bar += 1;
                self.here += 1;
            }
            self.until_tick += f64::from(RATE) * self.eighth / 2.0;
        }
    }

    /// Runs the clock and the conductor for `frames` without rendering a
    /// sample: what the tests that read only the notes scheduled need. The
    /// voice list stops at its polyphony; nothing grows.
    #[cfg(test)]
    fn skim(&mut self, frames: usize) {
        for _ in 0..frames {
            self.transport();
            self.advance();
        }
    }

    /// One frame of tempo inertia and of the tick countdown.
    fn advance(&mut self) {
        self.eighth += (self.eighth_target - self.eighth) / (f64::from(RATE) * self.inertia);
        self.until_tick -= 1.0;
    }

    /// Renders `out.len()` frames, at most a block at a time: the same frames,
    /// bit for bit, as as many calls of [`Tune::frame`]. The run is cut at
    /// every tick, so a note starts on exactly the frame it would have.
    pub fn render(&mut self, out: &mut [[f32; 2]]) {
        let mut done = 0;
        while done < out.len() {
            self.transport();
            let due = (self.until_tick.ceil() as usize).max(1);
            let run = due.min(out.len() - done).min(BLOCK);
            for _ in 0..run {
                self.advance();
            }
            self.orchestra
                .render(&mut out[done..done + run], &mut self.scratch[..]);
            done += run;
        }
    }

    // ------------------------------------------------------------ the conductor

    /// The bar line: accents are taken, the texture is decided (through a
    /// pivot bar where the final moves), the layers step and the tempo aims.
    fn conduct(&mut self) {
        let request = self.control.get();
        self.request = request;
        self.light = false;
        self.monarch = false;
        // A new theme is taken on the bar line: one bar's breath, held notes
        // and the orchestra's chord ringing on, then the new theme from its
        // first bar.
        self.theme_rest = request.theme != self.theme;
        if self.theme_rest {
            self.theme = request.theme;
            if matches!(
                self.texture,
                Texture::FrontDoor | Texture::Lobby | Texture::Calm
            ) {
                self.here = 0;
            }
        }
        if let Some(next) = self.pivot.take() {
            self.enter(next);
        } else {
            let want = self.wanted(request);
            if want != self.texture {
                let pivot = want.final_class() != self.texture.final_class()
                    || want.lydian() != self.texture.lydian();
                if want.ending() || !pivot {
                    self.enter(want);
                } else {
                    self.pivot = Some(want);
                }
            }
        }
        // Accents, each a counter that moved.
        if request.hunts != self.heard.hunts {
            let rested = self
                .horn_last
                .is_none_or(|last| self.bar >= last + HORN_REST);
            if rested && (self.texture.table() || self.pivot.is_some_and(Texture::table)) {
                self.horn = Some((0, request.hunt_mine));
                self.horn_last = Some(self.bar);
                self.horn_calls += 1;
            }
        }
        if request.monarchs != self.heard.monarchs && self.texture.table() {
            self.monarch = true;
            self.rotation = self.rotation.wrapping_add(1);
        }
        if request.spells != self.heard.spells && self.texture.table() && self.pivot.is_none() {
            self.light = true;
        }
        self.heard = request;
        self.layers.toward(self.target());
        // The tempo: endings slow by a fixed share a bar from where they began.
        let tension = request.tension;
        self.eighth_target = match self.texture {
            Texture::Victory => self.ending_from * 1.04_f64.powi(self.here.min(ENDING_BARS) as i32),
            Texture::Draw => {
                Texture::Draw.eighth(0.0) * 1.06_f64.powi(self.here.min(ENDING_BARS) as i32)
            }
            Texture::Defeat => {
                Texture::Defeat.eighth(0.0) * 1.07_f64.powi(self.here.min(ENDING_BARS) as i32)
            }
            texture @ (Texture::FrontDoor
            | Texture::Lobby
            | Texture::Build
            | Texture::Arrival
            | Texture::Calm) => texture.eighth(tension) * self.theme.book().tempo,
            texture => texture.eighth(tension),
        };
        self.ticks = self.texture.ticks();
        if self.texture == Texture::Tension && request.about_to_lose && self.here % 4 == 3 {
            // The bar that drops a beat: 5/8.
            self.ticks = 10;
        }
    }

    /// Starts a texture at this bar.
    fn enter(&mut self, texture: Texture) {
        if texture.ending() {
            self.ending_from = self.eighth.min(0.21);
            self.from = self.texture;
            self.inertia = 1.5;
        } else {
            self.inertia = 6.0;
        }
        if texture != Texture::Climax {
            self.climax_bars = 0;
        }
        // Every texture strikes its own bed; the pipe's drones belong to the
        // table (and the victory, which re-pitches them itself).
        self.unbed();
        if !texture.table() && texture != Texture::Victory && self.pipes_held.take().is_some() {
            self.orchestra.release(PIPES);
        }
        self.texture = texture;
        self.here = 0;
    }

    /// The texture a request asks for, with the table's hysteresis: tension
    /// enters at 0.35 and leaves below 0.2; the climax enters after two bars
    /// at 0.8 (or at once on a lethal board) and leaves below 0.5; an ending
    /// finishes its cadence before anything else is heard.
    fn wanted(&mut self, request: ScoreRequest) -> Texture {
        if self.texture.ending() && self.here < ENDING_BARS {
            return self.texture;
        }
        let t = request.tension;
        match request.scene {
            Scene::FrontDoor => Texture::FrontDoor,
            Scene::Lobby => Texture::Lobby,
            Scene::Build => Texture::Build,
            Scene::Opening => Texture::Arrival,
            Scene::Victory if !self.texture.ending() => Texture::Victory,
            Scene::Draw if !self.texture.ending() => Texture::Draw,
            Scene::Defeat if !self.texture.ending() => Texture::Defeat,
            Scene::Victory | Scene::Draw | Scene::Defeat => self.texture,
            Scene::Table => {
                self.hot_bars = if t >= 0.8 { self.hot_bars + 1 } else { 0 };
                let climax = request.lethal
                    || self.hot_bars >= 2
                    || (self.texture == Texture::Climax && t >= 0.5)
                    || (self.breath > 0 && t >= 0.5);
                if climax {
                    if self.breath > 0 {
                        self.breath -= 1;
                        return Texture::Tension;
                    }
                    self.climax_bars += 1;
                    if self.climax_bars > CLIMAX_RUN {
                        self.climax_bars = 0;
                        self.breath = BREATH;
                    }
                    return Texture::Climax;
                }
                self.breath = 0;
                let tense = matches!(
                    self.texture,
                    Texture::Tension | Texture::Hunt | Texture::Climax
                );
                if request.combat {
                    Texture::Hunt
                } else if t >= 0.35 || tense && t >= 0.2 {
                    Texture::Tension
                } else {
                    Texture::Calm
                }
            }
        }
    }

    /// The layers' levels this texture and request ask for.
    #[allow(clippy::too_many_lines)] // one table of levels
    fn target(&self) -> Layers {
        let t = self.request.tension;
        let pipe = pipe_level(t);
        let chanter = if t >= CHANTER_FROM { 1.0 } else { 0.0 };
        let rise = |from: f32, span: f32| ((t - from) / span).clamp(0.0, 1.0);
        let full = Layers {
            drone: 1.0,
            bass: 1.0,
            ostinato: 1.0,
            melody: 1.0,
            drums: 1.0,
            pipe: 0.0,
            chanter: 0.0,
            strings: 1.0,
            drive: 1.0,
            brass: 1.0,
            perc: 1.0,
        };
        let quiet = Layers {
            drive: 0.0,
            brass: 0.0,
            perc: 0.0,
            ..full
        };
        match self.texture {
            Texture::FrontDoor => Layers {
                drums: 0.0,
                ostinato: 0.4,
                bass: 0.6,
                strings: 0.45,
                ..quiet
            },
            Texture::Lobby => Layers {
                drums: 0.6,
                strings: 0.55,
                drive: 0.2,
                ..quiet
            },
            Texture::Build => Layers {
                melody: 0.0,
                drums: 0.0,
                strings: 0.5,
                ..quiet
            },
            Texture::Arrival => Layers {
                strings: 0.8,
                brass: 0.4,
                perc: 0.7,
                drive: 0.0,
                ..full
            },
            Texture::Calm => {
                // The 48-bar arc, aimed a bar ahead so each layer has swelled
                // by the bar it belongs to.
                let (strings, drive, brass, perc) = match (self.here + 1) % 48 / 8 {
                    1 => (0.7, 0.3, 0.0, 0.0),
                    2 => (1.0, 0.6, 0.9, 0.45),
                    3 => (0.6, 0.5, 0.0, 0.0),
                    4 => (0.55, 0.15, 0.0, 0.0),
                    _ => (0.35, 0.0, 0.0, 0.0),
                };
                Layers {
                    melody: if self.request.own_turn { 1.0 } else { 0.45 },
                    drums: 0.7,
                    pipe,
                    strings,
                    drive,
                    brass,
                    perc,
                    ..full
                }
            }
            Texture::Tension => Layers {
                pipe,
                chanter,
                strings: 0.7 + 0.3 * rise(0.35, 0.5),
                drive: rise(0.35, 0.4),
                brass: rise(0.5, 0.3),
                perc: rise(0.45, 0.35),
                ..full
            },
            Texture::Hunt => Layers {
                pipe,
                chanter,
                strings: 0.8,
                brass: 0.4 + 0.6 * rise(0.4, 0.4),
                perc: 0.5 + 0.5 * rise(0.4, 0.4),
                ..full
            },
            Texture::Climax => Layers {
                pipe: 1.0,
                chanter: 1.0,
                ..full
            },
            Texture::Victory => Layers {
                pipe: 1.0,
                chanter: 1.0,
                drive: 0.3,
                perc: 0.5,
                ..full
            },
            Texture::Draw => Layers {
                strings: 0.6,
                ..quiet
            },
            Texture::Defeat => Layers {
                strings: 0.75,
                brass: 0.0,
                ..quiet
            },
        }
    }

    // ------------------------------------------------------------ the players

    /// Seconds in `ticks` sixteenths at the present tempo.
    fn seconds(&self, ticks: f32) -> f32 {
        ticks * self.eighth as f32 / 2.0
    }

    /// A player's touch: each note a little louder or softer than the last,
    /// by the bar and the tick (deterministic, never a random detuning).
    fn human(&self) -> f32 {
        0.90 + 0.10 * ((self.bar * 7 + u64::from(self.tick) * 3) % 5) as f32 / 4.0
    }

    /// The phrase's breath and the metre's stress: an eight-bar swell (about
    /// −2.6 dB where a phrase begins to +1.4 dB at its crest, and back), a
    /// downbeat that leans in, the half-bar (or 7/8's 3+2+2 accents) that
    /// answers, offbeat sixteenths that give way. An ending keeps its own.
    fn dynamics(&self) -> f32 {
        const SWELL: [f32; 8] = [0.74, 0.82, 0.92, 1.02, 1.12, 1.18, 1.05, 0.88];
        let arc = if self.texture.ending() {
            1.0
        } else {
            SWELL[(self.here % 8) as usize]
        };
        let stress = match self.tick {
            0 => 1.18,
            6 => 1.07,
            10 if self.ticks == 14 => 1.07,
            t if t % 2 == 1 => 0.86,
            _ => 1.0,
        };
        arc * stress
    }

    /// Plays `pitch` on the recording of `family` nearest to it, an octave
    /// nearer when a doubling would carry it more than three semitones off
    /// every recording.
    fn play(&mut self, family: Family, pitch: u8, ticks: f32, gain: f32, touch: Touch) {
        let mut pitch = pitch;
        let off = |p: u8| bank::BANK[family.nearest(p)].midi.abs_diff(p);
        if off(pitch) > 3 {
            if off(pitch - 12) < off(pitch) {
                pitch -= 12;
            } else if off(pitch + 12) < off(pitch) {
                pitch += 12;
            }
        }
        let instrument = family.nearest(pitch);
        self.strike(instrument, pitch, ticks, gain, touch);
    }

    /// Plays one recording.
    fn strike(&mut self, instrument: usize, pitch: u8, ticks: f32, gain: f32, touch: Touch) {
        if gain <= 0.001 {
            return;
        }
        #[cfg(test)]
        self.played
            .push((instrument, pitch, self.texture, self.light, self.bar));
        let seconds = self.seconds(ticks);
        let gain = gain * self.human() * self.dynamics() * self.texture.level();
        self.orchestra.note(instrument, pitch, seconds, gain, touch);
    }

    /// A held bed under a texture, tagged so it is let go when it changes.
    fn hold(&mut self, tag: u8, family: Family, pitch: u8, gain: f32, touch: Touch) {
        if gain <= 0.001 {
            return;
        }
        let instrument = family.nearest(pitch);
        #[cfg(test)]
        self.played
            .push((instrument, pitch, self.texture, self.light, self.bar));
        let gain = gain * self.texture.level();
        self.orchestra.held(instrument, pitch, gain, touch, tag);
    }

    /// The bass a texture plays this bar, unless the bar is a pivot: then the
    /// new final's fifth, in the same octave.
    fn bass(&self, normal: u8) -> u8 {
        let class = match self.pivot {
            Some(next) => (next.final_class() + 7) % 12,
            None if self.light => BB,
            None => return normal,
        };
        let mut pitch = normal - normal % 12 + class;
        if pitch > normal + 6 {
            pitch -= 12;
        } else if pitch + 6 < normal {
            pitch += 12;
        }
        pitch
    }

    /// Whether the melody plays this bar: never in a pivot, a lit bar or the
    /// breath a change of theme takes.
    fn singing(&self) -> bool {
        self.pivot.is_none() && !self.light && !self.theme_rest
    }

    /// Sings one bar of a phrase as a scene voices it (`themes::voice`).
    fn sing(
        &mut self,
        phrase: Phrase,
        bar: usize,
        family: Family,
        gain: f32,
        touch: Touch,
        v: Voicing,
    ) {
        let (notes, n) = themes::voice(phrase, bar, v, self.texture.lydian() || self.light);
        self.line(&notes[..n], family, gain, touch, 0);
    }

    /// Plays the notes of one melody bar that start on this tick. `step`
    /// moves the line by scale steps (a fifth below is −4).
    fn line(&mut self, bar: &[(u8, u8)], family: Family, gain: f32, touch: Touch, step: i8) {
        let mut onset = 0u8;
        for &(pitch, eighths) in bar {
            if onset == self.tick && pitch > 0 {
                let pitch = self.shift(pitch, step);
                let long = eighths >= 3;
                // An ornament's neighbours stay inside the instrument (the
                // chanter has nine notes and no more).
                let lowest = bank::BANK[family.first].midi;
                let highest = bank::BANK[family.first + family.len - 1].midi;
                let above = self.shift(pitch, 1) <= highest + 1;
                let mordent = above
                    && (long && (self.bar + u64::from(onset)) % 3 == 1
                        || eighths == 2 && self.bar % 4 == 1 && onset > 0);
                let turn = long
                    && above
                    && !mordent
                    && self.bar % 4 == 3
                    && self.shift(pitch, -1) + 1 >= lowest;
                if turn {
                    // A turn: the note, above, the note, below, the note.
                    let upper = self.shift(pitch, 1);
                    let lower = self.shift(pitch, -1);
                    let sixteenth = self.seconds(1.0);
                    for (k, note) in [pitch, upper, pitch, lower].into_iter().enumerate() {
                        let late = touch.late(sixteenth * k as f32);
                        self.play(
                            family,
                            note,
                            0.9,
                            gain * if k == 0 { 1.0 } else { 0.8 },
                            late,
                        );
                    }
                    let rest = f32::from(eighths) * 2.0 - 4.2;
                    self.play(family, pitch, rest, gain, touch.late(sixteenth * 4.0));
                } else if mordent {
                    // A mordent: the note, its upper neighbour, the note.
                    let upper = self.shift(pitch, 1);
                    self.play(family, pitch, 0.9, gain, touch);
                    self.play(
                        family,
                        upper,
                        0.9,
                        gain * 0.85,
                        touch.late(self.seconds(1.0)),
                    );
                    self.play(
                        family,
                        pitch,
                        f32::from(eighths) * 2.0 - 2.2,
                        gain,
                        touch.late(self.seconds(2.0)),
                    );
                } else {
                    self.play(family, pitch, f32::from(eighths) * 2.0 * 0.95, gain, touch);
                }
            }
            onset += eighths * 2;
        }
    }

    /// `pitch` moved by `steps` within the set this bar sounds in.
    fn shift(&self, pitch: u8, steps: i8) -> u8 {
        let lydian = self.texture.lydian() || self.light;
        let mut pitch = pitch;
        let in_set = |p: u8| set_holds(lydian, p);
        for _ in 0..steps.unsigned_abs() {
            loop {
                pitch = if steps > 0 { pitch + 1 } else { pitch - 1 };
                if in_set(pitch) {
                    break;
                }
            }
        }
        pitch
    }

    fn schedule(&mut self) {
        match self.texture {
            Texture::FrontDoor | Texture::Lobby => self.lydian_home(),
            Texture::Build => self.build(),
            Texture::Arrival => self.arrival(),
            Texture::Calm => self.calm(),
            Texture::Tension => self.tension(),
            Texture::Hunt => self.hunt(),
            Texture::Climax => self.climax(),
            Texture::Victory => self.victory(),
            Texture::Draw => self.draw(),
            Texture::Defeat => self.defeat(),
        }
        self.orchestral();
        if !self.texture.ending() && self.texture != Texture::Arrival {
            self.fill();
            self.run();
        }
        if self.texture.table() {
            self.pipes();
            self.horn_call();
            self.accents();
        }
    }

    /// A drum fill into the next phrase: the last three sixteenths of every
    /// fourth bar, growing; every eighth bar of the tense textures adds the
    /// davul and a naker.
    fn fill(&mut self) {
        let drums = self.layers.drums;
        let n = self.ticks;
        if drums < 0.05 || self.here % 4 != 3 || self.tick + 3 < n {
            return;
        }
        let k = usize::from(self.tick + 3 - n);
        let big = self.here % 8 == 7
            && matches!(
                self.texture,
                Texture::Tension | Texture::Hunt | Texture::Climax
            );
        let gain = [0.05, 0.07, 0.10][k] * drums * if big { 1.6 } else { 1.0 };
        let drum = if k == 1 {
            at::FRAMEDRUM_SMALL_MUTED
        } else {
            at::FRAMEDRUM_SMALL
        };
        self.strike(drum, 60, 0.8, gain, Touch::at(0.15 + 0.1 * k as f32));
        if big && k == 2 {
            self.strike(at::DAVUL_2, 60, 2.0, 0.13 * drums, Touch::at(0.0));
            self.strike(at::NAKER_HIGH, 74, 1.0, 0.07 * drums, Touch::at(0.3));
        }
        // The orchestra's toms under the frame drums as the big drums rise.
        let perc = self.layers.perc;
        if perc > 0.3 && k > 0 {
            let tom = if k == 1 { at::TOM_HIGH } else { at::TOM_LOW };
            self.strike(tom, 60, 1.0, 0.11 * perc, Touch::at(-0.2 + 0.3 * k as f32));
        }
    }

    /// A running passage on the harp into every eighth bar's line: five
    /// sixteenths climbing the scale to the final.
    fn run(&mut self) {
        const COUNT: u8 = 5;
        if self.here % 8 != 7 || !self.singing() || self.tick + COUNT < self.ticks {
            return;
        }
        let target = match self.texture {
            Texture::Calm => C5,
            Texture::Tension | Texture::Climax => G4,
            Texture::Hunt => F4,
            _ => BB4,
        };
        let k = self.tick + COUNT - self.ticks;
        let pitch = self.shift(target, k as i8 - COUNT as i8);
        let gain = (0.06 + 0.015 * f32::from(k)) * self.layers.ostinato;
        self.play(bank::HARP, pitch, 1.6, gain, Touch::at(0.35));
    }

    /// The pipe's drones, under every table texture by the tension's level:
    /// at their recorded G, re-pitched to F for the hunt.
    fn pipes(&mut self) {
        if self.tick != 0 {
            return;
        }
        let level = self.layers.pipe;
        let root = match self.pivot.unwrap_or(self.texture) {
            Texture::Hunt => F2,
            _ => G2,
        };
        let held = self.pipes_held;
        let moved = held.is_none_or(|(pitch, gain)| pitch != root || (gain - level).abs() > 0.12);
        if level < 0.02 {
            if held.is_some() {
                self.orchestra.release(PIPES);
                self.pipes_held = None;
            }
            return;
        }
        if moved {
            self.orchestra.release(PIPES);
            self.hold(
                PIPES,
                bank::DRONE,
                root,
                0.13 * level,
                Touch::at(0.0).attack(0.8),
            );
            self.hold(
                PIPES,
                bank::DRONE,
                root + 12,
                0.08 * level,
                Touch::at(0.12).attack(0.8),
            );
            self.pipes_held = Some((root, level));
        }
    }

    /// A held bed (organ, bowed psaltery) on `pitches`, struck again only
    /// when its root moves.
    fn bed(&mut self, root: u8, voices: &[(Family, u8, f32, f32)]) {
        if self.bed_held == Some(root) {
            return;
        }
        self.orchestra.release(BED);
        for &(family, pitch, gain, pan) in voices {
            self.hold(
                BED,
                family,
                pitch,
                gain * self.layers.drone,
                Touch::at(pan).attack(0.6),
            );
        }
        self.bed_held = Some(root);
    }

    /// Lets the held bed go.
    fn unbed(&mut self) {
        if self.bed_held.take().is_some() {
            self.orchestra.release(BED);
        }
    }
}

impl Iterator for Tune {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        if self.ahead_read == self.ahead_len {
            // Taken and put back, never reallocated: an empty `Vec` costs
            // nothing, and this runs on the audio thread.
            let mut ahead = std::mem::take(&mut self.ahead);
            self.render(&mut ahead);
            self.ahead = ahead;
            self.ahead_len = BLOCK;
            self.ahead_read = 0;
        }
        let [left, right] = self.ahead[self.ahead_read];
        self.ahead_read += 1;
        self.right = Some(right);
        Some(left)
    }
}

#[cfg(test)]
mod tests;
