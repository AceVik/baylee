//! The score's notes: pitch names, the pitch sets and every melody, each
//! written for Baylee (`art/music/originality.py` and the `originality` test
//! hold them against the openings of the tunes we must not echo).
//!
//! A melody is a list of bars, a bar a list of `(MIDI pitch, eighths)`; 0 is
//! a rest. A 6/8 bar holds six eighths, a 7/8 bar seven, a 3/4 bar six (a
//! quarter is two).

/// A bar of a melody.
pub(super) type Bar = &'static [(u8, u8)];
/// A melody.
pub(super) type Phrase = &'static [Bar];

// Pitch classes of the finals.
pub(super) const C: u8 = 0;
pub(super) const F: u8 = 5;
pub(super) const G: u8 = 7;
pub(super) const BB: u8 = 10;

// The notes the score writes (MIDI; B♭ is "BB", E♭ "EB").
pub(super) const BB1: u8 = 34;
pub(super) const C2: u8 = 36;
pub(super) const D2: u8 = 38;
pub(super) const EB2: u8 = 39;
pub(super) const F2: u8 = 41;
pub(super) const G2: u8 = 43;
pub(super) const BB2: u8 = 46;
pub(super) const C3: u8 = 48;
pub(super) const D3: u8 = 50;
pub(super) const F3: u8 = 53;
pub(super) const G3: u8 = 55;
pub(super) const A3: u8 = 57;
pub(super) const BB3: u8 = 58;
pub(super) const C4: u8 = 60;
pub(super) const D4: u8 = 62;
pub(super) const EB4: u8 = 63;
pub(super) const F4: u8 = 65;
pub(super) const G4: u8 = 67;
pub(super) const A4: u8 = 69;
pub(super) const BB4: u8 = 70;
pub(super) const C5: u8 = 72;
pub(super) const D5: u8 = 74;
pub(super) const EB5: u8 = 75;
pub(super) const E5: u8 = 76;
pub(super) const F5: u8 = 77;
pub(super) const G5: u8 = 79;
pub(super) const BB5: u8 = 82;

/// Whether `pitch` is in the B♭ set (B♭ C D E♭ F G A), or with `lydian` in
/// B♭ Lydian's (E♮ for E♭).
pub(super) const fn set_holds(lydian: bool, pitch: u8) -> bool {
    match pitch % 12 {
        10 | 0 | 2 | 5 | 7 | 9 => true,
        3 => !lydian,
        4 => lydian,
        _ => false,
    }
}

// ------------------------------------------------------------- bass lines

/// The table's bass, C Dorian, one note a bar (and its second reading).
pub(super) const CALM_BASS: [u8; 8] = [C2, C2, BB1, C2, EB2, F2, G2, C2];
pub(super) const CALM_BASS_2: [u8; 8] = [C2, BB1, C2, EB2, F2, EB2, G2, C2];
/// The front door's and the lobby's, B♭ Lydian (C is its bright second).
pub(super) const FRONT_BASS: [u8; 8] = [BB1, BB1, C2, BB1, F2, G2, C2, BB1];
/// Tension's, G Aeolian.
pub(super) const TENSION_BASS: [u8; 8] = [G2, G2, F2, G2, D2, G2, BB2, G2];
/// Tension's psaltery, one note an eighth over the 7/8 bar.
pub(super) const TENSION_OSTINATO: [u8; 7] = [G5, G5, BB5, G5, F5, G5, D5];

// ------------------------------------------------- the four themes (C Dorian)

