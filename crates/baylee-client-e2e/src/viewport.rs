//! The window sizes a test runs at, and the interface's text steps.
//!
//! Sizes are **logical** pixels: what the client lays out in and what every
//! rectangle in `/state` is measured in. A client is started at scale factor
//! 1 (`BAYLEE_DEV_SCALE=1`), so a logical size is also the window's physical
//! size and a 2560 × 1440 layout fits on a Retina screen.
//!
//! The presets sit on the shell's own breakpoints (`lobby::shell::Metrics::of`,
//! `crates/baylee-client/CLAUDE.md` §"Lobby and rooms"): Compact below 760
//! wide, Narrow below 1180, Wide above; Phone below 500 high; Vast past
//! 1920 × 1080, where every shell size grows by `screen_scale`.
use std::fmt;

/// A window size with a name a failure can be read by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    /// What a report calls it (`"phone"`, `"full-hd"`).
    pub name: &'static str,
    /// Logical width.
    pub width: u32,
    /// Logical height.
    pub height: u32,
}

impl Viewport {
    /// A phone held sideways (Phone: under 500 high), as the shell tools size it.
    pub const PHONE: Self = Self::new("phone", 844, 390);
    /// Compact: stacked panels, 44 px targets.
    pub const COMPACT: Self = Self::new("compact", 720, 900);
    /// Narrow, at the shell tools' small desktop.
    pub const NARROW: Self = Self::new("narrow", 960, 700);
    /// The desktop layout floor; below it the interface shrinks uniformly.
    pub const DESKTOP_FLOOR: Self = Self::new("desktop-floor", 1280, 720);
    /// A common laptop.
    pub const LAPTOP: Self = Self::new("laptop", 1440, 900);
    /// 1920 × 1080, the last size before Vast.
    pub const FULL_HD: Self = Self::new("full-hd", 1920, 1080);
    /// 2560 × 1440: Vast, shell sizes × 1.25.
    pub const QHD: Self = Self::new("qhd", 2560, 1440);
    /// 3840 × 2160: Vast at its largest step (× 1.75). Needs a screen that
    /// holds it at scale 1; [`crate::Client::resize`] says so when it does not.
    pub const UHD: Self = Self::new("uhd", 3840, 2160);

    /// Every frame class the shell knows, smallest first, without 4K.
    pub const ALL: &'static [Self] = &[
        Self::PHONE,
        Self::COMPACT,
        Self::NARROW,
        Self::DESKTOP_FLOOR,
        Self::LAPTOP,
        Self::FULL_HD,
        Self::QHD,
    ];
    /// The desktop sizes the table is played at.
    pub const DESKTOP: &'static [Self] = &[Self::DESKTOP_FLOOR, Self::LAPTOP, Self::FULL_HD];
    /// The sizes under the desktop floor.
    pub const SMALL: &'static [Self] = &[Self::PHONE, Self::COMPACT, Self::NARROW];

    /// A size of a test's own.
    pub const fn new(name: &'static str, width: u32, height: u32) -> Self {
        Self {
            name,
            width,
            height,
        }
    }

    /// Whether a window measured `width × height` is this size, give or take
    /// the pixel a window manager rounds by.
    pub fn is(self, width: f32, height: f32) -> bool {
        (f64::from(width) - f64::from(self.width)).abs() <= 1.0
            && (f64::from(height) - f64::from(self.height)).abs() <= 1.0
    }

    /// `BAYLEE_DEV_WINDOW`'s spelling.
    pub fn spelled(self) -> String {
        format!("{}x{}", self.width, self.height)
    }
}

impl fmt::Display for Viewport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}×{})", self.name, self.width, self.height)
    }
}

/// The interface's text steps (Settings → Text size; `shellkit::TextSize`,
/// stored as its step number).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextStep {
    /// Step 1, × 0.790.
    Xs,
    /// Step 2, × 0.889.
    S,
    /// Step 3, × 1.000: the default.
    #[default]
    M,
    /// Step 4, × 1.125.
    L,
    /// Step 5, × 1.266.
    Xl,
}

impl TextStep {
    /// Every step, smallest first.
    pub const ALL: [Self; 5] = [Self::Xs, Self::S, Self::M, Self::L, Self::Xl];

    /// The number `client-settings.json` stores (`text_size`).
    pub const fn stored(self) -> u8 {
        match self {
            Self::Xs => 1,
            Self::S => 2,
            Self::M => 3,
            Self::L => 4,
            Self::Xl => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TextStep, Viewport};

    // The presets land in the frame classes they are named for: Phone is a
    // height class, Compact a width class (`Metrics::of`).
    const _: () = {
        assert!(Viewport::PHONE.height < 500);
        assert!(Viewport::COMPACT.width < 760 && Viewport::COMPACT.height >= 500);
        assert!(Viewport::NARROW.width >= 760 && Viewport::NARROW.width < 1180);
        assert!(Viewport::DESKTOP_FLOOR.width >= 1180);
        assert!(Viewport::QHD.width > 1920 && Viewport::QHD.height > 1080);
    };

    #[test]
    fn every_size_is_smallest_first_and_a_window_matches_its_size() {
        assert!(Viewport::LAPTOP.is(1440.0, 899.0));
        assert!(!Viewport::LAPTOP.is(1440.0, 897.0));
        let mut sorted = Viewport::ALL.to_vec();
        sorted.sort_by_key(|v| v.width * v.height);
        assert_eq!(sorted, Viewport::ALL, "ALL is smallest first");
    }

    #[test]
    fn a_text_step_is_stored_as_its_number() {
        let stored: Vec<u8> = TextStep::ALL.iter().map(|s| s.stored()).collect();
        assert_eq!(stored, [1, 2, 3, 4, 5]);
        assert_eq!(TextStep::default(), TextStep::M);
    }
}
