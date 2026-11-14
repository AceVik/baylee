//! Bounded, cursor-based owner reads; no record/package or other account projection.
use super::{
    ConnectionTrait, Error, Publication, Refusal, Run, Snapshot, Store, Uuid, column, decode, uuid7,
};

impl Store {
    /// Read one retained immutable publication, including closed enrollment cohorts.
    /// # Errors
    /// Unknown publication, malformed identity/content or unavailable storage.
    pub async fn published(&self, id: Uuid) -> Result<Publication, Error> {
        self.publication(&self.db, id, false).await
    }
    /// List accepting publications in descending UUID order. The cursor is exclusive.
    /// # Errors
    /// Invalid cursor/bound, malformed publication or unavailable storage.
    pub async fn cohorts(
        &self,
        before: Option<Uuid>,
        limit: u8,
    ) -> Result<Vec<(Uuid, Publication)>, Error> {
        page(before, limit)?;
        let rows = self.db.query_all_raw(self.statement(
            "SELECT id,publication FROM @tower.cohorts WHERE accepting AND ($1::uuid IS NULL OR id<$1) ORDER BY id DESC LIMIT $2",
            vec![before.into(), i64::from(limit).into()],
        )).await.map_err(|_| Error::Unavailable)?;
        rows.iter()
            .map(|row| {
                let publication: Publication = decode(&column::<Vec<u8>>(row, "publication")?)?;
                publication.validate()?;
                Ok((column(row, "id")?, publication))
            })
            .collect()
    }

    /// List only this authenticated account's unexpired runs. No owner row is created.
    /// # Errors
    /// Invalid identity/cursor/bound, malformed state or unavailable storage.
    pub async fn runs(
        &self,
        account: Uuid,
        before: Option<Uuid>,
        limit: u8,
    ) -> Result<Vec<Snapshot>, Error> {
        uuid7(account)?;
        page(before, limit)?;
        let rows = self.db.query_all_raw(self.statement(
            "SELECT r.id,r.cohort,r.state,r.revision,c.publication FROM @tower.runs r JOIN @tower.cohorts c ON c.id=r.cohort
             WHERE r.account=$1 AND r.expires_at>pg_catalog.clock_timestamp() AND ($2::uuid IS NULL OR r.id<$2) ORDER BY r.id DESC LIMIT $3",
            vec![account.into(), before.into(), i64::from(limit).into()],
        )).await.map_err(|_| Error::Unavailable)?;
        rows.iter()
            .map(|row| {
                let publication: Publication = decode(&column::<Vec<u8>>(row, "publication")?)?;
                publication.validate()?;
                let state: Run = decode(&column::<Vec<u8>>(row, "state")?)?;
                state.validate(&publication.catalog)?;
                if i64::try_from(state.revision).ok() != Some(column(row, "revision")?) {
                    return Err(Error::Unavailable);
                }
                Ok(Snapshot {
                    id: column(row, "id")?,
                    cohort: column(row, "cohort")?,
                    state,
                })
            })
            .collect()
    }
}

fn page(before: Option<Uuid>, limit: u8) -> Result<(), Error> {
    if !(1..=16).contains(&limit) {
        return Err(Refusal::Capacity.into());
    }
    if let Some(id) = before {
        uuid7(id)?;
    }
    Ok(())
}
