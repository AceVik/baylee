//! Authentic tavern sheet-music suite for Login (`Movement::Title`) and Lobby
//! (`Movement::Lobby`), performed on Celtic Folk Harp (`Harp`), physical-modelled
//! Renaissance Lute (`Lyre`), Plucked Psaltery (`Zither`), Solo Tavern Fiddle
//! (`Violin`), Warm Viola (`Viola`), and Bowed Cello (`Cello`).
//!
//! Transcribed note-for-note from four public-domain historical scores:
//! 1. *Greensleeves* (Anonymous English Renaissance Lute Ballad, 1580, A Dorian / A minor)
//! 2. *Scarborough Fair* (Traditional English Medieval Ballad, D Dorian)
//! 3. *Si Bheag, Si Mhor* (Turlough O'Carolan, 1691 Celtic Harp Air, D Major)
//! 4. *Kemp's Jig* (John Playford's *The English Dancing Master*, 1651, D Dorian)
use super::{Movement, arrangement::Instrument};
#[cfg(test)]
use crate::music::orchestra::chamber::Note;
use crate::music::{
    RATE, SampleSet,
    orchestra::{Orchestra, Touch},
};

const DATA: &[u8] = include_bytes!("memorial/performance.bin");
#[cfg(test)]
const HEADER: usize = 12;
#[cfg(test)]
const RECORD: usize = 20;

/// Each sixteenth-note subdivision in the tavern sheet-music score lasts 8,400
/// audio frames (0.175 s at 48 kHz -> 2.10 s per 12-subdivision bar).
const STEP_FRAMES: u32 = 8_400;
const STEP_SECONDS: f32 = STEP_FRAMES as f32 / RATE as f32;

pub(super) const fn contains(movement: Movement) -> bool {
    matches!(movement, Movement::Title | Movement::Lobby)
}

#[derive(Clone, Copy)]
struct SheetBar {
    bass1: u8,
    tones1: [u8; 3],
    bass2: u8,
    tones2: [u8; 3],
    melody: &'static [(u8, u8, u8)],
}

const fn bar_same(bass: u8, tones: [u8; 3], melody: &'static [(u8, u8, u8)]) -> SheetBar {
    SheetBar {
        bass1: bass,
        tones1: tones,
        bass2: bass,
        tones2: tones,
        melody,
    }
}

const fn bar_split(
    bass1: u8,
    tones1: [u8; 3],
    bass2: u8,
    tones2: [u8; 3],
    melody: &'static [(u8, u8, u8)],
) -> SheetBar {
    SheetBar {
        bass1,
        tones1,
        bass2,
        tones2,
        melody,
    }
}

