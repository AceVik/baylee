//! baylee-gamehost — shared game hosting for `baylee-engine-server`
//! (dev harness) and `baylee-gateway` (accounts/lobby).
//!
//! Contains: [`session`] (engine + AI seats with per-seat envelope
//! routing), [`view`] (per-seat hidden-information views), [`preset`]
//! (wire → core preset conversion).

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod preset;
pub mod session;
pub mod view;

pub use session::{RegistryLookup, SeatKind, Session};
pub use view::{PlayerView, player_view};
