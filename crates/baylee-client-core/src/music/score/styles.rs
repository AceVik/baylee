//! Ten deliberately separate idioms, sharing only the conductor and note budget.
//! Each owns its tonal route, melody book, pulse, palette, accompaniment, and
//! stack-depth climax layers.
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
        0.38,
        Role::Melody,
    );
}
fn balance(theme: Theme, out: &mut Notes) {
    let gain = match theme {
        Theme::Velvet | Theme::Circuit => 1.0,
        Theme::Copper => 1.24,
        Theme::Juniper => 0.85,
        Theme::Lagoon | Theme::Orbit => 0.96,
        Theme::Lantern => 0.97,
        Theme::Neon | Theme::Iron => 0.86,
        Theme::Mosaic => 1.02,
        _ => unreachable!("new palettes only"),
    };
    for note in &mut out.events[..out.len] {
        note.gain *= gain;
    }
}

fn add_stack_layers(
    out: &mut Notes,
    theme: Theme,
    harmony: Chord,
    arc: f32,
    stack_depth: u8,
    intensity: f32,
) {
    if (stack_depth < 2 && intensity < 0.52) || out.len + 2 > 16 {
        return;
    }
    let ticks = profile(theme).ticks;
    let Chord(root, _, fifth) = harmony;
    let swell = if theme == Theme::Circuit {
        Instrument::Pad
    } else {
        Instrument::FrenchHorn
    };
    out.add(
        swell,
        arrangement::near(root + fifth, 60),
        f32::from(ticks) * 0.78,
        (0.055 + 0.045 * intensity) * arc,
        Role::Harmony,
    );
    if (stack_depth >= 3 || intensity >= 0.76) && out.len < 16 {
        out.add(
            Instrument::Timpani,
            arrangement::near(root, 38),
            1.2,
            0.13 * arc,
            Role::Rhythm,
        );
    }
}

pub(super) fn notes_with_intensity(
    theme: Theme,
    movement: Movement,
    bar: u32,
    tick: u8,
    stack_depth: u8,
    intensity: f32,
) -> Notes {
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
        let boost = if quiet { 1.0 } else { 1.0 + 0.12 * intensity };
        out.add(
            lead,
            pitch,
            length * breath,
            if quiet { 0.28 } else { 0.36 } * boost * arc * accent * response_gain,
            Role::Melody,
        );
        if matches!(movement, Movement::Endgame | Movement::Victory)
            || (movement == Movement::Title && local % 32 >= 16)
            || (!quiet && (stack_depth >= 2 || intensity >= 0.55))
        {
            out.add(
                p.accent,
                if pitch >= 76 { pitch - 12 } else { pitch },
                length * 0.88,
                (0.11 + 0.04 * intensity) * arc,
                Role::Accent,
            );
        }
    }
    rhythm::accompany(&mut out, theme, movement, local, tick, harmony, arc);
    if !quiet && tick == 0 {
        add_stack_layers(&mut out, theme, harmony, arc, stack_depth, intensity);
    }
    balance(theme, &mut out);
    out
}
