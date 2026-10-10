//! CC0 recordings, prepared once before the audio stream starts.
//! Studio voices read pitch-corrected 48-kHz buffers directly. Original voices
//! read the original 44.1-kHz PCM with cubic interpolation. No callback cache miss
//! allocates: pitches outside the composed score use the original PCM reader.
use super::{Instrument, RATE, Touch};
use crate::music::{
    SampleSet, bank,
    score::{Movement, Theme, arrangement},
};
use std::sync::OnceLock;

pub(in crate::music) const SOURCE_RATE: u32 = 44_100;
const LOW: u8 = 28;
const PITCHES: usize = 61;
const FAMILIES: usize = 11;
const TAPS: usize = 32;
const PHASES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::music) enum Kind {
    Sus,
    Drone,
    Pluck,
    Bell,
    Drum,
}
/// ABI retained for the historical, generated bank. Metadata also serves audits.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(in crate::music) struct Def {
    pub name: &'static str,
    pub pcm: &'static [u8],
    pub midi: u8,
    pub cents: f32,
    pub looped: Option<(u32, u32)>,
    pub kind: Kind,
    pub attack: f32,
    pub release: f32,
    pub room_send: f32,
}
#[derive(Clone, Copy, Debug)]
pub(in crate::music) struct Family {
    pub first: usize,
    pub len: usize,
}
// Select at compile time: unused historical drums, organs, pipes and winds
// never become references in the runtime bank or its binary payload.
const SOURCES: [Family; 10] = [
    bank::HARP,
    bank::PSALTERY,
    bank::VIOLIN,
    bank::VIOLAS,
    bank::CELLO,
    bank::CONTRABASS,
    bank::TROMBONE,
    bank::VIOLINS_SPIC,
    bank::VIOLAS_SPIC,
    bank::CELLOS_SPIC,
];
const ACTIVE: [Def; 51] = {
    let mut selected = [bank::BANK[0]; 51];
    let mut family = 0;
    let mut count = 0;
    while family < SOURCES.len() {
        let source = SOURCES[family];
        let mut note = 0;
        while note < source.len {
            selected[count] = bank::BANK[source.first + note];
            note += 1;
            count += 1;
        }
        family += 1;
    }
    assert!(count == selected.len());
    selected
};
const FAMILIES_BY_SOURCE: [Family; 10] = {
    let mut families = [Family { first: 0, len: 0 }; 10];
    let mut i = 0;
    let mut first = 0;
    while i < families.len() {
        families[i] = Family {
            first,
            len: SOURCES[i].len,
        };
        first += SOURCES[i].len;
        i += 1;
    }
    families
};

impl Family {
    fn nearest(self, pitch: u8) -> usize {
        (self.first..self.first + self.len)
            .min_by_key(|&i| ACTIVE[i].midi.abs_diff(pitch))
            .expect("nonempty family")
    }
}
fn family(instrument: Instrument) -> Family {
    FAMILIES_BY_SOURCE[match instrument {
        Instrument::Harp | Instrument::Lyre => 0,
        Instrument::Zither => 1,
        Instrument::Violin => 2,
        Instrument::Viola => 3,
        Instrument::Cello => 4,
        Instrument::Bass => 5,
        Instrument::Trombone => 6,
        Instrument::ViolinShort => 7,
        Instrument::ViolaShort => 8,
        Instrument::CelloShort => 9,
        _ => unreachable!("synthetic voices have a separate renderer"),
    }]
}

fn key(instrument: Instrument, pitch: u8) -> usize {
    instrument as usize * PITCHES + usize::from(pitch.clamp(LOW, 88) - LOW)
}
fn rate(def: &Def, pitch: u8) -> f64 {
    f64::from(SOURCE_RATE) / f64::from(RATE)
        * 2.0_f64
            .powf((f64::from(pitch) - f64::from(def.midi) - f64::from(def.cents) / 100.0) / 12.0)
}
fn decode(def: &Def, frame: usize) -> f32 {
    let at = frame * 2;
    f32::from(i16::from_le_bytes([def.pcm[at], def.pcm[at + 1]])) / 32768.0
}

