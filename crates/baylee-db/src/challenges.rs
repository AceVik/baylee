//! Private claim bindings and immutable result outbox for the official host.
//! Account deletion cascades; delivered rows are removed. No player route reads this table.
use central_api::challenges::Completion;
use sea_orm::{ConnectionTrait, DbErr, QueryResult, Statement, Value};
use uuid::Uuid;
mod owner;
pub use owner::Owner;

/// One pending delivery or running claim, without execution secrets.
pub struct Run {
    /// Central's attempt identity.
    pub attempt: Uuid,
    /// The game proposed before claiming, retained on retry.
    pub game: Uuid,
    /// Seconds since reservation, independent of delivery queue rotation.
    pub age: i64,
    /// Immutable engine outcome, once durably recorded.
    pub result: Option<Completion>,
}

fn stmt(db: &impl ConnectionTrait, sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(db.get_database_backend(), sql, values)
}

/// Reserve the local binding before the remote claim. Returns its original game on retry.
/// # Errors
/// Database failure or an attempt bound to another account.
pub async fn reserve(
    db: &impl ConnectionTrait,
    attempt: Uuid,
    game: Uuid,
    account: Uuid,
    central: Uuid,
) -> Result<Uuid, DbErr> {
    let row = db
        .query_one_raw(stmt(
            db,
            "INSERT INTO challenge_run(attempt,game,account,central_account) SELECT $1,$2,$3,$4
         WHERE (SELECT count(*) FROM challenge_run WHERE account=$3)<2
            OR EXISTS(SELECT 1 FROM challenge_run WHERE attempt=$1 AND account=$3)
         ON CONFLICT(attempt) DO UPDATE SET touched_at=clock_timestamp()
         WHERE challenge_run.account=$3 AND challenge_run.central_account=$4
         RETURNING game",
            vec![attempt.into(), game.into(), account.into(), central.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("challenge account mismatch".into()))?;
    row.try_get("", "game")
}

/// Persist one outcome before trying the network. Exact retries are idempotent.
/// # Errors
/// Database failure, missing claim or conflicting outcome.
pub async fn complete(db: &impl ConnectionTrait, result: &Completion) -> Result<(), DbErr> {
    result
        .validate()
        .map_err(|_| DbErr::Custom("invalid challenge outcome".into()))?;
    let encoded = central_api::encode(result)
        .map_err(|_| DbErr::Custom("invalid challenge outcome".into()))?;
    let wrote = db
        .execute_raw(stmt(
            db,
            "UPDATE challenge_run SET result=$3, touched_at=clock_timestamp()
         WHERE attempt=$1 AND game=$2 AND (result IS NULL OR result=$3)",
            vec![
                Uuid::from_bytes(result.attempt).into(),
                Uuid::from_bytes(result.game).into(),
                encoded.into(),
            ],
        ))
        .await?;
    if wrote.rows_affected() != 1 {
        return Err(DbErr::Custom("challenge result conflict".into()));
    }
    Ok(())
}

/// Burn the launch permission before revealing the opening. A crashed game is never restarted from its beginning under the same attempt.
/// # Errors
/// Database failure.
pub async fn launch_once(
    db: &impl ConnectionTrait,
    attempt: Uuid,
    game: Uuid,
) -> Result<bool, DbErr> {
    Ok(db
        .execute_raw(stmt(
            db,
            "UPDATE challenge_run SET launched=true,touched_at=clock_timestamp()
         WHERE attempt=$1 AND game=$2 AND NOT launched AND result IS NULL",
            vec![attempt.into(), game.into()],
        ))
        .await?
        .rows_affected()
        == 1)
}

/// Read the oldest bounded batch; touching serviced rows rotates fairly through all runs.
/// # Errors
/// Database failure or corrupt stored outcome.
pub async fn pending(db: &impl ConnectionTrait) -> Result<Vec<Run>, DbErr> {
    db.query_all_raw(stmt(
        db,
        "SELECT attempt,game,result,extract(epoch FROM clock_timestamp()-created_at)::bigint AS age
         FROM challenge_run ORDER BY touched_at,attempt LIMIT 32",
        vec![],
    ))
    .await?
    .iter()
    .map(decode)
    .collect()
}

/// Re-read a queued identity after acquiring the host's admission lock.
/// # Errors
/// Database failure or corrupt stored outcome.
pub async fn find(db: &impl ConnectionTrait, attempt: Uuid) -> Result<Option<Run>, DbErr> {
    db.query_one_raw(stmt(
        db,
        "SELECT attempt,game,result,extract(epoch FROM clock_timestamp()-created_at)::bigint AS age
         FROM challenge_run WHERE attempt=$1",
        vec![attempt.into()],
    ))
    .await?
    .as_ref()
    .map(decode)
    .transpose()
}

fn decode(row: &QueryResult) -> Result<Run, DbErr> {
    let bytes: Option<Vec<u8>> = row.try_get("", "result")?;
    Ok(Run {
        attempt: row.try_get("", "attempt")?,
        game: row.try_get("", "game")?,
        age: row.try_get("", "age")?,
        result: bytes
            .map(|bytes| {
                central_api::decode(&bytes)
                    .map_err(|_| DbErr::Custom("invalid challenge outcome".into()))
            })
            .transpose()?,
    })
}

/// Move a serviced run to the end of the bounded delivery queue.
/// # Errors
/// Database failure.
pub async fn touch(db: &impl ConnectionTrait, attempt: Uuid) -> Result<(), DbErr> {
    db.execute_raw(stmt(
        db,
        "UPDATE challenge_run SET touched_at=clock_timestamp() WHERE attempt=$1",
        vec![attempt.into()],
    ))
    .await?;
    Ok(())
}

/// Forget only after Central confirms the terminal state (or no claim ever started).
/// # Errors
/// Database failure.
pub async fn delivered(db: &impl ConnectionTrait, attempt: Uuid) -> Result<(), DbErr> {
    db.execute_raw(stmt(
        db,
        "DELETE FROM challenge_run WHERE attempt=$1",
        vec![attempt.into()],
    ))
    .await?;
    Ok(())
}

/// Remove a rejected remote claim only when it never launched and has no outcome.
/// # Errors
/// Database failure.
pub async fn discard_unstarted(db: &impl ConnectionTrait, attempt: Uuid) -> Result<(), DbErr> {
    db.execute_raw(stmt(
        db,
        "DELETE FROM challenge_run WHERE attempt=$1 AND NOT launched AND result IS NULL",
        vec![attempt.into()],
    ))
    .await?;
    Ok(())
}
