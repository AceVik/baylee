//! Each scene's arrangement, tick by tick: the medieval consort (recorders,
//! psaltery, harp, strumstick, the pipe, frame drums and davul) singing the
//! chosen theme in the scene's mode, over the orchestral body
//! (`orchestral.rs`).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)] // ticks and bars; t, b, k, l, d: tick, bar, ending bar, layers, drums
#[allow(clippy::wildcard_imports)] // the score's note names, read as notation
use super::melodies::*;
use super::themes::Voicing;
use super::{ENDING_BARS, PIPES, Texture, Touch, Tune};
use crate::music::bank::{self, at};
use crate::music::orchestra::Family;

impl Tune {
    /// The lead this table sings with: the theme's, or its doubling's while
    /// the monarch has turned it.
    fn lead(&self) -> (Family, u8, f32) {
        let book = self.theme.book();
        if self.rotation % 2 == 1 {
            (book.double.0, book.double.1, book.lead_gain)
        } else {
            (book.lead, book.lead_centre, book.lead_gain)
        }
    }

    /// B♭ Lydian: the front door (the theme alone over drone, strings and
    /// harp) and the lobby (with its ostinato and a muted frame drum).
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    pub(super) fn lydian_home(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let lobby = self.texture == Texture::Lobby;
        let l = self.layers;
        let root = self.bass(FRONT_BASS[b % 8]);
        if t == 0 {
            self.root = root;
            self.bed(BB, &[(bank::ORGAN, BB2, 0.05, 0.0)]);
            self.play(
                bank::CONTRABASS,
                root,
                11.5,
                0.11 * l.bass,
                Touch::at(0.05).attack(0.25),
            );
            if lobby && b % 2 == 1 {
                self.strike(at::FRAMEDRUM_MUTED, 60, 2.0, 0.08 * l.drums, Touch::at(0.1));
            }
        }
        // Harp: the whole Lydian figure in the lobby, two strings of it at
        // the front door.
        if lobby && b % 2 == 1 {
            let degree = [0, 7, 12, 14, 18, 14, 12, 7, 12, 14, 18, 12][usize::from(t)];
            let gain = if t.is_multiple_of(2) { 0.10 } else { 0.065 };
            self.play(
                bank::HARP,
                BB3 + degree,
                3.0,
                gain * l.ostinato,
                Touch::at(-0.35),
            );
        } else if t.is_multiple_of(2) {
            let k = usize::from(t / 2);
            let degree = [0, 7, 12, 14, 18, 12][k];
            if lobby || k == 0 || k == 3 {
                let gain = [0.13, 0.10, 0.11, 0.09, 0.10, 0.09][k] * l.ostinato.max(0.5);
                self.play(bank::HARP, BB3 + degree, 5.0, gain, Touch::at(-0.35));
            }
            if lobby {
                let pitch = [BB4, D5, F5, E5, D5, C5][k];
                self.play(
                    bank::PSALTERY,
                    pitch,
                    3.0,
                    0.045 * l.ostinato,
                    Touch::at(0.4),
                );
            }
        }
        // The theme: 16 bars of it, 16 of drone and harp (32 at the lobby,
        // where it sings every other cycle).
        let cycle = if lobby { 64 } else { 32 };
        let pos = b % cycle;
        if pos < 16 && self.singing() {
            let book = self.theme.book();
            let phrase = if pos < 8 { book.a } else { book.b };
            let (family, centre, gain) = self.lead();
            let v = Voicing::at(centre).steps(-1);
            self.sing(phrase, pos % 8, family, gain * l.melody, Touch::at(0.15), v);
        }
    }

    /// B♭ Ionian, to think over: harp and psaltery over the drone, no melody.
    pub(super) fn build(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let l = self.layers;
        let root = self.bass(if b % 8 < 6 { BB1 } else { F2 });
        if t == 0 {
            self.root = root;
            self.bed(BB, &[(bank::ORGAN, BB2, 0.06, 0.0)]);
            self.play(
                bank::CONTRABASS,
                root,
                11.5,
                0.10 * l.bass,
                Touch::at(0.05).attack(0.25),
            );
        }
        if t.is_multiple_of(2) {
            let k = usize::from(t / 2);
            let degree = [0, 7, 12, 16, 14, 7][k];
            let harp_root = if b % 8 < 6 { BB3 } else { F3 };
            self.play(
                bank::HARP,
                harp_root + degree,
                5.0,
                0.11 * l.ostinato,
                Touch::at(-0.35),
            );
        }
        if b % 4 == 3 && (6..10).contains(&t) {
            let pitch = [BB4, C5, D5, F5][usize::from(t - 6)];
            self.play(
                bank::PSALTERY,
                pitch,
                4.0,
                0.06 * l.ostinato,
                Touch::at(0.4),
            );
        }
    }

