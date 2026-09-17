//! The sink: where a decided cue becomes a noise.
//!
//! [`baylee_client_core::cue`] is the whole of the thinking — which moments
//! are worth hearing, and the arithmetic that makes a triple block one sound
//! instead of three. This is the other half: **thirty-seven buffers**,
//! computed at startup and played through `bevy_audio`.
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
//! The samples are uncompressed, and that is the answer to "why not Opus".
//! Nothing here is stored and nothing is transmitted: the bytes are made a
//! millisecond before rodio decodes them again, so a codec has nothing to
//! save and could only take quality away. (`bevy_audio` reads wav, flac,
//! vorbis, mp3 and aac; its decoders have no Opus at all, so the question has
//! a second answer as well.)
//!
//! # One instrument
//!
//! All twelve are the same thing: a **struck rosewood bar**, one bar, played in
//! two ways. Five modes of the bar and five of the contact, under a force
//! pulse whose *duration* is the whole of what a mallet is — see
//! [`mallet`] — and over a tube whose air column takes 27 ms to bloom.
//! A sine has no body, a sawtooth is a synthesiser, and wood is what belongs
//! on a table with a leather rail round it. Metal was the other candidate and
//! is wrong for the same reason it is wrong in a room: a bell's upper
//! partials outlive its fundamental, so two of them overlapping is a chord
//! nobody asked for. The clamp in [`modes`] is what holds this module to that
//! claim.
//!
//! What separates the twelve is **pitch, gesture, level and distance**,
//! never timbre:
//!
//! - the four life cues are one two-strike gesture, falling a minor third for
//!   a loss and rising a major third for a gain. Two intervals rather than
//!   one mirrored interval, because mirroring would make a gain sound minor;
//!   together they outline a triad, so a lifelink trade — [`Cue::MyLifeLost`]
//!   and [`Cue::TheirLifeGained`] on one frame — is a chord and not an
//!   argument.
//! - somebody **else's** life is that gesture a fourth lower, at a third of
//!   the level, struck with the *same* mallet and heard across the room.
//!   Distance is a room and a level; a gentler blow would say somebody was
//!   hit more gently, which is not what a seat across the table means.
//! - the three endings are the only sounds longer than a blink, and what they
//!   add is **ring** — the same bar, let ring instead of stopped — never a
//!   new timbre and never more loudness. All three peak below a life cue.
//!   Life cues speak in thirds, which have a colour; endings speak in perfect
//!   intervals, which have none: a fifth up, a fourth down, a unison. All
//!   three come to rest on A and never on the tonic. `docs/design.md` retires
//!   hues rather than handing out new ones, and this is the same restraint.
//!
//! [`Cue::Refused`] is the one exception and is honest about it: a knuckle on
//! the rail, with no bar in it at all.
//!
//! # `YourMove` is the one that could ruin it
//!
//! It fires on every priority grant — two hundred times in a long game — and
//! a fixed tone at that rate is a metronome. Four things answer it here: it
//! is the **lowest** sound in the set at 147 Hz, so it sits under the game
//! rather than over it; it is the **quietest**, peaking at 0.22 against a
//! life cue's 1.00; it is **one touch** rather than a two-note gesture,
//! because a melody is a thing that repeats and a tap is not; and it is one
//! of **six variants** cycled in a fixed order, each detuned within ±16 cents
//! *and struck in a different place on the bar*. The spot is the stronger
//! half of that pair — it swings the 4× partial from 0.99 to 0.59 and lifts
//! an antisymmetric mode out of silence, where 16 cents on a yarn-struck
//! 147 Hz bar is barely there.
//!
//! It is *not* the shortest, and that is deliberate rather than an oversight:
//! a soft low note with a long decay is felt and then forgotten, where a
//! short one is a *click*, and a click two hundred times is the metronome
//! this is trying not to be.
//!
//! What is **not** built is the policy half, and it is the larger half: a
//! grant that follows the player's own action tells them nothing, so a
//! debounce of about 600 ms, a suppression window after the seat sends
//! anything, a refractory period, and a louder cue when the window is in the
//! background would together turn two hundred grants into a few dozen
//! touches. That belongs in `baylee-client-core` beside `reconnect.rs`, where
//! it can be tested without a device — and it wants a clock passed in, which
//! [`baylee_client_core::cue::Cues`] has no field for yet. Until it exists
//! the setting is the answer, and `Loudness::Off` is one chip on the settings
//! screen.
//!
//! # Three of them count
//!
//! The other deliberate omission used to be loudness by **amount**, and the
//! reasoning was right about where such a change had to happen and wrong
//! about what it was. A twelve-point hit is not a louder one-point hit — it
//! is one event whose number is already on the bar. But drawing three cards
//! *is* three events, and so is a resolution that puts counters on three
//! creatures, and the owner asked for both by name: "so that when several
//! cards are drawn, you hear that too".
//!
//! So the model grew a count ([`baylee_client_core::cue::Beat`]) and the sink
//! grew a buffer per count. A burst is the same blow struck N times at a gap
//! that **accelerates** — [`BURST_SQUEEZE`], because an even run is the
//! metronome this module spends most of its length avoiding — with every blow
//! taking its own row of a ladder, so three is three blows and never one
//! buffer played three times.
//!
//! - [`Cue::CardDrawn`] is one tap on **D5**, the tonic two octaves above
//!   [`Cue::YourMove`]: D3 is this seat's wait, D4 its life, D5 its hand. See
//!   [`draw`], which also states the one place this bends the rules above —
//!   it is the shortest sound here and it fires often.
//! - [`Cue::CreatureGrew`] and [`Cue::CreatureShrank`] are the life gesture's
//!   **diminutive**: the same word — more, or less — a fifth up, two and a
//!   half times faster, on a softer stick and at a third of the level. See
//!   [`counters`] for why A4 is the only note the pair can be built from.
//!
//! What that costs is the frame budget. [`MASTER`] was chosen when two cues
//! were the worst case an ordinary frame could hold, and they are not: a
//! *Sign in Blood* is a life loss and a draw, a *Fathom Mage* is a counter
//! and a draw. So [`audible`] now fills a frame greedily by [`rank`] and
//! drops what will not fit, instead of assuming everything always did.

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;

use crate::Duel;
use crate::prefs::Prefs;
use baylee_client_core::cue::{Beat, Cue, Loudness};
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

// --------------------------------------------------------------- the mallet

/// How long the mallet is in contact with the bar, in seconds.
///
/// This is the **whole** of what a mallet is here, and that is the correction
/// the second pass made. A mallet used to be a noise burst with its own
/// cutoff and its own attack ramp, *summed on top of* a fixed partial
/// spectrum — so a softer stick was the identical note with a different click
/// in front of it, and the ear heard two sounds rather than one. A real blow
/// is a force pulse of some duration, and a duration is a low-pass filter on
/// whatever it drives: [`contact_gain`] is that filter, and it reaches every
/// mode at once. Nothing else about a mallet needs saying.
///
/// The four are a ladder in contact time, and the spectrum follows from it:
/// hard at 1.2 ms passes 9 kHz, yarn at 4 ms is down 12 dB by 2 kHz and takes
/// the knock with it. That is also honestly **not** scale-invariant — the
/// same stick reads harder at A4 than at D3, because the filter is on
/// `f · contact` and not on the ratio — which is how a real instrument
/// behaves and is what the old fixed spectrum could not say.
mod mallet {
    /// A hard rubber core. The life cues, which have to be heard *through* a
    /// game rather than under it.
    pub(super) const HARD: f32 = 0.0012;
    /// Wound, medium. The endings, where the table is being set down.
    pub(super) const SOFT: f32 = 0.0025;
    /// Yarn. [`super::Cue::YourMove`], the two hundred times a game one.
    pub(super) const YARN: f32 = 0.0040;
    /// A knuckle, which is not a mallet at all. [`super::Cue::Refused`].
    pub(super) const KNUCKLE: f32 = 0.0060;
}

/// What a contact time of `contact` does to a component at `hz`.
///
/// `1/√(1 + (f·τ)²)`, a one-pole roll-off whose corner is `1/(2πτ)`. It is
/// the one number that makes a mallet a mallet, and the reason the partial
/// gains below are *impulse* gains: what the bar would give under a blow of
/// no duration. There are no blows of no duration.
fn contact_gain(hz: f32, contact: f32) -> f32 {
    let x = hz * contact;
    1.0 / (1.0 + x * x).sqrt()
}

/// How far the mallet lands from the centre of the bar, as a fraction of the
/// distance to the node of the 4× partial (0.144 of the bar's length).
///
/// A player does not hit the same spot twice, and the spot is *far* more
/// audible than a few cents of tuning: across this range the 4× partial
/// swings from 0.99 of its full strength to 0.59, and an antisymmetric mode
/// that is silent at the centre comes up out of nothing. That is what makes
/// two strikes in one cue two blows rather than one buffer played twice, and
/// it is most of what keeps [`Cue::YourMove`] from being a metronome.
#[derive(Clone, Copy)]
struct Spot(f32);

impl Spot {
    /// What this spot does to the fundamental, the 4× partial and the
    /// antisymmetric mode.
    ///
    /// Cosines of the mode shape at the striking point, which is why the
    /// fundamental barely moves (it has no node anywhere on the bar's
    /// striking half) and the 4× partial dies as the node is approached. The
    /// antisymmetric mode is a sine of it: exactly zero at the centre, which
    /// is why a centred strike has no trace of it at all.
    fn shape(self) -> (f32, f32, f32) {
        let o = self.0;
        (
            (0.82 * o).cos(),
            (std::f32::consts::FRAC_PI_2 * o).cos(),
            0.40 * (0.75 * o).sin(),
        )
    }
}

