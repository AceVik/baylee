//! Public-domain acoustic suites for Login (`Movement::Title`) and Lobby /
//! Deckbuilder (`Movement::Lobby`), unified around D minor / D Dorian / F major / D major:
//!
//! ### Login Title Suite (`Movement::Title`) — Stately, spacious & orchestral:
//! 1. George Frideric Handel (1685–1759), *Suite in D minor, HWV 437: IV. Sarabande* (1733)
//! 2. Antonín Dvořák (1841–1904), *Symphony No. 9, Op. 95: II. Largo* (1893, F major / D minor)
//!
//! Performed exclusively on real CC0 studio-recorded acoustic instruments (`Harp`,
//! `Violin`, `Viola`, `Cello`, `Bass`, `Trombone`) with gentle Concert Harp overtone
//! accents — zero static synthesizer pads or detuned organ patches.
//!
//! ### Lobby & Deckbuilder Tavern Suite (`Movement::Lobby`) — Cozy Acoustic Guitar & Folk Harp:
//! 1. *Greensleeves* (Anonymous English Renaissance Lute Ballad, 1580, D Dorian / D minor)
//! 2. *Scarborough Fair* (Traditional English Medieval Ballad, D Dorian)
//! 3. *Si Bheag, Si Mhor* (Turlough O'Carolan, 1691 Celtic Harp Air, D major)
//! 4. Gaspar Sanz (1640–1710), *Españoleta* (*Instrucción de música sobre la guitarra española*, 1674, D minor)
//!
//! Paced at `LOBBY_STEP_FRAMES = 13_200` (`0.275 s` sixteenth -> `0.550 s` eighth strum =
//! `109.091 BPM`). That pace was chosen to lock every strum 1:1 to the bard's right hand
//! in the 16.11.2026 tavern loop (`27` strums = `54` ticks = `14.85 s` = `891` frames @
//! `60 fps`). The 18.11.2026 loop is another scene of `750` frames (`12.5 s`), so the
//! pace is now only this suite's tempo and nothing on screen depends on it.
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

/// Sixteenth-note pulse for the stately Login Title suite: 10,800 frames (`0.225 s` -> `2.70 s` per `3/4` Sarabande bar).
pub(in crate::music) const TITLE_STEP_FRAMES: u32 = 10_800;
const TITLE_STEP_SECONDS: f32 = TITLE_STEP_FRAMES as f32 / RATE as f32;

/// Sixteenth-note pulse for the cozy Lobby Tavern suite: 13,200 frames at 48 kHz
/// (`0.275 s` sixteenth -> `0.550 s` eighth-note strum [`109.091 BPM`, `33f @ 60fps`] ->
/// `3.300 s` `6/8` bar [`198f @ 60fps`]; `54` sixteenth ticks [`27` Bard strums] =
/// `712,800` samples = `14.85 s`, the length of the 16.11.2026 tavern loop it was
/// tuned to; the loop that replaced it on 18.11.2026 is not locked to this pulse).
pub(in crate::music) const LOBBY_STEP_FRAMES: u32 = 13_200;
const LOBBY_STEP_SECONDS: f32 = LOBBY_STEP_FRAMES as f32 / RATE as f32;

pub(super) const fn contains(movement: Movement) -> bool {
    matches!(movement, Movement::Title | Movement::Lobby)
}

#[derive(Clone, Copy)]
struct SheetBar {
    split: u8,
    bass1: u8,
    tones1: [u8; 3],
    bass2: u8,
    tones2: [u8; 3],
    melody: &'static [(u8, u8, u8)],
}

const fn bar_same(bass: u8, tones: [u8; 3], melody: &'static [(u8, u8, u8)]) -> SheetBar {
    SheetBar {
        split: 12,
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
        split: 6,
        bass1,
        tones1,
        bass2,
        tones2,
        melody,
    }
}

