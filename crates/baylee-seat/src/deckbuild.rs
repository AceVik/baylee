//! Decks an AI player keeps for itself (`docs/llm-seat.md` §"Deck
//! building", `docs/proposals/ai-players.md` §"Decks").
//!
//! An AI account at Central has no library its runner may read, so its
//! decks are files on the runner's machine, one directory per AI, every
//! version kept and dated ([`history`]); the runner offers the newest of
//! each format. A deck is checked against the host's legality facts
//! ([`facts`]), which are what a ranked host checks it against when it is
//! seated.
//!
//! - [`facts`]: the host's catalog table and blocked cards, fetched as a
//!   guest and kept beside the decks;
//! - [`history`]: the chronicle of versions.

pub mod facts;
pub mod history;
