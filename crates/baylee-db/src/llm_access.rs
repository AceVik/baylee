//! Who may seat which hosted model (owner, 13.10.2026; `docs/protocol.md`
//! §"Who may seat which model").
//!
//! Per hosted profile id a policy: [`Policy::Registered`] (every registered
//! account), [`Policy::Listed`] (only the accounts on its list) or
//! [`Policy::Nobody`]. A profile without a row is listed with an empty list,
//! so a new profile is offered to nobody until an admin opens it; the one
//! the operator marks `default_for_registered` is open to every registered
//! account. A guest is never admitted, whatever the policy.
//!
//! Hand-written statements, as in [`crate::card_blocks`]. Reads are per
//! request (two rare routes), so there is no cache to go stale: a change is
//! in force at the next listing or order. A list row goes with its account
//! by cascade.

use std::collections::BTreeMap;

use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement, TransactionTrait as _};
use time::OffsetDateTime;
use uuid::Uuid;

/// The most accounts one change may add or remove.
pub const MAX_CHANGE: usize = 200;

/// The fewest characters a candidate search takes.
pub const MIN_SEARCH_CHARS: usize = 2;

/// The most candidates a search answers.
pub const MAX_CANDIDATES: u32 = 10;

/// Who may seat a profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Policy {
    /// Every registered account.
    Registered,
    /// Only the accounts on the profile's list.
    Listed,
    /// No one.
    Nobody,
}

impl Policy {
    /// The word the console and the table use.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Listed => "listed",
            Self::Nobody => "nobody",
        }
    }

    /// The policy a word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "registered" => Some(Self::Registered),
            "listed" => Some(Self::Listed),
            "nobody" => Some(Self::Nobody),
            _ => None,
        }
    }
}

/// One profile's stored policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Access {
    /// The profile's id.
    pub profile_id: String,
    /// Who may seat it.
    pub policy: Policy,
    /// The profile offered to every registered account by default.
    pub default_for_registered: bool,
    /// When it was last changed.
    pub updated_at: OffsetDateTime,
    /// By which admin.
    pub updated_by: String,
    /// How many accounts are on its list.
    pub listed: i64,
}

/// What one account's standing is with one profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Standing {
    /// The profile's policy.
    pub policy: Policy,
    /// Whether the account is on its list.
    pub listed: bool,
}

/// Whether an account with `standing` (`None`: the profile has no row) may
/// seat the profile; a guest never may.
#[must_use]
pub fn admits(standing: Option<Standing>, guest: bool) -> bool {
    if guest {
        return false;
    }
    match standing {
        Some(Standing {
            policy: Policy::Registered,
            ..
        }) => true,
        Some(Standing {
            policy: Policy::Listed,
            listed,
        }) => listed,
        Some(Standing {
            policy: Policy::Nobody,
            ..
        })
        | None => false,
    }
}

/// An account on a profile's list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allowed {
    /// The account's id.
    pub account_id: Uuid,
    /// Its username, if it has one.
    pub username: Option<String>,
    /// Its display name.
    pub display_name: String,
    /// Its tag.
    pub tag: i32,
    /// When it was added.
    pub added_at: OffsetDateTime,
    /// By which admin.
    pub added_by: String,
}

/// A registered account a search found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The account's id.
    pub account_id: Uuid,
    /// Its username.
    pub username: Option<String>,
    /// Its display name.
    pub display_name: String,
    /// Its tag.
    pub tag: i32,
}

/// Why a change to a list was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// No account has this id.
    Unknown(Uuid),
    /// This account is a guest's, which no list may hold.
    Guest(Uuid),
}

fn policy_of(row: &sea_orm::QueryResult) -> Result<Policy, DbErr> {
    let word: String = row.try_get("", "policy")?;
    Policy::parse(&word).ok_or_else(|| DbErr::Custom(format!("an unknown policy {word:?}")))
}

fn ids(accounts: &[Uuid]) -> String {
    accounts
        .iter()
        .map(Uuid::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

const ACCESS_COLUMNS: &str = "p.profile_id, p.policy, p.default_for_registered, p.updated_at, \
     p.updated_by, \
     (SELECT count(*) FROM llm_profile_allow l WHERE l.profile_id = p.profile_id) AS listed";

fn access_row(row: &sea_orm::QueryResult) -> Result<Access, DbErr> {
    Ok(Access {
        profile_id: row.try_get("", "profile_id")?,
        policy: policy_of(row)?,
        default_for_registered: row.try_get("", "default_for_registered")?,
        updated_at: row.try_get("", "updated_at")?,
        updated_by: row.try_get("", "updated_by")?,
        listed: row.try_get("", "listed")?,
    })
}

/// Every stored policy, by profile id.
///
/// # Errors
///
/// When the query fails.
pub async fn all(db: &impl ConnectionTrait) -> Result<Vec<Access>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            format!("SELECT {ACCESS_COLUMNS} FROM llm_profile_access p ORDER BY p.profile_id"),
        ))
        .await?;
    rows.iter().map(access_row).collect()
}

