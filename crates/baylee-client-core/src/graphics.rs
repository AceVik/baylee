//! The device's graphics settings: what each knob is, what each preset sets,
//! and how often a frame is drawn.
//!
//! Kept with the device (`ClientSettings` in the client) and not with the
//! account, because what a machine can draw quietly is a fact about the
//! machine: an M1 on battery and a desktop with a discrete card want different
//! answers from one player. The renderer applies it (`baylee-client`'s
//! `quality.rs`); everything that is a *decision* is here, where it is tested
//! without a GPU.
//!
//! Only knobs that change something are offered, and each one's doc says what
//! it costs, measured on an M1 Max at 1920×1080 logical (3840×2160 physical);
//! `docs/perf-client.md` has the numbers and how they were taken. There is
//! deliberately no shadow or lighting knob — the table has no lights at all
//! (`docs/client.md`), so a knob for them would move nothing — and no render
//! scale yet: the table would have to be drawn into a texture and the pointer
//! mapped back into it for picking, and a scale that only blurred the
//! interface would be the wrong half of the picture.
//!
//! Reduced motion is the account's (`Preferences::reduce_motion`), because it
//! is about the player and not about the machine; the graphics screen shows
//! it beside these and no preset ever changes it.
//!
//! Every field reads leniently: a value this build does not know (a newer
//! build's preset, a hand-edited file) falls back to that field's default
//! instead of refusing the whole settings file, which also holds the player's
//! gateways and guest sessions.

use serde::{Deserialize, Deserializer, Serialize};

/// Reads a field, falling back to its default for anything that does not
/// parse — never refusing the file around it. For `#[serde(deserialize_with)]`.
///
/// # Errors
/// Only when the input is not JSON at all; a value of the wrong shape is
/// the default instead.
pub fn lenient<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).unwrap_or_default())
}

/// A named set of every knob below, or `Custom` once one was moved by hand.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    /// For a machine that must stay cool: half the frames, still ambience.
    Low,
    /// The default on an M1 and on phones: sixty frames, the lighter
    /// ambience, quiet fans.
    #[default]
    Medium,
    /// Every effect at full detail, up to 120 frames.
    High,
    /// As many frames as the display shows, and the window keeps drawing
    /// smoothly behind other windows.
    Ultra,
    /// Some knob differs from every preset.
    Custom,
}

impl Preset {
    /// The presets a picker offers, in order; `Custom` is what a picker
    /// shows, never something it sets.
    pub const NAMED: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];

    /// The stable storage spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
            Self::Custom => "custom",
        }
    }
}

/// How the table's edges are smoothed. The front door and the lobby are
/// flat interface and draw no edge that needs it.
///
/// Measured cost on the M1 Max at a duel: MSAA 4× against none, 0.2 ms of GPU
/// per frame (Apple's tile memory resolves samples nearly free), FXAA a full
/// screen pass of about the same. Without either, the card and slab edges
/// stair-step visibly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiAliasing {
    /// Hard edges.
    Off,
    /// A post-process pass: cheap anywhere, a little soft on card text.
    Fxaa,
    /// Two samples per pixel.
    Msaa2,
    /// Four samples per pixel: sharp text, clean edges.
    #[default]
    Msaa4,
}

impl AntiAliasing {
    /// Every choice, in the order a picker offers them.
    pub const ALL: [Self; 4] = [Self::Off, Self::Fxaa, Self::Msaa2, Self::Msaa4];

    /// Samples per pixel the main pass renders with.
    #[must_use]
    pub const fn samples(self) -> u32 {
        match self {
            Self::Off | Self::Fxaa => 1,
            Self::Msaa2 => 2,
            Self::Msaa4 => 4,
        }
    }
}

/// Whether a frame waits for the display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VSync {
    /// Every frame waits for the display: no tearing.
    #[default]
    On,
    /// Waits, unless a frame is late, which then shows at once (tears
    /// rather than stutters). Where the platform has no such mode it is the
    /// same as `On`.
    Adaptive,
    /// Frames are shown as soon as they are drawn: lowest latency, tears.
    Off,
}

