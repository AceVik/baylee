//! Friends, chat devices, groups and undelivered chat messages (beta.8,
//! `docs/protocol.md` §"Friends and chat (beta.8)").
//!
//! Hand-written statements, as in [`crate::invites`] and [`crate::records`]:
//! what is worth having here is that each decision (a request meeting the
//! other's request, a group's last seat, a device cap) is one transaction
//! that two racing callers cannot both win.
//!
//! Nothing here sees a message's plaintext or a device's secret: a device
//! row holds public keys and a signature, a message row the ciphertext the
//! sender sealed, a slot the recipient device's wrap of its key.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement, TransactionTrait, Value};
use time::OffsetDateTime;
use uuid::Uuid;

fn stmt(db: &impl ConnectionTrait, sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(db.get_database_backend(), sql, values)
}

/// `ids` as one bind: a `uuid[]`.
fn uuids(ids: &[Uuid]) -> Value {
    ids.to_vec().into()
}

/// `ids` as one bind: a `text[]`.
fn texts(ids: &[String]) -> Value {
    ids.to_vec().into()
}

// ---------------------------------------------------------------------------
// Accounts as the social side sees them.
// ---------------------------------------------------------------------------

/// An account, as much of it as the social side needs: never its username,
/// address or anything to sign in with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Who {
    /// The account id; never leaves the gateway.
    pub id: Uuid,
    /// The tag, its public number.
    pub tag: i32,
    /// The display name.
    pub name: String,
    /// Whether it is a guest.
    pub guest: bool,
}

const WHO: &str = "a.id, a.tag, a.display_name, a.guest";

fn who(row: &sea_orm::QueryResult, prefix: &str) -> Result<Who, DbErr> {
    let col = |name: &str| format!("{prefix}{name}");
    Ok(Who {
        id: row.try_get("", &col("id"))?,
        tag: row.try_get("", &col("tag"))?,
        name: row.try_get("", &col("display_name"))?,
        guest: row.try_get("", &col("guest"))?,
    })
}

/// The account whose tag is `tag`.
///
/// # Errors
///
/// When the query fails.
pub async fn who_by_tag(db: &impl ConnectionTrait, tag: i32) -> Result<Option<Who>, DbErr> {
    let row = db
        .query_one_raw(stmt(
            db,
            &format!("SELECT {WHO} FROM account a WHERE a.tag = $1"),
            vec![tag.into()],
        ))
        .await?;
    row.map(|row| who(&row, "")).transpose()
}

/// The accounts whose tags are among `tags`, in no particular order.
///
/// # Errors
///
/// When the query fails.
pub async fn who_by_tags(db: &impl ConnectionTrait, tags: &[i32]) -> Result<Vec<Who>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            &format!("SELECT {WHO} FROM account a WHERE a.tag = ANY($1)"),
            vec![tags.to_vec().into()],
        ))
        .await?;
    rows.iter().map(|row| who(row, "")).collect()
}

/// The accounts among `ids`, in no particular order.
///
/// # Errors
///
/// When the query fails.
pub async fn who_by_ids(db: &impl ConnectionTrait, ids: &[Uuid]) -> Result<Vec<Who>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            &format!("SELECT {WHO} FROM account a WHERE a.id = ANY($1)"),
            vec![uuids(ids)],
        ))
        .await?;
    rows.iter().map(|row| who(row, "")).collect()
}

/// The settings the gateway enforces for an account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
pub struct Settings {
    /// 0: friends only; 1: friends and the players at my table.
    pub whispers_from: i16,
    /// Whether others may send a friend request.
    pub requests_open: bool,
    /// Whether friends are shown which table the account is at.
    pub share_table: bool,
}

/// The settings of each of `ids` that exists.
///
/// # Errors
///
/// When the query fails.
pub async fn settings_of(
    db: &impl ConnectionTrait,
    ids: &[Uuid],
) -> Result<Vec<(Uuid, Settings)>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            "SELECT id, whispers_from, requests_open, share_table FROM account WHERE id = ANY($1)",
            vec![uuids(ids)],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok((
                row.try_get("", "id")?,
                Settings {
                    whispers_from: row.try_get("", "whispers_from")?,
                    requests_open: row.try_get("", "requests_open")?,
                    share_table: row.try_get("", "share_table")?,
                },
            ))
        })
        .collect()
}

/// Stores an account's settings.
///
/// # Errors
///
/// When the update fails.
pub async fn put_settings(
    db: &impl ConnectionTrait,
    id: Uuid,
    settings: Settings,
) -> Result<(), DbErr> {
    db.execute_raw(stmt(
        db,
        "UPDATE account SET whispers_from = $2, requests_open = $3, share_table = $4 \
         WHERE id = $1",
        vec![
            id.into(),
            settings.whispers_from.into(),
            settings.requests_open.into(),
            settings.share_table.into(),
        ],
    ))
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Devices.
// ---------------------------------------------------------------------------

/// A chat device: its public keys and the statement binding them to the
/// account's tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    /// 16 lowercase hex characters.
    pub id: String,
    /// Whose.
    pub account_id: Uuid,
    /// Ed25519 identity key, 32 bytes.
    pub ik: Vec<u8>,
    /// X25519 key, 32 bytes.
    pub dk: Vec<u8>,
    /// The statement's signature, 64 bytes.
    pub dsig: Vec<u8>,
    /// The OS family.
    pub label: String,
    /// When it was registered.
    pub added_at: OffsetDateTime,
    /// When it last opened the socket.
    pub seen_at: OffsetDateTime,
}

const DEVICE: &str = "id, account_id, ik, dk, dsig, label, added_at, seen_at";

