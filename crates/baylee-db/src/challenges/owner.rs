//! One live reconciler per database/schema: another gateway must not void its games.
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbErr, Statement, TransactionTrait,
};

/// A dedicated transaction owns a PostgreSQL advisory lock until dropped.
pub struct Owner(DatabaseTransaction);

impl Owner {
    /// Try to own this schema's official challenge runtime. No waiting on another host.
    /// # Errors
    /// Database connectivity or transaction failure.
    pub async fn acquire(db: &DatabaseConnection) -> Result<Option<Self>, DbErr> {
        let transaction = db.begin().await?;
        let row = transaction.query_one_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT pg_try_advisory_xact_lock(hashtext(current_database()), hashtext(current_schema() || '/baylee/challenges/v1')) AS owned",
        )).await?.ok_or_else(|| DbErr::Custom("challenge ownership unavailable".into()))?;
        if row.try_get::<bool>("", "owned")? {
            Ok(Some(Self(transaction)))
        } else {
            Ok(None)
        }
    }

    /// Prove that the connection holding the lock is still alive before reconciliation.
    /// # Errors
    /// A lost transaction/connection; the caller must stop admission and owned games.
    pub async fn check(&self) -> Result<(), DbErr> {
        self.0
            .query_one_raw(Statement::from_string(
                self.0.get_database_backend(),
                "SELECT 1",
            ))
            .await?;
        Ok(())
    }

    /// Explicitly release ownership after draining this host.
    /// # Errors
    /// Rollback/connection failure (a disconnected database has already released it).
    pub async fn release(self) -> Result<(), DbErr> {
        self.0.rollback().await
    }
}
