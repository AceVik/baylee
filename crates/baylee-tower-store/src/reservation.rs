//! Owner-bound reservation; private randomness and opponents never come from a player.
use super::{
    ConnectionTrait, Deserialize, Error, Refusal, Serialize, Snapshot, Store, Uuid, column, decode,
    requests, uuid7,
};
use baylee_protocol::execution::{Identity, valid_capabilities};
use baylee_tower::v2::{
    matching::{self, Population},
    package::{Opponent, Origin, Package, Seeds},
};

/// The owner's permitted start-duel inputs, including the chosen approved gateway.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reserve {
    /// Owned run identity.
    pub run: Uuid,
    /// Fresh saved shop revision.
    pub revision: u64,
    /// Approved execution operator, never an executable path.
    pub host: Uuid,
}

/// Independently allocated authority randomness; not a player request type.
pub struct Entropy {
    /// Private uniform matchmaking seed.
    pub opponent: [u8; 32],
    /// Independent shuffle and frozen Expert streams.
    pub game: Seeds,
}

impl Store {
    /// Freeze exactly one opponent/build/package and its owner receipt atomically.
    /// Caller supplies independent authority entropy and an operator storage budget.
    /// # Errors
    /// Wrong owner, stale shop, duplicate key, unavailable exact host/content or quota.
    pub async fn reserve(
        &self,
        account: Uuid,
        request: Uuid,
        choice: Reserve,
        entropy: Entropy,
        byte_limit: u64,
    ) -> Result<Snapshot, Error> {
        uuid7(choice.run)?;
        uuid7(choice.host)?;
        let digest = requests::digest(b"reserve", &choice)?;
        let tx = self.transaction().await?;
        self.owner(&tx, account, false).await?;
        if let Some(previous) = self.repeated(&tx, account, request, &digest).await? {
            tx.commit().await.map_err(|_| Error::Unavailable)?;
            return Ok(previous);
        }
        let mut saved = self.locked_run(&tx, account, choice.run).await?;
        self.compatible_host(&tx, choice.host, &saved.publication.cohort.executor)
            .await?;
        let opponent = self.house_opponent(&tx, &saved, &entropy.opponent).await?;
        let frozen = Package::reserve(
            &saved.snapshot.state,
            choice.revision,
            &saved.publication.catalog,
            &saved.publication.cohort,
            opponent,
            entropy.game,
            byte_limit,
        )?;
        tx.execute_raw(self.statement(
            "INSERT INTO @tower.reservations(run,stage,host,package) VALUES($1,$2,$3,$4)",
            vec![
                choice.run.into(),
                i16::from(frozen.package.stage).into(),
                choice.host.into(),
                frozen.package.encode(&saved.publication.catalog)?.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        saved.snapshot.state = frozen.run;
        self.save_run(&tx, account, &saved.snapshot).await?;
        self.receipt(&tx, account, request, digest, &saved.snapshot)
            .await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(saved.snapshot)
    }

    pub(super) async fn compatible_host(
        &self,
        db: &impl ConnectionTrait,
        host: Uuid,
        identity: &Identity,
    ) -> Result<(), Error> {
        let row = db.query_one_raw(self.statement("SELECT capabilities FROM @tower.operators WHERE id=$1 AND role='host' AND enabled FOR SHARE", vec![host.into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::NotFound)?;
        let inventory: Vec<Identity> = decode(&column::<Vec<u8>>(&row, "capabilities")?)?;
        if !valid_capabilities(&inventory) || !inventory.contains(identity) {
            return Err(Refusal::Unavailable.into());
        }
        Ok(())
    }

    pub(super) async fn house_opponent(
        &self,
        db: &impl ConnectionTrait,
        saved: &super::state::Loaded,
        seed: &[u8; 32],
    ) -> Result<Opponent, Error> {
        let stage = saved.snapshot.state.shop.stage;
        let rows = db.query_all_raw(self.statement("SELECT archetype,build FROM @tower.house_builds WHERE cohort=$1 AND stage=$2 ORDER BY archetype LIMIT 3",
            vec![saved.snapshot.cohort.into(), i16::from(stage).into()])).await.map_err(|_| Error::Unavailable)?;
        let population = Population {
            count: rows.len() as u64,
            generation: 0,
            own: None,
            previous: &[],
        };
        let selection = matching::select(&population, seed)?.ok_or(Refusal::Unavailable)?;
        let row = rows
            .get(usize::try_from(selection.slot).map_err(|_| Error::Unavailable)?)
            .ok_or(Error::Unavailable)?;
        Ok(Opponent {
            stage,
            cohort: saved.publication.cohort.clone(),
            origin: Origin::House {
                archetype: u8::try_from(column::<i16>(row, "archetype")?)
                    .map_err(|_| Error::Unavailable)?,
            },
            build: decode(&column::<Vec<u8>>(row, "build")?)?,
        })
    }
}
