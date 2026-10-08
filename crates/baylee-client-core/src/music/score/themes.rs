//! The four themes the player chooses between (Settings → Audio → Music
//! theme), and how every scene's line is derived from the chosen one.
//!
//! A theme is two eight-bar phrases in C Dorian (the table's mode, 6/8) and a
//! character: a lead instrument, a doubling, a tempo. Every other scene sings
//! the same phrases moved within the one pitch set, so the whole score is one
//! theme heard in many lights: one degree down is B♭ (Lydian at the front
//! door, Ionian for the victory), three down is G Aeolian (tension, climax,
//! defeat), four down is F (the hunt, the draw). Tension re-rhythms each bar
//! as 3+2+2. Each instrument sings in its own register (`Voicing::centre`),
//! the chanter inside its nine notes (`fold`).
use super::melodies::{
    BALLAD_A, BALLAD_B, DANCE_A, DANCE_B, EPIC_A, EPIC_B, JIG_A, JIG_B, Phrase, set_holds,
};
use crate::music::{bank, orchestra::Family};

/// The four themes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Theme {
    /// A: a lyrical ballad in the British folk manner.
    Ballad = 0,
    /// B: a driving minor dance in the Slavic manner, its bars pulled into
    /// twos against the 6/8.
    Dance = 1,
    /// C: broad and heroic, for horns and strings.
    #[default]
    Epic = 2,
    /// D: a playful medieval jig.
    Jig = 3,
}

impl Theme {
    /// Every theme, in the order the settings show them.
    pub const ALL: [Self; 4] = [Self::Ballad, Self::Dance, Self::Epic, Self::Jig];

    /// The theme a request's two bits name.
    #[must_use]
    pub const fn of(bits: u8) -> Self {
        match bits & 3 {
            0 => Self::Ballad,
            1 => Self::Dance,
            3 => Self::Jig,
            _ => Self::Epic,
        }
    }

    /// Its material.
    pub(super) const fn book(self) -> &'static Book {
        match self {
            Self::Ballad => &BALLAD,
            Self::Dance => &DANCE,
            Self::Epic => &EPIC,
            Self::Jig => &JIG,
        }
    }
}

/// A theme's material and character.
pub(super) struct Book {
    /// The two phrases, C Dorian, 6/8, eight bars each.
    pub a: Phrase,
    pub b: Phrase,
    /// Who sings it at the table, and where.
    pub lead: Family,
    pub lead_centre: u8,
    pub lead_gain: f32,
    /// Who doubles it in its second phrase: family, centre, level.
    pub double: (Family, u8, f32),
    /// The tempo of the unhurried scenes, against the score's own (below 1 is
    /// faster).
    pub tempo: f64,
    /// Whether the plucked ostinato groups the bar 2+2+2 (a hemiola) rather
    /// than 3+3.
    pub hemiola: bool,
}

const BALLAD: Book = Book {
    a: BALLAD_A,
    b: BALLAD_B,
    lead: bank::ALTO,
    lead_centre: 71,
    lead_gain: 0.24,
    double: (bank::VIOLIN, 64, 0.4),
    tempo: 1.0,
    hemiola: false,
};
const DANCE: Book = Book {
    a: DANCE_A,
    b: DANCE_B,
    lead: bank::VIOLIN,
    lead_centre: 69,
    lead_gain: 0.34,
    double: (bank::ALTO, 72, 0.5),
    tempo: 0.93,
    hemiola: true,
};
const EPIC: Book = Book {
    a: EPIC_A,
    b: EPIC_B,
    lead: bank::TENOR,
    lead_centre: 66,
    lead_gain: 0.25,
    double: (bank::VIOLINS, 67, 0.55),
    tempo: 1.0,
    hemiola: false,
};
const JIG: Book = Book {
    a: JIG_A,
    b: JIG_B,
    lead: bank::ALTO,
    lead_centre: 72,
    lead_gain: 0.22,
    double: (bank::PSALTERY, 74, 0.35),
    tempo: 0.9,
    hemiola: false,
};

/// How a scene sings a phrase.
#[derive(Clone, Copy, Debug)]
pub(super) struct Voicing {
    /// Scale steps within the set (one degree down: C → B♭).
    pub steps: i8,
    /// Re-rhythm each bar as 3+2+2 (7/8).
    pub seven: bool,
    /// The register the instrument sings in: the phrase is moved by octaves
    /// so its mean lies nearest this.
    pub centre: u8,
    /// Keep every note inside the chanter's F4–G5.
    pub fold: bool,
    /// The pitch class the phrase's last note is drawn to: an ending's final.
    pub close: Option<u8>,
}

