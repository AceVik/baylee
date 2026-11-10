//! This crate's integration tests, as one test program.
//!
//! Each test file beside this one is a module of it: one program per file
//! linked the engine and every card again for each file, at every change to
//! either. nextest, the gates' runner, still starts every test in a process of
//! its own. A file's tests are `--test integration <file>::`.

#![allow(clippy::missing_docs_in_private_items)]

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

mod common;

mod cli_mind;
mod closed_room;
mod reconnect_window;
mod refused_chair;
mod seated_house;
mod selfplay;
mod stopped;
mod tethered;