impl VSync {
    /// Every choice, in the order a picker offers them.
    pub const ALL: [Self; 3] = [Self::On, Self::Adaptive, Self::Off];
}

/// The most frames drawn per second while the window has the focus.
///
/// The biggest single knob there is: every frame pays the whole screen's
/// shaders again. Measured at the front door, 120 frames cost 2.0× the CPU and
/// energy of 60 (`docs/perf-client.md`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameLimit {
    /// Thirty.
    Fps30,
    /// Sixty: smooth, and half the work of a 120 Hz display.
    #[default]
    Fps60,
    /// A hundred and twenty.
    Fps120,
    /// Whatever the display (or, with vsync off, the machine) manages.
    Unlimited,
}

impl FrameLimit {
    /// Every choice, in the order a picker offers them.
    pub const ALL: [Self; 4] = [Self::Fps30, Self::Fps60, Self::Fps120, Self::Unlimited];

    /// Frames per second, or `None` for no limit.
    #[must_use]
    pub const fn fps(self) -> Option<u32> {
        match self {
            Self::Fps30 => Some(30),
            Self::Fps60 => Some(60),
            Self::Fps120 => Some(120),
            Self::Unlimited => None,
        }
    }
}

/// The most frames drawn per second while the window is in the background
/// (not focused) or the front door and lobby have stood untouched for
/// [`IDLE_AFTER_SECS`]. A hidden or minimised window draws one frame a
/// second whatever this says: nobody can see it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundLimit {
    /// Five: the picture still follows the game, just in steps.
    Fps5,
    /// Fifteen.
    #[default]
    Fps15,
    /// Thirty.
    Fps30,
    /// Sixty: the background is drawn like the foreground.
    Fps60,
}

impl BackgroundLimit {
    /// Every choice, in the order a picker offers them.
    pub const ALL: [Self; 4] = [Self::Fps5, Self::Fps15, Self::Fps30, Self::Fps60];

    /// Frames per second.
    #[must_use]
    pub const fn fps(self) -> u32 {
        match self {
            Self::Fps5 => 5,
            Self::Fps15 => 15,
            Self::Fps30 => 30,
            Self::Fps60 => 60,
        }
    }
}

/// How much the ambient surfaces do: the front door's painted world, the
/// table's mineral cloth, the sky over it.
///
/// Gameplay light never depends on it — offers, the turn rim, the phase
/// lamp, shells and waves are what a player reads and stay at every level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effects {
    /// Ambience stands still: the front door is a painting with its water
    /// and fireflies frozen, the cloth's rivers stop flowing. With nothing
    /// ambient moving, an untouched screen can drop to the background rate.
    Low,
    /// Moving, at the lighter detail phones always had: three cloud layers
    /// instead of six, one plane of fireflies instead of two.
    #[default]
    Medium,
    /// Everything, at full detail.
    High,
}

impl Effects {
    /// Every choice, in the order a picker offers them.
    pub const ALL: [Self; 3] = [Self::Low, Self::Medium, Self::High];

    /// Whether ambient surfaces animate at all.
    #[must_use]
    pub const fn ambient_motion(self) -> bool {
        !matches!(self, Self::Low)
    }

    /// The detail flag the ambient shaders branch on: 1 for the full,
    /// 0 for the lighter versions.
    #[must_use]
    pub const fn detail(self) -> f32 {
        match self {
            Self::High => 1.0,
            Self::Low | Self::Medium => 0.0,
        }
    }
}

/// Seconds without any input after which the front door and the lobby draw
/// at the background rate. Never at the table, where an opponent's move is
/// worth watching at full rate whether or not the mouse moves.
pub const IDLE_AFTER_SECS: f32 = 30.0;

/// Frames per second a hidden or minimised window still draws.
pub const HIDDEN_FPS: u32 = 1;

