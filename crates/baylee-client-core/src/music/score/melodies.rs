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
pub(super) const A5: u8 = 81;
pub(super) const BB5: u8 = 82;
pub(super) const C6: u8 = 84;

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

// ----------------------------------------------------------------- melodies

/// The table's theme, C Dorian, first phrase (6/8).
pub(super) const CALM_A: Phrase = &[
    &[(C5, 3), (EB5, 1), (D5, 2)],
    &[(C5, 2), (BB4, 1), (G4, 3)],
    &[(BB4, 2), (C5, 1), (D5, 3)],
    &[(C5, 6)],
    &[(EB5, 3), (F5, 1), (G5, 2)],
    &[(F5, 2), (EB5, 1), (D5, 3)],
    &[(C5, 2), (D5, 1), (BB4, 2), (G4, 1)],
    &[(C5, 6)],
];
/// Its second phrase.
pub(super) const CALM_B: Phrase = &[
    &[(G5, 3), (F5, 1), (EB5, 2)],
    &[(D5, 2), (EB5, 1), (C5, 3)],
    &[(BB4, 2), (G4, 1), (BB4, 3)],
    &[(C5, 3), (BB4, 1), (D5, 2)],
    &[(EB5, 3), (D5, 1), (C5, 2)],
    &[(BB4, 2), (C5, 1), (D5, 3)],
    &[(C5, 2), (G4, 1), (A4, 2), (BB4, 1)],
    &[(C5, 6)],
];
/// The first phrase varied, for the cycle's second half.
pub(super) const CALM_A2: Phrase = &[
    &[(C5, 2), (D5, 1), (EB5, 3)],
    &[(D5, 2), (C5, 1), (G4, 3)],
    &[(BB4, 3), (C5, 1), (D5, 2)],
    &[(EB5, 3), (D5, 3)],
    &[(F5, 2), (G5, 1), (A5, 3)],
    &[(G5, 2), (F5, 1), (EB5, 3)],
    &[(D5, 2), (C5, 1), (BB4, 2), (D5, 1)],
    &[(C5, 6)],
];
/// The second phrase varied.
pub(super) const CALM_B2: Phrase = &[
    &[(G5, 3), (EB5, 1), (F5, 2)],
    &[(D5, 3), (C5, 3)],
    &[(BB4, 2), (D5, 1), (C5, 2), (EB5, 1)],
    &[(F5, 6)],
    &[(EB5, 3), (D5, 1), (C5, 2)],
    &[(BB4, 2), (G4, 1), (BB4, 3)],
    &[(C5, 2), (D5, 1), (BB4, 2), (A4, 1)],
    &[(C5, 6)],
];
/// The front door's theme, B♭ Lydian: the table's theme heard in the
/// geode's light (its shape on the Lydian degrees; E♮ is the light).
pub(super) const FRONT_A: Phrase = &[
    &[(BB4, 3), (D5, 1), (C5, 2)],
    &[(BB4, 2), (A4, 1), (F4, 3)],
    &[(A4, 2), (BB4, 1), (C5, 3)],
    &[(BB4, 6)],
    &[(D5, 3), (E5, 1), (F5, 2)],
    &[(E5, 2), (D5, 1), (C5, 3)],
    &[(BB4, 2), (C5, 1), (A4, 2), (F4, 1)],
    &[(BB4, 6)],
];
/// Its answer.
pub(super) const FRONT_B: Phrase = &[
    &[(F5, 3), (E5, 1), (D5, 2)],
    &[(C5, 2), (D5, 1), (BB4, 3)],
    &[(A4, 2), (F4, 1), (A4, 3)],
    &[(BB4, 3), (A4, 1), (C5, 2)],
    &[(D5, 3), (C5, 1), (BB4, 2)],
    &[(A4, 2), (BB4, 1), (C5, 3)],
    &[(BB4, 2), (F4, 1), (G4, 2), (A4, 1)],
    &[(BB4, 6)],
];
/// Tension's line, G Aeolian in 7/8 (3+2+2), on the tenor recorder.
pub(super) const TENSION_LINE: Phrase = &[
    &[(G4, 3), (BB4, 2), (A4, 2)],
    &[(G4, 3), (F4, 2), (D4, 2)],
    &[(BB4, 3), (C5, 2), (BB4, 2)],
    &[(A4, 5), (G4, 2)],
];
/// The chanter's answer to it, from a tension of 0.55.
pub(super) const TENSION_CHANTER: Phrase = &[
    &[(D5, 3), (C5, 2), (BB4, 2)],
    &[(A4, 3), (BB4, 2), (G4, 2)],
    &[(F4, 3), (G4, 2), (A4, 2)],
    &[(BB4, 3), (A4, 2), (G4, 2)],
];
/// The hunt, F Mixolydian in 6/8, after the horn.
pub(super) const HUNT: Phrase = &[
    &[(F4, 2), (A4, 1), (C5, 2), (A4, 1)],
    &[(BB4, 2), (G4, 1), (C5, 3)],
    &[(D5, 2), (C5, 1), (BB4, 2), (A4, 1)],
    &[(G4, 3), (F4, 3)],
];
/// The climax, G Aeolian, on the chanter over both drones.
pub(super) const CLIMAX: Phrase = &[
    &[(G4, 2), (A4, 1), (BB4, 2), (G4, 1)],
    &[(D5, 3), (C5, 1), (BB4, 2)],
    &[(A4, 2), (G4, 1), (A4, 2), (BB4, 1)],
    &[(G4, 4), (F4, 2)],
    &[(G4, 2), (BB4, 1), (D5, 2), (EB5, 1)],
    &[(D5, 3), (BB4, 1), (A4, 2)],
    &[(BB4, 2), (A4, 1), (G4, 2), (F4, 1)],
    &[(G4, 6)],
];
/// Its variant, every other sixteen bars.
pub(super) const CLIMAX_2: Phrase = &[
    &[(D5, 2), (C5, 1), (BB4, 2), (C5, 1)],
    &[(D5, 3), (G5, 1), (F5, 2)],
    &[(EB5, 2), (D5, 1), (C5, 2), (BB4, 1)],
    &[(A4, 4), (F4, 2)],
    &[(G4, 2), (A4, 1), (BB4, 2), (C5, 1)],
    &[(D5, 2), (EB5, 1), (D5, 2), (C5, 1)],
    &[(BB4, 2), (A4, 1), (F4, 2), (A4, 1)],
    &[(G4, 6)],
];
/// The victory, B♭ Ionian: the rising cadence.
pub(super) const VICTORY: Phrase = &[
    &[(F4, 2), (BB4, 1), (C5, 3)],
    &[(D5, 3), (EB5, 1), (D5, 2)],
    &[(C5, 2), (BB4, 1), (A4, 3)],
    &[(BB4, 6)],
    &[(D5, 3), (F5, 1), (G5, 2)],
    &[(F5, 2), (D5, 1), (EB5, 3)],
    &[(D5, 2), (C5, 1), (A4, 2), (C5, 1)],
    &[(BB4, 6)],
];
/// The draw, over the open fifth F–C: it ends on C, unresolved (3/4).
pub(super) const DRAW: Phrase = &[
    &[(A4, 4), (C5, 2)],
    &[(BB4, 4), (G4, 2)],
    &[(A4, 2), (BB4, 2), (C5, 2)],
    &[(D5, 6)],
    &[(C5, 4), (BB4, 2)],
    &[(A4, 4), (F4, 2)],
    &[(G4, 4), (A4, 2)],
    &[(C5, 6)],
];
/// The defeat, G Aeolian, descending to the final (3/4).
pub(super) const DEFEAT: Phrase = &[
    &[(D5, 4), (BB4, 2)],
    &[(C5, 4), (A4, 2)],
    &[(BB4, 4), (G4, 2)],
    &[(F4, 6)],
    &[(EB4, 4), (F4, 2)],
    &[(G4, 4), (A4, 2)],
    &[(BB4, 4), (A4, 2)],
    &[(G4, 6)],
];

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

/// Every melody, named, for the originality check and the tests.
#[cfg(test)]
pub(super) const MELODIES: &[(&str, Phrase)] = &[
    ("calm A", CALM_A),
    ("calm B", CALM_B),
    ("calm A2", CALM_A2),
    ("calm B2", CALM_B2),
    ("front A", FRONT_A),
    ("front B", FRONT_B),
    ("tension line", TENSION_LINE),
    ("tension chanter", TENSION_CHANTER),
    ("hunt", HUNT),
    ("climax", CLIMAX),
    ("climax 2", CLIMAX_2),
    ("victory", VICTORY),
    ("draw", DRAW),
    ("defeat", DEFEAT),
    ("horn call 1", CALLS[0]),
    ("horn call 2", CALLS[1]),
    ("horn call 3", CALLS[2]),
];
