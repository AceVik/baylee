use super::{
    CONSENT_REVISION, ConnectionTrait, Deserialize, Error, MAX_RUNS, Phase, Refusal, Run,
    Serialize, Snapshot, Store, Uuid, column, decode, encode, requests, uuid7,
};

/// Owner's enrollment choices, with no seed, price or external collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Start {
    /// Published compatible cohort.
    pub cohort: Uuid,
    /// Zero-based starter, within the immutable publication's choices.
    pub starter: u8,
    /// Explicit acceptance of the current visible echo-use explanation.
    pub consent: u16,
}

impl Store {
    /// Explicit enrollment/start after the UI has explained historical echo use.
    /// `seed` is authority randomness, not client input; abandoning before a verified
    /// duel reuses the stored account/cohort seed instead of granting another sample.
    /// # Errors
    /// Wrong identity/starter, request conflict, active run or unavailable cohort.
    pub async fn start(
        &self,
        account: Uuid,
        request: Uuid,
        choice: Start,
        seed: [u8; 32],
    ) -> Result<Snapshot, Error> {
        let Start {
            cohort,
            starter,
            consent,
        } = choice;
        if starter >= 8 || consent != CONSENT_REVISION {
            return Err(Refusal::Invalid.into());
        }
        let digest = requests::digest(b"start", &choice)?;
        let tx = self.transaction().await?;
        self.owner(&tx, account, true).await?;
        if let Some(previous) = self.repeated(&tx, account, request, &digest).await? {
            tx.commit().await.map_err(|_| Error::Unavailable)?;
            return Ok(previous);
        }
        tx.execute_raw(self.statement(
            "UPDATE @tower.players SET echo_consent=true,consent_revision=$2,consent_at=pg_catalog.clock_timestamp() WHERE account=$1",
            vec![account.into(), i16::try_from(consent).map_err(|_| Refusal::Invalid)?.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let publication = self.publication(&tx, cohort, true).await?;
        let starter = publication
            .starters
            .get(usize::from(starter))
            .ok_or(Refusal::Invalid)?
            .clone();
        if tx
            .query_one_raw(self.statement(
                "SELECT id FROM @tower.runs WHERE account=$1 AND cohort=$2 AND active",
                vec![account.into(), cohort.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .is_some()
        {
            return Err(Error::Conflict);
        }
        let allocated = tx
            .execute_raw(self.statement(
                "UPDATE @tower.players SET run_count=run_count+1 WHERE account=$1 AND run_count<$2",
                vec![account.into(), MAX_RUNS.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if allocated.rows_affected() != 1 {
            return Err(Refusal::Capacity.into());
        }
        tx.execute_raw(self.statement("INSERT INTO @tower.enrollment(account,cohort,start_seed) VALUES($1,$2,$3) ON CONFLICT(account,cohort) DO NOTHING",
            vec![account.into(), cohort.into(), seed.to_vec().into()]))
            .await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(self.statement(
                "SELECT start_seed FROM @tower.enrollment WHERE account=$1 AND cohort=$2",
                vec![account.into(), cohort.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let seed: [u8; 32] = column::<Vec<u8>>(&row, "start_seed")?
            .try_into()
            .map_err(|_| Error::Unavailable)?;
        let state = Run::begin_for(
            &publication.catalog,
            starter,
            &seed,
            publication.cohort.generator,
        )?;
        let row = tx.query_one_raw(self.statement("INSERT INTO @tower.runs(account,cohort,revision,active,state,shop_seed) VALUES($1,$2,0,true,$3,$4) RETURNING id",
            vec![account.into(), cohort.into(), encode(&state)?.into(), seed.to_vec().into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
        let response = Snapshot {
            id: column(&row, "id")?,
            cohort,
            state,
        };
        self.receipt(&tx, account, request, digest, &response)
            .await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(response)
    }

    /// Read only a specific authenticated owner's run. Foreign IDs disclose nothing.
    /// # Errors
    /// Unknown/foreign/expired run or invalid stored state.
    pub async fn get(&self, account: Uuid, id: Uuid) -> Result<Snapshot, Error> {
        uuid7(account)?;
        uuid7(id)?;
        let row = self.db.query_one_raw(self.statement("SELECT id,cohort,state,revision FROM @tower.runs WHERE account=$1 AND id=$2 AND expires_at>clock_timestamp()", vec![account.into(), id.into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::NotFound)?;
        let cohort = column(&row, "cohort")?;
        let publication = self.publication(&self.db, cohort, false).await?;
        let state: Run = decode(&column::<Vec<u8>>(&row, "state")?)?;
        state.validate_for(&publication.catalog, publication.cohort.generator)?;
        if i64::try_from(state.revision).ok() != Some(column(&row, "revision")?) {
            return Err(Error::Unavailable);
        }
        Ok(Snapshot { id, cohort, state })
    }
}

pub(super) fn active(phase: Phase) -> bool {
    !matches!(
        phase,
        Phase::Conquered | Phase::Exhausted | Phase::Abandoned | Phase::Interrupted
    )
}