    /// The table arriving: the big drums and a bell on B♭, a harp sweep,
    /// then the drone holds through the loading cover.
    pub(super) fn arrival(&mut self) {
        let t = self.tick;
        let l = self.layers;
        if t == 0 {
            self.root = self.bass(BB1);
            self.bed(BB, &[(bank::ORGAN, BB2, 0.07, 0.0)]);
            let root = self.root;
            self.play(
                bank::CONTRABASS,
                root,
                11.5,
                0.12 * l.bass,
                Touch::at(0.05).attack(0.3),
            );
        }
        match self.here {
            0 => {
                if t == 0 {
                    self.strike(at::DAVUL_FORTE, 60, 4.0, 0.22, Touch::at(0.0));
                    self.strike(at::TAIKO_FF, 60, 4.0, 0.18, Touch::at(0.0));
                    self.play(bank::CHIMES, BB4, 16.0, 0.08, Touch::at(0.25));
                }
                if t.is_multiple_of(2) {
                    let pitch = [BB2, C3, D3, F3, G3, BB3][usize::from(t / 2)];
                    self.play(bank::HARP, pitch + 12, 6.0, 0.12, Touch::at(-0.3));
                }
            }
            1 if t == 0 => {
                self.strike(at::DAVUL_1, 60, 3.0, 0.14, Touch::at(0.0));
                self.play(bank::CHIMES, F4, 16.0, 0.05, Touch::at(-0.25));
            }
            _ if t == 0 && self.here.is_multiple_of(2) => {
                self.play(bank::HARP, BB3, 8.0, 0.08, Touch::at(-0.3));
                self.play(bank::HARP, F4, 8.0, 0.06, Touch::at(-0.3));
            }
            _ => {}
        }
    }

    /// The table at rest, C Dorian in 6/8, over a 48-bar arc from intimate
    /// to heroic and back: the theme alone (8 bars), its answer with the
    /// strings swelling in (8), the theme on the horns with the whole body
    /// (8), the ostinato alone (8), the answer quietly (8), drone and harp (8).
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    pub(super) fn calm(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let l = self.layers;
        let book = self.theme.book();
        let reading = if (b / 48).is_multiple_of(2) {
            CALM_BASS
        } else {
            CALM_BASS_2
        };
        let normal = reading[b % 8];
        let root = self.bass(normal);
        if t == 0 {
            self.root = root;
            self.play(
                bank::STRUMSTICK,
                root + 12,
                11.5,
                0.15 * l.drone,
                Touch::at(-0.2),
            );
            if (b / 24).is_multiple_of(2) {
                self.bed(C, &[(bank::ORGAN, C3, 0.03, 0.0)]);
            } else {
                self.unbed();
            }
            self.play(
                bank::CONTRABASS,
                root,
                12.2,
                0.12 * l.bass,
                Touch::at(0.05).attack(0.25),
            );
            if self.request.own_turn && b.is_multiple_of(2) {
                self.play(bank::PIZZICATO, root, 6.0, 0.11 * l.bass, Touch::at(0.1));
            }
            if b % 2 == 1 {
                self.strike(at::FRAMEDRUM_MUTED, 60, 2.0, 0.09 * l.drums, Touch::at(0.1));
            }
        }
        // The dance pulls the drum into twos: a stroke on every quarter.
        let small = if book.hemiola {
            t == 4 || t == 8
        } else {
            t == 6
        };
        if small && b % 2 == 1 {
            self.strike(
                at::FRAMEDRUM_SMALL_MUTED,
                60,
                1.0,
                0.05 * l.drums,
                Touch::at(0.25),
            );
        }
        let pos = b % 48;
        let thin = pos >= 40;
        // Every other bar the harp runs in sixteenths: inner motion, not a pad.
        let harp = if self.light { root } else { normal };
        if b % 2 == 1 && !thin {
            let degree = [0, 7, 12, 7, 14, 7, 12, 7, 14, 12, 7, 12][usize::from(t)];
            let gain = if t.is_multiple_of(2) { 0.11 } else { 0.07 };
            self.play(
                bank::HARP,
                harp + 24 + degree,
                3.0,
                gain * l.ostinato,
                Touch::at(-0.35),
            );
        } else if t.is_multiple_of(2) && (!thin || t.is_multiple_of(6)) {
            let k = usize::from(t / 2);
            let degree = [0, 7, 12, 7, 14, 12][k];
            let gain = [0.14, 0.10, 0.11, 0.09, 0.10, 0.09][k];
            self.play(
                bank::HARP,
                harp + 24 + degree,
                5.0,
                gain * l.ostinato,
                Touch::at(-0.35),
            );
        }
        // The lute plucks the offbeats under it.
        if (t == 3 || t == 9) && !thin {
            let gain = 0.055 * l.ostinato * self.dynamics() * self.texture.level();
            let pitch = if t == 3 { root + 12 } else { root + 19 };
            self.orchestra
                .lute(pitch, self.seconds(4.0), gain, -0.3, 0.45);
        }
        if self.singing() {
            let (family, centre, gain) = self.lead();
            let gain = gain * l.melody;
            let (double, double_centre, double_gain) = book.double;
            match pos / 8 {
                0 => self.sing(
                    book.a,
                    pos % 8,
                    family,
                    gain,
                    Touch::at(0.15),
                    Voicing::at(centre),
                ),
                1 | 4 => {
                    self.sing(
                        book.b,
                        pos % 8,
                        family,
                        gain,
                        Touch::at(0.15),
                        Voicing::at(centre),
                    );
                    let touch = Touch::at(-0.3).late(0.03);
                    let v = Voicing::at(double_centre);
                    self.sing(book.b, pos % 8, double, gain * double_gain, touch, v);
                }
                2 => {
                    // The heroic statement: the horns take the theme.
                    self.horns(book.a, pos % 8, Voicing::at(57), 0.17 * l.brass.max(0.4));
                    self.sing(
                        book.a,
                        pos % 8,
                        family,
                        gain * 0.5,
                        Touch::at(0.2),
                        Voicing::at(centre),
                    );
                }
                _ => {}
            }
        }
        // Answers at the ends of phrases, by whose turn it is.
        if (pos % 8 == 3 || pos % 8 == 7) && (6..10).contains(&t) && self.singing() && pos < 40 {
            let pitch = [C5, D5, EB5, G5][usize::from(t - 6)];
            let gain = 0.09 * l.ostinato;
            match self.request.turn_seat % 3 {
                0 => self.play(bank::PSALTERY, pitch, 3.0, gain * 0.8, Touch::at(0.4)),
                1 => self.play(bank::HARP, pitch, 3.0, gain, Touch::at(0.3)),
                _ => self.play(bank::VIOLIN, pitch - 12, 2.0, gain * 0.6, Touch::at(0.35)),
            }
        }
    }

