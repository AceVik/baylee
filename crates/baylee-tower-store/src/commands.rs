use super::{
    Command, ConnectionTrait, Error, Refusal, Run, Snapshot, Store, Uuid, column, decode, encode,
    requests, runs, uuid7,
};

impl Store {
    /// Lock ownership, check/deduplicate the exact request, and commit mutation plus
    /// original response together. Same request with different contents is a conflict.
    /// # Errors
    /// Wrong owner, stale command, receipt conflict, capacity or storage failure.
    pub async fn command(
        &self,
        account: Uuid,
        request: Uuid,
        id: Uuid,
        revision: u64,
        command: &Command,
    ) -> Result<Snapshot, Error> {
        uuid7(id)?;
        let digest = requests::digest(b"command", &(id, revision, command))?;
        let tx = self.transaction().await?;
        self.owner(&tx, account, false).await?;
        if let Some(previous) = self.repeated(&tx, account, request, &digest).await? {
            return Ok(previous);
        }
        let row = tx.query_one_raw(self.statement("SELECT cohort,state,shop_seed,revision FROM @tower.runs WHERE account=$1 AND id=$2 AND expires_at>clock_timestamp() FOR UPDATE", vec![account.into(), id.into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::NotFound)?;
        let cohort = column(&row, "cohort")?;
        let publication = self.publication(&tx, cohort, false).await?;
        let state: Run = decode(&column::<Vec<u8>>(&row, "state")?)?;
        if i64::try_from(state.revision).ok() != Some(column(&row, "revision")?) {
            return Err(Error::Unavailable);
        }
        let seed: [u8; 32] = column::<Vec<u8>>(&row, "shop_seed")?
            .try_into()
            .map_err(|_| Error::Unavailable)?;
        let state = state.command(revision, command, &publication.catalog, &seed)?;
        let response = Snapshot { id, cohort, state };
        let stored_revision =
            i64::try_from(response.state.revision).map_err(|_| Refusal::Capacity)?;
        let changed = tx.execute_raw(self.statement("UPDATE @tower.runs SET state=$3,revision=$4,active=$5,touched_at=clock_timestamp(),expires_at=clock_timestamp()+interval '30 days' WHERE account=$1 AND id=$2",
            vec![account.into(), id.into(), encode(&response.state)?.into(), stored_revision.into(), runs::active(response.state.phase).into()]))
            .await.map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Error::NotFound);
        }
        self.receipt(&tx, account, request, digest, &response)
            .await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(response)
    }
}
