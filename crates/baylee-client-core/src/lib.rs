//! baylee-client-core — everything a duel client decides, with no renderer,
//! no transport, and no I/O.
//!
//! # Why the brain is a separate crate
//!
//! The platform plans two very different applications on top of the same duel:
//! the standalone Bevy table client, and later an open world in which a duel
//! happens inside a larger game. Those share every interesting decision — how
//! seats are arranged, when identical permanents may be drawn as one card,
//! which answers a pending choice will accept, how much texture memory a board
//! is allowed — and share none of the plumbing.
//!
//! Keeping the decisions here has two payoffs. Reuse is the obvious one. The
//! bigger one is that all of it is testable as arithmetic: a headless test can
//! assert that a tapped token never hides inside an untapped stack, or that an
//! eight-seat board fits a phone's texture budget, without starting a window.
//!
//! # Layers
//!
//! ```text
//!   baylee-view        wire types (no rules kernel)
//!        ^
//!   baylee-client-core layout · board model · interaction · image policy
//!        ^
//!   baylee-client      Bevy plugin: meshes, UI, input, asset loading
//! ```
//!
//! Nothing in this crate knows what a frame is.

#![warn(missing_docs)]
// Layout and threat arithmetic converts small counts (seats, cards in a lane,
// permanents on a board) into floats. Every one of them is bounded by what
// fits on a table, orders of magnitude below the point where an f32 mantissa
// loses a whole number, so the conversions are exact in practice.
#![allow(clippy::cast_precision_loss)]

pub mod automation;
pub mod board;
pub mod card_face;
pub mod deckbuilder;
pub mod images;
pub mod interaction;
pub mod layout;
pub mod lobby;
pub mod manapip;
pub mod prefs;
pub mod tabletop;

#[cfg(test)]
pub(crate) mod test_support;

pub use automation::{AutoAnswer, AutoPilot, PhaseOrders};
pub use board::{BoardModel, CardGroup, Lane, SeatPod, StackItem, ThreatSummary, TokenChip};
pub use card_face::{
    CardFace, CardText, CardTextEntry, Characteristics, FaceText, Stats, TextBlock,
};
pub use deckbuilder::{
    BuildField, Counts, Coverage, DeckBuilder, Entry, Group, PoolCard, Problem, Sort, Zone,
};
pub use images::{ArtSize, ImageKey, ImageRequest, TextureBudget};
pub use interaction::{CombatFocus, Interaction, Prompt, SelectionOutcome};
pub use layout::{LaneKind, SeatSlot, TableLayout};
pub use lobby::{
    DeckSummary, Field, FieldKind, GameMode, GameSeat, GameSummary, Lobby, LobbyEvent,
    LobbyRequest, Screen, SeatHandover,
};
pub use prefs::{Action, AutoRules, Chord, Keymap, Preferences};

/// Re-exported wire types, so a downstream crate needs one dependency to talk
/// to a host and render the result.
pub use baylee_view as view;