fn device(row: &sea_orm::QueryResult) -> Result<Device, DbErr> {
    Ok(Device {
        id: row.try_get("", "id")?,
        account_id: row.try_get("", "account_id")?,
        ik: row.try_get("", "ik")?,
        dk: row.try_get("", "dk")?,
        dsig: row.try_get("", "dsig")?,
        label: row.try_get("", "label")?,
        added_at: row.try_get("", "added_at")?,
        seen_at: row.try_get("", "seen_at")?,
    })
}

/// A device about to be registered; the caller checked its statement.
#[derive(Clone, Debug)]
pub struct NewDevice {
    /// Its id, derived from `ik`.
    pub id: String,
    /// Ed25519 identity key.
    pub ik: Vec<u8>,
    /// X25519 key.
    pub dk: Vec<u8>,
    /// The statement's signature.
    pub dsig: Vec<u8>,
    /// The OS family.
    pub label: String,
}

/// How a registration went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Registered {
    /// Stored; for a guest, `replaced` names the device it took the place of.
    Added {
        /// Devices removed to make room (a guest's one).
        replaced: Vec<String>,
    },
    /// This account already has this very device; nothing changed.
    Again,
    /// The account has as many devices as it may.
    Full,
    /// The key belongs to another account's device.
    Taken,
}

/// Registers `new` for `account`, which may have `max` devices; a guest's
/// (`replace`) new device takes the place of its old ones instead of being
/// refused.
///
/// # Errors
///
/// When the transaction fails.
pub async fn register_device(
    db: &DatabaseConnection,
    account: Uuid,
    new: &NewDevice,
    max: usize,
    replace: bool,
) -> Result<Registered, DbErr> {
    let tx = db.begin().await?;
    // The account's row, locked: two registrations at once count each
    // other.
    tx.query_one_raw(stmt(
        &tx,
        "SELECT id FROM account WHERE id = $1 FOR UPDATE",
        vec![account.into()],
    ))
    .await?;
    if let Some(row) = tx
        .query_one_raw(stmt(
            &tx,
            "SELECT account_id, id FROM social_device WHERE ik = $1 OR id = $2",
            vec![new.ik.clone().into(), new.id.clone().into()],
        ))
        .await?
    {
        let owner: Uuid = row.try_get("", "account_id")?;
        let id: String = row.try_get("", "id")?;
        tx.commit().await?;
        return Ok(if owner == account && id == new.id {
            Registered::Again
        } else {
            Registered::Taken
        });
    }
    let existing: Vec<String> = tx
        .query_all_raw(stmt(
            &tx,
            "SELECT id FROM social_device WHERE account_id = $1 ORDER BY added_at",
            vec![account.into()],
        ))
        .await?
        .iter()
        .map(|row| row.try_get("", "id"))
        .collect::<Result<_, _>>()?;
    let mut replaced = Vec::new();
    if existing.len() >= max {
        if !replace {
            tx.commit().await?;
            return Ok(Registered::Full);
        }
        tx.execute_raw(stmt(
            &tx,
            "DELETE FROM social_device WHERE account_id = $1",
            vec![account.into()],
        ))
        .await?;
        replaced = existing;
    }
    tx.execute_raw(stmt(
        &tx,
        "INSERT INTO social_device (id, account_id, ik, dk, dsig, label) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        vec![
            new.id.clone().into(),
            account.into(),
            new.ik.clone().into(),
            new.dk.clone().into(),
            new.dsig.clone().into(),
            new.label.clone().into(),
        ],
    ))
    .await?;
    tx.commit().await?;
    Ok(Registered::Added { replaced })
}

/// The device `id`, whosever it is.
///
/// # Errors
///
/// When the query fails.
pub async fn device_by_id(db: &impl ConnectionTrait, id: &str) -> Result<Option<Device>, DbErr> {
    let row = db
        .query_one_raw(stmt(
            db,
            &format!("SELECT {DEVICE} FROM social_device WHERE id = $1"),
            vec![id.into()],
        ))
        .await?;
    row.as_ref().map(device).transpose()
}

/// The devices of every account among `accounts`, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn devices_of(
    db: &impl ConnectionTrait,
    accounts: &[Uuid],
) -> Result<Vec<Device>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            &format!(
                "SELECT {DEVICE} FROM social_device WHERE account_id = ANY($1) \
                 ORDER BY added_at, id"
            ),
            vec![uuids(accounts)],
        ))
        .await?;
    rows.iter().map(device).collect()
}

