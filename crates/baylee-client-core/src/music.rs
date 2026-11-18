//! Unified D-minor / D-Dorian / F-major / D-major public-domain-oriented score:
//! - Login (`Place::FrontDoor`): noble Title suite (Handel *Sarabande HWV 437*, Dvořák *Largo*, Mussorgsky *Promenade*).
//! - Lobby & Deckbuilder (`Place::Lobby | Place::Build`): cozy Acoustic-Tavern-Guitar suite (Sanz, Tárrega, Sor, Bach).
//! - Arrival / Loading (`Place::Opening`): warm harmonic hush; Table music begins only after the loading cover clears.
//! - Table (`Place::Table`) & Endings: fifteen suites scaling continuously from Calm to Epic with live stack depth,
//!   resolved by D-major Victory, D-minor Defeat, and D-Dorian Draw cues.
//!
//! Native 48-kHz physical-modelled Acoustic Tavern Guitar, Renaissance lyre, and hybrid orchestral synthesizer layers
//! share one stereo room with CC0 acoustic recordings (`Studio48` or `Original441`), gliding seamlessly across beats.
//!
//! The default theme, [`MusicTheme::Saga`], plays the recorded soundtrack instead (`recorded`, native builds):
//! nine embedded Ogg-Opus files mixed into the same [`Tune`], with leitmotif stingers chosen by [`direct`].

#[allow(dead_code)] // Historical generated bank retains unused documented families.
mod bank;
mod direct;
mod orchestra;
#[cfg(not(target_arch = "wasm32"))]
mod recorded;
mod score;
pub use direct::{
    Ending, Memory, Place, STINGER_COOLDOWN, STINGER_GRACE, Scene, ScoreRequest, Stinger, direct,
};
pub use score::{Movement, Position, ScoreControl, Theme, Tune};

/// The complete suite chosen in Settings → Audio, remembered per device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicTheme {
    /// The recorded Baylee soundtrack: a title theme that returns as
    /// stingers at a game's epic moments (`music::recorded`). Where it cannot
    /// play (the browser build), it falls back to the rotating suites.
    #[default]
    Saga,
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
    /// Velvet Night · Piano.
    Velvet,
    /// Copperwork · Baroque.
    Copper,
    /// Juniper · Folk.
    Juniper,
    /// Lagoon Light · Bossa.
    Lagoon,
    /// Lanterns · Jazz waltz.
    Lantern,
    /// Neon Path · Synthwave.
    Neon,
    /// Pixelstorm · Chiptune.
    Circuit,
    /// Mosaic · Marimba.
    Mosaic,
    /// Orbit · Ambient.
    Orbit,
    /// Iron Pulse · Breakbeat.
    Iron,
    /// A different suite each game.
    Rotating,
}
impl MusicTheme {
    /// Every choice, in settings order.
    pub const ALL: [Self; 17] = [
        Self::Saga,
        Self::Ember,
        Self::Glass,
        Self::Thorn,
        Self::Tide,
        Self::Star,
        Self::Velvet,
        Self::Copper,
        Self::Juniper,
        Self::Lagoon,
        Self::Lantern,
        Self::Neon,
        Self::Circuit,
        Self::Mosaic,
        Self::Orbit,
        Self::Iron,
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
            Self::Velvet => Theme::Velvet,
            Self::Copper => Theme::Copper,
            Self::Juniper => Theme::Juniper,
            Self::Lagoon => Theme::Lagoon,
            Self::Lantern => Theme::Lantern,
            Self::Neon => Theme::Neon,
            Self::Circuit => Theme::Circuit,
            Self::Mosaic => Theme::Mosaic,
            Self::Orbit => Theme::Orbit,
            Self::Iron => Theme::Iron,
            // The recorded theme's synthesised fallback rotates as
            // "Rotating" does; rotating never reaches the recording.
            Self::Saga | Self::Rotating => Theme::ALL[(turn % 15) as usize],
        }
    }

    /// Whether this theme plays the recordings instead of the synthesised
    /// score: the recorded theme, on a build that can decode them.
    #[must_use]
    pub const fn recorded(self) -> bool {
        matches!(self, Self::Saga) && RECORDED
    }
}

/// Recorded sound bank, independent of the musical suite. Output remains 48 kHz.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum SampleSet {
    /// Windowed-sinc preparation at every used pitch, cached at 48 kHz.
    #[default]
    Studio48 = 0,
    /// Original 44.1-kHz PCM, cubic playback at the continuous output rate.
    Original441 = 1,
}
impl SampleSet {
    /// Settings order.
    pub const ALL: [Self; 2] = [Self::Studio48, Self::Original441];
    /// Stable development-control and file name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Studio48 => "studio48",
            Self::Original441 => "original441",
        }
    }
}

/// Whether this build carries the recorded soundtrack: native builds. The
/// browser build has no thread to decode it on and plays the synthesised
/// score instead.
pub const RECORDED: bool = cfg!(not(target_arch = "wasm32"));