struct Clip {
    data: Box<[f32]>,
    looped: Option<(usize, usize)>,
}
fn kernel(step: f64) -> Vec<[f32; TAPS]> {
    let cutoff = 0.94 / step.max(1.0);
    (0..=PHASES)
        .map(|phase| {
            let mut row = [0.0; TAPS];
            for (tap, value) in row.iter_mut().enumerate() {
                let x = tap as f64 - (TAPS / 2 - 1) as f64 - phase as f64 / PHASES as f64;
                let sinc = if x.abs() < 1e-9 {
                    cutoff
                } else {
                    (std::f64::consts::PI * cutoff * x).sin() / (std::f64::consts::PI * x)
                };
                let w = std::f64::consts::PI * x / (TAPS / 2) as f64;
                // Blackman window; normalized below to preserve DC at every phase.
                *value = (sinc * (0.42 + 0.5 * w.cos() + 0.08 * (2.0 * w).cos())) as f32;
            }
            let sum = row.iter().sum::<f32>();
            for value in &mut row {
                *value /= sum;
            }
            row
        })
        .collect()
}
fn prepare_clip(def: &Def, pitch: u8) -> Clip {
    let step = rate(def, pitch);
    let kernels = kernel(step);
    let frames = def
        .looped
        .map_or(def.pcm.len() / 2, |(_, end)| end as usize);
    let mut padded = vec![0.0; frames + TAPS * 2];
    for i in 0..frames {
        padded[TAPS / 2 - 1 + i] = decode(def, i);
    }
    if let Some((start, _)) = def.looped {
        for i in 0..TAPS {
            padded[TAPS / 2 - 1 + frames + i] = decode(def, start as usize + i);
        }
    }
    let length = (frames as f64 / step).floor() as usize;
    let mut data = Vec::with_capacity(length);
    for i in 0..length {
        let position = i as f64 * step;
        let at = position as usize;
        let exact_phase = (position - at as f64) * PHASES as f64;
        let phase = exact_phase as usize;
        let blend = (exact_phase - phase as f64) as f32;
        let samples = &padded[at..at + TAPS];
        data.push(
            samples
                .iter()
                .zip(kernels[phase].iter().zip(kernels[phase + 1].iter()))
                .map(|(sample, (left, right))| sample * (left + (right - left) * blend))
                .sum(),
        );
    }
    let looped = def
        .looped
        .map(|(start, _)| ((f64::from(start) / step).round() as usize, length));
    // Conversion rounds the loop's duration by at most one output frame.
    // Rejoin its two ends over 12 ms to remove that rounding discontinuity.
    if let Some((start, end)) = looped {
        let fade = 576.min((end - start) / 4);
        for i in 0..fade {
            let t = i as f32 / fade as f32;
            let continuation = data[start.saturating_sub(fade) + i];
            data[end - fade + i] = data[end - fade + i] * (1.0 - t) + continuation * t;
        }
    }
    Clip {
        data: data.into_boxed_slice(),
        looped,
    }
}
fn clips() -> &'static [Option<Clip>] {
    static CLIPS: OnceLock<Vec<Option<Clip>>> = OnceLock::new();
    CLIPS.get_or_init(|| {
        let mut used = [false; FAMILIES * PITCHES];
        for theme in Theme::ALL {
            for movement in Movement::ALL {
                for bar in 0..33 {
                    for tick in 0..theme.ticks() {
                        for note in arrangement::notes(theme, movement, bar, tick).as_slice() {
                            if note.instrument != Instrument::Lyre && !note.instrument.synthetic() {
                                used[key(note.instrument, note.pitch)] = true;
                            }
                        }
                    }
                }
            }
        }
        let instruments = [
            Instrument::Harp,
            Instrument::Zither,
            Instrument::Lyre,
            Instrument::Violin,
            Instrument::Viola,
            Instrument::Cello,
            Instrument::Bass,
            Instrument::Trombone,
            Instrument::ViolinShort,
            Instrument::ViolaShort,
            Instrument::CelloShort,
        ];
        used.iter()
            .enumerate()
            .map(|(i, &needed)| {
                needed.then(|| {
                    let pitch = LOW + (i % PITCHES) as u8;
                    prepare_clip(
                        &ACTIVE[family(instruments[i / PITCHES]).nearest(pitch)],
                        pitch,
                    )
                })
            })
            .collect()
    })
}
pub(super) fn prepare() {
    clips();
    originals();
}

/// Decode only the ten used acoustic families, once, outside playback.
fn originals() -> &'static [Box<[f32]>] {
    static ORIGINALS: OnceLock<Vec<Box<[f32]>>> = OnceLock::new();
    ORIGINALS.get_or_init(|| {
        ACTIVE
            .iter()
            .map(|def| {
                (0..def.pcm.len() / 2)
                    .map(|frame| decode(def, frame))
                    .collect::<Vec<_>>()
                    .into_boxed_slice()
            })
            .collect()
    })
}

