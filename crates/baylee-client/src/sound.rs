//! The sink: where a decided cue becomes a noise.
//!
//! [`baylee_client_core::cue`] is the whole of the thinking — which moments
//! are worth hearing, and the arithmetic that makes a triple block one sound
//! instead of three. This is the other half: fourteen tones, **computed** at
//! startup and played through `bevy_audio`.
//!
//! # Nothing here is a file
//!
//! `docs/legal.md` §5 is the rule, and it is clause 2's reasoning applied to
//! the ear: ornament is the easiest thing to borrow by accident, and
//! arithmetic borrows nothing. The felt, the seat mats and the lobby's
//! backdrop were all given that answer and so is this — [`render`] writes
//! samples and [`wav`] writes a RIFF header in front of them, so the client
//! ships no audio assets and there is no `sounds/` directory for anyone to
//! audit. A player's own sound pack stays reachable because a [`Cue`] is a
//! named moment and not a file name.
//!
//! # One instrument, struck nine ways
//!
//! All of them are the same thing: a **struck rosewood bar**. A fundamental
//! and two inharmonic partials at 3.93 and 9.56, each dying at its own rate,
//! under a short filtered noise burst — the mallet before the note. That is
//! what keeps a set of tones from being a set of *beeps*: a sine has no body,
//! a sawtooth is a synthesiser, and wood is what belongs on a table with a
//! leather rail round it. Metal was the other candidate and is wrong for the
//! same reason it is wrong in a room: a bell's upper partials outlive its
//! fundamental, so two of them overlapping is a chord nobody asked for.
//!
//! What separates the nine is **pitch, gesture and level**, never timbre:
//!
//! - the four life cues are one two-strike gesture, falling a minor third for
//!   a loss and rising a major third for a gain. Two intervals rather than
//!   one mirrored interval, because mirroring would make a gain sound minor;
//!   together they outline a triad, so a lifelink trade — [`Cue::MyLifeLost`]
//!   and [`Cue::TheirLifeGained`] on one frame — is a chord and not an
//!   argument.
//! - somebody **else's** life is that gesture a fourth lower, at a third of
//!   the level, with a softer mallet, the top partial nearly gone and one
//!   early reflection 19 ms behind it. "Elsewhere" is a filter and a
//!   reflection, not a volume knob: a single delayed, dulled copy is what an
//!   ear reads as *over there*.
//! - the three endings are the only sounds longer than a blink, and what they
//!   add is **ring** — the resonator tube under the bar — never a new timbre
//!   and never more loudness. All three peak below a life cue. Won rises and
//!   slows, lost falls and softens, drawn is two equal strikes; all three come
//!   to rest on A and never on the tonic, because a cadence left open is what
//!   keeps a win from being a fanfare. `docs/design.md` retires hues rather
//!   than handing out new ones, and this is the same restraint.
//!
//! # `YourMove` is the one that could ruin it
//!
//! It fires on every priority grant — two hundred times in a long game — and
//! a fixed tone at that rate is a metronome. Three things answer it here:
//! it is the **lowest** sound in the set at 147 Hz, so it sits under the game
//! rather than over it; it is the **quietest**, peaking at 0.154 against a
//! life cue's 0.700; it is **one touch** rather than a two-note gesture,
//! because a melody is a thing that repeats and a tap is not; and it is one
//! of **six variants** cycled in a fixed order, detuned within ±16 cents with
//! the second partial struck at a different spot on the bar. A real bar
//! drifts that much with the room and never reads as a wrong note, but no two
//! consecutive firings are the same sound.
//!
//! It is *not* the shortest — 900 ms, against a life cue's 650 — and that is
//! deliberate rather than an oversight: a soft low note with a long decay is
//! felt and then forgotten, where a short one is a *click*, and a click two
//! hundred times is the metronome this is trying not to be. Measured: at half
//! a second it is down to 0.018, a fiftieth of a life cue's peak.
//!
//! What is **not** built is the policy half, and it is the larger half: a
//! grant that follows the player's own action tells them nothing, so a
//! debounce of about 600 ms, a suppression window after the seat sends
//! anything, a refractory period, and a louder cue when the window is in the
//! background would together turn two hundred grants into a few dozen touches.
//! That belongs in `baylee-client-core` beside `reconnect.rs`, where it can be
//! tested without a device — and it wants a clock passed in, which
//! [`baylee_client_core::cue::Cues`] has no field for yet. Until it exists the
//! setting is the answer, and `Loudness::Off` is one chip on the settings
//! screen.
//!
//! The other deliberate omission is loudness by **amount**: a twelve-point
//! hit should be louder than a one-point one, and cannot be, because a
//! [`Cue`] is a flat variant with no payload — on purpose, so that every
//! reader gets a name. Giving it one is a change to the model, not to the
//! sink.

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;

use crate::Duel;
use crate::prefs::Prefs;
use baylee_client_core::cue::{Cue, Loudness};

/// Samples per second, everywhere.
const RATE: f32 = 44_100.0;

