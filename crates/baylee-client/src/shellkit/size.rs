//! Size class × input class × text step (the shell design, §2.7 and §8).
//!
//! Two axes are read from the window every frame. The **size class**
//! ([`Frame`]) decides layout; the **input class** ([`InputClass`]) decides
//! target sizes and affordances. An iPad at 1180 × 820 is Wide and Touch.
//!
//! `Phone` is decided on the **raw** logical height (or a phone platform),
//! never on the text step, so no step can turn a phone into a desktop
//! (M4-4: 844 × 390 divided by a small step's factor would read as Narrow or
//! Wide). The other
//! four classes come from the **effective** width — logical pixels divided by
//! the step's factor — so a larger step narrows the layout as it should.

use bevy::input::mouse::MouseMotion;
use bevy::input::touch::TouchInput;
use bevy::prelude::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The five text steps, per device (`ClientSettings::text_size`), geometric
/// at a ratio of 1.125 around the default, which is step 3 (`M`): the size
/// the shell had before there were steps.
///
/// Renamed on 09.10.2026 (the owner: the default reads M). The scale moved
/// up one name and gained a step at the top: today's M is what was L, L
/// what was XL, XL a new step 1.125 above that, S what was M, XS what was
/// S; the old smallest step (0.702) is gone. A stored step is migrated so
/// nobody's size changes ([`TextSize::from_stored`]); one stored at the
/// old XS reads the new XS, the nearest that is left.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TextSize {
    /// Step 1, 0.790 (the old `S`).
    Xs,
    /// Step 2, 0.889 (the old `M`).
    S,
    /// Step 3, 1.000: the default, the shell's sizes before the steps (the
    /// old `L`).
    #[default]
    M,
    /// Step 4, 1.125 (the old `XL`).
    L,
    /// Step 5, 1.266: new on 09.10.2026.
    Xl,
}

impl TextSize {
    /// Every step, smallest first.
    pub const ALL: [Self; 5] = [Self::Xs, Self::S, Self::M, Self::L, Self::Xl];

    /// What the step multiplies the shell's sizes by: 1.125 to the power of
    /// the step's distance from `M`, rounded as §8 tabulates it.
    #[must_use]
    pub const fn factor(self) -> f32 {
        match self {
            Self::Xs => 0.790,
            Self::S => 0.889,
            Self::M => 1.0,
            Self::L => 1.125,
            Self::Xl => 1.266,
        }
    }

    /// The step's number, 1 to 5.
    #[must_use]
    pub const fn step(self) -> u8 {
        match self {
            Self::Xs => 1,
            Self::S => 2,
            Self::M => 3,
            Self::L => 4,
            Self::Xl => 5,
        }
    }

    /// The step numbered `n` (1 to 5), as [`Self::step`] numbers them.
    #[must_use]
    pub const fn of_step(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::Xs),
            2 => Some(Self::S),
            3 => Some(Self::M),
            4 => Some(Self::L),
            5 => Some(Self::Xl),
            _ => None,
        }
    }

    /// The step's name as the settings screen shows it, and as dev-control
    /// takes it (`xs s m l xl`, lower case there).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Xs => "XS",
            Self::S => "S",
            Self::M => "M",
            Self::L => "L",
            Self::Xl => "XL",
        }
    }

    /// What a settings file stored, read as a step that keeps the size it
    /// had. Since 09.10.2026 a step is stored as its number (`3`); a file
    /// from before stored the old name (`"l"`), which meant the step one
    /// name up: old `l` (1.000) is the new `M`, old `xl` (1.125) the new
    /// `L`, old `m` the new `S`, old `s` the new `XS`, and old `xs` (0.702,
    /// a size there is no longer) the new `XS`. Anything else reads `None`.
    #[must_use]
    pub fn from_stored(value: &serde_json::Value) -> Option<Self> {
        if let Some(n) = value.as_u64() {
            return u8::try_from(n).ok().and_then(Self::of_step);
        }
        match value.as_str()? {
            "xs" | "s" => Some(Self::Xs),
            "m" => Some(Self::S),
            "l" => Some(Self::M),
            "xl" => Some(Self::L),
            _ => None,
        }
    }

    /// One step larger, or this one at the top.
    #[must_use]
    pub const fn larger(self) -> Self {
        match self {
            Self::Xs => Self::S,
            Self::S => Self::M,
            Self::M => Self::L,
            Self::L | Self::Xl => Self::Xl,
        }
    }

    /// One step smaller, or this one at the bottom.
    #[must_use]
    pub const fn smaller(self) -> Self {
        match self {
            Self::Xs | Self::S => Self::Xs,
            Self::M => Self::S,
            Self::L => Self::M,
            Self::Xl => Self::L,
        }
    }
}

/// Stored as the step's number, which no file from before the rename wrote
/// (those wrote a name), so the two can never be confused.
impl Serialize for TextSize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.step())
    }
}

