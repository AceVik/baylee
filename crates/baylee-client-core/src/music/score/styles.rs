//! Ten deliberately separate idioms, sharing only the conductor and note budget.
//! Each owns its tonal route, melody book, pulse, palette and accompaniment.
use super::{
    Movement, Theme,
    arrangement::{self, Chord, Instrument, Notes, Role},
    themes::Pages,
};
mod manuscript;
mod rhythm;

pub(super) struct Profile {
    pub ticks: u8,
    pub bpm: f64,
    harmony: [Chord; 8],
    lead: Instrument,
    comp: Instrument,
    bass: Instrument,
    accent: Instrument,
    pub pages: &'static Pages,
}
pub(super) const fn profile(theme: Theme) -> &'static Profile {
    &manuscript::SCORES[theme as usize - 5]
}
pub(super) fn chord(theme: Theme, movement: Movement, bar: u32) -> Chord {
    let p = profile(theme);
    let mut chord = p.harmony[bar as usize % 8];
    if chord.0 == p.harmony[0].0 {
        chord.1 = match movement {
            Movement::Victory => 4,
            Movement::Defeat => 3,
            Movement::Draw => {
                if bar % 8 < 4 {
                    4
                } else {
                    3
                }
            }
            Movement::Endgame | Movement::Combat if bar % 16 >= 8 => 4,
            _ => chord.1,
        };
    }
    chord
}
fn colour(pitch: u8, written: Chord, played: Chord) -> u8 {
    if pitch % 12 == (written.0 + written.1) % 12 {
        pitch + played.1 - written.1
    } else {
        pitch
    }
}
fn cue(out: &mut Notes, theme: Theme, movement: Movement, tick: u8) {
    if tick >= 4 {
        return;
    }
    let p = profile(theme);
    let root = arrangement::near(p.harmony[0].0, 58);
    let pitches = match movement {
        Movement::Victory => [root, root + 7, root + 16, root + 24],
        Movement::Defeat => [root + 19, root + 15, root + 7, root],
        _ => [root + 12, root + 7, root + 19, root + 12],
    };
    out.add(
        p.lead,
        pitches[usize::from(tick)],
        if tick == 3 {
            f32::from(p.ticks - 3)
        } else {
            0.9
        },
        0.40,
        Role::Melody,
    );
}
fn balance(theme: Theme, out: &mut Notes) {
    let gain = match theme {
        Theme::Velvet | Theme::Circuit => 1.0,
        Theme::Copper => 1.28,
        Theme::Juniper => 0.85,
        Theme::Lagoon | Theme::Orbit => 0.96,
        Theme::Lantern => 0.97,
        Theme::Neon => 0.86,
        Theme::Mosaic => 1.04,
        Theme::Iron => 0.87,
        _ => unreachable!("new palettes only"),
    };
    for note in &mut out.events[..out.len] {
        note.gain *= gain;
    }
}
pub(super) fn notes(theme: Theme, movement: Movement, bar: u32, tick: u8) -> Notes {
    let mut out = Notes {
        events: [arrangement::EMPTY; 24],
        len: 0,
    };
    if movement.ending() && bar == 0 {
        cue(&mut out, theme, movement, tick);
        balance(theme, &mut out);
        return out;
    }
    let local = if movement.ending() { bar - 1 } else { bar };
    let p = profile(theme);
    let harmony = chord(theme, movement, local);
    let arc = [0.88, 0.96, 1.0, 0.93][(local as usize / 8) % 4];
    let quiet = matches!(
        movement,
        Movement::Lobby | Movement::Defeat | Movement::Draw
    );
    if let Some((pitch, duration)) =
        arrangement::onset(arrangement::line(theme, movement, bar), tick)
    {
        let pitch = colour(pitch, p.harmony[local as usize % 8], harmony);
        let lead = if movement == Movement::Defeat {
            p.comp
        } else if movement == Movement::Title && (8..16).contains(&(local % 32)) {
            p.accent
        } else {
            p.lead
        };
        let pitch = if movement == Movement::Defeat {
            pitch.saturating_sub(12)
        } else {
            pitch
        };
        let length = f32::from(duration)
            * if movement == Movement::Combat {
                0.72
            } else {
                0.93
            };
        let breath = if local % 4 == 3 { 0.84 } else { 1.0 };
        let accent = if tick == 0 {
            1.05
        } else if tick.is_multiple_of(2) {
            0.97
        } else {
            0.91
        };
        let answering = movement == Movement::Title && (8..16).contains(&(local % 32));
        let response_gain = if answering {
            match lead {
                Instrument::Flute => 0.80,
                Instrument::Bell | Instrument::Violin => 0.90,
                _ => 0.95,
            }
        } else {
            1.0
        };
        out.add(
            lead,
            pitch,
            length * breath,
            if quiet { 0.29 } else { 0.38 } * arc * accent * response_gain,
            Role::Melody,
        );
        // The B section answers in another colour; the coda opens the register.
        if matches!(movement, Movement::Endgame | Movement::Victory)
            || (movement == Movement::Title && local % 32 >= 16)
        {
            out.add(
                p.accent,
                if pitch >= 76 { pitch - 12 } else { pitch },
                length * 0.88,
                0.13 * arc,
                Role::Accent,
            );
        }
    }
    rhythm::accompany(&mut out, theme, movement, local, tick, harmony, arc);
    balance(theme, &mut out);
    out
}
