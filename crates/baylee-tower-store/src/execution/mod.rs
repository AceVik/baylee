//! Durable append acknowledgements and fenced, separately authorized execution.
use super::{
    ConnectionTrait, Error, Principal, Refusal, Role, Store, Uuid, column, decode, encode,
    state::Loaded, uuid7,
};
use baylee_tower::v2::{
    execution::{Execution, Prefix},
    package::Package,
    run::{Phase, WorkerCommand},
};
pub use baylee_tower_api::host::{Acknowledged, Claimed, Lease, StoredChunk};

mod batch;
mod claim;
mod record;
mod recovery;
pub(super) mod verification;
pub use recovery::Recovery;

struct Current {
    saved: Loaded,
    reservation: Uuid,
    host: Uuid,
    bytes: Vec<u8>,
}

impl Current {
    fn execution(&self) -> Result<&Execution, Error> {
        self.saved
            .snapshot
            .state
            .execution
            .as_ref()
            .ok_or(Error::NotFound)
    }

    fn package(&self) -> Result<Package, Error> {
        let publication = &self.saved.publication;
        let package = Package::decode(
            &self.bytes,
            &publication.catalog,
            &publication.cohort.executor,
        )?;
        if package.digest(&publication.catalog)? != self.execution()?.package
            || package.stage != self.saved.snapshot.state.shop.stage
            || package.cohort != publication.cohort
        {
            return Err(Error::Unavailable);
        }
        Ok(package)
    }
}

impl Store {
    /// Role checked by caller. All mutations use owner -> run -> reservation lock order.
    async fn current(&self, db: &impl ConnectionTrait, run: Uuid) -> Result<Current, Error> {
        uuid7(run)?;
        let row = db
            .query_one_raw(self.statement(
                "SELECT account FROM @tower.runs WHERE id=$1",
                vec![run.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        let account = column(&row, "account")?;
        self.owner(db, account, false).await?;
        let saved = self.locked_run(db, account, run).await?;
        let row = db.query_one_raw(self.statement("SELECT id,host,package FROM @tower.reservations WHERE run=$1 AND stage=$2 FOR UPDATE",
            vec![run.into(), i16::from(saved.snapshot.state.shop.stage).into()])).await.map_err(|_| Error::Unavailable)?.ok_or(Error::NotFound)?;
        let current = Current {
            saved,
            reservation: column(&row, "id")?,
            host: column(&row, "host")?,
            bytes: column(&row, "package")?,
        };
        current.execution()?;
        Ok(current)
    }

    async fn worker(
        &self,
        db: &impl ConnectionTrait,
        principal: &Principal,
        lease: Lease,
    ) -> Result<Current, Error> {
        uuid7(lease.reservation)?;
        uuid7(lease.claim)?;
        self.role(db, principal, Role::Host).await?;
        let row = db
            .query_one_raw(self.statement(
                "SELECT run FROM @tower.reservations WHERE id=$1 AND host=$2",
                vec![lease.reservation.into(), principal.id.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        let current = self.current(db, column(&row, "run")?).await?;
        if current.reservation != lease.reservation
            || current.host != principal.id
            || current.execution()?.fence != lease.fence
        {
            return Err(Refusal::Stale.into());
        }
        db.query_one_raw(self.statement("SELECT claim FROM @tower.execution_claims WHERE reservation=$1 AND claim=$2 AND fence=$3 AND host=$4",
            vec![lease.reservation.into(), lease.claim.into(), signed(lease.fence)?.into(), principal.id.into()]))
            .await.map_err(|_| Error::Unavailable)?.ok_or(Error::NotFound)?;
        Ok(current)
    }

    async fn save_current(
        &self,
        db: &impl ConnectionTrait,
        current: &Current,
    ) -> Result<(), Error> {
        self.save_run(db, current.saved.account, &current.saved.snapshot)
            .await?;
        self.verification_state(db, current).await
    }
}

fn signed(value: u64) -> Result<i64, Error> {
    i64::try_from(value).map_err(|_| Refusal::Capacity.into())
}

fn live(current: &Current, lease: Lease, now: u64) -> Result<(), Error> {
    if current.saved.snapshot.state.phase != Phase::Playing {
        return Err(Refusal::Stale.into());
    }
    // Validation only; do not extend the saved lease or change the run revision.
    current.execution()?.renewed(lease.fence, now, 1)?;
    Ok(())
}
