//! Public-domain sheet-music suites for Login (`Movement::Title`) and Lobby /
//! Deckbuilder (`Movement::Lobby`), unified in D minor / D Dorian / F major / D major:
//!
//! ### Login Title Suite (`Movement::Title`) — Noble, mysterious & inviting:
//! 1. George Frideric Handel (1685–1759), *Suite in D minor, HWV 437: IV. Sarabande* (1733)
//! 2. Antonín Dvořák (1841–1904), *Symphony No. 9, Op. 95: II. Largo* (1893) paired with
//!    Modest Mussorgsky (1839–1881), *Pictures at an Exhibition: Promenade* (1874), in F major / D minor
//!
//! ### Lobby & Deckbuilder Tavern Suite (`Movement::Lobby`) — Cozy Acoustic Guitar ("Taverne, Gitarre, gemütlich"):
//! 1. Gaspar Sanz (1640–1710), *Españoleta* (*Instrucción de música sobre la guitarra española*, 1674, D minor)
//! 2. Francisco Tárrega (1852–1909), *Lágrima* (1891, D major) & *Adelita* (c. 1901, D minor)
//! 3. Fernando Sor (1778–1839), *Study Op. 35 No. 22* (1828, transposed to D minor / F major)
//! 4. Johann Sebastian Bach (1685–1750), *Lute Suite BWV 996: Bourrée* (c. 1712) & *Prelude BWV 999* (c. 1720, D minor)
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

/// Sixteenth-note pulse for the noble Login Title suite: 9,600 frames (0.20 s -> 2.40 s per bar).
const TITLE_STEP_FRAMES: u32 = 9_600;
const TITLE_STEP_SECONDS: f32 = TITLE_STEP_FRAMES as f32 / RATE as f32;

/// Sixteenth-note pulse for the cozy Lobby Acoustic Guitar suite: 8,400 frames (0.175 s -> 2.10 s per bar).
const LOBBY_STEP_FRAMES: u32 = 8_400;
const LOBBY_STEP_SECONDS: f32 = LOBBY_STEP_FRAMES as f32 / RATE as f32;

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

/// Login Piece I: Handel *Sarabande in D minor, HWV 437* (1733 — stately 3/4 sarabande, 16 bars).
const HANDEL_SARABANDE: [SheetBar; 16] = [
    // d -> F -> Bb -> A
    bar_same(38, [50, 57, 62], &[(0, 74, 4), (4, 77, 6), (10, 69, 2)]),
    bar_same(41, [53, 60, 65], &[(0, 72, 4), (4, 81, 6), (10, 77, 2)]),
    bar_same(46, [53, 58, 62], &[(0, 74, 4), (4, 82, 6), (10, 77, 2)]),
    bar_same(45, [52, 57, 61], &[(0, 76, 4), (4, 73, 6), (10, 69, 2)]),
    // d -> F -> C -> F
    bar_same(38, [50, 57, 62], &[(0, 74, 4), (4, 69, 4), (8, 65, 4)]),
    bar_same(41, [53, 60, 65], &[(0, 69, 4), (4, 72, 6), (10, 77, 2)]),
    bar_same(36, [48, 55, 60], &[(0, 76, 4), (4, 79, 6), (10, 72, 2)]),
    bar_same(41, [53, 60, 65], &[(0, 77, 8), (8, 72, 4)]),
    // d -> g -> C -> F
    bar_same(38, [50, 57, 62], &[(0, 81, 4), (4, 77, 6), (10, 74, 2)]),
    bar_same(43, [50, 55, 58], &[(0, 79, 4), (4, 74, 6), (10, 70, 2)]),
    bar_same(36, [48, 55, 60], &[(0, 76, 4), (4, 72, 6), (10, 67, 2)]),
    bar_same(41, [53, 60, 65], &[(0, 69, 4), (4, 77, 6), (10, 72, 2)]),
    // Bb -> g -> A -> d
    bar_same(46, [53, 58, 62], &[(0, 74, 4), (4, 70, 4), (8, 65, 4)]),
    bar_same(43, [50, 55, 58], &[(0, 67, 4), (4, 74, 6), (10, 70, 2)]),
    bar_same(45, [52, 57, 61], &[(0, 69, 4), (4, 73, 6), (10, 76, 2)]),
    bar_same(38, [50, 57, 62], &[(0, 74, 12)]),
];

