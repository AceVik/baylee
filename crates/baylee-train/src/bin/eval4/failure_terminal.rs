//! Eval-only audit CLI and fixtures around the shared native research boundary.
use anyhow::{Context as _, ensure};
#[cfg(test)]
use baylee_core::ids::ObjectId;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
#[cfg(test)]
use baylee_engine::choice::PlayerAction;
#[cfg(test)]
use baylee_gamehost::Session;
use baylee_gamehost::record::{Line, Source};
pub(super) use baylee_train::failure_terminal::{
    boundary_view_hash, contract, natural, replay, sample_binding, stop, validate_encoder,
};
use baylee_train::{convert4::RolloutStepSidecar4, features4::SourceKind4};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) mod audit;
#[cfg(test)]
mod tests;
