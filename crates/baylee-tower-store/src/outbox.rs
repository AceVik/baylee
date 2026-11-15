//! At-least-once delivery of immutable verified facts, separate from game authority.
use super::{Award, ConnectionTrait, Error, Principal, Refusal, Role, Store, Uuid, column, decode};

const LEASE_MS: i64 = 120_000;

/// A delivery fence, never an authorization credential or client-selected result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AwardLease {
    /// Stable event identity for Central deduplication, unchanged on every retry.
    pub event: Uuid,
    /// Database-generated fence, replaced after expiry or a deferred attempt.
    pub claim: Uuid,
}

/// Minimal permanent-accomplishment payload; no deck, RNG, record or display name.
pub struct AwardDelivery {
    /// Exact claim that may acknowledge this attempt.
    pub lease: AwardLease,
    /// Existing Central account; the recipient must recheck account existence.
    pub account: Uuid,
    /// Immutable independently verified progression fact.
    pub award: Award,
}

impl Store {
    /// Claim at most one due event for two minutes, without blocking another worker.
    /// A crash leaves the event eligible after expiry. No game state is modified.
    /// # Errors
    /// Wrong/disabled operator role, malformed stored event or database failure.
    pub async fn claim_award(&self, principal: &Principal) -> Result<Option<AwardDelivery>, Error> {
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Courier).await?;
        let now = i64::try_from(self.now(&tx).await?).map_err(|_| Error::Unavailable)?;
        let until = now.checked_add(LEASE_MS).ok_or(Error::Unavailable)?;
        let row = tx
            .query_one_raw(self.statement(
                "WITH next AS (SELECT id FROM @tower.award_outbox
                WHERE delivered_at IS NULL AND retry_after<=$1 AND lease_until<=$1
                ORDER BY retry_after,created_at,id LIMIT 1 FOR UPDATE SKIP LOCKED)
             UPDATE @tower.award_outbox a SET courier=$2,claim=pg_catalog.uuidv7(),lease_until=$3
             FROM next WHERE a.id=next.id RETURNING a.id,a.claim,a.account,a.event",
                vec![now.into(), principal.id.into(), until.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let delivery = row
            .map(|row| {
                Ok::<_, Error>(AwardDelivery {
                    lease: AwardLease {
                        event: column(&row, "id")?,
                        claim: column(&row, "claim")?,
                    },
                    account: column(&row, "account")?,
                    award: decode(&column::<Vec<u8>>(&row, "event")?)?,
                })
            })
            .transpose()?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(delivery)
    }

    /// Acknowledge only after Central has durably accepted this exact event.
    /// Repeating the same successful acknowledgment is harmless. Possession of a
    /// fence without the still-enabled Courier principal grants no access.
    /// # Errors
    /// Wrong/disabled role, stale/foreign claim, deleted event or database failure.
    pub async fn acknowledge_award(
        &self,
        principal: &Principal,
        lease: AwardLease,
    ) -> Result<(), Error> {
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Courier).await?;
        let changed = tx.execute_raw(self.statement(
            "UPDATE @tower.award_outbox SET delivered_at=COALESCE(delivered_at,pg_catalog.clock_timestamp())
             WHERE id=$1 AND claim=$2 AND courier=$3 AND (delivered_at IS NOT NULL
                OR lease_until>floor(extract(epoch FROM pg_catalog.clock_timestamp())*1000)::bigint)",
            vec![lease.event.into(), lease.claim.into(), principal.id.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Refusal::Stale.into());
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Retain a failed delivery with 5s–1h backoff; never persist remote error text.
    /// The cleared claim prevents a delayed acknowledgment of this failed attempt.
    /// # Errors
    /// Wrong/disabled role, stale/foreign/completed claim or database failure.
    pub async fn defer_award(&self, principal: &Principal, lease: AwardLease) -> Result<(), Error> {
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Courier).await?;
        let changed = tx.execute_raw(self.statement(
            "UPDATE @tower.award_outbox SET retry_after=
                floor(extract(epoch FROM pg_catalog.clock_timestamp())*1000)::bigint
                    +LEAST(3600000::bigint,5000::bigint << backoff),
                backoff=LEAST(10,backoff+1),claim=NULL,courier=NULL,lease_until=0
             WHERE id=$1 AND claim=$2 AND courier=$3 AND delivered_at IS NULL
                AND lease_until>floor(extract(epoch FROM pg_catalog.clock_timestamp())*1000)::bigint",
            vec![lease.event.into(), lease.claim.into(), principal.id.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Refusal::Stale.into());
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }
}
