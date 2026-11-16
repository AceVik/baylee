//! Idiomatic accompaniment with Tymoczko minimal-L1 Drop-2 open voicings,
//! Plomp-Levelt critical-band bass spacing, and clean bar-boundary articulation.
use super::{
    Chord, Instrument, Movement, Notes, Role, Theme,
    arrangement::{
        bass_root, clamp_for_next_chord, clear_register, near, parsimonious_open_triad,
        sweeten_cents,
    },
    profile,
};

fn add(out: &mut Notes, voice: Instrument, pitch: u8, length: f32, gain: f32) {
    out.add(voice, clear_register(pitch), length, gain, Role::Harmony);
}

#[allow(clippy::too_many_arguments)]
fn triad(
    out: &mut Notes,
    theme: Theme,
    movement: Movement,
    bar: u32,
    tick: u8,
    ticks: u8,
    voice: Instrument,
    harmony: Chord,
    next_harmony: Chord,
    length: f32,
    gain: f32,
) {
    let open = parsimonious_open_triad(theme, movement, bar);
    let pure_synth = matches!(theme, Theme::Neon | Theme::Circuit | Theme::Lagoon);
    for pitch in open {
        let len = clamp_for_next_chord(pitch, length, tick, ticks, harmony, next_harmony);
        let cents = sweeten_cents(voice, pitch, harmony, pure_synth);
        out.add_tuned(voice, pitch, len, gain, Role::Harmony, cents);
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

#[allow(clippy::too_many_lines, clippy::too_many_arguments)] // Ten distinct accompaniments, kept together for score review.
pub(super) fn accompany(
    out: &mut Notes,
    theme: Theme,
    movement: Movement,
    bar: u32,
    tick: u8,
    harmony: Chord,
    next_harmony: Chord,
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
    let fit =
        |pitch: u8, len: f32| clamp_for_next_chord(pitch, len, tick, ticks, harmony, next_harmony);
    let bass = if theme == Theme::Orbit {
        near(root, 46)
    } else {
        bass_root(root)
    };
    let open = parsimonious_open_triad(theme, movement, bar);
    if tick == 0 {
        let first_bass_hold = if movement == Movement::Defeat {
            f32::from(ticks) * 0.85
        } else {
            match theme {
                Theme::Lagoon => 2.8,
                Theme::Lantern | Theme::Circuit | Theme::Iron => 3.8,
                Theme::Neon => 1.8,
                _ if drive => (f32::from(ticks / 2) - 0.15).min(1.5),
                _ => f32::from(ticks) * 0.85,
            }
        };
        out.add(
            p.bass,
            bass,
            fit(bass, first_bass_hold),
            0.19 * gain,
            Role::Bass,
        );
    }
    if movement == Movement::Defeat {
        if tick == 0 {
            triad(
                out,
                theme,
                movement,
                bar,
                0,
                ticks,
                p.comp,
                harmony,
                next_harmony,
                f32::from(ticks) * 0.9,
                0.08 * gain,
            );
        }
        return;
    }
    match theme {
        Theme::Velvet => {
            // Wide open Drop-2 left/right hand, answered by inner voice.
            if [0, 2, 5].contains(&tick) {
                let pitch = match tick {
                    0 => open[0],
                    2 => open[1],
                    _ => open[2],
                };
                add(out, Instrument::Piano, pitch, fit(pitch, 2.3), 0.20 * gain);
            }
            if !quiet && tick == 0 {
                add(
                    out,
                    Instrument::Viola,
                    open[0],
                    fit(open[0], 7.4),
                    0.08 * gain,
                );
            }
        }
        Theme::Copper => {
            // Three-beat continuo, upper voice answers on the weak beats.
            let p_clav =
                clear_register(root + [0, fifth, third + 12, fifth, 12, third][usize::from(tick)]);
            add(
                out,
                Instrument::Clav,
                p_clav,
                fit(p_clav, 0.85),
                0.13 * gain,
            );
            if tick == 2 || tick == 4 {
                let p_vln = if tick == 2 { open[1] } else { open[2] };
                add(out, Instrument::Violin, p_vln, fit(p_vln, 1.7), 0.10 * gain);
            }
        }
        Theme::Juniper => {
            if tick == 0 || tick == 3 {
                triad(
                    out,
                    theme,
                    movement,
                    bar,
                    tick,
                    ticks,
                    Instrument::Guitar,
                    harmony,
                    next_harmony,
                    2.2,
                    0.12 * gain,
                );
            }
            if !quiet && tick == 3 {
                let p_vln = (root + fifth + 12).min(86);
                add(out, Instrument::Violin, p_vln, fit(p_vln, 2.7), 0.11 * gain);
            }
            if drive {
                kit(out, tick, &[0, 3], &[3], 0.22 * gain, true);
            }
        }
        Theme::Lagoon => {
            // Anticipated guitar open chords and non-overlapping root/fifth bossa bass.
            if [1, 4, 7].contains(&tick) {
                triad(
                    out,
                    theme,
                    movement,
                    bar,
                    tick,
                    ticks,
                    Instrument::Guitar,
                    harmony,
                    next_harmony,
                    1.6,
                    0.12 * gain,
                );
            }
            if tick == 3 || tick == 6 {
                let bp = if tick == 3 { bass + 7 } else { bass };
                let blen = if tick == 3 { 2.8 } else { 1.7 };
                out.add(p.bass, bp, fit(bp, blen), 0.17 * gain, Role::Bass);
            }
            if tick == 0 && !quiet {
                triad(
                    out,
                    theme,
                    movement,
                    bar,
                    0,
                    ticks,
                    Instrument::ElectricPiano,
                    harmony,
                    next_harmony,
                    5.8,
                    0.055 * gain,
                );
            }
            kit(out, tick, &[0, 3, 4], &[2, 6], 0.18 * gain, true);
        }
        Theme::Lantern => {
            // Conductor swings the eighths; open chords on two and three, non-overlapping bass.
            if tick == 2 || tick == 4 {
                triad(
                    out,
                    theme,
                    movement,
                    bar,
                    tick,
                    ticks,
                    Instrument::Piano,
                    harmony,
                    next_harmony,
                    1.3,
                    0.11 * gain,
                );
            }
            if tick == 4 {
                out.add(
                    p.bass,
                    bass + 7,
                    fit(bass + 7, 1.8),
                    0.14 * gain,
                    Role::Bass,
                );
            }
            kit(out, tick, &[0], &[3, 5], 0.17 * gain, true);
        }
        Theme::Neon => {
            if tick > 0 && tick.is_multiple_of(2) {
                let bp = bass + if tick == 6 { 12 } else { 0 };
                out.add(p.bass, bp, fit(bp, 1.7), 0.21 * gain, Role::Bass);
            }
            if tick == 0 {
                triad(
                    out,
                    theme,
                    movement,
                    bar,
                    0,
                    ticks,
                    Instrument::Pad,
                    harmony,
                    next_harmony,
                    7.6,
                    0.115 * gain,
                );
            }
            if !quiet && tick % 2 == 1 {
                let p_bell = clear_register(
                    root + [fifth, third + 12, 12, fifth + 12][usize::from(tick / 2)],
                );
                add(out, Instrument::Bell, p_bell, fit(p_bell, 1.5), 0.08 * gain);
            }
            if !quiet {
                kit(out, tick, &[0, 4], &[2, 6], 0.33 * gain, false);
            }
        }
        Theme::Circuit => {
            // Three-channel vocabulary: pulse lead, low triangle and clipped arpeggios.
            if tick % 2 == 1 {
                let p_chip = clear_register(root + [0, third, fifth, 12][usize::from(tick / 2)]);
                add(
                    out,
                    Instrument::ChipLead,
                    p_chip,
                    fit(p_chip, 0.55),
                    0.10 * gain,
                );
            }
            if tick == 4 {
                out.add(
                    p.bass,
                    bass + 7,
                    fit(bass + 7, 2.5),
                    0.19 * gain,
                    Role::Bass,
                );
            }
            if !quiet {
                kit(out, tick, &[0, 3, 4], &[2, 6], 0.22 * gain, false);
            }
        }
        Theme::Mosaic => {
            // Interlocking 2+2+3: low marimba enters in the gaps of the high cell, >= G3.
            if [0, 2, 4].contains(&tick) {
                let p_low = clear_register(
                    root + if tick == 2 {
                        third
                    } else if tick == 4 {
                        fifth
                    } else {
                        0
                    },
                );
                add(
                    out,
                    Instrument::Marimba,
                    p_low,
                    fit(p_low, 1.8),
                    0.21 * gain,
                );
            }
            if [1, 3, 6].contains(&tick) {
                let p_hi = clear_register(root + if tick == 3 { third + 12 } else { fifth });
                add(out, Instrument::Marimba, p_hi, fit(p_hi, 0.95), 0.12 * gain);
            }
            if bar % 4 == 3 && tick == 4 {
                let p_b = (root + 12).min(86);
                add(out, Instrument::Bell, p_b, fit(p_b, 2.6), 0.09 * gain);
            }
        }
        Theme::Orbit => {
            if tick == 0 {
                triad(
                    out,
                    theme,
                    movement,
                    bar,
                    0,
                    ticks,
                    Instrument::Pad,
                    harmony,
                    next_harmony,
                    7.8,
                    0.12 * gain,
                );
            }
            if tick == 3 || tick == 6 {
                let p_h = (root + if tick == 3 { fifth + 12 } else { third + 12 }).min(86);
                add(out, Instrument::Harp, p_h, fit(p_h, 3.5), 0.13 * gain);
            }
            if drive && tick.is_multiple_of(2) {
                let p_m = clear_register(root + fifth);
                add(out, Instrument::Marimba, p_m, fit(p_m, 1.4), 0.16 * gain);
            }
        }
        Theme::Iron => {
            if [0, 3, 5, 6].contains(&tick) {
                triad(
                    out,
                    theme,
                    movement,
                    bar,
                    tick,
                    ticks,
                    Instrument::ViolinShort,
                    harmony,
                    next_harmony,
                    0.7,
                    0.12 * gain,
                );
            }
            if tick == 4 {
                out.add(
                    p.bass,
                    bass + 7,
                    fit(bass + 7, 2.2),
                    0.21 * gain,
                    Role::Bass,
                );
            }
            if !quiet {
                kit(out, tick, &[0, 3, 5], &[2, 6, 7], 0.36 * gain, false);
            }
            if drive && tick == 0 {
                add(
                    out,
                    Instrument::AnalogLead,
                    open[0],
                    fit(open[0], 3.5),
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
        let bp = if theme == Theme::Mosaic && bass == 42 {
            bass + 12
        } else {
            bass + 7
        };
        out.add(p.bass, bp, fit(bp, 1.4), 0.19 * gain, Role::Bass);
    }
}
