//! Decks an AI player keeps for itself (`docs/llm-seat.md` §"Deck
//! building", `docs/proposals/ai-players.md` §"Decks").
//!
//! An AI account at Central has no library its runner may read, so its
//! decks are files on the runner's machine, one directory per AI, every
//! version kept and dated ([`history`]); the runner offers the newest of
//! each format. Every deck is held to the check a ranked host makes when it
//! seats one ([`check`]), against the host's own legality facts
//! ([`facts`]), so a deck that passes here is not refused at the table.
//!
//! - [`facts`]: the host's catalog table and blocked cards, read as a
//!   guest and kept beside the decks;
//! - [`check`]: the host's check, and its faults in words, card by card;
//! - [`pool`]: the cards a format's deck may hold, to search and read;
//! - [`brief`], [`talk`], [`work`]: a language model building or changing a
//!   deck, through the bridge's minds (an API, an agent CLI, a profile's
//!   fallbacks), told the check's faults until a deck passes;
//! - [`lend`]: a deck without a model, for the pilots that cannot build;
//! - [`exclude`]: cards a pilot's decks may not hold, from a file;
//! - [`file`], [`history`]: the deck file and the chronicle of versions.

pub mod brief;
pub mod check;
pub mod exclude;
pub mod facts;
pub mod file;
pub mod history;
pub mod lend;
pub mod pool;
pub mod talk;
#[cfg(test)]
mod tests;
pub mod work;
