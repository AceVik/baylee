//! Offline administration. These operations require the schema owner's SQL role.
use super::{ConnectionTrait, Error, Refusal, Store, Uuid, column, uuid7};

impl Store {
    /// Require the exact migrated schema and an effective owner role before offline
    /// administration. Ordinary runtime grants do not imply ownership.
    /// # Errors
    /// Missing history, unknown schema, insufficient SQL role or storage failure.
    pub async fn administration_ready(&self) -> Result<(), Error> {
        self.ready().await?;
        self.require_administrator().await
    }

    async fn require_administrator(&self) -> Result<(), Error> {
        let row = self
            .db
            .query_one_raw(self.statement(
                "SELECT pg_catalog.pg_has_role(current_user,n.nspowner,'USAGE') AS allowed
             FROM pg_catalog.pg_namespace n WHERE n.nspname=$1",
                vec![self.schema.clone().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        if column::<bool>(&row, "allowed")? {
            Ok(())
        } else {
            Err(Error::NotFound)
        }
    }

    /// Revoke an operator without removing records, rewriting outcomes or minting
    /// replacement credentials. Exact repeats remain successful.
    /// # Errors
    /// Non-owner SQL role, malformed/missing identity or storage failure.
    pub async fn revoke_operator(&self, operator: Uuid) -> Result<(), Error> {
        uuid7(operator)?;
        self.require_administrator().await?;
        let changed = self
            .db
            .execute_raw(self.statement(
                "UPDATE @tower.operators SET enabled=false WHERE id=$1",
                vec![operator.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Error::NotFound);
        }
        Ok(())
    }

    /// Stop new enrollment while preserving all owned runs and immutable content.
    /// This cannot reopen a paused cohort or alter its rules.
    /// # Errors
    /// Non-owner SQL role, malformed/missing identity or storage failure.
    pub async fn pause_enrollment(&self, cohort: Uuid) -> Result<(), Error> {
        uuid7(cohort)?;
        self.require_administrator().await?;
        let changed = self
            .db
            .execute_raw(self.statement(
                "UPDATE @tower.cohorts SET accepting=false WHERE id=$1",
                vec![cohort.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Error::NotFound);
        }
        Ok(())
    }

    /// Erase one account's Tower data on an individual request (`docs/privacy.md`
    /// §"Tower authority personal bests"): its echoes leave the pool, then the
    /// player row goes and takes its enrollment, runs, receipts, reservations,
    /// records, settlements, echo snapshots, personal bests and undelivered
    /// awards with it. Other players' duels against its echoes keep their
    /// immutable packages, which name the account as an attribution, and settle
    /// as before; their `opponent_account` stays, since a missing one means a
    /// house opponent. A later signed-in request may enroll the account anew.
    /// # Errors
    /// Non-owner SQL role, malformed identity or storage failure.
    pub async fn erase_account(&self, account: Uuid) -> Result<Erasure, Error> {
        uuid7(account)?;
        self.require_administrator().await?;
        let tx = self.transaction().await?;
        let held = tx
            .query_one_raw(self.statement(
                "SELECT account FROM @tower.players WHERE account=$1 FOR UPDATE",
                vec![account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if held.is_none() {
            tx.commit().await.map_err(|_| Error::Unavailable)?;
            return Ok(Erasure::Absent);
        }
        // A duel the gateway hosts or a verifier replays is left alone: the
        // operator ends it first (`supervise` quarantine, then interrupt). A
        // run reaches these phases again only under this row's lock.
        let busy = tx
            .query_one_raw(self.statement(
                "SELECT EXISTS(SELECT 1 FROM @tower.runs WHERE account=$1
                 AND pg_catalog.convert_from(state,'UTF8')::jsonb->>'phase' IN ('reserved','playing','verifying','quarantined')) AS busy",
                vec![account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        if column::<bool>(&busy, "busy")? {
            tx.commit().await.map_err(|_| Error::Unavailable)?;
            return Ok(Erasure::Busy);
        }
        self.remove_account_echoes(&tx, account).await?;
        let removed = tx
            .execute_raw(self.statement(
                "DELETE FROM @tower.players WHERE account=$1",
                vec![account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if removed.rows_affected() != 1 {
            return Err(Error::Unavailable);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(Erasure::Removed)
    }
}

/// What [`Store::erase_account`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Erasure {
    /// The account's Tower data is gone.
    Removed,
    /// Tower holds nothing for this account: never enrolled, or already
    /// erased. Check the identity before answering a request with it.
    Absent,
    /// A duel of the account is reserved, playing, being verified or
    /// quarantined; nothing changed.
    Busy,
}

/// The reviewed runtime grants (`runtime_grants.sql`), before substitution.
pub const RUNTIME_GRANTS: &str = include_str!("runtime_grants.sql");

impl Store {
    /// Renew the reviewed least-privilege grants of the service's runtime role
    /// (`runtime_grants.sql`): every earlier grant is taken back first, so the
    /// result is exact. Run after every migration, as the schema owner.
    /// # Errors
    /// Non-owner SQL role, an unsafe or owner-member role name, or storage failure.
    pub async fn grant_runtime(&self, role: &str) -> Result<(), Error> {
        if !super::valid_schema(role) {
            return Err(Refusal::Invalid.into());
        }
        self.administration_ready().await?;
        let tx = self.transaction().await?;
        // A runtime role that is (or inherits) the owner would make every
        // grant below meaningless; refuse it rather than pretend.
        let row = tx
            .query_one_raw(self.statement(
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_roles WHERE rolname=$1 AND NOT rolsuper)
                    AND NOT pg_catalog.pg_has_role($1,n.nspowner,'USAGE') AS separate
                 FROM pg_catalog.pg_namespace n WHERE n.nspname=$2",
                vec![role.into(), self.schema.clone().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        if !column::<bool>(&row, "separate")? {
            return Err(Refusal::Invalid.into());
        }
        let sql = self
            .sql(RUNTIME_GRANTS)
            .replace("@runtime", &format!("\"{role}\""));
        tx.execute_unprepared(&sql)
            .await
            .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Refuse to serve with more than the runtime grants: a connection that owns
    /// the schema, or may publish content, add operators or approve origins, is
    /// an administrator's, and the service never runs as one.
    /// # Errors
    /// Owner membership, an administrative table privilege, or storage failure.
    pub async fn runtime_least_privilege(&self) -> Result<(), Error> {
        let row = self
            .db
            .query_one_raw(self.statement(
                "SELECT pg_catalog.pg_has_role(current_user,n.nspowner,'USAGE')
                    OR pg_catalog.has_table_privilege(current_user,$2,'INSERT,UPDATE,DELETE,TRUNCATE')
                    OR pg_catalog.has_table_privilege(current_user,$3,'INSERT,UPDATE,DELETE,TRUNCATE')
                    OR pg_catalog.has_table_privilege(current_user,$4,'INSERT,DELETE,TRUNCATE')
                    OR pg_catalog.has_column_privilege(current_user,$4,'enabled','UPDATE')
                    OR pg_catalog.has_table_privilege(current_user,$5,'INSERT,UPDATE,DELETE,TRUNCATE')
                    OR pg_catalog.has_schema_privilege(current_user,n.oid,'CREATE') AS excess
                 FROM pg_catalog.pg_namespace n WHERE n.nspname=$1",
                vec![
                    self.schema.clone().into(),
                    self.sql("@tower.cohorts").into(),
                    self.sql("@tower.house_builds").into(),
                    self.sql("@tower.operators").into(),
                    self.sql("@tower.host_locations").into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        if column::<bool>(&row, "excess")? {
            Err(Error::Privileged)
        } else {
            Ok(())
        }
    }
}
