//! This crate's integration tests, as one test program.
//!
//! Each test file beside this one is a module of it: one program per file
//! linked the engine and every card again for each file, at every change to
//! either. nextest, the gates' runner, still starts every test in a process of
//! its own. A file's tests are `--test integration <file>::`.

#![allow(clippy::missing_docs_in_private_items)]

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

mod ai_activation_payment;
mod ai_coverage_guards;
mod ai_decisions;
mod ai_decks;
mod ai_exact_x;
mod ai_madness;
mod ai_mindslaver;
mod ai_miracle;
mod ai_upkeep_payments;
mod ai_ward;
mod fair_house;
mod question_note;
mod reference_profile;
mod stall_guard;
mod team_play;
mod view_patches;
