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
/// Measured on the M1 Max at a duel: MSAA 4× against none, 0.2 ms of GPU per
/// frame, inside the noise (Apple's tile memory resolves samples nearly
/// free); FXAA is not measured yet. Without either, the card and slab edges
/// stair-step visibly (`docs/perf-client.md`).
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
/// shaders again: the front door drew 120 frames when the GPU let it, and
/// the cap halves that work (`docs/perf-client.md` has what was measured).
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
/// (not focused) or a menu has stood untouched for [`IDLE_AFTER_SECS`].
/// While visible, the effective pace never drops below [`MIN_VISIBLE_FPS`]
/// (30 fps); a minimised or occluded window pauses at [`Pace::Paused`] (0 fps).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundLimit {
    /// Five in stored settings; clamped to [`MIN_VISIBLE_FPS`] while visible.
    Fps5,
    /// Fifteen in stored settings; clamped to [`MIN_VISIBLE_FPS`] while visible.
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

/// How many frames a table at rest draws (`Bildrate in Ruhe`): after
/// [`TABLE_SETTLE_SECS`] without input and without anything happening in the
/// game, the table eases to this and stays there until either comes back.
/// Never above the frame limit; at or above it the table does not ease at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestLimit {
    /// Thirty: half the cost of sixty, and the slow drift of the cloth and
    /// a flier's bob read the same (`docs/perf-baseline.md`, 08.10.2026).
    #[default]
    Fps30,
    /// Sixty.
    Fps60,
}

impl RestLimit {
    /// Every choice, in the order a picker offers them.
    pub const ALL: [Self; 2] = [Self::Fps30, Self::Fps60];

