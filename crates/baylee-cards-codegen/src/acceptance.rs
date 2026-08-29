//! Parser for `data/acceptance-decks.txt` — moved to `baylee-core` so
//! runtime crates (baylee-ai) can load the decks without depending on
//! codegen tooling. This module re-exports for compatibility.

pub use baylee_core::acceptance::{DeckRow, Zone, parse_decks, unique_names};
