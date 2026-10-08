//! The orchestral body under the medieval soloists (cinematic hybrid
//! orchestral): string sections holding swelling chords on a slow harmonic
//! rhythm, spiccato ostinati that drive by layers, low brass pedals and
//! swells, heroic horn statements of the theme, and big drums that build.
//! Every voice follows the bar's root (`Tune::root`) and its layer level, so
//! a scene grows from intimate to huge by adding layers and density, never
//! by switching tracks.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
use super::melodies::{Phrase, SPIC_HEMIOLA, SPIC_SEVEN, SPIC_SIX};
use super::themes::Voicing;
use super::{Texture, Touch, Tune};
use crate::music::bank::{self, at};

/// Voice tags for the held string chord and the low brass pedal.
pub(super) const STRINGS: u8 = 3;
pub(super) const LOW: u8 = 4;

/// `pitch` moved by octaves into `lo..=hi` (at least an octave wide).
pub(super) fn octave_into(pitch: u8, lo: u8, hi: u8) -> u8 {
    let mut pitch = pitch;
    while pitch < lo {
        pitch += 12;
    }
    while pitch > hi {
        pitch -= 12;
    }
    pitch
}

impl Tune {
    /// The orchestral body for this tick.
    pub(super) fn orchestral(&mut self) {
        self.string_chord();
        self.low_brass();
        self.spiccato();
        self.big_drums();
    }

    /// The string sections hold the bar's chord — cellos on the root, violas
    /// the fifth, violins the third — struck again when the root moves or
    /// the layer has grown or shrunk, each entering with its own swell.
    fn string_chord(&mut self) {
        if self.tick != 0 {
            return;
        }
        let level = self.layers.strings;
        let root = self.root;
        if level < 0.03 {
            if self.strings_held.take().is_some() {
                self.orchestra.release(STRINGS);
            }
            return;
        }
        let moved = self
            .strings_held
            .is_none_or(|(held, gain)| held != root || (gain - level).abs() > 0.15);
        if !moved {
            return;
        }
        self.orchestra.release(STRINGS);
        let cello = octave_into(root, 46, 58);
        let viola = octave_into(self.shift(root, 4), 53, 64);
        let violin = octave_into(self.shift(root, 2), 62, 73);
        self.hold(
            STRINGS,
            bank::CELLO,
            cello,
            0.075 * level,
            Touch::at(-0.25).attack(0.9),
        );
        self.hold(
            STRINGS,
            bank::VIOLAS,
            viola,
            0.065 * level,
            Touch::at(0.1).attack(1.2),
        );
        self.hold(
            STRINGS,
            bank::VIOLINS,
            violin,
            0.05 * level,
            Touch::at(0.35).attack(1.5),
        );
        if level > 0.7 {
            // The full section: the violins' octave above the cellos too.
            let upper = octave_into(root, 62, 73);
            self.hold(
                STRINGS,
                bank::VIOLINS,
                upper,
                0.035 * level,
                Touch::at(-0.4).attack(1.5),
            );
        }
        self.strings_held = Some((root, level));
    }

    /// The tuba's pedal on the root, and trombone swells on the chord in the
    /// tense scenes.
    fn low_brass(&mut self) {
        let level = self.layers.brass;
        let tense = matches!(
            self.texture,
            Texture::Tension | Texture::Hunt | Texture::Climax | Texture::Victory
        );
        if self.tick != 0 {
            return;
        }
        let root = self.root;
        if level < 0.25 || !tense {
            if self.low_held.take().is_some() {
                self.orchestra.release(LOW);
            }
        } else if self.low_held != Some(root) {
            self.orchestra.release(LOW);
            let pedal = octave_into(root, 34, 45);
            self.hold(
                LOW,
                bank::TUBA,
                pedal,
                0.10 * level,
                Touch::at(0.0).attack(0.6),
            );
            self.low_held = Some(root);
        }
        let every = if self.texture == Texture::Climax {
            2
        } else {
            4
        };
        if tense && level >= 0.4 && self.here.is_multiple_of(every) {
            let ticks = f32::from(self.ticks) * every as f32 - 1.0;
            let low = octave_into(root, 46, 57);
            let fifth = octave_into(self.shift(root, 4), 50, 61);
            self.play(
                bank::TROMBONE,
                low,
                ticks,
                0.075 * level,
                Touch::at(-0.15).attack(0.5),
            );
            self.play(
                bank::TROMBONE,
                fifth,
                ticks,
                0.06 * level,
                Touch::at(0.15).attack(0.7),
            );
        }
    }