    /// Frames per second.
    #[must_use]
    pub const fn fps(self) -> u32 {
        match self {
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

    /// Whether the table's channels are drawn cut into the slab, with the
    /// eye's ray followed down their banks; without it they are drawn from
    /// straight above, at every angle, and without their caustics.
    ///
    /// Only `Low` goes without. `Medium` is where every laptop and phone
    /// starts (`Graphics::auto_preset`), and the carved table is the table.
    #[must_use]
    pub const fn carved(self) -> bool {
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

/// The painting behind every screen outside the table (WP5, N4-6): whole,
/// dimmed, or none at all — readability for low vision, and a cheaper frame
/// on a weak GPU (the scene is not drawn at all on `Plain`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backdrop {
    /// The painting as it is.
    #[default]
    Painting,
    /// At 40 % of its light.
    Dimmed,
    /// None: the flat ground the camera clears to.
    Plain,
}

impl Backdrop {
    /// Every choice, in the order a picker offers them.
    pub const ALL: [Self; 3] = [Self::Painting, Self::Dimmed, Self::Plain];

    /// How much of the painting is drawn: its alpha over the flat ground.
    #[must_use]
    pub const fn share(self) -> f32 {
        match self {
            Self::Painting => 1.0,
            Self::Dimmed => 0.4,
            Self::Plain => 0.0,
        }
    }
}

/// How the window stands on a desktop (WP5): a window, a borderless window
/// over the whole monitor, or the monitor's own fullscreen. Not a preset's
/// knob: a preset never moves the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    /// A window.
    #[default]
    Windowed,
    /// A window without its frame, covering the monitor.
    Borderless,
    /// The monitor's fullscreen.
    Fullscreen,
}

impl DisplayMode {
    /// Every choice, in the order a picker offers them.
    pub const ALL: [Self; 3] = [Self::Windowed, Self::Borderless, Self::Fullscreen];
}

/// Seconds a changed display mode stands before it reverts by itself,
/// unless kept (S4-7): a mode the monitor cannot show leaves the player a
/// way back without seeing anything.
pub const DISPLAY_REVERT_SECS: f32 = 15.0;

/// Active frame-rate cap on menus (sign-in, lobby, deck builder, settings):
/// matches the `60/1` fps painted-world AVIF loops even on `High` (`120` fps
/// in 3D play) and `Ultra` (`Unlimited` in 3D play).
pub const MENU_ACTIVE_FPS: u32 = 60;

/// Seconds without any user or game action after which an active screen eases
/// smoothly to [`MIN_VISIBLE_FPS`] (`30` fps). Any user action or game event
/// brings the active rate back immediately.
pub const IDLE_AFTER_SECS: f32 = 120.0;

/// Seconds without input **or anything happening in the game** after which
/// the table eases from the frame limit to its [`RestLimit`]. Any input or
/// game event brings the full rate back on the frame it arrives.
pub const TABLE_SETTLE_SECS: f32 = 120.0;

/// Hard floor on frames per second whenever the window is visible (focused,
/// unfocused, or idle). Only a minimised or completely occluded window drops
/// below this (to [`Pace::Paused`], 0 fps).
pub const MIN_VISIBLE_FPS: u32 = 30;

/// The device's graphics settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, from = "Stored")]
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
    /// Frames per second at a table at rest.
    pub rest_limit: RestLimit,
    /// How much the ambient surfaces do.
    #[serde(deserialize_with = "lenient")]
    pub effects: Effects,
    /// The painting behind the screens outside the table.
    #[serde(deserialize_with = "lenient")]
    pub backdrop: Backdrop,
    /// How the window stands (desktop). Not a preset's knob.
    #[serde(deserialize_with = "lenient")]
    pub display_mode: DisplayMode,
    /// The frame-time counter in a corner. Not a preset's knob.
    pub show_frame_rate: bool,
}

impl Default for Graphics {
    fn default() -> Self {
        Self::of(Preset::Medium)
    }
}

/// [`Graphics`] as a file holds it. A file written before the backdrop
/// existed has none: it takes its preset's, so a device on Low stays
/// Low, painting and all, and is not turned `Custom` by its next change.
#[derive(Deserialize)]
#[serde(default)]
struct Stored {
    #[serde(deserialize_with = "lenient")]
    preset: Preset,
    #[serde(deserialize_with = "lenient")]
    anti_aliasing: AntiAliasing,
    #[serde(deserialize_with = "lenient")]
    vsync: VSync,
    #[serde(deserialize_with = "lenient")]
    frame_limit: FrameLimit,
    #[serde(deserialize_with = "lenient")]
    background_limit: BackgroundLimit,
    #[serde(deserialize_with = "lenient_option")]
    rest_limit: Option<RestLimit>,
    #[serde(deserialize_with = "lenient")]
    effects: Effects,
    #[serde(deserialize_with = "lenient_option")]
    backdrop: Option<Backdrop>,
    #[serde(deserialize_with = "lenient")]
    display_mode: DisplayMode,
    show_frame_rate: bool,
}

impl Default for Stored {
    fn default() -> Self {
        let g = Graphics::default();
        Self {
            preset: g.preset,
            anti_aliasing: g.anti_aliasing,
            vsync: g.vsync,
            frame_limit: g.frame_limit,
            background_limit: g.background_limit,
            rest_limit: None,
            effects: g.effects,
            backdrop: None,
            display_mode: g.display_mode,
            show_frame_rate: g.show_frame_rate,
        }
    }
}

impl From<Stored> for Graphics {
    fn from(s: Stored) -> Self {
        Self {
            preset: s.preset,
            anti_aliasing: s.anti_aliasing,
            vsync: s.vsync,
            frame_limit: s.frame_limit,
            background_limit: s.background_limit,
            rest_limit: s.rest_limit.unwrap_or(Self::of(s.preset).rest_limit),
            effects: s.effects,
            backdrop: s.backdrop.unwrap_or(Self::of(s.preset).backdrop),
            display_mode: s.display_mode,
            show_frame_rate: s.show_frame_rate,
        }
    }
}

/// [`lenient`] for a value a file may leave out: what does not parse is
/// absent.
fn lenient_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
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
            // Every preset eases an untouched screen to 30 fps after 120s;
            // `RestLimit::Fps60` remains available as an explicit user setting.
            rest_limit: RestLimit::Fps30,
            effects,
            backdrop: if matches!(preset, Preset::Low) {
                Backdrop::Plain
            } else {
                Backdrop::Painting
            },
            display_mode: DisplayMode::Windowed,
            show_frame_rate: false,
        }
    }

    /// These settings with `preset`'s knobs, keeping what no preset sets
    /// (the display mode, the frame-rate counter): the preset row writes
    /// the preset's knobs and nothing else.
    #[must_use]
    pub const fn with_preset(self, preset: Preset) -> Self {
        let mut next = Self::of(preset);
        next.display_mode = self.display_mode;
        next.show_frame_rate = self.show_frame_rate;
        next
    }

    /// The knobs that differ from the named preset's, for the preset row's
    /// *Custom · n changes* disclosure; none when it is a named preset.
    #[must_use]
    pub fn changes_from(&self, preset: Preset) -> Vec<&'static str> {
        let base = Self::of(preset);
        let mut out = Vec::new();
        if self.anti_aliasing != base.anti_aliasing {
            out.push("anti_aliasing");
        }
        if self.vsync != base.vsync {
            out.push("vsync");
        }
        if self.frame_limit != base.frame_limit {
            out.push("frame_limit");
        }
        if self.background_limit != base.background_limit {
            out.push("background_limit");
        }
        if self.rest_limit != base.rest_limit {
            out.push("rest_limit");
        }
        if self.effects != base.effects {
            out.push("effects");
        }
        if self.backdrop != base.backdrop {
            out.push("backdrop");
        }
        out
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
            && self.rest_limit == other.rest_limit
            && self.effects == other.effects
            && self.backdrop == other.backdrop
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
            let pro_max_ultra =
                name.contains("pro") || name.contains("max") || name.contains("ultra");
            // Every Pro, Max or Ultra Apple Silicon GPU (including M1 Pro,
            // M1 Max and M1 Ultra) starts at High; base M-series laptops and
            // tablets stay at Medium.
            return if pro_max_ultra {
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
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // independent facts about one window, not a state
pub struct Showing {
    /// The window has the keyboard focus.
    pub focused: bool,
    /// The window is hidden, minimised or entirely covered.
    pub hidden: bool,
    /// A menu (the front door, the lobby, settings) rather than a table.
    pub menu: bool,
    /// Seconds since the last key, click, pointer move, wheel or touch — or,
    /// at the table, since the game last moved (a new view, a tear).
    pub untouched_secs: f32,
    /// Nothing ambient moves (effects `Low`, or reduced motion): an idle
    /// menu then has nothing to draw until something happens.
    pub still: bool,
    /// A picture that moves by itself is being shown (the front door's and
    /// the lobby's loops): a menu left alone then goes on drawing, slowly,
    /// because a film that stops after half a minute reads as broken.
    pub picture: bool,
}

/// How often frames are drawn: as fast as allowed, one every so often, or
/// paused while hidden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    /// No limit beyond the display (or vsync off, none at all).
    Unlimited,
    /// At most this many frames a second.
    Fps(u32),
    /// Zero frames: the window is minimised or completely occluded.
    Paused,
}

