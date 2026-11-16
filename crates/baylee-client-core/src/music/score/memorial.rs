//! Sheet-music suites for Login (`Movement::Title`) and Lobby / Deckbuilder
//! (`Movement::Lobby`), unified in D minor / D Dorian / F major / D major:
//!
//! ### Login Title Suite (`Movement::Title`) — Epic Baylee Signature Overture:
//! 1. *Baylee Signature Overture* (16-bar dramatic arc anchored by the 8-note Baylee Signature
//!    Leitmotif `D4 -> A4 -> D5 -> F5 -> D5 -> C5 -> E5 -> C5`: Concert Harp signature statement,
//!    French Horn & Cello answer, full Strings/Horns/Timpani sweeping climax, and luminous D-major
//!    revelation bridging seamlessly back to the Concert Harp opening).
//! 2. *Heroic Sarabande Variation* (after George Frideric Handel, *Suite in D minor, HWV 437*, 1733,
//!    woven with the Baylee Signature Motif on Concert Harp, French Horns, Cello and Strings).
//!
//! ### Lobby & Deckbuilder Tavern Suite (`Movement::Lobby`) — Cozy Acoustic Guitar ("Taverne, Gitarre, gemütlich"):
//! 1. Gaspar Sanz (1640–1710), *Españoleta* (*Instrucción de música sobre la guitarra española*, 1674, D minor,
//!    opening with a warm acoustic-guitar echo of the Baylee `D4 -> A4 -> D5` signature call)
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

/// Sixteenth-note pulse for the cozy Lobby Acoustic Guitar suite: 7,500 frames at 48 kHz
/// (`0.15625 s` sixteenth -> `0.625 s` quarter beat [`96.0 BPM` in `3/4`, `75f @ 120fps`] ->
/// `1.875 s` bar [`225f @ 120fps`] -> `15.000 s` exact 8-bar period [`1,800f @ 120fps`,
/// `900f @ 60fps`, `450f @ 30fps`], zero cumulative phase drift across infinite loops).
pub(in crate::music) const LOBBY_STEP_FRAMES: u32 = 7_500;
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

const fn bar_split_at(
    split: u8,
    bass1: u8,
    tones1: [u8; 3],
    bass2: u8,
    tones2: [u8; 3],
    melody: &'static [(u8, u8, u8)],
) -> SheetBar {
    SheetBar {
        split,
        bass1,
        tones1,
        bass2,
        tones2,
        melody,
    }
}

/// Subtle drift-free Just-Intonation sweetening for sustained brass/pad voices in Title.
const fn sheet_sweeten_cents(bass: u8, tones: [u8; 3], pitch: u8) -> f32 {
    let root_pc = bass % 12;
    let rel = (pitch + 120 - root_pc) % 12;
    if rel == 7 {
        return 1.5;
    }
    let has_maj3 = (tones[0] % 12 == (root_pc + 4) % 12)
        || (tones[1] % 12 == (root_pc + 4) % 12)
        || (tones[2] % 12 == (root_pc + 4) % 12);
    let has_min3 = (tones[0] % 12 == (root_pc + 3) % 12)
        || (tones[1] % 12 == (root_pc + 3) % 12)
        || (tones[2] % 12 == (root_pc + 3) % 12);
    if rel == 4 && has_maj3 {
        -3.0
    } else if rel == 3 && has_min3 {
        3.0
    } else {
        0.0
    }
}

/// The 8-note Baylee Signature Leitmotif (`D4 -> A4 -> D5 -> F5 -> D5 -> C5 -> E5 -> C5`),
/// rising through a noble fifth and octave (`+7, +5`) to a poignant minor-tenth crest (`+3`),
/// then stepping through `Bb(add9)` to a luminous lift over `C major` (`-3, -2, +4, -4`).
#[cfg_attr(not(test), allow(dead_code))]
pub(in crate::music) const BAYLEE_SIGNATURE_MOTIF: [u8; 8] = [62, 69, 74, 77, 74, 72, 76, 72];