/// Login Piece I: George Frideric Handel, *Sarabande in D minor, HWV 437* (1733 — 16 bars in `3/4`).
const HANDEL_SARABANDE: [SheetBar; 16] = [
    // Bars 1–4: Dm -> F -> Bb -> A
    bar_same(38, [57, 62, 65], &[(0, 74, 4), (4, 74, 6), (10, 77, 2)]),
    bar_same(41, [57, 60, 65], &[(0, 72, 4), (4, 72, 6), (10, 77, 2)]),
    bar_same(46, [58, 62, 65], &[(0, 74, 4), (4, 82, 6), (10, 77, 2)]),
    bar_same(45, [57, 61, 64], &[(0, 76, 4), (4, 73, 6), (10, 69, 2)]),
    // Bars 5–8: Dm -> F -> C -> F
    bar_same(38, [57, 62, 65], &[(0, 74, 4), (4, 69, 6), (10, 65, 2)]),
    bar_same(41, [57, 60, 65], &[(0, 69, 4), (4, 72, 6), (10, 77, 2)]),
    bar_same(36, [55, 60, 64], &[(0, 76, 4), (4, 79, 6), (10, 72, 2)]),
    bar_same(41, [57, 60, 65], &[(0, 77, 8), (8, 72, 4)]),
    // Bars 9–12: Dm -> Gm -> C -> F (orchestral violin & cello swell)
    bar_same(38, [57, 62, 65], &[(0, 81, 4), (4, 77, 6), (10, 74, 2)]),
    bar_same(43, [55, 58, 62], &[(0, 79, 4), (4, 74, 6), (10, 70, 2)]),
    bar_same(36, [55, 60, 64], &[(0, 76, 4), (4, 72, 6), (10, 67, 2)]),
    bar_same(41, [57, 60, 65], &[(0, 69, 4), (4, 77, 6), (10, 72, 2)]),
    // Bars 13–16: Bb -> Gm -> A -> Dm
    bar_same(46, [58, 62, 65], &[(0, 74, 4), (4, 70, 4), (8, 65, 4)]),
    bar_same(43, [55, 58, 62], &[(0, 67, 4), (4, 74, 6), (10, 70, 2)]),
    bar_same(45, [57, 61, 64], &[(0, 69, 4), (4, 73, 6), (10, 76, 2)]),
    bar_same(38, [57, 62, 65], &[(0, 74, 12)]),
];

/// Login Piece II: Antonín Dvořák, *Symphony No. 9, Op. 95: II. Largo* (1893, F major / D minor, 16 bars).
const DVORAK_LARGO: [SheetBar; 16] = [
    bar_same(41, [57, 60, 65], &[(0, 69, 4), (4, 72, 2), (6, 72, 6)]),
    bar_split(
        41,
        [57, 60, 65],
        38,
        [57, 62, 65],
        &[(0, 69, 4), (4, 65, 2), (6, 62, 6)],
    ),
    bar_split(
        46,
        [58, 62, 65],
        36,
        [55, 60, 64],
        &[(0, 65, 4), (4, 70, 2), (6, 67, 6)],
    ),
    bar_same(41, [57, 60, 65], &[(0, 69, 8), (8, 65, 4)]),
    bar_same(41, [57, 60, 65], &[(0, 69, 4), (4, 72, 2), (6, 77, 6)]),
    bar_split(
        38,
        [57, 62, 65],
        43,
        [55, 58, 62],
        &[(0, 74, 4), (4, 69, 2), (6, 70, 6)],
    ),
    bar_split(
        36,
        [55, 60, 64],
        41,
        [57, 60, 65],
        &[(0, 72, 4), (4, 67, 2), (6, 69, 6)],
    ),
    bar_same(41, [57, 60, 65], &[(0, 65, 12)]),
    // Second phrase: gentle modal ascent returning home to D minor
    bar_same(38, [57, 62, 65], &[(0, 74, 4), (4, 77, 4), (8, 81, 4)]),
    bar_split(
        43,
        [55, 58, 62],
        36,
        [55, 60, 64],
        &[(0, 79, 4), (4, 74, 2), (6, 76, 6)],
    ),
    bar_split(
        41,
        [57, 60, 65],
        38,
        [57, 62, 65],
        &[(0, 77, 4), (4, 72, 2), (6, 74, 6)],
    ),
    bar_same(45, [57, 61, 64], &[(0, 73, 8), (8, 69, 4)]),
    bar_split(
        38,
        [57, 62, 65],
        46,
        [58, 62, 65],
        &[(0, 74, 4), (4, 77, 2), (6, 74, 6)],
    ),
    bar_split(
        43,
        [55, 58, 62],
        45,
        [57, 61, 64],
        &[(0, 70, 4), (4, 67, 2), (6, 69, 6)],
    ),
    bar_same(38, [57, 62, 65], &[(0, 65, 6), (6, 69, 6)]),
    bar_same(38, [57, 62, 65], &[(0, 62, 12)]),
];

