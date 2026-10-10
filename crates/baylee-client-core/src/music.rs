//! Baylee's continuous adaptive score: an original modal score around B♭,
//! performed from CC0 recordings of old instruments (recorders, bowed and
//! plucked psaltery, folk harp, bagpipe, frame drums, davul, bells; VCSL,
//! VSCO 2 CE, `FreePats`) and one synthesised lute, by one sampler and one
//! musical clock, from the front door to the table and its ending. Scene
//! changes alter future notes at bar lines; held notes and the room continue.
//! No track is stopped or crossfaded. Everything plays offline: the bank is
//! in the binary, and the drivers read only the [`PlayerView`].
//!
//! [`PlayerView`]: baylee_view::PlayerView

mod bank;
mod direct;
mod orchestra;
mod score;
pub use direct::{Ending, Memory, Place, Scene, ScoreRequest, direct};
pub use score::{Movement, Position, ScoreControl, Theme, Tune};

/// The complete suite chosen in Settings → Audio, remembered per device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicTheme {
    /// Glutpfad, in four.
    #[serde(alias = "ballad")]
    Ember,
    /// Mondglas, in three.
    #[serde(alias = "dance")]
    Glass,
    /// Dornenkrone, in four.
    #[serde(alias = "epic")]
    Thorn,
    /// Nebelhafen, in compound duple metre.
    #[serde(alias = "jig")]
    Tide,
    /// Sternfall, in five.
    Star,
    /// A different suite each game.
    #[default]
    Rotating,
}
impl MusicTheme {
    /// Every choice, in settings order.
    pub const ALL: [Self; 6] = [
        Self::Ember,
        Self::Glass,
        Self::Thorn,
        Self::Tide,
        Self::Star,
        Self::Rotating,
    ];
    /// The suite at this rotation.
    #[must_use]
    pub const fn pick(self, turn: u8) -> Theme {
        match self {
            Self::Ember => Theme::Ember,
            Self::Glass => Theme::Glass,
            Self::Thorn => Theme::Thorn,
            Self::Tide => Theme::Tide,
            Self::Star => Theme::Star,
            Self::Rotating => Theme::ALL[(turn % 5) as usize],
        }
    }
}

/// Stereo output sample rate, and the bank's.
pub const RATE: u32 = 44_100;
/// Interleaved left and right channels.
pub const CHANNELS: u16 = 2;

/// Read the embedded sample bank's table once before starting the audio
/// device.
pub fn prepare() {
    orchestra::instruments();
}

/// How loud the front door's music is, as this device remembers it.
///
/// Kept with the device's own settings and not the account's, because it
/// plays before anybody has signed in. Its fields are private so the volume
/// is always a number from 0 to 1: a NaN would be written to the settings
/// file as `null`, and a `null` there refuses the whole file, gateways and
/// guest sessions with it.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MusicLevel {
    volume: f32,
    muted: bool,
    /// The theme (absent from a file older than the themes: the default).
    theme: MusicTheme,
}

impl Default for MusicLevel {
    /// Half way, and playing: a player who never opens the settings hears it,
    /// under the room rather than over it.
    fn default() -> Self {
        Self {
            volume: 0.5,
            muted: false,
            theme: MusicTheme::default(),
        }
    }
}

impl MusicLevel {
    /// The volume the slider shows, 0 to 1.
    #[must_use]
    pub fn volume(self) -> f32 {
        if self.volume.is_finite() {
            self.volume.clamp(0.0, 1.0)
        } else {
            Self::default().volume
        }
    }

    /// Sets the volume, kept from 0 to 1; anything that is not a number is
    /// ignored.
    pub fn set_volume(&mut self, volume: f32) {
        if volume.is_finite() {
            self.volume = volume.clamp(0.0, 1.0);
        }
    }

    /// The theme the player chose.
    #[must_use]
    pub const fn theme(self) -> MusicTheme {
        self.theme
    }

    /// Chooses a theme: heard from the next bar line, no restart.
    pub const fn set_theme(&mut self, theme: MusicTheme) {
        self.theme = theme;
    }

    /// Whether the player silenced it.
    #[must_use]
    pub const fn muted(self) -> bool {
        self.muted
    }

    /// Silences it, or lets it play at the volume it had.
    pub const fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// What the music's amplitude is multiplied by: nothing when muted, and
    /// [`Self::loudness`] otherwise.
    #[must_use]
    pub fn gain(self) -> f32 {
        if self.muted { 0.0 } else { self.loudness() }
    }

    /// Silences the music if it can be heard, and lets it be heard if not:
    /// what its switch does (#296). A switch pressed to play music whose
    /// volume is 0 would do nothing a player could hear, so that also brings
    /// the volume back to where it starts.
    pub fn toggle(&mut self) {
        if self.gain() > 0.0 {
            self.muted = true;
        } else {
            self.muted = false;
            if self.volume() <= 0.0 {
                self.set_volume(Self::default().volume());
            }
        }
    }

    /// The amplitude the volume stands for, muted or not: its square,
    /// because the ear hears amplitude roughly as its logarithm and a linear
    /// slider would do all its work in its first quarter. A player fades
    /// towards silence by this and does not jump there.
    #[must_use]
    pub fn loudness(self) -> f32 {
        self.volume() * self.volume()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The level: half way and playing by default, silent when muted
    /// whatever the volume, and a volume that is no number never taken.
    #[test]
    fn the_music_level_is_a_volume_and_a_mute() {
        let mut level = MusicLevel::default();
        assert!(level.gain() > 0.0);
        level.set_muted(true);
        assert!(level.gain().abs() < f32::EPSILON);
        level.set_muted(false);
        level.set_volume(f32::NAN);
        assert!((level.volume() - 0.5).abs() < f32::EPSILON);
        level.set_volume(3.0);
        assert!((level.gain() - 1.0).abs() < f32::EPSILON);
        level.set_volume(0.0);
        assert!(level.gain().abs() < f32::EPSILON);
        // A settings file from before the music opens with it playing.
        let old: MusicLevel = serde_json::from_str("{}").expect("reads");
        assert_eq!(old, MusicLevel::default());
        level.set_theme(MusicTheme::Rotating);
        let kept: MusicLevel =
            serde_json::from_str(&serde_json::to_string(&level).expect("writes")).expect("reads");
        assert_eq!(kept, level, "the theme is kept with the level");
        let older: MusicLevel =
            serde_json::from_str(r#"{"volume":0.3,"muted":false}"#).expect("reads");
        assert_eq!(
            older.theme(),
            MusicTheme::Rotating,
            "a file from before the themes rotates through all four"
        );
    }

    /// The switch flips what is heard: a playing tune goes silent and keeps
    /// its volume, a silent one plays again, and one silenced by a volume of
    /// 0 comes back at the starting volume rather than staying silent.
    #[test]
    fn the_switch_flips_what_is_heard() {
        let mut level = MusicLevel::default();
        level.set_volume(0.8);
        level.toggle();
        assert!(level.muted() && level.gain().abs() < f32::EPSILON);
        level.toggle();
        assert!(!level.muted() && (level.volume() - 0.8).abs() < f32::EPSILON);
        level.set_volume(0.0);
        level.toggle();
        assert!(level.gain() > 0.0);
        assert!((level.volume() - MusicLevel::default().volume()).abs() < f32::EPSILON);
    }
}
