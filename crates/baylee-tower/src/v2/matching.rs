//! Sampling the whole indexed eligible population, never its first bounded page.
//! The store maintains dense slots per cohort/ordinal, one per account, and holds
//! that partition's revision/lock while resolving exclusions and reading a choice.
use super::MAX_STAGES;
use crate::Refusal;
use sha2::{Digest, Sha256};

/// Database snapshot of one compatible, currently eligible matching partition.
pub struct Population<'a> {
    /// Dense slots are exactly 0..count; at most PostgreSQL's signed bigint range.
    pub count: u64,
    /// Changes on insertion, withdrawal, expiry or representative replacement.
    pub generation: u64,
    /// This account's slot, if present; self-play is always excluded.
    pub own: Option<u64>,
    /// Current slots of previously encountered accounts; duplicates are harmless.
    /// Accounts no longer in the pool have no slot and must be omitted.
    pub previous: &'a [u64],
}

/// One indexed lookup under the same partition generation. Eligibility must still
/// be checked in that transaction before freezing the private package.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    /// Snapshot generation, not a reusable position after pool mutations.
    pub generation: u64,
    /// Exactly one dense eligible slot to read, independent of population size.
    pub slot: u64,
    /// True only when every other eligible account has already been encountered.
    pub repeated: bool,
}

/// Select uniformly among unseen eligible accounts, otherwise among non-self
/// accounts. Uniformity assumes an independently allocated uniform private seed;
/// this sampling policy is not a credential, operator trust or Sybil defense.
/// # Errors
/// Invalid database metadata/exclusions or exhaustion of bounded rejection sampling.
pub fn select(population: &Population<'_>, seed: &[u8; 32]) -> Result<Option<Selection>, Refusal> {
    if population.count > i64::MAX as u64
        || population.previous.len() > usize::from(MAX_STAGES)
        || population
            .own
            .into_iter()
            .chain(population.previous.iter().copied())
            .any(|slot| slot >= population.count)
    {
        return Err(Refusal::Invalid);
    }
    let mut excluded: Vec<_> = population
        .own
        .into_iter()
        .chain(population.previous.iter().copied())
        .collect();
    excluded.sort_unstable();
    excluded.dedup();
    let fresh = population.count - excluded.len() as u64;
    let repeated = fresh == 0;
    if repeated {
        excluded.clear();
        excluded.extend(population.own);
    }
    let available = population.count - excluded.len() as u64;
    if available == 0 {
        return Ok(None);
    }
    let rank = rank(seed, population.generation, available)?;
    Ok(Some(Selection {
        generation: population.generation,
        slot: expand(rank, &excluded),
        repeated,
    }))
}

/// Map a rank in the complement to its dense pool slot, in at most thirteen steps.
fn expand(mut rank: u64, excluded: &[u64]) -> u64 {
    for slot in excluded {
        if *slot > rank {
            break;
        }
        rank += 1;
    }
    rank
}

fn rank(seed: &[u8; 32], generation: u64, available: u64) -> Result<u64, Refusal> {
    // Discard the unequal low tail before modulo. Even the least favorable valid
    // range accepts at least half the words; still cap work rather than loop forever.
    let threshold = available.wrapping_neg() % available;
    for attempt in 0_u8..128 {
        let hash = Sha256::new()
            .chain_update(b"baylee/tower/eligible-slot/v2\0")
            .chain_update(seed)
            .chain_update(generation.to_le_bytes())
            .chain_update(available.to_le_bytes())
            .chain_update([attempt])
            .finalize();
        let value = u64::from_le_bytes(
            hash[..8]
                .try_into()
                .expect("SHA-256 has eight leading bytes"),
        );
        if value >= threshold {
            return Ok(value % available);
        }
    }
    Err(Refusal::Unavailable)
}

/// Constant-work dense-slot removal, executed under the partition lock. Move the
/// final row into a removed hole, then decrement count and increase generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Removal {
    /// Slot whose account is being withdrawn/expired.
    pub removed: u64,
    /// Last slot to move into the hole, or none when the removed slot was last.
    pub move_from: Option<u64>,
    /// New partition population after the atomic delete/move.
    pub remaining: u64,
}

/// Plan a bounded indexed deletion without changing other accounts' matching weight.
/// # Errors
/// Empty/oversized population or a slot outside it.
pub fn removal(count: u64, slot: u64) -> Result<Removal, Refusal> {
    if count > i64::MAX as u64 || slot >= count {
        return Err(Refusal::Invalid);
    }
    let last = count - 1;
    Ok(Removal {
        removed: slot,
        move_from: (slot != last).then_some(last),
        remaining: last,
    })
}

#[cfg(test)]
mod tests;