/// Piece I: *Greensleeves* (Anonymous, 1580 — A Dorian / A minor, 6/8 time, 16 bars).
const GREENSLEEVES: [SheetBar; 16] = [
    // Bar 1 (Am): C5(4) D5(2) | E5(3) F#5(1) E5(2)
    bar_same(
        45,
        [57, 64, 60],
        &[(0, 72, 4), (4, 74, 2), (6, 76, 3), (9, 78, 1), (10, 76, 2)],
    ),
    // Bar 2 (G): D5(4) B4(2) | G4(3) A4(1) B4(2)
    bar_same(
        43,
        [55, 62, 59],
        &[(0, 74, 4), (4, 71, 2), (6, 67, 3), (9, 69, 1), (10, 71, 2)],
    ),
    // Bar 3 (Am): C5(4) A4(2) | A4(3) G#4(1) A4(2)
    bar_same(
        45,
        [57, 64, 60],
        &[(0, 72, 4), (4, 69, 2), (6, 69, 3), (9, 68, 1), (10, 69, 2)],
    ),
    // Bar 4 (E): B4(4) G#4(2) | E4(4) A4(2)
    bar_same(
        40,
        [52, 59, 56],
        &[(0, 71, 4), (4, 68, 2), (6, 64, 4), (10, 69, 2)],
    ),
    // Bar 5 (Am): C5(4) D5(2) | E5(3) F#5(1) E5(2)
    bar_same(
        45,
        [57, 64, 60],
        &[(0, 72, 4), (4, 74, 2), (6, 76, 3), (9, 78, 1), (10, 76, 2)],
    ),
    // Bar 6 (G): D5(4) B4(2) | G4(3) A4(1) B4(2)
    bar_same(
        43,
        [55, 62, 59],
        &[(0, 74, 4), (4, 71, 2), (6, 67, 3), (9, 69, 1), (10, 71, 2)],
    ),
    // Bar 7 (Am -> E): C5(3) B4(1) A4(2) | G#4(3) F#4(1) G#4(2)
    bar_split(
        45,
        [57, 64, 60],
        40,
        [52, 59, 56],
        &[
            (0, 72, 3),
            (3, 71, 1),
            (4, 69, 2),
            (6, 68, 3),
            (9, 66, 1),
            (10, 68, 2),
        ],
    ),
    // Bar 8 (Am): A4(6) | A4(6)
    bar_same(45, [57, 64, 60], &[(0, 69, 6), (6, 69, 6)]),
    // Bar 9 (C): G5(6) | G5(3) F#5(1) E5(2)
    bar_same(
        48,
        [55, 60, 64],
        &[(0, 79, 6), (6, 79, 3), (9, 78, 1), (10, 76, 2)],
    ),
    // Bar 10 (G): D5(4) B4(2) | G4(3) A4(1) B4(2)
    bar_same(
        43,
        [55, 62, 59],
        &[(0, 74, 4), (4, 71, 2), (6, 67, 3), (9, 69, 1), (10, 71, 2)],
    ),
    // Bar 11 (Am): C5(4) A4(2) | A4(3) G#4(1) A4(2)
    bar_same(
        45,
        [57, 64, 60],
        &[(0, 72, 4), (4, 69, 2), (6, 69, 3), (9, 68, 1), (10, 69, 2)],
    ),
    // Bar 12 (E): B4(4) G#4(2) | E4(6)
    bar_same(40, [52, 59, 56], &[(0, 71, 4), (4, 68, 2), (6, 64, 6)]),
    // Bar 13 (C): G5(6) | G5(3) F#5(1) E5(2)
    bar_same(
        48,
        [55, 60, 64],
        &[(0, 79, 6), (6, 79, 3), (9, 78, 1), (10, 76, 2)],
    ),
    // Bar 14 (G): D5(4) B4(2) | G4(3) A4(1) B4(2)
    bar_same(
        43,
        [55, 62, 59],
        &[(0, 74, 4), (4, 71, 2), (6, 67, 3), (9, 69, 1), (10, 71, 2)],
    ),
    // Bar 15 (Am -> E): C5(3) B4(1) A4(2) | G#4(3) F#4(1) G#4(2)
    bar_split(
        45,
        [57, 64, 60],
        40,
        [52, 59, 56],
        &[
            (0, 72, 3),
            (3, 71, 1),
            (4, 69, 2),
            (6, 68, 3),
            (9, 66, 1),
            (10, 68, 2),
        ],
    ),
    // Bar 16 (Am): A4(8) A4(4)
    bar_same(45, [57, 64, 60], &[(0, 69, 8), (8, 69, 4)]),
];

