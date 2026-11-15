//! Orchestration of the five D-minor / D-Dorian / F-major / D-major suites,
//! with continuous vertical stack-depth and action-intensity scaling from
//! Calm ("Ruhig") chamber guitar/strings to Epic ("Episch") brass, timpani
//! and celesta climax. Fixed-capacity events, no audio I/O.
use super::{Movement, Theme, manuscript::Bar};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(in crate::music) enum Instrument {
    Harp,
    Zither,
    Lyre,
    Violin,
    Viola,
    Cello,
    Bass,
    Trombone,
    ViolinShort,
    ViolaShort,
    CelloShort,
    TavernGuitar,
    Piano,
    Clav,
    Flute,
    Guitar,
    ElectricPiano,
    AnalogLead,
    ChipLead,
    Marimba,
    Bell,
    Pad,
    SynthBass,
    FrenchHorn,
    StringPad,
    Celesta,
    Kick,
    Snare,
    Hat,
    Timpani,
}
impl Instrument {
    pub(in crate::music) const fn synthetic(self) -> bool {
        self as u8 >= Self::Piano as u8
    }
    pub(in crate::music) const fn sampled_acoustic(self) -> bool {
        !matches!(self, Self::Lyre | Self::TavernGuitar) && !self.synthetic()
    }
    #[cfg(test)]
    pub(in crate::music) const fn percussion(self) -> bool {
        matches!(self, Self::Kick | Self::Snare | Self::Hat | Self::Timpani)
    }
    pub(super) const fn pan(self) -> f32 {
        match self {
            Self::Harp => -0.38,
            Self::Zither => 0.34,
            Self::Lyre => 0.08,
            Self::TavernGuitar => -0.06,
            Self::Violin | Self::ViolinShort => -0.24,
            Self::Viola | Self::ViolaShort => 0.26,
            Self::Cello | Self::CelloShort => 0.16,
            Self::Trombone => -0.08,
            Self::FrenchHorn => 0.14,
            Self::StringPad => -0.20,
            Self::Celesta => 0.28,
            Self::Piano | Self::Guitar | Self::Clav => -0.18,
            Self::Marimba | Self::ElectricPiano => 0.18,
            Self::Bell | Self::Hat => 0.32,
            Self::Pad => -0.30,
            Self::Timpani => -0.12,
            _ => 0.0,
        }
    }
    pub(in crate::music) const fn short(self) -> bool {
        matches!(
            self,
            Self::ViolinShort | Self::ViolaShort | Self::CelloShort
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    Melody,
    Harmony,
    Bass,
    Rhythm,
    Accent,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::music) struct Note {
    pub instrument: Instrument,
    pub pitch: u8,
    pub length: f32,
    pub gain: f32,
    pub late: f32,
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) role: Role,
}
pub(super) const EMPTY: Note = Note {
    instrument: Instrument::Harp,
    pitch: 60,
    length: 0.0,
    gain: 0.0,
    late: 0.0,
    role: Role::Harmony,
};

pub(in crate::music) struct Notes {
    pub(super) events: [Note; 24],
    pub(super) len: usize,
}
impl Notes {
    pub(in crate::music) fn as_slice(&self) -> &[Note] {
        &self.events[..self.len]
    }
    pub(super) fn add(
        &mut self,
        instrument: Instrument,
        pitch: u8,
        length: f32,
        gain: f32,
        role: Role,
    ) {
        assert!(self.len < self.events.len(), "a tick fits the note budget");
        self.events[self.len] = Note {
            instrument,
            pitch,
            length,
            gain,
            role,
            late: if instrument == Instrument::Viola {
                0.045
            } else if instrument == Instrument::TavernGuitar && role != Role::Melody {
                0.012
            } else {
                0.0
            },
        };
        self.len += 1;
    }
}

/// Root, third and fifth as semitone intervals. Unified around D minor / D Dorian / F major / D major.
#[derive(Clone, Copy, Debug)]
pub(super) struct Chord(pub u8, pub u8, pub u8);
const DM: Chord = Chord(62, 3, 7);
const D_MAJ: Chord = Chord(62, 4, 7);
const F_MAJ: Chord = Chord(65, 4, 7);
const C_MAJ: Chord = Chord(60, 4, 7);
const BB_MAJ: Chord = Chord(58, 4, 7);
const GM: Chord = Chord(55, 3, 7);
const G_MAJ: Chord = Chord(55, 4, 7);
const AM: Chord = Chord(57, 3, 7);
const A_MAJ: Chord = Chord(57, 4, 7);

/// Consonant major/minor triads only. Each suite's harmonic route reflects its
/// classical inspiration while sharing the home D tonal centre across scenes.
pub(super) fn chord(theme: Theme, movement: Movement, bar: u32) -> Chord {
    if theme.exploration() {
        return super::styles::chord(theme, movement, bar);
    }
    let b = (bar % 8) as usize;
    let home = match theme {
        Theme::Ember => [DM, G_MAJ, AM, C_MAJ, DM, G_MAJ, F_MAJ, DM],
        Theme::Glass => [DM, F_MAJ, BB_MAJ, A_MAJ, DM, F_MAJ, GM, DM],
        Theme::Thorn => [DM, AM, F_MAJ, C_MAJ, DM, AM, GM, DM],
        Theme::Tide => [DM, C_MAJ, BB_MAJ, AM, DM, C_MAJ, F_MAJ, DM],
        Theme::Star => [DM, F_MAJ, C_MAJ, G_MAJ, DM, F_MAJ, AM, DM],
        _ => unreachable!("explorations dispatch above"),
    }[b];
    match movement {
        Movement::Victory if home.0 == DM.0 => D_MAJ,
        Movement::Defeat if home.0 == G_MAJ.0 => GM,
        Movement::Combat | Movement::Endgame | Movement::Draw
            if home.0 == DM.0 && bar % 16 >= 8 =>
        {
            D_MAJ
        }
        _ => home,
    }
}

/// Voice a chord tone near a register, reducing parallel octave jumps in the bed.
pub(super) fn near(pitch: u8, centre: u8) -> u8 {
    let mut value = i16::from(pitch);
    let centre = i16::from(centre);
    while value > centre + 6 {
        value -= 12;
    }
    while value < centre - 5 {
        value += 12;
    }
    u8::try_from(value).expect("instrument register")
}

/// Notes beginning at this tick, with composed rests retained.
pub(super) fn onset(bar: Bar, tick: u8) -> Option<(u8, u8)> {
    let mut at = 0;
    for &(pitch, length) in bar {
        if at == tick {
            return (pitch > 0).then_some((pitch, length));
        }
        at += length;
    }
    None
}

/// Explicit parallel major/minor mixture: F -> F# over D major, B -> Bb over G minor.
fn colour(pitch: u8, harmony: Chord) -> u8 {
    if harmony.0 == 62 && harmony.1 == 4 && pitch % 12 == 5 {
        pitch + 1
    } else if harmony.0 == 55 && harmony.1 == 3 && pitch % 12 == 11 {
        pitch - 1
    } else {
        pitch
    }
}

pub(super) fn line(theme: Theme, movement: Movement, bar: u32) -> Bar {
    let pages = theme.pages();
    let local = if movement.ending() {
        bar.saturating_sub(1)
    } else {
        bar
    };
    let b = (local % 8) as usize;
    let phrase = match movement {
        Movement::Title => {
            if local % 32 >= 16 && local % 32 < 24 {
                pages.answer
            } else {
                pages.title
            }
        }
        Movement::Lobby | Movement::Victory => pages.tavern,
        Movement::Standard | Movement::Defeat => pages.table,
        Movement::Combat => pages.combat,
        Movement::Endgame => {
            if local % 16 < 8 {
                pages.answer
            } else {
                pages.title
            }
        }
        Movement::Draw => {
            if local % 16 < 8 {
                pages.table
            } else {
                pages.tavern
            }
        }
    };
    phrase[b]
}

#[allow(clippy::too_many_arguments)]
fn melody(
    out: &mut Notes,
    theme: Theme,
    movement: Movement,
    bar: u32,
    tick: u8,
    arc: f32,
    harmony: Chord,
    stack_depth: u8,
    intensity: f32,
) {
    use Instrument::{Cello, Flute, Harp, Lyre, TavernGuitar, Trombone, Violin, ViolinShort};
    let Some((pitch, length)) = onset(line(theme, movement, bar), tick) else {
        return;
    };
    let pitch = colour(pitch, harmony);
    let length = f32::from(length) * 0.94;
    let pluck = match theme {
        Theme::Glass | Theme::Star => Harp,
        Theme::Ember => Flute,
        _ => TavernGuitar,
    };
    let dyn_scale = 1.0 + 0.14 * intensity;
    let (voice, pitch, gain) = match movement {
        Movement::Title => match theme {
            Theme::Ember => (Violin, pitch, 0.32),
            Theme::Glass if bar % 32 < 8 => (Harp, pitch, 0.38),
            Theme::Thorn if bar % 8 < 2 => (Trombone, pitch - 12, 0.28),
            Theme::Tide if bar % 32 < 16 => (Lyre, pitch, 0.36),
            _ => (Violin, pitch, 0.30),
        },
        Movement::Lobby => (if bar % 16 < 8 { TavernGuitar } else { Lyre }, pitch, 0.34),
        Movement::Standard => (
            if bar % 16 < 8 {
                TavernGuitar
            } else if bar % 8 < 4 {
                pluck
            } else {
                Lyre
            },
            pitch,
            0.21 + 0.07 * intensity,
        ),
        Movement::Combat => (ViolinShort, pitch, 0.28 * dyn_scale),
        Movement::Endgame => (Violin, pitch, 0.31 * dyn_scale),
        Movement::Victory => (Violin, pitch + 12, 0.28),
        Movement::Defeat => (Cello, pitch - 12, 0.22),
        Movement::Draw => (if bar % 16 < 8 { Harp } else { Lyre }, pitch, 0.25),
    };
    out.add(voice, pitch, length, gain * arc, Role::Melody);
    melody_layers(
        out,
        theme,
        movement,
        voice,
        bar,
        tick,
        pitch,
        length,
        arc,
        stack_depth,
        intensity,
    );
}

#[allow(clippy::too_many_arguments)]
fn melody_layers(
    out: &mut Notes,
    theme: Theme,
    movement: Movement,
    voice: Instrument,
    bar: u32,
    tick: u8,
    pitch: u8,
    length: f32,
    arc: f32,
    stack_depth: u8,
    intensity: f32,
) {
    use Instrument::{Celesta, Flute, FrenchHorn, Harp, Lyre, Trombone, Violin};
    match movement {
        Movement::Title | Movement::Endgame => {
            if movement == Movement::Endgame || (bar % 8 >= 4 && tick == 0) {
                out.add(
                    Trombone,
                    pitch - 12,
                    length,
                    if movement == Movement::Endgame {
                        0.22 + 0.05 * intensity
                    } else {
                        0.12
                    } * arc,
                    Role::Melody,
                );
            }
            if movement == Movement::Endgame && (stack_depth >= 2 || intensity >= 0.65) {
                out.add(
                    FrenchHorn,
                    pitch - 12,
                    length * 1.05,
                    (0.13 + 0.06 * intensity) * arc,
                    Role::Harmony,
                );
            }
            if movement == Movement::Endgame
                && (stack_depth >= 3 || intensity >= 0.80)
                && tick.is_multiple_of(2)
            {
                out.add(
                    Celesta,
                    near(pitch, 79),
                    length * 0.85,
                    0.085 * arc,
                    Role::Accent,
                );
            }
            if movement == Movement::Title && voice != Violin && bar % 4 >= 2 {
                out.add(Violin, pitch, length, 0.13 * arc, Role::Melody);
            }
            if tick == 0 && bar.is_multiple_of(4) {
                out.add(Lyre, pitch, 1.8, 0.24 * arc, Role::Accent);
            }
        }
        Movement::Victory => {
            out.add(Trombone, pitch - 12, length, 0.15 * arc, Role::Melody);
            out.add(
                FrenchHorn,
                pitch - 12,
                length * 1.1,
                0.12 * arc,
                Role::Harmony,
            );
            if tick == 0 {
                out.add(Celesta, pitch.min(86), 2.0, 0.10 * arc, Role::Accent);
            }
        }
        Movement::Standard => {
            if bar % 4 == 2 {
                out.add(Harp, pitch - 12, length, 0.11 * arc, Role::Melody);
            }
            // As stack/action grows in Standard, a solo violin/flute gently doubles the melody.
            if (stack_depth >= 1 || intensity >= 0.28) && tick.is_multiple_of(2) {
                out.add(
                    Violin,
                    pitch,
                    length * 0.95,
                    (0.08 + 0.09 * intensity) * arc,
                    Role::Melody,
                );
            }
        }
        Movement::Combat if stack_depth >= 2 || intensity >= 0.48 => {
            if tick == 0 || tick == theme.ticks() / 2 {
                out.add(
                    FrenchHorn,
                    pitch - 12,
                    length * 1.15,
                    (0.09 + 0.06 * intensity) * arc,
                    Role::Harmony,
                );
            }
        }
        Movement::Lobby if bar % 4 == 2 => {
            out.add(Violin, pitch + 12, length, 0.12 * arc, Role::Melody);
        }
        Movement::Draw if tick == 0 => {
            out.add(Flute, pitch, length * 1.1, 0.11 * arc, Role::Harmony);
        }
        _ => {}
    }
}

fn arpeggio(theme: Theme, Chord(root, third, fifth): Chord, tick: u8, stride: u8) -> u8 {
    let steps = match theme {
        Theme::Ember => [0, fifth, third + 12, fifth, 12],
        Theme::Glass => [0, third + 12, fifth + 12, third + 12, fifth],
        Theme::Thorn => [0, 12, fifth, 12, 0],
        Theme::Tide => [0, fifth, third + 12, 12, fifth + 12],
        Theme::Star => [0, third + 12, fifth + 12, 12, fifth],
        _ => unreachable!("explorations use their own accompaniment"),
    };
    root + steps[usize::from(tick / stride) % steps.len()]
}

#[allow(clippy::too_many_arguments)]
fn accompaniment(
    out: &mut Notes,
    movement: Movement,
    theme: Theme,
    bar: u32,
    tick: u8,
    harmony: Chord,
    arc: f32,
    stack_depth: u8,
    intensity: f32,
) {
    use Instrument::{Bass, Cello, TavernGuitar, Viola};
    let ticks = theme.ticks();
    let Chord(root, third, _) = harmony;
    let quiet = matches!(
        movement,
        Movement::Lobby | Movement::Defeat | Movement::Draw
    );
    if tick == 0 {
        let length = f32::from(ticks) - 0.25;
        let cello_gain = if movement == Movement::Lobby {
            0.14
        } else if movement == Movement::Standard {
            0.09 + 0.04 * intensity
        } else if quiet {
            0.11
        } else {
            0.14 + 0.035 * intensity
        };
        out.add(Cello, near(root, 49), length, cello_gain * arc, Role::Bass);
        if !quiet {
            let bass_gain = if movement == Movement::Standard {
                0.07 + 0.04 * intensity
            } else {
                0.12 + 0.035 * intensity
            };
            out.add(Bass, near(root, 38), length, bass_gain * arc, Role::Bass);
        }
        if matches!(movement, Movement::Lobby | Movement::Standard) {
            out.add(
                TavernGuitar,
                near(root, 48),
                length * 0.65,
                if movement == Movement::Lobby {
                    0.18
                } else {
                    0.105 * (1.0 - 0.25 * intensity)
                } * arc,
                Role::Bass,
            );
        }
        let viola_gain = if movement == Movement::Lobby {
            0.12
        } else if movement == Movement::Standard {
            0.072 + 0.03 * intensity
        } else {
            0.105 + 0.03 * intensity
        };
        out.add(
            Viola,
            near(root + third, 61),
            length,
            viola_gain * arc,
            Role::Harmony,
        );
        accompaniment_pads(out, movement, harmony, length, arc, stack_depth, intensity);
    }
    accompaniment_arpeggio(out, movement, theme, bar, tick, harmony, arc, intensity);
}

fn accompaniment_pads(
    out: &mut Notes,
    movement: Movement,
    Chord(root, third, fifth): Chord,
    length: f32,
    arc: f32,
    stack_depth: u8,
    intensity: f32,
) {
    use Instrument::{FrenchHorn, StringPad, Trombone, Viola};
    if matches!(
        movement,
        Movement::Title | Movement::Endgame | Movement::Victory | Movement::Draw
    ) {
        out.add(
            Viola,
            near(root + fifth, 66),
            length,
            0.075 * arc,
            Role::Harmony,
        );
        out.add(
            StringPad,
            near(root + fifth, 64),
            length,
            if movement == Movement::Endgame {
                0.075 + 0.035 * intensity
            } else {
                0.055
            } * arc,
            Role::Harmony,
        );
    } else if matches!(
        movement,
        Movement::Standard | Movement::Combat | Movement::Defeat
    ) && (movement == Movement::Defeat || stack_depth >= 1 || intensity >= 0.24)
    {
        out.add(
            StringPad,
            near(root + third, 61),
            length,
            (0.05 + 0.045 * intensity) * arc,
            Role::Harmony,
        );
    }
    if movement == Movement::Standard && (stack_depth >= 2 || intensity >= 0.38) {
        out.add(
            FrenchHorn,
            near(root + fifth, 57),
            length * 0.78,
            (0.065 + 0.04 * intensity) * arc,
            Role::Harmony,
        );
    }
    if movement == Movement::Combat {
        out.add(
            Trombone,
            near(root + third, 53),
            length * 0.65,
            0.060 * arc,
            Role::Harmony,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn accompaniment_arpeggio(
    out: &mut Notes,
    movement: Movement,
    theme: Theme,
    bar: u32,
    tick: u8,
    harmony: Chord,
    arc: f32,
    intensity: f32,
) {
    use Instrument::{Harp, Lyre, TavernGuitar, Zither};
    let ticks = theme.ticks();
    let Chord(root, third, fifth) = harmony;
    let quiet = matches!(
        movement,
        Movement::Lobby | Movement::Defeat | Movement::Draw
    );
    let stride = if movement == Movement::Defeat {
        4
    } else if theme == Theme::Tide && !quiet {
        1
    } else if theme == Theme::Star {
        3
    } else {
        2
    };
    if tick.is_multiple_of(stride)
        && !(quiet && tick + 2 >= ticks)
        && !(matches!(theme, Theme::Glass | Theme::Star)
            && movement == Movement::Lobby
            && bar % 4 == 3
            && tick > 0)
    {
        let note = arpeggio(theme, harmony, tick, stride);
        let family = if movement == Movement::Lobby && bar % 2 == 1 {
            Zither
        } else {
            Harp
        };
        let arp_gain = if movement == Movement::Lobby {
            0.15
        } else if movement == Movement::Standard {
            0.095 + 0.035 * intensity
        } else if quiet {
            0.12
        } else if theme == Theme::Tide {
            0.105
        } else {
            0.14
        };
        out.add(family, note, 2.2, arp_gain * arc, Role::Rhythm);
        if movement == Movement::Lobby && (tick == 2 || tick == 4) {
            out.add(
                TavernGuitar,
                near(root + fifth, 62),
                1.7,
                0.14 * arc,
                Role::Rhythm,
            );
            out.add(Lyre, near(root + third, 65), 1.5, 0.10 * arc, Role::Rhythm);
        }
    }
    if matches!(movement, Movement::Title | Movement::Endgame) && tick + 2 == ticks {
        out.add(Lyre, near(root + third, 73), 1.5, 0.22 * arc, Role::Accent);
    }
}

#[allow(clippy::too_many_arguments)]
fn drive(
    out: &mut Notes,
    theme: Theme,
    movement: Movement,
    tick: u8,
    harmony: Chord,
    arc: f32,
    stack_depth: u8,
    intensity: f32,
) {
    use Instrument::{CelloShort, Timpani, ViolaShort};
    let ticks = theme.ticks();
    let Chord(root, third, fifth) = harmony;
    if movement == Movement::Standard && (stack_depth >= 2 || intensity >= 0.42) {
        if tick == 0 || tick == ticks / 2 {
            out.add(
                CelloShort,
                near(root, 48),
                0.60,
                (0.08 + 0.06 * intensity) * arc,
                Role::Rhythm,
            );
        }
        return;
    }
    if !matches!(movement, Movement::Combat | Movement::Endgame) {
        return;
    }
    let full_climax = stack_depth >= 3 || intensity >= 0.78;
    if movement == Movement::Endgame && !full_climax && tick % 2 == 1 {
        return;
    }
    let strong = (theme == Theme::Thorn && (tick == 3 || tick == 6))
        || tick == 0
        || tick
            == if theme == Theme::Tide {
                3
            } else if theme == Theme::Star {
                6
            } else {
                4
            };
    let pitch = near(root + if tick % 4 < 2 { fifth } else { third }, 61);
    let boost = 1.0 + 0.16 * intensity;
    out.add(
        ViolaShort,
        pitch,
        0.58,
        if strong { 0.21 } else { 0.12 } * boost * arc,
        Role::Rhythm,
    );
    if strong || tick.is_multiple_of(2) {
        out.add(
            CelloShort,
            near(root, 48),
            0.65,
            if strong { 0.20 } else { 0.12 } * boost * arc,
            Role::Rhythm,
        );
    }
    // Pitched orchestral Timpani blooms on stack chains (2+) and epic Endgame climaxes.
    let wants_timpani = movement == Movement::Endgame || stack_depth >= 2 || intensity >= 0.52;
    if wants_timpani {
        if tick == 0 {
            out.add(
                Timpani,
                near(root, 38),
                1.4,
                (0.14 + 0.06 * intensity) * arc,
                Role::Rhythm,
            );
        } else if (full_climax || stack_depth >= 3) && tick == ticks / 2 {
            out.add(
                Timpani,
                near(root + fifth, 43),
                1.1,
                (0.11 + 0.05 * intensity) * arc,
                Role::Rhythm,
            );
        } else if stack_depth >= 4 && tick + 1 == ticks {
            out.add(Timpani, near(root, 38), 0.75, 0.11 * arc, Role::Rhythm);
        }
    }
}

/// Three distinct public-domain-inspired outcome melodies on Bar 0:
/// - Victory: Radiant D-major ascending fanfare (*Great Gate of Kiev* / Handel)
/// - Defeat: Somber D-minor descending elegy (Grieg *Åse's Death*)
/// - Draw: Contemplative D-Dorian open-fifth D–A modal suspension (Satie *Gnossienne*)
fn cue(out: &mut Notes, theme: Theme, movement: Movement, tick: u8) {
    use Instrument::{Celesta, Cello, Flute, FrenchHorn, Harp, Lyre, StringPad, Timpani, Trombone};
    let span = theme.ticks();
    let onsets = [0, 1, 2, 3];
    let Some(index) = onsets.iter().position(|&onset| onset == tick) else {
        return;
    };
    let pitch = match movement {
        Movement::Victory => {
            (match theme {
                Theme::Ember => [62, 69, 74, 86],
                Theme::Glass => [62, 66, 74, 81],
                Theme::Thorn => [57, 66, 74, 86],
                Theme::Tide => [62, 69, 78, 86],
                Theme::Star => [62, 66, 69, 86],
                _ => unreachable!("exploration cue"),
            })[index]
        }
        Movement::Defeat => {
            (match theme {
                Theme::Ember => [81, 77, 69, 62],
                Theme::Glass => [86, 77, 69, 62],
                Theme::Thorn => [81, 74, 65, 62],
                Theme::Tide => [77, 74, 69, 62],
                Theme::Star => [86, 81, 69, 62],
                _ => unreachable!("exploration cue"),
            })[index]
        }
        _ => [74, 81, 69, 74][index],
    };
    let length = if index == 3 {
        f32::from(span - 3) - 0.2
    } else {
        0.88
    };
    let voice = match movement {
        Movement::Defeat => Cello,
        Movement::Draw => Flute,
        _ => Trombone,
    };
    out.add(
        voice,
        if movement == Movement::Defeat || movement == Movement::Draw {
            pitch
        } else {
            pitch - 12
        },
        length,
        match movement {
            Movement::Defeat => 0.29,
            Movement::Draw => 0.26,
            _ => 0.31,
        },
        Role::Melody,
    );
    out.add(Harp, pitch, 1.8, 0.24, Role::Accent);
    out.add(Lyre, pitch, 1.7, 0.24, Role::Accent);
    match movement {
        Movement::Victory => {
            out.add(FrenchHorn, pitch - 12, length * 1.1, 0.16, Role::Harmony);
            if index == 0 || index == 3 {
                out.add(Timpani, 38, 1.6, 0.18, Role::Rhythm);
                out.add(Celesta, pitch, 2.0, 0.12, Role::Accent);
            }
        }
        Movement::Defeat if index == 0 => {
            out.add(StringPad, 50, f32::from(span) * 0.95, 0.10, Role::Harmony);
            out.add(StringPad, 57, f32::from(span) * 0.95, 0.08, Role::Harmony);
        }
        Movement::Draw if index == 0 => {
            out.add(StringPad, 50, f32::from(span) * 0.95, 0.09, Role::Harmony);
            out.add(FrenchHorn, 57, f32::from(span) * 0.90, 0.09, Role::Harmony);
        }
        _ => {}
    }
}

#[cfg(test)]
pub(in crate::music) fn notes(theme: Theme, movement: Movement, bar: u32, tick: u8) -> Notes {
    notes_with_intensity(theme, movement, bar, tick, 0, 0.0)
}

pub(in crate::music) fn notes_with_intensity(
    theme: Theme,
    movement: Movement,
    bar: u32,
    tick: u8,
    stack_depth: u8,
    intensity: f32,
) -> Notes {
    let intensity = intensity.clamp(0.0, 1.0);
    if theme.exploration() {
        return super::styles::notes_with_intensity(
            theme,
            movement,
            bar,
            tick,
            stack_depth,
            intensity,
        );
    }
    let mut out = Notes {
        events: [EMPTY; 24],
        len: 0,
    };
    if movement.ending() && bar == 0 {
        cue(&mut out, theme, movement, tick);
        return out;
    }
    let local = if movement.ending() { bar - 1 } else { bar };
    // AABA dynamics: statement, more intimate repeat, development, homecoming.
    let section = (local % 32 / 8) as usize;
    let arc = if movement == Movement::Endgame {
        [0.96, 0.92, 1.0, 0.95][section]
    } else {
        [0.90, 0.76, 1.0, 0.87][section] * [0.91, 0.97, 1.0, 0.90][(local % 4) as usize]
    };
    let harmony = chord(theme, movement, local % 16);
    melody(
        &mut out,
        theme,
        movement,
        bar,
        tick,
        arc,
        harmony,
        stack_depth,
        intensity,
    );
    accompaniment(
        &mut out,
        movement,
        theme,
        local,
        tick,
        harmony,
        arc,
        stack_depth,
        intensity,
    );
    drive(
        &mut out,
        theme,
        movement,
        tick,
        harmony,
        arc,
        stack_depth,
        intensity,
    );
    out
}
