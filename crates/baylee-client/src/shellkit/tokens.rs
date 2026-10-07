//! The shell's tokens (the shell design, §2.2), measured rather than chosen.
//!
//! The panel is today's sanctuary glass at α .88 over the dock's leather: the
//! lowest alpha at which ink and muted ink still pass against the painting's
//! white pixels is .84, so .88 stays. Sheets, menus and popovers are opaque.
//! **No text stands on the painting without a mist plate** (`MIST` ≥ .80),
//! and muted ink is never used on one: small text there is `INK`.

use crate::hud::palette;
use bevy::prelude::*;

/// The panel ground: `SANCTUARY_PANEL` at α .88.
pub const PANEL: Color = palette::SANCTUARY_PANEL;
/// Sheets, menus, popovers: the same ground, opaque.
pub const OPAQUE: Color = Color::srgb(0.035, 0.075, 0.105);
/// A selected row (`PANEL_HOT`, 5.6 : 1 for muted ink).
pub const SELECTED: Color = palette::PANEL_HOT;
/// A panel's border: `DOCK_EDGE` at .4.
pub const BORDER: Color = Color::srgba(0.48, 0.43, 0.33, 0.4);
/// The plate under every line that would otherwise stand on the painting.
pub const MIST: Color = Color::srgba(0.045, 0.095, 0.13, 0.80);
/// The scrim behind a sheet.
pub const SCRIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
/// Primary text: 11.5 : 1 on a panel against the painting's whitest pixel.
pub const INK: Color = palette::INK;
/// Secondary text: 5.2 : 1 on a panel. Never on a mist plate.
pub const MUTED: Color = palette::MUTED;
/// Disabled text: decoration only, always with a reason line.
pub const DISABLED: Color = Color::srgba(0.90, 0.93, 0.94, 0.40);
/// Focus and selection.
pub const ACCENT: Color = palette::ACCENT;
/// The favourite star, and the one gold face: Return to your game.
pub const GOLD: Color = palette::ACTIVE;
/// Delete, and Leave while hosting.
pub const DANGER: Color = palette::DANGER;
/// The primary face: the blue every default button wears.
pub const PRIMARY: Color = Color::srgb(0.14, 0.24, 0.33);
/// The primary face's rim.
pub const PRIMARY_EDGE: Color = Color::srgb(0.40, 0.54, 0.62);
/// A secondary control's ground.
pub const CONTROL: Color = palette::DOCK_GROUND;
/// A skeleton row's bars.
pub const SKELETON: Color = Color::srgba(0.90, 0.93, 0.94, 0.08);

/// Spacing, before the text step's factor: 4 · 8 · 12 · 16 · 24 · 32 · 48.
pub const SPACE: [f32; 7] = [4.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0];

/// Radii: controls, panels, pills.
pub const RADIUS_CONTROL: f32 = 8.0;
/// See [`RADIUS_CONTROL`].
pub const RADIUS_PANEL: f32 = 14.0;
/// See [`RADIUS_CONTROL`].
pub const RADIUS_PILL: f32 = 999.0;

/// The focus ring: `ACCENT`, 2 px, 2 px off the control (never scaled).
pub const RING_WIDTH: f32 = 2.0;
/// See [`RING_WIDTH`].
pub const RING_OFFSET: f32 = 2.0;

/// The smallest target a finger hits reliably, never scaled down (§2.4).
pub const HIT: f32 = 44.0;

/// `GlobalZIndex` bands (N-3): a popover over the page, a sheet over a
/// popover, a toast over a sheet, the loading veil over everything.
pub mod z {
    /// The page itself.
    pub const PAGE: i32 = 0;
    /// Popovers and menus.
    pub const POPOVER: i32 = 10;
    /// Sheets.
    pub const SHEET: i32 = 20;
    /// Toasts.
    pub const TOAST: i32 = 30;
    /// The loading veil.
    pub const VEIL: i32 = 40;
}

/// How many characters a label may take before its component must give it an
/// icon and a tooltip or move it into a menu (§2.3, C3-28). German runs
/// about 30 % longer, so it gets its own budget where the design gives one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Budget {
    /// English.
    pub en: usize,
    /// German.
    pub de: usize,
}

impl Budget {
    /// The budget for `lang` (an ISO code; anything not German reads as
    /// English).
    #[must_use]
    pub const fn of(self, german: bool) -> usize {
        if german { self.de } else { self.en }
    }
}

/// Buttons: 18, German 20 (with 10 px side padding instead of 12).
pub const BUTTON_BUDGET: Budget = Budget { en: 18, de: 20 };
/// Chips: 14.
pub const CHIP_BUDGET: Budget = Budget { en: 14, de: 14 };
/// Tabs: 16.
pub const TAB_BUDGET: Budget = Budget { en: 16, de: 16 };
/// Navigation pills: 12, German 13 — the approved German mock's own nav says
/// "Einstellungen" (`mocks/v3/src/decks-de.html`), thirteen letters, and
/// there is no shorter word for it that a German player reads at a glance.
pub const NAV_BUDGET: Budget = Budget { en: 12, de: 13 };
