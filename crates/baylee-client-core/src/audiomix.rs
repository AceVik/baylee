//! How loud this device plays: the overall volume, the game's own sounds, and
//! whether it falls silent behind other windows.
//!
//! Kept with the device beside the music's own level (`music::MusicLevel`),
//! for the reason that one gives: it applies before anybody has signed in.
//! The account's `Preferences::sound` (`cue::Loudness`, off, half or full)
//! stays what it is — a coarse switch that travels with the player — and the
//! game-sounds volume here is this machine's fine level under it.
//!
//! Volumes are 0 to 1 on the slider and squared on the way to the speaker,
//! as the music's is: the ear hears amplitude roughly as its logarithm, so a
//! linear slider would do all its work in its first quarter.

use serde::{Deserialize, Serialize};

/// The device's volumes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioMix {
    master: f32,
    effects: f32,
    /// Silence while another window has the focus.
    pub mute_in_background: bool,
    /// Whether the priority cue (`cue::Cue::YourMove`) is played on this
    /// device (DESIGN-v7 §4.5, D22). Off, the cue is still decided and
    /// reported (`/state.last_cue`), and nothing is played: the
    /// `Loudness::Off` discipline, one cue wide. Never gated by the music,
    /// which is a different knob. On by default, and a file from before it
    /// reads on.
    pub priority_cue: bool,
}

impl Default for AudioMix {
    /// Everything at full and audible in the background: what this client did
    /// before the knobs existed.
    fn default() -> Self {
        Self {
            master: 1.0,
            effects: 1.0,
            mute_in_background: false,
            priority_cue: true,
        }
    }
}

/// A slider value kept from 0 to 1; anything that is not a number reads as
/// `fallback`.
fn level(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

impl AudioMix {
    /// The overall volume the slider shows, 0 to 1.
    #[must_use]
    pub fn master(self) -> f32 {
        level(self.master, 1.0)
    }

    /// The game sounds' volume the slider shows, 0 to 1.
    #[must_use]
    pub fn effects(self) -> f32 {
        level(self.effects, 1.0)
    }

    /// Sets the overall volume; a value that is no number is ignored.
    pub fn set_master(&mut self, volume: f32) {
        if volume.is_finite() {
            self.master = volume.clamp(0.0, 1.0);
        }
    }

    /// Sets the game sounds' volume; a value that is no number is ignored.
    pub fn set_effects(&mut self, volume: f32) {
        if volume.is_finite() {
            self.effects = volume.clamp(0.0, 1.0);
        }
    }

    /// What every sound's amplitude is multiplied by, the music's included.
    #[must_use]
    pub fn master_gain(self, focused: bool) -> f32 {
        if self.mute_in_background && !focused {
            0.0
        } else {
            self.master() * self.master()
        }
    }

    /// What a game sound's amplitude is multiplied by, the overall volume
    /// included.
    #[must_use]
    pub fn effects_gain(self, focused: bool) -> f32 {
        self.master_gain(focused) * self.effects() * self.effects()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The defaults change nothing: what played before the knobs plays now.
    #[test]
    fn the_defaults_play_as_before() {
        let mix = AudioMix::default();
        assert!((mix.master_gain(true) - 1.0).abs() < f32::EPSILON);
        assert!((mix.effects_gain(false) - 1.0).abs() < f32::EPSILON);
        let read: AudioMix = serde_json::from_str("{}").expect("reads");
        assert_eq!(read, mix);
    }

    /// Squared on the way out, both multiply, and the background mute
    /// silences only an unfocused window.
    #[test]
    fn the_volumes_square_multiply_and_mute_in_the_background() {
        let mut mix = AudioMix::default();
        mix.set_master(0.5);
        mix.set_effects(0.5);
        assert!((mix.master_gain(true) - 0.25).abs() < 1e-6);
        assert!((mix.effects_gain(true) - 0.0625).abs() < 1e-6);
        mix.mute_in_background = true;
        assert!(mix.effects_gain(false).abs() < f32::EPSILON);
        assert!(mix.master_gain(true) > 0.0);
        mix.set_master(f32::NAN);
        assert!((mix.master() - 0.5).abs() < f32::EPSILON);
        mix.set_effects(7.0);
        assert!((mix.effects() - 1.0).abs() < f32::EPSILON);
        let kept: AudioMix =
            serde_json::from_str(&serde_json::to_string(&mix).expect("writes")).expect("reads");
        assert_eq!(kept, mix);
    }
}