// ------------------------------------------------------------------ the bar

/// D4, the bar the whole instrument is tuned from.
const ANCHOR: f32 = 293.66;

/// Two hundred and fifty of quality factor, for the tube under the bar.
///
/// A resonator tube is not a filter and not more ring: it is a *rise*. The
/// air column takes `Q/(πf₀)` to come up to strength — 27 ms at D4, 54 ms at
/// D3 — which is the bloom a marimba has and a synthesiser does not, and it
/// is what softens the seam between the knock and the note. The first draft
/// of this module claimed the endings "add the resonator tube" and only made
/// `ring` longer, which is a bar that changes size — the one thing "one
/// instrument" forbids.
const TUBE_Q: f32 = 25.0;

/// How long the fundamental of a freely ringing bar lasts, at `hz`.
///
/// `0.28 s` at D4 and longer as the bar gets bigger, which is the law a
/// marimba obeys: 0.49 s at D3, 0.20 s at A4. Every cue takes its ring from
/// this and a damping factor, so the endings are the **same bar** let ring
/// and the life cues are that bar stopped — rather than, as before, a bar
/// whose size was written down per cue.
fn free_ring(hz: f32) -> f32 {
    0.28 * (ANCHOR / hz).powf(0.8)
}

/// One mode of the bar: where it sits, how hard the blow drives it, and how
/// long it lives.
#[derive(Clone, Copy)]
struct Mode {
    hz: f32,
    /// What an impulse would give it, **before** [`contact_gain`].
    gain: f32,
    /// Seconds to `1/e` of the fast component. See [`envelope`].
    tau: f32,
}

/// Every mode a struck bar gives at this pitch, with this mallet, here.
///
/// Five of the bar and five of the contact, and the interesting half is that
/// the ratios **move with pitch**. A marimba tuner holds 1 : 4 : 10 in the
/// low bars and cannot hold it at the top — the bar runs out of the
/// undercutting that buys the tuning — so the ratios slide about a tenth of
/// an octave per octave. The slope is a choice rather than a citation; what
/// it buys is that D3 and A4 are not the same waveform at two speeds, which
/// is the first thing an ear catches in a set this size.
///
/// The decay law is the other half: `τ_k = τ₁ · r^-1.2`, **clamped** to 90 ms
/// above the fundamental and 30 ms from the third mode up. The clamp is the
/// bell guard. Without it a long ring drags its top along — [`Cue::GameLost`]'s A3
/// would have carried 2106 Hz for 90 ms and [`Cue::GameWon`]'s A4 4211 Hz for 67 —
/// which is precisely the metal this module's header says it is not.
fn modes(hz: f32, contact: f32, spot: Spot, ring: f32) -> Vec<Mode> {
    let octaves = (hz / ANCHOR).log2();
    let r2 = 3.92 - 0.15 * octaves;
    let r3 = 9.56 - 0.80 * octaves;
    let r4 = 1.9 * r3;
    let (g1, g4, ganti) = spot.shape();

    let bar = [
        (1.0, 1.00 * g1, ring),
        (r2, 0.55 * g4, (ring * r2.powf(-1.2)).min(0.090)),
        (r3, 0.30, (ring * r3.powf(-1.2)).min(0.030)),
        (r4, 0.12, (ring * r4.powf(-1.2)).min(0.030)),
        (2.63, ganti, (ring * 2.63_f32.powf(-1.2)).min(0.090)),
    ];

    let mut out: Vec<Mode> = bar
        .iter()
        .filter(|(_, gain, _)| gain.abs() > 1e-4)
        .map(|&(ratio, gain, tau)| {
            let f = hz * ratio;
            Mode {
                hz: f,
                gain: gain * contact_gain(f, contact),
                tau,
            }
        })
        .collect();

    // The knock: local deformation where the stick lands, which is a property
    // of the *contact* and not of the bar, so these are absolute frequencies
    // and do not move with pitch. Filtered noise was what stood here before,
    // and filtered noise is sand — a wooden knock is a handful of very high,
    // very heavily damped modes, gone in a millisecond or two. The yarn
    // mallet loses them without being told to: `contact_gain(6620, 0.004)` is
    // 0.04.
    for &(f, tau) in &KNOCK {
        out.push(Mode {
            hz: f,
            gain: 0.20 * contact_gain(f, contact),
            tau,
        });
    }
    out
}

/// The contact modes, in hertz and seconds.
const KNOCK: [(f32, f32); 5] = [
    (1900.0, 0.0025),
    (2830.0, 0.0020),
    (3710.0, 0.0016),
    (5090.0, 0.0012),
    (6620.0, 0.0010),
];

/// How one mode's amplitude moves through a strike.
///
/// Three things at once, and each of them answers a specific complaint about
/// the first draft:
///
/// - the **onset** is the integral of the force pulse — a raised-cosine ramp
///   over the contact time — so every mode comes up together and in step with
///   the blow. A note that started at full amplitude on sample zero is a
///   click, which is why the first draft needed a click glued on.
/// - the **decay is compound**, `0.72·e^(−t/τ) + 0.28·e^(−t/2.3τ)`. A single
///   exponential is a straight line in decibels, which is the organ-stop
///   tell; a real bar loses its energy two ways (to the air fast, into the
///   mount slowly) and the slow part is what makes an ending last without
///   being made louder or longer on paper.
/// - `t` is measured from the strike, so a mode struck late in a cue is not
///   somewhere in the middle of its own life.
fn envelope(t: f32, tau: f32, contact: f32) -> f32 {
    if t < 0.0 {
        return 0.0;
    }
    let onset = if t < contact {
        let x = t / contact;
        x - (std::f32::consts::TAU * x).sin() / std::f32::consts::TAU
    } else {
        1.0
    };
    let fast = (-t / tau).exp();
    let slow = (-t / (2.3 * tau)).exp();
    onset * (0.72 * fast + 0.28 * slow)
}

// --------------------------------------------------------------- the strikes

/// One blow.
#[derive(Clone, Copy)]
struct Strike {
    /// Seconds after the start of the buffer.
    at: f32,
    /// The bar's fundamental, in hertz. `None` is [`Cue::Refused`], which is
    /// not the bar at all.
    hz: f32,
    /// How long the fundamental rings, as a fraction of [`free_ring`]. The
    /// endings are 1.0 and everything else is stopped.
    damp: f32,
    /// Which mallet.
    contact: f32,
    /// Where on the bar.
    spot: Spot,
    /// Relative to the cue's own level.
    gain: f32,
    /// Whether the tube under the bar is in play.
    tube: bool,
}

/// Writes one strike into a mono buffer.
fn strike(buf: &mut [f32], blow: &Strike, noise: &mut Noise) {
    let ring = free_ring(blow.hz) * blow.damp;
    let modes = modes(blow.hz, blow.contact, blow.spot, ring);
    let start = (blow.at * RATE) as usize;
    let rise = TUBE_Q / (std::f32::consts::PI * blow.hz);

    // The fundamental is a *pair*, a tenth of a percent apart, half strength
    // each. Two nearly-equal modes beat at their difference — 1.4 s at D4,
    // 1.9 s at A3 — so only a sound that lives that long shows it, which is
    // exactly the three endings. A bar that is perfectly one frequency is a
    // sine, and the ear knows.
    for (i, sample) in buf.iter_mut().enumerate().skip(start) {
        let secs = (i - start) as f32 / RATE;
        let turns = std::f32::consts::TAU * secs;
        let mut here = 0.0;
        for (n, mode) in modes.iter().enumerate() {
            let amp = mode.gain * envelope(secs, mode.tau, blow.contact);
            if n == 0 {
                here += 0.5 * amp * (turns * mode.hz * 0.9988).sin();
                here += 0.5 * amp * (turns * mode.hz * 1.0012).sin();
            } else {
                here += amp * (turns * mode.hz).sin();
            }
        }
        if blow.tube {
            let amp = 0.5 * (1.0 - (-secs / rise).exp()) * (-secs / ring).exp();
            here += amp * (turns * blow.hz).sin();
        }
        *sample += blow.gain * here;
    }

    // Grit: the stick's own surface, audible only while it is touching. It
    // lives inside the contact window and has no decay of its own, which is
    // the difference between a texture and a shaker.
    grit(
        buf,
        start,
        blow.contact,
        if blow.contact < 0.002 { 0.03 } else { 0.01 },
        5000.0,
        noise,
    );
}

/// Noise under the blow, for exactly as long as the blow lasts.
fn grit(buf: &mut [f32], start: usize, contact: f32, gain: f32, cut: f32, noise: &mut Noise) {
    let n = (contact * RATE) as usize;
    let a = pole(cut);
    let mut lp = 0.0;
    for i in 0..n {
        let Some(sample) = buf.get_mut(start + i) else {
            break;
        };
        let x = i as f32 / n as f32;
        let window = 0.5 * (1.0 - (std::f32::consts::TAU * x).cos());
        lp = a.mul_add(lp - noise.next(), noise.next());
        *sample += gain * window * lp;
    }
}

