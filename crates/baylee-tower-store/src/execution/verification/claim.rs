use super::{
    ConnectionTrait, Error, Principal, Refusal, Role, Store, Uuid, VerificationJob,
    VerificationLease, column, deadline, signed,
};

impl Store {
    /// Read a bounded ready queue in a selected compatible cohort. Races are resolved
    /// by claiming a candidate; listing is never an execution/verification grant.
    /// # Errors
    /// Wrong/disabled role, invalid bound or storage failure.
    pub async fn verification_candidates(
        &self,
        principal: &Principal,
        cohort: Uuid,
        limit: u8,
    ) -> Result<Vec<Uuid>, Error> {
        if !(1..=32).contains(&limit) {
            return Err(Refusal::Capacity.into());
        }
        crate::uuid7(cohort)?;
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Verifier).await?;
        let rows = tx.query_all_raw(self.statement("SELECT j.reservation FROM @tower.verification_jobs j JOIN @tower.reservations e ON e.id=j.reservation JOIN @tower.runs r ON r.id=e.run
            WHERE r.cohort=$1 AND r.expires_at>pg_catalog.clock_timestamp() AND j.ready AND j.lease_until<=$2 ORDER BY j.created_at,j.reservation LIMIT $3",
            vec![cohort.into(), signed(self.now(&tx).await?)?.into(), i64::from(limit).into()])).await.map_err(|_| Error::Unavailable)?;
        let result = rows
            .iter()
            .map(|row| column(row, "reservation"))
            .collect::<Result<Vec<_>, _>>()?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(result)
    }

    /// Lease retained sealed evidence to an independently authenticated verifier.
    /// Exact retries share one live generation; expired jobs can be reassigned.
    /// # Errors
    /// Wrong role, unsealed evidence, busy job, stale claim or bounded claim capacity.
    pub async fn claim_verification(
        &self,
        principal: &Principal,
        reservation: Uuid,
        claim: Uuid,
        duration: u64,
    ) -> Result<VerificationJob, Error> {
        crate::uuid7(claim)?;
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Verifier).await?;
        let current = self.verification_current(&tx, reservation).await?;
        let binding = current.binding()?;
        let row = tx.query_one_raw(self.statement("SELECT generation,verifier,claim,lease_until FROM @tower.verification_jobs WHERE reservation=$1 AND fence=$2 AND ready FOR UPDATE", vec![reservation.into(), signed(binding.fence)?.into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Refusal::Stale)?;
        let now = self.now(&tx).await?;
        let previous: Option<Uuid> = column(&row, "claim")?;
        let old_generation: i64 = column(&row, "generation")?;
        let old_until: i64 = column(&row, "lease_until")?;
        let (generation, until) = if previous == Some(claim) {
            if column::<Option<Uuid>>(&row, "verifier")? != Some(principal.id)
                || old_until <= signed(now)?
            {
                return Err(Refusal::Stale.into());
            }
            (old_generation, old_until)
        } else {
            if old_until > signed(now)? {
                return Err(Refusal::Unavailable.into());
            }
            let generation = old_generation.checked_add(1).ok_or(Refusal::Capacity)?;
            let until = signed(deadline(now, duration)?)?;
            self.new_verification_claim(&tx, principal, reservation, claim, generation)
                .await?;
            tx.execute_raw(self.statement("UPDATE @tower.verification_jobs SET generation=$2,verifier=$3,claim=$4,lease_until=$5 WHERE reservation=$1",
                vec![reservation.into(), generation.into(), principal.id.into(), claim.into(), until.into()])).await.map_err(|_| Error::Unavailable)?;
            (generation, until)
        };
        let result = VerificationJob {
            lease: VerificationLease {
                reservation,
                claim,
                generation: u64::try_from(generation).map_err(|_| Error::Unavailable)?,
                binding,
            },
            package: current.package()?,
            catalog: current.saved.publication.catalog.clone(),
            lease_until: u64::try_from(until).map_err(|_| Error::Unavailable)?,
        };
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(result)
    }

    async fn new_verification_claim(
        &self,
        db: &impl ConnectionTrait,
        principal: &Principal,
        reservation: Uuid,
        claim: Uuid,
        generation: i64,
    ) -> Result<(), Error> {
        let row = db.query_one_raw(self.statement("SELECT count(*)::bigint AS count,COALESCE(bool_or(claim=$2),false) AS repeated FROM @tower.verification_claims WHERE reservation=$1", vec![reservation.into(), claim.into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
        if column::<bool>(&row, "repeated")? {
            return Err(Refusal::Stale.into());
        }
        if column::<i64>(&row, "count")? >= crate::MAX_EXECUTION_CLAIMS {
            return Err(Refusal::Capacity.into());
        }
        db.execute_raw(self.statement("INSERT INTO @tower.verification_claims(reservation,claim,generation,verifier) VALUES($1,$2,$3,$4)",
            vec![reservation.into(), claim.into(), generation.into(), principal.id.into()])).await.map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// Extend a still-live verifier lease without replacing its evidence or generation.
    /// # Errors
    /// Stale/wrong/expired verifier or invalid duration.
    pub async fn renew_verification(
        &self,
        principal: &Principal,
        lease: VerificationLease,
        duration: u64,
    ) -> Result<u64, Error> {
        let tx = self.transaction().await?;
        self.verifier(&tx, principal, lease).await?;
        let until = deadline(self.now(&tx).await?, duration)?;
        let row = tx.query_one_raw(self.statement("UPDATE @tower.verification_jobs SET lease_until=GREATEST(lease_until,$2) WHERE reservation=$1 RETURNING lease_until", vec![lease.reservation.into(), signed(until)?.into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
        let until =
            u64::try_from(column::<i64>(&row, "lease_until")?).map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(until)
    }
}