/// Piece II: *Scarborough Fair* (Traditional English Ballad — D Dorian, 3/4 time, 16 bars).
const SCARBOROUGH_FAIR: [SheetBar; 16] = [
    // "Are you going to Scarborough Fair?"
    bar_same(50, [57, 62, 65], &[(0, 62, 8), (8, 62, 4)]),
    bar_same(50, [57, 62, 65], &[(0, 69, 8), (8, 69, 4)]),
    bar_same(48, [55, 60, 64], &[(0, 64, 6), (6, 65, 2), (8, 64, 4)]),
    bar_same(50, [57, 62, 65], &[(0, 62, 12)]),
    // "Parsley, sage, rosemary and thyme"
    bar_split(
        41,
        [53, 60, 65],
        48,
        [55, 60, 64],
        &[(0, 69, 4), (4, 72, 8)],
    ),
    bar_split(
        50,
        [57, 62, 65],
        48,
        [55, 60, 64],
        &[(0, 74, 8), (8, 72, 4)],
    ),
    bar_split(
        50,
        [57, 62, 65],
        43,
        [55, 59, 62],
        &[(0, 69, 4), (4, 71, 4), (8, 67, 4)],
    ),
    bar_same(45, [57, 64, 60], &[(0, 69, 12)]),
    // "Remember me to one who lives there"
    bar_same(50, [57, 62, 65], &[(0, 74, 8), (8, 74, 4)]),
    bar_same(48, [55, 60, 64], &[(0, 74, 8), (8, 72, 4)]),
    bar_same(43, [55, 59, 62], &[(0, 69, 8), (8, 67, 4)]),
    bar_split(
        50,
        [57, 62, 65],
        48,
        [55, 60, 64],
        &[(0, 65, 4), (4, 64, 4), (8, 62, 4)],
    ),
    // "She once was a true love of mine"
    bar_same(50, [57, 62, 65], &[(0, 62, 8), (8, 69, 4)]),
    bar_split(
        43,
        [55, 59, 62],
        50,
        [57, 62, 65],
        &[(0, 67, 8), (8, 65, 4)],
    ),
    bar_split(
        48,
        [55, 60, 64],
        45,
        [52, 57, 60],
        &[(0, 64, 6), (6, 62, 2), (8, 60, 4)],
    ),
    bar_same(50, [57, 62, 65], &[(0, 62, 12)]),
];

/// Piece III: *Si Bheag, Si Mhor* (Turlough O'Carolan, 1691 — Celtic Harp Air in D Major, 3/4 time, 16 bars).
const SI_BHEAG_SI_MHOR: [SheetBar; 16] = [
    // A-Part
    bar_same(50, [57, 62, 66], &[(0, 78, 6), (6, 76, 2), (8, 74, 4)]),
    bar_same(47, [54, 59, 62], &[(0, 74, 6), (6, 76, 2), (8, 74, 4)]),
    bar_same(43, [55, 59, 62], &[(0, 71, 8), (8, 69, 4)]),
    bar_same(50, [57, 62, 66], &[(0, 66, 8), (8, 69, 4)]),
    bar_same(
        43,
        [55, 59, 62],
        &[(0, 71, 4), (4, 69, 2), (6, 71, 2), (8, 74, 4)],
    ),
    bar_same(45, [57, 61, 64], &[(0, 76, 8), (8, 74, 4)]),
    bar_split(
        43,
        [55, 59, 62],
        45,
        [57, 61, 64],
        &[(0, 71, 4), (4, 67, 4), (8, 69, 4)],
    ),
    bar_same(50, [57, 62, 66], &[(0, 74, 12)]),
    // B-Part
    bar_same(50, [57, 62, 66], &[(0, 78, 6), (6, 76, 2), (8, 74, 4)]),
    bar_same(47, [54, 59, 62], &[(0, 74, 4), (4, 76, 4), (8, 78, 4)]),
    bar_same(43, [55, 59, 62], &[(0, 79, 8), (8, 78, 4)]),
    bar_same(45, [57, 61, 64], &[(0, 76, 8), (8, 69, 4)]),
    bar_same(
        43,
        [55, 59, 62],
        &[(0, 71, 4), (4, 69, 2), (6, 71, 2), (8, 74, 4)],
    ),
    bar_split(
        50,
        [57, 62, 66],
        45,
        [57, 61, 64],
        &[(0, 78, 8), (8, 76, 4)],
    ),
    bar_split(
        43,
        [55, 59, 62],
        45,
        [57, 61, 64],
        &[(0, 74, 4), (4, 71, 4), (8, 69, 4)],
    ),
    bar_same(50, [57, 62, 66], &[(0, 74, 12)]),
];

