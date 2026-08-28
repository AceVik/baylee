//! baylee-ai — heuristic AI controllers with difficulty profiles (M3).
//!
//! AI seats consume the exact same engine contract as humans
//! (`view` + `pending` → `PlayerAction`); difficulty is parameterized
//! through [`AIProfile`], never duplicated logic.

#![warn(missing_docs)]

pub use baylee_core::preset::AIProfile;
