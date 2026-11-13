//! Immutable execution material. This crate runs no game rules and may be used by a gateway.
#![forbid(unsafe_code)]

pub mod stock;

use baylee_core::preset::{GamePreset, SeatController};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Maximum encoded package, matching Central's admission boundary.
pub const MAX_PACKAGE_BYTES: usize = central_api::challenges::MAX_PACKAGE_BYTES;

/// Build actually executing the task; never a version string supplied by a player.
#[must_use]
pub const fn executor() -> &'static str {
    baylee_build::short()
}

/// Private material retained by Central and delivered only to its official executor.
/// Deliberately has no `Debug`: opening hands, draw order and seeds are not log data.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    /// Supported package schema.
    pub schema: u8,
    /// Exact tested engine build; other builds cannot reinterpret an archived task.
    pub executor: String,
    /// Entire frozen two-seat setup, including the independent opponent policy seed.
    pub preset: GamePreset,
    /// Winning on or before this game turn earns the tactician distinction.
    pub target_turn: u32,
}

/// A malformed, incompatible or unsafe execution package.
#[derive(Debug, thiserror::Error)]
#[error("invalid or incompatible challenge package")]
pub struct InvalidPackage;

impl Package {
    /// Check Central's immutable publication and its corresponding execution package.
    ///
    /// # Errors
    /// Invalid content digest, incompatible executor or inconsistent turn target.
    pub fn from_publication(
        publication: &central_api::challenges::Publication,
    ) -> Result<Self, InvalidPackage> {
        publication.validate().map_err(|_| InvalidPackage)?;
        let package = Self::decode(&publication.package, executor())?;
        if publication.description.executor != package.executor
            || publication.description.target_turn != package.target_turn
        {
            return Err(InvalidPackage);
        }
        Ok(package)
    }
    /// Decode only bounded, supported execution material for this executor.
    ///
    /// # Errors
    /// Oversized, malformed, unsupported or unsafe material.
    pub fn decode(bytes: &[u8], executor: &str) -> Result<Self, InvalidPackage> {
        if bytes.is_empty() || bytes.len() > MAX_PACKAGE_BYTES {
            return Err(InvalidPackage);
        }
        let package: Self = serde_json::from_slice(bytes).map_err(|_| InvalidPackage)?;
        package.validate(executor)?;
        Ok(package)
    }

    /// Check structure and the frozen execution contract without running rules.
    /// The engine separately validates every card and every answer.
    ///
    /// # Errors
    /// Incompatible schema/build, malformed preset or unsupported execution mode.
    pub fn validate(&self, executor: &str) -> Result<(), InvalidPackage> {
        let preset = &self.preset;
        let rules = &preset.house_rules;
        if self.schema != 1
            || self.executor != executor
            || self.executor.is_empty()
            || self.executor.len() > 160
            || !(1..=500).contains(&self.target_turn)
            || preset.validate().is_err()
            || preset.seats.len() != 2
            || !matches!(preset.seats[0].controller, SeatController::Open)
            || !matches!(preset.seats[1].controller, SeatController::Ai(profile) if profile.foresight().is_none())
            || preset.seats.iter().any(|seat| seat.team.is_some())
            || rules.challenge_policy_seed.is_none()
            || rules.decision_timeout_secs != 0
            || rules.reconnect_window_secs != 0
            || rules.takebacks
            || rules.time_extension_votes
        {
            return Err(InvalidPackage);
        }
        Ok(())
    }

    /// Encode after validating against its declared build.
    ///
    /// # Errors
    /// Invalid or oversized execution material.
    pub fn encode(&self) -> Result<Vec<u8>, InvalidPackage> {
        self.validate(&self.executor)?;
        let bytes = serde_json::to_vec(self).map_err(|_| InvalidPackage)?;
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err(InvalidPackage);
        }
        Ok(bytes)
    }
}

/// Domain-separated seed: calendar seeds must not be reused as policy streams.
#[must_use]
pub fn seed(domain: &[u8], seed: u64) -> u64 {
    let digest = Sha256::new()
        .chain_update(b"baylee/challenge/seed/v1\0")
        .chain_update((domain.len() as u64).to_le_bytes())
        .chain_update(domain)
        .chain_update(seed.to_le_bytes())
        .finalize();
    u64::from_le_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}
