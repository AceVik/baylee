//! Same-stage player echoes. Selection is frozen by the service's reservation transaction.
use crate::{MAX_STAGES, Refusal};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Query work is bounded before selecting a candidate. Persistence pages the global pool.
pub const MAX_CANDIDATES: usize = 512;

/// An immutable compatibility cohort. Never silently compare different engine or balance rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cohort {
    /// Economy and item rules version.
    pub rules: u32,
    /// Exact retained executor build, not a player-selected binary path.
    pub executor: String,
    /// Digest of the immutable admitted card/item catalog and starting rules.
    pub catalog: [u8; 32],
}

impl Cohort {
    /// Check syntax; the service separately requires a published compatible catalog.
    /// # Errors
    /// Invalid version, unbounded/unprintable build identity or missing digest.
    pub fn validate(&self) -> Result<(), Refusal> {
        if self.rules == 0
            || self.executor.is_empty()
            || self.executor.len() > 160
            || !self.executor.bytes().all(|b| b.is_ascii_graphic())
            || self.catalog == [0; 32]
        {
            return Err(Refusal::Invalid);
        }
        Ok(())
    }
}

/// Private pool metadata. A public opponent projection deliberately omits this type.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    /// Database-allocated immutable snapshot `UUIDv7`.
    pub snapshot: [u8; 16],
    /// Central account `UUIDv7`, used for self/repeat exclusion; never a credential.
    pub account: [u8; 16],
    /// Starting at one; the same number of prior shop/income opportunities.
    pub stage: u8,
    /// Frozen matching cohort.
    pub cohort: Cohort,
    /// False after withdrawal/deletion; checked again under the reservation lock.
    pub eligible: bool,
}

/// All state needed for a selection, without a dependency on a gateway or calendar day.
pub struct Criteria<'a> {
    /// Account starting the duel.
    pub account: [u8; 16],
    /// Run's next duel ordinal.
    pub stage: u8,
    /// Run's frozen rules and executor cohort.
    pub cohort: &'a Cohort,
    /// Previously faced accounts, at most the run's bounded duel count.
    pub previous: &'a [[u8; 16]],
}

fn uuid7(value: &[u8; 16]) -> bool {
    value[6] >> 4 == 7 && value[8] >> 6 == 2
}

/// Choose a compatible echo, preferring someone not yet faced. `None` means an
/// explicitly labelled house fallback is required, never a fabricated player.
///
/// Persistence supplies at most one representative snapshot per account and
/// cohort/stage, so prolific contributors cannot multiply their matching weight.
/// The private random seed is generated once and retained with the reservation.
/// This pure operation is order-independent; it never rerolls on reconnect.
/// # Errors
/// Invalid target, oversized pool/history or repeated compatible account entries.
pub fn select<'a>(
    criteria: &Criteria<'_>,
    pool: &'a [Candidate],
    seed: &[u8; 32],
) -> Result<Option<&'a Candidate>, Refusal> {
    criteria.cohort.validate()?;
    if !uuid7(&criteria.account)
        || !(1..=MAX_STAGES).contains(&criteria.stage)
        || criteria.previous.len() > usize::from(MAX_STAGES)
        || pool.len() > MAX_CANDIDATES
        || criteria.previous.iter().any(|id| !uuid7(id))
    {
        return Err(Refusal::Invalid);
    }
    let compatible = |candidate: &&Candidate| {
        candidate.eligible
            && candidate.account != criteria.account
            && candidate.stage == criteria.stage
            && &candidate.cohort == criteria.cohort
    };
    let mut owners = BTreeSet::new();
    for candidate in pool.iter().filter(compatible) {
        if !uuid7(&candidate.account)
            || !uuid7(&candidate.snapshot)
            || !owners.insert(candidate.account)
        {
            return Err(Refusal::Invalid);
        }
    }
    let fresh = pool
        .iter()
        .filter(compatible)
        .any(|candidate| !criteria.previous.contains(&candidate.account));
    Ok(pool
        .iter()
        .filter(compatible)
        .filter(|candidate| !fresh || !criteria.previous.contains(&candidate.account))
        .min_by_key(|candidate| {
            let digest: [u8; 32] = Sha256::new()
                .chain_update(b"baylee/tower/echo-selection/v1\0")
                .chain_update(seed)
                .chain_update(candidate.account)
                .finalize()
                .into();
            (digest, candidate.account)
        }))
}

#[cfg(test)]
mod tests;