/// Login Piece I: *Baylee Signature Overture* (16 bars in 3/4, dramatic 4-stage symphonic arc):
/// - Bars 1–4: Crystalline Concert Harp statement of the Baylee Signature Motif over deep `D2` pedal.
/// - Bars 5–8: Noble French Horn & Cello answer in Drop-2 harmony with shimmering harp counter-arpeggios.
/// - Bars 9–12: Full epic sweeping climax (`Bb -> C -> Dm/Bb -> Gm/A`) with soaring Strings, Horns & Timpani.
/// - Bars 13–16: Triumphant `D major` signature revelation resolving luminously back to the Concert Harp loop.
const BAYLEE_SIGNATURE_OVERTURE: [SheetBar; 16] = [
    // Act I (Bars 1–4): Intimate Concert Harp Signature Statement
    bar_same(
        38,
        [53, 57, 62],
        &[(0, 62, 4), (4, 69, 2), (6, 74, 4), (10, 77, 2)],
    ),
    bar_split(
        46,
        [53, 58, 62],
        40,
        [52, 55, 60],
        &[(0, 74, 4), (4, 72, 2), (6, 76, 4), (10, 72, 2)],
    ),
    bar_split(
        41,
        [53, 57, 60],
        43,
        [55, 59, 62],
        &[(0, 69, 4), (4, 72, 2), (6, 74, 4), (10, 71, 2)],
    ),
    bar_same(45, [52, 57, 61], &[(0, 69, 8), (8, 64, 4)]),
    // Act II (Bars 5–8): Noble French Horn & Cello Answer
    bar_same(
        38,
        [53, 57, 62],
        &[(0, 62, 4), (4, 69, 2), (6, 74, 4), (10, 77, 2)],
    ),
    bar_split(
        46,
        [53, 58, 62],
        41,
        [53, 57, 60],
        &[(0, 79, 4), (4, 74, 2), (6, 81, 4), (10, 77, 2)],
    ),
    bar_split(
        43,
        [55, 58, 62],
        45,
        [52, 57, 61],
        &[(0, 79, 4), (4, 74, 2), (6, 76, 4), (10, 73, 2)],
    ),
    bar_split_at(
        8,
        38,
        [53, 57, 62],
        40,
        [52, 55, 60],
        &[(0, 74, 6), (6, 69, 2), (8, 72, 2), (10, 76, 2)],
    ),
    // Act III (Bars 9–12): Full Epic Sweeping Orchestral Climax (Bb -> C -> Dm/Bb -> Gm/A)
    bar_same(
        46,
        [53, 58, 62],
        &[(0, 77, 4), (4, 74, 2), (6, 82, 4), (10, 77, 2)],
    ),
    bar_same(
        36,
        [55, 60, 64],
        &[(0, 79, 4), (4, 76, 2), (6, 81, 4), (10, 79, 2)],
    ),
    bar_split(
        38,
        [53, 57, 62],
        46,
        [53, 58, 62],
        &[(0, 81, 4), (4, 77, 2), (6, 74, 4), (10, 70, 2)],
    ),
    bar_split(
        43,
        [55, 58, 62],
        45,
        [52, 57, 61],
        &[(0, 74, 4), (4, 79, 2), (6, 76, 4), (10, 73, 2)],
    ),
    // Act IV (Bars 13–16): Luminous D-Major Revelation & Seamless Bridge Back to Loop
    bar_same(
        38,
        [54, 57, 62],
        &[(0, 74, 4), (4, 69, 2), (6, 78, 4), (10, 74, 2)],
    ),
    bar_split(
        43,
        [55, 59, 62],
        46,
        [53, 58, 62],
        &[(0, 79, 4), (4, 74, 2), (6, 74, 4), (10, 70, 2)],
    ),
    bar_same(45, [52, 57, 61], &[(0, 69, 6), (6, 73, 4), (10, 69, 2)]),
    bar_same(38, [53, 57, 62], &[(0, 74, 8), (8, 69, 4)]),
];

