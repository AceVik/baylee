//! Durable Tower authority, separate from gateway and Central databases.
//! Callers must authenticate account/operator roles before entering this adapter.
//! Account IDs, UUIDs, content hashes and these Rust methods are not credentials.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use baylee_tower::{
    Refusal,
    cards::{Catalog, Collection},
    v2::{
        package::Cohort,
        run::{Command, Phase, Run},
    },
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, QueryResult, Statement,
    TransactionTrait, Value,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use uuid::Uuid;

mod commands;
mod migration;
mod publication;
mod requests;
mod runs;
pub use publication::Publication;
pub use runs::Start;

/// Maximum persisted owner state or immutable cohort publication.
pub const MAX_STATE_BYTES: usize = 128 * 1024;
/// Maximum saved shop/start receipts per run; record chunks use a separate stream.
pub const MAX_REQUESTS: i64 = 4096;
/// Version of the visible enrollment explanation; not implicit from authentication.
pub const CONSENT_REVISION: u16 = 1;

/// Deliberately value-free failures. Database messages never cross this boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Domain operation refused without mutation.
    #[error("Tower command refused: {0}")]
    Refused(#[from] Refusal),
    /// Missing row or wrong account; deliberately indistinguishable.
    #[error("Tower state unavailable")]
    NotFound,
    /// Reused request with different content or an existing active run.
    #[error("Tower request conflicts")]
    Conflict,
    /// Unknown/mismatched schema or unsupported database.
    #[error("Tower database schema incompatible")]
    Schema,
    /// Database/serialization fault; no credential or private payload included.
    #[error("Tower storage unavailable")]
    Unavailable,
}

/// Owner response. Never contains shop randomness or private opponent packages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Database-generated run `UUIDv7`.
    pub id: Uuid,
    /// Immutable published cohort identity.
    pub cohort: Uuid,
    /// The owner's saved state.
    pub state: Run,
}

/// Shared pool plus one explicitly isolated schema. No automatic migration at runtime.
#[derive(Clone)]
pub struct Store {
    db: DatabaseConnection,
    schema: String,
}

impl Store {
    /// Select an operator-configured schema. This grants no database privileges.
    /// The connection must have SQL/value logging disabled and bounded timeouts/pool.
    /// # Errors
    /// Non-PostgreSQL backend or unsafe schema identifier.
    pub fn new(db: DatabaseConnection, schema: &str) -> Result<Self, Error> {
        if db.get_database_backend() != DbBackend::Postgres || !schema_name(schema) {
            return Err(Error::Schema);
        }
        Ok(Self {
            db,
            schema: schema.into(),
        })
    }

    fn sql(&self, text: &str) -> String {
        text.replace("@tower", &format!("\"{}\"", self.schema))
    }

    fn statement(&self, text: &str, values: Vec<Value>) -> Statement {
        Statement::from_sql_and_values(DbBackend::Postgres, self.sql(text), values)
    }

    async fn transaction(&self) -> Result<DatabaseTransaction, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        tx.execute_unprepared("SET LOCAL statement_timeout = '5s'; SET LOCAL lock_timeout = '2s'")
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(tx)
    }
}

fn schema_name(name: &str) -> bool {
    (1..=63).contains(&name.len())
        && name.as_bytes()[0].is_ascii_lowercase()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn uuid7(id: Uuid) -> Result<(), Error> {
    let bytes = id.as_bytes();
    if bytes[6] >> 4 != 7 || bytes[8] >> 6 != 2 {
        return Err(Refusal::Invalid.into());
    }
    Ok(())
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>, Error> {
    let bytes = serde_json::to_vec(value).map_err(|_| Error::Unavailable)?;
    if bytes.len() > MAX_STATE_BYTES {
        return Err(Refusal::Capacity.into());
    }
    Ok(bytes)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    if bytes.len() > MAX_STATE_BYTES {
        return Err(Error::Unavailable);
    }
    serde_json::from_slice(bytes).map_err(|_| Error::Unavailable)
}

fn column<T: sea_orm::TryGetable>(row: &QueryResult, name: &str) -> Result<T, Error> {
    row.try_get("", name).map_err(|_| Error::Unavailable)
}
