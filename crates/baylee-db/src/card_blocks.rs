//! Blocked cards (owner, 12.10.2026): made and lifted through the admin
//! console, read by the gateway at start and after every change.
//!
//! Hand-written statements, as in [`crate::invites`]. A block is in force
//! while it is not lifted and its `until`, if it has one, is still ahead;
//! nothing sweeps an expired one, it simply stops counting. Rows are kept
//! after they end: the list is also the history.

use sea_orm::{ConnectionTrait, DbErr, Statement};
use time::OffsetDateTime;
use uuid::Uuid;

/// The longest reason a block may give, in characters (the table's bound).
pub const MAX_REASON_CHARS: usize = 200;

/// A block about to be stored.
#[derive(Clone, Debug)]
pub struct NewBlock {
    /// The card's `CardIndex`.
    pub card_index: u32,
    /// Its English name now.
    pub card_name: String,
    /// Why, in the admin's words: 1 to [`MAX_REASON_CHARS`] characters.
    pub reason: String,
    /// The admin who blocked it.
    pub created_by: String,
    /// When it ends by itself; `None` for never.
    pub until: Option<OffsetDateTime>,
}

/// A stored block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardBlock {
    /// Its id, which lifting it takes.
    pub id: Uuid,
    /// The card's `CardIndex`.
    pub card_index: u32,
    /// The card's English name when it was blocked.
    pub card_name: String,
    /// Why.
    pub reason: String,
    /// When it was made.
    pub created_at: OffsetDateTime,
    /// By whom.
    pub created_by: String,
    /// When it ends by itself, if ever.
    pub until: Option<OffsetDateTime>,
    /// When it was lifted, if it was.
    pub lifted_at: Option<OffsetDateTime>,
    /// By whom.
    pub lifted_by: Option<String>,
}

impl CardBlock {
    /// Whether it is in force at `now`: not lifted, and not past its end.
    #[must_use]
    pub fn in_force(&self, now: OffsetDateTime) -> bool {
        self.lifted_at.is_none() && self.until.is_none_or(|until| until > now)
    }
}

const COLUMNS: &str =
    "id, card_index, card_name, reason, created_at, created_by, until, lifted_at, lifted_by";

fn read(row: &sea_orm::QueryResult) -> Result<CardBlock, DbErr> {
    let index: i32 = row.try_get("", "card_index")?;
    Ok(CardBlock {
        id: row.try_get("", "id")?,
        card_index: u32::try_from(index)
            .map_err(|_| DbErr::Custom("a negative card index".into()))?,
        card_name: row.try_get("", "card_name")?,
        reason: row.try_get("", "reason")?,
        created_at: row.try_get("", "created_at")?,
        created_by: row.try_get("", "created_by")?,
        until: row.try_get("", "until")?,
        lifted_at: row.try_get("", "lifted_at")?,
        lifted_by: row.try_get("", "lifted_by")?,
    })
}

/// Stores a block, answering it; `None` when the card already has a block
/// in force at `now`, which has to be lifted first.
///
/// One statement, but not serialized against another one racing it: two
/// admins blocking the same card in the same instant may both succeed
/// (`READ COMMITTED`), and the card then stays blocked until both blocks
/// are lifted. Nothing worse follows from it.
///
/// # Errors
///
/// When the insert fails: a reason that is empty or over
/// [`MAX_REASON_CHARS`], an index over `i32::MAX`.
pub async fn create(
    db: &impl ConnectionTrait,
    new: &NewBlock,
    now: OffsetDateTime,
) -> Result<Option<CardBlock>, DbErr> {
    let index = i32::try_from(new.card_index)
        .map_err(|_| DbErr::Custom("a card index over i32::MAX".into()))?;
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "INSERT INTO card_block (card_index, card_name, reason, created_by, until) \
                 SELECT $1, $2, $3, $4, $5 \
                 WHERE NOT EXISTS ( \
                     SELECT 1 FROM card_block \
                     WHERE card_index = $1 AND lifted_at IS NULL \
                       AND (until IS NULL OR until > $6)) \
                 RETURNING {COLUMNS}"
            ),
            [
                index.into(),
                new.card_name.clone().into(),
                new.reason.clone().into(),
                new.created_by.clone().into(),
                new.until.into(),
                now.into(),
            ],
        ))
        .await?;
    row.as_ref().map(read).transpose()
}

/// Every block ever made, newest first.
///
/// # Errors
///
/// When the query fails.
pub async fn list(db: &impl ConnectionTrait) -> Result<Vec<CardBlock>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            format!("SELECT {COLUMNS} FROM card_block ORDER BY created_at DESC, id DESC"),
        ))
        .await?;
    rows.iter().map(read).collect()
}

/// The blocks in force at `now`, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn in_force(
    db: &impl ConnectionTrait,
    now: OffsetDateTime,
) -> Result<Vec<CardBlock>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "SELECT {COLUMNS} FROM card_block \
                 WHERE lifted_at IS NULL AND (until IS NULL OR until > $1) \
                 ORDER BY created_at, id"
            ),
            [now.into()],
        ))
        .await?;
    rows.iter().map(read).collect()
}

/// Lifts the block `id` as of `now`, by `by`. Answers the block as it now
/// stands; `None` for no such block, or one lifted or ended already, which
/// stays as it was.
///
/// # Errors
///
/// When the update fails.
pub async fn lift(
    db: &impl ConnectionTrait,
    id: Uuid,
    by: &str,
    now: OffsetDateTime,
) -> Result<Option<CardBlock>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "UPDATE card_block SET lifted_at = $2, lifted_by = $3 \
                 WHERE id = $1 AND lifted_at IS NULL AND (until IS NULL OR until > $2) \
                 RETURNING {COLUMNS}"
            ),
            [id.into(), now.into(), by.into()],
        ))
        .await?;
    row.as_ref().map(read).transpose()
}