/// The device's graphics settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Graphics {
    /// The preset last picked, `Custom` once a knob was moved by hand.
    #[serde(deserialize_with = "lenient")]
    pub preset: Preset,
    /// Edge smoothing on the table.
    #[serde(deserialize_with = "lenient")]
    pub anti_aliasing: AntiAliasing,
    /// Whether frames wait for the display.
    #[serde(deserialize_with = "lenient")]
    pub vsync: VSync,
    /// Frames per second with the focus.
    #[serde(deserialize_with = "lenient")]
    pub frame_limit: FrameLimit,
    /// Frames per second in the background and on an untouched front door.
    #[serde(deserialize_with = "lenient")]
    pub background_limit: BackgroundLimit,
    /// How much the ambient surfaces do.
    #[serde(deserialize_with = "lenient")]
    pub effects: Effects,
}

impl Default for Graphics {
    fn default() -> Self {
        Self::of(Preset::Medium)
    }
}

impl Graphics {
    /// Every knob as `preset` sets it. `Custom` is `Medium`'s knobs.
    #[must_use]
    pub const fn of(preset: Preset) -> Self {
        let (anti_aliasing, frame_limit, background_limit, effects) = match preset {
            Preset::Low => (
                AntiAliasing::Off,
                FrameLimit::Fps30,
                BackgroundLimit::Fps5,
                Effects::Low,
            ),
            Preset::Medium | Preset::Custom => (
                AntiAliasing::Msaa4,
                FrameLimit::Fps60,
                BackgroundLimit::Fps15,
                Effects::Medium,
            ),
            Preset::High => (
                AntiAliasing::Msaa4,
                FrameLimit::Fps120,
                BackgroundLimit::Fps30,
                Effects::High,
            ),
            Preset::Ultra => (
                AntiAliasing::Msaa4,
                FrameLimit::Unlimited,
                BackgroundLimit::Fps60,
                Effects::High,
            ),
        };
        Self {
            preset: if matches!(preset, Preset::Custom) {
                Preset::Medium
            } else {
                preset
            },
            anti_aliasing,
            vsync: VSync::On,
            frame_limit,
            background_limit,
            effects,
        }
    }

    /// The named preset whose knobs these are, or `Custom`.
    #[must_use]
    pub fn matching_preset(&self) -> Preset {
        Preset::NAMED
            .into_iter()
            .find(|preset| Self::of(*preset).same_knobs(*self))
            .unwrap_or(Preset::Custom)
    }

    fn same_knobs(self, other: Self) -> bool {
        self.anti_aliasing == other.anti_aliasing
            && self.vsync == other.vsync
            && self.frame_limit == other.frame_limit
            && self.background_limit == other.background_limit
            && self.effects == other.effects
    }

    /// Changes one knob through `change`, and names the preset that now
    /// matches — `Custom` unless the knobs happen to be a preset's.
    pub fn adjust(&mut self, change: impl FnOnce(&mut Self)) {
        change(self);
        self.preset = self.matching_preset();
    }

    /// The preset a device starts on before its player ever chose one,
    /// from the name the GPU driver reports.
    ///
    /// Quiet first: a laptop's integrated GPU (an M1, a phone, Intel or AMD
    /// integrated graphics) starts at `Medium`, a software rasteriser at
    /// `Low`, and a discrete card or a later Apple Pro/Max/Ultra chip at
    /// `High`. An unknown name is `Medium`: wrong on the quiet side.
    #[must_use]
    pub fn auto_preset(adapter: &str, mobile: bool) -> Preset {
        let name = adapter.to_ascii_lowercase();
        if [
            "llvmpipe",
            "swiftshader",
            "software",
            "lavapipe",
            "microsoft basic",
        ]
        .iter()
        .any(|soft| name.contains(soft))
        {
            return Preset::Low;
        }
        if mobile {
            return Preset::Medium;
        }
        if name.contains("apple") {
            let plain_m1 = name.contains("m1")
                && !(name.contains("pro") || name.contains("max") || name.contains("ultra"));
            let later = ["m2", "m3", "m4", "m5", "m6"]
                .iter()
                .any(|m| name.contains(m));
            let big = name.contains("max") || name.contains("ultra");
            // A Max or Ultra of the second generation on has fans that
            // hardly turn at High; the first generation, and every plain
            // chip, stays at Medium.
            return if later && big && !plain_m1 {
                Preset::High
            } else {
                Preset::Medium
            };
        }
        if [
            "nvidia",
            "geforce",
            "rtx",
            "radeon rx",
            "radeon pro",
            "arc a",
        ]
        .iter()
        .any(|discrete| name.contains(discrete))
        {
            return Preset::High;
        }
        Preset::Medium
    }
}