/// A — the ballad: lyrical and singable, in the British folk manner.
pub(super) const BALLAD_A: Phrase = &[
    &[(G4, 2), (C5, 1), (D5, 2), (C5, 1)],
    &[(EB5, 3), (D5, 1), (C5, 2)],
    &[(BB4, 2), (G4, 1), (BB4, 3)],
    &[(C5, 6)],
    &[(G4, 2), (C5, 1), (D5, 2), (EB5, 1)],
    &[(F5, 3), (D5, 1), (EB5, 2)],
    &[(C5, 2), (BB4, 1), (G4, 2), (A4, 1)],
    &[(G4, 6)],
];
/// The ballad's answer, home to C.
pub(super) const BALLAD_B: Phrase = &[
    &[(EB5, 3), (D5, 1), (C5, 2)],
    &[(D5, 2), (BB4, 1), (G4, 3)],
    &[(A4, 2), (C5, 1), (BB4, 2), (D5, 1)],
    &[(EB5, 6)],
    &[(D5, 2), (C5, 1), (BB4, 2), (G4, 1)],
    &[(F4, 3), (G4, 1), (A4, 2)],
    &[(BB4, 3), (D5, 1), (BB4, 2)],
    &[(C5, 6)],
];
/// B — the dance: a driving minor dance in the Slavic manner; bars pulled
/// into three twos (a hemiola) against the 6/8.
pub(super) const DANCE_A: Phrase = &[
    &[(C5, 1), (C5, 1), (D5, 1), (EB5, 2), (D5, 1)],
    &[(C5, 2), (BB4, 2), (G4, 2)],
    &[(G4, 1), (A4, 1), (BB4, 1), (D5, 2), (BB4, 1)],
    &[(A4, 2), (BB4, 2), (F4, 2)],
    &[(G4, 1), (BB4, 1), (G4, 1), (D5, 2), (C5, 1)],
    &[(EB5, 2), (D5, 2), (C5, 2)],
    &[(D5, 1), (C5, 1), (BB4, 1), (A4, 1), (BB4, 1), (G4, 1)],
    &[(C5, 2), (G4, 1), (C5, 3)],
];
/// The dance's second strain.
pub(super) const DANCE_B: Phrase = &[
    &[(EB5, 1), (EB5, 1), (D5, 1), (C5, 2), (D5, 1)],
    &[(EB5, 2), (C5, 2), (EB5, 2)],
    &[(D5, 1), (C5, 1), (BB4, 1), (C5, 2), (D5, 1)],
    &[(G4, 2), (A4, 2), (BB4, 2)],
    &[(C5, 1), (A4, 1), (BB4, 1), (G4, 2), (A4, 1)],
    &[(BB4, 2), (D5, 2), (C5, 2)],
    &[(EB5, 1), (D5, 1), (C5, 1), (BB4, 1), (A4, 1), (BB4, 1)],
    &[(C5, 2), (G4, 1), (C5, 3)],
];
/// C — the epic: broad and heroic, long notes and open leaps, for horns and
/// strings.
pub(super) const EPIC_A: Phrase = &[
    &[(C4, 3), (G4, 3)],
    &[(F4, 2), (EB4, 1), (D4, 3)],
    &[(C4, 2), (D4, 1), (EB4, 3)],
    &[(G4, 6)],
    &[(C5, 3), (BB4, 2), (A4, 1)],
    &[(G4, 3), (F4, 2), (EB4, 1)],
    &[(F4, 2), (G4, 1), (BB4, 2), (A4, 1)],
    &[(G4, 6)],
];
/// The epic's answer, rising to its height.
pub(super) const EPIC_B: Phrase = &[
    &[(C5, 3), (G4, 3)],
    &[(A4, 2), (BB4, 1), (C5, 3)],
    &[(D5, 3), (C5, 2), (BB4, 1)],
    &[(G4, 6)],
    &[(EB5, 3), (D5, 2), (C5, 1)],
    &[(BB4, 2), (A4, 1), (G4, 3)],
    &[(EB4, 2), (F4, 1), (G4, 2), (BB4, 1)],
    &[(C5, 6)],
];
/// D — the jig: a playful medieval jig, running eighths.
pub(super) const JIG_A: Phrase = &[
    &[(C5, 1), (D5, 1), (EB5, 1), (D5, 1), (C5, 1), (BB4, 1)],
    &[(C5, 1), (G4, 1), (G4, 1), (C5, 2), (D5, 1)],
    &[(EB5, 1), (F5, 1), (D5, 1), (EB5, 1), (C5, 1), (D5, 1)],
    &[(BB4, 1), (A4, 1), (G4, 1), (BB4, 3)],
    &[(C5, 1), (EB5, 1), (D5, 1), (F5, 1), (EB5, 1), (D5, 1)],
    &[(C5, 1), (BB4, 1), (C5, 1), (D5, 2), (G4, 1)],
    &[(A4, 1), (BB4, 1), (C5, 1), (A4, 1), (BB4, 1), (G4, 1)],
    &[(C5, 2), (G4, 1), (C5, 3)],
];
/// The jig's second strain.
pub(super) const JIG_B: Phrase = &[
    &[(F5, 1), (EB5, 1), (D5, 1), (EB5, 2), (C5, 1)],
    &[(D5, 1), (C5, 1), (BB4, 1), (C5, 2), (G4, 1)],
    &[(A4, 1), (BB4, 1), (C5, 1), (D5, 1), (EB5, 1), (F5, 1)],
    &[(D5, 3), (BB4, 3)],
    &[(C5, 1), (EB5, 1), (D5, 1), (C5, 1), (BB4, 1), (A4, 1)],
    &[(G4, 1), (BB4, 1), (A4, 1), (C5, 2), (BB4, 1)],
    &[(A4, 1), (G4, 1), (F4, 1), (G4, 1), (A4, 1), (BB4, 1)],
    &[(C5, 6)],
];

// -------------------------------------------------------- the string section

/// The spiccato strings' figures, in scale steps above the bar's root: a
/// 6/8 bar of eighths (3+3), the same bar as three twos (the dance's
/// hemiola), and a 7/8 bar of eighths (3+2+2).
pub(super) const SPIC_SIX: [i8; 6] = [0, 4, 7, 4, 2, 4];
pub(super) const SPIC_HEMIOLA: [i8; 6] = [0, 4, 0, 7, 0, 4];
pub(super) const SPIC_SEVEN: [i8; 7] = [0, 4, 2, 7, 4, 2, 4];

/// The hunting-horn calls, two bars each on the natural horn's notes in F
/// (C3 F3 A3 C4: its third to sixth partials).
pub(super) const CALLS: &[Phrase] = &[
    &[
        &[(C3, 1), (F3, 1), (A3, 1), (C4, 3)],
        &[(A3, 1), (F3, 1), (C4, 1), (F3, 3)],
    ],
    &[
        &[(F3, 2), (F3, 1), (C4, 2), (A3, 1)],
        &[(F3, 1), (A3, 1), (C4, 1), (F3, 3)],
    ],
    &[&[(C4, 3), (A3, 1), (F3, 2)], &[(C3, 2), (F3, 1), (A3, 3)]],
];

/// The horn calls and the bass lines and ostinati, named, for the
/// originality check (the themes' lines come from `themes::every_line`).
#[cfg(test)]
pub(super) const FIGURES: &[(&str, &[u8])] = &[
    ("calm bass", &CALM_BASS),
    ("calm bass 2", &CALM_BASS_2),
    ("front bass", &FRONT_BASS),
    ("tension bass", &TENSION_BASS),
    ("tension ostinato", &TENSION_OSTINATO),
];
