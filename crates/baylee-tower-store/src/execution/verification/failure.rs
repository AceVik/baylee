use super::{ConnectionTrait, Error, Principal, Refusal, Role, Store, VerificationLease, signed};
use crate::Snapshot;
use serde::{Deserialize, Serialize};

/// Persisted closed reason for technical quarantine; never a rules result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationFailure {
    /// Independently replayed evidence was inconsistent or illegal.
    InvalidRecord,
    /// CPU/wall/memory or another finite verification allocation was exhausted.
    Budget,
    /// Local executable, process observation or pipe operation failed.
    Worker,
    /// The exact owned worker violated its private response/progress contract.
    Protocol,
}

impl VerificationFailure {
    const fn name(self) -> &'static str {
        match self {
            Self::InvalidRecord => "invalid_record",
            Self::Budget => "budget",
            Self::Worker => "worker",
            Self::Protocol => "protocol",
        }
    }
}

impl Store {
    /// Quarantine only the failed verifier generation's still-current evidence.
    /// This needs a separate supervisor credential; verifier/host roles never imply it.
    /// Expiration alone does not invalidate a failure, but any reassignment does.
    /// # Errors
    /// Wrong role, replaced/completed claim, changed evidence or storage failure.
    pub async fn quarantine_verification(
        &self,
        supervisor: &Principal,
        lease: VerificationLease,
        reason: VerificationFailure,
    ) -> Result<Snapshot, Error> {
        let tx = self.transaction().await?;
        self.role(&tx, supervisor, Role::Supervisor).await?;
        let mut current = self.verification_current(&tx, lease.reservation).await?;
        if current.binding()? != lease.binding {
            return Err(Refusal::Stale.into());
        }
        tx.query_one_raw(self.statement(
            "SELECT reservation FROM @tower.verification_jobs WHERE reservation=$1 AND claim=$2 AND generation=$3 AND fence=$4 AND ready FOR UPDATE",
            vec![lease.reservation.into(), lease.claim.into(), signed(lease.generation)?.into(), signed(lease.binding.fence)?.into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Refusal::Stale)?;
        let state = &current.saved.snapshot.state;
        current.saved.snapshot.state =
            state.quarantined(state.revision, &current.saved.publication.catalog)?;
        self.save_current(&tx, &current).await?;
        tx.execute_raw(self.statement(
            "UPDATE @tower.verification_jobs SET failure=$2 WHERE reservation=$1",
            vec![lease.reservation.into(), reason.name().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(current.saved.snapshot)
    }
}