/// Login Piece II: Dvořák *Op. 95 II. Largo* (1893) & Mussorgsky *Promenade* (1874), F major -> D minor, 16 bars.
const DVORAK_PROMENADE_TITLE: [SheetBar; 16] = [
    // Dvořák Largo lyric call in F major -> D minor
    bar_same(41, [53, 57, 60], &[(0, 69, 3), (3, 72, 3), (6, 72, 6)]),
    bar_split(
        41,
        [53, 57, 60],
        38,
        [50, 57, 62],
        &[(0, 69, 3), (3, 67, 3), (6, 65, 6)],
    ),
    bar_split(
        46,
        [53, 58, 62],
        36,
        [48, 55, 60],
        &[(0, 70, 4), (4, 69, 4), (8, 72, 4)],
    ),
    bar_same(36, [48, 55, 60], &[(0, 67, 12)]),
    bar_same(41, [53, 57, 60], &[(0, 69, 3), (3, 72, 3), (6, 77, 6)]),
    bar_split(
        38,
        [50, 57, 62],
        43,
        [50, 55, 59],
        &[(0, 76, 3), (3, 74, 3), (6, 69, 6)],
    ),
    bar_split(
        46,
        [53, 58, 62],
        36,
        [48, 55, 60],
        &[(0, 70, 3), (3, 72, 3), (6, 69, 3), (9, 67, 3)],
    ),
    bar_same(41, [53, 57, 60], &[(0, 65, 12)]),
    // Mussorgsky Promenade stately modal answer, pivoting home to D minor
    bar_same(
        41,
        [53, 60, 65],
        &[(0, 67, 3), (3, 65, 3), (6, 69, 3), (9, 72, 3)],
    ),
    bar_split(
        46,
        [53, 58, 62],
        41,
        [53, 60, 65],
        &[(0, 77, 3), (3, 74, 3), (6, 72, 6)],
    ),
    bar_split(
        38,
        [50, 57, 62],
        36,
        [48, 55, 60],
        &[(0, 74, 4), (4, 69, 4), (8, 72, 4)],
    ),
    bar_same(43, [50, 55, 59], &[(0, 71, 6), (6, 67, 6)]),
    bar_split(
        38,
        [50, 57, 62],
        41,
        [53, 60, 65],
        &[(0, 69, 4), (4, 74, 4), (8, 77, 4)],
    ),
    bar_split(
        46,
        [53, 58, 62],
        43,
        [50, 55, 58],
        &[(0, 74, 4), (4, 70, 4), (8, 67, 4)],
    ),
    bar_same(45, [52, 57, 61], &[(0, 69, 6), (6, 73, 6)]),
    bar_same(38, [50, 57, 62], &[(0, 74, 12)]),
];