/// Removes `account`'s device `id`; whether there was one.
///
/// # Errors
///
/// When the delete fails.
pub async fn remove_device(
    db: &impl ConnectionTrait,
    account: Uuid,
    id: &str,
) -> Result<bool, DbErr> {
    let done = db
        .execute_raw(stmt(
            db,
            "DELETE FROM social_device WHERE id = $1 AND account_id = $2",
            vec![id.into(), account.into()],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Notes that device `id` opened the socket now.
///
/// # Errors
///
/// When the update fails.
pub async fn touch_device(db: &impl ConnectionTrait, id: &str) -> Result<(), DbErr> {
    db.execute_raw(stmt(
        db,
        "UPDATE social_device SET seen_at = now() WHERE id = $1",
        vec![id.into()],
    ))
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Friends, requests, blocks.
// ---------------------------------------------------------------------------

/// How two accounts stand, as `me` sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Relation {
    /// Friends.
    pub friend: bool,
    /// They asked me.
    pub incoming: bool,
    /// I asked them.
    pub outgoing: bool,
    /// I block them.
    pub blocked: bool,
    /// They block me.
    pub blocks_me: bool,
}

/// How `me` and `other` stand.
///
/// # Errors
///
/// When the query fails.
pub async fn relation(db: &impl ConnectionTrait, me: Uuid, other: Uuid) -> Result<Relation, DbErr> {
    let row = db
        .query_one_raw(stmt(
            db,
            "SELECT \
               EXISTS (SELECT 1 FROM friendship WHERE a = LEAST($1, $2) AND b = GREATEST($1, $2)) AS friend, \
               EXISTS (SELECT 1 FROM friend_request WHERE from_id = $2 AND to_id = $1) AS incoming, \
               EXISTS (SELECT 1 FROM friend_request WHERE from_id = $1 AND to_id = $2) AS outgoing, \
               EXISTS (SELECT 1 FROM social_block WHERE blocker = $1 AND blocked = $2) AS blocked, \
               EXISTS (SELECT 1 FROM social_block WHERE blocker = $2 AND blocked = $1) AS blocks_me",
            vec![me.into(), other.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("a relation query answered no row".into()))?;
    Ok(Relation {
        friend: row.try_get("", "friend")?,
        // A request from someone I block is never shown to me.
        incoming: row.try_get::<bool>("", "incoming")? && !row.try_get::<bool>("", "blocked")?,
        outgoing: row.try_get("", "outgoing")?,
        blocked: row.try_get("", "blocked")?,
        blocks_me: row.try_get("", "blocks_me")?,
    })
}

/// How a friend request went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requested {
    /// Stored (or, to someone who blocks the sender, looks stored).
    Pending,
    /// They had asked already: now friends.
    Friends,
    /// Already friends.
    AlreadyFriends,
    /// They take no requests.
    Closed,
    /// The sender has too many requests out, or one side has as many
    /// friends as it may.
    TooMany,
}

/// `from` asks `to` to be friends. `max_out` bounds `from`'s outstanding
/// requests and `max_friends` either side's friends.
///
/// A request to an account that blocks `from` is stored like any other
/// and never shown to that account ([`incoming`] leaves it out), so the
/// sender cannot tell it from one that was not answered.
///
/// # Errors
///
/// When the transaction fails.
pub async fn request_friend(
    db: &DatabaseConnection,
    from: Uuid,
    to: Uuid,
    max_out: usize,
    max_friends: usize,
) -> Result<Requested, DbErr> {
    let tx = db.begin().await?;
    // Both rows locked in one order, so a request each way at once is one
    // friendship, not two requests.
    tx.query_all_raw(stmt(
        &tx,
        "SELECT id FROM account WHERE id IN ($1, $2) ORDER BY id FOR UPDATE",
        vec![from.into(), to.into()],
    ))
    .await?;
    let relation = relation(&tx, from, to).await?;
    let outcome = if relation.friend {
        Requested::AlreadyFriends
    } else if relation.incoming && !relation.blocks_me {
        if friend_count(&tx, from).await? >= max_friends
            || friend_count(&tx, to).await? >= max_friends
        {
            Requested::TooMany
        } else {
            befriend(&tx, from, to).await?;
            Requested::Friends
        }
    } else if !requests_open(&tx, to).await? {
        Requested::Closed
    } else if relation.outgoing {
        Requested::Pending
    } else if count(
        &tx,
        "SELECT count(*) AS n FROM friend_request WHERE from_id = $1",
        vec![from.into()],
    )
    .await?
        >= max_out
    {
        Requested::TooMany
    } else {
        tx.execute_raw(stmt(
            &tx,
            "INSERT INTO friend_request (from_id, to_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            vec![from.into(), to.into()],
        ))
        .await?;
        Requested::Pending
    };
    tx.commit().await?;
    Ok(outcome)
}

async fn requests_open(db: &impl ConnectionTrait, id: Uuid) -> Result<bool, DbErr> {
    Ok(db
        .query_one_raw(stmt(
            db,
            "SELECT requests_open FROM account WHERE id = $1",
            vec![id.into()],
        ))
        .await?
        .map(|row| row.try_get::<bool>("", "requests_open"))
        .transpose()?
        .unwrap_or(false))
}

async fn count(db: &impl ConnectionTrait, sql: &str, values: Vec<Value>) -> Result<usize, DbErr> {
    let n: i64 = db
        .query_one_raw(stmt(db, sql, values))
        .await?
        .ok_or_else(|| DbErr::Custom("a count answered no row".into()))?
        .try_get("", "n")?;
    Ok(usize::try_from(n).unwrap_or(usize::MAX))
}

async fn friend_count(db: &impl ConnectionTrait, id: Uuid) -> Result<usize, DbErr> {
    count(
        db,
        "SELECT count(*) AS n FROM friendship WHERE a = $1 OR b = $1",
        vec![id.into()],
    )
    .await
}

/// Makes `x` and `y` friends and drops their requests either way.
async fn befriend(db: &impl ConnectionTrait, x: Uuid, y: Uuid) -> Result<(), DbErr> {
    db.execute_raw(stmt(
        db,
        "DELETE FROM friend_request WHERE (from_id = $1 AND to_id = $2) OR (from_id = $2 AND to_id = $1)",
        vec![x.into(), y.into()],
    ))
    .await?;
    db.execute_raw(stmt(
        db,
        "INSERT INTO friendship (a, b) VALUES (LEAST($1::uuid, $2::uuid), GREATEST($1::uuid, $2::uuid)) \
         ON CONFLICT DO NOTHING",
        vec![x.into(), y.into()],
    ))
    .await?;
    Ok(())
}

/// How an answer to a request went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accepted {
    /// Now friends.
    Friends,
    /// No such request (or it is from someone `me` blocks).
    NoRequest,
    /// One side has as many friends as it may.
    TooMany,
}

/// `me` accepts `from`'s request.
///
/// # Errors
///
/// When the transaction fails.
pub async fn accept(
    db: &DatabaseConnection,
    me: Uuid,
    from: Uuid,
    max_friends: usize,
) -> Result<Accepted, DbErr> {
    let tx = db.begin().await?;
    tx.query_all_raw(stmt(
        &tx,
        "SELECT id FROM account WHERE id IN ($1, $2) ORDER BY id FOR UPDATE",
        vec![from.into(), me.into()],
    ))
    .await?;
    let relation = relation(&tx, me, from).await?;
    let outcome = if !relation.incoming {
        Accepted::NoRequest
    } else if friend_count(&tx, me).await? >= max_friends
        || friend_count(&tx, from).await? >= max_friends
    {
        Accepted::TooMany
    } else {
        befriend(&tx, me, from).await?;
        Accepted::Friends
    };
    tx.commit().await?;
    Ok(outcome)
}

/// Drops the request from `from` to `to`; whether there was one.
///
/// # Errors
///
/// When the delete fails.
pub async fn drop_request(db: &impl ConnectionTrait, from: Uuid, to: Uuid) -> Result<bool, DbErr> {
    let done = db
        .execute_raw(stmt(
            db,
            "DELETE FROM friend_request WHERE from_id = $1 AND to_id = $2",
            vec![from.into(), to.into()],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Ends the friendship of `x` and `y`; whether there was one.
///
/// # Errors
///
/// When the delete fails.
pub async fn unfriend(db: &impl ConnectionTrait, x: Uuid, y: Uuid) -> Result<bool, DbErr> {
    let done = db
        .execute_raw(stmt(
            db,
            "DELETE FROM friendship WHERE a = LEAST($1::uuid, $2::uuid) AND b = GREATEST($1::uuid, $2::uuid)",
            vec![x.into(), y.into()],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// What a block ended besides itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Blocked {
    /// They were friends.
    pub was_friend: bool,
    /// `me` had asked them.
    pub had_outgoing: bool,
    /// They had asked `me`.
    pub had_incoming: bool,
}

/// `me` blocks `other`: also ends any friendship and requests both ways.
///
/// # Errors
///
/// When the transaction fails.
pub async fn block(db: &DatabaseConnection, me: Uuid, other: Uuid) -> Result<Blocked, DbErr> {
    let tx = db.begin().await?;
    let relation = relation(&tx, me, other).await?;
    unfriend(&tx, me, other).await?;
    tx.execute_raw(stmt(
        &tx,
        "DELETE FROM friend_request WHERE (from_id = $1 AND to_id = $2) OR (from_id = $2 AND to_id = $1)",
        vec![me.into(), other.into()],
    ))
    .await?;
    tx.execute_raw(stmt(
        &tx,
        "INSERT INTO social_block (blocker, blocked) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        vec![me.into(), other.into()],
    ))
    .await?;
    tx.commit().await?;
    Ok(Blocked {
        was_friend: relation.friend,
        had_outgoing: relation.outgoing,
        had_incoming: relation.incoming,
    })
}

/// `me` unblocks `other`; whether there was a block.
///
/// A request `other` sent while blocked goes with the block: it looked sent
/// and was never to arrive, and an unblock does not deliver it late.
///
/// # Errors
///
/// When a delete fails.
pub async fn unblock(db: &impl ConnectionTrait, me: Uuid, other: Uuid) -> Result<bool, DbErr> {
    let done = db
        .execute_raw(stmt(
            db,
            "DELETE FROM social_block WHERE blocker = $1 AND blocked = $2",
            vec![me.into(), other.into()],
        ))
        .await?;
    let unblocked = done.rows_affected() == 1;
    if unblocked {
        drop_request(db, other, me).await?;
    }
    Ok(unblocked)
}

/// `me`'s friends and since when, by name.
///
/// # Errors
///
/// When the query fails.
pub async fn friends(
    db: &impl ConnectionTrait,
    me: Uuid,
) -> Result<Vec<(Who, OffsetDateTime)>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            &format!(
                "SELECT {WHO}, f.since FROM friendship f \
                 JOIN account a ON a.id = CASE WHEN f.a = $1 THEN f.b ELSE f.a END \
                 WHERE f.a = $1 OR f.b = $1 ORDER BY a.display_name, a.tag"
            ),
            vec![me.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| Ok((who(row, "")?, row.try_get("", "since")?)))
        .collect()
}

/// The ids of `me`'s friends.
///
/// # Errors
///
/// When the query fails.
pub async fn friend_ids(db: &impl ConnectionTrait, me: Uuid) -> Result<Vec<Uuid>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            "SELECT CASE WHEN a = $1 THEN b ELSE a END AS id FROM friendship WHERE a = $1 OR b = $1",
            vec![me.into()],
        ))
        .await?;
    rows.iter().map(|row| row.try_get("", "id")).collect()
}

/// Requests to `me`, oldest first, leaving out any from an account `me`
/// blocks.
///
/// # Errors
///
/// When the query fails.
pub async fn incoming(db: &impl ConnectionTrait, me: Uuid) -> Result<Vec<Who>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            &format!(
                "SELECT {WHO} FROM friend_request r JOIN account a ON a.id = r.from_id \
                 WHERE r.to_id = $1 AND NOT EXISTS \
                   (SELECT 1 FROM social_block s WHERE s.blocker = $1 AND s.blocked = r.from_id) \
                 ORDER BY r.at"
            ),
            vec![me.into()],
        ))
        .await?;
    rows.iter().map(|row| who(row, "")).collect()
}

/// Requests from `me`, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn outgoing(db: &impl ConnectionTrait, me: Uuid) -> Result<Vec<Who>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            &format!(
                "SELECT {WHO} FROM friend_request r JOIN account a ON a.id = r.to_id \
                 WHERE r.from_id = $1 ORDER BY r.at"
            ),
            vec![me.into()],
        ))
        .await?;
    rows.iter().map(|row| who(row, "")).collect()
}

