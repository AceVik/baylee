//! Orchestration of the five manuscripts. Fixed-capacity events, no audio I/O.
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
}
impl Instrument {
    pub(super) const fn pan(self) -> f32 {
        match self {
            Self::Harp => -0.38,
            Self::Zither => 0.34,
            Self::Lyre => 0.08,
            Self::Violin | Self::ViolinShort => -0.24,
            Self::Viola | Self::ViolaShort => 0.26,
            Self::Cello | Self::CelloShort => 0.16,
            Self::Bass => 0.0,
            Self::Trombone => -0.06,
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
const EMPTY: Note = Note {
    instrument: Instrument::Harp,
    pitch: 60,
    length: 0.0,
    gain: 0.0,
    late: 0.0,
    role: Role::Harmony,
};

pub(in crate::music) struct Notes {
    events: [Note; 24],
    len: usize,
}
impl Notes {
    pub(in crate::music) fn as_slice(&self) -> &[Note] {
        &self.events[..self.len]
    }
    fn add(&mut self, instrument: Instrument, pitch: u8, length: f32, gain: f32, role: Role) {
        assert!(self.len < self.events.len(), "a tick fits the note budget");
        self.events[self.len] = Note {
            instrument,
            pitch,
            length,
            gain,
            role,
            late: if instrument == Instrument::Viola {
                0.045
            } else {
                0.0
            },
        };
        self.len += 1;
    }
}

/// Root, third and fifth as semitone intervals. The root is near middle C.
#[derive(Clone, Copy, Debug)]
pub(super) struct Chord(pub u8, pub u8, pub u8);
const BBM: Chord = Chord(58, 3, 7);
const BB: Chord = Chord(58, 4, 7);
const EB: Chord = Chord(63, 4, 7);
const EBM: Chord = Chord(63, 3, 7);
const FM: Chord = Chord(65, 3, 7);
const DB: Chord = Chord(61, 4, 7);
const AB: Chord = Chord(56, 4, 7);

/// Major/minor triads only. A suite's harmonic route supports its motif;
/// borrowed major/minor thirds are changed in every voice together.
pub(super) fn chord(theme: Theme, movement: Movement, bar: u32) -> Chord {
    let b = (bar % 8) as usize;
    let home = match theme {
        Theme::Ember => [BBM, EB, FM, AB, BBM, EB, EB, BBM],
        Theme::Glass => [BBM, DB, EB, AB, BBM, DB, EB, BBM],
        Theme::Thorn => [BBM, BBM, AB, EB, BBM, BBM, FM, BBM],
        Theme::Tide => [BBM, EB, AB, FM, BBM, EB, EB, BBM],
        Theme::Star => [BBM, FM, DB, EB, BBM, FM, AB, BBM],
    }[b];
    match movement {
        Movement::Victory if home.0 == BBM.0 => BB,
        Movement::Defeat if home.0 == EB.0 => EBM,
        Movement::Combat | Movement::Endgame | Movement::Draw
            if home.0 == BBM.0 && bar % 16 >= 8 =>
        {
            BB
        }
        _ => home,
    }
}

/// Voice a chord tone near a register, reducing parallel octave jumps in the bed.
fn near(pitch: u8, centre: u8) -> u8 {
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
fn onset(bar: Bar, tick: u8) -> Option<(u8, u8)> {
    let mut at = 0;
    for &(pitch, length) in bar {
        if at == tick {
            return (pitch > 0).then_some((pitch, length));
        }
        at += length;
    }
    None
}

/// Major/minor mixture is explicit: the borrowed third only changes over
/// that chord, avoiding a simultaneous minor and major third.
fn colour(pitch: u8, harmony: Chord) -> u8 {
    if harmony.0 == 58 && harmony.1 == 4 && pitch % 12 == 1 {
        pitch + 1
    } else if harmony.0 == 63 && harmony.1 == 3 && pitch % 12 == 7 {
        pitch - 1
    } else {
        pitch
    }
}

fn line(theme: Theme, movement: Movement, bar: u32) -> Bar {
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

fn melody(
    out: &mut Notes,
    theme: Theme,
    movement: Movement,
    bar: u32,
    tick: u8,
    arc: f32,
    harmony: Chord,
) {
    use Instrument::{Cello, Harp, Lyre, Trombone, Violin, ViolinShort, Zither};
    let Some((pitch, length)) = onset(line(theme, movement, bar), tick) else {
        return;
    };
    let pitch = colour(pitch, harmony);
    let length = f32::from(length) * 0.94;
    let pluck = match theme {
        Theme::Glass | Theme::Star => Harp,
        Theme::Thorn => Zither,
        _ => Lyre,
    };
    let (voice, pitch, gain) = match movement {
        Movement::Title => match theme {
            Theme::Ember => (Violin, pitch, 0.34),
            Theme::Glass if bar % 32 < 8 => (Harp, pitch, 0.43),
            Theme::Thorn if bar % 8 < 2 => (Trombone, pitch - 12, 0.30),
            Theme::Tide if bar % 32 < 16 => (Lyre, pitch, 0.42),
            _ => (Violin, pitch, 0.31),
        },
        Movement::Lobby => (if bar % 16 < 8 { Lyre } else { pluck }, pitch, 0.32),
        Movement::Standard => (if bar % 16 < 8 { pluck } else { Lyre }, pitch, 0.34),
        Movement::Combat => (ViolinShort, pitch, 0.30),
        Movement::Endgame => (Violin, pitch, 0.32),
        Movement::Victory => (Violin, pitch + 12, 0.29),
        Movement::Defeat => (Cello, pitch - 12, 0.21),
        Movement::Draw => (if bar % 16 < 8 { Harp } else { Lyre }, pitch, 0.27),
    };
    out.add(voice, pitch, length, gain * arc, Role::Melody);
    match movement {
        Movement::Title | Movement::Endgame => {
            if movement == Movement::Endgame || (bar % 8 >= 4 && tick == 0) {
                out.add(
                    Trombone,
                    pitch - 12,
                    length,
                    if movement == Movement::Endgame {
                        0.23
                    } else {
                        0.12
                    } * arc,
                    Role::Melody,
                );
            }
            if movement == Movement::Title && voice != Violin && bar % 4 >= 2 {
                out.add(Violin, pitch, length, 0.13 * arc, Role::Melody);
            }
            if tick == 0 && bar.is_multiple_of(4) {
                out.add(Lyre, pitch, 1.8, 0.26 * arc, Role::Accent);
            }
        }
        Movement::Victory => out.add(Trombone, pitch - 12, length, 0.16 * arc, Role::Melody),
        Movement::Standard if bar % 4 == 2 => {
            out.add(Harp, pitch - 12, length, 0.11 * arc, Role::Melody);
        }
        Movement::Lobby if bar % 8 == 6 => out.add(Violin, pitch, length, 0.10 * arc, Role::Melody),
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
    };
    root + steps[usize::from(tick / stride) % steps.len()]
}

fn accompaniment(
    out: &mut Notes,
    movement: Movement,
    theme: Theme,
    bar: u32,
    tick: u8,
    harmony: Chord,
    arc: f32,
) {
    use Instrument::{Bass, Cello, Harp, Lyre, Trombone, Viola, Zither};
    let ticks = theme.ticks();
    let Chord(root, third, fifth) = harmony;
    let quiet = matches!(
        movement,
        Movement::Lobby | Movement::Defeat | Movement::Draw
    );
    if tick == 0 {
        let length = f32::from(ticks) - 0.25;
        out.add(
            Cello,
            near(root, 49),
            length,
            if movement == Movement::Lobby && theme == Theme::Glass {
                0.075
            } else if quiet {
                0.10
            } else {
                0.14
            } * arc,
            Role::Bass,
        );
        // No pizzicato bass anywhere, including combat.
        if !quiet {
            out.add(Bass, near(root, 36), length, 0.12 * arc, Role::Bass);
        }
        if movement != Movement::Lobby || bar.is_multiple_of(2) {
            out.add(
                Viola,
                near(root + third, 61),
                length,
                0.105 * arc,
                Role::Harmony,
            );
        }
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
        }
        if movement == Movement::Combat {
            out.add(
                Trombone,
                near(root + third, 53),
                length * 0.65,
                0.065 * arc,
                Role::Harmony,
            );
        }
    }
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
        out.add(
            family,
            note,
            2.2,
            if quiet {
                0.12
            } else if theme == Theme::Tide {
                0.11
            } else {
                0.15
            } * arc,
            Role::Rhythm,
        );
    }
    if matches!(movement, Movement::Title | Movement::Endgame) && tick + 2 == ticks {
        out.add(Lyre, near(root + third, 73), 1.5, 0.24 * arc, Role::Accent);
    }
}

fn drive(out: &mut Notes, theme: Theme, movement: Movement, tick: u8, harmony: Chord, arc: f32) {
    use Instrument::{CelloShort, ViolaShort};
    if !matches!(movement, Movement::Combat | Movement::Endgame) {
        return;
    }
    if movement == Movement::Endgame && tick % 2 == 1 {
        return;
    }
    let Chord(root, third, fifth) = harmony;
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
    out.add(
        ViolaShort,
        pitch,
        0.58,
        if strong { 0.24 } else { 0.14 } * arc,
        Role::Rhythm,
    );
    if strong || tick.is_multiple_of(2) {
        out.add(
            CelloShort,
            near(root, 48),
            0.65,
            if strong { 0.23 } else { 0.13 } * arc,
            Role::Rhythm,
        );
    }
}

/// One short rising, falling or open-fifth cue, then an entirely
/// different aftermath arrangement. It is never repeated while a result stays up.
fn cue(out: &mut Notes, theme: Theme, movement: Movement, tick: u8) {
    use Instrument::{Cello, Harp, Lyre, Trombone};
    let span = theme.ticks();
    let onsets = [0, 1, 2, 3];
    let Some(index) = onsets.iter().position(|&onset| onset == tick) else {
        return;
    };
    let pitch = match movement {
        Movement::Victory => {
            (match theme {
                Theme::Ember => [58, 65, 74, 82],
                Theme::Glass => [65, 70, 74, 82],
                Theme::Thorn => [58, 70, 77, 86],
                Theme::Tide => [62, 70, 77, 82],
                Theme::Star => [58, 65, 77, 82],
            })[index]
        }
        Movement::Defeat => {
            (match theme {
                Theme::Ember => [77, 73, 65, 58],
                Theme::Glass => [82, 77, 65, 58],
                Theme::Thorn => [82, 77, 61, 58],
                Theme::Tide => [77, 70, 65, 58],
                Theme::Star => [82, 73, 65, 58],
            })[index]
        }
        _ => [70, 77, 65, 70][index],
    };
    let length = if index == 3 {
        f32::from(span - 3) - 0.2
    } else {
        0.85
    };
    let voice = if movement == Movement::Defeat {
        Cello
    } else {
        Trombone
    };
    out.add(
        voice,
        if movement == Movement::Defeat {
            pitch
        } else {
            pitch - 12
        },
        length,
        if movement == Movement::Defeat {
            0.31
        } else {
            0.34
        },
        Role::Melody,
    );
    out.add(Harp, pitch, 1.8, 0.28, Role::Accent);
    out.add(Lyre, pitch, 1.7, 0.32, Role::Accent);
}

pub(in crate::music) fn notes(theme: Theme, movement: Movement, bar: u32, tick: u8) -> Notes {
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
    let arc = [0.90, 0.76, 1.0, 0.87][section] * [0.91, 0.97, 1.0, 0.90][(local % 4) as usize];
    let harmony = chord(theme, movement, local % 16);
    melody(&mut out, theme, movement, bar, tick, arc, harmony);
    accompaniment(&mut out, movement, theme, local, tick, harmony, arc);
    drive(&mut out, theme, movement, tick, harmony, arc);
    out
}