/// What every finished buffer's peak is multiplied by.
///
/// Three decibels of headroom, and the arithmetic for it is the worst case
/// two cues can make: a life change at this seat and one elsewhere land on
/// the same frame at 1.00 and 0.40 of their own peaks, so the sum stays under
/// one and nothing clips. It is not a volume control — that is [`Loudness`],
/// which multiplies at playback, where a player can reach it.
///
/// *Two* is the worst case rather than three because of [`audible`]: the one
/// frame that could carry a third is the frame a game ends on, and there the
/// ending is played on its own. No number in this module would make those
/// three fit — a mixer would, and this module has none.
const MASTER: f32 = 0.70;

/// One partial of the bar: where it sits, how loud it starts, and how fast it
/// dies relative to the fundamental.
///
/// 3.93 and 9.56 are a rosewood bar's, and the short decays are what makes it
/// wood: the upper partials are gone in a tenth of the time the fundamental
/// takes, which is the whole difference between a struck bar and a struck
/// bell.
struct Partial {
    ratio: f32,
    gain: f32,
    decay: f32,
}

/// The bar, before any cue changes it.
const BAR: [Partial; 3] = [
    Partial {
        ratio: 1.0,
        gain: 1.0,
        decay: 1.0,
    },
    Partial {
        ratio: 3.93,
        gain: 0.32,
        decay: 0.35,
    },
    Partial {
        ratio: 9.56,
        gain: 0.10,
        decay: 0.15,
    },
];

/// What hits the bar.
///
/// The noise burst is the contact and the lowpass is the mallet's head: a
/// hard stick passes 2.2 kHz of it, yarn passes 700 Hz, and a knuckle on the
/// leather rail is mostly burst and almost no note. `attack` is the ramp the
/// note itself opens on — 1.5 ms is an anti-click, and the longer values are
/// a softer stroke rather than a safety margin.
struct Mallet {
    /// The one-pole corner, in hertz.
    cut: f32,
    /// The burst's level against the fundamental.
    gain: f32,
    /// How fast the burst dies, in seconds.
    decay: f32,
    /// The note's own opening ramp, in seconds.
    attack: f32,
}

/// A hard stick: the life cues at this seat.
const HARD: Mallet = Mallet {
    cut: 2200.0,
    gain: 0.18,
    decay: 0.005,
    attack: 0.0015,
};

/// Soft: somebody else's life, heard across the table.
const SOFT: Mallet = Mallet {
    cut: 1200.0,
    gain: 0.10,
    decay: 0.006,
    attack: 0.005,
};

/// Yarn, the softest thing there is, and [`Cue::YourMove`]'s alone.
const YARN: Mallet = Mallet {
    cut: 700.0,
    gain: 0.06,
    decay: 0.006,
    attack: 0.006,
};

/// Not the bar at all — a knuckle on the padded rail.
const KNUCKLE: Mallet = Mallet {
    cut: 600.0,
    gain: 0.55,
    decay: 0.010,
    attack: 0.001,
};

/// The stroke a win is struck with: brighter than [`SOFT`], because it is a
/// larger gesture on a larger bar.
const WON: Mallet = Mallet {
    cut: 1600.0,
    gain: 0.10,
    decay: 0.006,
    attack: 0.003,
};

/// And the one a loss is: duller still.
const LOST: Mallet = Mallet {
    cut: 900.0,
    gain: 0.10,
    decay: 0.006,
    attack: 0.004,
};

/// Just intonation off D4, which is the only scale in the building.
mod note {
    pub const FS3: f32 = 183.54;
    /// Not in the working scale, which is why a refusal is struck on it.
    pub const G3: f32 = 195.77;
    /// Where all three endings come to rest.
    pub const A3: f32 = 220.25;
    pub const B3: f32 = 244.72;
    pub const CS4: f32 = 275.31;
    pub const D4: f32 = 293.66;
    pub const FS4: f32 = 367.08;
    pub const A4: f32 = 440.49;
}

/// One blow: when, on what note, how long it rings and how hard.
struct Strike {
    /// Seconds from the start of the buffer.
    at: f32,
    /// The fundamental, in hertz.
    hz: f32,
    /// The fundamental's decay constant, in seconds. The partials take
    /// fractions of it.
    ring: f32,
    /// Against the loudest strike in the same cue.
    gain: f32,
}

/// One early reflection, which is how a sound is put across the table.
struct Room {
    /// Seconds behind the direct sound.
    delay: f32,
    /// How much of it comes back.
    gain: f32,
    /// The wall, as a one-pole corner in hertz.
    cut: f32,
}

/// The one reflection both `Their…` cues are heard through.
const ELSEWHERE: Room = Room {
    delay: 0.019,
    gain: 0.30,
    cut: 1500.0,
};

/// Everything one buffer needs.
struct Recipe {
    strikes: &'static [Strike],
    mallet: &'static Mallet,
    /// Seconds. Long enough for the last strike to have died.
    len: f32,
    /// What the finished buffer is normalised to, before [`MASTER`].
    peak: f32,
    /// Overrides for the bar's upper partials, where a cue wants a different
    /// body. `None` leaves [`BAR`] alone.
    second: Option<f32>,
    third: Option<f32>,
    /// A reflection, for a sound that is not happening here.
    room: Option<Room>,
}

