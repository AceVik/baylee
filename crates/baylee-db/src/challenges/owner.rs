//! One live reconciler per database/schema: another gateway must not void its games.
use sea_orm::sqlx::{self, ConnectOptions, Connection, PgConnection};
use sea_orm::{DatabaseConnection, DbErr, RuntimeErr};

/// A dedicated connection owns a PostgreSQL advisory lock until closed or dropped.
/// It never borrows a connection from the gateway's request pool.
pub struct Owner(PgConnection);

impl Owner {
    /// Try to own this schema's official challenge runtime. No waiting on another host.
    /// # Errors
    /// Database connectivity or lock failure.
    pub async fn acquire(db: &DatabaseConnection) -> Result<Option<Self>, DbErr> {
        // Keep the same TLS, credentials and URL-configured search_path without
        // serializing them. A long-lived pool checkout would starve ordinary
        // requests when BAYLEE_DB_POOL is small, even while this owner is idle.
        let mut connection = db
            .get_postgres_connection_pool()
            .connect_options()
            .connect()
            .await
            .map_err(database_error)?;
        let owned: bool = sqlx::query_scalar(
            "SELECT pg_try_advisory_lock(hashtext(current_database()), hashtext(current_schema() || '/baylee/challenges/v1'))",
        ).fetch_one(&mut connection).await.map_err(database_error)?;
        if owned {
            Ok(Some(Self(connection)))
        } else {
            Ok(None)
        }
    }

    /// Prove that the connection holding the lock is still alive before reconciliation.
    /// # Errors
    /// A lost connection; the caller must stop admission and owned games.
    pub async fn check(&mut self) -> Result<(), DbErr> {
        sqlx::query("SELECT 1")
            .execute(&mut self.0)
            .await
            .map_err(database_error)?;
        Ok(())
    }

    /// Explicitly release ownership after draining this host.
    /// # Errors
    /// Connection failure (a disconnected database has already released it).
    pub async fn release(self) -> Result<(), DbErr> {
        self.0.close().await.map_err(database_error)
    }
}

/// Keep driver errors typed without exposing the connection options.
fn database_error(error: sqlx::Error) -> DbErr {
    DbErr::Conn(RuntimeErr::SqlxError(error.into()))
}