/// Lobby Piece I: Gaspar Sanz *Españoleta* (1674 — Spanish baroque guitar air in D minor, 16 bars).
const SANZ_ESPANOLETA: [SheetBar; 16] = [
    bar_same(38, [50, 57, 62], &[(0, 69, 4), (4, 74, 4), (8, 73, 4)]),
    bar_same(38, [50, 57, 62], &[(0, 74, 4), (4, 76, 4), (8, 77, 4)]),
    bar_split(
        36,
        [48, 55, 60],
        41,
        [53, 57, 60],
        &[(0, 76, 6), (6, 72, 4), (10, 69, 2)],
    ),
    bar_same(41, [53, 57, 60], &[(0, 77, 8), (8, 76, 4)]),
    bar_split(
        46,
        [53, 58, 62],
        43,
        [50, 55, 58],
        &[(0, 74, 4), (4, 70, 4), (8, 67, 4)],
    ),
    bar_same(45, [52, 57, 61], &[(0, 69, 6), (6, 64, 6)]),
    bar_split(
        38,
        [50, 57, 62],
        45,
        [52, 57, 61],
        &[(0, 65, 4), (4, 69, 4), (8, 73, 4)],
    ),
    bar_same(38, [50, 57, 62], &[(0, 74, 12)]),
    // Second period (Sanz variation in thirds and sixths)
    bar_split(
        41,
        [53, 57, 60],
        36,
        [48, 55, 60],
        &[(0, 81, 4), (4, 77, 4), (8, 76, 4)],
    ),
    bar_split(
        38,
        [50, 57, 62],
        45,
        [52, 57, 60],
        &[(0, 74, 4), (4, 72, 4), (8, 69, 4)],
    ),
    bar_split(
        46,
        [53, 58, 62],
        41,
        [53, 57, 60],
        &[(0, 70, 4), (4, 74, 4), (8, 72, 4)],
    ),
    bar_same(36, [48, 55, 60], &[(0, 67, 8), (8, 72, 4)]),
    bar_split(
        38,
        [50, 57, 62],
        43,
        [50, 55, 58],
        &[(0, 74, 4), (4, 77, 4), (8, 70, 4)],
    ),
    bar_split(
        38,
        [50, 57, 62],
        45,
        [52, 57, 61],
        &[(0, 69, 4), (4, 65, 4), (8, 64, 4)],
    ),
    bar_split(
        43,
        [50, 55, 58],
        45,
        [52, 57, 61],
        &[(0, 67, 4), (4, 65, 4), (8, 61, 4)],
    ),
    bar_same(38, [50, 57, 62], &[(0, 62, 12)]),
];

/// Lobby Piece II: Francisco Tárrega *Lágrima* (1891, D major) & *Adelita* (c. 1901, D minor), 16 bars.
const TARREGA_LAGRIMA_ADELITA: [SheetBar; 16] = [
    // Bars 1–8: Warm D-major parlor guitar (*Lágrima* lyrical contour over Drop-D thumb bass)
    bar_same(38, [50, 57, 66], &[(0, 78, 4), (4, 76, 4), (8, 74, 4)]),
    bar_split(
        43,
        [50, 55, 59],
        38,
        [50, 57, 66],
        &[(0, 71, 6), (6, 74, 6)],
    ),
    bar_split(
        43,
        [50, 55, 59],
        45,
        [52, 57, 61],
        &[(0, 79, 4), (4, 76, 4), (8, 73, 4)],
    ),
    bar_same(38, [50, 57, 66], &[(0, 74, 8), (8, 69, 4)]),
    bar_same(47, [54, 59, 62], &[(0, 78, 4), (4, 74, 4), (8, 71, 4)]),
    bar_same(40, [52, 55, 59], &[(0, 76, 4), (4, 71, 4), (8, 67, 4)]),
    bar_same(45, [52, 57, 61], &[(0, 69, 4), (4, 73, 4), (8, 76, 4)]),
    bar_same(38, [50, 57, 66], &[(0, 74, 12)]),
    // Bars 9–16: Tender parallel D-minor turn (*Adelita* intimate mazurka lilt)
    bar_same(38, [50, 57, 65], &[(0, 81, 6), (6, 77, 3), (9, 74, 3)]),
    bar_same(43, [50, 55, 58], &[(0, 79, 6), (6, 74, 3), (9, 70, 3)]),
    bar_same(36, [48, 55, 60], &[(0, 76, 6), (6, 72, 3), (9, 67, 3)]),
    bar_same(41, [53, 57, 60], &[(0, 69, 8), (8, 72, 4)]),
    bar_same(46, [53, 58, 62], &[(0, 74, 6), (6, 70, 3), (9, 65, 3)]),
    bar_same(43, [50, 55, 58], &[(0, 67, 6), (6, 70, 6)]),
    bar_same(45, [52, 57, 61], &[(0, 69, 4), (4, 73, 4), (8, 69, 4)]),
    bar_same(38, [50, 57, 62], &[(0, 62, 12)]),
];