impl Recipe {
    /// The bar this recipe strikes, with its overrides applied.
    fn bar(&self) -> [f32; 3] {
        [
            BAR[0].gain,
            self.second.unwrap_or(BAR[1].gain),
            self.third.unwrap_or(BAR[2].gain),
        ]
    }
}

/// The eight cues that are one sound each.
///
/// [`Cue::YourMove`] is not here; it is [`VARIANTS`], which is six of them.
const RECIPES: [(Cue, Recipe); 8] = [
    (
        Cue::MyLifeLost,
        Recipe {
            strikes: &[
                Strike {
                    at: 0.0,
                    hz: note::D4,
                    ring: 0.110,
                    gain: 0.80,
                },
                Strike {
                    at: 0.095,
                    hz: note::B3,
                    ring: 0.125,
                    gain: 1.00,
                },
            ],
            mallet: &HARD,
            len: 0.650,
            peak: 1.00,
            second: None,
            third: None,
            room: None,
        },
    ),
    (
        Cue::MyLifeGained,
        Recipe {
            strikes: &[
                Strike {
                    at: 0.0,
                    hz: note::D4,
                    ring: 0.110,
                    gain: 1.00,
                },
                Strike {
                    at: 0.085,
                    hz: note::FS4,
                    ring: 0.095,
                    gain: 0.85,
                },
            ],
            mallet: &HARD,
            len: 0.550,
            peak: 0.80,
            second: None,
            third: None,
            room: None,
        },
    ),
    (
        Cue::TheirLifeLost,
        Recipe {
            strikes: &[
                Strike {
                    at: 0.0,
                    hz: note::A3,
                    ring: 0.140,
                    gain: 0.80,
                },
                Strike {
                    at: 0.095,
                    hz: note::FS3,
                    ring: 0.160,
                    gain: 1.00,
                },
            ],
            mallet: &SOFT,
            len: 0.800,
            peak: 0.40,
            second: None,
            third: Some(0.05),
            room: Some(ELSEWHERE),
        },
    ),
    (
        Cue::TheirLifeGained,
        Recipe {
            strikes: &[
                Strike {
                    at: 0.0,
                    hz: note::A3,
                    ring: 0.140,
                    gain: 1.00,
                },
                Strike {
                    at: 0.085,
                    hz: note::CS4,
                    ring: 0.120,
                    gain: 0.85,
                },
            ],
            mallet: &SOFT,
            len: 0.750,
            peak: 0.32,
            second: None,
            third: Some(0.05),
            room: Some(ELSEWHERE),
        },
    ),
    (
        Cue::Refused,
        Recipe {
            strikes: &[
                Strike {
                    at: 0.0,
                    hz: note::G3,
                    ring: 0.020,
                    gain: 1.00,
                },
                Strike {
                    at: 0.055,
                    hz: note::G3,
                    ring: 0.020,
                    gain: 0.80,
                },
            ],
            mallet: &KNUCKLE,
            len: 0.200,
            peak: 0.50,
            second: Some(0.25),
            third: Some(0.0),
            room: None,
        },
    ),
    (
        Cue::GameWon,
        Recipe {
            strikes: &[
                Strike {
                    at: 0.0,
                    hz: note::D4,
                    ring: 0.280,
                    gain: 0.80,
                },
                Strike {
                    at: 0.240,
                    hz: note::FS4,
                    ring: 0.280,
                    gain: 0.90,
                },
                Strike {
                    at: 0.560,
                    hz: note::A4,
                    ring: 0.450,
                    gain: 1.00,
                },
            ],
            mallet: &WON,
            len: 2.700,
            peak: 0.90,
            second: None,
            third: None,
            room: None,
        },
    ),
    (
        Cue::GameLost,
        Recipe {
            strikes: &[
                Strike {
                    at: 0.0,
                    hz: note::D4,
                    ring: 0.300,
                    gain: 1.00,
                },
                Strike {
                    at: 0.300,
                    hz: note::B3,
                    ring: 0.320,
                    gain: 0.90,
                },
                Strike {
                    at: 0.700,
                    hz: note::A3,
                    ring: 0.600,
                    gain: 0.85,
                },
            ],
            mallet: &LOST,
            len: 3.700,
            peak: 0.80,
            second: None,
            third: None,
            room: None,
        },
    ),
    (
        Cue::GameDrawn,
        Recipe {
            strikes: &[
                Strike {
                    at: 0.0,
                    hz: note::A3,
                    ring: 0.380,
                    gain: 1.00,
                },
                Strike {
                    at: 0.420,
                    hz: note::A3,
                    ring: 0.480,
                    gain: 0.95,
                },
            ],
            mallet: &SOFT,
            len: 2.800,
            peak: 0.72,
            second: None,
            third: None,
            room: None,
        },
    ),
];

/// The six [`Cue::YourMove`] taps, as a fundamental and a second-partial
/// amplitude, cycled in this order.
///
/// Detune within ±16 cents of D3, and a different second partial each time,
/// which is the mallet landing at a different spot along the bar. The order
/// is fixed and deliberately **not** sorted: an ascending run would read as a
/// glide, and a player who heard six rising taps would go looking for what
/// they meant.
const VARIANTS: [(f32, f32); 6] = [
    (146.83, 0.32),
    (147.77, 0.22),
    (146.24, 0.40),
    (148.20, 0.15),
    (145.65, 0.36),
    (147.26, 0.26),
];