/// The accounts `me` blocks, by name.
///
/// # Errors
///
/// When the query fails.
pub async fn blocked(db: &impl ConnectionTrait, me: Uuid) -> Result<Vec<Who>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            &format!(
                "SELECT {WHO} FROM social_block s JOIN account a ON a.id = s.blocked \
                 WHERE s.blocker = $1 ORDER BY a.display_name, a.tag"
            ),
            vec![me.into()],
        ))
        .await?;
    rows.iter().map(|row| who(row, "")).collect()
}

/// Which of `among` block `sender`.
///
/// # Errors
///
/// When the query fails.
pub async fn blockers_of(
    db: &impl ConnectionTrait,
    sender: Uuid,
    among: &[Uuid],
) -> Result<Vec<Uuid>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            "SELECT blocker FROM social_block WHERE blocked = $1 AND blocker = ANY($2)",
            vec![sender.into(), uuids(among)],
        ))
        .await?;
    rows.iter().map(|row| row.try_get("", "blocker")).collect()
}

// ---------------------------------------------------------------------------
// Groups.
// ---------------------------------------------------------------------------

/// A group, its members (in the order they joined) and its pending invites.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// Its id.
    pub id: Uuid,
    /// Its name.
    pub name: String,
    /// Members, first joined first, with whether each owns it.
    pub members: Vec<(Who, bool)>,
    /// Invited, not yet answered, oldest first.
    pub invited: Vec<Who>,
}

