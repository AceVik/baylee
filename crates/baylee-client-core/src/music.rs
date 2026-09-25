//! The music before the table (#296): a tune of our own, played by a
//! tracker's voices and synthesised here, one sample at a time.
//!
//! # Whose it is
//!
//! Every note below was written for this client, and every sound is
//! arithmetic: a square lead with its echo, a harp of pulse and triangle
//! breaking the chords, a triangle bass and a frame drum whose skin is three
//! falling sines. There is no sample, no module file from the demo scene and
//! no recording in it, and it quotes no tune: the style is the cracktros' and
//! the trainers', the notes are not (`docs/legal.md`).
//!
//! # What it is
//!
//! Thirty-two bars in 6/8, in D minor with the Dorian sixth for a colour of
//! magic (the G major of bar 10), at a dotted crotchet of 63: a slow lilt,
//! which is what keeps it an adventure and out of techno, where the beat
//! would be four even kicks. Four bars of chords and bass, a theme twice
//! over, a brighter middle in F with the drum under it, and a turn on A that
//! leads back to the first bar.
//!
//! It plays under the lobby and the builder as long as a player stays there
//! (#296, the owner, 25.09.): slow, and with every note ringing on under the
//! next, so a minute of it is company and not a signal.
//!
//! # Why it streams
//!
//! [`Tune`] is an endless iterator of stereo samples, not a buffer. The
//! audio thread pulls it, so none of it is computed on a frame, and nothing
//! is held: a minute of stereo would be ten megabytes of `f32` for music
//! that may play for a minute or an hour. And a loop that is *played*,
//! rather than repeated, has no seam to hide: the echo and the last note's
//! release ring on into the first bar, as they would if a band played it
//! twice.

/// Samples per second, per channel.
pub const RATE: u32 = 44_100;

/// Two, interleaved left then right.
pub const CHANNELS: u16 = 2;

/// One tracker row, a semiquaver: a dotted crotchet of 63.
///
/// A whole number of samples, so the loop is too, and the wrap lands on a
/// sample rather than between two.
const ROW: u32 = 7_000;

/// A bar of 6/8 in semiquavers.
const ROWS_PER_BAR: u32 = 12;

/// One beat, a dotted crotchet: two to a bar.
const BEAT: u32 = 6 * ROW;

/// Bars in the loop.
const BARS: usize = 32;

/// The whole tune, in samples per channel: 61 seconds.
pub const LOOP: u32 = ROW * ROWS_PER_BAR * BARS as u32;

/// Where the loop goes back to: the theme, bar 5. The four bars of chords
/// before it open the tune once, as a tracker's restart position leaves a
/// song's introduction behind; the last bar is a pickup into the theme.
pub const RESTART: u32 = ROW * ROWS_PER_BAR * 4;

/// The part that repeats, bars 5 to 32: 53.3 seconds.
pub const REPEATS: u32 = LOOP - RESTART;

/// The harmony, one chord per bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Chord {
    Dm,
    Gm,
    Bb,
    C,
    F,
    G,
    A,
}

impl Chord {
    /// The root, as a MIDI note in the harp's octave (G3 to F4).
    const fn root(self) -> u8 {
        match self {
            Self::Dm => 62,
            Self::Gm | Self::G => 55,
            Self::Bb => 58,
            Self::C => 60,
            Self::F => 65,
            Self::A => 57,
        }
    }

    /// Semitones from the root to the third.
    const fn third(self) -> u8 {
        match self {
            Self::Dm | Self::Gm => 3,
            Self::Bb | Self::C | Self::F | Self::G | Self::A => 4,
        }
    }

    /// Whether the harp turns on the major seventh rather than the octave:
    /// on the two chords that take it without a fight with the lead (B flat
    /// and F), where it is the shimmer the style calls magic.
    const fn seventh(self) -> bool {
        matches!(self, Self::Bb | Self::F)
    }

    /// The bass's root, in F2 to E3: the harp's root an octave or two down.
    const fn bass(self) -> u8 {
        let root = self.root() - 12;
        if root > 52 { root - 12 } else { root }
    }
}

use Chord::{A, Bb, C, Dm, F, G, Gm};