/// One tap, built at one of [`VARIANTS`].
///
/// A function rather than six more entries in [`RECIPES`], because the six
/// differ in two numbers and agree about everything else — and a `Strike` is
/// borrowed from a `const`, which cannot be written six times over without
/// six copies of the rest of the recipe.
fn tap(hz: f32, second: f32) -> Recipe {
    Recipe {
        strikes: Vec::leak(vec![Strike {
            at: 0.0,
            hz,
            ring: 0.240,
            gain: 1.00,
        }]),
        mallet: &YARN,
        len: 0.900,
        peak: 0.22,
        second: Some(second),
        third: None,
        room: None,
    }
}

/// A deterministic bit source for the mallet's contact noise.
///
/// xorshift32 with a fixed seed, re-seeded at the start of every buffer, so a
/// cue's samples are a pure function of its recipe and every machine hears
/// the same table. It is the same reason `tabletop`'s fbm hashes a lattice
/// point instead of drawing from an RNG: nothing generated here may depend on
/// a clock.
struct Noise(u32);

impl Noise {
    /// The seed. Any odd constant does; this one is legible in a hex dump.
    const SEED: u32 = 0x5EED_BA11;

    fn new() -> Self {
        Self(Self::SEED)
    }

    /// The next sample, in `-1.0..1.0`.
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        let bits = u16::try_from(self.0 >> 16).unwrap_or(u16::MAX);
        f32::from(bits) / f32::from(u16::MAX) * 2.0 - 1.0
    }
}

/// A one-pole lowpass coefficient for a corner at `cut` hertz.
fn pole(cut: f32) -> f32 {
    (-std::f32::consts::TAU * cut / RATE).exp()
}

/// Renders a recipe to samples in `-1.0..1.0`.
///
/// Three passes, in this order and not another: the strikes, then the room,
/// then the level. A reflection added *after* normalising would push the peak
/// back over what the recipe asked for, and a fade applied before the room
/// would be audible in the reflection.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn render(recipe: &Recipe) -> Vec<f32> {
    let len = (recipe.len * RATE) as usize;
    let mut out = vec![0.0f32; len];
    let mut noise = Noise::new();
    let bar = recipe.bar();
    let mallet = recipe.mallet;
    let contact = pole(mallet.cut);
    // The burst is spent in about six of its own decay constants; running the
    // filter past that is arithmetic on silence.
    let burst = (mallet.decay * 6.0 * RATE) as usize;

    for strike in recipe.strikes {
        let start = (strike.at * RATE) as usize;
        if start >= len {
            continue;
        }
        let mut low = 0.0f32;
        for (i, place) in out[start..].iter_mut().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f32 / RATE;
            let open = (t / mallet.attack).min(1.0);
            let mut sample = 0.0f32;
            for (partial, &gain) in BAR.iter().zip(bar.iter()) {
                if gain == 0.0 {
                    continue;
                }
                let hz = strike.hz * partial.ratio;
                let decay = (-t / (strike.ring * partial.decay)).exp();
                sample += gain * decay * (std::f32::consts::TAU * hz * t).sin();
            }
            if i < burst {
                let white = noise.next();
                low = contact.mul_add(low, (1.0 - contact) * white);
                sample += mallet.gain * low * (-t / mallet.decay).exp();
            }
            *place += strike.gain * open * sample;
        }
    }

    if let Some(room) = &recipe.room {
        let delay = (room.delay * RATE) as usize;
        let wall = pole(room.cut);
        let mut low = 0.0f32;
        let direct = out.clone();
        for (place, &dry) in out[delay..].iter_mut().zip(direct.iter()) {
            low = wall.mul_add(low, (1.0 - wall) * dry);
            *place += room.gain * low;
        }
    }

    level(&mut out, recipe.peak * MASTER);
    out
}

/// Scales a buffer so its loudest sample is `peak`, then fades its tail.
///
/// The fade is six milliseconds of a raised cosine. Without it a buffer ends
/// on whatever the exponential had reached, and a step from that to zero is a
/// click — the one artefact a listener always notices and never attributes to
/// the sound that caused it.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn level(buf: &mut [f32], peak: f32) {
    let loudest = buf.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    if loudest > 0.0 {
        let k = peak / loudest;
        for sample in buf.iter_mut() {
            *sample *= k;
        }
    }
    let fade = ((0.006 * RATE) as usize).min(buf.len());
    let n = buf.len();
    for (i, sample) in buf[n - fade..].iter_mut().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f32 / fade as f32;
        *sample *= f32::midpoint(1.0, (std::f32::consts::PI * t).cos());
    }
}