/// [`Cue::Refused`] is not the bar, and says so.
///
/// A knuckle on the rail: one low mode with nothing above it, and three
/// leather slaps that are gone in three milliseconds. The first draft put it
/// on the bar at G3 with a 20 ms ring, which is four cycles — no pitch is
/// perceived at all, so the note it was "outside the working scale" by was
/// inaudible, and the cue was a click with a justification. This has no
/// pitch to be wrong about.
fn thud(buf: &mut [f32], at: f32, gain: f32, noise: &mut Noise) {
    const BODY: [(f32, f32); 4] = [
        (190.0, 0.018),
        (900.0, 0.003),
        (1400.0, 0.0025),
        (2100.0, 0.002),
    ];
    let start = (at * RATE) as usize;
    for (i, sample) in buf.iter_mut().enumerate().skip(start) {
        let t = (i - start) as f32 / RATE;
        let mut v = 0.0;
        for (n, &(hz, tau)) in BODY.iter().enumerate() {
            let a = if n == 0 { 1.0 } else { 0.45 } * envelope(t, tau, mallet::KNUCKLE);
            v += a * (std::f32::consts::TAU * hz * t).sin();
        }
        *sample += gain * v;
    }
    grit(buf, start, mallet::KNUCKLE, 0.25, 2500.0, noise);
}

// ------------------------------------------------------------------ the room

/// Where the listener is standing relative to the sound.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Where {
    /// This seat, and the endings: the sound is in front of the player.
    Here,
    /// Another seat: across the table.
    Elsewhere,
    /// An ending, which is the same place as `Here` and wetter.
    Ending,
}

impl Where {
    /// Seconds between the direct sound and the first reflection.
    ///
    /// This is the cue the ear actually uses for distance, and the first
    /// draft had it **backwards**: it put 19 ms on the far sounds, and a
    /// *near* source is the one with the large gap. A far source's first
    /// reflection arrives almost with it.
    fn gap(self) -> f32 {
        match self {
            Self::Elsewhere => 0.005,
            Self::Here | Self::Ending => 0.012,
        }
    }

    /// Dry, early and late, as fractions of the direct sound.
    fn mix(self) -> (f32, f32, f32) {
        match self {
            Self::Here => (1.00, 0.20, 0.10),
            Self::Elsewhere => (0.40, 0.36, 0.30),
            Self::Ending => (1.00, 0.25, 0.40),
        }
    }
}

/// Six early reflections, in seconds and gain — **the same set in both
/// ears**.
///
/// Two things fix these numbers, and the second one was learned twice.
///
/// They are at no simple ratio to one another, because a delay at a simple
/// ratio is a comb filter and a comb filter has a **pitch**. That is not a
/// theoretical worry: the single 19 ms tap this replaces notched at
/// (n+½)·52.63 Hz, and F#3 — the second strike of [`Cue::TheirLifeLost`] —
/// sits 0.7 Hz from the n = 3 notch while A3, its first strike, sits near a
/// peak. A gesture written as a falling third at 0.80 then 1.00 was *heard*
/// as roughly 0.92 then 0.70: the room had inverted the dynamic of the cue.
///
/// And they are **one set**, not one per ear, which is the correction the
/// first stereo draft needed. Two different tap sets are two different combs,
/// and two combs disagree: measured over the eight pitches this instrument
/// uses, the two sets were −0.99 correlated at A3 and 16 dB apart in level
/// there — a hard pan and a near-total cancellation on any mono sum, at
/// exactly the pitch the "elsewhere" cues are built on. Physically it was
/// wrong as well: a reflection off a table reaches both ears within a
/// fraction of a millisecond, and what differs between the ears in a real
/// room is the diffuse tail. So the ears share their early reflections and
/// [`late`] is the only thing that differs.
///
/// The set itself is searched rather than chosen: over D3, F#3, A3, B3, C#4,
/// D4, F#4 and A4 it is flat to **2.2 dB**, against 21 dB for the first
/// draft's, with no two taps closer than 4.8 ms or further apart than 9.
/// Whatever colour it has, both ears have it, so it colours and never pans.
const EARLY: [(f32, f32); 6] = [
    (0.0139, 0.50),
    (0.0218, 0.42),
    (0.0271, 0.36),
    (0.0319, 0.29),
    (0.0390, 0.23),
    (0.0480, 0.18),
];

/// The late field's delay lines, per channel, in seconds.
///
/// Long, and that is the whole point. Four lines of 23–54 ms — the first
/// draft's — space their modes 19 to 43 Hz apart, and with a feedback of
/// 0.795 each mode has a gain of nearly five. Measured on a decaying note,
/// that reverb *amplified* A3 by 2.3× and D3 by 0.95: a three-to-one
/// coloration with its worst peak sitting on the pitch two of the life cues
/// are built from. It is finding three over again — a delay network has a
/// pitch — one layer further in.
///
/// At 63–115 ms the modes are 9 to 16 Hz apart, so every note sits on several
/// at once, and `10^(−3d/RT60)` puts the feedback at 0.33 rather than 0.80,
/// which is a gain of 1.5 instead of 4.9. Measured over the eight pitches the
/// instrument uses and both ears: **1.79** worst-to-best, against 2.80.
const LATE_L: [f32; 4] = [0.0631, 0.0797, 0.0971, 0.1153];
/// The other ear's — the *only* thing that differs between them. See
/// [`EARLY`] for why it is this and not the reflections.
const LATE_R: [f32; 4] = [0.0673, 0.0761, 0.1013, 0.1117];

/// How long the late field takes to fall 60 dB.
const RT60: f32 = 0.7;

/// What the late field is multiplied by, so that its mix number means what it
/// says.
///
/// A reverb has gain: it sums many delayed copies of a note that is still
/// sounding, so its peak is not the note's peak. Measured across the eight
/// pitches and both ears, this network averages about 1.1, so 0.9 makes
/// `Where::mix`'s third number a fraction of the dry peak rather than a
/// number to be tuned by ear against an unknown. Without it the *reverb* set
/// the level: `GameDrawn`'s loudest moment was 112 ms into a tail whose
/// second strike does not land until 420, which is a cue whose loudness is
/// decided by the room rather than by the blow.
const LATE_TRIM: f32 = 0.9;

/// A one-pole coefficient for a corner at `cut`.
fn pole(cut: f32) -> f32 {
    (-std::f32::consts::TAU * cut / RATE).exp()
}

/// The early reflections, low-passed. One set, heard by both ears.
fn early(dry: &[f32], gap: f32, cut: f32) -> Vec<f32> {
    let mut out = vec![0.0; dry.len()];
    for &(delay, gain) in &EARLY {
        let d = ((delay + gap) * RATE) as usize;
        for i in d..dry.len() {
            out[i] += gain * dry[i - d];
        }
    }
    let a = pole(cut);
    let mut lp = 0.0;
    for sample in &mut out {
        lp = a.mul_add(lp - *sample, *sample);
        *sample = lp;
    }
    out
}

/// One channel's late field: a feedback delay network.
///
/// Two series allpasses to smear the input, then four delay lines mixed into
/// one another by a Hadamard matrix — the standard way to get a dense tail
/// out of very little arithmetic, and *pure delay arithmetic*, which is what
/// keeps it deterministic. The one-pole inside each loop is why the tail gets
/// darker as it dies, which every real room does and no bare delay network
/// does.
fn late(dry: &[f32], lines: &[f32; 4], gap: f32) -> Vec<f32> {
    let n = dry.len();

    // Smear first: an allpass passes every frequency at the same level and
    // scrambles their arrival, which is what turns four echoes into a wash.
    let mut smeared = dry.to_vec();
    for &(delay, g) in &[(0.0043_f32, 0.6_f32), (0.0019, 0.6)] {
        let d = (delay * RATE) as usize;
        let mut buf = vec![0.0; n];
        let mut store = vec![0.0; d.max(1)];
        for i in 0..n {
            let z = store[i % d.max(1)];
            let v = smeared[i] + g * z;
            buf[i] = z - g * v;
            store[i % d.max(1)] = v;
        }
        smeared = buf;
    }

    let delays: Vec<usize> = lines.iter().map(|&d| (d * RATE) as usize).collect();
    let gains: Vec<f32> = lines
        .iter()
        .map(|&d| 10_f32.powf(-3.0 * d / RT60))
        .collect();
    let a = pole(3200.0);

    let mut store: Vec<Vec<f32>> = delays.iter().map(|&d| vec![0.0; d.max(1)]).collect();
    let mut lp = [0.0_f32; 4];
    let mut out = vec![0.0; n];
    let pre = (gap * RATE) as usize;

    for i in 0..n {
        let feed: Vec<f32> = (0..4)
            .map(|k| {
                let d = delays[k].max(1);
                store[k][i % d]
            })
            .collect();

        // Hadamard/2: every line hears every other, which is what makes the
        // tail dense instead of four separate echoes.
        let half = 0.5;
        let mix = [
            half * (feed[0] + feed[1] + feed[2] + feed[3]),
            half * (feed[0] - feed[1] + feed[2] - feed[3]),
            half * (feed[0] + feed[1] - feed[2] - feed[3]),
            half * (feed[0] - feed[1] - feed[2] + feed[3]),
        ];

        let input = if i >= pre { smeared[i - pre] } else { 0.0 };
        for k in 0..4 {
            let d = delays[k].max(1);
            lp[k] = a.mul_add(lp[k] - mix[k], mix[k]);
            store[k][i % d] = input + gains[k] * lp[k];
        }
        out[i] = LATE_TRIM * 0.5 * feed.iter().sum::<f32>();
    }
    out
}

// ----------------------------------------------------------------- a recipe

/// Everything one cue needs.
struct Recipe {
    strikes: &'static [Strike],
    /// `Some` for [`Cue::Refused`], which is a knuckle and not the bar.
    thuds: &'static [(f32, f32)],
    /// Seconds. Held against the tail by `no_cue_ends_on_a_step`.
    len: f32,
    /// What the finished buffer is normalised to, before [`MASTER`].
    peak: f32,
    room: Where,
}

