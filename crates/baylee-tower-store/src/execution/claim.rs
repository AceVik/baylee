use super::{
    Claimed, ConnectionTrait, Current, Error, Lease, Phase, Principal, Refusal, Role, Store, Uuid,
    WorkerCommand, column, signed, uuid7,
};

impl Store {
    /// Claim the exact owner-reserved duel using a unique execution-instance ID.
    /// Retries return the same live generation; expiry requires explicit recovery.
    /// # Errors
    /// Wrong/disabled host, missing exact artifact, live competition or stale claim.
    pub async fn claim(
        &self,
        principal: &Principal,
        run: Uuid,
        claim: Uuid,
        duration: u64,
    ) -> Result<Claimed, Error> {
        uuid7(claim)?;
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Host).await?;
        let mut current = self.current(&tx, run).await?;
        if current.host != principal.id {
            return Err(Error::NotFound);
        }
        let package = current.package()?;
        self.compatible_host(&tx, principal.id, &package.cohort.executor)
            .await?;
        let now = self.now(&tx).await?;
        let old = tx.query_one_raw(self.statement("SELECT fence FROM @tower.execution_claims WHERE reservation=$1 AND claim=$2 AND host=$3",
            vec![current.reservation.into(), claim.into(), principal.id.into()])).await.map_err(|_| Error::Unavailable)?;
        if let Some(row) = old {
            let execution = current.execution()?;
            if signed(execution.fence)? != column::<i64>(&row, "fence")?
                || current.saved.snapshot.state.phase != Phase::Playing
                || now >= execution.lease_until
            {
                return Err(Refusal::Stale.into());
            }
        } else {
            self.new_claim(&tx, &mut current, claim, now, duration)
                .await?;
        }
        let execution = current.execution()?;
        let response = Claimed {
            lease: Lease {
                reservation: current.reservation,
                claim,
                fence: execution.fence,
            },
            package,
            catalog: current.saved.publication.catalog.clone(),
            prefix: execution.prefix,
            lease_until: execution.lease_until,
        };
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(response)
    }

    async fn new_claim(
        &self,
        db: &impl ConnectionTrait,
        current: &mut Current,
        claim: Uuid,
        now: u64,
        duration: u64,
    ) -> Result<(), Error> {
        let row = db.query_one_raw(self.statement("SELECT count(*)::bigint AS count FROM @tower.execution_claims WHERE reservation=$1", vec![current.reservation.into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
        if column::<i64>(&row, "count")? >= crate::MAX_EXECUTION_CLAIMS {
            return Err(Refusal::Capacity.into());
        }
        let state = &current.saved.snapshot.state;
        current.saved.snapshot.state = state.worker(
            state.revision,
            now,
            WorkerCommand::Claim { duration },
            &current.saved.publication.catalog,
        )?;
        db.execute_raw(self.statement(
            "INSERT INTO @tower.execution_claims(reservation,claim,fence,host) VALUES($1,$2,$3,$4)",
            vec![
                current.reservation.into(),
                claim.into(),
                signed(current.execution()?.fence)?.into(),
                current.host.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        self.save_current(db, current).await
    }

    /// Renew only the currently assigned, still-live worker generation.
    /// # Errors
    /// Wrong operator/claim, expired generation or invalid duration.
    pub async fn renew(
        &self,
        principal: &Principal,
        lease: Lease,
        duration: u64,
    ) -> Result<u64, Error> {
        let tx = self.transaction().await?;
        let mut current = self.worker(&tx, principal, lease).await?;
        let now = self.now(&tx).await?;
        let state = &current.saved.snapshot.state;
        current.saved.snapshot.state = state.worker(
            state.revision,
            now,
            WorkerCommand::Renew {
                fence: lease.fence,
                duration,
            },
            &current.saved.publication.catalog,
        )?;
        self.save_current(&tx, &current).await?;
        let until = current.execution()?.lease_until;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(until)
    }
}