impl Group {
    /// Whether `account` is a member, and whether it owns the group.
    #[must_use]
    pub fn role(&self, account: Uuid) -> Option<bool> {
        self.members
            .iter()
            .find(|(who, _)| who.id == account)
            .map(|(_, owner)| *owner)
    }

    /// The member ids.
    #[must_use]
    pub fn member_ids(&self) -> Vec<Uuid> {
        self.members.iter().map(|(who, _)| who.id).collect()
    }
}

/// The groups `ids` names that exist, members and invites filled in.
///
/// # Errors
///
/// When a query fails.
pub async fn groups(db: &impl ConnectionTrait, ids: &[Uuid]) -> Result<Vec<Group>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            "SELECT id, name FROM chat_group WHERE id = ANY($1) ORDER BY created_at, id",
            vec![uuids(ids)],
        ))
        .await?;
    let mut groups: Vec<Group> = rows
        .iter()
        .map(|row| {
            Ok(Group {
                id: row.try_get("", "id")?,
                name: row.try_get("", "name")?,
                members: Vec::new(),
                invited: Vec::new(),
            })
        })
        .collect::<Result<_, DbErr>>()?;
    let members = db
        .query_all_raw(stmt(
            db,
            &format!(
                "SELECT m.group_id, m.owner, {WHO} FROM group_member m \
                 JOIN account a ON a.id = m.account_id \
                 WHERE m.group_id = ANY($1) ORDER BY m.joined_at, a.tag"
            ),
            vec![uuids(ids)],
        ))
        .await?;
    for row in &members {
        let group: Uuid = row.try_get("", "group_id")?;
        if let Some(g) = groups.iter_mut().find(|g| g.id == group) {
            g.members.push((who(row, "")?, row.try_get("", "owner")?));
        }
    }
    let invites = db
        .query_all_raw(stmt(
            db,
            &format!(
                "SELECT i.group_id, {WHO} FROM group_invite i \
                 JOIN account a ON a.id = i.account_id \
                 WHERE i.group_id = ANY($1) ORDER BY i.at, a.tag"
            ),
            vec![uuids(ids)],
        ))
        .await?;
    for row in &invites {
        let group: Uuid = row.try_get("", "group_id")?;
        if let Some(g) = groups.iter_mut().find(|g| g.id == group) {
            g.invited.push(who(row, "")?);
        }
    }
    Ok(groups)
}

/// One group.
///
/// # Errors
///
/// When a query fails.
pub async fn group(db: &impl ConnectionTrait, id: Uuid) -> Result<Option<Group>, DbErr> {
    Ok(groups(db, &[id]).await?.into_iter().next())
}

/// The ids of the groups `me` is in.
///
/// # Errors
///
/// When the query fails.
pub async fn group_ids_of(db: &impl ConnectionTrait, me: Uuid) -> Result<Vec<Uuid>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            "SELECT group_id FROM group_member WHERE account_id = $1",
            vec![me.into()],
        ))
        .await?;
    rows.iter().map(|row| row.try_get("", "group_id")).collect()
}

/// An open invite to a group, as the invitee sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupInvite {
    /// The group.
    pub group: Uuid,
    /// Its name.
    pub name: String,
    /// Who invited.
    pub inviter: Who,
    /// When.
    pub at: OffsetDateTime,
}

/// The open invites to `me`, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn group_invites_of(
    db: &impl ConnectionTrait,
    me: Uuid,
) -> Result<Vec<GroupInvite>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            &format!(
                "SELECT i.group_id, g.name, i.at, {WHO} FROM group_invite i \
                 JOIN chat_group g ON g.id = i.group_id \
                 JOIN account a ON a.id = i.inviter_id \
                 WHERE i.account_id = $1 ORDER BY i.at"
            ),
            vec![me.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(GroupInvite {
                group: row.try_get("", "group_id")?,
                name: row.try_get("", "name")?,
                inviter: who(row, "")?,
                at: row.try_get("", "at")?,
            })
        })
        .collect()
}

/// The accounts that share a group with `me`, `me` left out.
///
/// # Errors
///
/// When the query fails.
pub async fn co_members(db: &impl ConnectionTrait, me: Uuid) -> Result<Vec<Uuid>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            "SELECT DISTINCT o.account_id FROM group_member m \
             JOIN group_member o ON o.group_id = m.group_id \
             WHERE m.account_id = $1 AND o.account_id <> $1",
            vec![me.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| row.try_get("", "account_id"))
        .collect()
}