/// A strike with the fields most cues share.
const fn blow(at: f32, hz: f32, damp: f32, contact: f32, spot: f32, gain: f32) -> Strike {
    Strike {
        at,
        hz,
        damp,
        contact,
        spot: Spot(spot),
        gain,
        tube: true,
    }
}

/// The scale, in hertz.
///
/// Just intonation off D, so the intervals are exact small ratios rather than
/// twelve-root-of-two approximations of them: a 6:5 minor third has no beat
/// between its partials and an equal-tempered one has a slow one, which over
/// a two-note gesture is the difference between a chord and a wobble.
mod note {
    /// The bar the instrument is tuned from.
    pub(super) const D4: f32 = super::ANCHOR;
    /// An octave below it. [`super::Cue::YourMove`].
    pub(super) const D3: f32 = 146.83;
    /// A fifth above D4.
    pub(super) const A4: f32 = 440.49;
    /// A fourth below D4 — where the other seats' cues sit.
    pub(super) const A3: f32 = 220.25;
    /// A minor third below D4.
    pub(super) const B3: f32 = 244.72;
    /// A major third above D4.
    pub(super) const FS4: f32 = 367.08;
    /// A minor third below A3.
    pub(super) const FS3: f32 = 183.54;
    /// A major third above A3.
    pub(super) const CS4: f32 = 275.31;
    /// The octave above [`D4`]. [`super::Cue::CardDrawn`].
    ///
    /// Exactly 2 × the anchor, which is the most just interval there is, and
    /// the reason the draw sits on the tonic rather than beside it: D3 is
    /// this seat's wait, D4 is this seat's life, D5 is this seat's hand —
    /// three octaves of one note for three sizes of one owner. Every other
    /// candidate collides. A4 is where all three endings come to rest, and
    /// the most frequent cue in the set must not dilute that; F#4, B3 and C#4
    /// are the coloured second notes of the life gestures, so a draw on one
    /// of them would sound like half a life cue.
    pub(super) const D5: f32 = 587.32;
    /// A major third above [`A4`], and the octave above [`CS4`].
    ///
    /// Where a +1/+1 counter's gesture lands. See [`super::counters`] for why
    /// A4 is the only note on this scale that gesture can be built from.
    pub(super) const CS5: f32 = 550.62;
}

// ------------------------------------------------------------------ a burst

/// How the gaps inside a burst shrink, blow by blow.
///
/// A **smooth accelerando**, which is what keeps a run of near-identical
/// blows from being the metronome this module spends so much effort avoiding
/// — and it is also what a hand dealing cards does. A bouncing ball is
/// trivially counted and a clock is not, so speeding up costs nothing in
/// countability and buys the whole difference in character. Jitter would buy
/// the same character and lose the count.
const BURST_SQUEEZE: f32 = 0.955;

/// The first gap between two drawn cards, in seconds.
///
/// 118 ms is about eight and a half cards a second, and the seventh gap is
/// still 94 — comfortably above the ~40 ms where two taps fuse into one, and
/// above the 80 ms where a listener starts to lose the last of them.
const DRAW_GAP: f32 = 0.118;

/// The first gap between two creatures taking counters, in seconds.
///
/// Nearly twice the draw's, because each of these is a two-note *gesture* and
/// not a tap: the gap from one gesture's second blow to the next gesture's
/// first is 145 ms against the 45 ms inside a gesture, a ratio of 3.2 to 1.
/// Below about three to one the ear regroups the pair across the boundary and
/// the count comes apart.
const COUNTER_GAP: f32 = 0.190;

/// How far apart a counter gesture's two blows are, in seconds.
///
/// Inside the ~50 ms window two sounds are heard as one event, which is what
/// the count needs — three creatures must be three events, not six. And above
/// the ~20 ms where the *order* of two brief tones stops being identifiable,
/// which is what the direction needs: at 45 ms a player hears the second note
/// land above or below the first rather than inferring it. The first blow
/// gets twenty cycles at A4 before the second arrives, so it is a pitch and
/// not a click.
const FLAM: f32 = 0.045;

/// The k-th gap of a burst whose first gap is `first`.
fn gap(first: f32, k: usize) -> f32 {
    first * BURST_SQUEEZE.powi(i32::try_from(k).unwrap_or(0))
}

/// [`Cue::CardDrawn`]'s seven blows, as (cents, spot, gain).
///
/// Unsorted in every column, for the reason [`VARIANTS`] is: a ladder that
/// ramped would be a gesture, and a burst has to be a count. Cents stay
/// within ±12 and the gains within a tenth — a *small* dynamic, so the run is
/// not a machine, but never an accent, because an accented blow is heard as a
/// downbeat and a downbeat groups the count into bars.
const DRAW_LADDER: [(f32, f32, f32); 7] = [
    (0.0, 0.12, 1.00),
    (8.0, 0.42, 0.94),
    (-5.0, 0.28, 0.97),
    (12.0, 0.58, 0.91),
    (-10.0, 0.20, 0.95),
    (4.0, 0.50, 0.92),
    (-8.0, 0.35, 0.98),
];

/// The same for a counter gesture, as (cents, spot of the first blow, spot of
/// the second).
///
/// The detune moves **both** blows of a gesture together. The whole point of
/// tuning this instrument in just intonation is that a 6:5 third has no beat
/// between its partials; detuning one note of the pair and not the other puts
/// one there, which is the one thing a counter cue must not do — the interval
/// *is* the message.
const COUNTER_LADDER: [(f32, f32, f32); 5] = [
    (0.0, 0.15, 0.30),
    (9.0, 0.40, 0.20),
    (-6.0, 0.25, 0.45),
    (13.0, 0.55, 0.10),
    (-11.0, 0.10, 0.35),
];

/// Which ladder rows a count-of-one buffer is built from.
///
/// Three of them, cycled by [`Voices::pick`] exactly as [`Cue::YourMove`]'s
/// six are, and for the same reason: one is the count that fires forty times
/// a game, where a burst already varies inside itself. Rows 1, 3 and 5 rather
/// than the first three, so a solo is not the head of a burst.
const SOLOS: [usize; 3] = [0, 2, 4];

/// How a drawn card sounds, `rows.len()` of them.
///
/// One blow per card and nothing else. A two-note gesture per card would make
/// three cards six blows, and six blows in 600 ms is a texture rather than a
/// number — the whole requirement here is that a player can *count* it.
///
/// `damp` 0.30 leaves 48 ms of fundamental, which is 28 cycles at D5: short
/// enough that seven do not smear, long enough to be a pitch rather than the
/// click this module's header warns about. The mallet is [`mallet::HARD`],
/// which is where the snap is — a card is mostly contact and a little body,
/// and the contact is exactly what [`KNOCK`] is.
///
/// This is the shortest sound on the bar and it fires often, which is a
/// **bend**: the header argues at length that [`Cue::YourMove`] is
/// deliberately not the shortest, because a click two hundred times is a
/// metronome. Counting needs it — nobody counts 290 ms notes at 100 ms gaps —
/// and three things replace the defence: the solo variants and the per-blow
/// ladder, a rate nearer forty a game than two hundred, and a level of 0.20.
/// The "felt and then forgotten" argument does not carry over either:
/// `YourMove` is a nudge, and a draw is information the owner asked to hear.
fn draw(rows: &[usize]) -> Recipe {
    let mut strikes = Vec::with_capacity(rows.len());
    let mut at = 0.0;
    for (k, &row) in rows.iter().enumerate() {
        // The gap goes *between* blows, so it is taken before all but the
        // first. Added after every blow instead it would put a hundred
        // milliseconds of tail on the end that nobody asked for, and leave
        // `len` lying about the span.
        if k > 0 {
            at += gap(DRAW_GAP, k - 1);
        }
        let (cents, spot, gain) = DRAW_LADDER[row];
        strikes.push(blow(
            at,
            note::D5 * (cents / 1200.0).exp2(),
            0.30,
            mallet::HARD,
            spot,
            gain,
        ));
    }
    Recipe {
        strikes: strikes.leak(),
        thuds: &[],
        len: 0.90 + at,
        peak: if rows.len() > 1 { 0.22 } else { 0.20 },
        room: Where::Here,
    }
}

/// How creatures taking power/toughness counters sound, `rows.len()` of them.
///
/// A **diminutive of the life gesture**, which is the argument for the cue
/// existing at all: a counter says the word life says — more, or less — one
/// size smaller. So it is the same shape (a note, then a third), two and a
/// half times faster, ringing a third as long, struck with a softer stick and
/// at a third of the level. A softer stick is *right* here where it would be
/// wrong for a distant seat: "a gentler blow says somebody was hit more
/// gently" is exactly what a counter means against a life total.
///
/// A4 is the only note on this scale the gesture can be built from, and the
/// enumeration is short enough to write down. D4 and A3 are life's. From F#4
/// a major third *up* is a minor third by interval, so the colour inverts.
/// From B3 neither third lands in D major. D3 is [`Cue::YourMove`]'s. D5 is
/// the draw's, and a shrink's first blow would then *be* a draw blow — which
/// a Fathom Mage puts on one frame. A4 leaves [`note::CS5`] above and
/// [`note::FS4`] below, both on the scale and neither on the tonic.
///
/// The grace note is the quieter of the pair (0.80 against 1.00) because the
/// *destination* carries the direction, which is where the ear is going.
fn counters(up: bool, rows: &[usize]) -> Recipe {
    let mut strikes = Vec::with_capacity(rows.len() * 2);
    let mut at = 0.0;
    for (k, &row) in rows.iter().enumerate() {
        // Between gestures, not after each; see [`draw`].
        if k > 0 {
            at += gap(COUNTER_GAP, k - 1);
        }
        let (cents, first, second) = COUNTER_LADDER[row];
        let shift = (cents / 1200.0).exp2();
        let lands = if up { note::CS5 } else { note::FS4 };
        strikes.push(blow(at, note::A4 * shift, 0.35, mallet::SOFT, first, 0.80));
        strikes.push(blow(
            at + FLAM,
            lands * shift,
            0.35,
            mallet::SOFT,
            second,
            1.00,
        ));
    }
    let loud = if up { 0.28 } else { 0.34 };
    Recipe {
        strikes: strikes.leak(),
        thuds: &[],
        len: 1.00 + at,
        // The same +0.02 the draw takes, and for the same reason: `level`
        // normalises the whole buffer, so a burst whose blows pile a tenth on
        // top of one another would have each of them a tenth quieter than a
        // solo. Three has to sound like three *of the same thing*.
        peak: if rows.len() > 1 { loud + 0.02 } else { loud },
        room: Where::Here,
    }
}

