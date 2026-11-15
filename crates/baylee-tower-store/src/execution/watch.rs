//! Indexed expiration detection, with a locked fresh-state check before recovery.
use super::{
    ConnectionTrait, Current, Error, Phase, Principal, Refusal, Role, Store, Uuid, column, signed,
};
use crate::{MAX_AUTOMATIC_RECOVERIES, MAX_EXECUTION_CLAIMS, Snapshot};

/// A discovery hint, never authorization to reset a newer or renewed worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpiredExecution {
    /// The existing owned run, never a new match.
    pub run: Uuid,
    /// The generation observed in the expiry index.
    pub fence: u64,
    /// Its exact observed deadline in authority Unix milliseconds.
    pub lease_until: u64,
}

impl Store {
    pub(crate) async fn execution_watch(
        &self,
        db: &impl ConnectionTrait,
        saved: &Snapshot,
    ) -> Result<(), Error> {
        if saved.state.phase == Phase::Playing {
            let execution = saved.state.execution.as_ref().ok_or(Error::Unavailable)?;
            db.execute_raw(self.statement("INSERT INTO @tower.execution_watch AS saved(run,fence,lease_until) VALUES($1,$2,$3)
                ON CONFLICT(run) DO UPDATE SET fence=EXCLUDED.fence,lease_until=EXCLUDED.lease_until
                WHERE saved.fence<>EXCLUDED.fence OR saved.lease_until<>EXCLUDED.lease_until",
                vec![saved.id.into(), signed(execution.fence)?.into(), signed(execution.lease_until)?.into()])).await.map_err(|_| Error::Unavailable)?;
        } else {
            db.execute_raw(self.statement(
                "DELETE FROM @tower.execution_watch WHERE run=$1",
                vec![saved.id.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        }
        Ok(())
    }

    /// List bounded expired live executions without reading all private run states.
    /// # Errors
    /// Wrong supervisor role, invalid bound/cohort or unavailable storage.
    pub async fn expired_executions(
        &self,
        supervisor: &Principal,
        cohort: Uuid,
        limit: u8,
    ) -> Result<Vec<ExpiredExecution>, Error> {
        crate::uuid7(cohort)?;
        if !(1..=32).contains(&limit) {
            return Err(Refusal::Capacity.into());
        }
        let tx = self.transaction().await?;
        self.role(&tx, supervisor, Role::Supervisor).await?;
        let rows = tx.query_all_raw(self.statement("SELECT w.run,w.fence,w.lease_until FROM @tower.execution_watch w JOIN @tower.runs r ON r.id=w.run
            WHERE w.lease_until<=$1 AND r.cohort=$2 AND r.expires_at>pg_catalog.clock_timestamp()
            ORDER BY w.lease_until,w.run LIMIT $3", vec![signed(self.now(&tx).await?)?.into(), cohort.into(), i64::from(limit).into()])).await.map_err(|_| Error::Unavailable)?;
        let result = rows
            .iter()
            .map(|row| {
                Ok(ExpiredExecution {
                    run: column(row, "run")?,
                    fence: u64::try_from(column::<i64>(row, "fence")?)
                        .map_err(|_| Error::Unavailable)?,
                    lease_until: u64::try_from(column::<i64>(row, "lease_until")?)
                        .map_err(|_| Error::Unavailable)?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(result)
    }

    /// Revoke an expired generation and restore only its exact retained package/prefix.
    /// At most three automatic retries; disabled/incompatible hosts and exhausted
    /// budgets stay quarantined for explicit supervision. No rules result is assigned.
    /// # Errors
    /// Wrong role, renewed/replaced/settled work, invalid discovery hint or storage fault.
    pub async fn recover_expired(
        &self,
        supervisor: &Principal,
        observed: ExpiredExecution,
    ) -> Result<Snapshot, Error> {
        let tx = self.transaction().await?;
        self.role(&tx, supervisor, Role::Supervisor).await?;
        let mut current = self.current(&tx, observed.run).await?;
        let execution = current.execution()?;
        if current.saved.snapshot.state.phase != Phase::Playing
            || execution.sealed
            || execution.fence != observed.fence
            || execution.lease_until != observed.lease_until
            || self.now(&tx).await? < execution.lease_until
        {
            return Err(Refusal::Stale.into());
        }
        let resume = self.may_retry(&tx, &current).await?;
        let catalog = &current.saved.publication.catalog;
        let state = &current.saved.snapshot.state;
        let mut next = state.quarantined(state.revision, catalog)?;
        if resume {
            next = next.recovered(next.revision, catalog)?;
            tx.execute_raw(self.statement("UPDATE @tower.reservations SET automatic_recoveries=automatic_recoveries+1 WHERE id=$1", vec![current.reservation.into()])).await.map_err(|_| Error::Unavailable)?;
        }
        current.saved.snapshot.state = next;
        self.save_current(&tx, &current).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(current.saved.snapshot)
    }

    async fn may_retry(&self, db: &impl ConnectionTrait, current: &Current) -> Result<bool, Error> {
        let row = db.query_one_raw(self.statement("SELECT automatic_recoveries,(SELECT count(*)::bigint FROM @tower.execution_claims c WHERE c.reservation=r.id) AS claims
            FROM @tower.reservations r WHERE id=$1", vec![current.reservation.into()])).await.map_err(|_| Error::Unavailable)?.ok_or(Error::NotFound)?;
        if column::<i16>(&row, "automatic_recoveries")? >= MAX_AUTOMATIC_RECOVERIES
            || column::<i64>(&row, "claims")? >= MAX_EXECUTION_CLAIMS
        {
            return Ok(false);
        }
        match self
            .compatible_host(db, current.host, &current.saved.publication.cohort.executor)
            .await
        {
            Ok(()) => Ok(true),
            Err(Error::NotFound | Error::Refused(Refusal::Unavailable)) => Ok(false),
            Err(error) => Err(error),
        }
    }
}
