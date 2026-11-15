use super::{ConnectionTrait, Digest, Error, QueryResult, Sha256, Store, column};

const MIGRATIONS: &[&str] = &[
    include_str!("schema.sql"),
    include_str!("migrations/002_account_bounds.sql"),
    include_str!("migrations/003_operators.sql"),
    include_str!("migrations/004_execution.sql"),
    include_str!("migrations/005_verification.sql"),
    include_str!("migrations/006_echo_pool.sql"),
    include_str!("migrations/007_verification_failure.sql"),
    include_str!("migrations/008_record_batches.sql"),
    include_str!("migrations/009_execution_watch.sql"),
];

impl Store {
    /// Explicit operator migration, separate from ordinary runtime credentials.
    /// Existing state is never dropped/reset; unknown or changed history fails closed.
    /// # Errors
    /// Unsupported PostgreSQL, privileges, lock timeout or mismatched schema history.
    pub async fn migrate(&self) -> Result<(), Error> {
        let tx = self.transaction().await?;
        let version = tx
            .query_one_raw(self.statement(
                "SELECT pg_catalog.current_setting('server_version_num')::integer AS version",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Schema)?;
        if column::<i32>(&version, "version")? < 180_000 {
            return Err(Error::Schema);
        }
        tx.execute_raw(self.statement(
            "SELECT pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended($1, 0))",
            vec![format!("baylee/tower/schema/{}", self.schema).into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.execute_unprepared(&self.sql("CREATE SCHEMA IF NOT EXISTS @tower;
            CREATE TABLE IF NOT EXISTS @tower.schema_version (version integer PRIMARY KEY, digest bytea NOT NULL CHECK(pg_catalog.octet_length(digest)=32))"))
            .await.map_err(|_| Error::Unavailable)?;
        let rows = self.history(&tx).await?;
        checked(&rows)?;
        for (index, sql) in MIGRATIONS.iter().enumerate().skip(rows.len()) {
            tx.execute_unprepared(&self.sql(sql))
                .await
                .map_err(|_| Error::Unavailable)?;
            tx.execute_raw(self.statement(
                "INSERT INTO @tower.schema_version(version,digest) VALUES($1,$2)",
                vec![
                    i32::try_from(index + 1).map_err(|_| Error::Schema)?.into(),
                    Sha256::digest(sql.as_bytes()).to_vec().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    async fn history(&self, db: &impl ConnectionTrait) -> Result<Vec<QueryResult>, Error> {
        db.query_all_raw(self.statement(
            "SELECT version,digest FROM @tower.schema_version ORDER BY version LIMIT $1",
            vec![i64::try_from(MIGRATIONS.len() + 1).map_err(|_| Error::Schema)?.into()],
        ))
        .await
        .map_err(|_| Error::Schema)
    }

    /// Runtime readiness checks the exact complete migration sequence without writes.
    /// # Errors
    /// Missing/unknown history or database failure.
    pub async fn ready(&self) -> Result<(), Error> {
        let rows = self.history(&self.db).await?;
        if rows.len() != MIGRATIONS.len() {
            return Err(Error::Schema);
        }
        checked(&rows)
    }
}

fn checked(rows: &[QueryResult]) -> Result<(), Error> {
    if rows.len() > MIGRATIONS.len() {
        return Err(Error::Schema);
    }
    for (index, row) in rows.iter().enumerate() {
        if column::<i32>(row, "version")? != i32::try_from(index + 1).map_err(|_| Error::Schema)?
            || column::<Vec<u8>>(row, "digest")?
                != Sha256::digest(MIGRATIONS[index].as_bytes()).as_slice()
        {
            return Err(Error::Schema);
        }
    }
    Ok(())
}