/// Every buffer a counted cue needs, in count order, each with its variants.
///
/// The shape the sink stores: `takes[n - 1]` is the list of recipes for a
/// burst of `n`, and a list with more than one in it is cycled.
fn burst(cue: Cue, count: u8) -> Vec<Recipe> {
    let rows: Vec<usize> = (0..count as usize).collect();
    let of = |rows: &[usize]| match cue {
        Cue::CreatureGrew => counters(true, rows),
        Cue::CreatureShrank => counters(false, rows),
        _ => draw(rows),
    };
    if count == 1 {
        SOLOS.iter().map(|&row| of(&[row])).collect()
    } else {
        vec![of(&rows)]
    }
}

/// Every cue but [`Cue::YourMove`], which is six buffers and is built by
/// [`tap`].
///
/// The skeleton is the one the first draft chose and it survives: **life cues
/// speak in thirds and endings in perfect intervals**. A third has a colour —
/// minor for a loss, major for a gain — and that colour is the message; a
/// fifth, a fourth and a unison have none, which is what keeps an ending from
/// being an opinion about the game. All three still come to rest on A.
///
/// What did not survive is [`Cue::GameWon`]'s third strike. D4–F#4–A4 struck in order
/// is a root-position major triad, which is a fanfare however quietly it is
/// played, and "it rests on A so the cadence is open" does not hold once the
/// D has been struck first — the ear has heard the whole chord. The length it
/// was buying comes from the compound decay, the beat between the paired
/// fundamentals and the room instead.
const RECIPES: [(Cue, Recipe); 8] = [
    (
        Cue::MyLifeLost,
        Recipe {
            // 110 ms rather than 95: the first fundamental gets about thirty
            // cycles to be a note before the next knock lands on top of it.
            strikes: &[
                blow(0.0, note::D4, 0.55, mallet::HARD, 0.15, 1.0),
                blow(0.110, note::B3, 0.55, mallet::HARD, 0.30, 1.0),
            ],
            thuds: &[],
            len: 1.8,
            peak: 1.00,
            room: Where::Here,
        },
    ),
    (
        Cue::MyLifeGained,
        Recipe {
            strikes: &[
                blow(0.0, note::D4, 0.55, mallet::HARD, 0.15, 1.0),
                blow(0.100, note::FS4, 0.55, mallet::HARD, 0.30, 1.0),
            ],
            thuds: &[],
            len: 1.7,
            peak: 0.80,
            room: Where::Here,
        },
    ),
    (
        Cue::TheirLifeLost,
        Recipe {
            // The **same mallet** as my own life, which is the correction:
            // distance is a room and a level, never a gentler blow. A softer
            // stick says somebody was hit more gently, which is not what a
            // seat across the table means.
            strikes: &[
                blow(0.0, note::A3, 0.55, mallet::HARD, 0.15, 1.0),
                blow(0.110, note::FS3, 0.55, mallet::HARD, 0.30, 1.0),
            ],
            thuds: &[],
            len: 2.0,
            peak: 0.40,
            room: Where::Elsewhere,
        },
    ),
    (
        Cue::TheirLifeGained,
        Recipe {
            strikes: &[
                blow(0.0, note::A3, 0.55, mallet::HARD, 0.15, 1.0),
                blow(0.100, note::CS4, 0.55, mallet::HARD, 0.30, 1.0),
            ],
            thuds: &[],
            len: 1.9,
            peak: 0.32,
            room: Where::Elsewhere,
        },
    ),
    (
        Cue::Refused,
        Recipe {
            strikes: &[],
            thuds: &[(0.0, 1.0), (0.055, 0.80)],
            len: 0.9,
            peak: 0.50,
            room: Where::Here,
        },
    ),
    (
        Cue::GameWon,
        Recipe {
            strikes: &[
                blow(0.0, note::D4, 1.0, mallet::HARD, 0.15, 1.0),
                blow(0.380, note::A4, 1.0, mallet::HARD, 0.30, 1.0),
            ],
            thuds: &[],
            len: 3.2,
            peak: 0.90,
            room: Where::Ending,
        },
    ),
    (
        Cue::GameLost,
        Recipe {
            strikes: &[
                blow(0.0, note::D4, 1.0, mallet::SOFT, 0.15, 1.0),
                blow(0.460, note::A3, 1.0, mallet::SOFT, 0.30, 0.85),
            ],
            thuds: &[],
            len: 4.4,
            peak: 0.80,
            room: Where::Ending,
        },
    ),
    (
        Cue::GameDrawn,
        Recipe {
            // A unison, which is the whole point — and two *different* blows
            // at it, because the same pitch struck twice at the same spot is
            // one buffer played twice and the ear catches that in one
            // hearing.
            strikes: &[
                blow(0.0, note::A3, 1.0, mallet::SOFT, 0.15, 1.0),
                blow(0.420, note::A3, 1.0, mallet::SOFT, 0.35, 0.92),
            ],
            thuds: &[],
            len: 4.2,
            peak: 0.72,
            room: Where::Ending,
        },
    ),
];

/// [`Cue::YourMove`]'s six, as (cents off [`note::D3`], spot).
///
/// Detuned within ±16 cents **and** struck in six different places. The
/// detune alone was the first draft's answer and it is the weaker half: 16
/// cents on a yarn-struck 147 Hz bar is barely there — a real bar drifts that
/// much with the room and it never reads as a wrong note — while moving the
/// spot swings the 4× partial from 0.99 to 0.59 and brings an antisymmetric
/// mode up out of silence. Written in cents rather than in hertz because a
/// detune is a ratio, and six hand-typed frequencies hide whether any of them
/// has drifted out of the ±16 the doc promises.
///
/// Unsorted in both columns, so consecutive firings are not a ramp.
const VARIANTS: [(f32, f32); 6] = [
    (0.0, 0.10),
    (11.0, 0.45),
    (-7.0, 0.25),
    (16.0, 0.60),
    (-14.0, 0.35),
    (5.0, 0.15),
];

/// What a detune of `cents` does to the lowest bar.
fn detuned(cents: f32) -> f32 {
    note::D3 * (cents / 1200.0).exp2()
}

/// One of [`Cue::YourMove`]'s six.
///
/// A single yarn touch on the lowest bar, stopped at 0.60 — 290 ms of
/// fundamental with the tube's 54 ms bloom under it. It is *not* the shortest
/// cue and that is deliberate: a soft low note with a tail is felt and then
/// forgotten, where a short one is a click, and a click two hundred times is
/// the metronome this is trying not to be.
fn tap(hz: f32, spot: f32) -> Recipe {
    Recipe {
        strikes: Box::leak(Box::new([blow(0.0, hz, 0.60, mallet::YARN, spot, 1.0)])),
        thuds: &[],
        len: 2.4,
        peak: 0.22,
        room: Where::Here,
    }
}

// ---------------------------------------------------------------- rendering

/// A xorshift32, seeded the same way every time.
///
/// The only stochastic thing in the module, and it is consumed entirely by
/// [`grit`] — before the stereo split, so both channels hear the same grit
/// and the two ears never disagree about what the stick was made of. No
/// clock, no `rand`, no entropy: the same table sounds the same on every
/// machine, which is the rule every generated surface in this client obeys.
struct Noise(u32);

impl Noise {
    const SEED: u32 = 0x5EED_BA11;

    fn new() -> Self {
        Self(Self::SEED)
    }

    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32).mul_add(2.0, -1.0)
    }
}

/// One cue, as interleaved stereo samples.
///
/// # Stereo, and what it is allowed to claim
///
/// The direct sound is **identical in both ears**; only the room differs.
/// That is not timidity, it is the only honest option: a [`Cue`] carries no
/// seat, `Their…` is deduplicated across every seat that lost life on one
/// view, and an opponent across the table is in front of the player anyway —
/// so any pan would be a position the game cannot back up. Two *different*
/// reflection patterns with the same dry signal is heard as space rather than
/// as direction, and it is the whole of what stops a computed buffer sounding
/// like a sample. `the_two_ears_agree_about_where_the_sound_is` bounds the
/// difference, so a phone summing to mono loses a little reverb and cancels
/// nothing.
fn render(recipe: &Recipe) -> Vec<f32> {
    let n = (recipe.len * RATE) as usize;
    let mut dry = vec![0.0_f32; n];
    let mut noise = Noise::new();

    for s in recipe.strikes {
        strike(&mut dry, s, &mut noise);
    }
    for &(at, gain) in recipe.thuds {
        thud(&mut dry, at, gain, &mut noise);
    }

    let (dry_mix, er_mix, late_mix) = recipe.room.mix();
    let gap = recipe.room.gap();

    // Distance dulls. A one-pole at 4.5 kHz is air over a few metres, and it
    // is applied to the *direct* sound only — the room has its own filters.
    let direct: Vec<f32> = if recipe.room == Where::Elsewhere {
        let a = pole(4500.0);
        let mut lp = 0.0;
        dry.iter()
            .map(|&x| {
                lp = a.mul_add(lp - x, x);
                lp
            })
            .collect()
    } else {
        dry.clone()
    };

    let reflections = early(&dry, gap, 4500.0);
    let late_l = late(&dry, &LATE_L, gap);
    let late_r = late(&dry, &LATE_R, gap);

    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        let near = dry_mix * direct[i] + er_mix * reflections[i];
        out.push(near + late_mix * late_l[i]);
        out.push(near + late_mix * late_r[i]);
    }
    level(&mut out, recipe.peak);
    out
}

