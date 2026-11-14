//! Fenced execution metadata. A store must commit each chunk and this head together
//! before acknowledging it; these pure functions do not provide durability or auth.
use crate::Refusal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Maximum bytes in one ordered record append, outside the rules hot path.
pub const MAX_CHUNK_BYTES: usize = 256 * 1024;
/// Hard storage ceiling; each reservation freezes an operator limit at or below it.
/// Reaching it is a technical interruption, never a rules outcome.
pub const MAX_RECORD_BYTES: u64 = 512 * 1024 * 1024;
/// A renewable worker lease, independent of player decision or duel duration.
pub const MAX_LEASE_MS: u64 = 120_000;

/// Head of the authenticated, durably retained ordered chunk stream.
/// The digest is a content commitment, not a credential or proof of legal play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prefix {
    /// Number of accepted chunks, not engine decisions.
    pub chunks: u64,
    /// Total accepted record bytes.
    pub bytes: u64,
    /// SHA-256 commitment to package, preceding head, index, length and bytes.
    pub digest: [u8; 32],
}

impl Prefix {
    pub(super) fn validate(self, limit: u64) -> Result<(), Refusal> {
        if self.bytes > limit
            || self.chunks > self.bytes
            || self.bytes > self.chunks.saturating_mul(MAX_CHUNK_BYTES as u64)
            || (self.chunks == 0 && self != Self::default())
        {
            return Err(Refusal::Invalid);
        }
        Ok(())
    }
}

/// Stored beside a unique run/duel reservation and its immutable private package.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    /// Frozen package content hash; never replace it during recovery.
    pub package: [u8; 32],
    /// Increasing worker generation; old workers cannot renew, append or seal.
    pub fence: u64,
    /// Authority-clock Unix milliseconds. Zero means no live execution lease.
    pub lease_until: u64,
    /// Frozen technical storage budget, not a turn or game rules limit.
    pub byte_limit: u64,
    /// Recovery may never roll this back after its acknowledgement.
    pub prefix: Prefix,
    /// An immutable terminal candidate awaiting independent legal verification.
    pub sealed: bool,
}

impl Execution {
    /// Prepare one immutable reservation. Only the authority supplies these values.
    /// # Errors
    /// Invalid storage budget.
    pub fn new(package: [u8; 32], byte_limit: u64) -> Result<Self, Refusal> {
        let result = Self {
            package,
            fence: 0,
            lease_until: 0,
            byte_limit,
            prefix: Prefix::default(),
            sealed: false,
        };
        result.validate()?;
        Ok(result)
    }

    /// Check persisted metadata before a transition.
    /// # Errors
    /// Invalid bounds, lease/seal combination or impossible prefix.
    pub fn validate(&self) -> Result<(), Refusal> {
        if self.byte_limit == 0
            || self.byte_limit > MAX_RECORD_BYTES
            || (self.fence == 0
                && (self.lease_until != 0 || self.prefix != Prefix::default() || self.sealed))
            || (self.sealed && (self.prefix.chunks == 0 || self.lease_until != 0))
        {
            return Err(Refusal::Invalid);
        }
        self.prefix.validate(self.byte_limit)
    }

    /// Claim an unleased/expired reservation while preserving every acknowledged byte.
    /// The store also binds the authenticated operator to the returned generation.
    /// # Errors
    /// Live competing lease, sealed record, invalid duration or generation exhaustion.
    pub fn claimed(&self, now: u64, duration: u64) -> Result<Self, Refusal> {
        self.validate()?;
        if self.sealed || self.lease_until > now {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        next.fence = self.fence.checked_add(1).ok_or(Refusal::Capacity)?;
        next.lease_until = deadline(now, duration)?;
        Ok(next)
    }

    fn worker(&self, fence: u64, now: u64) -> Result<(), Refusal> {
        self.validate()?;
        if fence != self.fence || fence == 0 || now >= self.lease_until || self.sealed {
            return Err(Refusal::Stale);
        }
        Ok(())
    }

    /// Renew only a still-live authorized generation.
    /// # Errors
    /// Stale/expired generation or invalid lease duration.
    pub fn renewed(&self, fence: u64, now: u64, duration: u64) -> Result<Self, Refusal> {
        self.worker(fence, now)?;
        let mut next = self.clone();
        next.lease_until = self.lease_until.max(deadline(now, duration)?);
        Ok(next)
    }

    /// Compute an append's next head. A successful return is NOT a durable ACK:
    /// the store must persist the exact chunk and this head in one transaction first.
    /// # Errors
    /// Stale worker/head, empty/oversized chunk or exhausted technical record budget.
    pub fn appended(
        &self,
        fence: u64,
        now: u64,
        expected: Prefix,
        chunk: &[u8],
    ) -> Result<Self, Refusal> {
        self.worker(fence, now)?;
        if expected != self.prefix {
            return Err(Refusal::Stale);
        }
        if chunk.is_empty() || chunk.len() > MAX_CHUNK_BYTES {
            return Err(Refusal::Capacity);
        }
        let length = chunk.len() as u64;
        let bytes = self
            .prefix
            .bytes
            .checked_add(length)
            .filter(|bytes| *bytes <= self.byte_limit)
            .ok_or(Refusal::Capacity)?;
        let chunks = self.prefix.chunks.checked_add(1).ok_or(Refusal::Capacity)?;
        let digest = Sha256::new()
            .chain_update(b"baylee/tower/record-prefix/v2\0")
            .chain_update(self.package)
            .chain_update(self.prefix.digest)
            .chain_update(chunks.to_le_bytes())
            .chain_update(length.to_le_bytes())
            .chain_update(chunk)
            .finalize()
            .into();
        let mut next = self.clone();
        next.prefix = Prefix {
            chunks,
            bytes,
            digest,
        };
        Ok(next)
    }

    /// Freeze exactly the durable head offered for terminal verification.
    /// # Errors
    /// Stale worker/head or a missing record.
    pub fn sealed(&self, fence: u64, now: u64, expected: Prefix) -> Result<Self, Refusal> {
        self.worker(fence, now)?;
        if self.prefix != expected || expected.chunks == 0 {
            return Err(Refusal::Stale);
        }
        let mut next = self.clone();
        next.sealed = true;
        next.lease_until = 0;
        Ok(next)
    }

    /// Fence out execution after a trusted technical failure. Retains the package,
    /// prefix and seal; no outcome, refund, income or fresh opponent is created.
    /// # Errors
    /// Invalid state or exhausted generation.
    pub fn revoked(&self) -> Result<Self, Refusal> {
        self.validate()?;
        let mut next = self.clone();
        next.fence = next.fence.checked_add(1).ok_or(Refusal::Capacity)?;
        next.lease_until = 0;
        Ok(next)
    }
}

fn deadline(now: u64, duration: u64) -> Result<u64, Refusal> {
    if !(1..=MAX_LEASE_MS).contains(&duration) {
        return Err(Refusal::Invalid);
    }
    now.checked_add(duration).ok_or(Refusal::Capacity)
}

#[cfg(test)]
mod tests;
