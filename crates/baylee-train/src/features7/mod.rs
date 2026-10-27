//! Encoder 7 (`ENCODER_VERSION7 = 7`) state, inventory, grouping, and action encoder.

// Counts and coordinates are deliberately normalized into the f32 model ABI.
#![allow(clippy::cast_precision_loss)]

pub mod actions;
pub mod grouping;
pub mod inventory;
pub mod state;

pub use actions::{CandidateLimit7, encode_candidates7, encode_candidates7_grouped};
pub use grouping::{PermanentGroup7, group_battlefield_permanents};
pub use inventory::{
    DeckSection7, InventoryLedger7, PrintingFacts7, RegisteredCard7, RegisteredDeck7,
    RemainingCardGroup7,
};
pub use state::{
    CAND_FEAT_WIDTH7, ENCODER_VERSION7, ENTITY_FEAT_WIDTH7, GLOBAL_FEAT_WIDTH7, SEAT_FEAT_WIDTH7,
    encode_state7,
};
