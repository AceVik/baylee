//! Idiomatic accompaniment: no random melody generation or common backing loop.
use super::{Chord, Instrument, Movement, Notes, Role, Theme, arrangement::near, profile};

fn add(out: &mut Notes, voice: Instrument, pitch: u8, length: f32, gain: f32) {
    out.add(voice, pitch, length, gain, Role::Harmony);
}
fn triad(
    out: &mut Notes,
    voice: Instrument,
    Chord(root, third, fifth): Chord,
    length: f32,
    gain: f32,
) {
    for pitch in [
        near(root, 59),
        near(root + third, 62),
        near(root + fifth, 66),
    ] {
        add(out, voice, pitch, length, gain);
    }
}
fn kit(out: &mut Notes, tick: u8, kick: &[u8], snare: &[u8], gain: f32, brush: bool) {
    if kick.contains(&tick) {
        out.add(Instrument::Kick, 36, 0.7, gain, Role::Rhythm);
    }
    if snare.contains(&tick) {
        out.add(
            Instrument::Snare,
            38,
            if brush { 0.8 } else { 0.5 },
            gain * if brush { 0.28 } else { 0.62 },
            Role::Rhythm,
        );
    }
    out.add(
        Instrument::Hat,
        42,
        0.25,
        gain * if tick.is_multiple_of(2) { 0.15 } else { 0.22 },
        Role::Rhythm,
    );
}
#[allow(clippy::too_many_lines)] // Ten distinct accompaniments, kept together for score review.
pub(super) fn accompany(
    out: &mut Notes,
    theme: Theme,
    movement: Movement,
    bar: u32,
    tick: u8,
    harmony: Chord,
    arc: f32,
) {
    let p = profile(theme);
    let Chord(root, third, fifth) = harmony;
    let quiet = matches!(
        movement,
        Movement::Lobby | Movement::Defeat | Movement::Draw
    );
    let drive = matches!(movement, Movement::Combat | Movement::Endgame);
    let gain = arc * if quiet { 0.70 } else { 1.0 };
    let ticks = p.ticks;
    if tick == 0 {
        out.add(
            p.bass,
            near(root, if theme == Theme::Orbit { 47 } else { 40 }),
            if drive { 1.5 } else { f32::from(ticks) * 0.85 },
            0.19 * gain,
            Role::Bass,
        );
    }
    if movement == Movement::Defeat {
        if tick == 0 {
            triad(out, p.comp, harmony, f32::from(ticks) * 0.9, 0.08 * gain);
        }
        return;
    }
    match theme {
        Theme::Velvet => {
            // Wide left hand, answered by inner thirds. No ostinato underneath rests.
            if [0, 2, 5].contains(&tick) {
                let pitch = match tick {
                    0 => root,
                    2 => root + fifth,
                    _ => root + third + 12,
                };
                add(out, Instrument::Piano, pitch, 2.3, 0.20 * gain);
            }
            if !quiet && tick == 0 {
                add(
                    out,
                    Instrument::Viola,
                    near(root + third, 60),
                    7.4,
                    0.08 * gain,
                );
            }
        }
        Theme::Copper => {
            // Three-beat continuo, upper voice answers on the weak beats.
            add(
                out,
                Instrument::Clav,
                root + [0, fifth, third + 12, fifth, 12, third][usize::from(tick)],
                0.85,
                0.13 * gain,
            );
            if tick == 2 || tick == 4 {
                add(
                    out,
                    Instrument::Violin,
                    near(root + if tick == 2 { third } else { fifth }, 69),
                    1.7,
                    0.10 * gain,
                );
            }
        }
        Theme::Juniper => {
            if tick == 0 || tick == 3 {
                triad(out, Instrument::Guitar, harmony, 2.2, 0.12 * gain);
            }
            if !quiet && tick == 3 {
                add(out, Instrument::Violin, root + fifth + 12, 2.7, 0.11 * gain);
            }
            if drive {
                kit(out, tick, &[0, 3], &[3], 0.22 * gain, true);
            }
        }
        Theme::Lagoon => {
            // Anticipated guitar chords and root/fifth bass: the 3+3+2 bossa cell.
            if [1, 4, 7].contains(&tick) {
                triad(out, Instrument::Guitar, harmony, 1.6, 0.12 * gain);
            }
            if tick == 3 || tick == 6 {
                out.add(
                    p.bass,
                    near(root + if tick == 3 { fifth } else { 0 }, 42),
                    1.6,
                    0.17 * gain,
                    Role::Bass,
                );
            }
            if tick == 0 && !quiet {
                triad(out, Instrument::ElectricPiano, harmony, 5.8, 0.055 * gain);
            }
            kit(out, tick, &[0, 3, 4], &[2, 6], 0.18 * gain, true);
        }
        Theme::Lantern => {
            // Conductor swings the eighths; chords on two and three, bass on one.
            if tick == 2 || tick == 4 {
                triad(out, Instrument::Piano, harmony, 1.3, 0.11 * gain);
            }
            if tick == 4 {
                out.add(p.bass, near(root + fifth, 42), 1.4, 0.14 * gain, Role::Bass);
            }
            kit(out, tick, &[0], &[3, 5], 0.17 * gain, true);
        }
        Theme::Neon => {
            if tick.is_multiple_of(2) {
                out.add(
                    p.bass,
                    near(root, 36) + if tick == 6 { 12 } else { 0 },
                    1.4,
                    0.21 * gain,
                    Role::Bass,
                );
            }
            if tick == 0 {
                triad(out, Instrument::Pad, harmony, 7.6, 0.115 * gain);
            }
            if !quiet && tick % 2 == 1 {
                add(
                    out,
                    Instrument::Bell,
                    root + [fifth, third + 12, 12, fifth + 12][usize::from(tick / 2)],
                    1.5,
                    0.08 * gain,
                );
            }
            if !quiet {
                kit(out, tick, &[0, 4], &[2, 6], 0.33 * gain, false);
            }
        }
        Theme::Circuit => {
            // Three-channel vocabulary: pulse lead, low triangle and clipped arpeggios.
            if tick % 2 == 1 {
                add(
                    out,
                    Instrument::ChipLead,
                    root + [0, third, fifth, 12][usize::from(tick / 2)],
                    0.55,
                    0.10 * gain,
                );
            }
            if tick == 4 {
                out.add(p.bass, near(root + fifth, 40), 2.5, 0.19 * gain, Role::Bass);
            }
            if !quiet {
                kit(out, tick, &[0, 3, 4], &[2, 6], 0.22 * gain, false);
            }
        }
        Theme::Mosaic => {
            // Interlocking 2+2+3: low marimba enters in the gaps of the high cell.
            if [0, 2, 4].contains(&tick) {
                add(
                    out,
                    Instrument::Marimba,
                    root + if tick == 2 {
                        third
                    } else if tick == 4 {
                        fifth
                    } else {
                        0
                    },
                    1.8,
                    0.21 * gain,
                );
            }
            if [1, 3, 6].contains(&tick) {
                add(
                    out,
                    Instrument::Marimba,
                    root + if tick == 3 { third + 12 } else { fifth },
                    0.95,
                    0.12 * gain,
                );
            }
            if bar % 4 == 3 && tick == 4 {
                add(out, Instrument::Bell, root + 12, 2.8, 0.09 * gain);
            }
        }
        Theme::Orbit => {
            if tick == 0 {
                triad(out, Instrument::Pad, harmony, 7.8, 0.12 * gain);
            }
            if tick == 3 || tick == 6 {
                add(
                    out,
                    Instrument::Harp,
                    root + if tick == 3 { fifth + 12 } else { third + 12 },
                    3.5,
                    0.13 * gain,
                );
            }
            if drive && tick.is_multiple_of(2) {
                add(out, Instrument::Marimba, root + fifth, 1.4, 0.16 * gain);
            }
        }
        Theme::Iron => {
            if [0, 3, 5, 6].contains(&tick) {
                triad(out, Instrument::ViolinShort, harmony, 0.7, 0.12 * gain);
            }
            if tick == 4 {
                out.add(p.bass, near(root + fifth, 36), 2.2, 0.21 * gain, Role::Bass);
            }
            if !quiet {
                kit(out, tick, &[0, 3, 5], &[2, 6, 7], 0.36 * gain, false);
            }
            if drive && tick == 0 {
                add(
                    out,
                    Instrument::AnalogLead,
                    near(root + third, 62),
                    3.5,
                    0.11 * gain,
                );
            }
        }
        _ => unreachable!("only the ten new styles enter this arranger"),
    }
    if drive
        && tick == ticks / 2
        && !matches!(
            theme,
            Theme::Neon | Theme::Iron | Theme::Lagoon | Theme::Lantern | Theme::Circuit
        )
    {
        out.add(p.bass, near(root + fifth, 43), 1.4, 0.19 * gain, Role::Bass);
    }
}