/// One profile's stored policy, `None` for a profile without a row.
///
/// # Errors
///
/// When the query fails.
pub async fn one(db: &impl ConnectionTrait, profile: &str) -> Result<Option<Access>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!("SELECT {ACCESS_COLUMNS} FROM llm_profile_access p WHERE p.profile_id = $1"),
            [profile.into()],
        ))
        .await?;
    row.as_ref().map(access_row).transpose()
}

/// `account`'s standing with every profile that has a row.
///
/// # Errors
///
/// When the query fails.
pub async fn standing(
    db: &impl ConnectionTrait,
    account: Uuid,
) -> Result<BTreeMap<String, Standing>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT p.profile_id, p.policy, EXISTS ( \
                 SELECT 1 FROM llm_profile_allow l \
                 WHERE l.profile_id = p.profile_id AND l.account_id = $1) AS listed \
             FROM llm_profile_access p",
            [account.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok((
                row.try_get("", "profile_id")?,
                Standing {
                    policy: policy_of(row)?,
                    listed: row.try_get("", "listed")?,
                },
            ))
        })
        .collect()
}

/// Sets `profile`'s policy, by `by`, and answers it as stored.
///
/// `default_for_registered`: `Some(true)` makes it the default profile
/// (its policy must then be [`Policy::Registered`]) and takes the mark from
/// any other; `Some(false)` takes it from this one; `None` keeps it, except
/// that a policy other than registered always clears it.
///
/// # Errors
///
/// When a statement fails, and `DbErr::Custom` for the default mark with a
/// policy other than registered.
pub async fn set_policy(
    db: &DatabaseConnection,
    profile: &str,
    policy: Policy,
    default_for_registered: Option<bool>,
    by: &str,
) -> Result<Access, DbErr> {
    if default_for_registered == Some(true) && policy != Policy::Registered {
        return Err(DbErr::Custom(
            "the default profile is open to every registered account".into(),
        ));
    }
    let txn = db.begin().await?;
    let backend = txn.get_database_backend();
    if default_for_registered == Some(true) {
        txn.execute_raw(Statement::from_sql_and_values(
            backend,
            "UPDATE llm_profile_access SET default_for_registered = false, \
                 updated_at = now(), updated_by = $2 \
             WHERE default_for_registered AND profile_id <> $1",
            [profile.into(), by.into()],
        ))
        .await?;
    }
    // `$3`: the mark asked for, or NULL to keep it; a policy other than
    // registered clears it either way.
    txn.execute_raw(Statement::from_sql_and_values(
        backend,
        "INSERT INTO llm_profile_access (profile_id, policy, default_for_registered, updated_by) \
         VALUES ($1, $2, coalesce($3, false) AND $2 = 'registered', $4) \
         ON CONFLICT (profile_id) DO UPDATE SET \
             policy = excluded.policy, \
             default_for_registered = coalesce($3, llm_profile_access.default_for_registered) \
                 AND excluded.policy = 'registered', \
             updated_at = now(), \
             updated_by = excluded.updated_by",
        [
            profile.into(),
            policy.word().into(),
            default_for_registered.into(),
            by.into(),
        ],
    ))
    .await?;
    let access = one(&txn, profile)
        .await?
        .ok_or_else(|| DbErr::Custom("the policy was not stored".into()))?;
    txn.commit().await?;
    Ok(access)
}

