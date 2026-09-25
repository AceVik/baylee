//! The front door's music (#296): a tune of our own, played by a tracker's
//! voices and synthesised here, one sample at a time.
//!
//! # Whose it is
//!
//! Every note below was written for this client, and every sound is
//! arithmetic: a square lead with its echo, a pulse voice running chords as
//! fast arpeggios, a triangle bass and a drum made of noise and a falling
//! sine. There is no sample, no module file from the demo scene and no
//! recording in it, and it quotes no tune: the style is the cracktros' and
//! the trainers', the notes are not (`docs/legal.md`).
//!
//! # What it is
//!
//! Thirty-two bars in 6/8, in D minor with the Dorian sixth for a colour of
//! magic (the G major of bar 10), at a dotted crotchet of 76: a lilt, which
//! is what keeps it an adventure and out of techno, where the beat would be
//! four even kicks. Four bars of chords and bass, a theme twice over, a
//! brighter middle in F with the drum under it, and a turn on A that leads
//! back to the first bar.
//!
//! # Why it streams
//!
//! [`Tune`] is an endless iterator of stereo samples, not a buffer. The
//! audio thread pulls it, so none of it is computed on a frame, and nothing
//! is held: fifty seconds of stereo would be nine megabytes of `f32` for a
//! screen a player leaves in half a minute. And a loop that is *played*,
//! rather than repeated, has no seam to hide: the echo and the last note's
//! release ring on into the first bar, as they would if a band played it
//! twice.

/// Samples per second, per channel.
pub const RATE: u32 = 44_100;

/// Two, interleaved left then right.
pub const CHANNELS: u16 = 2;

/// One tracker row, a semiquaver: a dotted crotchet of 76.04.
///
/// A whole number of samples, so the loop is too, and the wrap lands on a
/// sample rather than between two.
const ROW: u32 = 5_800;

/// A bar of 6/8 in semiquavers.
const ROWS_PER_BAR: u32 = 12;

/// Bars in the loop.
const BARS: usize = 32;

/// The whole tune, in samples per channel: 50.5 seconds.
pub const LOOP: u32 = ROW * ROWS_PER_BAR * BARS as u32;

/// Where the loop goes back to: the theme, bar 5. The four bars of chords
/// before it open the tune once, as a tracker's restart position leaves a
/// song's introduction behind; the last bar is a pickup into the theme.
pub const RESTART: u32 = ROW * ROWS_PER_BAR * 4;

/// The part that repeats, bars 5 to 32: 44.2 seconds.
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
    /// The root, as a MIDI note in the arpeggio's octave (G3 to F4).
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

    /// Whether the arpeggio adds the major seventh: on the two chords that
    /// take it without a fight with the lead (B flat and F), where it is
    /// the shimmer the style calls magic.
    const fn seventh(self) -> bool {
        matches!(self, Self::Bb | Self::F)
    }

    /// The bass's root, in F2 to E3: the arpeggio's root an octave or two
    /// down.
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
    // Again, higher, with the drum's kick.
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

/// Which bars the drum plays in: none in the opening, the kick alone under
/// the theme's second time, the whole kit in the middle, and a breath in
/// the last bar so the turn is heard.
fn drum_in(bar: usize) -> Drum {
    match bar {
        12..=19 => Drum::Kick,
        20..=30 => Drum::Kit,
        _ => Drum::None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Drum {
    None,
    Kick,
    Kit,
}

/// A note of a voice, in samples from the top of the loop.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Note {
    start: u32,
    len: u32,
    hz: f32,
    /// For the arpeggio: the chord it runs, as semitones above `hz`, and
    /// how many of them it runs.
    chord: [u8; 5],
    tones: u8,
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
                chord: [0; 5],
                tones: 1,
            });
            row += len;
        }
    }
    out
}

