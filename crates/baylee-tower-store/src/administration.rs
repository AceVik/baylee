//! Offline administration. These operations require the schema owner's SQL role.
use super::{ConnectionTrait, Error, Store, Uuid, column, uuid7};

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
