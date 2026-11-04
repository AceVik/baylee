//! Ban lists (`docs/banlists-and-formats.md`): the official ones, kept in
//! step with the card feed, and a player's own.
//!
//! Hand-written statements, as in [`crate::card_blocks`]; every question that
//! could reach another account's list is asked with the account in it, so a
//! list that is not the caller's and one that does not exist answer alike.
//! Nothing here reads a card: what a status means is
//! `baylee_cards::formats::validate`.

use baylee_cardtext::{ListStatus, MAX_LIST_ENTRIES, MAX_LISTS_PER_ACCOUNT};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement, TransactionTrait, Value};
use time::OffsetDateTime;
use uuid::Uuid;

/// A stored list, without its cards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Banlist {
    /// Its id.
    pub id: Uuid,
    /// What it is called.
    pub name: String,
    /// The format it is for: Scryfall's key, or `freeform`.
    pub format: String,
    /// Whether it is an official list, which nobody edits.
    pub builtin: bool,
    /// Whose it is; `None` for an official list.
    pub owner: Option<Uuid>,
    /// Counts up with every change to its cards or its format.
    pub version: i64,
    /// When it last changed.
    pub updated_at: OffsetDateTime,
    /// How many cards it names.
    pub cards: u32,
}

/// One card on a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    /// The card's oracle id.
    pub oracle_id: Uuid,
    /// What the list says about it.
    pub status: Status,
}

/// [`ListStatus`] with an order, so a list's entries sort the same way twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Out of every deck.
    Banned,
    /// One copy.
    Restricted,
    /// In the deck, but not leading it.
    BannedAsCommander,
}

impl From<ListStatus> for Status {
    fn from(status: ListStatus) -> Self {
        match status {
            ListStatus::Banned => Self::Banned,
            ListStatus::Restricted => Self::Restricted,
            ListStatus::BannedAsCommander => Self::BannedAsCommander,
        }
    }
}

impl From<Status> for ListStatus {
    fn from(status: Status) -> Self {
        match status {
            Status::Banned => Self::Banned,
            Status::Restricted => Self::Restricted,
            Status::BannedAsCommander => Self::BannedAsCommander,
        }
    }
}

/// What came of asking for a new list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Created {
    /// It stands.
    Made(Banlist),
    /// The account holds [`MAX_LISTS_PER_ACCOUNT`] already.
    TooMany,
    /// The list to copy is not there for this account.
    NoSuchList,
}

/// What came of changing a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Changed {
    /// It stands so now.
    Made(Banlist),
    /// No list of this account's: another's, or none, and they read alike.
    NoSuchList,
    /// An official list, which nobody edits.
    Official,
    /// More cards than a list may hold.
    TooManyCards,
}

/// A change to an own list; an absent field leaves that part alone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Edit {
    /// Its new name.
    pub name: Option<String>,
    /// Its new format.
    pub format: Option<String>,
    /// Every card it now names.
    pub entries: Option<Vec<Entry>>,
}

const COLUMNS: &str = "b.id, b.name, b.format, b.builtin, b.owner, b.version, b.updated_at, \
     (SELECT count(*) FROM banlist_entries e WHERE e.banlist_id = b.id) AS cards";

fn statement(db: &impl ConnectionTrait, sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(db.get_database_backend(), sql, values)
}

fn read(row: &sea_orm::QueryResult) -> Result<Banlist, DbErr> {
    let cards: i64 = row.try_get("", "cards")?;
    Ok(Banlist {
        id: row.try_get("", "id")?,
        name: row.try_get("", "name")?,
        format: row.try_get("", "format")?,
        builtin: row.try_get("", "builtin")?,
        owner: row.try_get("", "owner")?,
        version: row.try_get("", "version")?,
        updated_at: row.try_get("", "updated_at")?,
        cards: u32::try_from(cards).unwrap_or(u32::MAX),
    })
}

fn read_entry(row: &sea_orm::QueryResult) -> Result<Entry, DbErr> {
    let word: String = row.try_get("", "status")?;
    let status = ListStatus::of(&word)
        .ok_or_else(|| DbErr::Custom(format!("a list status nobody wrote: {word}")))?;
    Ok(Entry {
        oracle_id: row.try_get("", "oracle_id")?,
        status: status.into(),
    })
}

/// Every official list, then this account's own, by name.
///
/// # Errors
///
/// When the database fails.
pub async fn listed(db: &impl ConnectionTrait, account: Uuid) -> Result<Vec<Banlist>, DbErr> {
    db.query_all_raw(statement(
        db,
        &format!(
            "SELECT {COLUMNS} FROM banlists b \
             WHERE b.builtin OR b.owner = $1 \
             ORDER BY b.builtin DESC, b.name, b.id"
        ),
        vec![account.into()],
    ))
    .await?
    .iter()
    .map(read)
    .collect()
}

