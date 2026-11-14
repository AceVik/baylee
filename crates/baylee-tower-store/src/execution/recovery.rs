use super::{ConnectionTrait, Error, Phase, Principal, Refusal, Role, Store, Uuid};
use crate::Snapshot;
use serde::{Deserialize, Serialize};

/// Technical supervision cannot alter rules results, builds, seeds, gold or opponents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Recovery {
    /// Revoke all current worker/verifier generations and retain every committed byte.
    Quarantine,
    /// Resume the same package/prefix; a sealed record only returns to verification.
    Resume {
        /// Required for an unsealed duel; forbidden when only verification resumes.
        host: Option<Uuid>,
    },
    /// End an unrecoverable run without granting a rules outcome or compensation.
    Interrupt,
}

impl Store {
    /// Apply an explicit supervisor transition against a fresh saved revision.
    /// No ordinary owner/host/verifier token grants this permission.
    /// # Errors
    /// Wrong role/revision/phase or an unavailable exact recovery executor.
    pub async fn supervise(
        &self,
        principal: &Principal,
        run: Uuid,
        revision: u64,
        operation: Recovery,
    ) -> Result<Snapshot, Error> {
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Supervisor).await?;
        let mut current = self.current(&tx, run).await?;
        let state = &current.saved.snapshot.state;
        let catalog = &current.saved.publication.catalog;
        let next = match operation {
            Recovery::Quarantine => state.quarantined(revision, catalog)?,
            Recovery::Interrupt => state.interrupted(revision, catalog)?,
            Recovery::Resume { host } => {
                if state.phase != Phase::Quarantined {
                    return Err(Refusal::Unavailable.into());
                }
                if current.execution()?.sealed {
                    if host.is_some() {
                        return Err(Refusal::Invalid.into());
                    }
                } else {
                    let host = host.ok_or(Refusal::Invalid)?;
                    self.compatible_host(&tx, host, &current.saved.publication.cohort.executor)
                        .await?;
                    tx.execute_raw(self.statement(
                        "UPDATE @tower.reservations SET host=$2 WHERE id=$1",
                        vec![current.reservation.into(), host.into()],
                    ))
                    .await
                    .map_err(|_| Error::Unavailable)?;
                }
                state.recovered(revision, catalog)?
            }
        };
        current.saved.snapshot.state = next;
        self.save_current(&tx, &current).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(current.saved.snapshot)
    }
}
