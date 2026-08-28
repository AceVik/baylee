//! baylee-cards-codegen — deterministic code generation.
//!
//! Inputs: Scryfall (subtype catalogs, per-card data — cached and committed),
//! the acceptance deck list (`data/acceptance-decks.txt`), and the local
//! forge-reference checkout (read-only). Outputs: subtype constants, per-card
//! stub files, the card registry, and the forge index.

#![warn(missing_docs)]

pub mod acceptance;
pub mod catalog;
pub mod error;
pub mod forge;
pub mod scryfall;
pub mod stubgen;

pub use error::CodegenError;
