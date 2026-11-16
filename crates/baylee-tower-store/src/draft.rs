//! Owner-bound starter offers share the durable anti-resampling enrollment seed.
use super::{ConnectionTrait, DatabaseTransaction, Error, Refusal, Store, Uuid, column};

impl Store {
    /// Read/create a stable three-choice sample without consenting to echo sharing
    /// or starting a run. The caller has authenticated this account. A paused
    /// publication may still be displayed, but `start` separately requires admission.
    /// # Errors
    /// Invalid owner/cohort, unsupported publication or unavailable storage.
    pub async fn starter_choices(
        &self,
        account: Uuid,
        cohort: Uuid,
        seed: [u8; 32],
    ) -> Result<Vec<u8>, Error> {
        let tx = self.transaction().await?;
        self.owner(&tx, account, true).await?;
        let publication = self.publication(&tx, cohort, false).await?;
        let choices = if publication.cohort.generator == baylee_tower::v2::generation::EXPEDITIONS {
            let seed = self.start_seed(&tx, account, cohort, seed).await?;
            baylee_tower::v2::draft::choices(&seed, publication.starters.len())?.to_vec()
        } else {
            let count = u8::try_from(publication.starters.len()).map_err(|_| Refusal::Invalid)?;
            (0..count).collect()
        };
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(choices)
    }

    // Call only after taking the owner lock. Both draft reads and Start use this
    // exact operation; concurrent browsers cannot obtain competing samples.
    pub(super) async fn start_seed(
        &self,
        tx: &DatabaseTransaction,
        account: Uuid,
        cohort: Uuid,
        seed: [u8; 32],
    ) -> Result<[u8; 32], Error> {
        tx.execute_raw(self.statement(
            "INSERT INTO @tower.enrollment(account,cohort,start_seed)
             SELECT $1,$2,$3 WHERE (SELECT count(*) FROM @tower.enrollment WHERE account=$1)<$4
             ON CONFLICT(account,cohort) DO NOTHING",
            vec![
                account.into(),
                cohort.into(),
                seed.to_vec().into(),
                super::MAX_RUNS.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(self.statement(
                "SELECT start_seed FROM @tower.enrollment WHERE account=$1 AND cohort=$2",
                vec![account.into(), cohort.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Refusal::Capacity)?;
        column::<Vec<u8>>(&row, "start_seed")?
            .try_into()
            .map_err(|_| Error::Unavailable)
    }
}
