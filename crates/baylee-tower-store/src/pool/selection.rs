use super::{
    ConnectionTrait, Error, Opponent, Origin, Partition, Refusal, Store, column, decode,
    matching::{self, Population},
    signed, unsigned,
};
use crate::{Uuid, state::Loaded};

impl Store {
    pub(crate) async fn select_opponent(
        &self,
        db: &impl ConnectionTrait,
        saved: &Loaded,
        seed: &[u8; 32],
    ) -> Result<Opponent, Error> {
        let mut partition = self
            .partition(db, saved.snapshot.cohort, saved.snapshot.state.shop.stage)
            .await?;
        // Stale selected entries are removed before resampling; no expired/withdrawn
        // row may become an opponent. A separate bounded janitor drains large backlogs.
        for _ in 0..32 {
            let (own, previous) = self.exclusions(db, &partition, saved).await?;
            let population = Population {
                count: partition.count,
                generation: partition.generation,
                own,
                previous: &previous,
            };
            let Some(selection) = matching::select(&population, seed)? else {
                return self.house_opponent(db, saved, seed).await;
            };
            let row = db.query_one_raw(self.statement("SELECT e.account,e.snapshot,s.build,(p.echo_consent AND p.echo_epoch=s.consent_epoch AND s.expires_at>pg_catalog.clock_timestamp()) AS eligible
                FROM @tower.eligible_echoes e JOIN @tower.echo_snapshots s ON s.id=e.snapshot JOIN @tower.players p ON p.account=e.account
                WHERE e.cohort=$1 AND e.stage=$2 AND e.slot=$3",
                vec![partition.cohort.into(), i16::from(partition.stage).into(), signed(selection.slot)?.into()])).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
            if !column::<bool>(&row, "eligible")? {
                self.remove_slot(db, &mut partition, selection.slot).await?;
                continue;
            }
            let account: Uuid = column(&row, "account")?;
            if account == saved.account {
                return Err(Error::Unavailable);
            }
            return Ok(Opponent {
                stage: partition.stage,
                cohort: saved.publication.cohort.clone(),
                origin: Origin::Echo {
                    snapshot: *column::<Uuid>(&row, "snapshot")?.as_bytes(),
                    account: *account.as_bytes(),
                },
                build: decode(&column::<Vec<u8>>(&row, "build")?)?,
            });
        }
        // Do not invent an empty pool/house opponent when cleanup has not completed.
        Err(Refusal::Unavailable.into())
    }

    async fn exclusions(
        &self,
        db: &impl ConnectionTrait,
        partition: &Partition,
        saved: &Loaded,
    ) -> Result<(Option<u64>, Vec<u64>), Error> {
        let rows = db.query_all_raw(self.statement("SELECT account,slot FROM @tower.eligible_echoes WHERE cohort=$1 AND stage=$2 AND (account=$3 OR account IN
            (SELECT opponent_account FROM @tower.reservations WHERE run=$4 AND opponent_account IS NOT NULL)) LIMIT 13",
            vec![partition.cohort.into(), i16::from(partition.stage).into(), saved.account.into(), saved.snapshot.id.into()])).await.map_err(|_| Error::Unavailable)?;
        let mut own = None;
        let mut previous = Vec::with_capacity(rows.len());
        for row in rows {
            let slot = unsigned(column(&row, "slot")?)?;
            if column::<Uuid>(&row, "account")? == saved.account {
                own = Some(slot);
            } else {
                previous.push(slot);
            }
        }
        Ok((own, previous))
    }
}
