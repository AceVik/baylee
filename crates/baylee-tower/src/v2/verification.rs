//! Bounded private verifier IPC, independent of engine and database implementations.
use super::{
    execution::{Execution, MAX_RECORD_BYTES},
    package::Package,
    run::{Binding, Settlement},
};
use crate::{Refusal, cards::Catalog};
use serde::{Deserialize, Serialize};

/// Maximum private request header before decoding; records stream separately.
pub const MAX_REQUEST_BYTES: usize = 256 * 1024;
/// Maximum result frame; no hidden card material is returned.
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024;

/// Technical resource allocation from the trusted process owner, never a player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    /// Raw record budget, at most 512 MiB.
    pub bytes: u64,
    /// Maximum JSON lines, at most two million.
    pub lines: u32,
    /// Maximum bytes per JSON line, at most one MiB.
    pub line_bytes: u32,
    /// Maximum engine journal entries, at most eight million.
    pub events: u32,
    /// Cooperative deadline, at most 55 seconds; the parent also kills stuck work.
    pub millis: u32,
}

impl Default for Resources {
    fn default() -> Self {
        Self {
            bytes: 64 * 1024 * 1024,
            lines: 250_000,
            line_bytes: 1024 * 1024,
            events: 2_000_000,
            millis: 55_000,
        }
    }
}

impl Resources {
    /// Refuse unbounded or nonsensical allocations before starting rules work.
    /// # Errors
    /// A technical allocation exceeds the native worker's hard ceilings.
    pub fn validate(self) -> Result<(), Refusal> {
        if !(1..=MAX_RECORD_BYTES).contains(&self.bytes)
            || !(2..=2_000_000).contains(&self.lines)
            || !(1..=1024 * 1024).contains(&self.line_bytes)
            || !(1..=8_000_000).contains(&self.events)
            || !(1..=55_000).contains(&self.millis)
        {
            return Err(Refusal::Capacity);
        }
        Ok(())
    }
}

/// Exact private job header; intentionally has no Debug implementation.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Strict private transport revision.
    pub schema: u16,
    /// Immutable authority package; no external path or executable command.
    pub package: Package,
    /// Frozen reviewed catalog.
    pub catalog: Catalog,
    /// Sealed execution generation and acknowledged record commitment.
    pub binding: Binding,
    /// Owner-selected finite verification allocation.
    pub resources: Resources,
}

impl Request {
    /// Validate structural bounds and exact commitment before invoking the engine.
    /// # Errors
    /// Invalid schema/package/prefix or an exhausted resource allocation.
    pub fn validate(&self) -> Result<(), Refusal> {
        self.resources.validate()?;
        if self.binding.prefix.bytes > self.resources.bytes {
            return Err(Refusal::Capacity);
        }
        if self.schema != 2 || self.package.digest(&self.catalog)? != self.binding.package {
            return Err(Refusal::Invalid);
        }
        let mut execution = Execution::new(self.binding.package, self.resources.bytes)?;
        execution.fence = self.binding.fence;
        execution.prefix = self.binding.prefix;
        execution.sealed = true;
        execution.validate()
    }

    /// Bound and decode one private header, refusing unknown fields and trailing data.
    /// # Errors
    /// Oversized, malformed or incompatible request.
    pub fn decode(bytes: &[u8]) -> Result<Self, Refusal> {
        if bytes.is_empty() || bytes.len() > MAX_REQUEST_BYTES {
            return Err(Refusal::Capacity);
        }
        let request: Self = serde_json::from_slice(bytes).map_err(|_| Refusal::Invalid)?;
        request.validate()?;
        Ok(request)
    }

    /// Canonical bounded encoding for the trusted parent's private pipe.
    /// # Errors
    /// Invalid request or serialized size outside the header budget.
    pub fn encode(&self) -> Result<Vec<u8>, Refusal> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| Refusal::Invalid)?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(Refusal::Capacity);
        }
        Ok(bytes)
    }
}

/// Value-free refusal. Neither variant is a rules loss/draw or permission to award.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rejected {
    /// Inconsistent material, corrupted stream or illegal/fabricated play.
    Invalid,
    /// Technical allocation exhausted; the saved game remains unresolved.
    Budget,
}

/// One bounded worker response, accepted only from an owned exact local executable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    /// Facts recomputed by native replay; the authority must still check its live lease.
    Verified {
        /// Exact evidence binding, actual rules result and consumed charges.
        settlement: Settlement,
    },
    /// No result may be applied to the run.
    Rejected {
        /// Closed refusal category; never raw error text or private record material.
        reason: Rejected,
    },
}
