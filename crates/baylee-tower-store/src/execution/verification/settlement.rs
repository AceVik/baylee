use super::{
    ConnectionTrait, Error, Principal, Refusal, Role, Store, Uuid, VerificationLease, column,
    signed,
};
use crate::{Snapshot, decode, encode};
use baylee_tower::v2::run::{Outcome, Settlement};
use serde::{Deserialize, Serialize};

/// Verified progression fact for idempotent permanent cosmetic delivery to Central.
/// No hidden deck, record, name, seed or operator credential leaves in this value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Award {
    /// Strict version of the accomplishment event, not a game rules revision.
    pub schema: u16,
    /// Globally owned run, used only for deduplication/history.
    pub run: Uuid,
    /// Immutable compatibility cohort.
    pub cohort: Uuid,
    /// Completed one-based ordinal.
    pub stage: u8,
    /// Verified rules outcome, not a client score.
    pub outcome: Outcome,
    /// Total verified victories in this run after the duel.
    pub wins: u8,
    /// Total verified defeats/rules draws after the duel.
    pub defeats: u8,
}

impl Store {
    /// Commit independently recomputed facts under the exact live verifier claim.
    /// Only the trusted verification adapter may call this after actual native replay;
    /// constructing a Settlement or possessing a host/owner token is not verification.
    /// `next_start_seed` is independent authority randomness for future enrollment;
    /// it is committed only after the first verified duel and ignored on exact retries.
    /// # Errors
    /// Wrong/stale verifier/evidence, conflicting retry, invalid consumption or failed
    /// transaction. Result, next shop, retained receipt and outbox commit together.
    pub async fn settle_verified(
        &self,
        principal: &Principal,
        lease: VerificationLease,
        report: &Settlement,
        next_start_seed: [u8; 32],
    ) -> Result<Snapshot, Error> {
        if report.binding != lease.binding {
            return Err(Refusal::Stale.into());
        }
        let bytes = encode(report)?;
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Verifier).await?;
        let row = tx.query_one_raw(self.statement("SELECT r.account FROM @tower.reservations e JOIN @tower.runs r ON r.id=e.run WHERE e.id=$1", vec![lease.reservation.into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::NotFound)?;
        self.owner(&tx, column(&row, "account")?, false).await?;
        if let Some(response) = self.settlement_retry(&tx, principal, lease, &bytes).await? {
            tx.commit().await.map_err(|_| Error::Unavailable)?;
            return Ok(response);
        }
        let mut current = self.verifier(&tx, principal, lease).await?;
        let package = current.package()?;
        let state = &current.saved.snapshot.state;
        current.saved.snapshot.state = state.settled_against(
            state.revision,
            report.clone(),
            &current.saved.publication.catalog,
            &current.saved.seed,
            &package,
        )?;
        self.save_current(&tx, &current).await?;
        let response = &current.saved.snapshot;
        let encoded = encode(response)?;
        self.charge_settlement(&tx, current.saved.account, encoded.len())
            .await?;
        tx.execute_raw(self.statement("INSERT INTO @tower.settled_duels(reservation,verifier,claim,generation,report,response) VALUES($1,$2,$3,$4,$5,$6)",
            vec![lease.reservation.into(), principal.id.into(), lease.claim.into(), signed(lease.generation)?.into(), bytes.into(), encoded.into()]))
            .await.map_err(|_| Error::Unavailable)?;
        self.admit_echo(
            &tx,
            current.saved.account,
            response.cohort,
            lease.reservation,
            &package,
        )
        .await?;
        if package.stage == 1 {
            // Authority randomness, never an owner/host-provided result field.
            tx.execute_raw(self.statement(
                "UPDATE @tower.enrollment SET start_seed=$3 WHERE account=$1 AND cohort=$2",
                vec![
                    current.saved.account.into(),
                    response.cohort.into(),
                    next_start_seed.to_vec().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        }
        let award = Award {
            schema: 2,
            run: response.id,
            cohort: response.cohort,
            stage: package.stage,
            outcome: report.outcome,
            wins: u8::try_from(response.state.wins()).map_err(|_| Error::Unavailable)?,
            defeats: u8::try_from(response.state.defeats()).map_err(|_| Error::Unavailable)?,
        };
        tx.execute_raw(self.statement(
            "INSERT INTO @tower.award_outbox(reservation,account,event) VALUES($1,$2,$3)",
            vec![
                lease.reservation.into(),
                current.saved.account.into(),
                encode(&award)?.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(current.saved.snapshot)
    }

    async fn settlement_retry(
        &self,
        db: &impl ConnectionTrait,
        principal: &Principal,
        lease: VerificationLease,
        report: &[u8],
    ) -> Result<Option<Snapshot>, Error> {
        let row = db.query_one_raw(self.statement("SELECT verifier,claim,generation,report,response FROM @tower.settled_duels WHERE reservation=$1", vec![lease.reservation.into()]))
            .await.map_err(|_| Error::Unavailable)?;
        row.map(|row| {
            if column::<Uuid>(&row, "verifier")? != principal.id
                || column::<Uuid>(&row, "claim")? != lease.claim
                || column::<i64>(&row, "generation")? != signed(lease.generation)?
                || column::<Vec<u8>>(&row, "report")? != report
            {
                return Err(Error::Conflict);
            }
            decode(&column::<Vec<u8>>(&row, "response")?)
        })
        .transpose()
    }

    async fn charge_settlement(
        &self,
        db: &impl ConnectionTrait,
        account: Uuid,
        bytes: usize,
    ) -> Result<(), Error> {
        let changed = db.execute_raw(self.statement("UPDATE @tower.players SET receipt_bytes=receipt_bytes+$2 WHERE account=$1 AND receipt_bytes<=$3-$2",
            vec![account.into(), signed(bytes as u64)?.into(), crate::MAX_RECEIPT_BYTES.into()])).await.map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Refusal::Capacity.into());
        }
        Ok(())
    }
}
