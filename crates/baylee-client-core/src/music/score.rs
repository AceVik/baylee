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
#[allow(clippy::wildcard_imports)] // the score's note names and melodies, read as notation
use melodies::*;

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
            Self::FrontDoor => 0.36,
            Self::Lobby => 0.345,
            Self::Build | Self::Arrival | Self::Calm => 1.0 / 3.0,
            Self::Tension => 0.20 - 0.015 * f64::from(tension),
            Self::Hunt => 0.185,
            Self::Climax => 60.0 / 112.0 / 3.0,
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
            Self::Calm => 0.61,
            Self::Tension => 0.92,
            Self::Hunt | Self::Victory => 1.26,
            Self::Climax => 1.30,
            Self::Draw => 1.10,
            Self::Defeat => 1.0,
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
    fn target(&self) -> Layers {
        let t = self.request.tension;
        let pipe = pipe_level(t);
        let chanter = if t >= CHANTER_FROM { 1.0 } else { 0.0 };
        let full = Layers {
            drone: 1.0,
            bass: 1.0,
            ostinato: 1.0,
            melody: 1.0,
            drums: 1.0,
            pipe: 0.0,
            chanter: 0.0,
        };
        match self.texture {
            Texture::FrontDoor => Layers {
                drums: 0.0,
                ostinato: 0.4,
                bass: 0.6,
                ..full
            },
            Texture::Lobby => Layers { drums: 0.6, ..full },
            Texture::Build => Layers {
                melody: 0.0,
                drums: 0.0,
                ..full
            },
            Texture::Arrival => full,
            Texture::Calm => Layers {
                melody: if self.request.own_turn { 1.0 } else { 0.45 },
                drums: 0.7,
                pipe,
                ..full
            },
            Texture::Tension | Texture::Hunt => Layers {
                pipe,
                chanter,
                ..full
            },
            Texture::Climax | Texture::Victory => Layers {
                pipe: 1.0,
                chanter: 1.0,
                ..full
            },
            Texture::Draw | Texture::Defeat => Layers { pipe: 0.0, ..full },
        }
    }

    // ------------------------------------------------------------ the players

    /// Seconds in `ticks` sixteenths at the present tempo.
    fn seconds(&self, ticks: f32) -> f32 {
        ticks * self.eighth as f32 / 2.0
    }

    /// A small deterministic phrase shaping, not random detuning.
    fn human(&self) -> f32 {
        0.94 + 0.06 * ((self.bar * 7 + u64::from(self.tick) * 3) % 5) as f32 / 4.0
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
        let gain = gain * self.human() * self.texture.level();
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

    /// Whether the melody plays this bar: never in a pivot or a lit bar.
    fn singing(&self) -> bool {
        self.pivot.is_none() && !self.light
    }

    /// Plays the notes of one melody bar that start on this tick. `step`
    /// moves the line by scale steps (a fifth below is −4).
    fn line(&mut self, bar: &[(u8, u8)], family: Family, gain: f32, touch: Touch, step: i8) {
        let mut onset = 0u8;
        for &(pitch, eighths) in bar {
            if onset == self.tick && pitch > 0 {
                let pitch = self.shift(pitch, step);
                let ornament = eighths >= 3 && self.bar % 4 == 1 && onset == 0;
                if ornament {
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
        if self.texture.table() {
            self.pipes();
            self.horn_call();
            self.accents();
        }
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

    /// B♭ Lydian: the front door (the alto alone over drone and harp) and the
    /// lobby (with its ostinato and a muted frame drum).
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    fn lydian_home(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let lobby = self.texture == Texture::Lobby;
        let l = self.layers;
        let root = self.bass(FRONT_BASS[b % 8]);
        if t == 0 {
            self.bed(
                BB,
                &[
                    (bank::ORGAN, BB2, 0.055, 0.0),
                    (bank::LONGBOW, BB4, 0.035, -0.3),
                    (bank::LONGBOW, F5, 0.03, 0.3),
                ],
            );
            self.play(
                bank::CONTRABASS,
                root,
                11.5,
                0.11 * l.bass,
                Touch::at(0.05).attack(0.25),
            );
            if lobby && b % 2 == 1 {
                self.strike(at::FRAMEDRUM_MUTED, 60, 2.0, 0.08 * l.drums, Touch::at(0.1));
            }
        }
        // Harp: the whole Lydian figure in the lobby, two strings of it at
        // the front door.
        if t.is_multiple_of(2) {
            let k = usize::from(t / 2);
            let degree = [0, 7, 12, 14, 18, 12][k];
            if lobby || k == 0 || k == 3 {
                let gain = [0.13, 0.10, 0.11, 0.09, 0.10, 0.09][k] * l.ostinato.max(0.5);
                self.play(bank::HARP, BB3 + degree, 5.0, gain, Touch::at(-0.35));
            }
            if lobby {
                let pitch = [BB4, F5, BB5, F5, E5, F5][k];
                self.play(
                    bank::PSALTERY,
                    pitch,
                    3.0,
                    0.06 * l.ostinato,
                    Touch::at(0.4),
                );
            }
        }
        // The theme: 16 bars of it, 16 of drone and harp (32 at the lobby,
        // where it sings every other cycle).
        let cycle = if lobby { 64 } else { 32 };
        let pos = b % cycle;
        if pos < 16 && self.singing() {
            let phrase = if pos < 8 { FRONT_A } else { FRONT_B };
            self.line(
                phrase[pos % 8],
                bank::ALTO,
                0.24 * l.melody,
                Touch::at(0.15),
                0,
            );
        }
    }

    /// B♭ Ionian, to think over: harp and psaltery over the drone, no melody.
    fn build(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let l = self.layers;
        let root = self.bass(if b % 8 < 6 { BB1 } else { F2 });
        if t == 0 {
            self.bed(
                BB,
                &[
                    (bank::ORGAN, BB2, 0.06, 0.0),
                    (bank::LONGBOW, F5, 0.025, 0.3),
                ],
            );
            self.play(
                bank::CONTRABASS,
                root,
                11.5,
                0.10 * l.bass,
                Touch::at(0.05).attack(0.25),
            );
        }
        if t.is_multiple_of(2) {
            let k = usize::from(t / 2);
            let degree = [0, 7, 12, 16, 14, 7][k];
            let harp_root = if b % 8 < 6 { BB3 } else { F3 };
            self.play(
                bank::HARP,
                harp_root + degree,
                5.0,
                0.11 * l.ostinato,
                Touch::at(-0.35),
            );
        }
        if b % 4 == 3 && (6..10).contains(&t) {
            let pitch = [BB4, C5, D5, F5][usize::from(t - 6)];
            self.play(
                bank::PSALTERY,
                pitch,
                4.0,
                0.08 * l.ostinato,
                Touch::at(0.4),
            );
        }
    }

    /// The table arriving: a davul stroke and a bell on B♭, a harp sweep,
    /// then the drone holds through the loading cover.
    fn arrival(&mut self) {
        let t = self.tick;
        let l = self.layers;
        if t == 0 {
            self.bed(
                BB,
                &[
                    (bank::ORGAN, BB2, 0.07, 0.0),
                    (bank::LONGBOW, BB4, 0.03, -0.3),
                ],
            );
            let root = self.bass(BB1);
            self.play(
                bank::CONTRABASS,
                root,
                11.5,
                0.12 * l.bass,
                Touch::at(0.05).attack(0.3),
            );
        }
        match self.here {
            0 => {
                if t == 0 {
                    self.strike(at::DAVUL_FORTE, 60, 4.0, 0.26, Touch::at(0.0));
                    self.play(bank::CHIMES, BB4, 16.0, 0.09, Touch::at(0.25));
                }
                if t.is_multiple_of(2) {
                    let pitch = [BB2, C3, D3, F3, G3, BB3][usize::from(t / 2)];
                    self.play(bank::HARP, pitch + 12, 6.0, 0.12, Touch::at(-0.3));
                }
            }
            1 if t == 0 => {
                self.strike(at::DAVUL_1, 60, 3.0, 0.16, Touch::at(0.0));
                self.play(bank::CHIMES, F4, 16.0, 0.06, Touch::at(-0.25));
            }
            _ if t == 0 && self.here.is_multiple_of(2) => {
                self.play(bank::HARP, BB3, 8.0, 0.08, Touch::at(-0.3));
                self.play(bank::HARP, F4, 8.0, 0.06, Touch::at(-0.3));
            }
            _ => {}
        }
    }

    /// The table at rest, C Dorian in 6/8: strumstick and a drone that
    /// changes colour every 24 bars, the bass line on the contrabass, the
    /// harp's broken fifths, a muted frame drum, and the theme over a 48-bar
    /// cycle whose instrument turns by cycle (and by the monarch).
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    fn calm(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let l = self.layers;
        let reading = if (b / 48).is_multiple_of(2) {
            CALM_BASS
        } else {
            CALM_BASS_2
        };
        let normal = reading[b % 8];
        let root = self.bass(normal);
        if t == 0 {
            self.play(
                bank::STRUMSTICK,
                root + 12,
                11.5,
                0.17 * l.drone,
                Touch::at(-0.2),
            );
            match (b / 24) % 3 {
                0 => self.bed(C, &[(bank::ORGAN, C3, 0.05, 0.0)]),
                1 => self.bed(
                    C,
                    &[
                        (bank::LONGBOW, C5, 0.035, -0.2),
                        (bank::LONGBOW, G5, 0.025, 0.2),
                    ],
                ),
                _ => self.unbed(),
            }
            self.play(
                bank::CONTRABASS,
                root,
                12.2,
                0.13 * l.bass,
                Touch::at(0.05).attack(0.25),
            );
            if self.request.own_turn && b.is_multiple_of(2) {
                self.play(bank::PIZZICATO, root, 6.0, 0.11 * l.bass, Touch::at(0.1));
            }
            if b % 2 == 1 {
                self.strike(at::FRAMEDRUM_MUTED, 60, 2.0, 0.09 * l.drums, Touch::at(0.1));
            }
        }
        if t == 6 && b % 2 == 1 {
            self.strike(
                at::FRAMEDRUM_SMALL_MUTED,
                60,
                1.0,
                0.05 * l.drums,
                Touch::at(0.25),
            );
        }
        let pos = b % 48;
        let thin = pos >= 40;
        if t.is_multiple_of(2) && (!thin || t.is_multiple_of(6)) {
            let k = usize::from(t / 2);
            let degree = [0, 7, 12, 7, 14, 12][k];
            let gain = [0.15, 0.11, 0.12, 0.10, 0.11, 0.10][k];
            // A pivot moves only the bass; a lit bar moves the harp with it.
            let harp = if self.light { root } else { normal };
            self.play(
                bank::HARP,
                harp + 24 + degree,
                5.0,
                gain * l.ostinato,
                Touch::at(-0.35),
            );
        }
        // The theme in its 48 bars: A B, ostinato alone, A' B', drone and harp.
        let phrase = match pos / 8 {
            0 => Some(CALM_A),
            1 => Some(CALM_B),
            3 => Some(CALM_A2),
            4 => Some(CALM_B2),
            _ => None,
        };
        if let Some(phrase) = phrase.filter(|_| self.singing()) {
            let bar = phrase[pos % 8];
            let gain = 0.25 * l.melody;
            match ((self.bar / 48) as u8).wrapping_add(self.rotation) % 4 {
                0 => self.line(bar, bank::ALTO, gain, Touch::at(0.15), 0),
                1 => self.line(bar, bank::VIOLIN, gain * 0.55, Touch::at(-0.2), 0),
                2 => {
                    self.line(bar, bank::ALTO, gain, Touch::at(0.15), 0);
                    // Heterophony: the fiddle a sixth below, a breath late.
                    self.line(
                        bar,
                        bank::VIOLIN,
                        gain * 0.35,
                        Touch::at(-0.3).late(0.04),
                        -5,
                    );
                }
                _ => {
                    self.line(bar, bank::ALTO, gain, Touch::at(0.15), 0);
                    self.line(
                        bar,
                        bank::PSALTERY,
                        gain * 0.35,
                        Touch::at(0.4).late(0.03),
                        0,
                    );
                }
            }
        }
        // Answers at the ends of phrases, by whose turn it is.
        if (pos % 8 == 3 || pos % 8 == 7) && (6..10).contains(&t) && self.singing() {
            let pitch = [C5, D5, EB5, G5][usize::from(t - 6)];
            let gain = 0.11 * l.ostinato;
            match self.request.turn_seat % 3 {
                0 => self.play(bank::PSALTERY, pitch, 3.0, gain, Touch::at(0.4)),
                1 => self.play(bank::HARP, pitch, 3.0, gain, Touch::at(0.3)),
                _ => self.play(bank::VIOLIN, pitch - 12, 2.0, gain * 0.6, Touch::at(0.35)),
            }
        }
    }

    /// Rising tension, G Aeolian in 7/8 (3+2+2): psaltery ostinato over a
    /// lute, bowed fifth, contrabass on the accents, the tenor's line and the
    /// fiddle's (the chanter's from 0.55) in turn; the drums enter one by one
    /// by tension; from 0.7 the bass leans E♭–D at phrase ends (D Phrygian).
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    fn tension(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let l = self.layers;
        let tension = self.request.tension;
        let heavy = ((tension - 0.35) / 0.5).clamp(0.0, 1.0);
        let dark = tension >= 0.7 && b % 4 == 3 && self.singing();
        let root = self.bass(TENSION_BASS[b % 8]);
        let five = self.ticks == 10;
        // Drums, each at its threshold.
        let d = l.drums;
        match t {
            0 => {
                let drum = if b.is_multiple_of(2) {
                    at::FRAMEDRUM_1
                } else {
                    at::FRAMEDRUM_2
                };
                self.strike(drum, 60, 2.0, 0.21 * d, Touch::at(-0.1));
                if tension >= 0.5 {
                    let davul = if b.is_multiple_of(2) {
                        at::DAVUL_1
                    } else {
                        at::DAVUL_2
                    };
                    self.strike(davul, 60, 3.0, (0.12 + 0.14 * heavy) * d, Touch::at(0.0));
                }
            }
            4 => self.strike(at::FRAMEDRUM_SMALL_MUTED, 60, 1.0, 0.07 * d, Touch::at(0.3)),
            6 => {
                self.strike(at::FRAMEDRUM_SMALL, 60, 1.0, 0.15 * d, Touch::at(0.2));
                if tension >= 0.6 {
                    let naker = if b.is_multiple_of(2) {
                        at::NAKER_1
                    } else {
                        at::NAKER_2
                    };
                    self.strike(naker, 72, 1.5, (0.07 + 0.07 * heavy) * d, Touch::at(0.25));
                }
                if tension >= 0.7 {
                    self.strike(at::TAMBOURINE_HIT, 60, 1.0, 0.06 * d, Touch::at(-0.3));
                }
            }
            10 if !five => {
                self.strike(at::FRAMEDRUM_SMALL_MUTED, 60, 1.0, 0.12 * d, Touch::at(0.2));
                if tension >= 0.6 {
                    self.strike(
                        at::NAKER_HIGH,
                        74,
                        1.0,
                        (0.05 + 0.06 * heavy) * d,
                        Touch::at(0.3),
                    );
                }
                if tension >= 0.7 {
                    self.strike(at::TAMBOURINE_HIT, 60, 1.0, 0.05 * d, Touch::at(-0.3));
                }
            }
            11 if tension >= 0.7 && !five => {
                let rope = if b.is_multiple_of(2) {
                    at::ROPESNARE_1
                } else {
                    at::ROPESNARE_2
                };
                self.strike(rope, 60, 0.8, 0.06 * d, Touch::at(0.3));
            }
            _ => {}
        }
        // Bass: the contrabass on the 3+2+2 accents, the cello holding.
        if t == 0 {
            self.play(bank::PIZZICATO, root, 3.0, 0.16 * l.bass, Touch::at(0.05));
            self.play(
                bank::CELLO,
                root + 12,
                f32::from(self.ticks) + 0.3,
                0.065 * l.bass,
                Touch::at(-0.1).attack(0.2),
            );
            self.play(
                bank::STRUMSTICK,
                G3,
                f32::from(self.ticks) * 0.9,
                0.15 * l.drone,
                Touch::at(0.0),
            );
            self.bed(
                G,
                &[
                    (bank::LONGBOW, D5, 0.06, -0.2),
                    (bank::LONGBOW, G5, 0.04, 0.2),
                ],
            );
        }
        if t == 6 {
            let pitch = if dark { EB2 } else { root };
            self.play(bank::PIZZICATO, pitch, 3.0, 0.11 * l.bass, Touch::at(0.05));
        }
        if t == 10 && !five {
            let pitch = if dark { D2 } else { self.shift(root, 4) };
            self.play(bank::PIZZICATO, pitch, 3.0, 0.11 * l.bass, Touch::at(0.05));
            if dark {
                self.play(bank::LONGBOW, EB5, 2.0, 0.05 * l.drone, Touch::at(-0.1));
            }
        }
        if t == 12 && dark && !five {
            self.play(bank::LONGBOW, D5, 2.0, 0.05 * l.drone, Touch::at(-0.1));
        }
        // The plucked ostinato and the lute beneath it.
        if t.is_multiple_of(2) {
            let k = usize::from(t / 2);
            let pitch = TENSION_OSTINATO[k % TENSION_OSTINATO.len()];
            let gain = if matches!(k, 0 | 3 | 5) { 0.14 } else { 0.10 };
            self.play(
                bank::PSALTERY,
                pitch,
                3.0,
                gain * l.ostinato,
                Touch::at(0.35),
            );
            let gain = 0.09 * l.ostinato * self.texture.level();
            self.orchestra
                .lute(pitch - 24, self.seconds(5.0), gain, -0.35, 0.4);
        }
        // The line: the tenor, then the fiddle or the chanter, four bars each.
        if self.singing() && !five {
            let bar = TENSION_LINE[b % 4];
            if (b / 4).is_multiple_of(2) {
                self.line(bar, bank::TENOR, 0.24 * l.melody, Touch::at(0.1), 0);
            } else if l.chanter > 0.0 {
                let bar = TENSION_CHANTER[b % 4];
                self.line(
                    bar,
                    bank::CHANTER,
                    0.24 * l.chanter,
                    Touch::at(0.05).attack(0.015).release(0.05),
                    0,
                );
            } else {
                self.line(
                    bar,
                    bank::VIOLIN,
                    0.11 * l.melody,
                    Touch::at(-0.25).late(0.02),
                    7,
                );
            }
        }
    }

    /// The hunt: attackers are declared, F Mixolydian in a 6/8 jig; the horn
    /// calls (`horn_call`), the tenor answers, the chanter rides from 0.55.
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    fn hunt(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let l = self.layers;
        let tension = self.request.tension;
        let d = l.drums;
        let root = self.bass(F2);
        match t {
            0 => {
                let drum = if b.is_multiple_of(2) {
                    at::FRAMEDRUM_1
                } else {
                    at::FRAMEDRUM_2
                };
                self.strike(drum, 60, 2.0, 0.22 * d, Touch::at(-0.1));
                if tension >= 0.5 {
                    let davul = if b.is_multiple_of(2) {
                        at::DAVUL_1
                    } else {
                        at::DAVUL_2
                    };
                    self.strike(davul, 60, 3.0, 0.20 * d, Touch::at(0.0));
                }
                self.play(bank::PIZZICATO, root, 3.0, 0.16 * l.bass, Touch::at(0.05));
                self.play(
                    bank::CONTRABASS,
                    root,
                    12.2,
                    0.11 * l.bass,
                    Touch::at(0.05).attack(0.2),
                );
                self.play(bank::STRUMSTICK, F3, 10.0, 0.15 * l.drone, Touch::at(-0.1));
                self.bed(
                    F,
                    &[
                        (bank::LONGBOW, C5, 0.05, -0.2),
                        (bank::LONGBOW, F5, 0.04, 0.2),
                    ],
                );
            }
            4 | 10 => self.strike(at::FRAMEDRUM_SMALL_MUTED, 60, 1.0, 0.09 * d, Touch::at(0.3)),
            6 => {
                self.strike(at::FRAMEDRUM_SMALL, 60, 1.0, 0.16 * d, Touch::at(0.2));
                self.strike(at::TAMBOURINE_HIT, 60, 1.0, 0.06 * d, Touch::at(-0.3));
                self.play(bank::PIZZICATO, C2, 3.0, 0.12 * l.bass, Touch::at(0.05));
                if tension >= 0.6 {
                    self.strike(at::NAKER_1, 72, 1.5, 0.08 * d, Touch::at(0.25));
                }
            }
            11 if tension >= 0.7 => {
                self.strike(at::ROPESNARE_1, 60, 0.8, 0.06 * d, Touch::at(0.3));
            }
            _ => {}
        }
        if t.is_multiple_of(2) {
            let k = usize::from(t / 2);
            let pitch = [F5, A5, C6, A5, F5, C5][k];
            let gain = if k.is_multiple_of(3) { 0.13 } else { 0.09 };
            self.play(
                bank::PSALTERY,
                pitch,
                2.5,
                gain * l.ostinato,
                Touch::at(0.4),
            );
        }
        // After a call, the tenor answers it an octave up; the chanter, when
        // the pipe is up, rides the hunt.
        if self.horn.is_none() && self.singing() {
            let bar = HUNT[b % 4];
            if l.chanter > 0.0 {
                self.line(
                    bar,
                    bank::CHANTER,
                    0.22 * l.chanter,
                    Touch::at(0.05).attack(0.015).release(0.05),
                    0,
                );
            } else if b % 8 < 4 {
                self.line(bar, bank::TENOR, 0.18 * l.melody, Touch::at(0.15), 0);
            }
        }
    }

    /// The hunting-horn call: two bars on the natural horn's notes in F, on
    /// the bar after attackers are declared. This seat's own attack calls
    /// near and full; another's from further off. Never the same call twice
    /// running.
    fn horn_call(&mut self) {
        let Some((bar, mine)) = self.horn else {
            return;
        };
        let call = CALLS[(self.horn_calls as usize + CALLS.len() - 1) % CALLS.len()];
        let gain = if mine { 0.30 } else { 0.17 };
        let pan = if mine { 0.1 } else { -0.55 };
        self.line(
            call[usize::from(bar)],
            bank::HORN,
            gain,
            Touch::at(pan).attack(0.04),
            0,
        );
        if self.tick + 1 == self.ticks {
            self.horn = if bar == 0 { Some((1, mine)) } else { None };
        }
    }

    /// One-bar accents: the monarch's bell and harp, and a big spell's B♭
    /// Lydian light.
    fn accents(&mut self) {
        let t = self.tick;
        if self.monarch && t == 0 {
            self.strike(at::HANDBELL_1, 60, 6.0, 0.08, Touch::at(0.35));
        }
        if self.monarch && (2..8).contains(&t) {
            let pitch = [F4, G4, BB4, C5, D5, F5][usize::from(t - 2)];
            self.play(bank::HARP, pitch, 3.0, 0.09, Touch::at(-0.3));
        }
        if self.light && t < 6 {
            // B♭ C D E F, rising, and the bell: E♮ is the light.
            let pitch = [BB4, C5, D5, E5, F5, BB5][usize::from(t)];
            self.play(bank::HARP, pitch, 4.0, 0.10, Touch::at(-0.25));
            if t == 0 {
                self.strike(at::HANDBELL_2, 60, 6.0, 0.07, Touch::at(0.3));
                self.play(bank::CHIMES, BB4, 12.0, 0.05, Touch::at(0.2));
            }
        }
    }

    /// The climax: G Aeolian in a fast jig, both pipes and the davul.
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    fn climax(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let l = self.layers;
        let d = l.drums;
        let root = self.bass(G2);
        match t {
            0 => {
                let even = b.is_multiple_of(2);
                self.strike(
                    if even { at::DAVUL_1 } else { at::DAVUL_2 },
                    60,
                    3.0,
                    0.28 * d,
                    Touch::at(0.0),
                );
                self.strike(
                    if even {
                        at::FRAMEDRUM_1
                    } else {
                        at::FRAMEDRUM_2
                    },
                    60,
                    2.0,
                    0.24 * d,
                    Touch::at(-0.1),
                );
                if b >= 8 {
                    self.strike(at::FINGERCYMBAL, 60, 4.0, 0.045 * d, Touch::at(-0.3));
                }
                self.play(bank::PIZZICATO, root, 3.0, 0.17 * l.bass, Touch::at(0.05));
                self.play(
                    bank::CONTRABASS,
                    root,
                    12.2,
                    0.11 * l.bass,
                    Touch::at(0.05).attack(0.2),
                );
                self.play(
                    bank::CELLO,
                    D3,
                    12.2,
                    0.06 * l.bass,
                    Touch::at(-0.1).attack(0.2),
                );
                self.play(bank::STRUMSTICK, G3, 10.0, 0.16 * l.drone, Touch::at(0.0));
                self.bed(G, &[(bank::LONGBOW, D5, 0.04, -0.2)]);
                if b.is_multiple_of(8) {
                    self.play(bank::CHIMES, G4, 14.0, 0.07, Touch::at(0.2));
                }
            }
            4 | 10 => {
                self.strike(at::FRAMEDRUM_SMALL_MUTED, 60, 1.0, 0.10 * d, Touch::at(0.3));
                if t == 10 {
                    self.strike(at::NAKER_HIGH, 74, 1.0, 0.07 * d, Touch::at(0.3));
                }
            }
            6 => {
                self.strike(at::FRAMEDRUM_SMALL, 60, 1.0, 0.19 * d, Touch::at(0.2));
                let naker = if b.is_multiple_of(2) {
                    at::NAKER_1
                } else {
                    at::NAKER_2
                };
                self.strike(naker, 72, 1.5, 0.11 * d, Touch::at(0.25));
                self.strike(at::ROPESNARE_1, 60, 0.8, 0.08 * d, Touch::at(0.3));
                self.strike(at::TAMBOURINE_HIT, 60, 1.0, 0.065 * d, Touch::at(-0.3));
                self.play(bank::PIZZICATO, D2, 3.0, 0.12 * l.bass, Touch::at(0.05));
            }
            8 if b % 4 == 3 => {
                self.strike(at::TAMBOURINE_SHAKE, 60, 2.0, 0.05 * d, Touch::at(-0.3));
            }
            11 => self.strike(at::ROPESNARE_2, 60, 0.8, 0.05 * d, Touch::at(0.3)),
            _ => {}
        }
        if t.is_multiple_of(2) {
            let k = usize::from(t / 2);
            let pitch = [G5, D5, G5, D5, BB5, D5][k];
            let gain = if k.is_multiple_of(3) { 0.13 } else { 0.09 };
            self.play(
                bank::PSALTERY,
                pitch,
                2.5,
                gain * l.ostinato,
                Touch::at(0.4),
            );
        }
        if self.singing() && self.horn.is_none() {
            let pass = (b / 8) % 2;
            let bar = if (b / 16).is_multiple_of(2) {
                CLIMAX[b % 8]
            } else {
                CLIMAX_2[b % 8]
            };
            let pipe = Touch::at(0.05).attack(0.015).release(0.05);
            self.line(bar, bank::CHANTER, 0.28 * l.chanter, pipe, 0);
            self.line(
                bar,
                bank::VIOLIN,
                0.09 * l.melody,
                Touch::at(-0.3).late(0.035),
                7,
            );
            if pass == 1 {
                self.line(
                    bar,
                    bank::ALTO,
                    0.11 * l.melody,
                    Touch::at(0.3).late(0.01),
                    0,
                );
                self.line(
                    bar,
                    bank::TENOR,
                    0.09 * l.melody,
                    Touch::at(-0.15).late(0.01),
                    -4,
                );
            }
        }
    }

    /// The first two bars of an ending keep the last texture's pulse, thinner
    /// each bar: an ending grows out of what was playing.
    fn thin_drums(&mut self) {
        if self.here >= 2 || self.tick != 0 {
            return;
        }
        let level = if self.here == 0 { 0.6 } else { 0.3 };
        if matches!(
            self.from,
            Texture::Tension | Texture::Hunt | Texture::Climax
        ) {
            self.strike(at::FRAMEDRUM_1, 60, 2.0, 0.2 * level, Touch::at(-0.1));
        }
    }

    /// Victory in B♭: the pipes' drones re-pitched to B♭ and F, the chanter,
    /// recorders and fiddle on the rising cadence, the harp broadening, the
    /// davul and bells; it slows by 4 % a bar and its final rings.
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    fn victory(&mut self) {
        let t = self.tick;
        let k = self.here;
        self.thin_drums();
        if k == 0 && t == 0 {
            self.orchestra.release(PIPES);
            self.hold(PIPES, bank::DRONE, BB2, 0.12, Touch::at(0.0).attack(0.3));
            self.hold(PIPES, bank::DRONE, F3, 0.07, Touch::at(0.12).attack(0.3));
            self.pipes_held = None;
        }
        if k >= ENDING_BARS {
            self.after(
                BB,
                &[
                    (bank::ORGAN, BB2, 0.05, 0.0),
                    (bank::LONGBOW, F5, 0.025, 0.3),
                ],
            );
            if k == ENDING_BARS + 1 && t == 0 {
                self.orchestra.release(PIPES);
            }
            return;
        }
        let k = k as usize;
        if t == 0 {
            self.play(
                bank::CONTRABASS,
                BB1,
                12.3,
                0.14,
                Touch::at(0.05).attack(0.2),
            );
            self.play(
                bank::CELLO,
                F2 + 12,
                12.3,
                0.07,
                Touch::at(-0.1).attack(0.2),
            );
            self.strike(
                if k.is_multiple_of(2) {
                    at::DAVUL_1
                } else {
                    at::DAVUL_2
                },
                60,
                3.0,
                0.22,
                Touch::at(0.0),
            );
            self.strike(at::FRAMEDRUM_1, 60, 2.0, 0.15, Touch::at(-0.1));
            let bell = if k.is_multiple_of(2) {
                at::HANDBELL_2
            } else {
                at::HANDBELL_1
            };
            self.strike(bell, 60, 6.0, 0.06, Touch::at(0.3));
        }
        if t == 6 && k < 6 {
            self.strike(at::FRAMEDRUM_SMALL, 60, 1.0, 0.12, Touch::at(0.2));
            self.strike(at::NAKER_1, 72, 1.5, 0.08, Touch::at(0.25));
        }
        if t.is_multiple_of(2) {
            let degree = [0, 7, 12, 16, 19, 16][usize::from(t / 2)];
            self.play(bank::HARP, BB3 + degree, 6.0, 0.11, Touch::at(-0.35));
        }
        let last = k + 1 == ENDING_BARS as usize;
        let release = if last { 3.0 } else { 0.3 };
        let bar = VICTORY[k];
        self.line(
            bar,
            bank::CHANTER,
            0.27,
            Touch::at(0.05)
                .attack(0.015)
                .release(if last { 2.5 } else { 0.05 }),
            0,
        );
        self.line(
            bar,
            bank::ALTO,
            0.12,
            Touch::at(0.3).late(0.01).release(release),
            0,
        );
        self.line(
            bar,
            bank::VIOLIN,
            0.09,
            Touch::at(-0.3).late(0.035).release(release),
            7,
        );
        self.line(
            bar,
            bank::TENOR,
            0.09,
            Touch::at(-0.15).late(0.01).release(release),
            -7,
        );
        if t == 0 && k == 3 {
            self.play(bank::CHIMES, BB4, 16.0, 0.08, Touch::at(0.2));
        }
        if t == 0 && last {
            self.play(bank::CHIMES, BB4, 24.0, 0.13, Touch::at(0.2));
            self.strike(at::SLEIGH, 60, 6.0, 0.05, Touch::at(0.3));
            self.strike(at::DAVUL_FORTE, 60, 6.0, 0.28, Touch::at(0.0));
        }
        if t == 2 && last {
            self.play(bank::CHIMES, D5, 24.0, 0.09, Touch::at(-0.2));
        }
    }

    /// The draw: the pipe falls away, an open fifth F–C under the alto, whose
    /// line ends on C, unresolved; in 3/4, slowing by 6 % a bar.
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    fn draw(&mut self) {
        let t = self.tick;
        let k = self.here;
        self.thin_drums();
        if k == 0 && t == 0 {
            self.orchestra.release(PIPES);
            self.pipes_held = None;
        }
        if k >= ENDING_BARS {
            self.after(
                F,
                &[(bank::CELLO, F3, 0.06, -0.2), (bank::VIOLIN, C4, 0.03, 0.2)],
            );
            return;
        }
        let k = k as usize;
        if t == 0 {
            self.play(
                bank::CELLO,
                F3,
                12.6,
                0.09,
                Touch::at(-0.2).attack(if k == 0 { 0.5 } else { 0.1 }),
            );
            self.play(
                bank::VIOLIN,
                C4,
                12.6,
                0.045,
                Touch::at(0.2).attack(if k == 0 { 0.5 } else { 0.1 }),
            );
            self.play(
                bank::CONTRABASS,
                F2,
                12.3,
                0.12,
                Touch::at(0.05).attack(0.3),
            );
            if k < 4 {
                self.strike(
                    at::FRAMEDRUM_MUTED,
                    60,
                    2.0,
                    0.09 * (1.0 - k as f32 / 4.0),
                    Touch::at(0.1),
                );
            }
        }
        if t.is_multiple_of(4) {
            let degree = [0, 7, 12][usize::from(t / 4)];
            let gain = 0.12 * (1.0 - k as f32 / 10.0) + 0.03;
            self.play(bank::HARP, F3 + degree, 8.0, gain, Touch::at(-0.35));
        }
        let last = k + 1 == ENDING_BARS as usize;
        let touch = Touch::at(0.15)
            .late(0.01)
            .release(if last { 3.0 } else { 0.4 });
        self.line(DRAW[k], bank::ALTO, 0.21, touch, 0);
        self.line(DRAW[k], bank::PSALTERY, 0.07, Touch::at(0.4), 0);
        if t == 0 && last {
            self.strike(at::HANDBELL_1, 60, 8.0, 0.06, Touch::at(0.3));
        }
    }

    /// The defeat: a slow davul heartbeat that dies away, the contrabass and
    /// cellos on G and D, the harp in halves, the tenor descending to G with
    /// the fiddle beneath; one low bell. In 3/4, slowing by 7 % a bar.
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    fn defeat(&mut self) {
        let t = self.tick;
        let k = self.here;
        self.thin_drums();
        if k == 0 && t == 0 {
            self.orchestra.release(PIPES);
            self.pipes_held = None;
        }
        if k >= ENDING_BARS {
            self.after(
                G,
                &[
                    (bank::CONTRABASS, G2, 0.07, 0.0),
                    (bank::CELLO, D3, 0.05, -0.2),
                ],
            );
            return;
        }
        let k = k as usize;
        if t == 0 {
            self.play(
                bank::CONTRABASS,
                G2,
                12.6,
                0.12,
                Touch::at(0.05).attack(0.3),
            );
            self.play(
                bank::CELLO,
                D3,
                12.6,
                0.08,
                Touch::at(-0.2).attack(if k == 0 { 0.6 } else { 0.1 }),
            );
            if k < 6 {
                let fade = 1.0 - k as f32 / 7.0;
                self.strike(at::DAVUL_1, 60, 4.0, 0.15 * fade, Touch::at(0.0));
            }
        }
        if t == 4 && k < 6 {
            self.strike(
                at::DAVUL_2,
                60,
                4.0,
                0.08 * (1.0 - k as f32 / 7.0),
                Touch::at(0.0),
            );
        }
        if k.is_multiple_of(2) && (t == 0 || t == 6) {
            self.play(
                bank::HARP,
                if t == 0 { G3 } else { D4 },
                10.0,
                if t == 0 { 0.10 } else { 0.07 },
                Touch::at(-0.3),
            );
        }
        let last = k + 1 == ENDING_BARS as usize;
        let release = if last { 3.5 } else { 0.4 };
        self.line(
            DEFEAT[k],
            bank::TENOR,
            0.22,
            Touch::at(0.1).late(0.01).release(release),
            0,
        );
        self.line(
            DEFEAT[k],
            bank::VIOLIN,
            0.055,
            Touch::at(-0.3).late(0.04).release(release),
            0,
        );
        if t == 0 && last {
            self.play(bank::CHIMES, G4, 24.0, 0.08, Touch::at(0.2));
        }
    }

    /// After an ending's cadence: its last chord held, softly, until the
    /// player leaves the result.
    fn after(&mut self, root: u8, voices: &[(Family, u8, f32, f32)]) {
        if self.tick == 0 {
            self.bed(root, voices);
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