    /// The spiccato strings: cellos on the figure, violas (and at full drive
    /// the violins) an octave and a fifth above, accents on each group.
    fn spiccato(&mut self) {
        let drive = self.layers.drive;
        if drive < 0.03 {
            return;
        }
        let t = self.tick;
        let seven = self.ticks == 14;
        if !t.is_multiple_of(2) {
            // Sixteenths between the eighths once the drive is full.
            if drive > 0.75
                && matches!(
                    self.texture,
                    Texture::Tension | Texture::Climax | Texture::Hunt
                )
            {
                let pitch = octave_into(self.root, 43, 55);
                self.play(
                    bank::CELLOS_SPIC,
                    pitch,
                    0.8,
                    0.035 * drive,
                    Touch::at(-0.2),
                );
            }
            return;
        }
        let k = usize::from(t / 2);
        let figure: &[i8] = if seven || self.ticks == 10 {
            &SPIC_SEVEN
        } else if self.theme.book().hemiola && self.texture == Texture::Calm {
            &SPIC_HEMIOLA
        } else {
            &SPIC_SIX
        };
        let Some(&step) = figure.get(k) else { return };
        let accent = if seven {
            matches!(k, 0 | 3 | 5)
        } else if self.theme.book().hemiola && self.texture == Texture::Calm {
            k.is_multiple_of(2)
        } else {
            k.is_multiple_of(3)
        };
        let gain = 0.065 * drive * if accent { 1.35 } else { 1.0 };
        let low = octave_into(self.shift(self.root, step), 43, 57);
        self.play(bank::CELLOS_SPIC, low, 1.2, gain, Touch::at(-0.25));
        let mid = octave_into(self.shift(self.root, step + 4), 50, 62);
        self.play(bank::VIOLAS_SPIC, mid, 1.2, gain * 0.8, Touch::at(0.2));
        if drive > 0.6 {
            let high = octave_into(self.shift(self.root, step + 7), 62, 72);
            self.play(bank::VIOLINS_SPIC, high, 1.0, gain * 0.55, Touch::at(0.4));
        }
    }

    /// The big drums: taiko on the strong beats as the scene builds, the bass
    /// drum's hit at the climax, a timpani roll into each new phrase, a soft
    /// cymbal swell into the climax's phrase, the gong where it arrives.
    fn big_drums(&mut self) {
        let level = self.layers.perc;
        let t = self.tick;
        let b = self.here;
        if level < 0.03 {
            return;
        }
        let g = level;
        match self.texture {
            Texture::Tension => match t {
                0 => self.strike(at::TAIKO_F, 60, 3.0, 0.20 * g, Touch::at(0.0)),
                6 => self.strike(at::TAIKO_MF, 60, 3.0, 0.13 * g, Touch::at(-0.15)),
                10 if self.ticks == 14 => {
                    self.strike(at::TAIKO_STICKS, 60, 2.0, 0.10 * g, Touch::at(0.2));
                }
                _ => {}
            },
            Texture::Hunt => match t {
                0 => self.strike(at::TAIKO_F, 60, 3.0, 0.18 * g, Touch::at(0.0)),
                6 => self.strike(at::TAIKO_MF, 60, 3.0, 0.12 * g, Touch::at(-0.15)),
                9 => self.strike(at::TAIKO_STICKS, 60, 2.0, 0.07 * g, Touch::at(0.2)),
                _ => {}
            },
            Texture::Climax => match t {
                0 => {
                    self.strike(at::TAIKO_FF, 60, 3.0, 0.22 * g, Touch::at(0.0));
                    if b.is_multiple_of(2) {
                        self.strike(at::BIGDRUM_HIT, 60, 4.0, 0.20 * g, Touch::at(0.0));
                    }
                    if b == 0 {
                        self.strike(at::GONG, 60, 12.0, 0.12 * g, Touch::at(-0.2));
                    }
                }
                3 | 9 => self.strike(at::TAIKO_STICKS, 60, 2.0, 0.08 * g, Touch::at(0.25)),
                6 => self.strike(at::TAIKO_F, 60, 3.0, 0.16 * g, Touch::at(-0.1)),
                _ => {}
            },
            Texture::Calm | Texture::Arrival if t == 0 && b.is_multiple_of(2) => {
                self.strike(at::TAIKO_MF, 60, 3.0, 0.12 * g, Touch::at(0.0));
            }
            _ => {}
        }
        // A roll into each new phrase, from the second-last bar of eight.
        if t == 0 && b % 8 == 6 && level > 0.3 {
            self.strike(
                at::TIMPANI_ROLL,
                60,
                24.0,
                0.09 * g,
                Touch::at(0.1).attack(0.8),
            );
        }
        if t == 0 && self.texture == Texture::Climax && b % 16 == 12 {
            self.strike(
                at::CYMBAL_SWELL,
                60,
                24.0,
                0.035 * g,
                Touch::at(-0.3).attack(1.5),
            );
        }
    }

    /// The horns, with a trombone beneath, state a phrase heroically: the
    /// theme in the horns' middle register, the trombone an octave below.
    pub(super) fn horns(&mut self, phrase: Phrase, bar: usize, v: Voicing, gain: f32) {
        let horn = Voicing { centre: 57, ..v };
        self.sing(
            phrase,
            bar,
            bank::HORN,
            gain,
            Touch::at(-0.1).attack(0.08),
            horn,
        );
        let bone = Voicing { centre: 50, ..v };
        self.sing(
            phrase,
            bar,
            bank::TROMBONE,
            gain * 0.5,
            Touch::at(0.2).attack(0.1),
            bone,
        );
    }
}