/// Continuous stereo output rate, independent of the selected sample bank.
pub const RATE: u32 = 48_000;
/// Interleaved left and right channels.
pub const CHANNELS: u16 = 2;

/// Prepare band-limited instrument tables before starting the audio device.
pub fn prepare() {
    orchestra::prepare();
}

/// How loud the front door's music is, as this device remembers it.
///
/// Kept with the device's own settings and not the account's, because it
/// plays before anybody has signed in. Its fields are private so the volume
/// is always a number from 0 to 1: a NaN would be written to the settings
/// file as `null`, and a `null` there refuses the whole file, gateways and
/// guest sessions with it.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(from = "Stored", into = "Stored")]
pub struct MusicLevel {
    volume: f32,
    muted: bool,
    /// The theme (absent from a file older than the themes: the default).
    theme: MusicTheme,
    samples: SampleSet,
}

/// A [`MusicLevel`] as the settings file holds it.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Stored {
    volume: f32,
    muted: bool,
    theme: MusicTheme,
    samples: SampleSet,
    /// Written as `true` by every client that has the recorded soundtrack.
    /// A file from before it stored the old default, `rotating`, whether
    /// anybody chose it or not, and a device that kept it would never hear
    /// the soundtrack: without this mark `rotating` reads as today's
    /// default. Any other stored theme was a choice and is kept.
    #[serde(default)]
    soundtrack: bool,
}

impl Default for Stored {
    fn default() -> Self {
        MusicLevel::default().into()
    }
}

impl From<MusicLevel> for Stored {
    fn from(level: MusicLevel) -> Self {
        Self {
            volume: level.volume,
            muted: level.muted,
            theme: level.theme,
            samples: level.samples,
            soundtrack: true,
        }
    }
}

impl From<Stored> for MusicLevel {
    fn from(stored: Stored) -> Self {
        let theme = if !stored.soundtrack && stored.theme == MusicTheme::Rotating {
            MusicTheme::default()
        } else {
            stored.theme
        };
        Self {
            volume: stored.volume,
            muted: stored.muted,
            theme,
            samples: stored.samples,
        }
    }
}

impl Default for MusicLevel {
    /// Half way, and playing: a player who never opens the settings hears it,
    /// under the room rather than over it.
    fn default() -> Self {
        Self {
            volume: 0.5,
            muted: false,
            theme: MusicTheme::default(),
            samples: SampleSet::default(),
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

    /// Chooses a theme: heard from the next musical pulse, no restart.
    pub const fn set_theme(&mut self, theme: MusicTheme) {
        self.theme = theme;
    }

    /// The sample bank, available for every suite.
    #[must_use]
    pub const fn samples(self) -> SampleSet {
        self.samples
    }
    /// Change timbre at the next pulse without restarting the score.
    pub const fn set_samples(&mut self, samples: SampleSet) {
        self.samples = samples;
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
            MusicTheme::Saga,
            "a file from before the themes plays the recorded soundtrack"
        );
        let rotating: MusicLevel =
            serde_json::from_str(r#"{"volume":0.3,"theme":"rotating"}"#).expect("reads");
        assert_eq!(
            rotating.theme(),
            MusicTheme::Saga,
            "the old default was stored unasked: such a device gets the recording"
        );
        let mut chosen = rotating;
        chosen.set_theme(MusicTheme::Rotating);
        let saved = serde_json::to_string(&chosen).expect("writes");
        let read: MusicLevel = serde_json::from_str(&saved).expect("reads");
        assert_eq!(
            read.theme(),
            MusicTheme::Rotating,
            "rotating chosen in a client that has the recording is kept"
        );
        let mut louder = rotating;
        louder.set_volume(0.8);
        let saved = serde_json::to_string(&louder).expect("writes");
        let read: MusicLevel = serde_json::from_str(&saved).expect("reads");
        assert_eq!(
            read.theme(),
            MusicTheme::Saga,
            "saving another setting does not turn the old default into a choice"
        );
        assert!(
            saved.contains(r#""soundtrack":true"#),
            "what this client writes is marked as its own"
        );
        let suite: MusicLevel =
            serde_json::from_str(r#"{"volume":0.3,"theme":"thorn"}"#).expect("reads");
        assert_eq!(
            suite.theme(),
            MusicTheme::Thorn,
            "a suite stored by an older client was chosen and is kept"
        );
        let fresh = serde_json::to_string(&MusicLevel::default()).expect("writes");
        let read: MusicLevel = serde_json::from_str(&fresh).expect("reads");
        assert_eq!(
            read,
            MusicLevel::default(),
            "a fresh level reads back as it was"
        );
        assert_eq!(MusicTheme::ALL[0], MusicTheme::Saga, "first in the list");
        assert!(
            (0..60).all(|turn| Theme::ALL.contains(&MusicTheme::Rotating.pick(turn))),
            "rotating walks the fifteen suites only"
        );
        assert_eq!(
            MusicTheme::ALL
                .iter()
                .filter(|theme| theme.recorded())
                .count(),
            usize::from(RECORDED),
            "only the recorded theme plays the recordings"
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