/// Reads a number (this build's) or an old name (migrated, keeping the
/// size); anything else is the default, as every lenient knob reads.
impl<'de> Deserialize<'de> for TextSize {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(Self::from_stored(&value).unwrap_or_default())
    }
}

/// How much room there is: the five size classes of §2.7.
///
/// `Compact`, `Narrow` and `Wide` are today's three lobby frames (renamed in
/// `WP0a`); `Phone` and `Vast` are the shell's. [`Frame::of`] is the old
/// width-only reading, which never answers `Phone` or `Vast`: the screens
/// not yet moved onto the shell keep drawing exactly what they drew.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Frame {
    /// A landscape phone: raw logical height under 500, or a phone platform.
    Phone,
    /// Effective width under 760: a small desktop window.
    Compact,
    /// 760 to 1180: a tablet, or a half-screen window.
    Narrow,
    /// 1180 to 2560: a desktop window, a large tablet.
    Wide,
    /// 2560 and wider: 4K at scale 1.
    Vast,
}

/// Below this raw logical height the window is a phone's (§2.7).
pub const PHONE_HEIGHT: f32 = 500.0;
/// The effective widths the other four classes change at.
pub const COMPACT_BELOW: f32 = 760.0;
/// See [`COMPACT_BELOW`].
pub const NARROW_BELOW: f32 = 1180.0;
/// See [`COMPACT_BELOW`].
pub const WIDE_BELOW: f32 = 2560.0;

impl Frame {
    /// The frame a window of this width is in, the lobby's reading before
    /// the shell: three classes, width only, no text step.
    #[must_use]
    pub fn of(width: f32) -> Self {
        if width < COMPACT_BELOW {
            Self::Compact
        } else if width < NARROW_BELOW {
            Self::Narrow
        } else {
            Self::Wide
        }
    }

    /// The shell's reading: all five classes (§2.7, §8).
    #[must_use]
    pub fn classify(view: Viewport, step: TextSize) -> Self {
        // The raw height alone: a native tablet (Android or iOS, 1180 x 820)
        // is Wide + Touch like an iPad in a browser, not a phone; a phone in
        // landscape is under 500 tall at every scale measured (the table
        // design, amendment to §2.7).
        if view.height < PHONE_HEIGHT {
            return Self::Phone;
        }
        let effective = view.width / step.factor();
        if effective < COMPACT_BELOW {
            Self::Compact
        } else if effective < NARROW_BELOW {
            Self::Narrow
        } else if effective < WIDE_BELOW {
            Self::Wide
        } else {
            Self::Vast
        }
    }
}

/// What the client is running on, as far as the shell's layout cares.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Platform {
    /// macOS, Windows, Linux.
    Desktop,
    /// A browser.
    Web,
    /// Android or iOS: always `Touch`, and sized by its window like any
    /// other — a tablet is not a phone.
    Mobile,
}

impl Platform {
    /// The platform this build runs on.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(any(target_os = "android", target_os = "ios")) {
            Self::Mobile
        } else if cfg!(target_arch = "wasm32") {
            Self::Web
        } else {
            Self::Desktop
        }
    }

    /// Whether this platform is touch-only (Android, iOS).
    #[must_use]
    pub const fn is_mobile(self) -> bool {
        matches!(self, Self::Mobile)
    }
}

/// The window as the shell reads it: logical size, platform, input class.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Viewport {
    /// Logical width.
    pub width: f32,
    /// Logical height.
    pub height: f32,
    /// What the build runs on.
    pub platform: Platform,
    /// Finger or pointer.
    pub input: InputClass,
}

impl Viewport {
    /// A desktop viewport with a pointer, for tests and for the fallback
    /// when no window exists (a headless app).
    #[must_use]
    pub const fn desktop(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            platform: Platform::Desktop,
            input: InputClass::Pointer,
        }
    }
}

/// Finger or pointer (§2.7): decides target sizes and affordances.
///
/// `Touch` on Android and iOS from the start, and on any platform after the
/// first touch; `Pointer` otherwise, and again at the first mouse move (a
/// tablet with a trackpad). The browser's `pointer: coarse` query is not
/// read yet: the first touch is.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputClass {
    /// A mouse or a trackpad: hover exists, key caps are drawn.
    Pointer,
    /// A finger: every target ≥ 44 × 44, no hover-only affordance.
    Touch,
}

impl Default for InputClass {
    fn default() -> Self {
        if Platform::current().is_mobile() {
            Self::Touch
        } else {
            Self::Pointer
        }
    }
}

/// Pins the input class, so a dev-control run can measure the Touch layout
/// on a desktop (`/input {"class":"touch"}`). Absent in a normal run.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct PinnedInput(pub InputClass);

