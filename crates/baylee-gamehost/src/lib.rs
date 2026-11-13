//! baylee-gamehost — shared game hosting for `baylee-engine-server`
//! (dev harness) and `baylee-gateway` (accounts/lobby).
//!
//! Contains: [`session`] (engine + AI seats with per-seat envelope
//! routing), [`view`] (per-seat hidden-information views), [`preset`]
//! (wire → core preset conversion).

#![warn(missing_docs)]

// The tests look cards up, so their binary holds the pool.
#[cfg(test)]
use baylee_cards_pool as _;

pub mod harness;
mod log;
pub mod preset;
pub mod record;
mod scouting;
pub mod session;
pub mod view;

pub use baylee_view::{GameStatic, PlayerView, SeatIdentity};
/// A session's game forked for one seat's lookahead (`Session::fork_for`), for
/// a harness that drives the copy itself: only under the `simulation`
/// feature, which no hosting build enables.
#[cfg(any(test, feature = "simulation"))]
pub use session::SimulationSession;
pub use session::project as replay;
/// What a host lets the house's searches spend (`Session::set_foresight_host`)
/// and what a table's searches have cost (`Session::foresight_stats`): under
/// the `foresight` feature, which the hosting builds enable.
#[cfg(any(test, feature = "foresight"))]
pub use session::{
    ForesightStats, HostForesight, SearchCost, SearchPermit, SearchPermits, Searches, WallGuard,
};
/// What a probed search saw (`Session::take_foresight_probe`) and a searched
/// question kept for study: the harness's, under `simulation`.
#[cfg(any(test, feature = "simulation"))]
pub use session::{ProbedLeaf, SEARCH_SHARE, SearchedQuestion};
pub use session::{RegistryLookup, SeatKind, Session};
pub use view::{SeatContext, game_static, owed_payment, player_view};
