use super::{
    ConnectionTrait, Error, Package, Refusal, Store, Uuid, column, encode, signed, unsigned,
};
use crate::{Principal, Role};

impl Store {
    /// Only called inside settlement, after its verified terminal row has been inserted.
    /// Outcome never influences eligibility or representative choice.
    pub(crate) async fn admit_echo(
        &self,
        db: &impl ConnectionTrait,
        account: Uuid,
        cohort: Uuid,
        reservation: Uuid,
        package: &Package,
    ) -> Result<(), Error> {
        let owner = db
            .query_one_raw(self.statement(
                "SELECT echo_consent,echo_epoch FROM @tower.players WHERE account=$1",
                vec![account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        if !column::<bool>(&owner, "echo_consent")? {
            return Ok(());
        }
        let epoch: i64 = column(&owner, "echo_epoch")?;
        let build = encode(&package.builds[0])?;
        let row = db.query_one_raw(self.statement("INSERT INTO @tower.echo_snapshots(reservation,account,cohort,stage,consent_epoch,build) VALUES($1,$2,$3,$4,$5,$6) RETURNING id",
            vec![reservation.into(), account.into(), cohort.into(), i16::from(package.stage).into(), epoch.into(), build.into()])).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
        let snapshot: Uuid = column(&row, "id")?;
        let mut partition = self.partition(db, cohort, package.stage).await?;
        let previous = db.query_one_raw(self.statement("SELECT e.slot,(s.consent_epoch=$4 AND s.expires_at>pg_catalog.clock_timestamp()) AS eligible FROM @tower.eligible_echoes e JOIN @tower.echo_snapshots s ON s.id=e.snapshot
            WHERE e.cohort=$1 AND e.stage=$2 AND e.account=$3",
            vec![cohort.into(), i16::from(package.stage).into(), account.into(), epoch.into()])).await.map_err(|_| Error::Unavailable)?;
        if let Some(previous) = previous {
            if column::<bool>(&previous, "eligible")? {
                return Ok(());
            }
            self.remove_slot(db, &mut partition, unsigned(column(&previous, "slot")?)?)
                .await?;
        }
        db.execute_raw(self.statement("INSERT INTO @tower.eligible_echoes(cohort,stage,slot,account,snapshot) VALUES($1,$2,$3,$4,$5)",
            vec![cohort.into(), i16::from(package.stage).into(), signed(partition.count)?.into(), account.into(), snapshot.into()])).await.map_err(|_| Error::Unavailable)?;
        partition.count = partition.count.checked_add(1).ok_or(Refusal::Capacity)?;
        self.changed_partition(db, &mut partition).await
    }

    /// Take every eligible echo of `account` out of the dense pool, one partition at
    /// a time in a fixed order, inside the caller's transaction (which holds the
    /// player's row lock). A plain delete would leave a hole in the slots, and the
    /// snapshot key refuses the player's deletion while one is eligible.
    pub(crate) async fn remove_account_echoes(
        &self,
        db: &impl ConnectionTrait,
        account: Uuid,
    ) -> Result<(), Error> {
        let partitions = db
            .query_all_raw(self.statement(
                "SELECT cohort,stage FROM @tower.eligible_echoes WHERE account=$1 ORDER BY cohort,stage",
                vec![account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        for row in partitions {
            let cohort: Uuid = column(&row, "cohort")?;
            let stage =
                u8::try_from(column::<i16>(&row, "stage")?).map_err(|_| Error::Unavailable)?;
            let mut partition = self.partition(db, cohort, stage).await?;
            // Read the slot under the partition's lock: another removal moves slots.
            let held = db
                .query_one_raw(self.statement(
                    "SELECT slot FROM @tower.eligible_echoes WHERE cohort=$1 AND stage=$2 AND account=$3",
                    vec![cohort.into(), i16::from(stage).into(), account.into()],
                ))
                .await
                .map_err(|_| Error::Unavailable)?;
            if let Some(held) = held {
                self.remove_slot(db, &mut partition, unsigned(column(&held, "slot")?)?)
                    .await?;
            }
        }
        Ok(())
    }

    /// Withdraw future echo use/attribution immediately. Old snapshots never become
    /// eligible again merely because a later start explicitly reenrolls the account.
    /// Existing private reservations keep their committed rules material.
    /// # Errors
    /// Unknown account, exhausted consent generation or storage failure.
    pub async fn withdraw_echoes(&self, account: Uuid) -> Result<(), Error> {
        let tx = self.transaction().await?;
        self.owner(&tx, account, false).await?;
        tx.execute_raw(self.statement("UPDATE @tower.players SET echo_consent=false,echo_epoch=echo_epoch+1 WHERE account=$1 AND echo_consent", vec![account.into()]))
            .await.map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Resolve local echo-consent/retention visibility at projection time. The service
    /// must additionally consult current Central identity visibility before naming it.
    /// # Errors
    /// Malformed IDs or storage failure; missing/deleted references return false.
    pub async fn echo_visible(&self, account: Uuid, snapshot: Uuid) -> Result<bool, Error> {
        crate::uuid7(account)?;
        crate::uuid7(snapshot)?;
        Ok(self.db.query_one_raw(self.statement("SELECT s.id FROM @tower.echo_snapshots s JOIN @tower.players p ON p.account=s.account
            WHERE s.id=$1 AND s.account=$2 AND p.echo_consent AND p.echo_epoch=s.consent_epoch AND s.expires_at>pg_catalog.clock_timestamp()", vec![snapshot.into(), account.into()]))
            .await.map_err(|_| Error::Unavailable)?.is_some())
    }

    /// Bounded background scan/removal of expired/withdrawn dense slots. The limit
    /// counts inspected rows; a saved cursor advances even when no row is stale.
    /// Never scans the whole population or cascades through the dense index.
    /// # Errors
    /// Wrong role, bad bound/stage or inconsistent persisted partition.
    pub async fn prune_echoes(
        &self,
        principal: &Principal,
        cohort: Uuid,
        stage: u8,
        limit: u8,
    ) -> Result<u8, Error> {
        if !(1..=64).contains(&limit) {
            return Err(Refusal::Capacity.into());
        }
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Supervisor).await?;
        let mut partition = self.partition(&tx, cohort, stage).await?;
        let mut removed = 0;
        for _ in 0..limit {
            if partition.count == 0 {
                break;
            }
            partition.cursor %= partition.count;
            let row = tx.query_one_raw(self.statement("SELECT (p.echo_consent AND p.echo_epoch=s.consent_epoch AND s.expires_at>pg_catalog.clock_timestamp()) AS eligible
                FROM @tower.eligible_echoes e JOIN @tower.echo_snapshots s ON s.id=e.snapshot JOIN @tower.players p ON p.account=e.account
                WHERE e.cohort=$1 AND e.stage=$2 AND e.slot=$3",
                vec![cohort.into(), i16::from(stage).into(), signed(partition.cursor)?.into()])).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
            if column::<bool>(&row, "eligible")? {
                partition.cursor += 1;
            } else {
                let slot = partition.cursor;
                self.remove_slot(&tx, &mut partition, slot).await?;
                removed += 1;
            }
        }
        tx.execute_raw(self.statement(
            "UPDATE @tower.echo_partitions SET prune_slot=$3 WHERE cohort=$1 AND stage=$2",
            vec![
                cohort.into(),
                i16::from(stage).into(),
                signed(partition.cursor)?.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(removed)
    }
}