/// Scales a finished buffer to `peak` and takes the last six milliseconds
/// down to nothing.
///
/// The fade is not a taste decision: a buffer that stops while it is still
/// moving is a step, and a step is a click at every frequency at once. Six
/// milliseconds of raised cosine is below the ear's resolution for a change
/// of level and far above it for a discontinuity.
///
/// The peak is taken over **both** channels together, so the stereo image is
/// not tilted by whichever ear happened to be louder.
fn level(buf: &mut [f32], peak: f32) {
    let found = buf.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    if found > 0.0 {
        let k = peak * MASTER / found;
        for s in buf.iter_mut() {
            *s *= k;
        }
    }
    let frames = buf.len() / 2;
    let fade = ((0.006 * RATE) as usize).min(frames);
    for i in 0..fade {
        let x = i as f32 / fade as f32;
        let g = f32::midpoint(1.0, (std::f32::consts::PI * x).cos());
        let f = frames - fade + i;
        buf[f * 2] *= g;
        buf[f * 2 + 1] *= g;
    }
}

/// A 44-byte RIFF header and 16-bit **stereo** samples behind it.
///
/// `bevy_audio`'s `AudioSource` holds *encoded* bytes that rodio decodes, so
/// there is no door in it that takes samples — which is why this function
/// exists and why `wav` is in the workspace's bevy feature list. Uncompressed
/// on purpose: nothing here is stored or transmitted, so a codec could only
/// take quality away. (`bevy_audio` decodes wav, flac, vorbis, mp3 and aac;
/// rodio's decoders have no Opus at all, so that question has an answer and
/// it is no.)
fn wav(samples: &[f32]) -> Vec<u8> {
    const CHANNELS: u16 = 2;
    let data = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&u32::try_from(36 + data).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    out.extend_from_slice(&CHANNELS.to_le_bytes());
    out.extend_from_slice(&(RATE as u32).to_le_bytes());
    out.extend_from_slice(&(RATE as u32 * u32::from(CHANNELS) * 2).to_le_bytes());
    out.extend_from_slice(&(CHANNELS * 2).to_le_bytes()); // block align
    out.extend_from_slice(&16_u16.to_le_bytes()); // bits
    out.extend_from_slice(b"data");
    out.extend_from_slice(&u32::try_from(data).unwrap_or(u32::MAX).to_le_bytes());
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Wraps finished bytes in the asset `bevy_audio` plays.
///
/// `AudioSource` is a struct with a public `Arc<[u8]>` of *encoded* file
/// bytes and no constructor, which is the whole reason [`wav`] exists: there
/// is no door in it that takes samples.
fn source(bytes: Vec<u8>) -> AudioSource {
    AudioSource {
        bytes: bytes.into(),
    }
}

/// A quiet gear train with a low resonant catch, synthesised once per client.
/// Stereo PCM shares the ordinary sound preference and owns no shipped asset.
pub(crate) fn compass_voice() -> AudioSource {
    let frames = (RATE * 1.25) as usize;
    let mut samples = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / RATE;
        let mut sample = 0.0;
        for j in 0..9 {
            let at = j as f32 * 0.105;
            let age = t - at;
            if age >= 0.0 {
                sample += (age * 1800.0).sin() * (-age * 95.0).exp() * 0.045;
                sample += (age * 460.0).sin() * (-age * 30.0).exp() * 0.025;
            }
        }
        let catch = (t - 1.02).max(0.0);
        sample += (catch * 610.0).sin() * (-catch * 28.0).exp() * 0.11;
        samples.extend_from_slice(&[sample, sample]);
    }
    source(wav(&samples))
}

/// The synthesised table, one entry per cue.
///
/// A list of pairs rather than an array indexed by the enum, because a
/// `usize` for a `Cue` is a second spelling of the same thing and the two
/// would drift. Nine comparisons of a `Copy` discriminant, once per sound, is
/// not a cost worth a second spelling.
#[derive(Resource, Default)]
pub struct Voices {
    /// Each cue *at each count* and the one or more buffers it may be played
    /// from.
    ///
    /// Keyed by the whole [`Beat`] rather than by the cue, because a burst of
    /// three is a different buffer and not a louder one. Nine of the twelve
    /// cues have exactly one entry, at count 1.
    voices: Vec<(Beat, Vec<Handle<AudioSource>>)>,
    /// How many sounds have been asked for, which is what picks a variant.
    played: usize,
}

impl Voices {
    /// How many cue-and-count pairs have a voice.
    ///
    /// Twenty-six, or the client is silent somewhere: nine cues that count to
    /// one, a draw that counts to seven and two counter cues that count to
    /// five. `every_cue_has_a_voice` builds the same number out of
    /// [`Cue::counts`] rather than writing it down twice.
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