/// Which cards the lists [`listed`] answers name, as `(list, oracle id)`, for
/// a search to read without asking for each list.
///
/// # Errors
///
/// When the database fails.
pub async fn listed_cards(
    db: &impl ConnectionTrait,
    account: Uuid,
) -> Result<Vec<(Uuid, Uuid)>, DbErr> {
    db.query_all_raw(statement(
        db,
        "SELECT e.banlist_id, e.oracle_id FROM banlist_entries e \
         JOIN banlists b ON b.id = e.banlist_id \
         WHERE b.builtin OR b.owner = $1",
        vec![account.into()],
    ))
    .await?
    .iter()
    .map(|row| {
        Ok((
            row.try_get("", "banlist_id")?,
            row.try_get("", "oracle_id")?,
        ))
    })
    .collect()
}

/// The list `id` if `account` may read it: an official one or its own.
///
/// # Errors
///
/// When the database fails.
pub async fn readable(
    db: &impl ConnectionTrait,
    id: Uuid,
    account: Uuid,
) -> Result<Option<Banlist>, DbErr> {
    db.query_one_raw(statement(
        db,
        &format!(
            "SELECT {COLUMNS} FROM banlists b \
             WHERE b.id = $1 AND (b.builtin OR b.owner = $2)"
        ),
        vec![id.into(), account.into()],
    ))
    .await?
    .as_ref()
    .map(read)
    .transpose()
}

/// The list `id` whoever asks, for a room that already holds it: whether a
/// caller may see it is the room's question, not this one's.
///
/// # Errors
///
/// When the database fails.
pub async fn held(db: &impl ConnectionTrait, id: Uuid) -> Result<Option<Banlist>, DbErr> {
    db.query_one_raw(statement(
        db,
        &format!("SELECT {COLUMNS} FROM banlists b WHERE b.id = $1"),
        vec![id.into()],
    ))
    .await?
    .as_ref()
    .map(read)
    .transpose()
}

/// Every card on the list, by oracle id.
///
/// # Errors
///
/// When the database fails, or a stored status is not one of the three.
pub async fn entries(db: &impl ConnectionTrait, id: Uuid) -> Result<Vec<Entry>, DbErr> {
    db.query_all_raw(statement(
        db,
        "SELECT oracle_id, status FROM banlist_entries \
         WHERE banlist_id = $1 ORDER BY oracle_id",
        vec![id.into()],
    ))
    .await?
    .iter()
    .map(read_entry)
    .collect()
}

/// Writes `entries` over what the list holds, in `tx`.
async fn put_entries(tx: &impl ConnectionTrait, id: Uuid, entries: &[Entry]) -> Result<(), DbErr> {
    tx.execute_raw(statement(
        tx,
        "DELETE FROM banlist_entries WHERE banlist_id = $1",
        vec![id.into()],
    ))
    .await?;
    if entries.is_empty() {
        return Ok(());
    }
    let oracles: Vec<String> = entries.iter().map(|e| e.oracle_id.to_string()).collect();
    let statuses: Vec<&str> = entries
        .iter()
        .map(|e| ListStatus::from(e.status).word())
        .collect();
    // Two arrays and one statement: an official list runs to thousands of
    // rows on a feed that bans by the set.
    tx.execute_raw(statement(
        tx,
        "INSERT INTO banlist_entries (banlist_id, oracle_id, status) \
         SELECT $1, a.oracle::uuid, a.status \
         FROM unnest(string_to_array($2, ','), string_to_array($3, ',')) AS a(oracle, status)",
        vec![
            id.into(),
            oracles.join(",").into(),
            statuses.join(",").into(),
        ],
    ))
    .await?;
    Ok(())
}

