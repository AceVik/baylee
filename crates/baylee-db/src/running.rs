//! Running games the gateway keeps across its own restart
//! (`docs/proposals/games-survive-restarts.md` §"Step 3b"): one row a game,
//! the gateway's own JSON at its own version, of a game whose record is
//! started and not complete.
//!
//! Hand-written statements, as for the records beside them: the gateway
//! writes a game's whole row each time and reads every row once, as it
//! starts. The body is the gateway's business; this module never reads
//! inside it. Seat token hashes and session keys are in it, so nothing a
//! player can ask reaches this table.

use sea_orm::{ConnectionTrait, DbErr, Statement};

/// The most rows a start reads: far more games than one gateway runs, and
/// the bound on what a damaged table can make it hold.
pub const MAX_RUNNING: u64 = 100_000;

/// One kept game, as it was last written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Running {
    /// The game.
    pub game_id: String,
    /// The version of `body`'s shape.
    pub kept: i32,
    /// The gateway's JSON.
    pub body: String,
}

/// Writes `game_id`'s row: `body` at version `kept`, replacing what was
/// there. Nothing is written, and `false` comes back, for a game whose
/// record is not started or already complete: a write that comes after the
/// game ended does not bring it back.
///
/// # Errors
///
/// When the statement fails, `body` included when it is not JSON.
pub async fn keep(
    db: &impl ConnectionTrait,
    game_id: &str,
    kept: i32,
    body: &str,
) -> Result<bool, DbErr> {
    let done = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO running_game (game_id, kept, body) \
             SELECT r.game_id, $2, $3::jsonb FROM game_record r \
             WHERE r.game_id = $1 AND NOT r.complete \
             ON CONFLICT (game_id) DO UPDATE SET \
                 kept = EXCLUDED.kept, body = EXCLUDED.body, written_at = now()",
            [game_id.into(), kept.into(), body.into()],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Removes `game_id`'s row, if it has one.
///
/// # Errors
///
/// When the statement fails.
pub async fn forget(db: &impl ConnectionTrait, game_id: &str) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM running_game WHERE game_id = $1",
        [game_id.into()],
    ))
    .await?;
    Ok(())
}

/// Every kept game whose record is not complete, oldest first, at most
/// [`MAX_RUNNING`]. A row whose record completed is left out here and goes
/// with [`forget_finished`].
///
/// # Errors
///
/// When the statement fails.
pub async fn all(db: &impl ConnectionTrait) -> Result<Vec<Running>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT k.game_id, k.kept, k.body::text AS body FROM running_game k \
             JOIN game_record r ON r.game_id = k.game_id \
             WHERE NOT r.complete ORDER BY k.game_id LIMIT $1",
            [i64::try_from(MAX_RUNNING).unwrap_or(i64::MAX).into()],
        ))
        .await?;
    rows.into_iter()
        .map(|row| {
            Ok(Running {
                game_id: row.try_get("", "game_id")?,
                kept: row.try_get("", "kept")?,
                body: row.try_get("", "body")?,
            })
        })
        .collect()
}

/// Removes the rows of games whose record is complete: a row is removed with
/// its record's last piece, so this finds only what an older gateway left.
/// How many went.
///
/// # Errors
///
/// When the statement fails.
pub async fn forget_finished(db: &impl ConnectionTrait) -> Result<u64, DbErr> {
    let done = db
        .execute_raw(Statement::from_string(
            db.get_database_backend(),
            "DELETE FROM running_game k USING game_record r \
             WHERE r.game_id = k.game_id AND r.complete",
        ))
        .await?;
    Ok(done.rows_affected())
}

/// Those of `accounts` that no longer exist: a kept chair of one of them is
/// emptied before its game is resumed, as the deletion would have done to
/// it in a gateway that was running.
///
/// # Errors
///
/// When the statement fails.
pub async fn gone_accounts(
    db: &impl ConnectionTrait,
    accounts: &[uuid::Uuid],
) -> Result<Vec<uuid::Uuid>, DbErr> {
    if accounts.is_empty() {
        return Ok(Vec::new());
    }
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT x.id FROM unnest($1::uuid[]) AS x(id) \
             WHERE NOT EXISTS (SELECT 1 FROM account a WHERE a.id = x.id)",
            [accounts.to_vec().into()],
        ))
        .await?;
    rows.into_iter().map(|row| row.try_get("", "id")).collect()
}