/// Makes a group named `name` owned by `owner`; `None` when `owner` is in
/// `max_groups` already.
///
/// # Errors
///
/// When the transaction fails.
pub async fn create_group(
    db: &DatabaseConnection,
    owner: Uuid,
    name: &str,
    max_groups: usize,
) -> Result<Option<Uuid>, DbErr> {
    let tx = db.begin().await?;
    tx.query_one_raw(stmt(
        &tx,
        "SELECT id FROM account WHERE id = $1 FOR UPDATE",
        vec![owner.into()],
    ))
    .await?;
    if count(
        &tx,
        "SELECT count(*) AS n FROM group_member WHERE account_id = $1",
        vec![owner.into()],
    )
    .await?
        >= max_groups
    {
        tx.commit().await?;
        return Ok(None);
    }
    let id: Uuid = tx
        .query_one_raw(stmt(
            &tx,
            "INSERT INTO chat_group (name) VALUES ($1) RETURNING id",
            vec![name.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("the insert answered no id".into()))?
        .try_get("", "id")?;
    tx.execute_raw(stmt(
        &tx,
        "INSERT INTO group_member (group_id, account_id, owner) VALUES ($1, $2, true)",
        vec![id.into(), owner.into()],
    ))
    .await?;
    tx.commit().await?;
    Ok(Some(id))
}

/// Renames group `id`.
///
/// # Errors
///
/// When the update fails.
pub async fn rename_group(db: &impl ConnectionTrait, id: Uuid, name: &str) -> Result<(), DbErr> {
    db.execute_raw(stmt(
        db,
        "UPDATE chat_group SET name = $2 WHERE id = $1",
        vec![id.into(), name.into()],
    ))
    .await?;
    Ok(())
}

/// How an invite to a group went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupInvited {
    /// Stored.
    Done,
    /// Already invited; nothing changed.
    Again,
    /// Already a member.
    Member,
    /// Members and invites together are at `max_members`.
    Full,
}

/// `inviter` invites `invitee` to group `id`, which holds `max_members`
/// members and pending invites together.
///
/// # Errors
///
/// When the transaction fails.
pub async fn invite_to_group(
    db: &DatabaseConnection,
    id: Uuid,
    invitee: Uuid,
    inviter: Uuid,
    max_members: usize,
) -> Result<GroupInvited, DbErr> {
    let tx = db.begin().await?;
    tx.query_one_raw(stmt(
        &tx,
        "SELECT id FROM chat_group WHERE id = $1 FOR UPDATE",
        vec![id.into()],
    ))
    .await?;
    let outcome = if count(
        &tx,
        "SELECT count(*) AS n FROM group_member WHERE group_id = $1 AND account_id = $2",
        vec![id.into(), invitee.into()],
    )
    .await?
        > 0
    {
        GroupInvited::Member
    } else if count(
        &tx,
        "SELECT count(*) AS n FROM group_invite WHERE group_id = $1 AND account_id = $2",
        vec![id.into(), invitee.into()],
    )
    .await?
        > 0
    {
        GroupInvited::Again
    } else if count(
        &tx,
        "SELECT (SELECT count(*) FROM group_member WHERE group_id = $1) \
              + (SELECT count(*) FROM group_invite WHERE group_id = $1) AS n",
        vec![id.into()],
    )
    .await?
        >= max_members
    {
        GroupInvited::Full
    } else {
        tx.execute_raw(stmt(
            &tx,
            "INSERT INTO group_invite (group_id, account_id, inviter_id) VALUES ($1, $2, $3)",
            vec![id.into(), invitee.into(), inviter.into()],
        ))
        .await?;
        GroupInvited::Done
    };
    tx.commit().await?;
    Ok(outcome)
}

/// How taking up an invite went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Joined {
    /// A member now.
    Joined,
    /// No invite to take up.
    NoInvite,
    /// `me` is in as many groups as it may.
    TooMany,
}

/// `me` takes up its invite to group `id`.
///
/// # Errors
///
/// When the transaction fails.
pub async fn join_group(
    db: &DatabaseConnection,
    id: Uuid,
    me: Uuid,
    max_groups: usize,
) -> Result<Joined, DbErr> {
    let tx = db.begin().await?;
    tx.query_one_raw(stmt(
        &tx,
        "SELECT id FROM account WHERE id = $1 FOR UPDATE",
        vec![me.into()],
    ))
    .await?;
    let invited = tx
        .execute_raw(stmt(
            &tx,
            "DELETE FROM group_invite WHERE group_id = $1 AND account_id = $2",
            vec![id.into(), me.into()],
        ))
        .await?
        .rows_affected()
        == 1;
    let outcome = if !invited {
        Joined::NoInvite
    } else if count(
        &tx,
        "SELECT count(*) AS n FROM group_member WHERE account_id = $1",
        vec![me.into()],
    )
    .await?
        >= max_groups
    {
        // The invite stays: rolled back with the rest.
        tx.rollback().await?;
        return Ok(Joined::TooMany);
    } else {
        tx.execute_raw(stmt(
            &tx,
            "INSERT INTO group_member (group_id, account_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            vec![id.into(), me.into()],
        ))
        .await?;
        Joined::Joined
    };
    tx.commit().await?;
    Ok(outcome)
}

/// Who invited `invitee` to group `id`, if it is invited.
///
/// # Errors
///
/// When the query fails.
pub async fn group_inviter(
    db: &impl ConnectionTrait,
    id: Uuid,
    invitee: Uuid,
) -> Result<Option<Uuid>, DbErr> {
    let row = db
        .query_one_raw(stmt(
            db,
            "SELECT inviter_id FROM group_invite WHERE group_id = $1 AND account_id = $2",
            vec![id.into(), invitee.into()],
        ))
        .await?;
    row.map(|row| row.try_get("", "inviter_id")).transpose()
}

/// Drops `invitee`'s invite to group `id`, answering who had invited them.
///
/// # Errors
///
/// When the delete fails.
pub async fn drop_group_invite(
    db: &impl ConnectionTrait,
    id: Uuid,
    invitee: Uuid,
) -> Result<Option<Uuid>, DbErr> {
    let row = db
        .query_one_raw(stmt(
            db,
            "DELETE FROM group_invite WHERE group_id = $1 AND account_id = $2 RETURNING inviter_id",
            vec![id.into(), invitee.into()],
        ))
        .await?;
    row.map(|row| row.try_get("", "inviter_id")).transpose()
}

