//! Public routing is approved by the operator, never supplied by a remote host/player.
use super::{ConnectionTrait, Error, Phase, Refusal, Store, Uuid, column, uuid7};
use baylee_tower_api::{Host, Location};

impl Store {
    /// Offline operator approval using the migration/admin connection, never an HTTP
    /// host/player operation. A host's destination is immutable; rehosting needs a
    /// separately approved identity and explicit reservation supervision.
    /// # Errors
    /// Noncanonical/cross-environment origin, wrong role, changed mapping or storage fault.
    pub async fn approve_host(
        &self,
        host: Uuid,
        origin: &str,
        development: bool,
    ) -> Result<(), Error> {
        uuid7(host)?;
        central_api::gateway::validate_origin(origin, development).map_err(|_| Refusal::Invalid)?;
        let changed = self
            .db
            .execute_raw(self.statement(
                "INSERT INTO @tower.host_locations AS saved(host,origin,development)
            SELECT id,$2,$3 FROM @tower.operators WHERE id=$1 AND role='host' AND enabled
            ON CONFLICT(host) DO UPDATE SET origin=EXCLUDED.origin
            WHERE saved.origin=EXCLUDED.origin AND saved.development=EXCLUDED.development",
                vec![host.into(), origin.into(), development.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Error::NotFound);
        }
        Ok(())
    }

    /// Locate one fresh owned assignment under the same owner/run/reservation locks
    /// as execution. An old command receipt can never send the player to an old host.
    /// No credential, private package, fence or record prefix is returned.
    /// # Errors
    /// Missing/foreign/expired run, invalid stored data or unavailable storage.
    pub async fn locate(&self, account: Uuid, run: Uuid) -> Result<Location, Error> {
        uuid7(run)?;
        let tx = self.transaction().await?;
        self.owner(&tx, account, false).await?;
        let saved = self.locked_run(&tx, account, run).await?;
        let state = &saved.snapshot.state;
        let mut location = Location {
            run,
            revision: state.revision,
            phase: state.phase,
            host: None,
            game: None,
        };
        if matches!(state.phase, Phase::Reserved | Phase::Playing) {
            let row = tx.query_one_raw(self.statement("SELECT r.id,r.host FROM @tower.reservations r WHERE r.run=$1 AND r.stage=$2 FOR UPDATE",
                vec![run.into(), i16::from(state.shop.stage).into()])).await.map_err(|_| Error::Unavailable)?.ok_or(Error::NotFound)?;
            let reservation: Uuid = column(&row, "id")?;
            let host: Uuid = column(&row, "host")?;
            // An approved location never changes (`approve_host` only repeats it), so
            // the enabled operator row is the one to hold; the runtime has no
            // UPDATE on locations to lock them with (`runtime_grants.sql`).
            let destination = tx.query_one_raw(self.statement("SELECT l.origin,l.development,o.name FROM @tower.host_locations l JOIN @tower.operators o ON o.id=l.host
                WHERE l.host=$1 AND o.role='host' AND o.enabled FOR SHARE OF o", vec![host.into()])).await.map_err(|_| Error::Unavailable)?;
            if let Some(destination) = destination {
                let origin = column::<String>(&destination, "origin")?;
                central_api::gateway::validate_origin(
                    &origin,
                    column(&destination, "development")?,
                )
                .map_err(|_| Error::Unavailable)?;
                location.host = Some(Host {
                    id: host,
                    name: column(&destination, "name")?,
                    origin,
                });
                if state.phase == Phase::Playing {
                    let fence = state.execution.as_ref().ok_or(Error::Unavailable)?.fence;
                    let row = tx.query_one_raw(self.statement("SELECT claim FROM @tower.execution_claims WHERE reservation=$1 AND fence=$2 AND host=$3",
                        vec![reservation.into(), i64::try_from(fence).map_err(|_| Error::Unavailable)?.into(), host.into()])).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
                    location.game = Some(column(&row, "claim")?);
                }
            }
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(location)
    }
}