/// Lobby Piece I: *Greensleeves* (Anonymous, 1580 — D minor / C major / F major, `6/8` ballad, 16 bars).
const GREENSLEEVES: [SheetBar; 16] = [
    // Verse: Dm -> C -> Dm -> A
    bar_split(
        38,
        [57, 62, 65],
        36,
        [55, 60, 64],
        &[(0, 65, 4), (4, 69, 2), (6, 67, 4), (10, 64, 2)],
    ),
    bar_split(
        46,
        [58, 62, 65],
        45,
        [57, 64, 69],
        &[(0, 62, 4), (4, 65, 2), (6, 64, 6)],
    ),
    bar_split(
        38,
        [57, 62, 65],
        36,
        [55, 60, 64],
        &[(0, 65, 4), (4, 74, 2), (6, 72, 4), (10, 67, 2)],
    ),
    bar_split(
        38,
        [57, 62, 65],
        45,
        [57, 61, 64],
        &[(0, 69, 6), (6, 69, 4), (10, 73, 2)],
    ),
    bar_split(
        38,
        [57, 62, 65],
        36,
        [55, 60, 64],
        &[(0, 74, 4), (4, 69, 2), (6, 67, 4), (10, 64, 2)],
    ),
    bar_split(
        46,
        [58, 62, 65],
        45,
        [57, 61, 64],
        &[(0, 65, 4), (4, 62, 2), (6, 61, 6)],
    ),
    bar_split(
        38,
        [57, 62, 65],
        45,
        [57, 61, 64],
        &[(0, 62, 4), (4, 65, 2), (6, 69, 4), (10, 64, 2)],
    ),
    bar_same(38, [57, 62, 65], &[(0, 62, 12)]),
    // Refrain ("Greensleeves was all my joy"): F -> C -> Dm -> A -> F -> C -> A -> Dm
    bar_same(41, [57, 60, 65], &[(0, 77, 6), (6, 77, 4), (10, 72, 2)]),
    bar_same(36, [55, 60, 64], &[(0, 76, 4), (4, 72, 2), (6, 67, 6)]),
    bar_same(38, [57, 62, 65], &[(0, 74, 4), (4, 69, 2), (6, 65, 6)]),
    bar_same(45, [57, 61, 64], &[(0, 69, 6), (6, 64, 6)]),
    bar_same(41, [57, 60, 65], &[(0, 77, 6), (6, 77, 4), (10, 72, 2)]),
    bar_same(36, [55, 60, 64], &[(0, 76, 4), (4, 72, 2), (6, 67, 6)]),
    bar_split(
        38,
        [57, 62, 65],
        45,
        [57, 61, 64],
        &[(0, 69, 4), (4, 65, 2), (6, 64, 4), (10, 61, 2)],
    ),
    bar_same(38, [57, 62, 65], &[(0, 62, 12)]),
];

/// Lobby Piece II: *Scarborough Fair* (Traditional English Dorian Ballad, D Dorian, 16 bars).
const SCARBOROUGH_FAIR: [SheetBar; 16] = [
    // "Are you going to Scarborough Fair?"
    bar_same(38, [57, 62, 65], &[(0, 62, 8), (8, 62, 4)]),
    bar_same(38, [57, 62, 65], &[(0, 69, 8), (8, 69, 4)]),
    bar_split(
        36,
        [55, 60, 64],
        38,
        [57, 62, 69],
        &[(0, 64, 6), (6, 65, 4), (10, 64, 2)],
    ),
    bar_same(38, [57, 62, 65], &[(0, 62, 12)]),
    // "Parsley, sage, rosemary and thyme"
    bar_split(
        41,
        [57, 60, 65],
        36,
        [55, 60, 64],
        &[(0, 69, 4), (4, 72, 2), (6, 76, 6)],
    ),
    bar_split(
        38,
        [57, 62, 65],
        43,
        [55, 59, 62],
        &[(0, 74, 6), (6, 71, 4), (10, 67, 2)],
    ),
    bar_split(
        41,
        [57, 60, 65],
        36,
        [55, 60, 64],
        &[(0, 69, 6), (6, 67, 6)],
    ),
    bar_same(38, [57, 62, 65], &[(0, 69, 12)]),
    // "Remember me to one who lives there"
    bar_same(38, [57, 62, 65], &[(0, 74, 8), (8, 74, 4)]),
    bar_same(36, [55, 60, 64], &[(0, 72, 6), (6, 67, 6)]),
    bar_split(
        38,
        [57, 62, 65],
        36,
        [55, 60, 64],
        &[(0, 69, 4), (4, 65, 2), (6, 64, 6)],
    ),
    bar_same(36, [55, 60, 64], &[(0, 60, 12)]),
    // "She once was a true love of mine"
    bar_split(
        38,
        [57, 62, 65],
        41,
        [57, 60, 65],
        &[(0, 62, 6), (6, 69, 6)],
    ),
    bar_split(
        36,
        [55, 60, 64],
        43,
        [55, 59, 62],
        &[(0, 67, 6), (6, 62, 6)],
    ),
    bar_split(
        38,
        [57, 62, 69],
        36,
        [55, 60, 64],
        &[(0, 64, 4), (4, 62, 2), (6, 60, 6)],
    ),
    bar_same(38, [57, 62, 65], &[(0, 62, 12)]),
];