    /// Rising tension, G Aeolian in 7/8 (3+2+2): the theme re-rhythmed on the
    /// tenor, answered by the chanter from 0.55 (the fiddle before it), the
    /// horns from 0.65; psaltery and lute ostinato, contrabass on the
    /// accents; the drums enter one by one by tension; from 0.7 the strings
    /// lean E♭–D at phrase ends (D Phrygian).
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    pub(super) fn tension(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let l = self.layers;
        let tension = self.request.tension;
        let heavy = ((tension - 0.35) / 0.5).clamp(0.0, 1.0);
        let dark = tension >= 0.7 && b % 4 == 3 && self.singing();
        let root = self.bass(TENSION_BASS[b % 8]);
        let five = self.ticks == 10;
        let d = l.drums;
        match t {
            0 => {
                let drum = if b.is_multiple_of(2) {
                    at::FRAMEDRUM_1
                } else {
                    at::FRAMEDRUM_2
                };
                self.strike(drum, 60, 2.0, 0.19 * d, Touch::at(-0.1));
                if tension >= 0.5 {
                    let davul = if b.is_multiple_of(2) {
                        at::DAVUL_1
                    } else {
                        at::DAVUL_2
                    };
                    self.strike(davul, 60, 3.0, (0.11 + 0.12 * heavy) * d, Touch::at(0.0));
                }
            }
            4 => self.strike(at::FRAMEDRUM_SMALL_MUTED, 60, 1.0, 0.07 * d, Touch::at(0.3)),
            6 => {
                self.strike(at::FRAMEDRUM_SMALL, 60, 1.0, 0.14 * d, Touch::at(0.2));
                if tension >= 0.6 {
                    let naker = if b.is_multiple_of(2) {
                        at::NAKER_1
                    } else {
                        at::NAKER_2
                    };
                    self.strike(naker, 72, 1.5, (0.07 + 0.07 * heavy) * d, Touch::at(0.25));
                }
                if tension >= 0.7 {
                    self.strike(at::TAMBOURINE_HIT, 60, 1.0, 0.04 * d, Touch::at(-0.3));
                }
            }
            10 if !five => {
                self.strike(at::FRAMEDRUM_SMALL_MUTED, 60, 1.0, 0.11 * d, Touch::at(0.2));
                if tension >= 0.6 {
                    let gain = (0.05 + 0.06 * heavy) * d;
                    self.strike(at::NAKER_HIGH, 74, 1.0, gain, Touch::at(0.3));
                }
            }
            11 if tension >= 0.7 && !five => {
                let rope = if b.is_multiple_of(2) {
                    at::ROPESNARE_1
                } else {
                    at::ROPESNARE_2
                };
                self.strike(rope, 60, 0.8, 0.06 * d, Touch::at(0.3));
            }
            _ => {}
        }
        // Bass: the contrabass on the 3+2+2 accents.
        if t == 0 {
            self.root = root;
            self.unbed();
            self.play(bank::PIZZICATO, root, 3.0, 0.15 * l.bass, Touch::at(0.05));
            let ticks = f32::from(self.ticks) * 0.9;
            self.play(bank::STRUMSTICK, G3, ticks, 0.13 * l.drone, Touch::at(0.0));
        }
        if t == 6 {
            let pitch = if dark { EB2 } else { root };
            self.play(bank::PIZZICATO, pitch, 3.0, 0.11 * l.bass, Touch::at(0.05));
        }
        if t == 10 && !five {
            let pitch = if dark { D2 } else { self.shift(root, 4) };
            self.play(bank::PIZZICATO, pitch, 3.0, 0.11 * l.bass, Touch::at(0.05));
            if dark {
                self.play(bank::VIOLINS, EB4, 2.0, 0.05 * l.strings, Touch::at(-0.1));
            }
        }
        if t == 12 && dark && !five {
            self.play(bank::VIOLINS, D4, 2.0, 0.05 * l.strings, Touch::at(-0.1));
        }
        // The plucked ostinato and the lute beneath it; from 0.55 a sixteenth
        // after each accent drives it on.
        if tension >= 0.55 && matches!(t, 1 | 7 | 11) {
            let pitch = TENSION_OSTINATO[usize::from(t / 2) % TENSION_OSTINATO.len()];
            self.play(
                bank::PSALTERY,
                pitch - 12,
                1.5,
                0.05 * l.ostinato,
                Touch::at(0.3),
            );
        }
        if t.is_multiple_of(2) {
            let k = usize::from(t / 2);
            let pitch = TENSION_OSTINATO[k % TENSION_OSTINATO.len()];
            let gain = if matches!(k, 0 | 3 | 5) { 0.10 } else { 0.07 };
            self.play(
                bank::PSALTERY,
                pitch - 12,
                3.0,
                gain * l.ostinato,
                Touch::at(0.35),
            );
            let gain = 0.08 * l.ostinato * self.texture.level();
            self.orchestra
                .lute(pitch - 24, self.seconds(5.0), gain, -0.35, 0.4);
        }
        // The line: the tenor, then the chanter (or the fiddle), eight bars
        // each; the horns join the tenor's turn as the brass rises.
        if self.singing() && !five {
            let book = self.theme.book();
            let v = Voicing::at(66).steps(-3).seven();
            if (b / 8).is_multiple_of(2) {
                self.sing(
                    book.a,
                    b % 8,
                    bank::TENOR,
                    0.23 * l.melody,
                    Touch::at(0.1),
                    v,
                );
                if l.brass > 0.5 {
                    self.horns(book.a, b % 8, v, 0.13 * l.brass);
                }
            } else if l.chanter > 0.0 {
                let pipe = Touch::at(0.05).attack(0.015).release(0.05);
                let v = Voicing::at(72).steps(-3).seven().fold();
                self.sing(book.b, b % 8, bank::CHANTER, 0.21 * l.chanter, pipe, v);
            } else {
                let v = Voicing::at(64).steps(-3).seven();
                let touch = Touch::at(-0.25).late(0.02);
                self.sing(book.b, b % 8, bank::VIOLIN, 0.11 * l.melody, touch, v);
            }
        }
    }

