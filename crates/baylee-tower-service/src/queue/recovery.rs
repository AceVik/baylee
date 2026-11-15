use super::{Error, Queue, Uuid};
use baylee_tower::v2::run::Phase;
use std::future::Future;

pub(super) enum Expiration {
    Idle,
    Resumed,
    Quarantined,
}

impl Queue {
    pub(super) async fn recover_one(
        &self,
        cohort: Uuid,
        stop: impl Future<Output = ()>,
    ) -> Result<Expiration, Error> {
        let candidates = tokio::select! {
            biased;
            () = stop => return Err(Error::Cancelled),
            candidates = self.store.expired_executions(&self.supervisor, cohort, 1) => candidates?,
        };
        let Some(&candidate) = candidates.first() else {
            return Ok(Expiration::Idle);
        };
        // Complete this bounded transaction even if shutdown arrives after discovery;
        // it can only revoke expired work and preserve the existing immutable prefix.
        let snapshot = match self
            .store
            .recover_expired(&self.supervisor, candidate)
            .await
        {
            Ok(snapshot) => snapshot,
            Err(
                baylee_tower_store::Error::NotFound
                | baylee_tower_store::Error::Refused(baylee_tower::Refusal::Stale),
            ) => return Ok(Expiration::Idle),
            Err(error) => return Err(error.into()),
        };
        match snapshot.state.phase {
            Phase::Reserved => Ok(Expiration::Resumed),
            Phase::Quarantined => Ok(Expiration::Quarantined),
            _ => Err(Error::Contract),
        }
    }
}
