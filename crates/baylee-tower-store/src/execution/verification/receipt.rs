use super::{Error, Principal, Refusal, Role, Store, VerificationLease, column};
use crate::{ConnectionTrait, Snapshot, decode};
use baylee_tower::v2::run::Settlement;

impl Store {
    /// Recover an already committed adjudication after a lost service response.
    /// The exact original verifier, claim, generation and evidence are required;
    /// this never replays, re-awards or overwrites later owner progression.
    /// # Errors
    /// Revoked/wrong role, another claimant's receipt or incompatible evidence.
    pub async fn verification_receipt(
        &self,
        principal: &Principal,
        lease: VerificationLease,
    ) -> Result<Option<Snapshot>, Error> {
        crate::uuid7(lease.reservation)?;
        crate::uuid7(lease.claim)?;
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Verifier).await?;
        let row = tx.query_one_raw(self.statement(
            "SELECT verifier,claim,generation,report,response FROM @tower.settled_duels WHERE reservation=$1",
            vec![lease.reservation.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        let response = row
            .map(|row| {
                let report: Settlement = decode(&column::<Vec<u8>>(&row, "report")?)?;
                if column::<uuid::Uuid>(&row, "verifier")? != principal.id
                    || column::<uuid::Uuid>(&row, "claim")? != lease.claim
                    || column::<i64>(&row, "generation")? != super::signed(lease.generation)?
                    || report.binding != lease.binding
                {
                    return Err(Refusal::Stale.into());
                }
                decode(&column::<Vec<u8>>(&row, "response")?)
            })
            .transpose()?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(response)
    }
}