/// Puts `accounts` on `profile`'s list, by `by`; a profile without a row
/// gets one, listed. Answers how many were not on it already, or why none
/// was added: an id no account has, or a guest's. All or nothing.
///
/// # Errors
///
/// When a statement fails.
pub async fn allow(
    db: &DatabaseConnection,
    profile: &str,
    accounts: &[Uuid],
    by: &str,
) -> Result<Result<u64, Refused>, DbErr> {
    let txn = db.begin().await?;
    let backend = txn.get_database_backend();
    let found = txn
        .query_all_raw(Statement::from_sql_and_values(
            backend,
            "SELECT id, guest FROM account \
             WHERE id::text = ANY(string_to_array($1::text, ','))",
            [ids(accounts).into()],
        ))
        .await?;
    let mut known = BTreeMap::new();
    for row in &found {
        let id: Uuid = row.try_get("", "id")?;
        let guest: bool = row.try_get("", "guest")?;
        known.insert(id, guest);
    }
    for id in accounts {
        match known.get(id) {
            None => return Ok(Err(Refused::Unknown(*id))),
            Some(true) => return Ok(Err(Refused::Guest(*id))),
            Some(false) => {}
        }
    }
    txn.execute_raw(Statement::from_sql_and_values(
        backend,
        "INSERT INTO llm_profile_access (profile_id, policy, updated_by) \
         VALUES ($1, 'listed', $2) ON CONFLICT (profile_id) DO NOTHING",
        [profile.into(), by.into()],
    ))
    .await?;
    let added = txn
        .execute_raw(Statement::from_sql_and_values(
            backend,
            "INSERT INTO llm_profile_allow (profile_id, account_id, added_by) \
             SELECT $1, a.id, $3 FROM account a \
             WHERE a.id::text = ANY(string_to_array($2::text, ',')) AND NOT a.guest \
             ON CONFLICT (profile_id, account_id) DO NOTHING",
            [profile.into(), ids(accounts).into(), by.into()],
        ))
        .await?
        .rows_affected();
    txn.commit().await?;
    Ok(Ok(added))
}

/// Takes `accounts` off `profile`'s list; answers how many were on it.
///
/// # Errors
///
/// When the statement fails.
pub async fn disallow(
    db: &impl ConnectionTrait,
    profile: &str,
    accounts: &[Uuid],
) -> Result<u64, DbErr> {
    Ok(db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM llm_profile_allow \
             WHERE profile_id = $1 AND account_id::text = ANY(string_to_array($2::text, ','))",
            [profile.into(), ids(accounts).into()],
        ))
        .await?
        .rows_affected())
}

/// The accounts on `profile`'s list, by username (then display name).
///
/// # Errors
///
/// When the query fails.
pub async fn allowed(db: &impl ConnectionTrait, profile: &str) -> Result<Vec<Allowed>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT a.id, a.username, a.display_name, a.tag, l.added_at, l.added_by \
             FROM llm_profile_allow l JOIN account a ON a.id = l.account_id \
             WHERE l.profile_id = $1 \
             ORDER BY coalesce(a.username_key, lower(a.display_name)), a.tag",
            [profile.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(Allowed {
                account_id: row.try_get("", "id")?,
                username: row.try_get("", "username")?,
                display_name: row.try_get("", "display_name")?,
                tag: row.try_get("", "tag")?,
                added_at: row.try_get("", "added_at")?,
                added_by: row.try_get("", "added_by")?,
            })
        })
        .collect()
}

/// Registered accounts whose username or display name starts with
/// `prefix` (any case), at most `limit` (capped at [`MAX_CANDIDATES`]):
/// exact username first, then by username. A prefix under
/// [`MIN_SEARCH_CHARS`] characters finds nothing. `%`, `_` and `\` in it
/// are matched as themselves.
///
/// # Errors
///
/// When the query fails.
pub async fn candidates(
    db: &impl ConnectionTrait,
    prefix: &str,
    limit: u32,
) -> Result<Vec<Candidate>, DbErr> {
    let prefix = prefix.trim();
    if prefix.chars().count() < MIN_SEARCH_CHARS {
        return Ok(Vec::new());
    }
    let escaped: String = prefix
        .to_lowercase()
        .chars()
        .flat_map(|c| match c {
            '%' | '_' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect();
    let limit = limit.clamp(1, MAX_CANDIDATES);
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "SELECT a.id, a.username, a.display_name, a.tag FROM account a \
                 WHERE NOT a.guest AND ( \
                     coalesce(a.username_key, '') LIKE $1 || '%' ESCAPE '\\' \
                     OR lower(a.display_name) LIKE $1 || '%' ESCAPE '\\') \
                 ORDER BY a.username_key = $1 DESC NULLS LAST, \
                     coalesce(a.username_key, lower(a.display_name)), a.tag \
                 LIMIT {limit}"
            ),
            [escaped.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(Candidate {
                account_id: row.try_get("", "id")?,
                username: row.try_get("", "username")?,
                display_name: row.try_get("", "display_name")?,
                tag: row.try_get("", "tag")?,
            })
        })
        .collect()
}