    /// The hunt: attackers are declared, F Mixolydian in a 6/8 jig; the horn
    /// calls (`horn_call`), then the theme in F: the chanter when the pipe is
    /// up, else the tenor, the horns beneath as the brass rises.
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    pub(super) fn hunt(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let l = self.layers;
        let tension = self.request.tension;
        let d = l.drums;
        let root = self.bass(F2);
        match t {
            0 => {
                self.root = root;
                self.unbed();
                let drum = if b.is_multiple_of(2) {
                    at::FRAMEDRUM_1
                } else {
                    at::FRAMEDRUM_2
                };
                self.strike(drum, 60, 2.0, 0.20 * d, Touch::at(-0.1));
                if tension >= 0.5 {
                    let davul = if b.is_multiple_of(2) {
                        at::DAVUL_1
                    } else {
                        at::DAVUL_2
                    };
                    self.strike(davul, 60, 3.0, 0.18 * d, Touch::at(0.0));
                }
                self.play(bank::PIZZICATO, root, 3.0, 0.15 * l.bass, Touch::at(0.05));
                self.play(
                    bank::CONTRABASS,
                    root,
                    12.2,
                    0.10 * l.bass,
                    Touch::at(0.05).attack(0.2),
                );
                self.play(bank::STRUMSTICK, F3, 10.0, 0.14 * l.drone, Touch::at(-0.1));
            }
            4 | 10 => self.strike(at::FRAMEDRUM_SMALL_MUTED, 60, 1.0, 0.09 * d, Touch::at(0.3)),
            6 => {
                self.strike(at::FRAMEDRUM_SMALL, 60, 1.0, 0.15 * d, Touch::at(0.2));
                self.play(bank::PIZZICATO, C2, 3.0, 0.11 * l.bass, Touch::at(0.05));
                if tension >= 0.6 {
                    self.strike(at::NAKER_1, 72, 1.5, 0.08 * d, Touch::at(0.25));
                }
            }
            11 if tension >= 0.7 => {
                self.strike(at::ROPESNARE_1, 60, 0.8, 0.06 * d, Touch::at(0.3));
            }
            _ => {}
        }
        // The jig's lilt: a light tambourine on the third eighth of each beat.
        if matches!(t, 4 | 10) {
            self.strike(at::TAMBOURINE_HIT, 60, 0.8, 0.03 * d, Touch::at(-0.35));
        }
        if t.is_multiple_of(2) {
            let k = usize::from(t / 2);
            let pitch = [F4, A4, C5, A4, F4, C5][k];
            let gain = if k.is_multiple_of(3) { 0.11 } else { 0.08 };
            self.play(bank::HARP, pitch, 2.5, gain * l.ostinato, Touch::at(0.4));
        }
        if self.horn.is_none() && self.singing() {
            let book = self.theme.book();
            if l.chanter > 0.0 {
                let pipe = Touch::at(0.05).attack(0.015).release(0.05);
                let v = Voicing::at(72).steps(-4).fold();
                self.sing(book.a, b % 8, bank::CHANTER, 0.2 * l.chanter, pipe, v);
            } else {
                let v = Voicing::at(66).steps(-4);
                self.sing(
                    book.a,
                    b % 8,
                    bank::TENOR,
                    0.18 * l.melody,
                    Touch::at(0.15),
                    v,
                );
            }
            if l.brass > 0.5 && b % 16 >= 8 {
                self.horns(book.a, b % 8, Voicing::at(57).steps(-4), 0.12 * l.brass);
            }
        }
    }