impl Voicing {
    pub(super) const fn at(centre: u8) -> Self {
        Self {
            steps: 0,
            seven: false,
            centre,
            fold: false,
            close: None,
        }
    }
    pub(super) const fn steps(mut self, steps: i8) -> Self {
        self.steps = steps;
        self
    }
    pub(super) const fn seven(mut self) -> Self {
        self.seven = true;
        self
    }
    pub(super) const fn fold(mut self) -> Self {
        self.fold = true;
        self
    }
    pub(super) const fn close(mut self, class: u8) -> Self {
        self.close = Some(class);
        self
    }
}

/// `pitch` moved by `steps` scale steps within the set (`lydian`: E♮).
pub(super) fn step_in(pitch: u8, steps: i8, lydian: bool) -> u8 {
    let mut pitch = pitch;
    for _ in 0..steps.unsigned_abs() {
        loop {
            pitch = if steps > 0 { pitch + 1 } else { pitch - 1 };
            if set_holds(lydian, pitch) {
                break;
            }
        }
    }
    pitch
}

/// The chanter's compass: F4 to G5.
const CHANTER: (u8, u8) = (65, 79);

/// The octave a voicing moves a phrase by, so its mean meets the centre.
fn octave(phrase: Phrase, v: Voicing, lydian: bool) -> i16 {
    let (mut sum, mut count) = (0i32, 0i32);
    for bar in phrase {
        for &(pitch, _) in *bar {
            if pitch > 0 {
                sum += i32::from(step_in(pitch, v.steps, lydian));
                count += 1;
            }
        }
    }
    if count == 0 {
        return 0;
    }
    let mean = sum as f32 / count as f32;
    ((f32::from(v.centre) - mean) / 12.0).round() as i16 * 12
}

/// One bar of a phrase as a scene sings it: at most eight notes, no
/// allocation. Rests (pitch 0) are kept.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // MIDI pitches
pub(super) fn voice(
    phrase: Phrase,
    bar: usize,
    v: Voicing,
    lydian: bool,
) -> ([(u8, u8); 8], usize) {
    let mut out = [(0u8, 0u8); 8];
    let source = phrase[bar % phrase.len()];
    let mut n = 0;
    if v.seven {
        // First note, the note sounding at the bar's middle, the last: 3+2+2.
        let mut onset = 0u8;
        let mut middle = source[0].0;
        for &(pitch, eighths) in source {
            if onset <= 3 && onset + eighths > 3 {
                middle = pitch;
            }
            onset += eighths;
        }
        let last = source[source.len() - 1].0;
        for (pitch, eighths) in [(source[0].0, 3), (middle, 2), (last, 2)] {
            out[n] = (pitch, eighths);
            n += 1;
        }
    } else {
        for &note in source.iter().take(8) {
            out[n] = note;
            n += 1;
        }
    }
    let shift = octave(phrase, v, lydian);
    let last_bar = bar % phrase.len() + 1 == phrase.len();
    for (k, note) in out[..n].iter_mut().enumerate() {
        if note.0 == 0 {
            continue;
        }
        let mut pitch = i16::from(step_in(note.0, v.steps, lydian)) + shift;
        if let Some(class) = v.close.filter(|_| last_bar && k + 1 == n) {
            // The nearest pitch of the final's class.
            let up = (i16::from(class) - pitch).rem_euclid(12);
            pitch += if up <= 6 { up } else { up - 12 };
        }
        if v.fold {
            while pitch < i16::from(CHANTER.0) {
                pitch += 12;
            }
            while pitch > i16::from(CHANTER.1) {
                pitch -= 12;
            }
        }
        note.0 = pitch.clamp(24, 100) as u8;
    }
    (out, n)
}

/// Every line the score derives, per theme, for the originality and range
/// tests: (name, pitches).
#[cfg(test)]
pub(super) fn every_line() -> Vec<(String, Vec<u8>)> {
    let mut lines = Vec::new();
    for theme in Theme::ALL {
        let book = theme.book();
        let ways: [(&str, Voicing, bool); 8] = [
            ("calm", Voicing::at(book.lead_centre), false),
            ("front", Voicing::at(book.lead_centre).steps(-1), true),
            ("tension", Voicing::at(66).steps(-3).seven(), false),
            ("chanter", Voicing::at(72).steps(-3).seven().fold(), false),
            ("hunt", Voicing::at(66).steps(-4), false),
            ("climax", Voicing::at(72).steps(-3).fold(), false),
            ("victory", Voicing::at(72).steps(-1).fold().close(10), false),
            ("defeat", Voicing::at(64).steps(-3).close(7), false),
        ];
        for (name, v, lydian) in ways {
            for (half, phrase) in [("A", book.a), ("B", book.b)] {
                let mut pitches = Vec::new();
                for bar in 0..phrase.len() {
                    let (notes, n) = voice(phrase, bar, v, lydian);
                    pitches.extend(notes[..n].iter().filter(|p| p.0 > 0).map(|p| p.0));
                }
                lines.push((format!("{theme:?} {name} {half}"), pitches));
            }
        }
    }
    lines
}
