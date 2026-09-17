//! baylee-cards-codegen — deterministic code generation.
//!
//! Inputs: Scryfall (subtype catalogs, per-card data — cached and committed),
//! the acceptance deck list (`data/acceptance-decks.txt`), and the local
//! card-script reference checkout (read-only). Outputs: subtype constants, per-card
//! stub files, the card registry, and the script index.

#![warn(missing_docs)]

pub mod acceptance;
pub mod body;
pub mod cardindex;
pub mod catalog;
pub mod error;
pub mod landgen;
pub mod layout;
pub mod ledger;
pub mod lines;
pub mod names;
pub mod scriptgen;
pub mod scripts;
pub mod scryfall;
pub mod stubgen;
pub mod tokengen;

pub use error::CodegenError;