/// Piece IV: *Kemp's Jig* (Playford's *English Dancing Master*, 1651 — D Dorian, 6/8 time, 16 bars).
const KEMPS_JIG: [SheetBar; 16] = [
    bar_same(
        50,
        [57, 62, 65],
        &[(0, 77, 4), (4, 76, 2), (6, 74, 4), (10, 72, 2)],
    ),
    bar_same(43, [55, 59, 62], &[(0, 71, 4), (4, 72, 2), (6, 74, 6)]),
    bar_same(
        50,
        [57, 62, 65],
        &[(0, 77, 4), (4, 76, 2), (6, 74, 4), (10, 72, 2)],
    ),
    bar_split(
        45,
        [57, 60, 64],
        43,
        [55, 59, 62],
        &[(0, 71, 2), (2, 72, 2), (4, 69, 2), (6, 67, 6)],
    ),
    bar_split(
        41,
        [53, 60, 65],
        48,
        [55, 60, 64],
        &[(0, 69, 4), (4, 71, 2), (6, 72, 4), (10, 69, 2)],
    ),
    bar_split(
        43,
        [55, 59, 62],
        50,
        [57, 62, 65],
        &[(0, 71, 4), (4, 72, 2), (6, 74, 4), (10, 76, 2)],
    ),
    bar_split(
        50,
        [57, 62, 65],
        45,
        [57, 61, 64],
        &[(0, 77, 3), (3, 76, 1), (4, 74, 2), (6, 73, 4), (10, 74, 2)],
    ),
    bar_same(50, [57, 62, 65], &[(0, 74, 8), (8, 69, 4)]),
    // Second 8-bar statement
    bar_same(
        50,
        [57, 62, 65],
        &[(0, 77, 4), (4, 76, 2), (6, 74, 4), (10, 72, 2)],
    ),
    bar_same(43, [55, 59, 62], &[(0, 71, 4), (4, 72, 2), (6, 74, 6)]),
    bar_same(
        50,
        [57, 62, 65],
        &[(0, 77, 4), (4, 76, 2), (6, 74, 4), (10, 72, 2)],
    ),
    bar_split(
        45,
        [57, 60, 64],
        43,
        [55, 59, 62],
        &[(0, 71, 2), (2, 72, 2), (4, 69, 2), (6, 67, 6)],
    ),
    bar_split(
        41,
        [53, 60, 65],
        48,
        [55, 60, 64],
        &[(0, 69, 4), (4, 71, 2), (6, 72, 4), (10, 69, 2)],
    ),
    bar_split(
        43,
        [55, 59, 62],
        50,
        [57, 62, 65],
        &[(0, 71, 4), (4, 72, 2), (6, 74, 4), (10, 76, 2)],
    ),
    bar_split(
        50,
        [57, 62, 65],
        45,
        [57, 61, 64],
        &[(0, 77, 3), (3, 76, 1), (4, 74, 2), (6, 73, 4), (10, 74, 2)],
    ),
    bar_same(50, [57, 62, 65], &[(0, 74, 6), (6, 74, 6)]),
];

const PIECES: [[SheetBar; 16]; 4] = [GREENSLEEVES, SCARBOROUGH_FAIR, SI_BHEAG_SI_MHOR, KEMPS_JIG];