impl Pace {
    /// Seconds between frames, or `None` for no limit (`INFINITY` when paused).
    #[must_use]
    pub fn interval_secs(self) -> Option<f32> {
        match self {
            Self::Unlimited => None,
            #[allow(clippy::cast_precision_loss)]
            Self::Fps(fps) => Some(1.0 / fps.max(1) as f32),
            Self::Paused => Some(f32::INFINITY),
        }
    }
}

impl Graphics {
    /// Whether ambient clocks may pause completely: when the window is hidden
    /// (minimised or occluded), or on an idle menu whose ambient motion was
    /// already turned off (`Effects::Low` or `reduce_motion`). Visible ambient
    /// scenes and video loops never freeze at the 30 fps idle rate.
    #[must_use]
    pub fn rests(&self, showing: Showing) -> bool {
        showing.hidden
            || (showing.menu && showing.still && showing.untouched_secs >= IDLE_AFTER_SECS)
    }

    /// The pace for a window in this state, and whether input should wake a
    /// frame at once (a reduced pace) or wait for the next one (a cap).
    ///
    /// Policy:
    /// - Hidden (minimised or completely occluded): [`Pace::Paused`] (0 fps).
    /// - Visible (focused, unfocused, or idle): never below [`MIN_VISIBLE_FPS`]
    ///   (30 fps).
    /// - Active focused menu (sign-in, lobby, settings): runs at
    ///   [`MENU_ACTIVE_FPS`] (60 fps, or [`Self::frame_limit`] when lower) for
    ///   [`IDLE_AFTER_SECS`] (`120.0`s), eases smoothly to 30 fps, and returns
    ///   immediately to 60 fps on any user action.
    /// - Active focused table: runs at [`Self::frame_limit`] (60 fps on
    ///   `Medium`, 120 fps on `High`, unlimited on `Ultra`) for
    ///   [`TABLE_SETTLE_SECS`] (`120.0`s), eases to [`Self::rest_limit`]
    ///   (30 fps by default across all presets), and returns immediately on any
    ///   user action or game event.
    /// - Unfocused visible window: [`Self::background_limit`] clamped to at
    ///   least [`MIN_VISIBLE_FPS`] (30 fps), and never faster than the focused
    ///   cap.
    #[must_use]
    pub fn pace(&self, showing: Showing) -> (Pace, bool) {
        if showing.hidden {
            return (Pace::Paused, true);
        }
        let focused = self.frame_limit.fps().map(|f| f.max(MIN_VISIBLE_FPS));
        let held = |fps: u32| {
            let floored = fps.max(MIN_VISIBLE_FPS);
            Pace::Fps(focused.map_or(floored, |cap| floored.min(cap)))
        };
        if !showing.focused {
            let bg = if showing.menu && showing.untouched_secs >= IDLE_AFTER_SECS {
                MIN_VISIBLE_FPS
            } else {
                let raw = self.background_limit.fps().max(MIN_VISIBLE_FPS);
                if showing.menu {
                    raw.min(MENU_ACTIVE_FPS)
                } else {
                    raw
                }
            };
            return (held(bg), true);
        }
        if showing.menu {
            if showing.untouched_secs >= IDLE_AFTER_SECS {
                return (held(MIN_VISIBLE_FPS), true);
            }
            let menu_fps = focused.map_or(MENU_ACTIVE_FPS, |cap| cap.min(MENU_ACTIVE_FPS));
            return (Pace::Fps(menu_fps), false);
        }
        let rest = self.rest_limit.fps().max(MIN_VISIBLE_FPS);
        if showing.untouched_secs >= TABLE_SETTLE_SECS && focused.is_none_or(|cap| rest < cap) {
            return (Pace::Fps(rest), true);
        }
        (focused.map_or(Pace::Unlimited, Pace::Fps), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT_DESK: Showing = Showing {
        focused: true,
        hidden: false,
        menu: false,
        untouched_secs: 0.0,
        still: false,
        picture: false,
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

    /// A file from before the backdrop takes its preset's; one that names
    /// a backdrop keeps it; a WP5 file reads back as written.
    #[test]
    fn a_file_without_a_backdrop_takes_its_preset_s() {
        // Low as a file before WP5 wrote it: every knob but the new three.
        let mut old = serde_json::to_value(Graphics::of(Preset::Low)).unwrap();
        for new in ["backdrop", "display_mode", "show_frame_rate"] {
            old.as_object_mut().unwrap().remove(new);
        }
        let g: Graphics = serde_json::from_value(old).expect("reads");
        assert_eq!(g.preset, Preset::Low);
        assert_eq!(g.backdrop, Graphics::of(Preset::Low).backdrop);
        let mut moved = g;
        moved.adjust(|_| {});
        assert_eq!(moved.preset, Preset::Low, "still Low, not Custom");
        let named = r#"{"preset":"low","backdrop":"dimmed"}"#;
        let g: Graphics = serde_json::from_str(named).expect("reads");
        assert_eq!(g.backdrop, Backdrop::Dimmed);
        let mut g = Graphics::of(Preset::High);
        g.show_frame_rate = true;
        let back: Graphics = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back, g);
        let junk: Graphics = serde_json::from_str(r#"{"backdrop":7}"#).expect("reads");
        assert_eq!(junk, Graphics::default());
    }

    /// A file from before the rest rate takes its preset's (`Fps30` across all
    /// presets) and stays that preset; one that names it keeps it, and one that
    /// names nonsense takes the preset's.
    #[test]
    fn a_file_without_a_rest_rate_takes_its_preset_s() {
        for preset in Preset::NAMED {
            let mut old = serde_json::to_value(Graphics::of(preset)).unwrap();
            old.as_object_mut().unwrap().remove("rest_limit");
            let g: Graphics = serde_json::from_value(old).expect("reads");
            assert_eq!(g.rest_limit, RestLimit::Fps30);
            assert_eq!(g.matching_preset(), preset, "still {preset:?}");
        }
        let named: Graphics =
            serde_json::from_str(r#"{"preset":"medium","rest_limit":"fps60"}"#).expect("reads");
        assert_eq!(named.rest_limit, RestLimit::Fps60);
        assert_eq!(named.matching_preset(), Preset::Custom);
        let junk: Graphics =
            serde_json::from_str(r#"{"preset":"high","rest_limit":7}"#).expect("reads");
        assert_eq!(junk.rest_limit, RestLimit::Fps30);
    }

    /// *Custom* appears iff a device row differs from the preset's table
    /// (WP5): every knob a preset sets, moved alone, makes it `Custom`; the
    /// knobs no preset sets (display mode, the counter) never do; and the
    /// preset row keeps those when it writes a preset.
    #[test]
    fn custom_appears_iff_a_preset_knob_differs() {
        for preset in Preset::NAMED {
            let base = Graphics::of(preset);
            assert!(base.changes_from(preset).is_empty());
            let moved: [fn(&mut Graphics); 6] = [
                |g| {
                    g.anti_aliasing = if g.anti_aliasing == AntiAliasing::Off {
                        AntiAliasing::Fxaa
                    } else {
                        AntiAliasing::Off
                    }
                },
                |g| {
                    g.vsync = if g.vsync == VSync::Off {
                        VSync::On
                    } else {
                        VSync::Off
                    }
                },
                |g| {
                    g.frame_limit = if g.frame_limit == FrameLimit::Fps30 {
                        FrameLimit::Fps60
                    } else {
                        FrameLimit::Fps30
                    }
                },
                |g| {
                    g.background_limit = if g.background_limit == BackgroundLimit::Fps5 {
                        BackgroundLimit::Fps60
                    } else {
                        BackgroundLimit::Fps5
                    }
                },
                |g| {
                    g.effects = if g.effects == Effects::Low {
                        Effects::High
                    } else {
                        Effects::Low
                    }
                },
                |g| {
                    g.backdrop = if g.backdrop == Backdrop::Dimmed {
                        Backdrop::Plain
                    } else {
                        Backdrop::Dimmed
                    }
                },
            ];
            for change in moved {
                let mut g = base;
                g.adjust(change);
                // Moving one knob may land on another preset's table; it is
                // then that preset, never a wrong name.
                let named = g.matching_preset();
                assert_eq!(g.preset, named);
                if named == Preset::Custom {
                    assert_eq!(g.changes_from(preset).len(), 1, "{preset:?}");
                }
                assert_ne!(named, preset, "{preset:?}: a moved knob kept its name");
            }
            for change in [
                (|g: &mut Graphics| g.display_mode = DisplayMode::Fullscreen) as fn(&mut Graphics),
                |g| g.show_frame_rate = true,
            ] {
                let mut g = base;
                g.adjust(change);
                assert_eq!(g.preset, preset, "not a preset's knob");
            }
        }
        let mine = Graphics {
            display_mode: DisplayMode::Borderless,
            show_frame_rate: true,
            ..Graphics::of(Preset::Low)
        };
        let high = mine.with_preset(Preset::High);
        assert_eq!(high.preset, Preset::High);
        assert_eq!(
            (high.display_mode, high.show_frame_rate),
            (DisplayMode::Borderless, true)
        );
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

    /// Active focused menus and tables hold their full rate for `120.0`s, ease
    /// smoothly to 30 fps (or the table's `RestLimit`), and return immediately
    /// on any user or game action.
    #[test]
    fn the_pace_follows_the_window() {
        let medium = Graphics::of(Preset::Medium);
        assert_eq!(medium.pace(AT_DESK), (Pace::Fps(60), false));
        for active_secs in [0.0, 2.0, 30.0, 119.9] {
            let table_active = Showing {
                untouched_secs: active_secs,
                ..AT_DESK
            };
            let menu_active = Showing {
                menu: true,
                untouched_secs: active_secs,
                ..AT_DESK
            };
            assert_eq!(medium.pace(table_active), (Pace::Fps(60), false));
            assert_eq!(medium.pace(menu_active), (Pace::Fps(60), false));
            assert!(!medium.rests(menu_active));
        }
        let table_resting = Showing {
            untouched_secs: TABLE_SETTLE_SECS,
            ..AT_DESK
        };
        assert_eq!(medium.pace(table_resting), (Pace::Fps(30), true));
        let table_untouched = Showing {
            untouched_secs: 600.0,
            ..AT_DESK
        };
        assert_eq!(medium.pace(table_untouched), (Pace::Fps(30), true));
        // Resting at sixty under a cap of sixty stays at the cap without waking.
        let mut sixty = medium;
        sixty.rest_limit = RestLimit::Fps60;
        assert_eq!(sixty.pace(table_untouched), (Pace::Fps(60), false));
        assert_eq!(sixty.pace(table_untouched), sixty.pace(AT_DESK));
        // High (e.g. Apple M1 Max): menus run 60 -> 120s -> 30 -> 60; tables
        // run 120 -> 120s -> 30 -> 120.
        let high = Graphics::of(Preset::High);
        let ultra = Graphics::of(Preset::Ultra);
        assert_eq!(high.rest_limit, RestLimit::Fps30);
        assert_eq!(ultra.rest_limit, RestLimit::Fps30);
        assert_eq!(high.pace(AT_DESK), (Pace::Fps(120), false));
        assert_eq!(high.pace(table_resting), (Pace::Fps(30), true));
        assert_eq!(high.pace(table_untouched), (Pace::Fps(30), true));
        assert_eq!(ultra.pace(table_resting), (Pace::Fps(30), true));
        let high_menu_active = Showing {
            menu: true,
            untouched_secs: 0.0,
            ..AT_DESK
        };
        assert_eq!(high.pace(high_menu_active), (Pace::Fps(60), false));
        assert_eq!(ultra.pace(high_menu_active), (Pace::Fps(60), false));
        assert_eq!(
            high.pace(Showing {
                untouched_secs: 119.9,
                ..high_menu_active
            }),
            (Pace::Fps(60), false)
        );
        let low = Graphics::of(Preset::Low);
        assert_eq!(low.pace(table_untouched), (Pace::Fps(30), false));
        let menu_idle = Showing {
            menu: true,
            untouched_secs: IDLE_AFTER_SECS,
            ..AT_DESK
        };
        assert_eq!(medium.pace(menu_idle), (Pace::Fps(30), true));
        assert_eq!(high.pace(menu_idle), (Pace::Fps(30), true));
        assert_eq!(ultra.pace(menu_idle), (Pace::Fps(30), true));
        // Immediate return to 60 fps on any user action (`untouched_secs = 0.0`).
        for g in [medium, high, ultra] {
            assert_eq!(
                g.pace(Showing {
                    untouched_secs: 0.0,
                    ..menu_idle
                }),
                (Pace::Fps(60), false)
            );
        }
        let interval = Pace::Fps(60).interval_secs().expect("a limit");
        assert!((interval - 1.0 / 60.0).abs() < 1e-6);
        assert_eq!(Pace::Unlimited.interval_secs(), None);
        assert_eq!(Pace::Paused.interval_secs(), Some(f32::INFINITY));
    }

    /// Every visible window (including unfocused) stays at `>= 30` fps, and
    /// `0` fps (`Pace::Paused`) happens strictly when minimised or occluded.
    #[test]
    fn visible_windows_never_drop_below_thirty_and_only_hidden_pauses() {
        for preset in Preset::NAMED {
            let g = Graphics::of(preset);
            let hidden = Showing {
                hidden: true,
                ..AT_DESK
            };
            assert_eq!(g.pace(hidden), (Pace::Paused, true), "{preset:?} hidden");
            assert!(g.rests(hidden), "{preset:?} hidden rests");
            for focused in [true, false] {
                for menu in [true, false] {
                    for untouched_secs in [0.0, 2.0, 30.0, IDLE_AFTER_SECS, 3600.0] {
                        let s = Showing {
                            focused,
                            hidden: false,
                            menu,
                            untouched_secs,
                            still: false,
                            picture: menu,
                        };
                        let (pace, _) = g.pace(s);
                        assert_ne!(pace, Pace::Paused, "{preset:?} {s:?} paused while visible");
                        if let Pace::Fps(fps) = pace {
                            assert!(
                                fps >= MIN_VISIBLE_FPS,
                                "{preset:?} {s:?} dropped to {fps} fps"
                            );
                        }
                    }
                }
            }
        }
        let medium = Graphics::of(Preset::Medium);
        let away = Showing {
            focused: false,
            ..AT_DESK
        };
        assert_eq!(medium.pace(away), (Pace::Fps(MIN_VISIBLE_FPS), true));
        let ultra = Graphics::of(Preset::Ultra);
        assert_eq!(ultra.pace(AT_DESK), (Pace::Unlimited, false));
        assert_eq!(ultra.pace(away), (Pace::Fps(60), true));
        assert_eq!(
            ultra.pace(Showing {
                menu: true,
                untouched_secs: IDLE_AFTER_SECS,
                ..away
            }),
            (Pace::Fps(MIN_VISIBLE_FPS), true)
        );
        // A reduced pace is still held to the focused cap when that cap is 30.
        let mut odd = Graphics::of(Preset::Low);
        odd.background_limit = BackgroundLimit::Fps60;
        assert_eq!(odd.pace(away), (Pace::Fps(30), true));
    }

    /// Visible moving scenes and loops never freeze at the 30 fps idle rate;
    /// only `still` menus (`Effects::Low` / `reduce_motion`) or hidden windows
    /// pause ambient clocks.
    #[test]
    fn a_moving_picture_keeps_an_idle_menu_drawing() {
        let medium = Graphics::of(Preset::Medium);
        let high = Graphics::of(Preset::High);
        let idle = Showing {
            menu: true,
            untouched_secs: IDLE_AFTER_SECS,
            ..AT_DESK
        };
        assert!(!medium.rests(idle));
        assert!(!high.rests(idle));
        let idle_film = Showing {
            picture: true,
            ..idle
        };
        assert!(!medium.rests(idle_film));
        assert_eq!(medium.pace(idle_film), (Pace::Fps(MIN_VISIBLE_FPS), true));
        assert_eq!(high.pace(idle_film), high.pace(idle));
        let idle_still = Showing {
            still: true,
            ..idle_film
        };
        assert!(medium.rests(idle_still));
        assert!(high.rests(idle_still));
        assert_eq!(medium.pace(idle_still), (Pace::Fps(MIN_VISIBLE_FPS), true));
        assert!(!medium.rests(Showing {
            menu: false,
            ..idle_still
        }));
    }

    /// The device's GPU picks the starting preset, quiet side first, with
    /// Apple M1 Pro / M1 Max / M1 Ultra selecting `Preset::High`.
    #[test]
    fn the_starting_preset_is_picked_from_the_gpu_quiet_first() {
        assert_eq!(Graphics::auto_preset("Apple M1", false), Preset::Medium);
        assert_eq!(Graphics::auto_preset("Apple M1 Pro", false), Preset::High);
        assert_eq!(Graphics::auto_preset("Apple M1 Max", false), Preset::High);
        assert_eq!(Graphics::auto_preset("Apple M1 Ultra", false), Preset::High);
        assert_eq!(Graphics::auto_preset("Apple M3 Max", false), Preset::High);
        assert_eq!(Graphics::auto_preset("Apple M4", false), Preset::Medium);
        assert_eq!(Graphics::auto_preset("Apple M1 Max", true), Preset::Medium);
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

    /// The carved table is what a device shows before its player chose
    /// anything: every level but `Low` draws it, and so does every preset a
    /// GPU can start on except the software rasteriser's.
    #[test]
    fn only_low_effects_draw_the_table_flat() {
        assert!(!Effects::Low.carved());
        assert!(Effects::Medium.carved() && Effects::High.carved());
        for adapter in ["Apple M1", "Apple M1 Max", "NVIDIA GeForce RTX 4070", ""] {
            for mobile in [false, true] {
                let start = Graphics::of(Graphics::auto_preset(adapter, mobile));
                assert!(start.effects.carved(), "{adapter:?}, mobile {mobile}");
            }
        }
        assert!(
            !Graphics::of(Graphics::auto_preset("llvmpipe", false))
                .effects
                .carved()
        );
    }
}
