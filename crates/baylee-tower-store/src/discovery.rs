//! Indexed exact-executor discovery, independent of an operator's installed inventory.
use super::{ConnectionTrait, Error, Refusal, Store, Uuid, column, uuid7};
use baylee_protocol::execution::Identity;
use baylee_tower_api::Host;

impl Store {
    pub(super) async fn save_availability(
        &self,
        db: &impl ConnectionTrait,
        host: Uuid,
        available: &[Identity],
    ) -> Result<(), Error> {
        // The caller holds this host's operator write lock. A stale concurrent
        // advertisement cannot interleave partial capability/availability state.
        db.execute_raw(self.statement(
            "DELETE FROM @tower.host_availability WHERE host=$1",
            vec![host.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        if available.is_empty() {
            return Ok(());
        }
        let mut values = vec![host.into()];
        let mut rows = Vec::with_capacity(available.len());
        for identity in available {
            values.push(identity.to_metadata().into());
            rows.push(format!(
                "($1,${},pg_catalog.clock_timestamp()+interval '60 seconds')",
                values.len()
            ));
        }
        db.execute_raw(self.statement(
            &format!(
                "INSERT INTO @tower.host_availability(host,executor,expires_at) VALUES {}",
                rows.join(",")
            ),
            values,
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// Fresh approved destinations for this owner's exact retained cohort.
    /// Discovery is only a capacity hint; assignment/claims still check live state.
    /// # Errors
    /// Foreign/expired run, invalid cursor/bound or unavailable storage.
    pub async fn hosts(
        &self,
        account: Uuid,
        run: Uuid,
        before: Option<Uuid>,
        limit: u8,
    ) -> Result<Vec<Host>, Error> {
        uuid7(run)?;
        if let Some(before) = before {
            uuid7(before)?;
        }
        if !(1..=16).contains(&limit) {
            return Err(Refusal::Capacity.into());
        }
        let tx = self.transaction().await?;
        self.owner(&tx, account, false).await?;
        let saved = self.locked_run(&tx, account, run).await?;
        let rows = tx.query_all_raw(self.statement(
            "SELECT o.id,o.name,l.origin,l.development FROM @tower.host_availability a
             JOIN @tower.operators o ON o.id=a.host JOIN @tower.host_locations l ON l.host=a.host
             WHERE a.executor=$1 AND a.expires_at>pg_catalog.clock_timestamp() AND o.enabled AND o.role='host'
               AND ($2::uuid IS NULL OR o.id<$2) ORDER BY o.id DESC LIMIT $3",
            vec![saved.publication.cohort.executor.to_metadata().into(), before.into(), i64::from(limit).into()],
        )).await.map_err(|_| Error::Unavailable)?;
        let hosts = rows
            .iter()
            .map(|row| {
                let origin = column::<String>(row, "origin")?;
                central_api::gateway::validate_origin(&origin, column(row, "development")?)
                    .map_err(|_| Error::Unavailable)?;
                Ok(Host {
                    id: column(row, "id")?,
                    name: column(row, "name")?,
                    origin,
                })
            })
            .collect::<Result<_, Error>>()?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(hosts)
    }
}