/// What the window is doing, as far as pacing cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Showing {
    /// The window has the keyboard focus.
    pub focused: bool,
    /// The window is hidden, minimised or entirely covered.
    pub hidden: bool,
    /// The front door or the lobby, untouched for [`IDLE_AFTER_SECS`].
    pub idle_screen: bool,
}

/// How often frames are drawn: as fast as allowed, or one every so often.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pace {
    /// No limit beyond the display (or vsync off, none at all).
    Unlimited,
    /// At most this many frames a second.
    Fps(u32),
}

impl Pace {
    /// Seconds between frames, or `None` for no limit.
    #[must_use]
    pub fn interval_secs(self) -> Option<f32> {
        match self {
            Self::Unlimited => None,
            #[allow(clippy::cast_precision_loss)]
            Self::Fps(fps) => Some(1.0 / fps.max(1) as f32),
        }
    }
}

impl Graphics {
    /// The pace for a window in this state.
    ///
    /// Hidden beats everything (nobody sees it); then the background or an
    /// idle front door takes the background limit, never faster than the
    /// focused limit; then the focused limit.
    #[must_use]
    pub fn pace(&self, showing: Showing) -> Pace {
        if showing.hidden {
            return Pace::Fps(HIDDEN_FPS);
        }
        let focused = self.frame_limit.fps();
        if !showing.focused || showing.idle_screen {
            let background = self.background_limit.fps();
            return Pace::Fps(focused.map_or(background, |f| background.min(f)));
        }
        focused.map_or(Pace::Unlimited, Pace::Fps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT_DESK: Showing = Showing {
        focused: true,
        hidden: false,
        idle_screen: false,
    };

    /// Each named preset is its own: setting one and asking which preset
    /// matches names it back, and no two presets share their knobs.
    #[test]
    fn every_preset_names_itself_back_and_none_shares_its_knobs() {
        for preset in Preset::NAMED {
            let graphics = Graphics::of(preset);
            assert_eq!(graphics.preset, preset);
            assert_eq!(graphics.matching_preset(), preset);
        }
        for (i, a) in Preset::NAMED.iter().enumerate() {
            for b in &Preset::NAMED[i + 1..] {
                assert!(
                    !Graphics::of(*a).same_knobs(Graphics::of(*b)),
                    "{a:?} and {b:?} set the same knobs"
                );
            }
        }
    }

    /// The presets climb: each one draws at least as many frames and at
    /// least as much ambience as the one below it.
    #[test]
    fn the_presets_climb() {
        let rate = |g: Graphics| g.frame_limit.fps().unwrap_or(u32::MAX);
        for pair in Preset::NAMED.windows(2) {
            let (low, high) = (Graphics::of(pair[0]), Graphics::of(pair[1]));
            assert!(rate(low) <= rate(high), "{pair:?}");
            assert!(low.background_limit.fps() <= high.background_limit.fps());
            assert!(low.effects.detail() <= high.effects.detail());
        }
        assert_eq!(Graphics::default(), Graphics::of(Preset::Medium));
    }

    /// Moving a knob makes the preset `Custom`, and moving it back names the
    /// preset again.
    #[test]
    fn a_moved_knob_is_custom_until_it_matches_again() {
        let mut graphics = Graphics::of(Preset::Medium);
        graphics.adjust(|g| g.effects = Effects::High);
        assert_eq!(graphics.preset, Preset::Custom);
        graphics.adjust(|g| g.effects = Effects::Medium);
        assert_eq!(graphics.preset, Preset::Medium);
        graphics.adjust(|g| *g = Graphics::of(Preset::High));
        assert_eq!(graphics.preset, Preset::High);
    }

    /// The pace: the focused limit at the desk, the background limit
    /// behind other windows and on an idle front door — never faster than
    /// the focused one — and one frame a second when nobody can see it.
    #[test]
    fn the_pace_follows_the_window() {
        let medium = Graphics::of(Preset::Medium);
        assert_eq!(medium.pace(AT_DESK), Pace::Fps(60));
        let away = Showing {
            focused: false,
            ..AT_DESK
        };
        assert_eq!(medium.pace(away), Pace::Fps(15));
        let idle = Showing {
            idle_screen: true,
            ..AT_DESK
        };
        assert_eq!(medium.pace(idle), Pace::Fps(15));
        let hidden = Showing {
            hidden: true,
            ..AT_DESK
        };
        assert_eq!(medium.pace(hidden), Pace::Fps(HIDDEN_FPS));
        assert_eq!(Graphics::of(Preset::Ultra).pace(AT_DESK), Pace::Unlimited);
        assert_eq!(Graphics::of(Preset::Ultra).pace(away), Pace::Fps(60));
        // A background limit above the focused one is held to it.
        let mut odd = Graphics::of(Preset::Low);
        odd.background_limit = BackgroundLimit::Fps60;
        assert_eq!(odd.pace(away), Pace::Fps(30));
        let interval = Pace::Fps(60).interval_secs().expect("a limit");
        assert!((interval - 1.0 / 60.0).abs() < 1e-6);
        assert_eq!(Pace::Unlimited.interval_secs(), None);
    }

    /// The device's GPU picks the starting preset, quiet side first.
    #[test]
    fn the_starting_preset_is_picked_from_the_gpu_quiet_first() {
        assert_eq!(Graphics::auto_preset("Apple M1", false), Preset::Medium);
        assert_eq!(Graphics::auto_preset("Apple M1 Max", false), Preset::Medium);
        assert_eq!(Graphics::auto_preset("Apple M3 Max", false), Preset::High);
        assert_eq!(Graphics::auto_preset("Apple M4", false), Preset::Medium);
        assert_eq!(
            Graphics::auto_preset("NVIDIA GeForce RTX 4070 Ti", false),
            Preset::High
        );
        assert_eq!(
            Graphics::auto_preset("llvmpipe (LLVM 17.0.6, 256 bits)", false),
            Preset::Low
        );
        assert_eq!(
            Graphics::auto_preset("Adreno (TM) 740", true),
            Preset::Medium
        );
        assert_eq!(
            Graphics::auto_preset("Intel(R) Iris(R) Xe Graphics", false),
            Preset::Medium
        );
        assert_eq!(Graphics::auto_preset("", false), Preset::Medium);
    }

    /// A settings file from a newer build, or one edited by hand, keeps
    /// every knob it can read and defaults the rest — it never refuses.
    #[test]
    fn an_unknown_value_falls_back_to_its_default_and_refuses_nothing() {
        let read: Graphics = serde_json::from_str(
            r#"{"preset":"cinematic","anti_aliasing":"fxaa","frame_limit":7,"effects":"high"}"#,
        )
        .expect("reads");
        assert_eq!(read.preset, Preset::Medium);
        assert_eq!(read.anti_aliasing, AntiAliasing::Fxaa);
        assert_eq!(read.frame_limit, FrameLimit::Fps60);
        assert_eq!(read.effects, Effects::High);
        let empty: Graphics = serde_json::from_str("{}").expect("reads");
        assert_eq!(empty, Graphics::default());
        let kept: Graphics =
            serde_json::from_str(&serde_json::to_string(&read).expect("writes")).expect("reads");
        assert_eq!(kept, read);
    }

    /// The anti-aliasing sample counts the renderer asks for.
    #[test]
    fn anti_aliasing_names_its_samples() {
        let samples: Vec<u32> = AntiAliasing::ALL.iter().map(|a| a.samples()).collect();
        assert_eq!(samples, [1, 1, 2, 4]);
        assert!(!Effects::Low.ambient_motion());
        assert!(Effects::Medium.ambient_motion() && Effects::High.ambient_motion());
    }
}
