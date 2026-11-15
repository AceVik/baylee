//! Durable ready-queue execution. Polling is authority transport, never game rules.
use crate::{Artifact, Error, Verifier};
use baylee_tower::v2::verification::Resources;
use baylee_tower_store::{Principal, Role, Snapshot, Store, VerificationFailure};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use uuid::Uuid;
mod recovery;
use recovery::Expiration;

/// One queue worker with separate adjudication and technical-recovery identities.
/// Clone shares its single slot; configure more independent workers for capacity.
#[derive(Clone)]
pub struct Queue {
    store: Store,
    verifier: Principal,
    supervisor: Principal,
    worker: Verifier,
    admission: Arc<Semaphore>,
}

/// One inspected queue step, with no hidden record/build in its response.
#[derive(Debug, PartialEq, Eq)]
pub enum QueueResult {
    /// No presently claimable work, including a lost claim race.
    Idle,
    /// Native proof and progression committed together.
    Settled(Snapshot),
    /// Evidence retained and fenced for explicit technical recovery; no award.
    Quarantined {
        /// Preserved run after revoking its execution/verification generations.
        snapshot: Snapshot,
        /// Closed technical category, with no private payload or raw process log.
        reason: VerificationFailure,
    },
}

/// Value-free counters returned on graceful queue shutdown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueueSummary {
    /// Successful independently verified settlements in this worker lifetime.
    pub settled: u64,
    /// Retained failed jobs that require recovery, never losses/draws.
    pub quarantined: u64,
    /// Expired execution generations safely returned to their existing reservation.
    pub recovered: u64,
    /// Expired executions retained for explicit supervision after automatic retries.
    pub execution_quarantined: u64,
}

impl Queue {
    /// Configure one dedicated worker from operator-installed artifacts/credentials.
    /// # Errors
    /// Wrong roles or invalid finite replay allocation.
    pub fn new(
        store: Store,
        verifier: Principal,
        supervisor: Principal,
        artifact: Artifact,
        resources: Resources,
    ) -> Result<Self, Error> {
        if verifier.role() != Role::Verifier || supervisor.role() != Role::Supervisor {
            return Err(Error::Contract);
        }
        Ok(Self {
            store,
            verifier,
            supervisor,
            worker: Verifier::new(artifact, 1, resources)?,
            admission: Arc::new(Semaphore::new(1)),
        })
    }

    /// Claim at most one ready job in an operator-selected cohort, and apply only
    /// native proof. A failed child is reaped before exact-generation quarantine.
    /// # Errors
    /// Cancellation, unavailable capacity, artifact mismatch or storage failure.
    pub async fn next(
        &self,
        cohort: Uuid,
        stop: impl Future<Output = ()>,
    ) -> Result<QueueResult, Error> {
        let _admission = self
            .admission
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        if self.worker.running() != 0 {
            return Err(Error::Busy);
        }
        tokio::pin!(stop);
        let ready = tokio::select! {
            biased;
            () = &mut stop => return Err(Error::Cancelled),
            ready = self.store.verification_candidates(&self.verifier, cohort, 1) => ready?,
        };
        let Some(&reservation) = ready.first() else {
            return Ok(QueueResult::Idle);
        };
        let job = tokio::select! {
            biased;
            () = &mut stop => return Err(Error::Cancelled),
            job = self.store.claim_verification(&self.verifier, reservation, Uuid::now_v7(), 120_000) => job,
        };
        let job = match job {
            Ok(job) => job,
            Err(baylee_tower_store::Error::Refused(
                baylee_tower::Refusal::Unavailable | baylee_tower::Refusal::Stale,
            )) => return Ok(QueueResult::Idle),
            Err(error) => return Err(error.into()),
        };
        // An operator misconfiguration cannot quarantine valid evidence just
        // because this worker has no compatible executable installed.
        if &job.package.cohort.executor != self.worker.identity() {
            return Err(Error::Contract);
        }
        let lease = job.lease;
        let reason = match self
            .worker
            .adjudicate(&self.store, &self.verifier, job, &mut stop)
            .await
        {
            Ok(snapshot) => return Ok(QueueResult::Settled(snapshot)),
            Err(Error::Record) => VerificationFailure::InvalidRecord,
            Err(Error::Budget) => VerificationFailure::Budget,
            Err(Error::Process) => VerificationFailure::Worker,
            Err(Error::Contract) => VerificationFailure::Protocol,
            Err(error) => return Err(error),
        };
        // Once work failed, finish the bounded recovery transaction even if
        // shutdown arrives now. A changed generation still prevents mutation.
        let snapshot = self
            .store
            .quarantine_verification(&self.supervisor, lease, reason)
            .await?;
        Ok(QueueResult::Quarantined { snapshot, reason })
    }

    /// Automatically consume the durable queue; idle polling is bounded to once
    /// per second. Shutdown awaits owned-process cleanup. Store/config errors stop
    /// this worker visibly instead of hot-looping or synthesizing a game result.
    /// # Errors
    /// Authority access/configuration failure; retained jobs remain recoverable.
    pub async fn serve(
        &self,
        cohort: Uuid,
        stop: impl Future<Output = ()>,
    ) -> Result<QueueSummary, Error> {
        tokio::pin!(stop);
        let mut summary = QueueSummary::default();
        loop {
            match self.recover_one(cohort, &mut stop).await {
                Ok(Expiration::Resumed) => summary.recovered = summary.recovered.saturating_add(1),
                Ok(Expiration::Quarantined) => {
                    summary.execution_quarantined = summary.execution_quarantined.saturating_add(1);
                }
                Ok(Expiration::Idle) => {}
                Err(Error::Cancelled) => return Ok(summary),
                Err(error) => return Err(error),
            }
            match self.next(cohort, &mut stop).await {
                Ok(QueueResult::Settled(_)) => summary.settled = summary.settled.saturating_add(1),
                Ok(QueueResult::Quarantined { .. }) => {
                    summary.quarantined = summary.quarantined.saturating_add(1);
                }
                Ok(QueueResult::Idle) | Err(Error::Busy) => tokio::select! {
                    biased;
                    () = &mut stop => return Ok(summary),
                    () = tokio::time::sleep(Duration::from_secs(1)) => {},
                },
                Err(Error::Cancelled) => return Ok(summary),
                Err(error) => return Err(error),
            }
        }
    }
}