/// Login Piece II: *Heroic Sarabande Variation* (Handel *HWV 437* harmonic architecture woven
/// with the Baylee Signature Motif, voiced for Concert Harp, French Horns, Cello and Strings).
const BAYLEE_HEROIC_SARABANDE: [SheetBar; 16] = [
    // d -> F -> Bb -> A (opens with Baylee Signature Call)
    bar_same(
        38,
        [53, 57, 62],
        &[(0, 62, 4), (4, 69, 2), (6, 74, 4), (10, 77, 2)],
    ),
    bar_same(41, [53, 57, 60], &[(0, 72, 4), (4, 81, 6), (10, 77, 2)]),
    bar_same(46, [53, 58, 62], &[(0, 74, 4), (4, 82, 6), (10, 77, 2)]),
    bar_same(45, [52, 57, 61], &[(0, 76, 4), (4, 73, 6), (10, 69, 2)]),
    // d -> F -> C/E -> F (Horn & Cello answer)
    bar_same(38, [53, 57, 62], &[(0, 74, 4), (4, 69, 4), (8, 65, 4)]),
    bar_same(41, [53, 57, 60], &[(0, 69, 4), (4, 72, 6), (10, 77, 2)]),
    bar_same(40, [52, 55, 60], &[(0, 76, 4), (4, 79, 6), (10, 72, 2)]),
    bar_same(41, [53, 57, 60], &[(0, 77, 8), (8, 72, 4)]),
    // d -> g -> C -> F (Epic climax)
    bar_same(38, [53, 57, 62], &[(0, 81, 4), (4, 77, 6), (10, 74, 2)]),
    bar_same(43, [55, 58, 62], &[(0, 79, 4), (4, 74, 6), (10, 70, 2)]),
    bar_same(36, [55, 60, 64], &[(0, 76, 4), (4, 79, 6), (10, 72, 2)]),
    bar_same(41, [53, 57, 60], &[(0, 69, 4), (4, 77, 6), (10, 72, 2)]),
    // Bb -> g -> A -> d (Graceful Concert Harp bridge)
    bar_same(46, [53, 58, 62], &[(0, 74, 4), (4, 70, 4), (8, 65, 4)]),
    bar_same(43, [55, 58, 62], &[(0, 67, 4), (4, 74, 6), (10, 70, 2)]),
    bar_same(45, [52, 57, 61], &[(0, 69, 4), (4, 73, 6), (10, 76, 2)]),
    bar_same(38, [53, 57, 62], &[(0, 74, 8), (8, 69, 4)]),
];