/// The harmony, bar by bar.
const CHORDS: [Chord; BARS] = [
    // Four bars of chords and bass.
    Dm, Bb, C, A, //
    // The theme.
    Dm, Bb, F, C, Dm, G, Bb, A, //
    // Again, higher, with the drum's deep stroke.
    Dm, Bb, F, C, Gm, Bb, A, Dm, //
    // The middle, in F, with the whole drum.
    F, C, Dm, Bb, F, C, Gm, A, //
    // The turn back to the first bar.
    Bb, C, A, A,
];

/// The lead, bar by bar: a note and its length in rows (semiquavers).
/// An empty bar rests.
const LEAD: [&str; BARS] = [
    "",
    "",
    "",
    "",
    "A4:6 D5:2 E5:2 F5:2",
    "G5:4 F5:2 D5:6",
    "C5:2 D5:2 C5:2 A4:6",
    "G4:2 A4:2 C5:2 E5:6",
    "F5:6 E5:2 D5:2 E5:2",
    "D5:4 B4:2 G4:6",
    "Bb4:2 D5:2 F5:2 G5:4 F5:2",
    "E5:6 C#5:6",
    "D5:6 A5:6",
    "Bb5:4 A5:2 F5:6",
    "A5:2 G5:2 F5:2 C5:4 F5:2",
    "E5:6 C6:4 A5:2",
    "G5:4 Bb5:2 A5:2 G5:2 F5:2",
    "F5:4 D5:2 Bb4:6",
    "C#5:2 E5:2 A5:2 G5:4 E5:2",
    "D5:12",
    "A5:6 C6:4 A5:2",
    "G5:6 E5:4 C5:2",
    "D5:2 E5:2 F5:2 A5:2 G5:2 F5:2",
    "D5:6 F5:6",
    "A5:6 C6:4 D6:2",
    "C6:6 G5:4 E5:2",
    "F5:2 G5:2 Bb5:2 D6:4 Bb5:2",
    "A5:6 E5:6",
    "F5:4 D5:2 Bb4:4 D5:2",
    "E5:4 G5:2 C6:6",
    "A5:6 E5:2 C#5:2 E5:2",
    "A4:6 E4:2 F4:2 G4:2",
];

