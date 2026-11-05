//! baylee-train — the Rust half of the trained AI.
//!
//! The trained AI learns only from games whose every card works, so this
//! crate starts with that rule ([`working`]), then plays and records the games
//! it learns from ([`selfplay`]) with the house decks ([`housedeck`]). The
//! records are the gamehost's own (`baylee_gamehost::record`): raw inputs,
//! replayable on a fresh engine, so a change to what the trainer reads from a
//! game never means playing the games again.
//!
//! Nothing that ships links this crate; `xtask` does, so `deck-check` and the
//! precon status (`decks-status`) ask the same question the trainer does.

#[cfg(feature = "onnx")]
pub mod batchnet;
pub use baylee_cards::carddsl;
#[cfg(feature = "play")]
pub mod cardstructure;
pub mod cardwalk;
#[cfg(feature = "play")]
pub mod convert;
#[cfg(feature = "play")]
pub mod convert3;
#[cfg(feature = "play")]
pub mod convert4;
pub mod deckgen;
#[cfg(feature = "play")]
pub mod failure_terminal;
#[cfg(feature = "play")]
pub mod features;
#[cfg(feature = "play")]
pub mod features3;
#[cfg(feature = "play")]
pub mod features4;
#[cfg(feature = "play")]
pub mod features7;
#[cfg(feature = "play")]
pub mod features8;
#[cfg(feature = "play")]
pub mod features9;
pub mod housedeck;
#[cfg(feature = "live-seat")]
pub mod live4;
#[cfg(feature = "llm")]
pub mod llmchair;
#[cfg(feature = "onnx")]
pub mod netplay;
#[cfg(feature = "onnx")]
pub mod netplay3;
#[cfg(feature = "onnx")]
pub mod netplay4;
#[cfg(feature = "onnx")]
pub use netplay4::{BAYLEE_KI_NAME, BayleeKI};
#[cfg(feature = "onnx")]
pub mod onnx;
#[cfg(feature = "play")]
pub mod policy;
#[cfg(feature = "play")]
pub mod selfplay;
pub mod working;

#[cfg(all(test, feature = "play"))]
mod damage_fixture;
#[cfg(all(test, feature = "play"))]
mod encoder_contract;