    /// The next buffer for a beat, or `None` if it has no voice.
    fn pick(&self, beat: Beat) -> Option<&Handle<AudioSource>> {
        let takes = &self.voices.iter().find(|(which, _)| *which == beat)?.1;
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
    commands.insert_resource(crate::compass::CompassVoice(sources.add(compass_voice())));
    let mut voices = Vec::with_capacity(Cue::ALL.len());
    for (cue, recipe) in &RECIPES {
        let bytes = wav(&render(recipe));
        voices.push((Beat::once(*cue), vec![sources.add(source(bytes))]));
    }
    let taps = VARIANTS
        .iter()
        .map(|&(cents, spot)| sources.add(source(wav(&render(&tap(detuned(cents), spot))))))
        .collect();
    voices.push((Beat::once(Cue::YourMove), taps));
    // The counted three, one buffer per count. Built from `Cue::counts` and
    // not from a number written down here, so a ceiling that moves in the
    // model moves the buffers with it rather than leaving the top of a burst
    // silent.
    for cue in Cue::ALL.into_iter().filter(|cue| cue.most() > 1) {
        for count in cue.counts() {
            let takes = burst(cue, count)
                .iter()
                .map(|recipe| sources.add(source(wav(&render(recipe)))))
                .collect();
            voices.push((Beat::of(cue, count), takes));
        }
    }
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
    for beat in audible(&duel.cues.take()) {
        if let Some(voices) = voices.as_mut() {
            sound(&mut commands, voices, beat, level);
        }
    }
}

/// What a cue's finished buffer peaks at, after [`MASTER`].
///
/// The one number [`audible`] needs, and it is read out of the recipe rather
/// than tabulated a second time — a table of levels beside the levels is a
/// table that goes stale on the first tuning pass.
fn peak_of(beat: Beat) -> f32 {
    if let Some((_, recipe)) = RECIPES.iter().find(|(cue, _)| *cue == beat.cue) {
        return recipe.peak * MASTER;
    }
    match beat.cue {
        Cue::YourMove => tap(note::D3, VARIANTS[0].1).peak * MASTER,
        cue => {
            burst(cue, beat.count)
                .first()
                .map_or(0.0, |recipe| recipe.peak)
                * MASTER
        }
    }
}

/// How loud a cue is allowed to be when it is competing for one frame.
///
/// Low is loud. It is a *ranking* and never an amplitude — the amplitudes are
/// the recipes' — and what it decides is which cue is dropped when a frame
/// carries more than the loudspeaker does. The order is the order a player
/// would want it in: a refusal first, because it is the one cue that answers
/// something the player themselves just did and a silent refusal reads as a
/// dead button; then life, which is the number a player must not miss; then
/// the table's texture, shrink before grow because losing is the one to
/// notice; then the draw; then the nudge.
///
/// Endings are not in it, because an ending is not ranked — it is alone.
fn rank(cue: Cue) -> u8 {
    match cue {
        Cue::Refused => 0,
        Cue::MyLifeLost | Cue::MyLifeGained => 1,
        Cue::TheirLifeLost | Cue::TheirLifeGained => 2,
        Cue::CreatureShrank => 3,
        Cue::CreatureGrew => 4,
        Cue::CardDrawn => 5,
        _ => 6,
    }
}

/// Which of one frame's beats are actually played.
///
/// Two rules, and the second of them is a **bend** of what [`MASTER`]'s doc
/// says. The ending rule is unchanged: the frame a game ends on carries the
/// lethal hit as well — a life loss here, a life loss there, and the ending —
/// and 0.70 + 0.28 + 0.63 clips at the loudest moment of the game, so an
/// ending is played **alone**.
///
/// What has changed is everything else. `MASTER` was chosen when two was the
/// worst case an ordinary frame could hold, and it is not any more: a *Sign in
/// Blood* is a life loss and a draw, an infect combat is life and a shrink, a
/// *Fathom Mage* is a counter and a draw, and any of them can land beside
/// [`Cue::YourMove`]. So the frame is filled **greedily by [`rank`]** while
/// the sum of the peaks still fits under one, and what does not fit is
/// dropped.
///
/// Dropped and never ducked, which is the same choice `Cues` makes
/// everywhere: this module has no mixer and wants none — a sound either
/// happens or it does not. And the beats are still **drained** by the caller
/// either way, so nothing here changes what `/state` reports or what
/// [`Cue::YourMove`]'s variant counter is on. Silencing and deciding stay two
/// questions.
fn audible(beats: &[Beat]) -> Vec<Beat> {
    if let Some(&ending) = beats.iter().find(|beat| beat.cue.ends_the_game()) {
        return vec![ending];
    }
    let mut wanted = beats.to_vec();
    wanted.sort_by_key(|beat| rank(beat.cue));
    let mut room = 1.0;
    let mut kept: Vec<Beat> = Vec::with_capacity(wanted.len());
    for beat in wanted {
        let loud = peak_of(beat);
        if loud <= room {
            room -= loud;
            kept.push(beat);
        }
    }
    // Back into the order the frame decided them in, so `/state` and the ear
    // agree about what happened first.
    kept.sort_by_key(|beat| beats.iter().position(|b| b == beat).unwrap_or_default());
    kept
}

/// Plays one cue, if the player wants to hear anything.
///
/// The counter moves whether or not a sound comes out, so turning the volume
/// down and back up does not put [`Cue::YourMove`] back on the variant it was
/// on. The point of the cycle is that consecutive *firings* differ, not
/// consecutive audible ones.
fn sound(commands: &mut Commands, voices: &mut Voices, beat: Beat, level: Loudness) {
    let handle = voices.pick(beat).cloned();
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
            [Beat::once(Cue::GameLost)]
        );
        app.update();
        let duel = app.world().resource::<Duel>();
        assert!(duel.cues.pending().is_empty(), "the queue was drained");
        assert_eq!(
            duel.cues.last(),
            Some(Beat::once(Cue::GameLost)),
            "and remembered"
        );
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
        assert_eq!(duel.cues.last(), Some(Beat::once(Cue::Refused)));
    }

    /// Every cue is synthesised, **at every count it can carry**.
    ///
    /// Run against the real startup system in a real asset world, because
    /// what can go wrong here is a missing *arm* rather than wrong
    /// arithmetic: [`RECIPES`] is eight entries and [`Cue::ALL`] is twelve,
    /// three of which are several buffers each. A count with no buffer is a
    /// draw of four that plays nothing at all — silent at runtime and loud in
    /// no test.
    #[test]
    fn every_cue_has_a_voice() {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<AudioSource>()
            .add_systems(Startup, voice_the_cues);
        app.update();
        let voices = app.world().resource::<Voices>();
        let wanted: usize = Cue::ALL.iter().map(|cue| cue.counts().count()).sum();
        assert_eq!(voices.len(), wanted, "a cue with no sound");
        for cue in Cue::ALL {
            for count in cue.counts() {
                let beat = Beat::of(cue, count);
                assert!(
                    voices.pick(beat).is_some(),
                    "{} has no voice at {count}",
                    cue.name()
                );
            }
        }
    }

    /// A count past a cue's ceiling is played as the ceiling, not as silence.
    ///
    /// [`Beat::of`] clamps, so this is really a test that nothing between the
    /// model and the sink goes round it — a `Beat` built by hand with a twelve
    /// in it would find no buffer and drop the sound at exactly the moment
    /// there was most to hear.
    #[test]
    fn drawing_a_whole_deck_sounds_like_a_handful() {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<AudioSource>()
            .add_systems(Startup, voice_the_cues);
        app.update();
        let voices = app.world().resource::<Voices>();
        let most = Beat::of(Cue::CardDrawn, Cue::CardDrawn.most());
        assert_eq!(Beat::of(Cue::CardDrawn, 60), most, "a Windfall is a hand");
        assert!(voices.pick(most).is_some());
    }

    /// A burst is one blow per thing that happened, and the run accelerates.
    ///
    /// The arithmetic nothing else can see: [`burst`] builds the strikes and
    /// [`render`] turns them into samples, so a ladder indexed wrongly or a
    /// gap that failed to accumulate would come out as a buffer that still
    /// peaks correctly, still renders twice the same, and is simply the wrong
    /// sound.
    #[test]
    fn a_burst_is_one_blow_per_thing_that_happened() {
        for cue in Cue::ALL.iter().filter(|cue| cue.most() > 1) {
            let per = usize::from(*cue != Cue::CardDrawn) + 1;
            for count in cue.counts() {
                for recipe in burst(*cue, count) {
                    assert_eq!(
                        recipe.strikes.len(),
                        per * count as usize,
                        "{} at {count} is the wrong number of blows",
                        cue.name()
                    );
                }
            }
        }
        let three = &burst(Cue::CardDrawn, 3)[0];
        let (first, second) = (
            three.strikes[1].at - three.strikes[0].at,
            three.strikes[2].at - three.strikes[1].at,
        );
        assert!(first > second, "the burst does not accelerate");
        assert!(second > 0.080, "the last gap fuses at {second}");
    }

    /// A counter gesture's two blows are one event, and the second says which
    /// way.
    ///
    /// Both halves matter and both are easy to get backwards. Inside the
    /// ~50 ms grouping window the pair is heard as one thing, which is what
    /// makes three creatures three and not six; and the *destination* is the
    /// louder blow, because that is the note carrying the direction.
    #[test]
    fn a_counter_is_one_gesture_that_lands_somewhere() {
        for (cue, up) in [(Cue::CreatureGrew, true), (Cue::CreatureShrank, false)] {
            let solo = &burst(cue, 1)[0];
            let (first, second) = (&solo.strikes[0], &solo.strikes[1]);
            let apart = second.at - first.at;
            assert!(
                (0.020..=0.050).contains(&apart),
                "{} is {apart}s apart: one event needs under 50 ms and a \
                 direction needs over 20",
                cue.name()
            );
            assert!(second.gain > first.gain, "the grace note is the loud one");
            assert_eq!(second.hz > first.hz, up, "{} lands wrong", cue.name());
        }
    }

    /// The room does not invert a counter gesture.
    ///
    /// This module has been bitten twice by a delay network having a *pitch* —
    /// a comb notch that turned a falling third written 0.80 then 1.00 into
    /// 0.92 then 0.70 — and both times at a pitch the tap set had not been
    /// searched over. [`note::CS5`] and [`note::D5`] are new and were not in
    /// that search, so the dynamic written in [`counters`] is checked *after*
    /// the room rather than trusted before it: the destination has to still be
    /// the louder blow once the reflections are on it.
    #[test]
    fn the_room_leaves_the_gesture_pointing_where_it_was_aimed() {
        for cue in [Cue::CreatureGrew, Cue::CreatureShrank] {
            let solo = &burst(cue, 1)[0];
            let samples = render(solo);
            let loudest = |from: f32, to: f32| {
                let a = ((from * RATE) as usize * 2).min(samples.len());
                let b = ((to * RATE) as usize * 2).min(samples.len());
                samples[a..b].iter().fold(0.0_f32, |m, s| m.max(s.abs()))
            };
            let grace = loudest(0.0, FLAM);
            let lands = loudest(FLAM, FLAM + 0.060);
            assert!(
                lands > grace,
                "{}: the room made the grace note ({grace:.3}) the loud one \
                 against the destination ({lands:.3})",
                cue.name()
            );
        }
    }

    /// Thirty-seven buffers, thirty-seven different sounds.
    ///
    /// A recipe copied and not edited is the easy mistake in a table this
    /// shape, and it is invisible: two cues that sound alike are a client
    /// saying two different things the same way. Compared as bytes, because
    /// the whole point of computing them is that they are reproducible.
    #[test]
    fn no_two_cues_sound_alike() {
        let mut rendered: Vec<Vec<u8>> = all_the_sounds()
            .iter()
            .map(|(_, recipe)| wav(&render(recipe)))
            .collect();
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
            .map(|&(cents, spot)| wav(&render(&tap(detuned(cents), spot))))
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
        for (cue, recipe) in all_the_sounds() {
            assert_eq!(
                wav(&render(&recipe)),
                wav(&render(&recipe)),
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
        for (cue, recipe) in all_the_sounds() {
            let samples = render(&recipe);
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
        let together = once(Cue::MyLifeLost) + once(Cue::TheirLifeLost);
        assert!(together < 1.0, "two cues sum to {together}");
    }

    /// One of a cue, at what it is normalised to after [`MASTER`].
    fn once(cue: Cue) -> f32 {
        peak_of(Beat::once(cue))
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
        let lethal = [Cue::MyLifeLost, Cue::TheirLifeLost, Cue::GameWon].map(Beat::once);
        let raw: f32 = lethal.iter().map(|&beat| peak_of(beat)).sum();
        assert!(raw > 1.0, "the three would not have clipped anyway");
        let kept = audible(&lethal);
        assert_eq!(
            kept,
            vec![Beat::once(Cue::GameWon)],
            "the ending is not alone"
        );
        let together: f32 = kept.iter().map(|&beat| peak_of(beat)).sum();
        assert!(together < 1.0, "the ending clips at {together}");
    }

    /// And an ordinary frame is left exactly as it was.
    #[test]
    fn a_frame_with_no_ending_keeps_every_cue() {
        let pair = [Cue::MyLifeLost, Cue::TheirLifeLost].map(Beat::once);
        assert_eq!(audible(&pair), pair.to_vec());
    }

    /// Every cue's peak is where the module says it is.
    ///
    /// [`peak_of`] is read by [`audible`] and has three branches — the recipe
    /// table, the tap, and the bursts — because the three are stored three
    /// different ways. A branch that answered zero would make its cue free,
    /// so the frame would admit it *and* everything under it, and the clip
    /// would arrive at the loudest moment rather than the quietest.
    #[test]
    fn every_cue_can_say_how_loud_it_is() {
        for cue in Cue::ALL {
            for count in cue.counts() {
                let loud = peak_of(Beat::of(cue, count));
                assert!(
                    loud > 0.1 && loud <= 1.0,
                    "{} at {count} claims {loud}",
                    cue.name()
                );
            }
        }
    }

    /// The frames a real game makes, and which of them still fit.
    ///
    /// The bend [`audible`] documents, held as numbers. A *Sign in Blood* is
    /// a life loss and a draw; an infect combat is life and a shrink; a
    /// *Fathom Mage* is a counter and a draw — none of those existed when
    /// [`MASTER`] was chosen against exactly two cues. Each is checked to
    /// **fit**, because a budget that dropped them would be a quieter client
    /// than the owner asked for; and the lethal combat frame is checked to be
    /// **cut**, because that is what a budget is for.
    #[test]
    fn a_frame_admits_what_it_can_hold_and_drops_the_rest() {
        let fits = |cues: &[Cue]| {
            let frame: Vec<Beat> = cues.iter().map(|&cue| Beat::once(cue)).collect();
            audible(&frame).len() == frame.len()
        };
        assert!(fits(&[Cue::MyLifeLost, Cue::CardDrawn]), "Sign in Blood");
        assert!(fits(&[Cue::MyLifeLost, Cue::CreatureShrank]), "infect");
        assert!(fits(&[Cue::CreatureGrew, Cue::CardDrawn]), "Fathom Mage");
        assert!(fits(&[Cue::CardDrawn, Cue::YourMove]), "an ordinary turn");

        // A whole multiplayer combat: both life cues, a shrink, a draw and
        // the nudge. 0.70 + 0.28 + 0.25 + 0.15 + 0.15 is 1.53.
        let combat = [
            Cue::MyLifeLost,
            Cue::TheirLifeLost,
            Cue::CreatureShrank,
            Cue::CardDrawn,
            Cue::YourMove,
        ]
        .map(Beat::once);
        let raw: f32 = combat.iter().map(|&beat| peak_of(beat)).sum();
        assert!(raw > 1.0, "the five would not have clipped anyway");
        let kept = audible(&combat);
        let together: f32 = kept.iter().map(|&beat| peak_of(beat)).sum();
        assert!(together <= 1.0, "the frame still clips at {together}");
        assert!(!kept.is_empty(), "a busy frame went silent");
        // The ranking, not merely the sum: what survives a full frame is the
        // life a player must not miss, never the texture under it.
        assert!(
            kept.contains(&Beat::once(Cue::MyLifeLost)),
            "the loudest thing that happened was dropped"
        );
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
            2,
            "not stereo, so the two ears would be read as one at double speed"
        );
        // The three fields that have to agree with the channel count, and the
        // reason this test names all of them: a stereo file with a mono block
        // align decodes at half speed with the ears interleaved, which is a
        // failure that *plays* — the worst kind to leave to the ear.
        assert_eq!(
            u16::from_le_bytes([bytes[32], bytes[33]]),
            4,
            "block align is not two channels of sixteen bits"
        );
        assert_eq!(
            u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
            44_100 * 4,
            "byte rate does not match the block align"
        );
        assert_eq!(samples.len() % 2, 0, "an odd sample count is half a frame");
    }

    /// Both ears hear the same sound, and only the room differs.
    ///
    /// The design's one rule about stereo, held as a number. A [`Cue`] names
    /// a moment and never a seat — `Their…` is deduplicated across every seat
    /// that lost life on one view — so a pan would be a position the game
    /// cannot back up. What is decorrelated is the *room*: two different sets
    /// of reflections behind one dry signal, which the ear reads as space
    /// rather than as direction.
    ///
    /// What is bounded is **mono compatibility**, in two numbers, and what is
    /// deliberately *not* bounded is how much the two channels differ.
    ///
    /// A side-to-mid ratio would be the obvious check and it is the wrong
    /// one: a long reverberant tail carries far more energy than a stopped
    /// note, so a genuinely distant cue is more than half side energy while
    /// still being perfectly centred — `TheirLifeLost` sits at 0.51 and is
    /// right to. What must hold is that the two ears **agree**: a positive
    /// correlation, so nothing is inverted or hard-panned, and a mono sum
    /// that keeps almost all of its peak, so a phone speaker loses reverb and
    /// not the sound.
    ///
    /// The first draft of the room failed both: two different early-reflection
    /// sets measured −0.99 correlated at A3, which is a cue that all but
    /// disappears the moment anyone sums it.
    #[test]
    fn the_two_ears_agree_about_where_the_sound_is() {
        for (cue, recipe) in all_the_sounds() {
            let samples = render(&recipe);
            let (mid, side) = samples
                .as_chunks::<2>()
                .0
                .iter()
                .fold((0.0, 0.0), |(m, s), f| {
                    (m + (f[0] + f[1]).powi(2), s + (f[0] - f[1]).powi(2))
                });
            assert!(side > 0.0, "{} is the same in both ears", cue.name());
            let corr = (mid - side) / (mid + side);
            assert!(
                corr > 0.25,
                "{} has ears that disagree: correlation {corr:.2}",
                cue.name()
            );

            let loudest = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
            let summed = samples
                .as_chunks::<2>()
                .0
                .iter()
                .fold(0.0_f32, |m, f| m.max((f32::midpoint(f[0], f[1])).abs()));
            assert!(
                summed > 0.85 * loudest,
                "{} loses {:.0}% of its peak in mono",
                cue.name(),
                100.0 * (1.0 - summed / loudest)
            );
        }
    }

    /// No mode above the fundamental outlives a tenth of a second.
    ///
    /// The bell guard, and the reason it is a test rather than a comment: the
    /// decay law is `τ₁ · r^-1.2`, so a *longer* ring lengthens every partial
    /// with it, and the endings ring four times as long as a life cue. An
    /// ending whose 9.56 partial rang for 90 ms would be a struck bell, which
    /// is exactly what this module's header says it is not — and the clamp
    /// that stops it is one `.min()` easy to drop while tuning.
    #[test]
    fn nothing_above_the_fundamental_rings_like_a_bell() {
        for (cue, recipe) in all_the_sounds() {
            for blow in recipe.strikes {
                let ring = free_ring(blow.hz) * blow.damp;
                for mode in modes(blow.hz, blow.contact, blow.spot, ring).iter().skip(1) {
                    assert!(
                        mode.tau <= 0.090,
                        "{}: {:.0} Hz rings {:.0} ms",
                        cue.name(),
                        mode.hz,
                        mode.tau * 1000.0
                    );
                }
            }
        }
    }

    /// A buffer ends at silence.
    ///
    /// The fade is what makes this true; without it the tail is wherever the
    /// exponential had reached, and the step to zero is a click — the one
    /// artefact a listener always notices and never blames on the right
    /// thing.
    #[test]
    fn no_cue_ends_on_a_step() {
        for (cue, recipe) in all_the_sounds() {
            let samples = render(&recipe);
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
            seen.push(
                voices
                    .pick(Beat::once(Cue::YourMove))
                    .cloned()
                    .expect("a tap"),
            );
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
        for (i, &(cents, spot)) in VARIANTS.iter().enumerate() {
            let path = format!("{dir}/YourMove-{i}.wav");
            std::fs::write(&path, wav(&render(&tap(detuned(cents), spot)))).expect("written");
        }
        // The counted three, every count and every variant — which is most of
        // what there is to listen *to* now, and all of what the design's own
        // perceptual claims rest on: that seven taps at these gaps can be
        // counted, and that a 45 ms flam reads as a direction rather than as
        // one thickened note. Neither is a thing a test can assert.
        for cue in Cue::ALL.into_iter().filter(|cue| cue.most() > 1) {
            for count in cue.counts() {
                for (i, recipe) in burst(cue, count).iter().enumerate() {
                    let path = format!("{dir}/{}-{count}-{i}.wav", cue.name());
                    std::fs::write(&path, wav(&render(recipe))).expect("written");
                }
            }
        }
    }

    /// Every buffer the client will ever synthesise, with the cue it belongs
    /// to.
    ///
    /// [`RECIPES`] used to be the whole set and is now eight of twenty-six:
    /// six taps and the counted three's bursts are built by functions rather
    /// than written into a table. Every test below that says "every cue" has
    /// to mean this, or the tests that hold the *instrument* together — the
    /// bell guard, the clip bound, the ends-at-silence rule, the two ears —
    /// would all be silently ignoring the newest and most frequent sounds in
    /// the set, which is exactly the shape of a test that passes and proves
    /// nothing.
    fn all_the_sounds() -> Vec<(Cue, Recipe)> {
        let mut out: Vec<(Cue, Recipe)> = Vec::new();
        for (cue, recipe) in RECIPES {
            out.push((cue, recipe));
        }
        for (cents, spot) in VARIANTS {
            out.push((Cue::YourMove, tap(detuned(cents), spot)));
        }
        for cue in Cue::ALL.into_iter().filter(|cue| cue.most() > 1) {
            for count in cue.counts() {
                out.extend(burst(cue, count).into_iter().map(|recipe| (cue, recipe)));
            }
        }
        out
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