/// Marks all acoustic instrument pitches used by the Tavern sheet-music suite
/// so [`crate::music::orchestra::sampled`] pre-builds 48-kHz Studio clips for them.
pub(in crate::music) fn mark_studio_pitches(used: &mut [bool], key: fn(Instrument, u8) -> usize) {
    for piece in &PIECES {
        for bar in piece {
            for &b in &[bar.bass1, bar.bass2] {
                used[key(Instrument::Cello, b)] = true;
            }
            for tones in [bar.tones1, bar.tones2] {
                for t in tones {
                    used[key(Instrument::Harp, t)] = true;
                    used[key(Instrument::Zither, t)] = true;
                    used[key(Instrument::Viola, t)] = true;
                }
            }
            for &(_, pitch, _) in bar.melody {
                used[key(Instrument::Harp, pitch)] = true;
                used[key(Instrument::Zither, pitch)] = true;
                used[key(Instrument::Violin, pitch)] = true;
                if pitch >= 12 {
                    used[key(Instrument::Harp, pitch - 12)] = true;
                }
            }
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(in crate::music) struct Event {
    frame: u32,
    length: u32,
    gain: f32,
    pan: f32,
    pub instrument: u8,
    pub velocity: u8,
    pub pitch: u8,
}
#[cfg(test)]
impl Event {
    pub(in crate::music) fn note(self) -> Note {
        Note {
            instrument: self.instrument,
            pitch: self.pitch,
            frames: self.length,
            velocity: self.velocity,
            gain: self.gain,
            pan: self.pan,
        }
    }
}
fn number(at: usize) -> u32 {
    u32::from_le_bytes(
        DATA[at..at + 4]
            .try_into()
            .expect("checked-in performance record"),
    )
}
#[cfg(test)]
fn count() -> usize {
    number(4) as usize
}
fn end() -> u32 {
    number(8)
}
#[cfg(test)]
fn event(index: usize) -> Event {
    let at = HEADER + index * RECORD;
    Event {
        frame: number(at),
        length: number(at + 4),
        gain: f32::from_bits(number(at + 8)),
        pan: f32::from_bits(number(at + 12)),
        instrument: DATA[at + 16],
        velocity: DATA[at + 18],
        pitch: DATA[at + 17],
    }
}
/// All prepared events; the chamber bank covers every pitch and velocity layer.
#[cfg(test)]
pub(in crate::music) fn events() -> impl Iterator<Item = Event> {
    (0..count()).map(event)
}

#[derive(Default)]
pub(super) struct Performance {
    frame: u32,
    cursor: usize,
}
impl Performance {
    /// Schedule notes from the four-piece Tavern sheet-music suite onto `orchestra`.
    /// Bound the next observation to 50 ms so scene changes still respond promptly.
    pub(super) fn play(&mut self, orchestra: &mut Orchestra, samples: SampleSet) -> u32 {
        if self.frame.is_multiple_of(STEP_FRAMES) {
            let total_sub = (self.frame / STEP_FRAMES) as usize;
            let bar_total = total_sub / 12;
            let sub = (total_sub % 12) as u8;
            // Each 16-bar piece plays twice (32 bars per piece: intimate statement, then full tavern ensemble).
            let piece_idx = (bar_total / 32) % PIECES.len();
            let bar_in_32 = bar_total % 32;
            let rep = bar_in_32 / 16;
            let bar_in_piece = bar_in_32 % 16;
            let sheet = PIECES[piece_idx][bar_in_piece];
            Self::schedule_tavern_step(orchestra, samples, sheet, sub, rep, bar_in_piece);
            self.cursor = total_sub + 1;
        }
        let next_step = ((self.frame / STEP_FRAMES) + 1) * STEP_FRAMES;
        let next = next_step.min(end());
        (next - self.frame).clamp(1, RATE / 20)
    }

    fn schedule_tavern_step(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        sheet: SheetBar,
        sub: u8,
        rep: usize,
        bar_in_piece: usize,
    ) {
        let tones = if sub < 6 { sheet.tones1 } else { sheet.tones2 };

        // 1. Warm Bowed Cello bass & inner Viola triad pad
        if sub == 0 {
            let hold = if sheet.bass1 == sheet.bass2 {
                STEP_SECONDS * 11.5
            } else {
                STEP_SECONDS * 5.8
            };
            orchestra.note_with_samples(
                Instrument::Cello,
                sheet.bass1,
                hold,
                0.155,
                Touch::at(0.18).attack(0.055).release(0.30),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Viola,
                sheet.tones1[1],
                STEP_SECONDS * 10.5,
                0.095,
                Touch::at(0.26).attack(0.075).release(0.32).late(0.025),
                samples,
            );
        } else if sub == 6 && sheet.bass2 != sheet.bass1 {
            orchestra.note_with_samples(
                Instrument::Cello,
                sheet.bass2,
                STEP_SECONDS * 5.8,
                0.145,
                Touch::at(0.18).attack(0.055).release(0.28),
                samples,
            );
        }

        // 2. Flowing Celtic Folk Harp arpeggio across the bar (even sixteenth-ticks: 0, 2, 4, 6, 8, 10)
        if sub.is_multiple_of(2) {
            let arp_pitch = match sub {
                0 | 6 => tones[0],
                2 | 10 => tones[1],
                _ => tones[2],
            };
            let arp_gain = if sub == 0 || sub == 6 { 0.19 } else { 0.145 };
            orchestra.note_with_samples(
                Instrument::Harp,
                arp_pitch,
                STEP_SECONDS * 5.5,
                arp_gain,
                Touch::at(-0.36).attack(0.004).release(0.28),
                samples,
            );
            if (sub == 0 || sub == 6) && rep == 1 {
                orchestra.note_with_samples(
                    Instrument::Zither,
                    tones[2],
                    STEP_SECONDS * 4.5,
                    0.115,
                    Touch::at(0.34).attack(0.004).release(0.24).late(0.016),
                    samples,
                );
            }
        }

        // 3. Sheet-music melody on Renaissance Lute (Lyre), Folk Harp, and Solo Tavern Fiddle (Violin)
        Self::schedule_tavern_melody(orchestra, samples, sheet, tones, sub, rep, bar_in_piece);
    }

    fn schedule_tavern_melody(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        sheet: SheetBar,
        tones: [u8; 3],
        sub: u8,
        rep: usize,
        bar_in_piece: usize,
    ) {
        for &(onset, pitch, dur_ticks) in sheet.melody {
            if onset != sub {
                continue;
            }
            let dur = f32::from(dur_ticks) * STEP_SECONDS;
            orchestra.note_with_samples(
                Instrument::Lyre,
                pitch,
                dur * 1.15,
                0.28,
                Touch::at(0.08),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Harp,
                pitch,
                dur * 1.25,
                0.185,
                Touch::at(-0.28).attack(0.004).release(0.30).late(0.011),
                samples,
            );
            if sub == 0 || sub == 6 {
                orchestra.note_with_samples(
                    Instrument::Lyre,
                    tones[0],
                    STEP_SECONDS * 4.2,
                    0.14,
                    Touch::at(0.14).late(0.008),
                    samples,
                );
            }
            if rep == 1 || bar_in_piece >= 8 {
                let violin_gain = if rep == 1 { 0.20 } else { 0.165 };
                orchestra.note_with_samples(
                    Instrument::Violin,
                    pitch,
                    dur * 0.96,
                    violin_gain,
                    Touch::at(-0.22).attack(0.045).release(0.26).late(0.006),
                    samples,
                );
            }
        }
    }

    /// Preserve the place between sign-in and lobby, and pause it during a duel.
    pub(super) fn advance(&mut self) -> bool {
        self.frame += 1;
        let boundary = self.frame.is_multiple_of(RATE * 4);
        if self.frame == end() {
            self.frame = 0;
            self.cursor = 0;
        }
        boundary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_editions_fit_the_runtime_and_finish_before_the_loop() {
        assert_eq!(&DATA[..4], b"BAY2");
        assert_eq!(DATA.len(), HEADER + count() * RECORD);
        assert_eq!(count(), 2053); // Full piano parts, written repeats and our strings.
        let mut previous = 0;
        let mut ends = Vec::new();
        let mut polyphony = 0;
        for note in events() {
            assert!(note.frame >= previous);
            assert!(note.length > 0 && note.frame + note.length + RATE < end());
            assert!((27..=97).contains(&note.pitch));
            assert!(note.gain.is_finite() && note.gain > 0.0 && note.gain <= 0.9);
            assert!(note.pan.abs() <= 1.0);
            ends.retain(|&end| end > note.frame);
            ends.push(note.frame + note.length + RATE);
            polyphony = polyphony.max(ends.len());
            previous = note.frame;
        }
        assert!(polyphony < 64, "including release tails: {polyphony}");
        assert!(end() > RATE * 500);
        assert!(end() < RATE * 520);
    }
    #[test]
    fn a_loop_restarts_at_the_first_note_after_a_full_release() {
        let mut score = Performance {
            frame: end() - 1,
            cursor: count(),
        };
        score.advance();
        assert_eq!(score.frame, 0);
        assert_eq!(score.cursor, 0);
    }
}
