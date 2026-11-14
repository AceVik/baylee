use crate::{Artifact, Error, process};
use baylee_tower::v2::{
    run::Settlement,
    verification::{Rejected, Request, Resources, Response},
};
use baylee_tower_store::{Principal, Role, Snapshot, Store, VerificationJob};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::{Semaphore, oneshot};

/// Bounded local adjudication capacity. The authority supplies already leased jobs.
#[derive(Clone)]
pub struct Verifier {
    artifact: Artifact,
    resources: Resources,
    capacity: Arc<Semaphore>,
    slots: usize,
}

impl Verifier {
    /// Configure trusted local capacity and finite native replay resources.
    /// # Errors
    /// Zero/excessive capacity or a resource allocation outside hard ceilings.
    pub fn new(artifact: Artifact, capacity: u8, resources: Resources) -> Result<Self, Error> {
        resources.validate().map_err(|_| Error::Budget)?;
        if !(1..=16).contains(&capacity) {
            return Err(Error::Contract);
        }
        Ok(Self {
            artifact,
            resources,
            capacity: Arc::new(Semaphore::new(usize::from(capacity))),
            slots: usize::from(capacity),
        })
    }

    /// Occupied local slots, retained until each owned child has exited.
    #[must_use]
    pub fn running(&self) -> usize {
        self.slots - self.capacity.available_permits()
    }

    /// Stream retained evidence to the exact owned native worker, then atomically
    /// apply its independently derived result under the still-live verifier lease.
    /// Cancellation kills/reaps before returning; dropping this future signals an
    /// independently owned cleanup task. No process gets DB/operator credentials.
    /// # Errors
    /// Busy/cancelled/bounded/refused verification or stale/failed store settlement.
    pub async fn adjudicate(
        &self,
        store: &Store,
        principal: &Principal,
        job: VerificationJob,
        stop: impl Future<Output = ()>,
    ) -> Result<Snapshot, Error> {
        if principal.role() != Role::Verifier
            || job.package.cohort.executor != self.artifact.identity
        {
            return Err(Error::Contract);
        }
        tokio::pin!(stop);
        let receipt = tokio::select! {
            biased;
            () = &mut stop => return Err(Error::Cancelled),
            receipt = store.verification_receipt(principal, job.lease) => receipt?,
        };
        if let Some(receipt) = receipt {
            return Ok(receipt);
        }
        let request = Request {
            schema: 2,
            package: job.package,
            catalog: job.catalog,
            binding: job.lease.binding,
            resources: self.resources,
        };
        let header = request.encode().map_err(|reason| match reason {
            baylee_tower::Refusal::Capacity => Error::Budget,
            _ => Error::Contract,
        })?;
        let permit = self
            .capacity
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let (keep, cancelled) = oneshot::channel();
        let artifact = self.artifact.clone();
        let worker_store = store.clone();
        let worker_principal = principal.clone();
        let lease = job.lease;
        let mut worker = tokio::spawn(async move {
            let result = run(
                &artifact,
                header,
                &worker_store,
                &worker_principal,
                lease,
                cancelled,
            )
            .await;
            drop(permit);
            result
        });
        let report = tokio::select! {
            biased;
            () = &mut stop => {
                drop(keep);
                let _ = worker.await.map_err(|_| Error::Process)?;
                return Err(Error::Cancelled);
            },
            result = &mut worker => result.map_err(|_| Error::Process)??,
        };
        drop(keep);
        if report.binding != lease.binding {
            return Err(Error::Contract);
        }
        let mut seed = [0; 32];
        getrandom::fill(&mut seed).map_err(|_| Error::Process)?;
        Ok(store
            .settle_verified(principal, lease, &report, seed)
            .await?)
    }
}

async fn run(
    artifact: &Artifact,
    header: Vec<u8>,
    store: &Store,
    principal: &Principal,
    lease: baylee_tower_store::VerificationLease,
    cancelled: oneshot::Receiver<()>,
) -> Result<Settlement, Error> {
    let mut child = process::command(&artifact.path, "--tower-verify")
        .spawn()
        .map_err(|_| Error::Process)?;
    let stdin = child.stdin.take().ok_or(Error::Process)?;
    let stdout = child.stdout.take().ok_or(Error::Process)?;
    let stderr = child.stderr.take().ok_or(Error::Process)?;
    let pid = child.id().ok_or(Error::Process)?;
    let limits = baylee_worker::Limits::default();
    let exchange = async {
        let (written, response, (), status) = tokio::try_join!(
            async { Ok::<_, Error>(process::write(stdin, header, store, principal, lease).await) },
            process::read(stdout),
            process::progress(stderr, limits),
            async { child.wait().await.map_err(|_| Error::Process) },
        )?;
        if !status.success() {
            return Err(Error::Process);
        }
        match response {
            Response::Verified { settlement } if settlement.binding == lease.binding => {
                written?;
                Ok(settlement)
            }
            Response::Verified { .. } => Err(Error::Contract),
            Response::Rejected {
                reason: Rejected::Invalid,
            } => Err(Error::Record),
            Response::Rejected {
                reason: Rejected::Budget,
            } => Err(Error::Budget),
        }
    };
    let result = tokio::select! {
        biased;
        _ = cancelled => Err(Error::Cancelled),
        error = process::memory(pid, limits.resident_bytes) => Err(error),
        result = tokio::time::timeout(Duration::from_secs(90), exchange) => result.unwrap_or(Err(Error::Budget)),
    };
    process::reap(&mut child, result.is_err()).await?;
    result
}
