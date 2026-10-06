//! Game records (#315): what a game's engine sent of its record, stored as it
//! came, and read back only for a player who sat at the game.
//!
//! Hand-written statements rather than entities: three tables the gateway
//! only appends to and reads whole, where the one thing worth having is that
//! a piece is added and counted in one statement.
//!
//! Nothing a seat or the lobby can ask reaches these tables. The one reader
//! is [`for_seated`], which the gateway calls for a bug report from a player
//! who sat at the game, and which answers nothing for anyone else.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement, TransactionTrait, Value};
use uuid::Uuid;

/// Who sat in one seat of a recorded game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seat {
    /// The account that played it; `None` for a chair the house or a
    /// host's seat bridge played.
    pub account: Option<Uuid>,
    /// The host whose seat bridge played it (a language model at its
    /// table), who answers for the seat without having played it.
    pub delegated_by: Option<Uuid>,
}

impl Seat {
    /// A seat an account played.
    #[must_use]
    pub const fn played_by(account: Uuid) -> Self {
        Self {
            account: Some(account),
            delegated_by: None,
        }
    }
}

/// Starts a game's record: its row, and who sat in which seat (nobody, for
/// a chair the house plays). Starting one that exists changes nothing, so an
/// engine that attaches twice cannot rewrite who sat where.
///
/// # Errors
///
/// When a statement fails.
pub async fn open(db: &DatabaseConnection, game_id: &str, seats: &[Seat]) -> Result<(), DbErr> {
    let txn = db.begin().await?;
    let backend = txn.get_database_backend();
    let made = txn
        .execute_raw(Statement::from_sql_and_values(
            backend,
            "INSERT INTO game_record (game_id) VALUES ($1) ON CONFLICT DO NOTHING",
            [game_id.into()],
        ))
        .await?;
    if made.rows_affected() == 1 {
        for (seat, who) in seats.iter().enumerate() {
            let seat =
                i16::try_from(seat).map_err(|_| DbErr::Custom("seat out of range".into()))?;
            txn.execute_raw(Statement::from_sql_and_values(
                backend,
                "INSERT INTO game_record_seat (game_id, seat, account_id, delegated_by) \
                 VALUES ($1, $2, $3, $4)",
                [
                    game_id.into(),
                    seat.into(),
                    Value::Uuid(who.account),
                    Value::Uuid(who.delegated_by),
                ],
            ))
            .await?;
        }
    }
    txn.commit().await
}

/// Adds piece `seq` of a game's record; the last piece marks it complete.
///
/// Returns whether it was added: a piece already stored, or one for a game
/// whose record was never started, is not.
///
/// # Errors
///
/// When the statement fails.
pub async fn append(
    db: &impl ConnectionTrait,
    game_id: &str,
    seq: u32,
    data: Vec<u8>,
    last: bool,
) -> Result<bool, DbErr> {
    let seq = i32::try_from(seq).map_err(|_| DbErr::Custom("piece out of range".into()))?;
    let done = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "WITH piece AS ( \
                 INSERT INTO game_record_chunk (game_id, seq, data) \
                 SELECT game_id, $2, $3 FROM game_record WHERE game_id = $1 \
                 ON CONFLICT DO NOTHING \
                 RETURNING octet_length(data) AS n) \
             UPDATE game_record r SET \
                 bytes = r.bytes + piece.n, \
                 complete = r.complete OR $4, \
                 ended_at = CASE WHEN $4 THEN now() ELSE r.ended_at END \
             FROM piece WHERE r.game_id = $1",
            [game_id.into(), seq.into(), data.into(), last.into()],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// A game's record as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The pieces in order, one gzip stream.
    pub data: Vec<u8>,
    /// Whether the last piece arrived.
    pub complete: bool,
}

/// The record of `game_id`, if `account` sat at that game; `None` for
/// anyone else, and for a game with no record.
///
/// # Errors
///
/// When the statement fails.
pub async fn for_seated(
    db: &impl ConnectionTrait,
    game_id: &str,
    account: Uuid,
) -> Result<Option<Record>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT r.complete, \
                 coalesce((SELECT string_agg(c.data, ''::bytea ORDER BY c.seq) \
                           FROM game_record_chunk c WHERE c.game_id = r.game_id), ''::bytea) \
                   AS data \
             FROM game_record r \
             WHERE r.game_id = $1 AND EXISTS ( \
                 SELECT 1 FROM game_record_seat s \
                 WHERE s.game_id = r.game_id AND s.account_id = $2)",
            [game_id.into(), account.into()],
        ))
        .await?;
    row.map(|row| {
        Ok(Record {
            data: row.try_get("", "data")?,
            complete: row.try_get("", "complete")?,
        })
    })
    .transpose()
}