/// Wraps samples in a 16-bit mono RIFF header.
///
/// Written out by hand rather than taken from a crate because it is
/// forty-four bytes and one cast, and because this is the only audio format
/// the client will ever *produce*: it ships no files, so nothing here has to
/// read one.
fn wav(samples: &[f32]) -> Vec<u8> {
    let data = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&u32::try_from(36 + data).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // the PCM chunk's length
    out.extend_from_slice(&1u16.to_le_bytes()); // uncompressed
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rate = RATE as u32;
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // bytes per frame
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&u32::try_from(data).unwrap_or(u32::MAX).to_le_bytes());
    for &sample in samples {
        #[allow(clippy::cast_possible_truncation)]
        let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// One finished buffer as a playable asset.
///
/// `AudioSource` is a `Arc<[u8]>` of an encoded file and a `rodio` decoder
/// behind it, which is why [`wav`] exists at all: there is no door into
/// `bevy_audio` that takes samples.
fn source(bytes: Vec<u8>) -> AudioSource {
    AudioSource {
        bytes: bytes.into(),
    }
}

/// The synthesised table, one entry per cue.
///
/// A list of pairs rather than an array indexed by the enum, because a
/// `usize` for a `Cue` is a second spelling of the same thing and the two
/// would drift. Nine comparisons of a `Copy` discriminant, once per sound, is
/// not a cost worth a second spelling.
#[derive(Resource, Default)]
pub struct Voices {
    /// Each cue and the one or more buffers it may be played from.
    voices: Vec<(Cue, Vec<Handle<AudioSource>>)>,
    /// How many sounds have been asked for, which is what picks a variant.
    played: usize,
}

impl Voices {
    /// How many cues have a voice. Nine, or the client is silent somewhere.
    #[must_use]
    pub fn len(&self) -> usize {
        self.voices.len()
    }

    /// Whether nothing has been synthesised, which is what a headless test
    /// has.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.voices.is_empty()
    }

    /// The next buffer for a cue, or `None` if it has no voice.
    fn pick(&self, cue: Cue) -> Option<&Handle<AudioSource>> {
        let takes = &self.voices.iter().find(|(which, _)| *which == cue)?.1;
        takes.get(self.played % takes.len())
    }
}

/// Synthesises every cue once, at startup.
///
/// About fifteen seconds of audio in total, computed on the frame the app
/// opens and then never again. A system rather than a `LazyLock` because what
/// comes out is an `Asset`, and the only place to put one of those is a
/// `World`.
///
/// The asset store is `Option` for the same reason [`play_the_cues`]'s
/// resources are: this plugin is embeddable, and an application that
/// installed no `AudioPlugin` has no `Assets<AudioSource>` at all. A `ResMut`
/// there would panic on the app's first frame — in the one case the rest of
/// this module is written to survive.
pub fn voice_the_cues(mut commands: Commands, sources: Option<ResMut<Assets<AudioSource>>>) {
    let Some(mut sources) = sources else {
        return;
    };
    let mut voices = Vec::with_capacity(Cue::ALL.len());
    for (cue, recipe) in &RECIPES {
        let bytes = wav(&render(recipe));
        voices.push((*cue, vec![sources.add(source(bytes))]));
    }
    let taps = VARIANTS
        .iter()
        .map(|&(hz, second)| sources.add(source(wav(&render(&tap(hz, second))))))
        .collect();
    voices.push((Cue::YourMove, taps));
    commands.insert_resource(Voices { voices, played: 0 });
}

/// Hands this frame's cues to the sink and remembers the last of them.
///
/// In `DuelSet::Present` and not in `Sync`, which is what makes
/// [`baylee_client_core::cue::Cues::retract`] work at all: every source of a
/// cue and everything that answers a question have both run by the time this
/// does, so a chime the client withdrew inside the frame never reaches the
/// sink.
///
/// The early return is not a micro-optimisation. `Duel` is a resource and
/// touching it mutably marks it changed, so a drain that ran unconditionally
/// would report the whole duel as having moved on every frame of a game where
/// nothing happened at all.
///
/// [`Voices`] and [`Prefs`] are both `Option`: this plugin is meant to be
/// embeddable, and an application that installed no audio backend — or a
/// headless test — must still decide, drain and report its cues.
pub fn play_the_cues(
    mut commands: Commands,
    mut duel: ResMut<Duel>,
    voices: Option<ResMut<Voices>>,
    prefs: Option<Res<Prefs>>,
) {
    if duel.cues.pending().is_empty() {
        return;
    }
    let level = prefs.map_or_else(Loudness::default, |prefs| prefs.all().sound);
    let mut voices = voices;
    for cue in audible(&duel.cues.take()) {
        if let Some(voices) = voices.as_mut() {
            sound(&mut commands, voices, cue, level);
        }
    }
}

/// Which of one frame's cues are actually played.
///
/// Ordinarily all of them: [`MASTER`] is chosen so that the two loudest life
/// cues fit together, and `docs/client.md` §"One event, one cue" is why a
/// frame cannot carry two of the *same* sound. The exception is the frame a
/// game ends on, which is the one frame that carries three — the lethal hit
/// is a life loss here, a life loss there, and the ending, and 0.70 + 0.28 +
/// 0.63 clips at the loudest moment of the game.
///
/// So an ending is played **alone**. It is the only thing on that frame
/// anybody is listening for, and the life change that caused it is already
/// drawn on two bars and flashed on both. Dropping the quiet cues rather than
/// ducking them is the same choice `Cues` makes everywhere else: this module
/// has no mixer and wants none — a sound either happens or does not.
///
/// The cues are still **drained** by the caller either way, so nothing here
/// changes what `/state` reports or what [`Cue::YourMove`]'s counter is on.
/// Silencing and deciding stay two questions.
fn audible(cues: &[Cue]) -> Vec<Cue> {
    match cues.iter().find(|cue| cue.ends_the_game()) {
        Some(&ending) => vec![ending],
        None => cues.to_vec(),
    }
}