/// Follows the last kind of input: a touch makes it Touch, a mouse move
/// Pointer. Written only when it changes, so screens can rebuild on it.
pub(crate) fn follow_the_input(
    mut touches: MessageReader<TouchInput>,
    mut motion: MessageReader<MouseMotion>,
    pinned: Option<Res<PinnedInput>>,
    mut class: ResMut<InputClass>,
) {
    let touched = touches.read().count() > 0;
    let moved = motion.read().count() > 0;
    let want = if let Some(pinned) = pinned {
        pinned.0
    } else if touched {
        InputClass::Touch
    } else if moved && !Platform::current().is_mobile() {
        InputClass::Pointer
    } else {
        *class
    };
    class.set_if_neq(want);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M4-4: Phone reads the raw height, so no text step turns a landscape
    /// phone into a desktop; the other classes read the effective width.
    #[test]
    fn a_phone_is_a_phone_at_every_step_and_the_rest_follow_the_effective_width() {
        for step in TextSize::ALL {
            for (w, h) in [(844.0, 390.0), (920.0, 443.0), (640.0, 360.0)] {
                assert_eq!(
                    Frame::classify(Viewport::desktop(w, h), step),
                    Frame::Phone,
                    "{w} x {h} at step {}",
                    step.step()
                );
            }
        }
        assert_eq!(
            Frame::classify(Viewport::desktop(1200.0, 800.0), TextSize::Xl),
            Frame::Narrow
        );
        assert_eq!(
            Frame::classify(Viewport::desktop(1200.0, 800.0), TextSize::Xs),
            Frame::Wide
        );
        assert_eq!(
            Frame::classify(Viewport::desktop(1000.0, 800.0), TextSize::Xs),
            Frame::Wide
        );
        assert_eq!(
            Frame::classify(Viewport::desktop(2560.0, 1440.0), TextSize::M),
            Frame::Vast
        );
        assert_eq!(
            Frame::classify(Viewport::desktop(1920.0, 1080.0), TextSize::M),
            Frame::Wide
        );
        assert_eq!(
            Frame::classify(Viewport::desktop(960.0, 700.0), TextSize::M),
            Frame::Narrow
        );
        assert_eq!(
            Frame::classify(Viewport::desktop(700.0, 700.0), TextSize::M),
            Frame::Compact
        );
        // A native tablet is sized by its window: Wide, not Phone; a native
        // phone in landscape is a Phone by its height.
        let tablet = Viewport {
            platform: Platform::Mobile,
            ..Viewport::desktop(1180.0, 820.0)
        };
        assert_eq!(Frame::classify(tablet, TextSize::M), Frame::Wide);
        let phone = Viewport {
            platform: Platform::Mobile,
            ..Viewport::desktop(920.0, 443.0)
        };
        assert_eq!(Frame::classify(phone, TextSize::M), Frame::Phone);
    }

    /// The old width-only reading never answers the shell's two new classes,
    /// so the screens not yet on the shell draw what they drew.
    #[test]
    fn the_old_reading_keeps_its_three_classes() {
        assert_eq!(Frame::of(400.0), Frame::Compact);
        assert_eq!(Frame::of(844.0), Frame::Narrow);
        assert_eq!(Frame::of(5000.0), Frame::Wide);
    }

    /// The rename of 09.10.2026: a step a file stored before it keeps its
    /// size (old `l` 1.000 is the new `M`, old `xl` the new `L`, …), the old
    /// smallest step reads the new smallest, and this build stores numbers,
    /// which no older file holds.
    #[test]
    fn a_stored_step_keeps_its_size_across_the_rename() {
        let old = [
            ("xs", TextSize::Xs, 0.790),
            ("s", TextSize::Xs, 0.790),
            ("m", TextSize::S, 0.889),
            ("l", TextSize::M, 1.0),
            ("xl", TextSize::L, 1.125),
        ];
        for (name, want, factor) in old {
            let read: TextSize =
                serde_json::from_value(serde_json::json!(name)).expect("an old name reads");
            assert_eq!(read, want, "old {name:?}");
            assert!((read.factor() - factor).abs() < 1e-6, "old {name:?}");
        }
        // The old sizes, kept: old s, m, l, xl were 0.790, 0.889, 1.0, 1.125.
        for step in TextSize::ALL {
            let text = serde_json::to_string(&step).expect("serializes");
            assert_eq!(text, step.step().to_string(), "stored as its number");
            let back: TextSize = serde_json::from_str(&text).expect("reads back");
            assert_eq!(back, step);
        }
        for junk in [
            serde_json::json!("huge"),
            serde_json::json!(9),
            serde_json::json!(null),
        ] {
            let read: TextSize = serde_json::from_value(junk).expect("anything reads");
            assert_eq!(read, TextSize::M);
        }
        assert!((TextSize::M.factor() - 1.0).abs() < f32::EPSILON);
        assert_eq!(TextSize::default(), TextSize::M);
        assert!(
            TextSize::Xl.factor() > 1.125,
            "a step above the old largest"
        );
    }

    #[test]
    fn the_factors_are_the_ratio_the_design_tabulates() {
        for pair in TextSize::ALL.windows(2) {
            let ratio = pair[1].factor() / pair[0].factor();
            assert!((ratio - 1.125).abs() < 0.002, "{pair:?}: {ratio}");
        }
        assert_eq!(TextSize::default(), TextSize::M);
    }
}
