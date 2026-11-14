//! Execution-only operations; none are accepted as owner shop commands.
use super::{Catalog, Execution, Phase, Prefix, Refusal, Run};

/// Authenticated worker operation. The authority supplies the clock, identity and
/// saved head; it must persist chunks atomically before delivering acknowledgements.
pub enum WorkerCommand<'a> {
    /// Acquire a lease for a reserved/recoverable exact package.
    Claim {
        /// Renewable lease duration in milliseconds.
        duration: u64,
    },
    /// Keep the same worker generation alive.
    Renew {
        /// Authorized worker generation.
        fence: u64,
        /// Renewable duration in milliseconds.
        duration: u64,
    },
    /// Retain the next exact bytes before their consequences are revealed.
    Append {
        /// Authorized worker generation.
        fence: u64,
        /// Previously acknowledged head.
        expected: Prefix,
        /// Bounded ordered record bytes.
        chunk: &'a [u8],
    },
    /// Freeze the final head for independent verification.
    Seal {
        /// Authorized worker generation.
        fence: u64,
        /// Exact terminal head already persisted.
        expected: Prefix,
    },
}

impl Run {
    /// Freeze shopping together with the authority's immutable package reservation.
    /// # Errors
    /// Wrong phase/revision, reused package or invalid technical budget.
    pub fn reserved(
        &self,
        revision: u64,
        package: [u8; 32],
        byte_limit: u64,
        catalog: &Catalog,
    ) -> Result<Self, Refusal> {
        self.check(revision, catalog)?;
        if self.phase != Phase::Shop {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        next.execution = Some(Execution::new(package, byte_limit)?);
        next.phase = Phase::Reserved;
        next.shop.mythic_choice = false;
        next.revised(catalog)
    }

    /// Apply a fenced worker transition. No record bytes are stored by this function.
    /// # Errors
    /// Wrong phase/revision, stale worker/head, storage budget or lease violation.
    pub fn worker(
        &self,
        revision: u64,
        now: u64,
        command: WorkerCommand<'_>,
        catalog: &Catalog,
    ) -> Result<Self, Refusal> {
        self.check(revision, catalog)?;
        let execution = self.execution.as_ref().ok_or(Refusal::Unavailable)?;
        let mut next = self.clone();
        next.execution = Some(match (self.phase, command) {
            (Phase::Reserved, WorkerCommand::Claim { duration }) => {
                next.phase = Phase::Playing;
                execution.claimed(now, duration)?
            }
            (Phase::Playing, WorkerCommand::Renew { fence, duration }) => {
                execution.renewed(fence, now, duration)?
            }
            (
                Phase::Playing,
                WorkerCommand::Append {
                    fence,
                    expected,
                    chunk,
                },
            ) => execution.appended(fence, now, expected, chunk)?,
            (Phase::Playing, WorkerCommand::Seal { fence, expected }) => {
                next.phase = Phase::Verifying;
                execution.sealed(fence, now, expected)?
            }
            _ => return Err(Refusal::Unavailable),
        });
        next.revised(catalog)
    }

    /// Trusted supervisor stops execution/verification for an explicit technical failure.
    /// # Errors
    /// Wrong revision or no outstanding recoverable reservation.
    pub fn quarantined(&self, revision: u64, catalog: &Catalog) -> Result<Self, Refusal> {
        self.check(revision, catalog)?;
        if !matches!(
            self.phase,
            Phase::Reserved | Phase::Playing | Phase::Verifying
        ) {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        next.execution = Some(self.execution.as_ref().ok_or(Refusal::Invalid)?.revoked()?);
        next.phase = Phase::Quarantined;
        next.revised(catalog)
    }

    /// Trusted recovery returns to the exact retained prefix/package or re-verifies a
    /// sealed record. The execution boundary must replay that prefix before revealing.
    /// # Errors
    /// Wrong revision/phase. There is no fresh opponent or rewind operation.
    pub fn recovered(&self, revision: u64, catalog: &Catalog) -> Result<Self, Refusal> {
        self.check(revision, catalog)?;
        if self.phase != Phase::Quarantined {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        next.phase = if self.execution.as_ref().ok_or(Refusal::Invalid)?.sealed {
            Phase::Verifying
        } else {
            Phase::Reserved
        };
        next.revised(catalog)
    }

    /// Explicit trusted retirement after recovery cannot be completed; no game result.
    /// # Errors
    /// Wrong revision or a run not in quarantine.
    pub fn interrupted(&self, revision: u64, catalog: &Catalog) -> Result<Self, Refusal> {
        self.check(revision, catalog)?;
        if self.phase != Phase::Quarantined {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        next.phase = Phase::Interrupted;
        next.revised(catalog)
    }
}