/// A new own list, empty, or a copy of a list this account may read.
///
/// # Errors
///
/// When the database fails: a name or format outside the table's bounds.
pub async fn create(
    db: &DatabaseConnection,
    account: Uuid,
    name: &str,
    format: &str,
    copy_of: Option<Uuid>,
) -> Result<Created, DbErr> {
    let tx = db.begin().await?;
    // The account's row, locked: two creations at once count each other.
    tx.query_one_raw(statement(
        &tx,
        "SELECT id FROM account WHERE id = $1 FOR UPDATE",
        vec![account.into()],
    ))
    .await?;
    let row = tx
        .query_one_raw(statement(
            &tx,
            "SELECT count(*) AS held FROM banlists WHERE owner = $1",
            vec![account.into()],
        ))
        .await?;
    let held: i64 = row.map_or(Ok(0), |row| row.try_get("", "held"))?;
    if usize::try_from(held).unwrap_or(usize::MAX) >= MAX_LISTS_PER_ACCOUNT {
        tx.rollback().await?;
        return Ok(Created::TooMany);
    }
    let copied = match copy_of {
        Some(source) => {
            if readable(&tx, source, account).await?.is_none() {
                tx.rollback().await?;
                return Ok(Created::NoSuchList);
            }
            entries(&tx, source).await?
        }
        None => Vec::new(),
    };
    if copied.len() > MAX_LIST_ENTRIES {
        tx.rollback().await?;
        return Ok(Created::TooMany);
    }
    let row = tx
        .query_one_raw(statement(
            &tx,
            "INSERT INTO banlists (name, format, owner) VALUES ($1, $2, $3) RETURNING id",
            vec![name.into(), format.into(), account.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("an insert that returned nothing".into()))?;
    let id: Uuid = row.try_get("", "id")?;
    put_entries(&tx, id, &copied).await?;
    let made = readable(&tx, id, account)
        .await?
        .ok_or_else(|| DbErr::Custom("a list gone as it was made".into()))?;
    tx.commit().await?;
    Ok(Created::Made(made))
}

/// Changes one of `account`'s own lists. A change to its cards or its format
/// moves its version; a rename does not, because it changes no verdict.
///
/// # Errors
///
/// When the database fails: a name or format outside the table's bounds.
pub async fn edit(
    db: &DatabaseConnection,
    id: Uuid,
    account: Uuid,
    change: &Edit,
) -> Result<Changed, DbErr> {
    if change
        .entries
        .as_ref()
        .is_some_and(|entries| entries.len() > MAX_LIST_ENTRIES)
    {
        return Ok(Changed::TooManyCards);
    }
    let tx = db.begin().await?;
    let row = tx
        .query_one_raw(statement(
            &tx,
            "SELECT builtin, owner FROM banlists WHERE id = $1 FOR UPDATE",
            vec![id.into()],
        ))
        .await?;
    let Some(row) = row else {
        tx.rollback().await?;
        return Ok(Changed::NoSuchList);
    };
    let builtin: bool = row.try_get("", "builtin")?;
    let owner: Option<Uuid> = row.try_get("", "owner")?;
    if builtin {
        tx.rollback().await?;
        return Ok(Changed::Official);
    }
    if owner != Some(account) {
        tx.rollback().await?;
        return Ok(Changed::NoSuchList);
    }
    let now = OffsetDateTime::now_utc();
    if let Some(name) = &change.name {
        tx.execute_raw(statement(
            &tx,
            "UPDATE banlists SET name = $2, updated_at = $3 WHERE id = $1",
            vec![id.into(), name.clone().into(), now.into()],
        ))
        .await?;
    }
    if let Some(format) = &change.format {
        tx.execute_raw(statement(
            &tx,
            "UPDATE banlists SET format = $2, version = version + 1, updated_at = $3 \
             WHERE id = $1 AND format <> $2",
            vec![id.into(), format.clone().into(), now.into()],
        ))
        .await?;
    }
    if let Some(wanted) = &change.entries {
        let mut wanted = wanted.clone();
        wanted.sort_unstable();
        let mut stored = entries(&tx, id).await?;
        stored.sort_unstable();
        if stored != wanted {
            put_entries(&tx, id, &wanted).await?;
            tx.execute_raw(statement(
                &tx,
                "UPDATE banlists SET version = version + 1, updated_at = $2 WHERE id = $1",
                vec![id.into(), now.into()],
            ))
            .await?;
        }
    }
    let made = readable(&tx, id, account)
        .await?
        .ok_or_else(|| DbErr::Custom("a list gone as it changed".into()))?;
    tx.commit().await?;
    Ok(Changed::Made(made))
}

/// Deletes one of `account`'s own lists, and says whether there was one.
/// An official list is nobody's, so it is never this account's to delete.
///
/// # Errors
///
/// When the database fails.
pub async fn delete(db: &impl ConnectionTrait, id: Uuid, account: Uuid) -> Result<bool, DbErr> {
    Ok(db
        .execute_raw(statement(
            db,
            "DELETE FROM banlists WHERE id = $1 AND owner = $2",
            vec![id.into(), account.into()],
        ))
        .await?
        .rows_affected()
        > 0)
}

/// Brings the official list of `format` in step with what the feed says, and
/// says whether anything moved. Its version counts up only when its cards
/// change, so reading the same feed twice is no new version.
///
/// # Errors
///
/// When the database fails.
pub async fn sync_official(
    db: &DatabaseConnection,
    format: &str,
    wanted: &[Entry],
) -> Result<bool, DbErr> {
    let mut wanted = wanted.to_vec();
    wanted.sort_unstable();
    let tx = db.begin().await?;
    tx.execute_raw(statement(
        &tx,
        "INSERT INTO banlists (name, format, builtin) VALUES ($1, $1, true) \
         ON CONFLICT (format) WHERE builtin DO NOTHING",
        vec![format.into()],
    ))
    .await?;
    let row = tx
        .query_one_raw(statement(
            &tx,
            "SELECT id FROM banlists WHERE builtin AND format = $1 FOR UPDATE",
            vec![format.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("an official list gone as it was made".into()))?;
    let id: Uuid = row.try_get("", "id")?;
    let mut stored = entries(&tx, id).await?;
    stored.sort_unstable();
    let moved = stored != wanted;
    if moved {
        put_entries(&tx, id, &wanted).await?;
        tx.execute_raw(statement(
            &tx,
            "UPDATE banlists SET version = version + 1, updated_at = $2 WHERE id = $1",
            vec![id.into(), OffsetDateTime::now_utc().into()],
        ))
        .await?;
    }
    tx.commit().await?;
    Ok(moved)
}