    /// The hunting-horn call: two bars on the natural horn's notes in F, on
    /// the bar after attackers are declared. This seat's own attack calls
    /// near and full; another's from further off. Never the same call twice
    /// running.
    pub(super) fn horn_call(&mut self) {
        let Some((bar, mine)) = self.horn else {
            return;
        };
        let call = CALLS[(self.horn_calls as usize + CALLS.len() - 1) % CALLS.len()];
        let gain = if mine { 0.30 } else { 0.17 };
        let pan = if mine { 0.1 } else { -0.55 };
        self.line(
            call[usize::from(bar)],
            bank::HORN,
            gain,
            Touch::at(pan).attack(0.04),
            0,
        );
        if self.tick + 1 == self.ticks {
            self.horn = if bar == 0 { Some((1, mine)) } else { None };
        }
    }

    /// One-bar accents: the monarch's bell and harp, and a big spell's B♭
    /// Lydian light.
    pub(super) fn accents(&mut self) {
        let t = self.tick;
        if self.monarch && t == 0 {
            self.strike(at::HANDBELL_1, 60, 6.0, 0.06, Touch::at(0.35));
        }
        if self.monarch && (2..8).contains(&t) {
            let pitch = [F4, G4, BB4, C5, D5, F5][usize::from(t - 2)];
            self.play(bank::HARP, pitch, 3.0, 0.09, Touch::at(-0.3));
        }
        if self.light && t < 6 {
            // B♭ C D E F, rising, and the bell: E♮ is the light.
            let pitch = [BB3, C4, D4, E5 - 12, F4, BB4][usize::from(t)];
            self.play(bank::HARP, pitch, 4.0, 0.11, Touch::at(-0.25));
            if t == 0 {
                self.strike(at::HANDBELL_2, 60, 6.0, 0.05, Touch::at(0.3));
                self.play(bank::CHIMES, BB4, 12.0, 0.045, Touch::at(0.2));
            }
        }
    }