/// Which bars the drum plays in: none in the opening, the deep stroke alone
/// under the theme's second time, the whole drum in the middle, and a breath
/// in the last bar so the turn is heard.
fn drum_in(bar: usize) -> Drum {
    match bar {
        12..=19 => Drum::Deep,
        20..=30 => Drum::Whole,
        _ => Drum::None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Drum {
    None,
    /// The deep stroke on each bar's first beat.
    Deep,
    /// That, a lighter one on the second beat, and fingers on the rim.
    Whole,
}

/// A note of a voice, in samples from the top of the loop.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Note {
    start: u32,
    len: u32,
    hz: f32,
}

/// A MIDI note from its name, "C#5" or "Bb4".
fn midi(name: &str) -> Option<u8> {
    let mut chars = name.chars();
    let class = match chars.next()? {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest = chars.as_str();
    let (shift, octave) = match rest.as_bytes().first()? {
        b'#' => (1, &rest[1..]),
        b'b' => (-1, &rest[1..]),
        _ => (0, rest),
    };
    let octave: i32 = octave.parse().ok()?;
    u8::try_from(12 * (octave + 1) + class + shift).ok()
}

fn hz(midi: u8) -> f32 {
    440.0 * ((f32::from(midi) - 69.0) / 12.0).exp2()
}

/// One bar of a voice: its notes and how many rows they fill.
fn bar(text: &str) -> Option<(Vec<(u8, u32)>, u32)> {
    let mut notes = Vec::new();
    let mut rows = 0;
    for token in text.split_whitespace() {
        let (name, len) = token.split_once(':')?;
        let len: u32 = len.parse().ok()?;
        notes.push((midi(name)?, len));
        rows += len;
    }
    Some((notes, rows))
}

/// The lead's notes, from [`LEAD`].
///
/// # Panics
/// On a bar that does not read, which `every_bar_is_a_whole_bar` stops at
/// the door.
fn lead() -> Vec<Note> {
    let mut out = Vec::new();
    for (index, text) in LEAD.iter().enumerate() {
        let (notes, _) = bar(text).expect("every bar reads");
        let mut row = index as u32 * ROWS_PER_BAR;
        for (midi, len) in notes {
            out.push(Note {
                start: row * ROW,
                len: len * ROW,
                hz: hz(midi),
            });
            row += len;
        }
    }
    out
}

/// The harp: each bar's chord broken into semiquavers, up and back down
/// within each beat, every tone ringing on under the next. On the chords
/// with a seventh it turns on the seventh, and in the middle the figure
/// climbs an octave higher, so the accompaniment opens up where the tune
/// does.
fn harp() -> Vec<Note> {
    let mut out = Vec::new();
    for (index, chord) in CHORDS.iter().enumerate() {
        let third = chord.third();
        let top = if chord.seventh() { 11 } else { 12 };
        let figure = if (20..28).contains(&index) {
            [0, 7, 12, 12 + third, 12, 7]
        } else {
            [0, third, 7, top, 7, third]
        };
        for beat in 0..2 {
            for (step, up) in (0..).zip(figure) {
                let row = index as u32 * ROWS_PER_BAR + beat * 6 + step;
                out.push(Note {
                    start: row * ROW,
                    len: ROW,
                    hz: hz(chord.root() + up),
                });
            }
        }
    }
    out
}

/// The bass: the root held for a beat in the opening and in the last two
/// bars, where it marks the turn, and a rocking figure under everything
/// else: root, root, fifth, octave.
fn bass() -> Vec<Note> {
    let mut out = Vec::new();
    for (index, chord) in CHORDS.iter().enumerate() {
        let top = index as u32 * ROWS_PER_BAR;
        let root = chord.bass();
        let figure: &[(u32, u8, u32)] = if (4..BARS - 2).contains(&index) {
            &[(0, 0, 4), (4, 0, 2), (6, 7, 4), (10, 12, 2)]
        } else {
            &[(0, 0, 6), (6, 0, 6)]
        };
        for &(row, up, len) in figure {
            out.push(Note {
                start: (top + row) * ROW,
                len: len * ROW,
                hz: hz(root + up),
            });
        }
    }
    out
}

/// How a voice rises and falls: a linear attack, an exponential fall to a
/// sustained level, and an exponential release once the note is let go.
#[derive(Clone, Copy)]
struct Envelope {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

impl Envelope {
    /// The level `age` seconds into a note held for `held`.
    fn level(self, age: f32, held: f32) -> f32 {
        let on = |t: f32| {
            if t < self.attack {
                t / self.attack
            } else {
                self.sustain + (1.0 - self.sustain) * (-(t - self.attack) / self.decay).exp()
            }
        };
        if age < held {
            on(age)
        } else {
            on(held) * (-(age - held) / self.release).exp()
        }
    }
}

/// `PolyBLEP`: the step of a square wave with its aliasing taken out, which
/// a naive square at 44.1 kHz would fold back as a hiss of wrong notes.
fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

fn pulse(phase: f32, duty: f32, dt: f32) -> f32 {
    let naive = if phase < duty { 1.0 } else { -1.0 };
    naive + blep(phase, dt) - blep((phase - duty).rem_euclid(1.0), dt)
}

fn triangle(phase: f32) -> f32 {
    4.0 * (phase - 0.5).abs() - 1.0
}

/// How many notes of one voice sound at once: the one last struck and those
/// before it, ringing out. By the time a fourth note takes its place, the
/// oldest has fallen below a fiftieth of its level on every voice here.
const SLOTS: usize = 4;

/// A note sounding: the note, how long since it was struck, and its level.
type Sound = (Note, f32, f32);

/// One voice playing its notes. A note struck while the last still sounds
/// does not cut it off: the last rings out under it on its own release, as
/// a string does, so a voice never clicks from one note to the next.
struct Voice {
    notes: Vec<Note>,
    envelope: Envelope,
    /// The notes sounding, newest first, each with where its oscillator is
    /// in its cycle.
    sounding: [Option<(usize, f32)>; SLOTS],
    /// The next note to strike.
    next: usize,
}

impl Voice {
    fn new(notes: Vec<Note>, envelope: Envelope) -> Self {
        Self {
            notes,
            envelope,
            sounding: [None; SLOTS],
            next: 0,
        }
    }

    /// Moves to `pos`, one sample on from the last call, and says which
    /// notes sound there, slot by slot.
    fn at(&mut self, pos: u32, wrapped: bool) -> [Option<Sound>; SLOTS] {
        if wrapped {
            self.next = self.notes.partition_point(|note| note.start < RESTART);
        }
        while let Some(note) = self.notes.get(self.next)
            && note.start <= pos
        {
            // Struck from the top of its cycle, as a tracker retriggers a
            // note, so every time round sounds the same.
            self.sounding.rotate_right(1);
            self.sounding[0] = Some((self.next, 0.0));
            self.next += 1;
        }
        self.sounding.map(|slot| {
            let note = self.notes[slot?.0];
            // A note from the end of the loop still ringing into its restart.
            let age = if pos >= note.start {
                pos - note.start
            } else {
                pos + REPEATS - note.start
            };
            let age = age as f32 / RATE as f32;
            let held = note.len as f32 / RATE as f32;
            let level = self.envelope.level(age, held);
            (level > 1e-4).then_some((note, age, level))
        })
    }

    /// Advances the oscillator in `slot` by one sample at `hz`, and says
    /// where in its cycle it was and how far a sample moves it.
    fn step(&mut self, slot: usize, hz: f32) -> (f32, f32) {
        let dt = hz / RATE as f32;
        let Some((_, phase)) = self.sounding[slot].as_mut() else {
            return (0.0, dt);
        };
        let was = *phase;
        *phase = (*phase + dt).fract();
        (was, dt)
    }
}

/// The lead's vibrato: this many semitones either way, at this rate, coming
/// in over the first half second of a held note, as a singer's would.
const VIBRATO: f32 = 0.18;
const VIBRATO_HZ: f32 = 5.5;

/// The echo: the lead again a beat later, fainter each time.
const ECHO: usize = BEAT as usize;
const ECHO_GAIN: f32 = 0.42;
const ECHO_FEEDBACK: f32 = 0.33;

/// How loud each voice is before [`MASTER`].
const LEAD_GAIN: f32 = 0.30;
const HARP_GAIN: f32 = 0.16;
const BASS_GAIN: f32 = 0.34;
const DRUM_GAIN: f32 = 0.26;

/// The whole mix, chosen so the loudest bar peaks near 0.8 and nothing
/// clips (`the_tune_is_loud_enough_and_never_clips`).
const MASTER: f32 = 0.65;

/// Where the mix is rounded off: the square's edges are there, and its
/// fizz above is not, which is most of what makes a chip voice tiring.
const WARMTH: f32 = 4_500.0;

/// The harp's pulse width's slow sweep: once round every two bars, which
/// both the restart and the end are whole numbers of, so the loop ends where
/// it began.
const SWEEP: u32 = 2 * ROWS_PER_BAR * ROW;

/// A stroke on the frame drum: how deep it sounds, how long it rings and
/// how hard it is struck.
#[derive(Clone, Copy)]
struct Stroke {
    pitch: f32,
    ring: f32,
    weight: f32,
}

/// The stroke on beat `beat` of the loop, if the drum plays one: the deep
/// stroke, in the middle of the skin, on each bar's first beat, and in the
/// middle section a lighter one nearer the rim on its second.
fn stroke(beat: u32) -> Option<Stroke> {
    let first = beat.is_multiple_of(2);
    match (drum_in((beat / 2) as usize), first) {
        (Drum::None, _) | (Drum::Deep, false) => None,
        (_, true) => Some(Stroke {
            pitch: 78.0,
            ring: 0.28,
            weight: 1.0,
        }),
        (Drum::Whole, false) => Some(Stroke {
            pitch: 124.0,
            ring: 0.16,
            weight: 0.5,
        }),
    }
}

/// How hard a hand strikes on beat `beat`: never twice quite the same, and
/// the same every time round the loop, since it is read off the beat.
fn touch(beat: u32) -> f32 {
    let hash = beat.wrapping_mul(0x9E37_79B1) >> 24;
    0.86 + 0.14 * hash as f32 / 255.0
}

/// A drum's skin `t` seconds after it was struck. The fundamental falls a
/// little as the skin settles from the blow, and over it ring the
/// membrane's next two modes, which are what a sine lacks to sound like a
/// drum: a round skin's overtones stand at 1.59 and 2.14 times its
/// fundamental, not at whole multiples, and die faster.
fn skin(t: f32, pitch: f32, ring: f32) -> f32 {
    use std::f32::consts::TAU;
    let fall = 0.2 * pitch;
    let phase = pitch * t + fall * 0.03 * (1.0 - (-t / 0.03).exp());
    let attack = (t / 0.003).min(1.0);
    attack
        * ((TAU * phase).sin() * (-t / ring).exp()
            + 0.4 * (TAU * 1.593 * phase).sin() * (-t / (0.5 * ring)).exp()
            + 0.2 * (TAU * 2.136 * phase).sin() * (-t / (0.3 * ring)).exp())
}

/// The tune, as an endless stream of interleaved stereo samples.
pub struct Tune {
    pos: u32,
    /// Whether `pos` has just gone back to [`RESTART`].
    wrapped: bool,
    lead: Voice,
    harp: Voice,
    bass: Voice,
    echo: Vec<f32>,
    echo_at: usize,
    noise: u32,
    /// The noise with its highs taken off: the hand's thump on the skin.
    hand: f32,
    /// A gentle low-pass on each side, at [`WARMTH`].
    smooth: [f32; 2],
    /// The right-hand sample of the frame whose left went out last.
    right: Option<f32>,
}

impl Default for Tune {
    fn default() -> Self {
        Self::new()
    }
}

impl Tune {
    /// The tune from its first bar.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pos: 0,
            wrapped: false,
            lead: Voice::new(
                lead(),
                Envelope {
                    attack: 0.006,
                    decay: 0.35,
                    sustain: 0.62,
                    release: 0.09,
                },
            ),
            harp: Voice::new(
                harp(),
                Envelope {
                    attack: 0.004,
                    decay: 0.3,
                    sustain: 0.25,
                    release: 0.12,
                },
            ),
            bass: Voice::new(
                bass(),
                Envelope {
                    attack: 0.004,
                    decay: 0.18,
                    sustain: 0.7,
                    release: 0.04,
                },
            ),
            echo: vec![0.0; ECHO],
            echo_at: 0,
            noise: Self::SEED,
            hand: 0.0,
            smooth: [0.0; 2],
            right: None,
        }
    }

    /// Where the drum's noise starts, at the top of every loop.
    const SEED: u32 = 0x9E37_79B9;

    fn noise(&mut self) -> f32 {
        let mut x = self.noise;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.noise = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn lead(&mut self) -> f32 {
        let mut out = 0.0;
        for (slot, sound) in self.lead.at(self.pos, self.wrapped).into_iter().enumerate() {
            let Some((note, age, level)) = sound else {
                continue;
            };
            let depth = ((age - 0.25) / 0.35).clamp(0.0, 1.0) * VIBRATO;
            let bend = depth * (std::f32::consts::TAU * VIBRATO_HZ * age).sin();
            let (phase, dt) = self.lead.step(slot, note.hz * (bend / 12.0).exp2());
            out += pulse(phase, 0.5, dt) * level;
        }
        out
    }

    /// The harp: a triangle for its body and a narrow pulse, its width
    /// slowly sweeping, for the pluck.
    fn harp(&mut self) -> f32 {
        let turn = (self.pos % SWEEP) as f32 / SWEEP as f32;
        let duty = 0.3 + 0.14 * (std::f32::consts::TAU * turn).sin();
        let mut out = 0.0;
        for (slot, sound) in self.harp.at(self.pos, self.wrapped).into_iter().enumerate() {
            let Some((note, _, level)) = sound else {
                continue;
            };
            let (phase, dt) = self.harp.step(slot, note.hz);
            out += (0.6 * triangle(phase) + 0.4 * pulse(phase, duty, dt)) * level;
        }
        out
    }

    fn bass(&mut self) -> f32 {
        let mut out = 0.0;
        for (slot, sound) in self.bass.at(self.pos, self.wrapped).into_iter().enumerate() {
            let Some((note, _, level)) = sound else {
                continue;
            };
            let (phase, _) = self.bass.step(slot, note.hz);
            out += triangle(phase) * level;
        }
        out
    }

    /// The frame drum: its strokes, each with the hand's thump, the last
    /// beat's still ringing under this one's, and in the middle the fingers
    /// on the rim on the last quaver of each beat.
    fn drum(&mut self) -> f32 {
        if self.pos == RESTART {
            self.noise = Self::SEED;
            self.hand = 0.0;
        }
        let noise = self.noise();
        let a = 1.0 - (-std::f32::consts::TAU * 900.0 / RATE as f32).exp();
        self.hand += a * (noise - self.hand);
        let beat = self.pos / BEAT;
        let into = self.pos % BEAT;
        let t = into as f32 / RATE as f32;
        let mut out = 0.0;
        for back in 0..2 {
            let Some(struck) = beat.checked_sub(back) else {
                continue;
            };
            let Some(hit) = stroke(struck) else {
                continue;
            };
            let since = t + (back * BEAT) as f32 / RATE as f32;
            let weight = hit.weight * touch(struck);
            out += weight * skin(since, hit.pitch, hit.ring);
            if back == 0 {
                out += weight * 0.5 * self.hand * (-since / 0.012).exp();
            }
        }
        if drum_in((beat / 2) as usize) == Drum::Whole && into >= 4 * ROW {
            let since = (into - 4 * ROW) as f32 / RATE as f32;
            let tap = skin(since, 330.0, 0.03) + 1.2 * self.hand * (-since / 0.006).exp();
            out += 0.22 * touch(beat + 7) * tap;
        }
        out
    }

    /// The next frame, left and right.
    pub fn frame(&mut self) -> [f32; 2] {
        let lead = self.lead() * LEAD_GAIN;
        let harp = self.harp() * HARP_GAIN;
        let bass = self.bass() * BASS_GAIN;
        let drum = self.drum() * DRUM_GAIN;
        let echo = self.echo[self.echo_at];
        self.echo[self.echo_at] = lead + echo * ECHO_FEEDBACK;
        self.echo_at = (self.echo_at + 1) % ECHO;
        let echo = echo * ECHO_GAIN;
        // The lead a little left and its echo right, the harp a little
        // right: the old trick of one channel's echo on the other.
        let left = lead * 0.85 + echo * 0.35 + harp * 0.7 + bass + drum;
        let right = lead * 0.55 + echo * 0.9 + harp + bass + drum;
        let a = 1.0 - (-std::f32::consts::TAU * WARMTH / RATE as f32).exp();
        self.smooth[0] += a * (left - self.smooth[0]);
        self.smooth[1] += a * (right - self.smooth[1]);
        self.pos += 1;
        self.wrapped = self.pos == LOOP;
        if self.wrapped {
            self.pos = RESTART;
        }
        [self.smooth[0] * MASTER, self.smooth[1] * MASTER]
    }
}

impl Iterator for Tune {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        let [left, right] = self.frame();
        self.right = Some(right);
        Some(left)
    }
}

/// How loud the front door's music is, as this device remembers it.
///
/// Kept with the device's own settings and not the account's, because it
/// plays before anybody has signed in. Its fields are private so the volume
/// is always a number from 0 to 1: a NaN would be written to the settings
/// file as `null`, and a `null` there refuses the whole file, gateways and
/// guest sessions with it.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MusicLevel {
    volume: f32,
    muted: bool,
}