/// Lobby Piece III: Fernando Sor *Study Op. 35 No. 22* (1828, transposed to D minor / F major, 16 bars).
const SOR_STUDY_OP35_22: [SheetBar; 16] = [
    bar_same(38, [50, 57, 62], &[(0, 65, 8), (8, 69, 4)]),
    bar_same(38, [50, 57, 62], &[(0, 74, 8), (8, 77, 4)]),
    bar_same(45, [52, 57, 61], &[(0, 76, 8), (8, 73, 4)]),
    bar_same(38, [50, 57, 62], &[(0, 74, 12)]),
    bar_same(36, [48, 55, 60], &[(0, 76, 8), (8, 79, 4)]),
    bar_same(41, [53, 57, 60], &[(0, 81, 8), (8, 77, 4)]),
    bar_same(36, [48, 55, 60], &[(0, 76, 4), (4, 72, 4), (8, 76, 4)]),
    bar_same(41, [53, 57, 60], &[(0, 77, 12)]),
    bar_same(43, [50, 55, 58], &[(0, 79, 8), (8, 74, 4)]),
    bar_same(38, [50, 57, 62], &[(0, 77, 8), (8, 69, 4)]),
    bar_same(45, [52, 57, 61], &[(0, 76, 8), (8, 73, 4)]),
    bar_same(38, [50, 57, 62], &[(0, 74, 8), (8, 65, 4)]),
    bar_same(46, [53, 58, 62], &[(0, 70, 8), (8, 74, 4)]),
    bar_same(43, [50, 55, 58], &[(0, 67, 8), (8, 70, 4)]),
    bar_same(45, [52, 57, 61], &[(0, 69, 4), (4, 64, 4), (8, 69, 4)]),
    bar_same(38, [50, 57, 62], &[(0, 62, 12)]),
];

/// Lobby Piece IV: J.S. Bach *Bourrée BWV 996* & *Prelude BWV 999* (c. 1712–1720, D minor, 16 bars).
const BACH_BOURREE_BWV996: [SheetBar; 16] = [
    bar_split(
        38,
        [50, 57, 62],
        45,
        [52, 57, 61],
        &[(0, 74, 3), (3, 76, 3), (6, 77, 3), (9, 73, 3)],
    ),
    bar_split(
        38,
        [50, 57, 62],
        36,
        [48, 55, 60],
        &[(0, 74, 6), (6, 69, 3), (9, 72, 3)],
    ),
    bar_split(
        46,
        [53, 58, 62],
        41,
        [53, 57, 60],
        &[(0, 70, 3), (3, 69, 3), (6, 65, 3), (9, 69, 3)],
    ),
    bar_split(
        43,
        [50, 55, 58],
        45,
        [52, 57, 61],
        &[(0, 67, 6), (6, 69, 6)],
    ),
    bar_split(
        38,
        [50, 57, 62],
        36,
        [48, 55, 60],
        &[(0, 74, 3), (3, 76, 3), (6, 79, 3), (9, 76, 3)],
    ),
    bar_split(
        41,
        [53, 57, 60],
        46,
        [53, 58, 62],
        &[(0, 81, 6), (6, 74, 6)],
    ),
    bar_split(
        36,
        [48, 55, 60],
        41,
        [53, 57, 60],
        &[(0, 76, 3), (3, 72, 3), (6, 77, 3), (9, 72, 3)],
    ),
    bar_same(41, [53, 57, 60], &[(0, 77, 12)]),
    // BWV 999 Prelude-inspired harp/guitar dialogue resolving to D minor
    bar_same(
        38,
        [50, 57, 62],
        &[(0, 69, 3), (3, 74, 3), (6, 81, 3), (9, 77, 3)],
    ),
    bar_same(
        43,
        [50, 55, 58],
        &[(0, 74, 3), (3, 70, 3), (6, 79, 3), (9, 74, 3)],
    ),
    bar_same(
        45,
        [52, 57, 61],
        &[(0, 73, 3), (3, 69, 3), (6, 76, 3), (9, 73, 3)],
    ),
    bar_same(38, [50, 57, 62], &[(0, 74, 6), (6, 69, 6)]),
    bar_same(
        46,
        [53, 58, 62],
        &[(0, 70, 3), (3, 74, 3), (6, 77, 3), (9, 74, 3)],
    ),
    bar_same(
        43,
        [50, 55, 58],
        &[(0, 70, 3), (3, 67, 3), (6, 74, 3), (9, 70, 3)],
    ),
    bar_same(
        45,
        [52, 57, 61],
        &[(0, 69, 3), (3, 65, 3), (6, 64, 3), (9, 61, 3)],
    ),
    bar_same(38, [50, 57, 62], &[(0, 62, 12)]),
];

