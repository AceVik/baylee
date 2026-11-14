//! Dense global matching partitions. All mutations hold the partition row lock.
use super::{ConnectionTrait, Error, Refusal, Store, Uuid, column, decode, encode};
use baylee_tower::v2::{
    MAX_STAGES, matching,
    package::{Opponent, Origin, Package},
};

mod eligibility;
mod selection;

struct Partition {
    cohort: Uuid,
    stage: u8,
    count: u64,
    generation: u64,
    cursor: u64,
}

impl Store {
    async fn partition(
        &self,
        db: &impl ConnectionTrait,
        cohort: Uuid,
        stage: u8,
    ) -> Result<Partition, Error> {
        if !(1..=MAX_STAGES).contains(&stage) {
            return Err(Refusal::Invalid.into());
        }
        db.execute_raw(self.statement("INSERT INTO @tower.echo_partitions(cohort,stage) VALUES($1,$2) ON CONFLICT(cohort,stage) DO NOTHING", vec![cohort.into(), i16::from(stage).into()]))
            .await.map_err(|_| Error::Unavailable)?;
        let row = db.query_one_raw(self.statement("SELECT population,generation,prune_slot FROM @tower.echo_partitions WHERE cohort=$1 AND stage=$2 FOR UPDATE", vec![cohort.into(), i16::from(stage).into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
        Ok(Partition {
            cohort,
            stage,
            count: unsigned(column(&row, "population")?)?,
            generation: unsigned(column(&row, "generation")?)?,
            cursor: unsigned(column(&row, "prune_slot")?)?,
        })
    }

    async fn remove_slot(
        &self,
        db: &impl ConnectionTrait,
        partition: &mut Partition,
        slot: u64,
    ) -> Result<(), Error> {
        let removal = matching::removal(partition.count, slot)?;
        let removed = db
            .execute_raw(self.statement(
                "DELETE FROM @tower.eligible_echoes WHERE cohort=$1 AND stage=$2 AND slot=$3",
                vec![
                    partition.cohort.into(),
                    i16::from(partition.stage).into(),
                    signed(slot)?.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if removed.rows_affected() != 1 {
            return Err(Error::Unavailable);
        }
        if let Some(last) = removal.move_from {
            let moved = db.execute_raw(self.statement("UPDATE @tower.eligible_echoes SET slot=$3 WHERE cohort=$1 AND stage=$2 AND slot=$4",
                vec![partition.cohort.into(), i16::from(partition.stage).into(), signed(slot)?.into(), signed(last)?.into()])).await.map_err(|_| Error::Unavailable)?;
            if moved.rows_affected() != 1 {
                return Err(Error::Unavailable);
            }
        }
        partition.count = removal.remaining;
        self.changed_partition(db, partition).await
    }

    async fn changed_partition(
        &self,
        db: &impl ConnectionTrait,
        partition: &mut Partition,
    ) -> Result<(), Error> {
        partition.generation = partition
            .generation
            .checked_add(1)
            .ok_or(Refusal::Capacity)?;
        db.execute_raw(self.statement("UPDATE @tower.echo_partitions SET population=$3,generation=$4 WHERE cohort=$1 AND stage=$2",
            vec![partition.cohort.into(), i16::from(partition.stage).into(), signed(partition.count)?.into(), signed(partition.generation)?.into()])).await.map_err(|_| Error::Unavailable)?;
        Ok(())
    }
}

fn signed(value: u64) -> Result<i64, Error> {
    i64::try_from(value).map_err(|_| Refusal::Capacity.into())
}

fn unsigned(value: i64) -> Result<u64, Error> {
    u64::try_from(value).map_err(|_| Error::Unavailable)
}