    /// The climax: G Aeolian in a fast jig — both pipes, the davul and the
    /// whole orchestra; the chanter and the violins sing the theme, the
    /// horns and trombones state it beneath.
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    pub(super) fn climax(&mut self) {
        let t = self.tick;
        let b = self.here as usize;
        let l = self.layers;
        let d = l.drums;
        let root = self.bass(G2);
        match t {
            0 => {
                self.root = root;
                self.unbed();
                let even = b.is_multiple_of(2);
                let davul = if even { at::DAVUL_1 } else { at::DAVUL_2 };
                self.strike(davul, 60, 3.0, 0.24 * d, Touch::at(0.0));
                let frame = if even {
                    at::FRAMEDRUM_1
                } else {
                    at::FRAMEDRUM_2
                };
                self.strike(frame, 60, 2.0, 0.2 * d, Touch::at(-0.1));
                if b >= 8 {
                    self.strike(at::FINGERCYMBAL, 60, 4.0, 0.03 * d, Touch::at(-0.3));
                }
                self.play(bank::PIZZICATO, root, 3.0, 0.16 * l.bass, Touch::at(0.05));
                self.play(
                    bank::CONTRABASS,
                    root,
                    12.2,
                    0.10 * l.bass,
                    Touch::at(0.05).attack(0.2),
                );
                self.play(bank::STRUMSTICK, G3, 10.0, 0.14 * l.drone, Touch::at(0.0));
                if b.is_multiple_of(8) {
                    self.play(bank::CHIMES, G4, 14.0, 0.05, Touch::at(0.2));
                }
            }
            4 | 10 => {
                self.strike(at::FRAMEDRUM_SMALL_MUTED, 60, 1.0, 0.09 * d, Touch::at(0.3));
                if t == 10 {
                    self.strike(at::NAKER_HIGH, 74, 1.0, 0.06 * d, Touch::at(0.3));
                }
            }
            6 => {
                self.strike(at::FRAMEDRUM_SMALL, 60, 1.0, 0.17 * d, Touch::at(0.2));
                let naker = if b.is_multiple_of(2) {
                    at::NAKER_1
                } else {
                    at::NAKER_2
                };
                self.strike(naker, 72, 1.5, 0.10 * d, Touch::at(0.25));
                self.strike(at::ROPESNARE_1, 60, 0.8, 0.07 * d, Touch::at(0.3));
                self.strike(at::TAMBOURINE_HIT, 60, 1.0, 0.045 * d, Touch::at(-0.3));
                self.play(bank::PIZZICATO, D2, 3.0, 0.11 * l.bass, Touch::at(0.05));
            }
            11 => self.strike(at::ROPESNARE_2, 60, 0.8, 0.045 * d, Touch::at(0.3)),
            _ => {}
        }
        if t.is_multiple_of(2) {
            let k = usize::from(t / 2);
            let pitch = [D5, BB4, D5, BB4, C5, BB4][k];
            let gain = if k.is_multiple_of(3) { 0.09 } else { 0.06 };
            self.play(
                bank::PSALTERY,
                pitch,
                2.5,
                gain * l.ostinato,
                Touch::at(0.4),
            );
        }
        if self.singing() && self.horn.is_none() {
            let book = self.theme.book();
            let phrase = if (b / 8).is_multiple_of(2) {
                book.a
            } else {
                book.b
            };
            let pipe = Touch::at(0.05).attack(0.015).release(0.05);
            let v = Voicing::at(72).steps(-3).fold();
            self.sing(phrase, b % 8, bank::CHANTER, 0.25 * l.chanter, pipe, v);
            let v = Voicing::at(67).steps(-3);
            self.sing(
                phrase,
                b % 8,
                bank::VIOLINS,
                0.075 * l.melody,
                Touch::at(-0.3).late(0.03),
                v,
            );
            let heroic = if (b / 16).is_multiple_of(2) {
                0.12
            } else {
                0.17
            };
            self.horns(phrase, b % 8, Voicing::at(57).steps(-3), heroic * l.brass);
        }
    }

