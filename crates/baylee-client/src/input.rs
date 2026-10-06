//! Input: keyboard first, pointer second.
//!
//! `docs/keyboard-map.md` makes a commitment — every choice the game can ask is
//! answerable without a pointer, and nothing requires drag-and-drop. That is
//! not only an accessibility promise; a competitive player passing priority
//! forty times a turn will not reach for a mouse each time.
//!
//! Both paths converge on the same place: they build a [`PlayerAction`] through
//! [`baylee_client_core::interaction::Interaction`], which refuses anything the
//! engine did not offer. No input handler decides legality by itself.
//!
//! Which key does what is the account's, not this module's: every binding
//! comes from `Keymap` and is resolved through `crate::keys`. The primary key
//! (`Enter` by default) is "the click", with a fixed precedence: the card
//! under the cursor, then the selected phase button, then confirm/pass —
//! which is why it is not the pass key: `Space` is `Confirm` and passes
//! whatever the cursor happens to be resting on.

use crate::hud::{
    AbilityButton, ChoiceButton, HandCardVisual, MenuAction, MenuButton, PlayerTab, PreviewResize,
    PromptAction, PromptButton, TrayCard, TrayFilter, TrayMinimise, TrayNone, TraySort, TrayTab,
};
use crate::keys::Fired;
use crate::settings::ClientSettings;
use crate::table::CardVisual;
use crate::{Deed, Duel, HoverSpot};
use baylee_client_core::abilitysheet;
use baylee_client_core::arrange::{Arrangement, Nudge};
use baylee_client_core::automation::AutoPilot;
use baylee_client_core::browser::Placement;
use baylee_client_core::filterdialog::FilterPanel;
use baylee_client_core::i18n::{Phrase, Refusal};
use baylee_client_core::interaction::{Interaction, Prompt, SelectionOutcome};
use baylee_client_core::prefs::Action;
use baylee_client_core::touch::Answer;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::PlayerAction;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

mod activation;
mod answering;
mod browser_input;
mod clicks;
mod keyboard_input;
mod picking;
mod pointing;
mod sheet_keys;
mod sheets;

pub use activation::*;
pub(crate) use answering::*;
pub use browser_input::*;
pub use clicks::*;
pub use keyboard_input::*;
pub use picking::*;
pub use pointing::*;
pub use sheet_keys::*;
pub use sheets::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod refusal_tests;

/// The zone browser's sheet actually moves when its header is dragged.
///
/// A drag cannot be proved through the `dev-control` harness — `/pointer`
/// presses and releases in one call, so there is no frame in the middle where
/// the cursor is somewhere else — and "declared but never wired" is a bug this
/// client has shipped before. So it is proved here instead: the system is put
/// in an `App` with a window, a sheet and the two markers, and what is
/// asserted is the sheet's own `Node`, not that `update()` returned.
#[cfg(test)]
mod dragging;

#[cfg(test)]
mod library_land_tests;