/// Plays one cue, if the player wants to hear anything.
///
/// The counter moves whether or not a sound comes out, so turning the volume
/// down and back up does not put [`Cue::YourMove`] back on the variant it was
/// on. The point of the cycle is that consecutive *firings* differ, not
/// consecutive audible ones.
fn sound(commands: &mut Commands, voices: &mut Voices, cue: Cue, level: Loudness) {
    let handle = voices.pick(cue).cloned();
    voices.played = voices.played.wrapping_add(1);
    if !level.audible() {
        return;
    }
    if let Some(handle) = handle {
        commands.spawn((
            AudioPlayer::new(handle),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(level.gain())),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::interaction::Outcome;

    /// What [`watch`] saw on the frame that just ran.
    ///
    /// "Did this frame mark the duel dirty" can only be asked from *inside*
    /// the frame. `App::update` ends with `World::clear_trackers`, which moves
    /// `last_change_tick` past every write the update made, so a `Ref<Duel>`
    /// taken afterwards answers `false` whatever the systems did — the first
    /// draft of the counter-test below asked from outside and passed with the
    /// early return deleted.
    #[derive(Resource, Default)]
    struct Dirtied(bool);

    /// Runs after [`play_the_cues`] and records whether it moved the tick.
    fn watch(duel: Res<Duel>, mut dirtied: ResMut<Dirtied>) {
        dirtied.0 = duel.is_changed();
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Duel>()
            .init_resource::<Dirtied>()
            .add_systems(Update, (play_the_cues, watch).chain());
        app
    }

    /// The drain is wired, which is the thing worth saying about a sink:
    /// "decided but never drained" is a queue that grows for the length of a
    /// game.
    #[test]
    fn a_decided_cue_is_drained_by_the_frame_after_it() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<Duel>()
            .cues
            .note_ending(Outcome::YouLost);
        assert_eq!(
            app.world().resource::<Duel>().cues.pending(),
            [Cue::GameLost]
        );
        app.update();
        let duel = app.world().resource::<Duel>();
        assert!(duel.cues.pending().is_empty(), "the queue was drained");
        assert_eq!(duel.cues.last(), Some(Cue::GameLost), "and remembered");
    }

    /// The counter-test for the early return: a frame with nothing to say
    /// must not report the duel as having changed, or every reader that
    /// watches the resource rebuilds on every frame of a quiet game.
    ///
    /// The second half is the counter-test's own counter-test. A watcher that
    /// can never see dirt would pass the first assertion however wrong the
    /// system was, so the cue goes in through `bypass_change_detection` —
    /// leaving [`play_the_cues`] as the only thing that can have moved the
    /// tick on the frame after it.
    #[test]
    fn a_silent_frame_does_not_touch_the_duel() {
        let mut app = app();
        // `init_resource` marked it; that is not a frame's doing.
        app.update();
        app.update();
        assert!(
            !app.world().resource::<Dirtied>().0,
            "a frame with no cues in it marked the whole duel dirty"
        );

        app.world_mut()
            .resource_mut::<Duel>()
            .bypass_change_detection()
            .cues
            .note_refusal();
        app.update();
        assert!(
            app.world().resource::<Dirtied>().0,
            "the watcher never sees dirt, so the assertion above proves nothing"
        );
    }

    /// A client with no device still decides, drains and remembers.
    ///
    /// The headless case, and the reason both resources are `Option` in the
    /// system's signature.
    #[test]
    fn a_client_with_no_device_still_drains() {
        let mut app = app();
        app.world_mut().resource_mut::<Duel>().cues.note_refusal();
        app.update();
        let duel = app.world().resource::<Duel>();
        assert!(duel.cues.pending().is_empty());
        assert_eq!(duel.cues.last(), Some(Cue::Refused));
    }

    /// Every cue is synthesised. A cue with no voice is silent at runtime and
    /// loud in no test.
    ///
    /// Run against the real startup system in a real asset world, because
    /// what can go wrong here is a missing *arm* rather than wrong
    /// arithmetic: [`RECIPES`] is eight entries and [`Cue::ALL`] is nine.
    #[test]
    fn every_cue_has_a_voice() {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<AudioSource>()
            .add_systems(Startup, voice_the_cues);
        app.update();
        let voices = app.world().resource::<Voices>();
        assert_eq!(voices.len(), Cue::ALL.len(), "a cue with no sound");
        for cue in Cue::ALL {
            assert!(voices.pick(cue).is_some(), "{} has no voice", cue.name());
        }
    }

    /// Nine cues, nine different sounds.
    ///
    /// A recipe copied and not edited is the easy mistake in a table this
    /// shape, and it is invisible: two cues that sound alike are a client
    /// saying two different things the same way. Compared as bytes, because
    /// the whole point of computing them is that they are reproducible.
    #[test]
    fn no_two_cues_sound_alike() {
        let mut rendered: Vec<Vec<u8>> = RECIPES
            .iter()
            .map(|(_, recipe)| wav(&render(recipe)))
            .collect();
        rendered.push(wav(&render(&tap(VARIANTS[0].0, VARIANTS[0].1))));
        let before = rendered.len();
        rendered.sort_unstable();
        rendered.dedup();
        assert_eq!(before, rendered.len(), "two cues render the same bytes");
    }

    /// And the six taps are six sounds, which is the whole of what keeps
    /// [`Cue::YourMove`] from being a metronome.
    #[test]
    fn no_two_taps_sound_alike() {
        let mut rendered: Vec<Vec<u8>> = VARIANTS
            .iter()
            .map(|&(hz, second)| wav(&render(&tap(hz, second))))
            .collect();
        let before = rendered.len();
        rendered.sort_unstable();
        rendered.dedup();
        assert_eq!(before, rendered.len(), "two taps render the same bytes");
    }

    /// Rendering twice gives the same bytes.
    ///
    /// The property the whole approach rests on: a generated sound that
    /// depended on a clock or on an unseeded RNG would be different on every
    /// run, and `no_two_cues_sound_alike` would pass while saying nothing.
    /// The mallet's noise is the only thing here that could break it.
    #[test]
    fn a_cue_renders_the_same_bytes_twice() {
        for (cue, recipe) in &RECIPES {
            assert_eq!(
                wav(&render(recipe)),
                wav(&render(recipe)),
                "{} is not reproducible",
                cue.name()
            );
        }
    }

    /// Nothing clips, and nothing is silent either.
    ///
    /// Bounded on **both** sides deliberately: a one-sided "quiet enough"
    /// assertion is what let a felt authored four times too dark through, and
    /// the same mistake in a sink is a cue that plays and cannot be heard.
    #[test]
    fn every_cue_is_loud_enough_to_hear_and_quiet_enough_not_to_clip() {
        let mut recipes: Vec<(Cue, Recipe)> = Vec::new();
        for (hz, second) in VARIANTS {
            recipes.push((Cue::YourMove, tap(hz, second)));
        }
        for (cue, recipe) in RECIPES.iter().chain(recipes.iter()) {
            let samples = render(recipe);
            let peak = samples.iter().fold(0.0f32, |a, s| a.max(s.abs()));
            let want = recipe.peak * MASTER;
            assert!(
                (peak - want).abs() < 0.01,
                "{} peaks at {peak}, asked for {want}",
                cue.name()
            );
            assert!(peak <= 1.0, "{} clips at {peak}", cue.name());
            assert!(peak > 0.1, "{} is inaudible at {peak}", cue.name());
        }
    }

    /// The two loudest things that can happen at once still fit.
    ///
    /// The two loudest *life* cues on one frame are what [`MASTER`] was
    /// chosen against: a change at this seat and one elsewhere, which is the
    /// combat damage step of any multiplayer turn. Written here so that a
    /// change to either peak fails rather than merely distorts.
    ///
    /// It is **not** the loudest frame there is — see
    /// [`an_ending_silences_the_life_cues_under_it`] for the one that is, and
    /// for what stops it.
    #[test]
    fn two_life_cues_on_one_frame_do_not_clip() {
        let together = peak_of(Cue::MyLifeLost) + peak_of(Cue::TheirLifeLost);
        assert!(together < 1.0, "two cues sum to {together}");
    }

    /// What a recipe is normalised to, after [`MASTER`].
    fn peak_of(cue: Cue) -> f32 {
        RECIPES
            .iter()
            .find(|(which, _)| *which == cue)
            .map(|(_, recipe)| recipe.peak * MASTER)
            .expect("a recipe")
    }

    /// A lethal hit is three cues on one frame, and three do not fit.
    ///
    /// The frame a game ends on carries the damage that ended it: a life loss
    /// here, a life loss there, and the ending. 0.70 + 0.28 + 0.63 is 1.61,
    /// which clips — at the loudest moment of the game, where a listener is
    /// least likely to forgive it. So an ending is played **alone**: it is
    /// the only thing on that frame anybody is listening for, and the life
    /// change that caused it is already drawn on two bars.
    #[test]
    fn an_ending_silences_the_life_cues_under_it() {
        let lethal = [Cue::MyLifeLost, Cue::TheirLifeLost, Cue::GameWon];
        let raw: f32 = lethal.iter().map(|&cue| peak_of(cue)).sum();
        assert!(raw > 1.0, "the three would not have clipped anyway");
        let kept = audible(&lethal);
        assert_eq!(kept, vec![Cue::GameWon], "the ending is not alone");
        let together: f32 = kept.iter().map(|&cue| peak_of(cue)).sum();
        assert!(together < 1.0, "the ending clips at {together}");
    }

    /// And an ordinary frame is left exactly as it was.
    #[test]
    fn a_frame_with_no_ending_keeps_every_cue() {
        let pair = [Cue::MyLifeLost, Cue::TheirLifeLost];
        assert_eq!(audible(&pair), pair.to_vec());
    }

    /// The header says what the bytes are.
    ///
    /// Forty-four bytes written by hand, so what can be wrong is a *field*,
    /// and a wrong field is a sound that plays at the wrong speed or does not
    /// decode at all — which on a device is indistinguishable from silence.
    #[test]
    fn the_riff_header_describes_the_samples_that_follow() {
        let samples = render(&RECIPES[0].1);
        let bytes = wav(&samples);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[36..40], b"data");
        let declared = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]);
        assert_eq!(
            declared as usize,
            samples.len() * 2,
            "the data chunk lies about its length"
        );
        assert_eq!(bytes.len(), 44 + samples.len() * 2, "trailing bytes");
        let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        assert_eq!(rate, 44_100, "the wrong sample rate is the wrong pitch");
        assert_eq!(
            u16::from_le_bytes([bytes[22], bytes[23]]),
            1,
            "not mono, so every second sample would be the other ear's"
        );
    }

    /// A buffer ends at silence.
    ///
    /// The fade is what makes this true; without it the tail is wherever the
    /// exponential had reached, and the step to zero is a click — the one
    /// artefact a listener always notices and never blames on the right
    /// thing.
    #[test]
    fn no_cue_ends_on_a_step() {
        for (cue, recipe) in &RECIPES {
            let samples = render(recipe);
            let last = samples.last().copied().unwrap_or_default();
            assert!(
                last.abs() < 0.001,
                "{} ends at {last}, which is a click",
                cue.name()
            );
        }
    }

    /// The taps are asked for in order and the order wraps.
    ///
    /// The cycle is the anti-metronome measure, and "played the same variant
    /// every time" is a bug that a listener would hear long before a test
    /// would — unless the test is this one.
    #[test]
    fn consecutive_taps_are_different_buffers() {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<AudioSource>()
            .add_systems(Startup, voice_the_cues);
        app.update();
        let mut voices = app.world_mut().resource_mut::<Voices>();
        let mut seen = Vec::new();
        for _ in 0..VARIANTS.len() * 2 {
            seen.push(voices.pick(Cue::YourMove).cloned().expect("a tap"));
            voices.played += 1;
        }
        for pair in seen.windows(2) {
            assert_ne!(pair[0], pair[1], "two taps in a row from one buffer");
        }
        assert_eq!(seen[0], seen[VARIANTS.len()], "the cycle does not wrap");
    }

    /// Writes every cue to `BAYLEE_CUE_DUMP` as a `.wav`, and hears nothing.
    ///
    /// `#[ignore]`d, because it writes files and asserts almost nothing. It
    /// exists because a *generated* sound has no other audit surface: every
    /// other decision in this client can be read off a screenshot or a
    /// number, and this one can only be listened to. `measure, do not
    /// theorise` applies to the ear as well.
    ///
    /// ```text
    /// BAYLEE_CUE_DUMP=/tmp/cues cargo test -p baylee-client --lib \
    ///     -- --ignored every_cue_written_out
    /// ```
    #[test]
    #[ignore = "writes files; run it when you want to hear them"]
    fn every_cue_written_out() {
        let Ok(dir) = std::env::var("BAYLEE_CUE_DUMP") else {
            panic!("set BAYLEE_CUE_DUMP to a directory");
        };
        std::fs::create_dir_all(&dir).expect("a directory");
        for (cue, recipe) in &RECIPES {
            let path = format!("{dir}/{}.wav", cue.name());
            std::fs::write(&path, wav(&render(recipe))).expect("written");
        }
        for (i, &(hz, second)) in VARIANTS.iter().enumerate() {
            let path = format!("{dir}/YourMove-{i}.wav");
            std::fs::write(&path, wav(&render(&tap(hz, second)))).expect("written");
        }
    }

    /// A [`Prefs`] with the sound turned off.
    ///
    /// Built through `Prefs::default` and one field, because the resource
    /// owns its own storage slot and a test that constructed it by hand would
    /// be asserting against a second spelling of it.
    fn muted() -> Prefs {
        let mut prefs = Prefs::default();
        prefs.edit().sound = Loudness::Off;
        prefs
    }

    /// A muted client plays nothing and still moves the cycle.
    ///
    /// Two claims in one test because they are the same decision: `Off` is a
    /// playback setting and not a second kind of silence, so the model above
    /// it goes on exactly as it did.
    #[test]
    fn off_spawns_nothing_and_still_advances() {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<AudioSource>()
            .init_resource::<Duel>()
            .insert_resource(muted())
            .add_systems(Startup, voice_the_cues)
            .add_systems(Update, play_the_cues);
        app.update();
        app.world_mut().resource_mut::<Duel>().cues.note_refusal();
        app.update();
        assert_eq!(
            app.world().resource::<Voices>().played,
            1,
            "a muted cue did not move the cycle"
        );
        let players = app
            .world_mut()
            .query::<&AudioPlayer>()
            .iter(app.world())
            .count();
        assert_eq!(players, 0, "something was played while muted");
    }
}