impl Default for MusicLevel {
    /// Half way, and playing: a player who never opens the settings hears it,
    /// under the room rather than over it.
    fn default() -> Self {
        Self {
            volume: 0.5,
            muted: false,
        }
    }
}

impl MusicLevel {
    /// The volume the slider shows, 0 to 1.
    #[must_use]
    pub fn volume(self) -> f32 {
        if self.volume.is_finite() {
            self.volume.clamp(0.0, 1.0)
        } else {
            Self::default().volume
        }
    }

    /// Sets the volume, kept from 0 to 1; anything that is not a number is
    /// ignored.
    pub fn set_volume(&mut self, volume: f32) {
        if volume.is_finite() {
            self.volume = volume.clamp(0.0, 1.0);
        }
    }

    /// Whether the player silenced it.
    #[must_use]
    pub const fn muted(self) -> bool {
        self.muted
    }

    /// Silences it, or lets it play at the volume it had.
    pub const fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// What the music's amplitude is multiplied by: nothing when muted, and
    /// [`Self::loudness`] otherwise.
    #[must_use]
    pub fn gain(self) -> f32 {
        if self.muted { 0.0 } else { self.loudness() }
    }

    /// Silences the music if it can be heard, and lets it be heard if not:
    /// what its switch does (#296). A switch pressed to play music whose
    /// volume is 0 would do nothing a player could hear, so that also brings
    /// the volume back to where it starts.
    pub fn toggle(&mut self) {
        if self.gain() > 0.0 {
            self.muted = true;
        } else {
            self.muted = false;
            if self.volume() <= 0.0 {
                self.set_volume(Self::default().volume());
            }
        }
    }