const TITLE_PIECES: [[SheetBar; 16]; 2] = [HANDEL_SARABANDE, DVORAK_PROMENADE_TITLE];
const LOBBY_PIECES: [[SheetBar; 16]; 4] = [
    SANZ_ESPANOLETA,
    TARREGA_LAGRIMA_ADELITA,
    SOR_STUDY_OP35_22,
    BACH_BOURREE_BWV996,
];

/// Marks all acoustic instrument pitches used by both the Title and Lobby suites
/// so [`crate::music::orchestra::sampled`] pre-builds 48-kHz Studio clips for them.
pub(in crate::music) fn mark_studio_pitches(used: &mut [bool], key: fn(Instrument, u8) -> usize) {
    used[key(Instrument::Harp, 74)] = true;
    for piece in &TITLE_PIECES {
        for bar in piece {
            for &b in &[bar.bass1, bar.bass2] {
                used[key(Instrument::Bass, b)] = true;
                used[key(Instrument::Cello, b + 12)] = true;
            }
            for tones in [bar.tones1, bar.tones2] {
                for t in tones {
                    used[key(Instrument::Harp, t)] = true;
                    used[key(Instrument::Viola, t)] = true;
                }
            }
            for &(_, pitch, _) in bar.melody {
                used[key(Instrument::Harp, pitch)] = true;
                used[key(Instrument::Violin, pitch)] = true;
            }
        }
    }
    for piece in &LOBBY_PIECES {
        for bar in piece {
            for &b in &[bar.bass1, bar.bass2] {
                used[key(Instrument::Cello, b + 12)] = true;
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
                used[key(Instrument::Violin, pitch)] = true;
            }
        }
    }
}

#[cfg(test)]
pub(in crate::music) fn sheet_melodies() -> Vec<(&'static str, Vec<u8>)> {
    [
        ("Handel Sarabande HWV 437", &HANDEL_SARABANDE),
        (
            "Dvorak Largo & Mussorgsky Promenade",
            &DVORAK_PROMENADE_TITLE,
        ),
        ("Gaspar Sanz Espanoleta", &SANZ_ESPANOLETA),
        ("Tarrega Lagrima & Adelita", &TARREGA_LAGRIMA_ADELITA),
        ("Fernando Sor Study Op. 35 No. 22", &SOR_STUDY_OP35_22),
        (
            "Bach Bourree BWV 996 & Prelude BWV 999",
            &BACH_BOURREE_BWV996,
        ),
    ]
    .into_iter()
    .map(|(name, bars)| {
        let pitches = bars
            .iter()
            .flat_map(|bar| bar.melody.iter().map(|&(_, p, _)| p))
            .collect();
        (name, pitches)
    })
    .collect()
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
    title_frame: u32,
    lobby_frame: u32,
}
impl Performance {
    /// Step to the next 16-bar guitar piece when returning to the Lobby after a duel.
    pub(super) fn next_lobby_piece(&mut self) {
        if self.lobby_frame > 0 {
            let piece_frames = LOBBY_STEP_FRAMES * 12 * 16;
            self.lobby_frame = ((self.lobby_frame / piece_frames) + 1) * piece_frames;
        }
    }

