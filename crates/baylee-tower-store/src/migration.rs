use super::{ConnectionTrait, Digest, Error, Sha256, Store, column};

const DDL: &str = include_str!("schema.sql");

impl Store {
    /// Explicit operator migration, separate from ordinary runtime credentials.
    /// Existing state is never dropped/reset; unknown or changed history fails closed.
    /// # Errors
    /// Unsupported PostgreSQL, privileges, lock timeout or mismatched schema history.
    pub async fn migrate(&self) -> Result<(), Error> {
        let tx = self.transaction().await?;
        let version = tx
            .query_one_raw(self.statement(
                "SELECT current_setting('server_version_num')::integer AS version",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Schema)?;
        if column::<i32>(&version, "version")? < 180_000 {
            return Err(Error::Schema);
        }
        tx.execute_raw(self.statement(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            vec![format!("baylee/tower/schema/{}", self.schema).into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.execute_unprepared(&self.sql("CREATE SCHEMA IF NOT EXISTS @tower;
            CREATE TABLE IF NOT EXISTS @tower.schema_version (version integer PRIMARY KEY, digest bytea NOT NULL CHECK(octet_length(digest)=32))"))
            .await.map_err(|_| Error::Unavailable)?;
        let rows = tx
            .query_all_raw(self.statement(
                "SELECT version,digest FROM @tower.schema_version ORDER BY version",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let digest = Sha256::digest(DDL.as_bytes()).to_vec();
        if rows.is_empty() {
            tx.execute_unprepared(&self.sql(DDL))
                .await
                .map_err(|_| Error::Unavailable)?;
            tx.execute_raw(self.statement(
                "INSERT INTO @tower.schema_version(version,digest) VALUES(1,$1)",
                vec![digest.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        } else if rows.len() != 1
            || column::<i32>(&rows[0], "version")? != 1
            || column::<Vec<u8>>(&rows[0], "digest")? != digest
        {
            return Err(Error::Schema);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Runtime readiness checks the exact migration without changing the schema.
    /// # Errors
    /// Missing/unknown history or database failure.
    pub async fn ready(&self) -> Result<(), Error> {
        let rows = self
            .db
            .query_all_raw(self.statement(
                "SELECT version,digest FROM @tower.schema_version ORDER BY version",
                vec![],
            ))
            .await
            .map_err(|_| Error::Schema)?;
        if rows.len() != 1
            || column::<i32>(&rows[0], "version")? != 1
            || column::<Vec<u8>>(&rows[0], "digest")? != Sha256::digest(DDL.as_bytes()).as_slice()
        {
            return Err(Error::Schema);
        }
        Ok(())
    }
}