/// The arpeggio: each bar's chord, struck twice, once on each beat. In
/// the middle it takes the octave on top as well, so the accompaniment
/// changes where the tune does.
fn arpeggio() -> Vec<Note> {
    let mut out = Vec::new();
    for (index, chord) in CHORDS.iter().enumerate() {
        let mut tones = vec![0, chord.third(), 7];
        if chord.seventh() {
            tones.push(11);
        }
        if (20..28).contains(&index) {
            tones.push(12);
        }
        let mut notes = [0; 5];
        notes[..tones.len()].copy_from_slice(&tones);
        for beat in 0..2 {
            let row = index as u32 * ROWS_PER_BAR + beat * 6;
            out.push(Note {
                start: row * ROW,
                len: 6 * ROW,
                hz: hz(chord.root()),
                chord: notes,
                tones: tones.len() as u8,
            });
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
                chord: [0; 5],
                tones: 1,
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

/// One voice playing its notes: which is sounding, and its oscillator.
struct Voice {
    notes: Vec<Note>,
    envelope: Envelope,
    /// The note sounding or ringing out, if any has been struck yet.
    current: Option<usize>,
    /// The next note to strike.
    next: usize,
    phase: f32,
}

impl Voice {
    fn new(notes: Vec<Note>, envelope: Envelope) -> Self {
        Self {
            notes,
            envelope,
            current: None,
            next: 0,
            phase: 0.0,
        }
    }

    /// Moves to `pos`, one sample on from the last call, and says which
    /// note sounds there, how long since it was struck, and at what level.
    fn at(&mut self, pos: u32, wrapped: bool) -> Option<(Note, f32, f32)> {
        if wrapped {
            self.next = self.notes.partition_point(|note| note.start < RESTART);
        }
        while let Some(note) = self.notes.get(self.next)
            && note.start <= pos
        {
            // Struck from the top of its cycle, as a tracker retriggers a
            // note, so every time round sounds the same.
            self.current = Some(self.next);
            self.next += 1;
            self.phase = 0.0;
        }
        let note = self.notes[self.current?];
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
    }

    /// Advances the oscillator by one sample at `hz`, and says where in its
    /// cycle it was and how far a sample moves it.
    fn step(&mut self, hz: f32) -> (f32, f32) {
        let dt = hz / RATE as f32;
        let phase = self.phase;
        self.phase = (self.phase + dt).fract();
        (phase, dt)
    }
}

/// The lead's vibrato: this many semitones either way, at this rate, coming
/// in over the first half second of a held note, as a singer's would.
const VIBRATO: f32 = 0.18;
const VIBRATO_HZ: f32 = 5.5;

/// The arpeggio's speed: a new note of the chord every fiftieth of a
/// second, a PAL frame, which is where the sound comes from.
const ARP_STEP: u32 = RATE / 50;

/// The echo: the lead again a beat later, fainter each time.
const ECHO: usize = (6 * ROW) as usize;
const ECHO_GAIN: f32 = 0.42;
const ECHO_FEEDBACK: f32 = 0.33;

/// How loud each voice is before [`MASTER`].
const LEAD_GAIN: f32 = 0.30;
const ARP_GAIN: f32 = 0.12;
const BASS_GAIN: f32 = 0.34;
const DRUM_GAIN: f32 = 0.30;

/// The whole mix, chosen so the loudest bar peaks near 0.8 and nothing
/// clips (`the_tune_is_loud_enough_and_never_clips`).
const MASTER: f32 = 0.8;

/// The pulse width's slow sweep: once round every two bars, which both the
/// restart and the end are whole numbers of, so the loop ends where it
/// began.
const SWEEP: u32 = 2 * ROWS_PER_BAR * ROW;

/// The tune, as an endless stream of interleaved stereo samples.
pub struct Tune {
    pos: u32,
    /// Whether `pos` has just gone back to [`RESTART`].
    wrapped: bool,
    lead: Voice,
    arpeggio: Voice,
    bass: Voice,
    echo: Vec<f32>,
    echo_at: usize,
    noise: u32,
    /// The last hi-hat noise, which the hat is the difference from.
    hat_last: f32,
    /// A gentle low-pass on each side, which takes the edge off the square.
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
            arpeggio: Voice::new(
                arpeggio(),
                Envelope {
                    attack: 0.004,
                    decay: 0.5,
                    sustain: 0.45,
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
            hat_last: 0.0,
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
        let Some((note, age, level)) = self.lead.at(self.pos, self.wrapped) else {
            self.lead.step(0.0);
            return 0.0;
        };
        let depth = ((age - 0.25) / 0.35).clamp(0.0, 1.0) * VIBRATO;
        let bend = depth * (std::f32::consts::TAU * VIBRATO_HZ * age).sin();
        let (phase, dt) = self.lead.step(note.hz * (bend / 12.0).exp2());
        pulse(phase, 0.5, dt) * level
    }

    fn arpeggio(&mut self) -> f32 {
        let Some((note, age, level)) = self.arpeggio.at(self.pos, self.wrapped) else {
            self.arpeggio.step(0.0);
            return 0.0;
        };
        let step = (age * RATE as f32) as u32 / ARP_STEP;
        let up = note.chord[(step % u32::from(note.tones)) as usize];
        let (phase, dt) = self.arpeggio.step(note.hz * (f32::from(up) / 12.0).exp2());
        let turn = (self.pos % SWEEP) as f32 / SWEEP as f32;
        let duty = 0.3 + 0.14 * (std::f32::consts::TAU * turn).sin();
        pulse(phase, duty, dt) * level
    }

    fn bass(&mut self) -> f32 {
        let Some((note, _, level)) = self.bass.at(self.pos, self.wrapped) else {
            self.bass.step(0.0);
            return 0.0;
        };
        let (phase, _) = self.bass.step(note.hz);
        triangle(phase) * level
    }

    /// The drum: a kick on the first beat, and in the middle a snare on the
    /// second and a hat on the last quaver of each.
    fn drum(&mut self) -> f32 {
        if self.pos == RESTART {
            self.noise = Self::SEED;
        }
        let bar = (self.pos / (ROW * ROWS_PER_BAR)) as usize;
        let row = self.pos / ROW % ROWS_PER_BAR;
        let t = (self.pos % ROW) as f32 / RATE as f32;
        let kit = drum_in(bar);
        let noise = self.noise();
        let mut out = 0.0;
        if kit != Drum::None && row == 0 {
            // A sine falling from 120 Hz to 45, its phase the integral of
            // that fall.
            let phase = 45.0 * t + 75.0 * 0.03 * (1.0 - (-t / 0.03).exp());
            out += (std::f32::consts::TAU * phase).sin() * (-t / 0.09).exp() * (t / 0.002).min(1.0);
        }
        if kit == Drum::Kit {
            if row == 6 {
                let body = (std::f32::consts::TAU * 185.0 * t).sin() * (-t / 0.03).exp();
                out += (noise * 0.55 * (-t / 0.05).exp() + body * 0.35) * (t / 0.001).min(1.0);
            }
            if row == 4 || row == 10 {
                let hat = noise - self.hat_last;
                out += hat * 0.18 * (-t / 0.012).exp();
            }
        }
        self.hat_last = noise;
        out
    }

    /// The next frame, left and right.
    pub fn frame(&mut self) -> [f32; 2] {
        let lead = self.lead() * LEAD_GAIN;
        let arpeggio = self.arpeggio() * ARP_GAIN;
        let bass = self.bass() * BASS_GAIN;
        let drum = self.drum() * DRUM_GAIN;
        let echo = self.echo[self.echo_at];
        self.echo[self.echo_at] = lead + echo * ECHO_FEEDBACK;
        self.echo_at = (self.echo_at + 1) % ECHO;
        let echo = echo * ECHO_GAIN;
        // The lead a little left and its echo right, the chords a little
        // right: the old trick of one channel's echo on the other.
        let left = lead * 0.85 + echo * 0.35 + arpeggio * 0.7 + bass + drum;
        let right = lead * 0.55 + echo * 0.9 + arpeggio + bass + drum;
        let a = 1.0 - (-std::f32::consts::TAU * 7_000.0 / RATE as f32).exp();
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