    /// The first two bars of an ending keep the last texture's pulse, thinner
    /// each bar: an ending grows out of what was playing.
    pub(super) fn thin_drums(&mut self) {
        if self.here >= 2 || self.tick != 0 {
            return;
        }
        let level = if self.here == 0 { 0.6 } else { 0.3 };
        if matches!(
            self.from,
            Texture::Tension | Texture::Hunt | Texture::Climax
        ) {
            self.strike(at::FRAMEDRUM_1, 60, 2.0, 0.2 * level, Touch::at(-0.1));
        }
    }

    /// Victory in B♭: the pipes' drones re-pitched to B♭ and F, the theme in
    /// B♭ on the chanter, violins and horns, the harp broadening, the davul
    /// and bells; it slows by 4 % a bar and its final rings under the gong.
    #[allow(clippy::too_many_lines)] // one texture's arrangement, read top to bottom
    pub(super) fn victory(&mut self) {
        let t = self.tick;
        let k = self.here;
        self.thin_drums();
        if t == 0 {
            self.root = BB1;
        }
        if k == 0 && t == 0 {
            self.orchestra.release(PIPES);
            self.hold(PIPES, bank::DRONE, BB2, 0.11, Touch::at(0.0).attack(0.3));
            self.hold(PIPES, bank::DRONE, F3, 0.06, Touch::at(0.12).attack(0.3));
            self.pipes_held = None;
        }
        if k >= ENDING_BARS {
            self.after(
                BB,
                &[
                    (bank::ORGAN, BB2, 0.05, 0.0),
                    (bank::VIOLINS, F4, 0.03, 0.3),
                ],
            );
            if k == ENDING_BARS + 1 && t == 0 {
                self.orchestra.release(PIPES);
            }
            return;
        }
        let k = k as usize;
        if t == 0 {
            self.play(
                bank::CONTRABASS,
                BB1,
                12.3,
                0.13,
                Touch::at(0.05).attack(0.2),
            );
            let davul = if k.is_multiple_of(2) {
                at::DAVUL_1
            } else {
                at::DAVUL_2
            };
            self.strike(davul, 60, 3.0, 0.2, Touch::at(0.0));
            self.strike(at::TAIKO_F, 60, 3.0, 0.12, Touch::at(0.0));
            self.strike(at::FRAMEDRUM_1, 60, 2.0, 0.13, Touch::at(-0.1));
            let bell = if k.is_multiple_of(2) {
                at::HANDBELL_2
            } else {
                at::HANDBELL_1
            };
            self.strike(bell, 60, 6.0, 0.045, Touch::at(0.3));
        }
        if t == 6 && k < 6 {
            self.strike(at::FRAMEDRUM_SMALL, 60, 1.0, 0.11, Touch::at(0.2));
            self.strike(at::NAKER_1, 72, 1.5, 0.07, Touch::at(0.25));
        }
        if t.is_multiple_of(2) {
            let degree = [0, 7, 12, 16, 19, 16][usize::from(t / 2)];
            self.play(bank::HARP, BB3 + degree, 6.0, 0.10, Touch::at(-0.35));
        }
        let last = k + 1 == ENDING_BARS as usize;
        let release = if last { 3.0 } else { 0.3 };
        let book = self.theme.book();
        let pipe = Touch::at(0.05)
            .attack(0.015)
            .release(if last { 2.5 } else { 0.05 });
        let v = Voicing::at(72).steps(-1).fold().close(BB);
        self.sing(book.a, k, bank::CHANTER, 0.23, pipe, v);
        let v = Voicing::at(67).steps(-1).close(BB);
        self.sing(
            book.a,
            k,
            bank::VIOLINS,
            0.07,
            Touch::at(-0.3).late(0.03).release(release),
            v,
        );
        self.horns(book.a, k, Voicing::at(57).steps(-1).close(BB), 0.16);
        if t == 0 && k == 3 {
            self.play(bank::CHIMES, BB4, 16.0, 0.06, Touch::at(0.2));
        }
        if t == 0 && last {
            self.play(bank::CHIMES, BB4, 24.0, 0.10, Touch::at(0.2));
            self.strike(at::DAVUL_FORTE, 60, 6.0, 0.24, Touch::at(0.0));
            self.strike(at::GONG, 60, 24.0, 0.10, Touch::at(-0.2));
        }
        if t == 2 && last {
            self.play(bank::CHIMES, D5, 24.0, 0.07, Touch::at(-0.2));
        }
    }

