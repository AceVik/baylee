//! Shared transaction helpers. Every mutable run path locks owner before run.
use super::{
    ConnectionTrait, Error, Publication, Refusal, Run, Snapshot, Store, Uuid, column, decode,
    encode, runs,
};

pub(super) struct Loaded {
    pub account: Uuid,
    pub snapshot: Snapshot,
    pub publication: Publication,
    pub seed: [u8; 32],
}

impl Store {
    /// Caller already holds this owner's row lock.
    pub(super) async fn locked_run(
        &self,
        db: &impl ConnectionTrait,
        account: Uuid,
        id: Uuid,
    ) -> Result<Loaded, Error> {
        let row = db.query_one_raw(self.statement("SELECT cohort,state,shop_seed,revision FROM @tower.runs WHERE account=$1 AND id=$2 AND expires_at>pg_catalog.clock_timestamp() FOR UPDATE",
            vec![account.into(), id.into()])).await.map_err(|_| Error::Unavailable)?.ok_or(Error::NotFound)?;
        let cohort = column(&row, "cohort")?;
        let publication = self.publication(db, cohort, false).await?;
        let state: Run = decode(&column::<Vec<u8>>(&row, "state")?)?;
        state.validate_for(&publication.catalog, publication.cohort.generator)?;
        if i64::try_from(state.revision).ok() != Some(column(&row, "revision")?) {
            return Err(Error::Unavailable);
        }
        let seed = column::<Vec<u8>>(&row, "shop_seed")?
            .try_into()
            .map_err(|_| Error::Unavailable)?;
        Ok(Loaded {
            account,
            snapshot: Snapshot { id, cohort, state },
            publication,
            seed,
        })
    }

    pub(super) async fn save_run(
        &self,
        db: &impl ConnectionTrait,
        account: Uuid,
        response: &Snapshot,
    ) -> Result<(), Error> {
        let revision = i64::try_from(response.state.revision).map_err(|_| Refusal::Capacity)?;
        let changed = db.execute_raw(self.statement("UPDATE @tower.runs SET state=$3,revision=$4,active=$5,touched_at=pg_catalog.clock_timestamp(),expires_at=pg_catalog.clock_timestamp()+interval '30 days' WHERE account=$1 AND id=$2",
            vec![account.into(), response.id.into(), encode(&response.state)?.into(), revision.into(), runs::active(response.state.phase).into()]))
            .await.map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Error::NotFound);
        }
        self.execution_watch(db, response).await?;
        Ok(())
    }

    pub(super) async fn now(&self, db: &impl ConnectionTrait) -> Result<u64, Error> {
        let row = db.query_one_raw(self.statement("SELECT floor(extract(epoch FROM pg_catalog.clock_timestamp())*1000)::bigint AS now", vec![]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
        u64::try_from(column::<i64>(&row, "now")?).map_err(|_| Error::Unavailable)
    }
}
