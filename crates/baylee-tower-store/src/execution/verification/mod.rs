//! A separately authenticated verifier leases only the retained sealed evidence.
use super::{ConnectionTrait, Current, Error, Phase, Principal, Role, Store, Uuid, column, signed};
use baylee_tower::{
    Refusal,
    cards::Catalog,
    v2::{execution::MAX_LEASE_MS, package::Package, run::Binding},
};
use serde::{Deserialize, Serialize};

mod claim;
mod record;
mod settlement;
pub use settlement::Award;

/// Identifies both the sealed execution and one independently fenced verification job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationLease {
    /// Immutable reservation, never a player-supplied replacement record.
    pub reservation: Uuid,
    /// Verifier's unique job-instance request identity, not a credential.
    pub claim: Uuid,
    /// Monotonic verifier generation, independent of the execution generation.
    pub generation: u64,
    /// Exact package, execution fence and sealed acknowledged record head.
    pub binding: Binding,
}

/// Private input delivered to the dedicated verifier role, never a player.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationJob {
    /// Fenced immutable evidence to which the result must bind.
    pub lease: VerificationLease,
    /// Canonical reserved game input.
    pub package: Package,
    /// Frozen reviewed card pool.
    pub catalog: Catalog,
    /// Renewable authority deadline in Unix milliseconds.
    pub lease_until: u64,
}

impl Current {
    fn binding(&self) -> Result<Binding, Error> {
        let execution = self.execution()?;
        if self.saved.snapshot.state.phase != Phase::Verifying || !execution.sealed {
            return Err(Refusal::Stale.into());
        }
        Ok(Binding {
            package: execution.package,
            fence: execution.fence,
            prefix: execution.prefix,
        })
    }
}

impl Store {
    pub(super) async fn verification_state(
        &self,
        db: &impl ConnectionTrait,
        current: &Current,
    ) -> Result<(), Error> {
        match current.saved.snapshot.state.phase {
            Phase::Verifying => {
                db.execute_raw(self.statement("INSERT INTO @tower.verification_jobs AS saved(reservation,fence) VALUES($1,$2)
                    ON CONFLICT(reservation) DO UPDATE SET fence=EXCLUDED.fence,generation=saved.generation+1,verifier=NULL,claim=NULL,lease_until=0,ready=true
                    WHERE saved.fence<>EXCLUDED.fence", vec![current.reservation.into(), signed(current.execution()?.fence)?.into()]))
                    .await.map_err(|_| Error::Unavailable)?;
            }
            Phase::Quarantined
            | Phase::Interrupted
            | Phase::Shop
            | Phase::Conquered
            | Phase::Exhausted => {
                db.execute_raw(self.statement("UPDATE @tower.verification_jobs SET ready=false,lease_until=0,generation=generation+1 WHERE reservation=$1 AND (ready OR lease_until<>0)", vec![current.reservation.into()]))
                    .await.map_err(|_| Error::Unavailable)?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn verification_current(
        &self,
        db: &impl ConnectionTrait,
        reservation: Uuid,
    ) -> Result<Current, Error> {
        crate::uuid7(reservation)?;
        let row = db
            .query_one_raw(self.statement(
                "SELECT run FROM @tower.reservations WHERE id=$1",
                vec![reservation.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        let current = self.current(db, column(&row, "run")?).await?;
        if current.reservation != reservation {
            return Err(Refusal::Stale.into());
        }
        current.binding()?;
        Ok(current)
    }

    async fn verifier(
        &self,
        db: &impl ConnectionTrait,
        principal: &Principal,
        lease: VerificationLease,
    ) -> Result<Current, Error> {
        self.role(db, principal, Role::Verifier).await?;
        let current = self.verification_current(db, lease.reservation).await?;
        if current.binding()? != lease.binding {
            return Err(Refusal::Stale.into());
        }
        db.query_one_raw(self.statement("SELECT reservation FROM @tower.verification_jobs WHERE reservation=$1 AND verifier=$2 AND claim=$3 AND generation=$4 AND fence=$5 AND ready AND lease_until>$6 FOR UPDATE",
            vec![lease.reservation.into(), principal.id.into(), lease.claim.into(), signed(lease.generation)?.into(), signed(lease.binding.fence)?.into(), signed(self.now(db).await?)?.into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Refusal::Stale)?;
        Ok(current)
    }
}

fn deadline(now: u64, duration: u64) -> Result<u64, Error> {
    if !(1..=MAX_LEASE_MS).contains(&duration) {
        return Err(Refusal::Invalid.into());
    }
    now.checked_add(duration)
        .ok_or_else(|| Refusal::Capacity.into())
}