    /// The draw: the pipe falls away, an open fifth F–C in the low strings,
    /// the theme's answer in F on the tenor, ending on C, unresolved; in 3/4,
    /// slowing by 6 % a bar.
    pub(super) fn draw(&mut self) {
        let t = self.tick;
        let k = self.here;
        self.thin_drums();
        if t == 0 {
            self.root = F2;
        }
        if k == 0 && t == 0 {
            self.orchestra.release(PIPES);
            self.pipes_held = None;
        }
        if k >= ENDING_BARS {
            self.after(
                F,
                &[(bank::CELLO, F3, 0.06, -0.2), (bank::VIOLAS, C4, 0.04, 0.2)],
            );
            return;
        }
        let k = k as usize;
        if t == 0 {
            self.play(
                bank::CONTRABASS,
                F2,
                12.3,
                0.11,
                Touch::at(0.05).attack(0.3),
            );
            if k < 4 {
                let gain = 0.08 * (1.0 - k as f32 / 4.0);
                self.strike(at::FRAMEDRUM_MUTED, 60, 2.0, gain, Touch::at(0.1));
            }
        }
        if t.is_multiple_of(4) {
            let degree = [0, 7, 12][usize::from(t / 4)];
            let gain = 0.11 * (1.0 - k as f32 / 10.0) + 0.03;
            self.play(bank::HARP, F3 + degree, 8.0, gain, Touch::at(-0.35));
        }
        let last = k + 1 == ENDING_BARS as usize;
        let touch = Touch::at(0.15)
            .late(0.01)
            .release(if last { 3.0 } else { 0.4 });
        let book = self.theme.book();
        let v = Voicing::at(66).steps(-4).close(C);
        self.sing(book.b, k, bank::TENOR, 0.21, touch, v);
        self.sing(book.b, k, bank::HARP, 0.05, Touch::at(0.4), v);
        if t == 0 && last {
            self.strike(at::HANDBELL_1, 60, 8.0, 0.045, Touch::at(0.3));
        }
    }

    /// The defeat: a slow davul heartbeat that dies away, the low strings on G
    /// and D, the harp in halves, the theme's answer in G Aeolian on the tenor
    /// descending to G with the cellos beneath; one low bell. In 3/4,
    /// slowing by 7 % a bar.
    pub(super) fn defeat(&mut self) {
        let t = self.tick;
        let k = self.here;
        self.thin_drums();
        if t == 0 {
            self.root = G2;
        }
        if k == 0 && t == 0 {
            self.orchestra.release(PIPES);
            self.pipes_held = None;
        }
        if k >= ENDING_BARS {
            self.after(
                G,
                &[
                    (bank::CONTRABASS, G2, 0.07, 0.0),
                    (bank::CELLO, D3, 0.05, -0.2),
                ],
            );
            return;
        }
        let k = k as usize;
        if t == 0 {
            self.play(
                bank::CONTRABASS,
                G2,
                12.6,
                0.11,
                Touch::at(0.05).attack(0.3),
            );
            if k < 6 {
                let fade = 1.0 - k as f32 / 7.0;
                self.strike(at::DAVUL_1, 60, 4.0, 0.13 * fade, Touch::at(0.0));
                self.strike(at::TAIKO_MF, 60, 4.0, 0.06 * fade, Touch::at(0.0));
            }
        }
        if t == 4 && k < 6 {
            let gain = 0.07 * (1.0 - k as f32 / 7.0);
            self.strike(at::DAVUL_2, 60, 4.0, gain, Touch::at(0.0));
        }
        if k.is_multiple_of(2) && (t == 0 || t == 6) {
            let (pitch, gain) = if t == 0 { (G3, 0.10) } else { (D4, 0.07) };
            self.play(bank::HARP, pitch, 10.0, gain, Touch::at(-0.3));
        }
        let last = k + 1 == ENDING_BARS as usize;
        let release = if last { 3.5 } else { 0.4 };
        let book = self.theme.book();
        let v = Voicing::at(64).steps(-3).close(G);
        self.sing(
            book.b,
            k,
            bank::TENOR,
            0.21,
            Touch::at(0.1).late(0.01).release(release),
            v,
        );
        let v = Voicing::at(52).steps(-3).close(G);
        self.sing(
            book.b,
            k,
            bank::CELLO,
            0.06,
            Touch::at(-0.3).late(0.04).release(release),
            v,
        );
        if t == 0 && last {
            self.play(bank::CHIMES, G4, 24.0, 0.06, Touch::at(0.2));
        }
    }

    /// After an ending's cadence: its last chord held, softly, until the
    /// player leaves the result.
    fn after(&mut self, root: u8, voices: &[(Family, u8, f32, f32)]) {
        if self.tick == 0 {
            self.bed(root, voices);
        }
    }
}