/// Takes `account` out of group `id`, then hands on or deletes the group
/// as [`reown`] does. Whether it was a member.
///
/// # Errors
///
/// When a statement fails.
pub async fn remove_member(
    db: &DatabaseConnection,
    id: Uuid,
    account: Uuid,
) -> Result<bool, DbErr> {
    let tx = db.begin().await?;
    let done = tx
        .execute_raw(stmt(
            &tx,
            "DELETE FROM group_member WHERE group_id = $1 AND account_id = $2",
            vec![id.into(), account.into()],
        ))
        .await?
        .rows_affected()
        == 1;
    reown(&tx).await?;
    tx.commit().await?;
    Ok(done)
}

/// The owner `from` hands group `id` to the member `to`. Whether it did.
///
/// # Errors
///
/// When the transaction fails.
pub async fn hand_over(
    db: &DatabaseConnection,
    id: Uuid,
    from: Uuid,
    to: Uuid,
) -> Result<bool, DbErr> {
    let tx = db.begin().await?;
    let is_member = count(
        &tx,
        "SELECT count(*) AS n FROM group_member WHERE group_id = $1 AND account_id = $2",
        vec![id.into(), to.into()],
    )
    .await?
        == 1;
    let dropped = tx
        .execute_raw(stmt(
            &tx,
            "UPDATE group_member SET owner = false WHERE group_id = $1 AND account_id = $2 AND owner",
            vec![id.into(), from.into()],
        ))
        .await?
        .rows_affected()
        == 1;
    if !(is_member && dropped) || from == to {
        tx.rollback().await?;
        return Ok(false);
    }
    tx.execute_raw(stmt(
        &tx,
        "UPDATE group_member SET owner = true WHERE group_id = $1 AND account_id = $2",
        vec![id.into(), to.into()],
    ))
    .await?;
    tx.commit().await?;
    Ok(true)
}

/// Deletes group `id`, its members and invites with it.
///
/// # Errors
///
/// When the delete fails.
pub async fn delete_group(db: &impl ConnectionTrait, id: Uuid) -> Result<bool, DbErr> {
    let done = db
        .execute_raw(stmt(
            db,
            "DELETE FROM chat_group WHERE id = $1",
            vec![id.into()],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// What [`reown`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reowned {
    /// Groups that passed to the member who joined first.
    pub handed_on: u64,
    /// Groups deleted because nobody was left in them.
    pub deleted: u64,
}

/// After any departure: a group nobody is left in is deleted, and a group
/// without an owner passes to the member who joined first.
///
/// # Errors
///
/// When a statement fails.
pub async fn reown(db: &impl ConnectionTrait) -> Result<Reowned, DbErr> {
    let deleted = db
        .execute_raw(stmt(
            db,
            "DELETE FROM chat_group g WHERE NOT EXISTS \
               (SELECT 1 FROM group_member m WHERE m.group_id = g.id)",
            vec![],
        ))
        .await?
        .rows_affected();
    let handed_on = db
        .execute_raw(stmt(
            db,
            "UPDATE group_member m SET owner = true \
             FROM (SELECT DISTINCT ON (group_id) group_id, account_id FROM group_member \
                   WHERE group_id IN (SELECT group_id FROM group_member GROUP BY group_id \
                                      HAVING NOT bool_or(owner)) \
                   ORDER BY group_id, joined_at, account_id) first \
             WHERE m.group_id = first.group_id AND m.account_id = first.account_id",
            vec![],
        ))
        .await?
        .rows_affected();
    Ok(Reowned { handed_on, deleted })
}

// ---------------------------------------------------------------------------
// Messages and their slots.
// ---------------------------------------------------------------------------

/// A message about to be stored.
#[derive(Clone, Debug)]
pub struct NewMessage {
    /// The sender's `UUIDv7`.
    pub id: Uuid,
    /// The conversation's id.
    pub conv: String,
    /// The sending account.
    pub sender: Uuid,
    /// The sending device.
    pub sender_dev: String,
    /// The sealed message without its wraps (JSON).
    pub sealed: Vec<u8>,
}

/// One recipient device's slot.
#[derive(Clone, Debug)]
pub struct NewSlot {
    /// The device.
    pub device: String,
    /// Its wrap of the message key, 48 bytes.
    pub wrap: Vec<u8>,
    /// When it is deleted undelivered.
    pub expires_at: OffsetDateTime,
}

/// How storing a message went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stored {
    /// Stored at `at`; `slots` names each device's slot number.
    Done {
        /// When the gateway received it.
        at: OffsetDateTime,
        /// `(device, seq)` per slot.
        slots: Vec<(String, i64)>,
    },
    /// A message with this id is stored already.
    Duplicate,
    /// A recipient device holds `cap` slots already.
    Full,
}

/// Stores `message` with one slot per `slots`, unless a device among them
/// holds `cap` undelivered slots already. One transaction: the message is
/// never stored without its slots.
///
/// # Errors
///
/// When the transaction fails.
pub async fn store_message(
    db: &DatabaseConnection,
    message: &NewMessage,
    slots: &[NewSlot],
    cap: usize,
) -> Result<Stored, DbErr> {
    let tx = db.begin().await?;
    let devices: Vec<String> = slots.iter().map(|s| s.device.clone()).collect();
    let full = tx
        .query_one_raw(stmt(
            &tx,
            "SELECT device_id FROM chat_delivery WHERE device_id = ANY($1) \
             GROUP BY device_id HAVING count(*) >= $2 LIMIT 1",
            vec![
                texts(&devices),
                i64::try_from(cap).unwrap_or(i64::MAX).into(),
            ],
        ))
        .await?;
    if full.is_some() {
        tx.rollback().await?;
        return Ok(Stored::Full);
    }
    let Some(row) = tx
        .query_one_raw(stmt(
            &tx,
            "INSERT INTO chat_message (id, conv, sender_id, sender_dev, sealed) \
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO NOTHING RETURNING at",
            vec![
                message.id.into(),
                message.conv.clone().into(),
                message.sender.into(),
                message.sender_dev.clone().into(),
                message.sealed.clone().into(),
            ],
        ))
        .await?
    else {
        tx.rollback().await?;
        return Ok(Stored::Duplicate);
    };
    let at: OffsetDateTime = row.try_get("", "at")?;
    let mut stored = Vec::with_capacity(slots.len());
    for slot in slots {
        let seq: i64 = tx
            .query_one_raw(stmt(
                &tx,
                "INSERT INTO chat_delivery (device_id, message_id, wrap, expires_at) \
                 VALUES ($1, $2, $3, $4) RETURNING seq",
                vec![
                    slot.device.clone().into(),
                    message.id.into(),
                    slot.wrap.clone().into(),
                    slot.expires_at.into(),
                ],
            ))
            .await?
            .ok_or_else(|| DbErr::Custom("the insert answered no seq".into()))?
            .try_get("", "seq")?;
        stored.push((slot.device.clone(), seq));
    }
    tx.commit().await?;
    Ok(Stored::Done { at, slots: stored })
}

/// A slot waiting for its device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    /// The slot's number.
    pub seq: i64,
    /// The conversation.
    pub conv: String,
    /// Who sent it.
    pub sender: Who,
    /// From which device.
    pub sender_dev: String,
    /// The sealed message without its wraps (JSON).
    pub sealed: Vec<u8>,
    /// This device's wrap.
    pub wrap: Vec<u8>,
    /// When the gateway received it.
    pub at: OffsetDateTime,
}