/// Lobby Piece III: Turlough O'Carolan, *Si Bheag, Si Mhor* (1691 Celtic Harp Air, D major, 16 bars).
const SI_BHEAG_SI_MHOR: [SheetBar; 16] = [
    bar_same(38, [57, 62, 66], &[(0, 66, 6), (6, 74, 4), (10, 69, 2)]),
    bar_same(43, [55, 59, 62], &[(0, 71, 6), (6, 67, 6)]),
    bar_split(
        38,
        [57, 62, 66],
        45,
        [57, 61, 64],
        &[(0, 66, 4), (4, 69, 2), (6, 64, 6)],
    ),
    bar_same(38, [57, 62, 66], &[(0, 62, 12)]),
    bar_same(43, [55, 59, 62], &[(0, 67, 4), (4, 71, 2), (6, 74, 6)]),
    bar_same(38, [57, 62, 66], &[(0, 69, 4), (4, 66, 2), (6, 62, 6)]),
    bar_split(
        40,
        [55, 59, 64],
        45,
        [57, 61, 64],
        &[(0, 64, 6), (6, 69, 4), (10, 73, 2)],
    ),
    bar_same(38, [57, 62, 66], &[(0, 74, 12)]),
    // Second strain: soaring D-major harp arch settling home
    bar_same(38, [57, 62, 66], &[(0, 74, 6), (6, 78, 4), (10, 74, 2)]),
    bar_same(43, [55, 59, 62], &[(0, 79, 6), (6, 74, 6)]),
    bar_split(
        38,
        [57, 62, 66],
        47,
        [59, 62, 66],
        &[(0, 78, 4), (4, 74, 2), (6, 71, 6)],
    ),
    bar_same(45, [57, 61, 64], &[(0, 69, 8), (8, 64, 4)]),
    bar_split(
        43,
        [55, 59, 62],
        38,
        [57, 62, 66],
        &[(0, 67, 4), (4, 71, 2), (6, 69, 6)],
    ),
    bar_split(
        40,
        [55, 59, 64],
        45,
        [57, 61, 64],
        &[(0, 67, 4), (4, 64, 2), (6, 61, 6)],
    ),
    bar_split(
        38,
        [57, 62, 66],
        43,
        [55, 59, 62],
        &[(0, 62, 4), (4, 66, 2), (6, 67, 6)],
    ),
    bar_same(38, [57, 62, 66], &[(0, 66, 6), (6, 62, 6)]),
];

