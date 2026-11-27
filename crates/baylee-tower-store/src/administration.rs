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