/// Lobby Piece I: Gaspar Sanz *Españoleta* (1674 — Spanish baroque guitar air in D minor, 16 bars,
/// opening with a warm Tavern Guitar echo of the `D4 -> A4 -> D5` Baylee Signature Call).
const SANZ_ESPANOLETA: [SheetBar; 16] = [
    bar_split_at(
        8,
        38,
        [50, 57, 62],
        45,
        [52, 57, 61],
        &[(0, 62, 3), (3, 69, 1), (4, 74, 4), (8, 73, 4)],
    ),
    bar_same(38, [50, 57, 62], &[(0, 74, 4), (4, 76, 4), (8, 77, 4)]),
    bar_split(
        36,
        [48, 55, 60],
        41,
        [53, 57, 60],
        &[(0, 76, 6), (6, 72, 4), (10, 69, 2)],
    ),
    bar_split_at(
        8,
        41,
        [53, 57, 60],
        36,
        [48, 55, 60],
        &[(0, 77, 8), (8, 76, 4)],
    ),
    bar_split_at(
        8,
        46,
        [53, 58, 62],
        43,
        [55, 58, 62],
        &[(0, 74, 4), (4, 70, 4), (8, 67, 4)],
    ),
    bar_same(45, [52, 57, 61], &[(0, 69, 6), (6, 64, 6)]),
    bar_split_at(
        8,
        38,
        [50, 57, 62],
        45,
        [52, 57, 61],
        &[(0, 65, 4), (4, 69, 4), (8, 73, 4)],
    ),
    bar_same(38, [50, 57, 62], &[(0, 74, 12)]),
    // Second period (Sanz variation in thirds and sixths)
    bar_split_at(
        8,
        41,
        [53, 57, 60],
        36,
        [48, 55, 60],
        &[(0, 81, 4), (4, 77, 4), (8, 76, 4)],
    ),
    bar_split_at(
        4,
        38,
        [50, 57, 62],
        45,
        [52, 57, 60],
        &[(0, 74, 4), (4, 72, 4), (8, 69, 4)],
    ),
    bar_split_at(
        8,
        46,
        [53, 58, 62],
        41,
        [53, 57, 60],
        &[(0, 70, 4), (4, 74, 4), (8, 72, 4)],
    ),
    bar_same(36, [48, 55, 60], &[(0, 67, 8), (8, 72, 4)]),
    bar_split_at(
        8,
        38,
        [50, 57, 62],
        43,
        [50, 55, 58],
        &[(0, 74, 4), (4, 77, 4), (8, 70, 4)],
    ),
    bar_split_at(
        8,
        38,
        [50, 57, 62],
        45,
        [52, 57, 61],
        &[(0, 69, 4), (4, 65, 4), (8, 64, 4)],
    ),
    bar_split_at(
        8,
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
    bar_same(38, [50, 57, 62], &[(0, 78, 4), (4, 76, 4), (8, 74, 4)]),
    bar_split(
        43,
        [50, 55, 59],
        38,
        [50, 57, 66],
        &[(0, 71, 6), (6, 74, 6)],
    ),
    bar_split_at(
        4,
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
    bar_same(43, [55, 58, 62], &[(0, 67, 6), (6, 70, 6)]),
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
    bar_same(43, [55, 58, 62], &[(0, 67, 8), (8, 70, 4)]),
    bar_same(45, [52, 57, 61], &[(0, 69, 4), (4, 64, 4), (8, 69, 4)]),
    bar_same(38, [50, 57, 62], &[(0, 62, 12)]),
];

/// Lobby Piece IV: J.S. Bach *Bourrée BWV 996* & *Prelude BWV 999* (c. 1712–1720, D minor, 16 bars).
const BACH_BOURREE_BWV996: [SheetBar; 16] = [
    bar_split_at(
        9,
        38,
        [50, 57, 62],
        45,
        [52, 57, 61],
        &[(0, 74, 3), (3, 76, 3), (6, 77, 3), (9, 73, 3)],
    ),
    bar_split(
        38,
        [50, 57, 62],
        45,
        [52, 57, 60],
        &[(0, 74, 6), (6, 69, 3), (9, 72, 3)],
    ),
    bar_split_at(
        3,
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
    bar_split_at(
        3,
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
        [55, 58, 62],
        &[(0, 70, 3), (3, 67, 3), (6, 74, 3), (9, 70, 3)],
    ),
    bar_split(
        38,
        [50, 57, 62],
        45,
        [52, 57, 61],
        &[(0, 69, 3), (3, 65, 3), (6, 64, 3), (9, 61, 3)],
    ),
    bar_same(38, [50, 57, 62], &[(0, 62, 12)]),
];

const TITLE_PIECES: [[SheetBar; 16]; 2] = [BAYLEE_SIGNATURE_OVERTURE, BAYLEE_HEROIC_SARABANDE];
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
                used[key(Instrument::Cello, pitch - 12)] = true;
                used[key(Instrument::Trombone, pitch - 12)] = true;
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
pub(in crate::music) fn audit_memorial_harmony() -> (usize, usize, usize, usize, f64) {
    let mut low_violations = 0usize;
    let mut m2_clashes = 0usize;
    let mut maj7_clashes = 0usize;
    let mut tritone_clashes = 0usize;
    let mut l1_sum = 0.0_f64;
    let mut l1_steps = 0usize;

    for pieces_and_mode in [(&TITLE_PIECES[..], true), (&LOBBY_PIECES[..], false)] {
        let (pieces, is_title) = pieces_and_mode;
        for piece in pieces {
            let mut prev_tones: Option<[u8; 3]> = None;
            for rep in 0..2_usize {
                let mut active: Vec<(f32, f32, u8)> = Vec::new();
                for (b_idx, bar) in piece.iter().enumerate() {
                    for tones in [bar.tones1, bar.tones2] {
                        if let Some(prev) = prev_tones
                            && prev != tones
                        {
                            let d: usize = prev
                                .iter()
                                .zip(tones.iter())
                                .map(|(a, b)| usize::from(a.abs_diff(*b)))
                                .sum();
                            l1_sum += d as f64;
                            l1_steps += 1;
                        }
                        prev_tones = Some(tones);
                    }
                    collect_bar_notes(&mut active, *bar, b_idx, rep, is_title);
                    for sub in 0..12_u8 {
                        let now = (b_idx * 12) as f32 + f32::from(sub) + 0.1;
                        let mut sounding: Vec<u8> = active
                            .iter()
                            .filter(|&&(st, en, _)| st <= now && now < en)
                            .map(|&(_, _, p)| p)
                            .collect();
                        sounding.sort_unstable();
                        sounding.dedup();
                        for i in 0..sounding.len() {
                            for j in (i + 1)..sounding.len() {
                                let diff = sounding[j] - sounding[i];
                                if sounding[i] < 50 && diff < 7 {
                                    low_violations += 1;
                                }
                                if diff % 12 == 1 {
                                    m2_clashes += 1;
                                } else if diff % 12 == 11 {
                                    maj7_clashes += 1;
                                } else if diff % 12 == 6 {
                                    tritone_clashes += 1;
                                }
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
        maj7_clashes,
        tritone_clashes,
        l1_sum / l1_steps.max(1) as f64,
    )
}

#[cfg(test)]
fn collect_bar_notes(
    active: &mut Vec<(f32, f32, u8)>,
    bar: SheetBar,
    b_idx: usize,
    rep: usize,
    is_title: bool,
) {
    let base = (b_idx * 12) as f32;
    let has_split = bar.split < 12 && (bar.bass1 != bar.bass2 || bar.tones1 != bar.tones2);
    let split_f = f32::from(bar.split);
    let hold1 = if has_split { split_f - 0.2 } else { 11.6 };
    active.push((base, base + hold1, bar.bass1));
    active.push((base, base + hold1, bar.bass1 + 12));
    if is_title {
        active.push((base, base + hold1, bar.tones1[1]));
        active.push((base, base + hold1, bar.tones1[2]));
    } else if rep == 1 {
        active.push((base, base + hold1, bar.tones1[1]));
    }
    if has_split {
        let hold2 = (12.0 - split_f) - 0.2;
        active.push((base + split_f, base + split_f + hold2, bar.bass2));
        active.push((base + split_f, base + split_f + hold2, bar.bass2 + 12));
        if is_title {
            active.push((base + split_f, base + split_f + hold2, bar.tones2[1]));
            active.push((base + split_f, base + split_f + hold2, bar.tones2[2]));
        } else if rep == 1 {
            active.push((base + split_f, base + split_f + hold2, bar.tones2[1]));
        }
    }
    for sub in 0..12_u8 {
        let first = sub < bar.split;
        let tones = if first { bar.tones1 } else { bar.tones2 };
        let seg_end = if first && has_split { bar.split } else { 12 };
        let rem = (f32::from(seg_end - sub) - 0.18).max(0.5);
        let s = base + f32::from(sub);
        if is_title && sub.is_multiple_of(2) {
            let arp = if (8..12).contains(&b_idx) && sub == 6 {
                tones[0] + 12
            } else {
                [tones[0], tones[1], tones[2], tones[1], tones[0], tones[2]][(sub / 2) as usize]
            };
            active.push((s, s + rem.min(4.8), arp));
            if b_idx == 0 && sub == 0 {
                for t in tones {
                    active.push((s, s + rem.min(5.4), t));
                }
            }
        } else if !is_title && sub.is_multiple_of(2) {
            let arp = match sub {
                0 | 6 => tones[0],
                2 | 8 => tones[1],
                _ => tones[2],
            };
            active.push((s, s + rem.min(4.8), arp));
            if sub == 0 || sub == 6 {
                active.push((s, s + rem.min(5.4), tones[2]));
            }
        }
        for &(onset, mp, dur) in bar.melody {
            if onset == sub {
                let mdur = f32::from(dur).min(rem);
                active.push((s, s + mdur, mp));
                if is_title {
                    active.push((s, s + mdur, mp - 12));
                }
            }
        }
    }
}

#[cfg(test)]
pub(in crate::music) fn sheet_melodies() -> Vec<(&'static str, Vec<u8>)> {
    [
        ("Baylee Signature Overture", &BAYLEE_SIGNATURE_OVERTURE),
        (
            "Baylee Heroic Sarabande Variation",
            &BAYLEE_HEROIC_SARABANDE,
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

    /// Four-stage dramatic dynamic envelope for the 16-bar Login Overture:
    /// Act I (`0..4`) intimate Concert Harp signature -> Act II (`4..8`) Horn & Cello answer ->
    /// Act III (`8..12`) full epic sweeping climax -> Act IV (`12..16`) luminous bridge to loop.
    fn title_arc(bar_in_piece: usize) -> f32 {
        match bar_in_piece {
            4..=7 => 0.98,
            8..=11 => 1.18,
            12 => 1.02,
            13 => 0.86,
            14 => 0.72,
            _ => 0.56,
        }
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
        let arc = Self::title_arc(bar_in_piece);
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
        let rem_steps = (f32::from(seg_end - sub) - 0.18).max(0.55);

        if sub == 0 || (has_split && sub == sheet.split) {
            Self::schedule_title_foundation(
                orchestra,
                samples,
                sheet,
                sub,
                bass,
                tones,
                rem_steps,
                bar_in_piece,
                arc,
            );
        }
        if (8..12).contains(&bar_in_piece) && bar_in_piece == 11 && sub == 6 {
            orchestra.note_with_samples(
                Instrument::Timpani,
                bass,
                TITLE_STEP_SECONDS * 4.5,
                0.115,
                Touch::at(-0.12),
                samples,
            );
        }

        // Cascading Sitka-spruce Concert Harp arpeggios across every bar
        if sub.is_multiple_of(2) {
            let slot = (sub / 2) as usize;
            let arp_pitch = if (8..12).contains(&bar_in_piece) && sub == 6 {
                tones[0] + 12
            } else {
                [tones[0], tones[1], tones[2], tones[1], tones[0], tones[2]][slot]
            };
            orchestra.note_with_samples(
                Instrument::ConcertHarp,
                arp_pitch,
                TITLE_STEP_SECONDS * rem_steps.min(4.8),
                (if sub.is_multiple_of(6) { 0.165 } else { 0.132 }) * arc,
                Touch::at(-0.22).late(0.006),
                samples,
            );
            if sub == 0 || sub == 6 {
                orchestra.note_with_samples(
                    Instrument::Harp,
                    tones[if sub == 0 { 0 } else { 2 }],
                    TITLE_STEP_SECONDS * rem_steps.min(5.2),
                    0.115 * arc,
                    Touch::at(-0.36).attack(0.004).release(0.30).late(0.012),
                    samples,
                );
            }
            if (4..12).contains(&bar_in_piece) && (sub == 0 || sub == 4) {
                orchestra.note_with_samples(
                    Instrument::TavernGuitar,
                    tones[(sub / 4) as usize],
                    TITLE_STEP_SECONDS * rem_steps.min(4.6),
                    0.105 * arc,
                    Touch::at(-0.08).late(0.014),
                    samples,
                );
            }
        }
        Self::schedule_title_melody(
            orchestra,
            samples,
            sheet,
            sub,
            rem_steps,
            rep,
            piece_idx,
            bar_in_piece,
            arc,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn schedule_title_foundation(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        sheet: SheetBar,
        sub: u8,
        bass: u8,
        tones: [u8; 3],
        rem_steps: f32,
        bar_in_piece: usize,
        arc: f32,
    ) {
        let hold = TITLE_STEP_SECONDS * rem_steps;
        let b_gain = (if sub == 0 { 0.125 } else { 0.115 }) * arc;
        if sub == 0 || sheet.bass2 != sheet.bass1 {
            orchestra.note_with_samples(
                Instrument::Bass,
                bass,
                hold,
                b_gain,
                Touch::at(0.0).attack(0.065).release(0.36),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Cello,
                bass + 12,
                hold,
                b_gain * 0.96,
                Touch::at(0.16).attack(0.060).release(0.34),
                samples,
            );
            if (8..12).contains(&bar_in_piece) && sub == 0 {
                orchestra.note_with_samples(
                    Instrument::Timpani,
                    bass,
                    TITLE_STEP_SECONDS * 5.6,
                    0.145,
                    Touch::at(-0.12),
                    samples,
                );
            } else if bar_in_piece == 0 && sub == 0 {
                orchestra.note_with_samples(
                    Instrument::Timpani,
                    bass,
                    TITLE_STEP_SECONDS * 5.4,
                    0.052,
                    Touch::at(-0.12),
                    samples,
                );
                for (k, &t) in tones.iter().enumerate() {
                    orchestra.note_with_samples(
                        Instrument::ConcertHarp,
                        t,
                        TITLE_STEP_SECONDS * 5.2,
                        0.082,
                        Touch::at(-0.24).late(0.020 * k as f32),
                        samples,
                    );
                }
            }
        }
        if sub == 0 || sheet.tones2 != sheet.tones1 {
            let pad_c = sheet_sweeten_cents(bass, tones, tones[1]);
            orchestra.note_with_samples(
                Instrument::StringPad,
                tones[1],
                hold,
                0.068 * arc,
                Touch::at(-0.22).cents(pad_c),
                samples,
            );
            if (4..15).contains(&bar_in_piece) {
                orchestra.note_with_samples(
                    Instrument::Viola,
                    tones[2],
                    hold * 0.96,
                    0.080 * arc,
                    Touch::at(0.24).attack(0.075).release(0.34).late(0.030),
                    samples,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn schedule_title_melody(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        sheet: SheetBar,
        sub: u8,
        rem_steps: f32,
        rep: usize,
        piece_idx: usize,
        bar_in_piece: usize,
        arc: f32,
    ) {
        let first = sub < sheet.split;
        let bass = if first { sheet.bass1 } else { sheet.bass2 };
        let tones = if first { sheet.tones1 } else { sheet.tones2 };
        for &(onset, pitch, dur_ticks) in sheet.melody {
            if onset != sub {
                continue;
            }
            let max_dur = rem_steps * TITLE_STEP_SECONDS;
            let dur = (f32::from(dur_ticks) * TITLE_STEP_SECONDS).min(max_dur);
            let horn_low_c = sheet_sweeten_cents(bass, tones, pitch - 12);
            let horn_hi_c = sheet_sweeten_cents(bass, tones, pitch);
            let harp_lead = if bar_in_piece < 4 || bar_in_piece == 15 {
                0.175
            } else {
                0.155 * arc
            };
            orchestra.note_with_samples(
                Instrument::ConcertHarp,
                pitch,
                (dur * 1.15).min(max_dur),
                harp_lead,
                Touch::at(-0.20),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Harp,
                pitch,
                (dur * 1.10).min(max_dur),
                0.105 * arc,
                Touch::at(-0.32).attack(0.004).release(0.30).late(0.012),
                samples,
            );
            if bar_in_piece < 4 {
                // Act I: Crystalline Concert Harp leads; soft horn fifth halo adds mystery.
                let halo = if rep == 1 || piece_idx == 1 {
                    0.082
                } else {
                    0.052
                };
                orchestra.note_with_samples(
                    Instrument::FrenchHorn,
                    pitch - 12,
                    dur.min(max_dur),
                    halo,
                    Touch::at(0.14).late(0.015).cents(horn_low_c),
                    samples,
                );
            } else if bar_in_piece < 8 {
                // Act II: Noble French Horns & Cello take up the Baylee Signature Motif.
                orchestra.note_with_samples(
                    Instrument::FrenchHorn,
                    pitch - 12,
                    (dur * 1.02).min(max_dur),
                    0.210,
                    Touch::at(0.14).cents(horn_low_c),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::FrenchHorn,
                    pitch,
                    dur.min(max_dur),
                    0.115,
                    Touch::at(0.10).late(0.010).cents(horn_hi_c),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::Cello,
                    pitch - 12,
                    dur * 0.96,
                    0.175,
                    Touch::at(0.18).attack(0.055).release(0.32),
                    samples,
                );
                if bar_in_piece >= 6 || rep == 1 {
                    orchestra.note_with_samples(
                        Instrument::Violin,
                        pitch,
                        dur * 0.94,
                        0.125,
                        Touch::at(-0.24).attack(0.055).release(0.30).late(0.008),
                        samples,
                    );
                }
            } else {
                Self::schedule_title_climax_and_bridge(
                    orchestra,
                    samples,
                    onset,
                    pitch,
                    dur,
                    max_dur,
                    horn_low_c,
                    horn_hi_c,
                    bar_in_piece,
                    arc,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn schedule_title_climax_and_bridge(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        onset: u8,
        pitch: u8,
        dur: f32,
        max_dur: f32,
        horn_low_c: f32,
        horn_hi_c: f32,
        bar_in_piece: usize,
        arc: f32,
    ) {
        if bar_in_piece < 12 {
            // Act III: Full Epic Sweeping Climax — soaring Strings + Horns + Trombone + Celesta!
            orchestra.note_with_samples(
                Instrument::Violin,
                pitch,
                dur * 0.97,
                0.220,
                Touch::at(-0.24).attack(0.048).release(0.32),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::FrenchHorn,
                pitch - 12,
                (dur * 1.03).min(max_dur),
                0.190,
                Touch::at(0.14).cents(horn_low_c),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::FrenchHorn,
                pitch,
                dur.min(max_dur),
                0.105,
                Touch::at(0.10).late(0.008).cents(horn_hi_c),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Cello,
                pitch - 12,
                dur * 0.96,
                0.140,
                Touch::at(0.18).attack(0.050).release(0.32),
                samples,
            );
            if onset == 0 || onset == 6 {
                orchestra.note_with_samples(
                    Instrument::Trombone,
                    pitch - 12,
                    dur * 0.92,
                    0.120,
                    Touch::at(-0.08).attack(0.045).release(0.30),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::Celesta,
                    pitch.min(86),
                    (TITLE_STEP_SECONDS * 4.8).min(max_dur),
                    0.082,
                    Touch::at(0.28).late(0.016),
                    samples,
                );
            }
        } else {
            // Act IV: Triumphant D-major revelation settling smoothly into the Concert Harp loop.
            orchestra.note_with_samples(
                Instrument::FrenchHorn,
                pitch - 12,
                dur.min(max_dur),
                0.145 * arc,
                Touch::at(0.14).cents(horn_low_c),
                samples,
            );
            if bar_in_piece <= 13 {
                orchestra.note_with_samples(
                    Instrument::Violin,
                    pitch,
                    dur * 0.95,
                    0.155 * arc,
                    Touch::at(-0.24).attack(0.055).release(0.30),
                    samples,
                );
            }
            if bar_in_piece <= 14 {
                orchestra.note_with_samples(
                    Instrument::Cello,
                    pitch - 12,
                    dur * 0.95,
                    0.125 * arc,
                    Touch::at(0.18).attack(0.055).release(0.32),
                    samples,
                );
            }
            if bar_in_piece == 12 && onset == 0 {
                orchestra.note_with_samples(
                    Instrument::Celesta,
                    pitch.min(86),
                    (TITLE_STEP_SECONDS * 5.0).min(max_dur),
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
        let rem_steps = (f32::from(seg_end - sub) - 0.18).max(0.55);

        // 1. Warm thumb-plucked Drop-D Acoustic Tavern Guitar bass + soft Cello hearth root
        if sub == 0 || (has_split && sub == sheet.split) {
            let hold = LOBBY_STEP_SECONDS * rem_steps;
            if sub == 0 || sheet.bass2 != sheet.bass1 {
                orchestra.note_with_samples(
                    Instrument::TavernGuitar,
                    bass,
                    hold,
                    if sub == 0 { 0.235 } else { 0.22 },
                    Touch::at(-0.08),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::Cello,
                    bass + 12,
                    hold,
                    if sub == 0 { 0.095 } else { 0.088 },
                    Touch::at(0.18).attack(0.065).release(0.32),
                    samples,
                );
            }
            if rep == 1 && (sub == 0 || sheet.tones2 != sheet.tones1) {
                orchestra.note_with_samples(
                    Instrument::Viola,
                    tones[1],
                    hold * 0.94,
                    0.072,
                    Touch::at(0.26).attack(0.075).release(0.32).late(0.025),
                    samples,
                );
            }
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
                LOBBY_STEP_SECONDS * rem_steps.min(4.6),
                if sub == 0 || sub == 6 { 0.145 } else { 0.13 },
                Touch::at(-0.04).late(0.010),
                samples,
            );
            if sub == 0 || sub == 6 {
                orchestra.note_with_samples(
                    Instrument::ConcertHarp,
                    tones[2],
                    LOBBY_STEP_SECONDS * rem_steps.min(5.2),
                    0.115,
                    Touch::at(-0.22).late(0.012),
                    samples,
                );
                orchestra.note_with_samples(
                    Instrument::Harp,
                    tones[2],
                    LOBBY_STEP_SECONDS * rem_steps.min(5.2),
                    0.105,
                    Touch::at(-0.36).attack(0.004).release(0.28).late(0.020),
                    samples,
                );
            } else if sub == 4 && rep == 1 {
                orchestra.note_with_samples(
                    Instrument::Zither,
                    tones[1],
                    LOBBY_STEP_SECONDS * rem_steps.min(4.0),
                    0.085,
                    Touch::at(0.34).attack(0.004).release(0.26).late(0.018),
                    samples,
                );
            }
        }
        Self::schedule_lobby_melody(
            orchestra,
            samples,
            sheet.melody,
            sub,
            rem_steps,
            rep,
            bar_in_piece,
        );
    }

    // 3. Singing nylon/gut Acoustic Tavern Guitar melody (+ gentle Concert Harp/Lute/Violin warmth)
    fn schedule_lobby_melody(
        orchestra: &mut Orchestra,
        samples: SampleSet,
        melody: &[(u8, u8, u8)],
        sub: u8,
        rem_steps: f32,
        rep: usize,
        bar_in_piece: usize,
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
                0.295,
                Touch::at(-0.05).late(0.004),
                samples,
            );
            orchestra.note_with_samples(
                Instrument::Lyre,
                pitch,
                (dur * 1.06).min(max_dur),
                0.135,
                Touch::at(0.12).late(0.012),
                samples,
            );
            if sub == 0 || sub == 6 {
                orchestra.note_with_samples(
                    Instrument::Harp,
                    pitch,
                    (dur * 1.15).min(max_dur),
                    0.12,
                    Touch::at(-0.28).attack(0.004).release(0.28).late(0.015),
                    samples,
                );
            }
            if rep == 1 && bar_in_piece >= 8 {
                orchestra.note_with_samples(
                    Instrument::ConcertHarp,
                    pitch,
                    (dur * 1.10).min(max_dur),
                    0.145,
                    Touch::at(-0.20).late(0.010),
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