/// Lobby Piece IV: Gaspar Sanz, *Españoleta* (*Instrucción de música sobre la guitarra española*, 1674, D minor, 16 bars).
const SANZ_ESPANOLETA: [SheetBar; 16] = [
    bar_same(38, [57, 62, 65], &[(0, 62, 4), (4, 69, 4), (8, 74, 4)]),
    bar_same(38, [57, 62, 65], &[(0, 77, 6), (6, 74, 6)]),
    bar_split(
        43,
        [55, 58, 62],
        36,
        [55, 60, 64],
        &[(0, 70, 4), (4, 67, 2), (6, 72, 6)],
    ),
    bar_same(41, [57, 60, 65], &[(0, 69, 8), (8, 65, 4)]),
    bar_split(
        46,
        [58, 62, 65],
        43,
        [55, 58, 62],
        &[(0, 74, 6), (6, 70, 4), (10, 67, 2)],
    ),
    bar_same(45, [57, 61, 64], &[(0, 69, 6), (6, 61, 6)]),
    bar_split(
        38,
        [57, 62, 65],
        45,
        [57, 61, 64],
        &[(0, 62, 6), (6, 64, 4), (10, 69, 2)],
    ),
    bar_same(38, [57, 62, 65], &[(0, 62, 12)]),
    // Second half: warm Spanish guitar cantabile
    bar_same(41, [57, 60, 65], &[(0, 77, 6), (6, 72, 4), (10, 69, 2)]),
    bar_same(36, [55, 60, 64], &[(0, 76, 6), (6, 72, 6)]),
    bar_split(
        38,
        [57, 62, 65],
        46,
        [58, 62, 65],
        &[(0, 74, 6), (6, 70, 4), (10, 65, 2)],
    ),
    bar_same(45, [57, 61, 64], &[(0, 69, 8), (8, 64, 4)]),
    bar_split(
        38,
        [57, 62, 65],
        36,
        [55, 60, 64],
        &[(0, 74, 6), (6, 72, 4), (10, 67, 2)],
    ),
    bar_split(
        46,
        [58, 62, 65],
        43,
        [55, 58, 62],
        &[(0, 70, 6), (6, 67, 6)],
    ),
    bar_split(
        45,
        [57, 61, 64],
        38,
        [57, 62, 65],
        &[(0, 69, 4), (4, 64, 2), (6, 65, 6)],
    ),
    bar_same(38, [57, 62, 65], &[(0, 62, 12)]),
];

const TITLE_PIECES: [&[SheetBar; 16]; 2] = [&HANDEL_SARABANDE, &DVORAK_LARGO];
const LOBBY_PIECES: [&[SheetBar; 16]; 4] = [
    &GREENSLEEVES,
    &SCARBOROUGH_FAIR,
    &SI_BHEAG_SI_MHOR,
    &SANZ_ESPANOLETA,
];

/// Pre-mark every sampled acoustic `(Instrument, pitch)` pair used by the Login and
/// Lobby suites so `sampled::prepare` builds studio-grade 48 kHz buffers ahead of time.
pub(in crate::music) fn mark_studio_pitches(used: &mut [bool], key: fn(Instrument, u8) -> usize) {
    for suite in &TITLE_PIECES {
        for bar in *suite {
            for &(bass, tones) in &[(bar.bass1, bar.tones1), (bar.bass2, bar.tones2)] {
                used[key(Instrument::Bass, bass)] = true;
                used[key(Instrument::Cello, bass + 12)] = true;
                used[key(Instrument::Trombone, bass + 12)] = true;
                used[key(Instrument::Viola, tones[1])] = true;
                for &t in &tones {
                    used[key(Instrument::Harp, t)] = true;
                }
            }
            for &(_, pitch, _) in bar.melody {
                used[key(Instrument::Harp, pitch)] = true;
                used[key(Instrument::Violin, pitch)] = true;
                used[key(Instrument::Cello, pitch.saturating_sub(12))] = true;
            }
        }
    }
    for suite in &LOBBY_PIECES {
        for bar in *suite {
            for &(bass, tones) in &[(bar.bass1, bar.tones1), (bar.bass2, bar.tones2)] {
                used[key(Instrument::Cello, bass + 12)] = true;
                used[key(Instrument::Viola, tones[1])] = true;
                used[key(Instrument::Harp, tones[2])] = true;
            }
            for &(_, pitch, _) in bar.melody {
                used[key(Instrument::Violin, pitch)] = true;
                used[key(Instrument::Harp, pitch)] = true;
            }
        }
    }
}