/// Up to `limit` of device `device`'s slots, lowest first, leaving out the
/// ones in `skip` (sent and not yet acked).
///
/// # Errors
///
/// When the query fails.
pub async fn pending(
    db: &impl ConnectionTrait,
    device: &str,
    skip: &[i64],
    limit: usize,
) -> Result<Vec<Pending>, DbErr> {
    let rows = db
        .query_all_raw(stmt(
            db,
            &format!(
                "SELECT d.seq, d.wrap, m.conv, m.sender_dev, m.sealed, m.at, {WHO} \
                 FROM chat_delivery d \
                 JOIN chat_message m ON m.id = d.message_id \
                 JOIN account a ON a.id = m.sender_id \
                 WHERE d.device_id = $1 AND NOT (d.seq = ANY($2)) AND d.expires_at > now() \
                 ORDER BY d.seq LIMIT $3"
            ),
            vec![
                device.into(),
                skip.to_vec().into(),
                i64::try_from(limit).unwrap_or(i64::MAX).into(),
            ],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(Pending {
                seq: row.try_get("", "seq")?,
                conv: row.try_get("", "conv")?,
                sender: who(row, "")?,
                sender_dev: row.try_get("", "sender_dev")?,
                sealed: row.try_get("", "sealed")?,
                wrap: row.try_get("", "wrap")?,
                at: row.try_get("", "at")?,
            })
        })
        .collect()
}

/// Deletes device `device`'s slots `seqs`, which it has; how many went.
///
/// # Errors
///
/// When the delete fails.
pub async fn ack(db: &impl ConnectionTrait, device: &str, seqs: &[i64]) -> Result<u64, DbErr> {
    if seqs.is_empty() {
        return Ok(0);
    }
    let done = db
        .execute_raw(stmt(
            db,
            "DELETE FROM chat_delivery WHERE device_id = $1 AND seq = ANY($2)",
            vec![device.into(), seqs.to_vec().into()],
        ))
        .await?;
    Ok(done.rows_affected())
}

/// What [`sweep`] removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Swept {
    /// Slots past their expiry.
    pub slots: u64,
    /// Messages no slot was left for.
    pub messages: u64,
}

/// Deletes the slots past their expiry at `now`, then every message that
/// has no slot left.
///
/// # Errors
///
/// When a delete fails.
pub async fn sweep(db: &impl ConnectionTrait, now: OffsetDateTime) -> Result<Swept, DbErr> {
    let slots = db
        .execute_raw(stmt(
            db,
            "DELETE FROM chat_delivery WHERE expires_at <= $1",
            vec![now.into()],
        ))
        .await?
        .rows_affected();
    let messages = db
        .execute_raw(stmt(
            db,
            "DELETE FROM chat_message m WHERE NOT EXISTS \
               (SELECT 1 FROM chat_delivery d WHERE d.message_id = m.id)",
            vec![],
        ))
        .await?
        .rows_affected();
    Ok(Swept { slots, messages })
}

/// Counts for the admin console: never an id.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Chat devices.
    pub devices: i64,
    /// Friendships.
    pub friendships: i64,
    /// Groups.
    pub groups: i64,
    /// Undelivered slots.
    pub pending_slots: i64,
}

/// The counts the admin console shows.
///
/// # Errors
///
/// When the query fails.
pub async fn counts(db: &impl ConnectionTrait) -> Result<Counts, DbErr> {
    let row = db
        .query_one_raw(stmt(
            db,
            "SELECT (SELECT count(*) FROM social_device) AS devices, \
                    (SELECT count(*) FROM friendship) AS friendships, \
                    (SELECT count(*) FROM chat_group) AS groups, \
                    (SELECT count(*) FROM chat_delivery) AS pending_slots",
            vec![],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("a count answered no row".into()))?;
    Ok(Counts {
        devices: row.try_get("", "devices")?,
        friendships: row.try_get("", "friendships")?,
        groups: row.try_get("", "groups")?,
        pending_slots: row.try_get("", "pending_slots")?,
    })
}