    /// Schedule notes for either the noble Login Title suite (`Movement::Title`)
    /// or the cozy Acoustic Guitar Tavern suite (`Movement::Lobby`).
    /// Aligns observations to musical sixteenth pulses (`<= 0.20 s`) for beat-aligned transitions.
    pub(super) fn play(
        &mut self,
        movement: Movement,
        orchestra: &mut Orchestra,
        samples: SampleSet,
    ) -> u32 {
        if movement == Movement::Title {
            if self.title_frame.is_multiple_of(TITLE_STEP_FRAMES) {
                let total_sub = (self.title_frame / TITLE_STEP_FRAMES) as usize;
                let bar_total = total_sub / 12;
                let sub = (total_sub % 12) as u8;
                let piece_idx = (bar_total / 32) % TITLE_PIECES.len();
                let bar_in_32 = bar_total % 32;
                let rep = bar_in_32 / 16;
                let bar_in_piece = bar_in_32 % 16;
                let sheet = TITLE_PIECES[piece_idx][bar_in_piece];
                Self::schedule_title_step(
                    orchestra,
                    samples,
                    sheet,
                    sub,
                    rep,
                    piece_idx,
                    bar_in_piece,
                );
            }
            let next_step = ((self.title_frame / TITLE_STEP_FRAMES) + 1) * TITLE_STEP_FRAMES;
            return (next_step - self.title_frame).clamp(1, TITLE_STEP_FRAMES);
        }

        if self.lobby_frame.is_multiple_of(LOBBY_STEP_FRAMES) {
            let total_sub = (self.lobby_frame / LOBBY_STEP_FRAMES) as usize;
            let bar_total = total_sub / 12;
            let sub = (total_sub % 12) as u8;
            let piece_idx = (bar_total / 32) % LOBBY_PIECES.len();
            let bar_in_32 = bar_total % 32;
            let rep = bar_in_32 / 16;
            let bar_in_piece = bar_in_32 % 16;
            let sheet = LOBBY_PIECES[piece_idx][bar_in_piece];
            Self::schedule_lobby_guitar_step(orchestra, samples, sheet, sub, rep, bar_in_piece);
            self.cursor = total_sub + 1;
        }
        let next_step = ((self.lobby_frame / LOBBY_STEP_FRAMES) + 1) * LOBBY_STEP_FRAMES;
        (next_step - self.lobby_frame).clamp(1, LOBBY_STEP_FRAMES)
    }

    fn schedule_title_step(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        sheet: SheetBar,
        sub: u8,
        rep: usize,
        piece_idx: usize,
        bar_in_piece: usize,
    ) {
        let tones = if sub < 6 { sheet.tones1 } else { sheet.tones2 };
        if sub == 0 {
            let hold = if sheet.bass1 == sheet.bass2 {
                TITLE_STEP_SECONDS * 11.4
            } else {
                TITLE_STEP_SECONDS * 5.7
            };
            orchestra.note_with_samples(
                Instrument::Bass,
                sheet.bass1,
                hold,
                0.135,
                Touch::at(0.0).attack(0.065).release(0.38),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Cello,
                sheet.bass1 + 12,
                hold,
                0.135,
                Touch::at(0.16).attack(0.060).release(0.36),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::StringPad,
                sheet.tones1[1],
                TITLE_STEP_SECONDS * 11.2,
                0.075,
                Touch::at(-0.22),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Viola,
                sheet.tones1[2],
                TITLE_STEP_SECONDS * 10.6,
                0.085,
                Touch::at(0.24).attack(0.075).release(0.36).late(0.030),
                samples,
            );
        } else if sub == 6 && sheet.bass2 != sheet.bass1 {
            orchestra.note_with_samples(
                Instrument::Bass,
                sheet.bass2,
                TITLE_STEP_SECONDS * 5.7,
                0.125,
                Touch::at(0.0).attack(0.065).release(0.36),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Cello,
                sheet.bass2 + 12,
                TITLE_STEP_SECONDS * 5.7,
                0.125,
                Touch::at(0.16).attack(0.060).release(0.34),
                samples,
            );
        }

        // Noble Harp & Acoustic Guitar sarabande/promenade rolled chords
        if [0, 4, 8].contains(&sub) {
            let idx = (sub / 4) as usize;
            orchestra.note_with_samples(
                Instrument::Harp,
                tones[idx],
                TITLE_STEP_SECONDS * 5.4,
                if sub == 4 { 0.16 } else { 0.135 },
                Touch::at(-0.36).attack(0.004).release(0.32),
                samples,
            );
            if sub == 0 || sub == 4 {
                orchestra.note_with_samples(
                    Instrument::TavernGuitar,
                    tones[idx],
                    TITLE_STEP_SECONDS * 4.8,
                    0.125,
                    Touch::at(-0.08).late(0.014),
                    samples,
                );
            }
        }
        Self::schedule_title_melody(
            orchestra,
            samples,
            sheet.melody,
            sub,
            rep,
            piece_idx,
            bar_in_piece,
        );
    }

