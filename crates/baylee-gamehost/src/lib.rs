//! baylee-gamehost — shared game hosting for `baylee-engine-server`
//! (dev harness) and `baylee-gateway` (accounts/lobby).
//!
//! Contains: [`session`] (engine + AI seats with per-seat envelope
//! routing), [`view`] (per-seat hidden-information views), [`preset`]
//! (wire → core preset conversion).

#![warn(missing_docs)]

pub mod harness;
pub mod preset;
mod scouting;
pub mod session;
pub mod view;

pub use baylee_view::{GameStatic, PlayerView, SeatIdentity};
pub use session::{RegistryLookup, SeatKind, Session};
pub use view::{SeatContext, game_static, owed_payment, player_view};