/// Read cubic PCM across a loop with the same waveform on both sides.
fn sample_at(def: &Def, frame: isize) -> f32 {
    if frame < 0 {
        return 0.0;
    }
    let mut at = frame as usize;
    if let Some((start, end)) = def.looped
        && at >= end as usize
    {
        at = start as usize + (at - end as usize) % (end - start) as usize;
    }
    if at >= def.pcm.len() / 2 {
        0.0
    } else {
        decode(def, at)
    }
}
fn cubic(def: &Def, raw: &[f32], position: f64) -> f32 {
    let at = position as isize;
    let fraction = (position - at as f64) as f32;
    let end = def.looped.map_or(raw.len(), |(_, end)| end as usize);
    let [a, b, c, d] = if at >= 1 && (at as usize + 2) < end {
        let [a, b, c, d] = raw[at as usize - 1..at as usize + 3] else {
            unreachable!("four frames")
        };
        [a, b, c, d]
    } else {
        [-1, 0, 1, 2].map(|offset| sample_at(def, at + offset))
    };
    b + 0.5
        * fraction
        * (c - a
            + fraction * (2.0 * a - 5.0 * b + 4.0 * c - d + fraction * (3.0 * (b - c) + d - a)))
}
enum Reader {
    Studio {
        clip: &'static Clip,
        at: usize,
    },
    Original {
        def: &'static Def,
        raw: &'static [f32],
        at: f64,
        step: f64,
    },
}
impl Reader {
    fn next(&mut self) -> Option<f32> {
        match self {
            Self::Studio { clip, at } => {
                if let Some((start, end)) = clip.looped
                    && *at == end
                {
                    *at = start;
                }
                let value = *clip.data.get(*at)?;
                *at += 1;
                Some(value)
            }
            Self::Original { def, raw, at, step } => {
                if let Some((start, end)) = def.looped {
                    if *at >= f64::from(end) {
                        *at -= f64::from(end - start);
                    }
                } else if *at >= (def.pcm.len() / 2) as f64 {
                    return None;
                }
                let value = cubic(def, raw, *at);
                *at += *step;
                Some(value)
            }
        }
    }
}
pub(super) struct Voice {
    reader: Reader,
    wait: u32,
    hold: u32,
    age: u32,
    attack: f32,
    release: f32,
    envelope: f32,
    released: bool,
    gain: [f32; 2],
    tone: f32,
    low: f32,
}
impl Voice {
    #[cfg(test)]
    pub(super) const fn level(&self) -> f32 {
        self.envelope
    }
    pub(super) fn new(
        instrument: Instrument,
        pitch: u8,
        seconds: f32,
        gain: [f32; 2],
        touch: Touch,
        samples: SampleSet,
    ) -> Self {
        let sample = family(instrument).nearest(pitch);
        let def = &ACTIVE[sample];
        let reader = if samples == SampleSet::Studio48 {
            clips()[key(instrument, pitch)]
                .as_ref()
                .map(|clip| Reader::Studio { clip, at: 0 })
        } else {
            None
        }
        .unwrap_or_else(|| Reader::Original {
            def,
            raw: &originals()[sample],
            at: 0.0,
            step: rate(def, pitch),
        });
        // Keep natural recorded articulation. Short/plucked attacks must not
        // disappear under the long strings' envelope.
        let plucked = matches!(instrument, Instrument::Harp | Instrument::Zither);
        let attack = if plucked || instrument.short() {
            0.003
        } else {
            touch.attack.unwrap_or(0.025).min(0.04)
        };
        let release = touch.release.unwrap_or(def.release).max(0.04);
        Self {
            reader,
            wait: (touch.late.max(0.0) * RATE as f32) as u32,
            hold: (seconds * RATE as f32) as u32,
            age: 0,
            attack: 1.0 / (attack * RATE as f32),
            release: 1.0 / (release * RATE as f32),
            envelope: 0.0,
            released: false,
            gain,
            tone: if instrument == Instrument::Zither {
                0.34
            } else {
                1.0
            },
            low: 0.0,
        }
    }
    pub(super) fn release(&mut self) {
        if !self.released {
            self.released = true;
            self.release = self.envelope / (RATE as f32 * 0.18);
        }
    }
    pub(super) fn next(&mut self) -> Option<[f32; 2]> {
        if self.wait > 0 {
            self.wait -= 1;
            return Some([0.0; 2]);
        }
        if self.age >= self.hold {
            self.released = true;
        }
        if self.released {
            self.envelope = (self.envelope - self.release).max(0.0);
            if self.envelope <= 0.000_001 {
                return None;
            }
        } else {
            self.envelope = (self.envelope + self.attack).min(1.0);
        }
        self.age += 1;
        let input = self.reader.next()?;
        self.low += (input - self.low) * self.tone;
        let x = self.low * self.envelope * self.envelope * (3.0 - 2.0 * self.envelope);
        Some([x * self.gain[0], x * self.gain[1]])
    }
}

#[cfg(test)]
mod tests;