    fn schedule_title_melody(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        melody: &[(u8, u8, u8)],
        sub: u8,
        rep: usize,
        piece_idx: usize,
        bar_in_piece: usize,
    ) {
        for &(onset, pitch, dur_ticks) in melody {
            if onset != sub {
                continue;
            }
            let dur = f32::from(dur_ticks) * TITLE_STEP_SECONDS;
            if piece_idx == 0 {
                // Handel Sarabande: warm French Horn + Violin + Harp accent
                orchestra.note_with_samples(
                    Instrument::FrenchHorn,
                    pitch - 12,
                    dur * 1.05,
                    0.165,
                    Touch::at(0.14),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::Violin,
                    pitch,
                    dur * 0.98,
                    0.215,
                    Touch::at(-0.22).attack(0.050).release(0.32),
                    samples,
                );
            } else {
                // Dvořák Largo & Mussorgsky Promenade: lyrical Flute + French Horn (+ Violin on B-part/repeat)
                orchestra.note_with_samples(
                    Instrument::Flute,
                    pitch,
                    dur * 1.02,
                    0.225,
                    Touch::at(-0.12),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::FrenchHorn,
                    pitch - 12,
                    dur * 1.06,
                    0.155,
                    Touch::at(0.16).late(0.010),
                    samples,
                );
                if rep == 1 || bar_in_piece >= 8 {
                    orchestra.note_with_samples(
                        Instrument::Violin,
                        pitch,
                        dur * 0.96,
                        0.175,
                        Touch::at(-0.24).attack(0.055).release(0.32).late(0.008),
                        samples,
                    );
                }
            }
            orchestra.note_with_samples(
                Instrument::Harp,
                pitch,
                dur * 1.2,
                0.135,
                Touch::at(-0.30).attack(0.004).release(0.32).late(0.012),
                samples,
            );
            if onset == 0 && bar_in_piece.is_multiple_of(4) {
                orchestra.note_with_samples(
                    Instrument::Celesta,
                    pitch.min(86),
                    TITLE_STEP_SECONDS * 5.0,
                    0.075,
                    Touch::at(0.28).late(0.018),
                    samples,
                );
            }
        }
    }

    fn schedule_lobby_guitar_step(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        sheet: SheetBar,
        sub: u8,
        rep: usize,
        bar_in_piece: usize,
    ) {
        let tones = if sub < 6 { sheet.tones1 } else { sheet.tones2 };

        // 1. Warm thumb-plucked Drop-D Acoustic Tavern Guitar bass + soft Cello hearth root
        if sub == 0 {
            let hold = if sheet.bass1 == sheet.bass2 {
                LOBBY_STEP_SECONDS * 11.2
            } else {
                LOBBY_STEP_SECONDS * 5.6
            };
            orchestra.note_with_samples(
                Instrument::TavernGuitar,
                sheet.bass1,
                hold,
                0.235,
                Touch::at(-0.08),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Cello,
                sheet.bass1 + 12,
                hold,
                0.095,
                Touch::at(0.18).attack(0.065).release(0.34),
                samples,
            );
            if rep == 1 {
                orchestra.note_with_samples(
                    Instrument::Viola,
                    sheet.tones1[1],
                    LOBBY_STEP_SECONDS * 10.2,
                    0.072,
                    Touch::at(0.26).attack(0.075).release(0.34).late(0.025),
                    samples,
                );
            }
        } else if sub == 6 && sheet.bass2 != sheet.bass1 {
            orchestra.note_with_samples(
                Instrument::TavernGuitar,
                sheet.bass2,
                LOBBY_STEP_SECONDS * 5.6,
                0.22,
                Touch::at(-0.08),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Cello,
                sheet.bass2 + 12,
                LOBBY_STEP_SECONDS * 5.6,
                0.088,
                Touch::at(0.18).attack(0.065).release(0.32),
                samples,
            );
        }

        // 2. Intimate fingerstyle Acoustic Guitar arpeggio + gentle Folk Harp droplets
        if sub.is_multiple_of(2) {
            let arp_pitch = match sub {
                0 | 6 => tones[0],
                2 | 8 => tones[1],
                _ => tones[2],
            };
            orchestra.note_with_samples(
                Instrument::TavernGuitar,
                arp_pitch,
                LOBBY_STEP_SECONDS * 4.8,
                if sub == 0 || sub == 6 { 0.145 } else { 0.13 },
                Touch::at(-0.04).late(0.010),
                samples,
            );
            if sub == 0 || sub == 6 {
                orchestra.note_with_samples(
                    Instrument::Harp,
                    tones[2],
                    LOBBY_STEP_SECONDS * 5.4,
                    0.135,
                    Touch::at(-0.36).attack(0.004).release(0.30).late(0.016),
                    samples,
                );
            } else if sub == 4 && rep == 1 {
                orchestra.note_with_samples(
                    Instrument::Zither,
                    tones[1],
                    LOBBY_STEP_SECONDS * 4.2,
                    0.085,
                    Touch::at(0.34).attack(0.004).release(0.26).late(0.018),
                    samples,
                );
            }
        }
        Self::schedule_lobby_melody(orchestra, samples, sheet.melody, sub, rep, bar_in_piece);
    }

