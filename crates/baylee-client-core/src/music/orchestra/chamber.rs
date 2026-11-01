//! The memorial's real grand piano, solo violin and cello in stereo.
//! Long natural recordings, two piano velocity layers and shaped bow phrases;
//! no plucked folk timbres or additive piano patch enter this ensemble.
mod bank;
#[cfg(test)]
mod tests;
use crate::music::RATE;
use bank::{Bank, Clip, bank};

const VOICES: usize = 64;

#[derive(Clone, Copy, Debug)]
pub(in crate::music) struct Note {
    pub instrument: u8,
    pub pitch: u8,
    pub frames: u32,
    pub velocity: u8,
    pub gain: f32,
    pub pan: f32,
}
struct Voice {
    bank: &'static Bank,
    soft: Clip,
    loud: Clip,
    mix: f32,
    age: u32,
    hold: u32,
    release: u32,
    released_at: Option<u32>,
    released_level: f32,
    gain: [f32; 2],
    bowed: bool,
}
impl Voice {
    fn new(note: Note) -> Self {
        let bank = bank();
        let soft = bank
            .clip(note.instrument, note.pitch, 0)
            .expect("score pitch prepared");
        let loud = bank.clip(note.instrument, note.pitch, 1).unwrap_or(soft);
        let bowed = note.instrument != 0;
        let velocity = f32::from(note.velocity) / 80.0;
        let gain = note.gain * velocity.powf(if bowed { 1.0 } else { 1.55 });
        // Balance the recorded stereo image; do not collapse it to mono then pan.
        let pan = note.pan.clamp(-1.0, 1.0);
        Self {
            bank,
            soft,
            loud,
            mix: if bowed {
                0.0
            } else {
                ((f32::from(note.velocity) - 45.0) / 42.0).clamp(0.0, 1.0)
            },
            age: 0,
            hold: note.frames,
            release: if bowed { RATE / 2 } else { RATE * 3 / 4 },
            released_at: None,
            released_level: 1.0,
            gain: [gain * (1.0 - pan.max(0.0)), gain * (1.0 + pan.min(0.0))],
            bowed,
        }
    }
    fn held_level(&self) -> f32 {
        if !self.bowed {
            return (self.age as f32 / 96.0).min(1.0);
        }
        let attack = (self.age as f32 / (RATE as f32 * 0.22)).min(1.0);
        let phase = (self.age as f32 / self.hold.max(1) as f32).min(1.0);
        // A smooth swell follows the bow; recorded vibrato supplies the pitch motion.
        attack * (0.72 + 1.12 * phase * (1.0 - phase))
    }
    fn release(&mut self, abrupt: bool) {
        if self.released_at.is_none() {
            self.released_level = self.held_level();
            self.released_at = Some(self.age);
            if abrupt {
                self.release = RATE / 4;
            }
        }
    }
    fn next(&mut self) -> Option<[f32; 2]> {
        if self.age >= self.hold {
            self.release(false);
        }
        let envelope = if let Some(start) = self.released_at {
            let phase = (self.age - start) as f32 / self.release as f32;
            if phase >= 1.0 {
                return None;
            }
            // Smooth at both ends, no hard key-off truncation of a piano tail.
            self.released_level * (1.0 - phase * phase * (3.0 - 2.0 * phase))
        } else {
            self.held_level()
        };
        let age = self.age as usize;
        if age >= self.soft.frames.max(self.loud.frames) {
            return None;
        }
        let a = self.bank.sample(self.soft, age);
        let b = self.bank.sample(self.loud, age);
        self.age += 1;
        Some(std::array::from_fn(|i| {
            (a[i] + (b[i] - a[i]) * self.mix) * envelope * self.gain[i]
        }))
    }
}

pub(super) fn prepare() {
    bank();
}
pub(super) struct Ensemble {
    voices: Vec<Voice>,
}
impl Default for Ensemble {
    fn default() -> Self {
        prepare();
        Self {
            voices: Vec::with_capacity(VOICES),
        }
    }
}
impl Ensemble {
    pub(super) fn note(&mut self, note: Note) {
        if self.voices.len() < VOICES {
            self.voices.push(Voice::new(note));
        }
    }
    pub(super) fn release(&mut self) {
        for voice in &mut self.voices {
            voice.release(true);
        }
    }
    pub(super) fn frame(&mut self, dry: &mut [f32; 2], feed: &mut f32) {
        self.voices.retain_mut(|voice| {
            let Some(note) = voice.next() else {
                return false;
            };
            for i in 0..2 {
                dry[i] += note[i];
            }
            *feed += (note[0] + note[1]) * 0.11;
            true
        });
    }
    pub(super) fn render(&mut self, dry: &mut [[f32; 2]], feed: &mut [f32]) {
        self.voices.retain_mut(|voice| {
            for (frame, send) in dry.iter_mut().zip(feed.iter_mut()) {
                let Some(note) = voice.next() else {
                    return false;
                };
                for i in 0..2 {
                    frame[i] += note[i];
                }
                *send += (note[0] + note[1]) * 0.11;
            }
            true
        });
    }
}