    /// The amplitude the volume stands for, muted or not: its square,
    /// because the ear hears amplitude roughly as its logarithm and a linear
    /// slider would do all its work in its first quarter. A player fades
    /// towards silence by this and does not jump there.
    #[must_use]
    pub fn loudness(self) -> f32 {
        self.volume() * self.volume()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every bar of the lead reads and fills exactly its twelve rows.
    #[test]
    fn every_bar_is_a_whole_bar() {
        for (index, text) in LEAD.iter().enumerate() {
            let (_, rows) = bar(text).unwrap_or_else(|| panic!("bar {} does not read", index + 1));
            assert!(
                rows == ROWS_PER_BAR || (text.is_empty() && rows == 0),
                "bar {} fills {rows} rows",
                index + 1
            );
        }
        assert_eq!(midi("A4"), Some(69));
        assert_eq!(midi("C#5"), Some(73));
        assert_eq!(midi("Bb4"), Some(70));
        assert_eq!(midi("H4"), None);
    }

    /// One loop of the tune, as frames.
    fn frames(tune: &mut Tune, n: u32) -> Vec<[f32; 2]> {
        (0..n).map(|_| tune.frame()).collect()
    }

    /// Loud enough to be heard, and never past 0.9 on either side, so it
    /// never clips however the mixer sums it with a cue.
    #[test]
    fn the_tune_is_loud_enough_and_never_clips() {
        let mut tune = Tune::new();
        let all = frames(&mut tune, LOOP);
        let peak = all.iter().flatten().fold(0.0_f32, |peak, s| {
            assert!(s.is_finite());
            peak.max(s.abs())
        });
        assert!((0.4..=0.9).contains(&peak), "peak {peak}");
    }

    /// The loop plays on without a seam: after the last bar comes the
    /// theme again, bar 5, with the last note's release and the echo
    /// ringing into it, and no step there larger than the tune takes
    /// anywhere inside itself. Once the echo has died away, the second time
    /// round is the first again, sample for sample, and the opening four
    /// bars are never heard again.
    #[test]
    fn the_loop_plays_on_without_a_seam() {
        let mut tune = Tune::new();
        let first = frames(&mut tune, LOOP);
        let again = frames(&mut tune, REPEATS);
        let step = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs().max((a[1] - b[1]).abs());
        let inside = first
            .windows(2)
            .map(|w| step(w[0], w[1]))
            .fold(0.0_f32, f32::max);
        let seam = step(first[LOOP as usize - 1], again[0]);
        assert!(
            seam <= inside,
            "the seam steps {seam}, the tune at most {inside}"
        );
        let theme = &first[RESTART as usize..];
        // What rings over is heard: bar 5 is not what it was the first time.
        assert!(step(theme[100], again[100]) > 1e-6);
        // And ten seconds in, the echo of the first time round is gone.
        let settled = (RATE * 10) as usize;
        let drift = theme[settled..]
            .iter()
            .zip(&again[settled..])
            .map(|(a, b)| step(*a, *b))
            .fold(0.0_f32, f32::max);
        assert!(drift < 1e-3, "the second time differs by {drift}");
    }

    /// The drum is a skin and not a hiss (#296: the noise drum sounded
    /// made). Through the middle, where it plays most, the weight of its
    /// sound lies low, where a frame drum's does.
    #[test]
    fn the_drum_is_a_skin_and_not_a_hiss() {
        let mut tune = Tune::new();
        let (from, to) = (20 * ROWS_PER_BAR * ROW, 31 * ROWS_PER_BAR * ROW);
        let (mut last, mut power, mut slope) = (0.0_f32, 0.0_f64, 0.0_f64);
        for pos in 0..to {
            tune.pos = pos;
            let sample = tune.drum();
            if pos >= from {
                power += f64::from(sample * sample);
                slope += f64::from((sample - last) * (sample - last));
            }
            last = sample;
        }
        // A sine at f moves 2·sin(πf/RATE) of its size a sample: read back
        // as a frequency, the ratio says where the sound's weight lies.
        let centre = ((slope / power).sqrt() / 2.0).asin() * f64::from(RATE) / std::f64::consts::PI;
        assert!(centre < 400.0, "the drum's weight lies at {centre:.0} Hz");
    }

    /// The level: half way and playing by default, silent when muted
    /// whatever the volume, and a volume that is no number never taken.
    #[test]
    fn the_music_level_is_a_volume_and_a_mute() {
        let mut level = MusicLevel::default();
        assert!(level.gain() > 0.0);
        level.set_muted(true);
        assert!(level.gain().abs() < f32::EPSILON);
        level.set_muted(false);
        level.set_volume(f32::NAN);
        assert!((level.volume() - 0.5).abs() < f32::EPSILON);
        level.set_volume(3.0);
        assert!((level.gain() - 1.0).abs() < f32::EPSILON);
        level.set_volume(0.0);
        assert!(level.gain().abs() < f32::EPSILON);
        // A settings file from before the music opens with it playing.
        let old: MusicLevel = serde_json::from_str("{}").expect("reads");
        assert_eq!(old, MusicLevel::default());
        let kept: MusicLevel =
            serde_json::from_str(&serde_json::to_string(&level).expect("writes")).expect("reads");
        assert_eq!(kept, level);
    }

    /// The switch flips what is heard: a playing tune goes silent and keeps
    /// its volume, a silent one plays again, and one silenced by a volume of
    /// 0 comes back at the starting volume rather than staying silent.
    #[test]
    fn the_switch_flips_what_is_heard() {
        let mut level = MusicLevel::default();
        level.set_volume(0.8);
        level.toggle();
        assert!(level.muted() && level.gain().abs() < f32::EPSILON);
        level.toggle();
        assert!(!level.muted() && (level.volume() - 0.8).abs() < f32::EPSILON);
        level.set_volume(0.0);
        level.toggle();
        assert!(level.gain() > 0.0);
        assert!((level.volume() - MusicLevel::default().volume()).abs() < f32::EPSILON);
    }

    /// Writes two loops to the file `BAYLEE_MUSIC_WAV` names, for a person
    /// to listen to, and says how fast the tune is made.
    #[test]
    #[ignore = "writes a recording"]
    fn record() {
        let Ok(path) = std::env::var("BAYLEE_MUSIC_WAV") else {
            return;
        };
        let started = std::time::Instant::now();
        let mut tune = Tune::new();
        let frames = LOOP + REPEATS;
        let samples: Vec<f32> = (&mut tune).take(frames as usize * 2).collect();
        let took = started.elapsed();
        let seconds = f64::from(frames) / f64::from(RATE);
        eprintln!(
            "{seconds:.1} s of music made in {took:?}: {:.0}× faster than it plays",
            seconds / took.as_secs_f64()
        );
        let mut out = Vec::with_capacity(44 + samples.len() * 2);
        let data = u32::try_from(samples.len() * 2).expect("fits");
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16_u32.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&CHANNELS.to_le_bytes());
        out.extend_from_slice(&RATE.to_le_bytes());
        out.extend_from_slice(&(RATE * 4).to_le_bytes());
        out.extend_from_slice(&4_u16.to_le_bytes());
        out.extend_from_slice(&16_u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data.to_le_bytes());
        for s in samples {
            let v = (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
            out.extend_from_slice(&v.to_le_bytes());
        }
        std::fs::write(path, out).expect("writes the recording");
    }
}