    // 3. Singing nylon/gut Acoustic Tavern Guitar melody (+ gentle Lute/Flute/Violin warmth)
    fn schedule_lobby_melody(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        melody: &[(u8, u8, u8)],
        sub: u8,
        rep: usize,
        bar_in_piece: usize,
    ) {
        for &(onset, pitch, dur_ticks) in melody {
            if onset != sub {
                continue;
            }
            let dur = f32::from(dur_ticks) * LOBBY_STEP_SECONDS;
            orchestra.note_with_samples(
                Instrument::TavernGuitar,
                pitch,
                dur * 1.18,
                0.295,
                Touch::at(-0.05).late(0.004),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Lyre,
                pitch,
                dur * 1.10,
                0.135,
                Touch::at(0.12).late(0.012),
                samples,
            );
            if sub == 0 || sub == 6 {
                orchestra.note_with_samples(
                    Instrument::Harp,
                    pitch,
                    dur * 1.25,
                    0.13,
                    Touch::at(-0.28).attack(0.004).release(0.30).late(0.015),
                    samples,
                );
            }
            if rep == 1 && bar_in_piece >= 8 {
                orchestra.note_with_samples(
                    Instrument::Flute,
                    pitch,
                    dur * 0.94,
                    0.115,
                    Touch::at(0.16).late(0.010),
                    samples,
                );
            } else if rep == 1 {
                orchestra.note_with_samples(
                    Instrument::Violin,
                    pitch,
                    dur * 0.94,
                    0.125,
                    Touch::at(-0.22).attack(0.055).release(0.28).late(0.008),
                    samples,
                );
            }
        }
    }

    /// Advance the active suite's sample clock; Lobby <-> Deckbuilder stays continuous,
    /// while Title and Duel leave the Lobby guitar cursor paused until return.
    #[cfg(test)]
    pub(super) fn advance(&mut self) -> bool {
        self.advance_for(Movement::Lobby)
    }

    pub(super) fn advance_for(&mut self, movement: Movement) -> bool {
        self.frame += 1;
        let boundary = self.frame.is_multiple_of(RATE * 4);
        if self.frame == end() {
            self.frame = 0;
            self.cursor = 0;
        }
        if movement == Movement::Title {
            let loop_len = TITLE_STEP_FRAMES * 12 * 32 * TITLE_PIECES.len() as u32;
            self.title_frame = (self.title_frame + 1) % loop_len;
        } else {
            let loop_len = LOBBY_STEP_FRAMES * 12 * 32 * LOBBY_PIECES.len() as u32;
            self.lobby_frame = (self.lobby_frame + 1) % loop_len;
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
            title_frame: 0,
            lobby_frame: 0,
        };
        score.advance();
        assert_eq!(score.frame, 0);
        assert_eq!(score.cursor, 0);
    }
}
