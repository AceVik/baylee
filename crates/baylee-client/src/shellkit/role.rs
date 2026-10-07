//! What a shell node *is*, for the checks that read the dev-control dump.
//!
//! `/state.shell_nodes` reports every node's rect and text; a node carrying a
//! [`Role`] also reports `"k"`, which is what lets a script hold the shell to
//! the design without a renderer: a label inside a `button` against the
//! button budget, a `hit` wrapper against 44 × 44 under Touch, a `panel` or
//! `mist` rect as the ground the contrast check reads pixels under.

use super::tokens::{BUTTON_BUDGET, Budget, CHIP_BUDGET, NAV_BUDGET, TAB_BUDGET};
use bevy::prelude::*;

/// A shell node's kind.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// The header bar.
    Header,
    /// A translucent panel on the painting.
    Panel,
    /// An opaque surface: a sheet, a menu, a popover.
    Opaque,
    /// A mist plate under a line that would stand on the painting.
    Mist,
    /// The 44 × 44 hit wrapper around a control (§2.4, S4-2).
    Hit,
    /// A button's face.
    Button,
    /// A filter chip.
    Chip,
    /// A tab.
    Tab,
    /// A navigation pill in the header.
    Nav,
    /// A header pill (gateway, account).
    Pill,
    /// One option of a segmented control.
    Segment,
    /// A toggle.
    Toggle,
    /// A stepper.
    Stepper,
    /// A slider's track.
    Slider,
    /// A text or search field's box.
    Field,
    /// A menu's item.
    MenuItem,
    /// A list or settings row.
    Row,
    /// A deck tile.
    Tile,
    /// A toast.
    Toast,
    /// A skeleton bar.
    Skeleton,
    /// An error line.
    Error,
    /// A storage tag on a settings row.
    Tag,
    /// A key cap (Pointer only).
    KeyCap,
    /// A count beside a chip's or a tab's label: not part of the label's
    /// budget (§2.3 budgets the words).
    Count,
    /// A scroll container: what it holds may lie past its edges, scrolled
    /// out of view rather than overflowing (the checks read it so).
    Scroll,
}

impl Role {
    /// The name the dump reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Panel => "panel",
            Self::Opaque => "opaque",
            Self::Mist => "mist",
            Self::Hit => "hit",
            Self::Button => "button",
            Self::Chip => "chip",
            Self::Tab => "tab",
            Self::Nav => "nav",
            Self::Pill => "pill",
            Self::Segment => "segment",
            Self::Toggle => "toggle",
            Self::Stepper => "stepper",
            Self::Slider => "slider",
            Self::Field => "field",
            Self::MenuItem => "menu_item",
            Self::Row => "row",
            Self::Tile => "tile",
            Self::Toast => "toast",
            Self::Skeleton => "skeleton",
            Self::Error => "error",
            Self::Tag => "tag",
            Self::KeyCap => "keycap",
            Self::Count => "count",
            Self::Scroll => "scroll",
        }
    }

    /// The label budget a component of this kind holds its text to (§2.3).
    #[must_use]
    pub const fn budget(self) -> Option<Budget> {
        match self {
            Self::Button => Some(BUTTON_BUDGET),
            Self::Chip => Some(CHIP_BUDGET),
            Self::Tab => Some(TAB_BUDGET),
            Self::Nav => Some(NAV_BUDGET),
            _ => None,
        }
    }
}