#[cfg(test)]
pub(in crate::music) fn audit_memorial_harmony() -> (usize, usize, usize, usize, f64) {
    let mut low_violations = 0usize;
    let mut m2_clashes = 0usize;
    let mut m7_clashes = 0usize;
    let mut tritone_clashes = 0usize;
    let (mut l1_sum, mut l1_cnt) = (0.0_f64, 0usize);

    for suite in TITLE_PIECES.iter().chain(LOBBY_PIECES.iter()) {
        let mut prev_tones: Option<[u8; 3]> = None;
        for bar in *suite {
            for (half, (range, (bass, mut tones))) in [
                (0_u8..bar.split, (bar.bass1, bar.tones1)),
                (bar.split..12_u8, (bar.bass2, bar.tones2)),
            ]
            .into_iter()
            .enumerate()
            {
                if half == 1 && bar.split >= 12 {
                    continue;
                }
                tones.sort_unstable();
                if let Some(prev) = prev_tones
                    && prev != tones
                {
                    l1_sum += prev
                        .iter()
                        .zip(&tones)
                        .map(|(a, b)| f64::from(a.abs_diff(*b)))
                        .sum::<f64>();
                    l1_cnt += 1;
                }
                prev_tones = Some(tones);
                if bass + 12 < 48 || tones[0] < 50 || tones[0].saturating_sub(bass) < 12 {
                    low_violations += 1;
                }
                for sub in range {
                    let mut sounding = vec![bass, tones[0], tones[1], tones[2]];
                    for &(onset, pitch, dur) in bar.melody {
                        if sub >= onset && sub < onset.saturating_add(dur) {
                            sounding.push(pitch);
                        }
                    }
                    sounding.sort_unstable();
                    sounding.dedup();
                    for i in 0..sounding.len() {
                        for j in (i + 1)..sounding.len() {
                            let d = (sounding[j] - sounding[i]) % 12;
                            if d == 1 {
                                m2_clashes += 1;
                            } else if d == 11 {
                                m7_clashes += 1;
                            } else if d == 6 {
                                tritone_clashes += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    (
        low_violations,
        m2_clashes,
        m7_clashes,
        tritone_clashes,
        l1_sum / l1_cnt.max(1) as f64,
    )
}

#[cfg(test)]
pub(in crate::music) fn sheet_melodies() -> Vec<(&'static str, Vec<u8>)> {
    [
        ("Handel Sarabande HWV 437", &HANDEL_SARABANDE),
        ("Dvorak New World Largo", &DVORAK_LARGO),
        ("Greensleeves", &GREENSLEEVES),
        ("Scarborough Fair", &SCARBOROUGH_FAIR),
        ("Si Bheag Si Mhor", &SI_BHEAG_SI_MHOR),
        ("Gaspar Sanz Espanoleta", &SANZ_ESPANOLETA),
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
    /// Step to the next 16-bar tavern piece when returning to the Lobby after a duel.
    pub(super) fn next_lobby_piece(&mut self) {
        if self.lobby_frame > 0 {
            let piece_frames = LOBBY_STEP_FRAMES * 12 * 16;
            self.lobby_frame = ((self.lobby_frame / piece_frames) + 1) * piece_frames;
        }
    }

    /// Schedule notes for either the stately Login Title suite (`Movement::Title`)
    /// or the cozy Acoustic Guitar & Folk Harp Tavern suite (`Movement::Lobby`).
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
                Self::schedule_title_step(orchestra, samples, sheet, sub, rep, bar_in_piece);
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
            Self::schedule_lobby_step(orchestra, samples, sheet, sub, rep, bar_in_piece);
            self.cursor = total_sub + 1;
        }
        let next_step = ((self.lobby_frame / LOBBY_STEP_FRAMES) + 1) * LOBBY_STEP_FRAMES;
        (next_step - self.lobby_frame).clamp(1, LOBBY_STEP_FRAMES)
    }

    fn title_arc(bar_in_piece: usize) -> f32 {
        match bar_in_piece {
            0..=3 => 0.72,
            4..=7 => 0.90,
            8..=11 => 1.08,
            12..=13 => 0.92,
            _ => 0.76,
        }
    }

    fn bar_slice(sheet: SheetBar, sub: u8) -> (bool, u8, [u8; 3], f32) {
        let has_split =
            sheet.split < 12 && (sheet.bass1 != sheet.bass2 || sheet.tones1 != sheet.tones2);
        let first_half = sub < sheet.split;
        let bass = if first_half { sheet.bass1 } else { sheet.bass2 };
        let tones = if first_half {
            sheet.tones1
        } else {
            sheet.tones2
        };
        let seg_end = if first_half && has_split {
            sheet.split
        } else {
            12
        };
        let rem_steps = (f32::from(seg_end - sub) - 0.15).max(0.6);
        (has_split, bass, tones, rem_steps)
    }

    fn schedule_title_step(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        sheet: SheetBar,
        sub: u8,
        rep: usize,
        bar_in_piece: usize,
    ) {
        let arc = Self::title_arc(bar_in_piece);
        let (has_split, bass, tones, rem_steps) = Self::bar_slice(sheet, sub);

        if sub == 0 || (has_split && sub == sheet.split) {
            let hold = TITLE_STEP_SECONDS * rem_steps;
            if sub == 0 || sheet.bass2 != sheet.bass1 {
                orchestra.note_with_samples(
                    Instrument::Bass,
                    bass,
                    hold,
                    0.115 * arc,
                    Touch::at(0.0).attack(0.085).release(0.42),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::Cello,
                    bass + 12,
                    hold,
                    0.110 * arc,
                    Touch::at(0.18).attack(0.075).release(0.40),
                    samples,
                );
            }
            if bar_in_piece >= 4 && (sub == 0 || sheet.tones2 != sheet.tones1) {
                orchestra.note_with_samples(
                    Instrument::Viola,
                    tones[1],
                    hold * 0.95,
                    0.078 * arc,
                    Touch::at(-0.20).attack(0.090).release(0.38).late(0.020),
                    samples,
                );
            }
            if (8..12).contains(&bar_in_piece) && sub == 0 {
                orchestra.note_with_samples(
                    Instrument::Trombone,
                    bass + 12,
                    hold * 0.90,
                    0.068,
                    Touch::at(-0.10).attack(0.085).release(0.36),
                    samples,
                );
            }
        }

        if sub == 0 || sub == 4 || sub == 8 {
            let harp_pitch = match sub {
                0 => tones[0],
                4 => tones[1],
                _ => tones[2],
            };
            orchestra.note_with_samples(
                Instrument::Harp,
                harp_pitch,
                TITLE_STEP_SECONDS * rem_steps.min(5.5),
                (if sub == 4 { 0.148 } else { 0.125 }) * arc,
                Touch::at(-0.32).attack(0.004).release(0.38),
                samples,
            );
            if sub == 4 {
                orchestra.note_with_samples(
                    Instrument::Harp,
                    tones[2],
                    TITLE_STEP_SECONDS * rem_steps.min(5.0),
                    0.118 * arc,
                    Touch::at(-0.26).attack(0.004).release(0.38).late(0.028),
                    samples,
                );
            }
            if sub == 0 && bar_in_piece.is_multiple_of(2) {
                orchestra.note_with_samples(
                    Instrument::ConcertHarp,
                    tones[2],
                    TITLE_STEP_SECONDS * rem_steps.min(5.0),
                    0.072 * arc,
                    Touch::at(-0.20).late(0.022),
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
            bar_in_piece,
            rem_steps,
            arc,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn schedule_title_melody(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        melody: &[(u8, u8, u8)],
        sub: u8,
        rep: usize,
        bar_in_piece: usize,
        rem_steps: f32,
        arc: f32,
    ) {
        for &(onset, pitch, dur_ticks) in melody {
            if onset != sub {
                continue;
            }
            let max_dur = rem_steps * TITLE_STEP_SECONDS;
            let dur = (f32::from(dur_ticks) * TITLE_STEP_SECONDS).min(max_dur);
            if bar_in_piece < 4 {
                orchestra.note_with_samples(
                    Instrument::Harp,
                    pitch,
                    (dur * 1.15).min(max_dur),
                    0.215,
                    Touch::at(-0.24).attack(0.004).release(0.40),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::Cello,
                    pitch - 12,
                    dur * 0.96,
                    if rep == 1 { 0.125 } else { 0.095 },
                    Touch::at(0.18).attack(0.068).release(0.36),
                    samples,
                );
            } else if bar_in_piece < 12 {
                let climax = bar_in_piece >= 8;
                orchestra.note_with_samples(
                    Instrument::Violin,
                    pitch,
                    dur * 0.96,
                    if climax { 0.210 } else { 0.175 },
                    Touch::at(-0.22).attack(0.058).release(0.35),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::Cello,
                    pitch - 12,
                    dur * 0.95,
                    if climax { 0.135 } else { 0.110 },
                    Touch::at(0.18).attack(0.064).release(0.36),
                    samples,
                );
            } else {
                orchestra.note_with_samples(
                    Instrument::Harp,
                    pitch,
                    (dur * 1.15).min(max_dur),
                    0.185 * arc,
                    Touch::at(-0.24).attack(0.004).release(0.40),
                    samples,
                );
                if bar_in_piece <= 14 {
                    orchestra.note_with_samples(
                        Instrument::Violin,
                        pitch,
                        dur * 0.95,
                        0.145 * arc,
                        Touch::at(-0.22).attack(0.065).release(0.36),
                        samples,
                    );
                }
            }
        }
    }

    fn schedule_lobby_step(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        sheet: SheetBar,
        sub: u8,
        rep: usize,
        bar_in_piece: usize,
    ) {
        let (has_split, bass, tones, rem_steps) = Self::bar_slice(sheet, sub);

        if sub == 0 || (has_split && sub == sheet.split) {
            let hold = LOBBY_STEP_SECONDS * rem_steps;
            if sub == 0 || sheet.bass2 != sheet.bass1 {
                orchestra.note_with_samples(
                    Instrument::TavernGuitar,
                    bass,
                    hold,
                    if sub == 0 { 0.185 } else { 0.165 },
                    Touch::at(-0.08),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::Cello,
                    bass + 12,
                    hold * 0.95,
                    0.068,
                    Touch::at(0.20).attack(0.080).release(0.38),
                    samples,
                );
            }
            if rep == 1 && sub == 0 {
                orchestra.note_with_samples(
                    Instrument::Viola,
                    tones[1],
                    hold * 0.92,
                    0.052,
                    Touch::at(0.26).attack(0.090).release(0.36).late(0.024),
                    samples,
                );
            }
        }

        if sub.is_multiple_of(2) {
            let arp_pitch = match sub {
                0 | 6 => tones[0],
                2 | 8 => tones[1],
                _ => tones[2],
            };
            orchestra.note_with_samples(
                Instrument::TavernGuitar,
                arp_pitch,
                LOBBY_STEP_SECONDS * rem_steps.min(4.8),
                if sub == 0 || sub == 6 { 0.115 } else { 0.095 },
                Touch::at(-0.04).late(0.010),
                samples,
            );
            if sub == 0 || sub == 6 {
                orchestra.note_with_samples(
                    Instrument::Harp,
                    tones[2],
                    LOBBY_STEP_SECONDS * rem_steps.min(5.2),
                    0.105,
                    Touch::at(-0.34).attack(0.004).release(0.34).late(0.018),
                    samples,
                );
            }
            if sub == 0 && bar_in_piece.is_multiple_of(4) {
                orchestra.note_with_samples(
                    Instrument::ConcertHarp,
                    tones[1],
                    LOBBY_STEP_SECONDS * rem_steps.min(4.8),
                    0.058,
                    Touch::at(-0.22).late(0.024),
                    samples,
                );
            }
        }
        Self::schedule_lobby_melody(
            orchestra,
            samples,
            sheet.melody,
            sub,
            rep,
            bar_in_piece,
            rem_steps,
        );
    }

    fn schedule_lobby_melody(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        melody: &[(u8, u8, u8)],
        sub: u8,
        rep: usize,
        bar_in_piece: usize,
        rem_steps: f32,
    ) {
        for &(onset, pitch, dur_ticks) in melody {
            if onset != sub {
                continue;
            }
            let max_dur = rem_steps * LOBBY_STEP_SECONDS;
            let dur = (f32::from(dur_ticks) * LOBBY_STEP_SECONDS).min(max_dur);
            orchestra.note_with_samples(
                Instrument::TavernGuitar,
                pitch,
                (dur * 1.12).min(max_dur),
                0.235,
                Touch::at(-0.05).late(0.004),
                samples,
            );
            if rep == 1 && bar_in_piece < 8 {
                orchestra.note_with_samples(
                    Instrument::Violin,
                    pitch,
                    dur * 0.94,
                    0.112,
                    Touch::at(-0.22).attack(0.068).release(0.32).late(0.010),
                    samples,
                );
            } else if rep == 1 {
                orchestra.note_with_samples(
                    Instrument::Harp,
                    pitch,
                    (dur * 1.12).min(max_dur),
                    0.118,
                    Touch::at(-0.28).attack(0.004).release(0.34).late(0.012),
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
