//! What the admin console reads beyond counts (`docs/protocol.md` §"The
//! admin console"): the accounts, one account with its decks and games, the
//! handles of accounts the lobby holds, and the days of the last month.
//!
//! The owner asked to see who plays (09.10.2026), so unlike [`crate::stats`]
//! these answer rows that name accounts. They still answer nothing secret
//! and nothing a player could not be told about themselves: never a
//! password hash, a token or its hash, an e-mail address (whether there is
//! one, and whether it was confirmed, is all), a deck's cards or a record's
//! bytes. The one reader is the gateway's loopback console, which hands
//! them only to the feedback service's signed-in admins.
//!
//! Hand-written statements, as in [`crate::stats`]: each answer is one
//! round trip, with every count a correlated subquery over an indexed
//! column (`session_token_by_account`, `game_record_seat_by_account`,
//! `deck`'s account index).

use sea_orm::{ConnectionTrait, DbErr, QueryResult, Statement, Value};
use time::OffsetDateTime;
use uuid::Uuid;

/// The most rows one page of [`accounts`] answers.
pub const MAX_PAGE: u32 = 200;

/// How many of an account's latest games [`account`] lists.
pub const RECENT_GAMES: u32 = 25;

/// The longest series [`daily`] answers.
pub const MAX_DAYS: u32 = 120;

/// Which accounts a listing shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    /// Every account.
    #[default]
    All,
    /// Accounts that sign in with a username.
    Registered,
    /// Guests.
    Guests,
}

impl Kind {
    /// The kind a query string names, or `None` for no such kind.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "" | "all" => Some(Self::All),
            "registered" => Some(Self::Registered),
            "guest" | "guests" => Some(Self::Guests),
            _ => None,
        }
    }

    const fn sql(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Registered => "registered",
            Self::Guests => "guest",
        }
    }
}

/// The order a listing comes in. Every order ends on the id, so a page
/// never shows a row twice or skips one between requests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Order {
    /// Newest first.
    #[default]
    Newest,
    /// Oldest first.
    Oldest,
    /// By display name, then tag.
    Name,
    /// Most games first.
    Games,
    /// The most recently active first; accounts with no session last.
    Active,
}

impl Order {
    /// The order a query string names, or `None` for no such order.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "" | "newest" => Some(Self::Newest),
            "oldest" => Some(Self::Oldest),
            "name" => Some(Self::Name),
            "games" => Some(Self::Games),
            "active" => Some(Self::Active),
            _ => None,
        }
    }

    /// The `ORDER BY` clause: one of five fixed strings, never built from
    /// what a caller sent.
    const fn sql(self) -> &'static str {
        match self {
            Self::Newest => "a.created_at DESC, a.id DESC",
            Self::Oldest => "a.created_at ASC, a.id ASC",
            Self::Name => "lower(a.display_name) ASC, a.tag ASC, a.id ASC",
            Self::Games => "games DESC, a.created_at DESC, a.id DESC",
            Self::Active => "renewed_at DESC NULLS LAST, a.created_at DESC, a.id DESC",
        }
    }
}

/// What a listing asks for.
#[derive(Clone, Debug, Default)]
pub struct Query {
    /// Part of a username, a display name, or a tag in hex (`#af03`).
    pub text: String,
    /// Exactly the account with this tag, registered or guest, when set:
    /// the moderator's "find the sender of a reported chat message" (the
    /// tag is public in every handle, and unique).
    pub tag: Option<i32>,
    /// Registered, guests or both.
    pub kind: Kind,
    /// Only these accounts, when set: the console's "online" filter, the
    /// ids it holds in memory.
    pub only: Option<Vec<Uuid>>,
    /// The order.
    pub order: Order,
    /// How many rows to skip.
    pub offset: u32,
    /// How many to answer, at most [`MAX_PAGE`].
    pub limit: u32,
}

/// How long a session lives from its last renewal, for an account and a
/// guest: the gateway's `auth::Lifetime`s, which this crate does not know.
/// A session's expiry less its lifetime is when it was last renewed, the
/// latest moment the store knows the account was active.
#[derive(Clone, Copy, Debug)]
pub struct Lifetimes {
    /// An account's session.
    pub account: time::Duration,
    /// A guest's.
    pub guest: time::Duration,
}

/// One account as the console lists it.
///
/// @description One account as the admin console lists it: never a
/// password, a token, an e-mail address or a deck's cards.
/// @openapi
/// @openapi rename `ConsoleAccountRow`
#[expect(
    clippy::struct_excessive_bools,
    reason = "a row of facts the console shows side by side, each a yes or no"
)]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Account {
    /// @description The account's id.
    /// @internal
    /// Its id.
    pub id: Uuid,
    /// @description Its username, the name it signs in with; `null` for a
    /// guest, which has none.
    /// @internal
    /// The login, for an account that has one.
    pub username: Option<String>,
    /// @description The name other players see.
    /// @internal
    /// What other players see.
    pub display_name: String,
    /// @description The number that tells two accounts of one name apart,
    /// shown in hex in its handle (`Alice#af03`).
    /// @internal
    /// The tag that tells two of a name apart.
    pub tag: i32,
    /// @description Whether it is a guest.
    /// @internal
    /// Whether it is a guest.
    pub guest: bool,
    /// @description When it was made, RFC 3339 in UTC.
    /// @openapi format: date-time
    /// @internal
    /// When it was made.
    pub created_at: String,
    /// @description Whether it has an e-mail address (never the address).
    /// @internal
    /// Whether it has an e-mail address (never the address).
    pub has_email: bool,
    /// @description Whether that address was confirmed.
    /// @internal
    /// Whether that address was confirmed.
    pub confirmed: bool,
    /// @description The language code it registered in.
    /// @internal
    /// The language it registered in.
    pub lang: String,
    /// @description Whether a closed-beta key admitted it.
    /// @internal
    /// Whether a closed-beta key admitted it.
    pub by_key: bool,
    /// @description The note of the key that admitted it; `null` when no
    /// key did, or the key is gone or has no note.
    /// @internal
    /// The note of that key, if it still exists and has one.
    pub key_note: Option<String>,
    /// @description The version of the terms it last accepted, `null` for
    /// none.
    /// @internal
    /// The version of the terms it last accepted.
    pub terms_version: Option<String>,
    /// @description How many decks it has.
    /// @internal
    /// Its decks.
    pub decks: i64,
    /// @description How many recorded games it sat in.
    /// @internal
    /// The recorded games it sat in.
    pub games: i64,
    /// @description How many of its sessions have not lapsed.
    /// @internal
    /// Sessions that have not lapsed.
    pub sessions: i64,
    /// @description When its latest live session was last renewed, RFC 3339
    /// in UTC: it was active then, or up to a renewal interval (6 hours, a
    /// guest's a day) later. `null` without a live session.
    /// @openapi format: date-time
    /// @internal
    /// When its latest session was last renewed: it was active then, or up
    /// to a renewal interval later. `None` without a live session.
    pub active_at: Option<String>,
    /// @description Whether an admin reset its password and it has not
    /// chosen its own yet.
    /// @internal
    /// An admin reset its password and it has not chosen its own yet
    /// (beta.8).
    pub reset_pending: bool,
    /// @description The ban in force on it, `null` when none is (a lapsed
    /// ban is none).
    /// @internal
    /// The ban in force on it, if one is (beta.8); a lapsed one is none.
    pub ban: Option<Ban>,
}

/// A ban in force, as the console shows it (beta.8).
///
/// @description A ban in force on an account.
/// @openapi
/// @openapi rename `ConsoleBan`
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Ban {
    /// @description Why, in the admin's words, which its player is shown.
    /// @internal
    /// The admin's words, which its player is shown.
    pub reason: String,
    /// @description When it ends, RFC 3339 in UTC; `null` for a ban for
    /// good.
    /// @openapi format: date-time
    /// @internal
    /// Until when; `None` for good.
    pub until: Option<String>,
    /// @description Since when, RFC 3339 in UTC.
    /// @openapi format: date-time
    /// @internal
    /// Since when.
    pub at: String,
    /// @description The admin who banned it.
    /// @internal
    /// The admin who banned it.
    pub by: String,
}

/// One of an account's decks, without its cards.
///
/// @description One of an account's decks, without its cards.
/// @openapi
/// @openapi rename `ConsoleDeck`
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Deck {
    /// @description The deck's id.
    /// @internal
    /// Its id.
    pub id: Uuid,
    /// @description Its name.
    /// @internal
    /// Its name.
    pub name: String,
    /// @description Its format (`commander`, `freeform`, …).
    /// @internal
    /// Its format.
    pub format: String,
    /// @description How many cards its main deck holds: each line's count
    /// (`40 Forest`), a line without one counting once.
    /// @internal
    /// How many cards its main deck holds: each line's count (`40 Forest`),
    /// a line without one counting once.
    pub cards: i64,
    /// @description Its version: how many versions it has had.
    /// @internal
    /// How many versions it has had.
    pub version: i32,
    /// @description When it last changed, RFC 3339 in UTC.
    /// @openapi format: date-time
    /// @internal
    /// When it last changed.
    pub updated_at: String,
}

/// A chair of a recorded game.
///
/// @description One chair of a recorded game.
/// @openapi
/// @openapi rename `ConsoleChair`
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Chair {
    /// @description The chair's seat number.
    /// @internal
    /// Its number.
    pub seat: i32,
    /// @description Who sat there, by handle (`Alice#af03`); `null` for a
    /// chair no account held (the house's) or whose account is gone.
    /// @internal
    /// Who sat there, as `Name#tag`; `None` for a chair no account held
    /// (the house) or whose account is gone.
    pub player: Option<String>,
    /// @description Whether that account is a guest; `null` when `player`
    /// is.
    /// @internal
    /// Whether that account is a guest.
    pub guest: Option<bool>,
}

/// One game an account sat in.
///
/// @description One recorded game an account sat in.
/// @openapi
/// @openapi rename `ConsoleGame`
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Game {
    /// @description The game's id.
    /// @internal
    /// The game's id.
    pub game_id: String,
    /// @description When it started, RFC 3339 in UTC.
    /// @openapi format: date-time
    /// @internal
    /// When it started.
    pub started_at: String,
    /// @description When its record ended, RFC 3339 in UTC; `null` while
    /// it has not.
    /// @openapi format: date-time
    /// @internal
    /// When its record ended, if it did.
    pub ended_at: Option<String>,
    /// @description Whether its record is complete.
    /// @internal
    /// Whether its record is complete.
    pub complete: bool,
    /// @description The seat number of the account's chair.
    /// @internal
    /// Which chair the account had.
    pub seat: i32,
    /// @description Every chair, in seat order.
    /// @internal
    /// Every chair.
    pub chairs: Vec<Chair>,
}

/// One account in full: its row, decks and latest games.
///
/// @description One account in full: its row as the console lists it,
/// its decks and its latest games.
/// @openapi
/// @openapi rename `ConsoleDetail`
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Detail {
    /// @description The account's row, as `GET /admin/accounts` lists it.
    /// @internal
    /// The row a listing shows.
    #[serde(flatten)]
    pub account: Account,
    /// @description When it last accepted the terms, RFC 3339 in UTC;
    /// `null` if it never did.
    /// @openapi format: date-time
    /// @internal
    /// When it last accepted the terms.
    pub terms_accepted_at: Option<String>,
    /// @description When its e-mail address was confirmed, RFC 3339 in
    /// UTC; `null` when it was not.
    /// @openapi format: date-time
    /// @internal
    /// When its address was confirmed.
    pub confirmed_at: Option<String>,
    /// @description Its decks, the latest changed first.
    /// @internal
    /// Its decks, the latest changed first.
    pub deck_list: Vec<Deck>,
    /// @description Its latest 25 recorded games, newest first.
    /// @internal
    /// Its latest [`RECENT_GAMES`] games, newest first.
    pub recent_games: Vec<Game>,
}

/// An account the lobby holds, by the handle others see.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    /// What other players see.
    pub display_name: String,
    /// Its tag.
    pub tag: i32,
    /// Whether it is a guest.
    pub guest: bool,
}

/// One day of [`daily`].
///
/// @description One UTC day of the admin console's statistics.
/// @openapi
/// @openapi rename `ConsoleDay`
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Day {
    /// @description The day, `YYYY-MM-DD` in UTC.
    /// @openapi format: date
    /// @internal
    /// `2026-10-09`, a UTC day.
    pub day: String,
    /// @description Registered accounts made that day that still exist.
    /// @internal
    /// Registered accounts made that day that still exist.
    pub registered: i64,
    /// @description Guests made that day that still exist.
    /// @internal
    /// Guests made that day that still exist.
    pub guests: i64,
    /// @description Recorded games started that day.
    /// @internal
    /// Recorded games started.
    pub started: i64,
    /// @description Recorded games finished that day.
    /// @internal
    /// Recorded games finished.
    pub finished: i64,
    /// @description Accounts that sat in a game started that day.
    /// @internal
    /// Accounts that sat in a game started that day.
    pub players: i64,
}

/// `to_char` of a `timestamptz` column as the console reads moments.
macro_rules! iso {
    ($column:expr) => {
        concat!(
            "to_char(",
            $column,
            " AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')"
        )
    };
}

/// The columns of [`Account`], over `account a`. `$1` is now, `$2` and `$3`
/// an account's and a guest's session lifetime in seconds.
const ACCOUNT_COLUMNS: &str = concat!(
    "a.id, a.username, a.display_name, a.tag, a.guest, ",
    iso!("a.created_at"),
    " AS created_at, \
     a.email IS NOT NULL AS has_email, \
     a.email IS NOT NULL AND a.confirmed_at IS NOT NULL AS confirmed, \
     a.lang, a.invite_id IS NOT NULL AS by_key, \
     (SELECT i.note FROM invite i WHERE i.id = a.invite_id) AS key_note, \
     a.terms_version, \
     (SELECT count(*) FROM deck d WHERE d.account_id = a.id) AS decks, \
     (SELECT count(*) FROM game_record_seat s WHERE s.account_id = a.id) AS games, \
     (SELECT count(*) FROM session_token t \
        WHERE t.account_id = a.id AND t.expires_at > $1) AS sessions, \
     (SELECT max(t.expires_at) FROM session_token t \
        WHERE t.account_id = a.id AND t.expires_at > $1) \
        - make_interval(secs => CASE WHEN a.guest THEN $3 ELSE $2 END) AS renewed_at, \
     a.must_change_password AS reset_pending, \
     a.banned_at IS NOT NULL AND (a.banned_until IS NULL OR a.banned_until > $1) AS banned, \
     a.ban_reason, a.banned_by, ",
    iso!("a.banned_at"),
    " AS banned_at, ",
    iso!("a.banned_until"),
    " AS banned_until"
);

/// What a listing matches, after the four parameters of
/// [`ACCOUNT_COLUMNS`]'s first three and: `$4` the text, `$5` the kind,
/// `$6` the ids as one comma-separated string (or `NULL`).
const ACCOUNT_FILTER: &str = "\
    ($4 = '' \
        OR strpos(coalesce(a.username_key, ''), lower($4)) > 0 \
        OR strpos(lower(a.display_name), lower($4)) > 0 \
        OR to_hex(a.tag) = ltrim(lower(split_part($4, '#', 2)), '0') \
        OR a.id::text = lower($4)) \
    AND ($5 = 'all' OR a.guest = ($5 = 'guest')) \
    AND ($6::text IS NULL OR a.id::text = ANY(string_to_array($6::text, ','))) \
    AND ($7::int4 IS NULL OR a.tag = $7::int4)";

fn iso_of(at: OffsetDateTime) -> String {
    let at = at.to_offset(time::UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

fn seconds(d: time::Duration) -> f64 {
    d.as_seconds_f64()
}

fn account_row(row: &QueryResult) -> Result<Account, DbErr> {
    let renewed: Option<OffsetDateTime> = row.try_get("", "renewed_at")?;
    Ok(Account {
        id: row.try_get("", "id")?,
        username: row.try_get("", "username")?,
        display_name: row.try_get("", "display_name")?,
        tag: row.try_get("", "tag")?,
        guest: row.try_get("", "guest")?,
        created_at: row.try_get("", "created_at")?,
        has_email: row.try_get("", "has_email")?,
        confirmed: row.try_get("", "confirmed")?,
        lang: row.try_get("", "lang")?,
        by_key: row.try_get("", "by_key")?,
        key_note: row.try_get("", "key_note")?,
        terms_version: row.try_get("", "terms_version")?,
        decks: row.try_get("", "decks")?,
        games: row.try_get("", "games")?,
        sessions: row.try_get("", "sessions")?,
        active_at: renewed.map(iso_of),
        reset_pending: row.try_get("", "reset_pending")?,
        ban: if row.try_get("", "banned")? {
            Some(Ban {
                reason: row
                    .try_get::<Option<String>>("", "ban_reason")?
                    .unwrap_or_default(),
                until: row.try_get("", "banned_until")?,
                at: row
                    .try_get::<Option<String>>("", "banned_at")?
                    .unwrap_or_default(),
                by: row
                    .try_get::<Option<String>>("", "banned_by")?
                    .unwrap_or_default(),
            })
        } else {
            None
        },
    })
}

fn base_values(now: OffsetDateTime, lifetimes: Lifetimes) -> Vec<Value> {
    vec![
        now.into(),
        seconds(lifetimes.account).into(),
        seconds(lifetimes.guest).into(),
    ]
}

/// One page of accounts and how many match in all.
///
/// # Errors
///
/// When a query fails.
pub async fn accounts(
    db: &impl ConnectionTrait,
    query: &Query,
    now: OffsetDateTime,
    lifetimes: Lifetimes,
) -> Result<(i64, Vec<Account>), DbErr> {
    let only = query.only.as_ref().map(|ids| {
        ids.iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join(",")
    });
    let mut values = base_values(now, lifetimes);
    values.extend([
        query.text.trim().to_owned().into(),
        query.kind.sql().into(),
        only.into(),
        query.tag.into(),
    ]);
    let backend = db.get_database_backend();
    // The count reads the same filter with the same numbering, so the three
    // values it does not use are still bound and typed by the planner.
    let total = db
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            format!(
                "SELECT count(*) AS total FROM account a \
                 WHERE $1::timestamptz IS NOT NULL AND $2::float8 IS NOT NULL \
                 AND $3::float8 IS NOT NULL AND {ACCOUNT_FILTER}"
            ),
            values.clone(),
        ))
        .await?
        .map(|row| row.try_get::<i64>("", "total"))
        .transpose()?
        .unwrap_or(0);
    let limit = query.limit.clamp(1, MAX_PAGE);
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            backend,
            format!(
                "SELECT {ACCOUNT_COLUMNS} FROM account a WHERE {ACCOUNT_FILTER} \
                 ORDER BY {} LIMIT {limit} OFFSET {}",
                query.order.sql(),
                query.offset
            ),
            values,
        ))
        .await?;
    let accounts = rows.iter().map(account_row).collect::<Result<_, _>>()?;
    Ok((total, accounts))
}

/// One account with its decks and latest games, or `None` for no such
/// account.
///
/// # Errors
///
/// When a query fails.
pub async fn account(
    db: &impl ConnectionTrait,
    id: Uuid,
    now: OffsetDateTime,
    lifetimes: Lifetimes,
) -> Result<Option<Detail>, DbErr> {
    let backend = db.get_database_backend();
    let mut values = base_values(now, lifetimes);
    values.push(id.into());
    let Some(row) = db
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            format!(
                "SELECT {ACCOUNT_COLUMNS}, {} AS terms_accepted_at, {} AS confirmed_at \
                 FROM account a WHERE a.id = $4",
                iso!("a.terms_accepted_at"),
                iso!("a.confirmed_at"),
            ),
            values,
        ))
        .await?
    else {
        return Ok(None);
    };
    let account = account_row(&row)?;
    let terms_accepted_at = row.try_get("", "terms_accepted_at")?;
    let confirmed_at = if account.has_email {
        row.try_get("", "confirmed_at")?
    } else {
        None
    };

    let decks = db
        .query_all_raw(Statement::from_sql_and_values(
            backend,
            concat!(
                "SELECT id, name, format, \
                 (SELECT coalesce(sum(coalesce(substring(c FROM '^\\s*(\\d+)')::bigint, 1)), 0) \
                    FROM unnest(cards) AS c)::bigint AS cards, version, ",
                iso!("updated_at"),
                " AS updated_at FROM deck WHERE account_id = $1 \
                 ORDER BY updated_at DESC, id DESC"
            ),
            [id.into()],
        ))
        .await?
        .iter()
        .map(|row| {
            Ok(Deck {
                id: row.try_get("", "id")?,
                name: row.try_get("", "name")?,
                format: row.try_get("", "format")?,
                cards: row.try_get("", "cards")?,
                version: row.try_get("", "version")?,
                updated_at: row.try_get("", "updated_at")?,
            })
        })
        .collect::<Result<_, DbErr>>()?;

    let games = db
        .query_all_raw(Statement::from_sql_and_values(
            backend,
            format!(
                concat!(
                    "SELECT g.game_id, ",
                    iso!("g.started_at"),
                    " AS started_at, ",
                    iso!("g.ended_at"),
                    " AS ended_at, g.complete, s.seat, \
                     (SELECT coalesce(json_agg(json_build_object(\
                        'seat', x.seat, \
                        'player', CASE WHEN o.id IS NULL THEN NULL \
                            ELSE o.display_name || '#' || to_hex(o.tag) END, \
                        'guest', o.guest) ORDER BY x.seat), '[]'::json)::text \
                      FROM game_record_seat x LEFT JOIN account o ON o.id = x.account_id \
                      WHERE x.game_id = g.game_id) AS chairs \
                     FROM game_record_seat s JOIN game_record g ON g.game_id = s.game_id \
                     WHERE s.account_id = $1 \
                     ORDER BY g.started_at DESC, g.game_id DESC LIMIT {}"
                ),
                RECENT_GAMES
            ),
            [id.into()],
        ))
        .await?
        .iter()
        .map(|row| {
            let chairs: String = row.try_get("", "chairs")?;
            Ok(Game {
                game_id: row.try_get("", "game_id")?,
                started_at: row.try_get("", "started_at")?,
                ended_at: row.try_get("", "ended_at")?,
                complete: row.try_get("", "complete")?,
                seat: row.try_get("", "seat")?,
                chairs: serde_json::from_str(&chairs)
                    .map_err(|e| DbErr::Custom(format!("a game's chairs: {e}")))?,
            })
        })
        .collect::<Result<_, DbErr>>()?;

    Ok(Some(Detail {
        account,
        terms_accepted_at,
        confirmed_at,
        deck_list: decks,
        recent_games: games,
    }))
}

/// The handle and kind of each of `ids` that exists.
///
/// # Errors
///
/// When the query fails.
pub async fn named(db: &impl ConnectionTrait, ids: &[Uuid]) -> Result<Vec<(Uuid, Named)>, DbErr> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let joined = ids
        .iter()
        .map(Uuid::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT id, display_name, tag, guest FROM account \
             WHERE id::text = ANY(string_to_array($1::text, ','))",
            [joined.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok((
                row.try_get("", "id")?,
                Named {
                    display_name: row.try_get("", "display_name")?,
                    tag: row.try_get("", "tag")?,
                    guest: row.try_get("", "guest")?,
                },
            ))
        })
        .collect()
}

/// The last `days` UTC days up to and including today's, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn daily(
    db: &impl ConnectionTrait,
    now: OffsetDateTime,
    days: u32,
) -> Result<Vec<Day>, DbErr> {
    let days = days.clamp(1, MAX_DAYS);
    let today = crate::stats::start_of_day(now);
    let first = today - time::Duration::days(i64::from(days) - 1);
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "WITH days AS (SELECT generate_series($1::timestamptz, $2::timestamptz, \
                 interval '1 day') AS d) \
             SELECT to_char(d AT TIME ZONE 'UTC', 'YYYY-MM-DD') AS day, \
               (SELECT count(*) FROM account a WHERE NOT a.guest \
                  AND a.created_at >= d AND a.created_at < d + interval '1 day') AS registered, \
               (SELECT count(*) FROM account a WHERE a.guest \
                  AND a.created_at >= d AND a.created_at < d + interval '1 day') AS guests, \
               (SELECT count(*) FROM game_record g \
                  WHERE g.started_at >= d AND g.started_at < d + interval '1 day') AS started, \
               (SELECT count(*) FROM game_record g WHERE g.complete \
                  AND g.ended_at >= d AND g.ended_at < d + interval '1 day') AS finished, \
               (SELECT count(DISTINCT s.account_id) FROM game_record_seat s \
                  JOIN game_record g ON g.game_id = s.game_id \
                  WHERE s.account_id IS NOT NULL \
                  AND g.started_at >= d AND g.started_at < d + interval '1 day') AS players \
             FROM days ORDER BY d",
            [first.into(), today.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(Day {
                day: row.try_get("", "day")?,
                registered: row.try_get("", "registered")?,
                guests: row.try_get("", "guests")?,
                started: row.try_get("", "started")?,
                finished: row.try_get("", "finished")?,
                players: row.try_get("", "players")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_string_names_a_kind_and_an_order_or_nothing() {
        assert_eq!(Kind::parse(""), Some(Kind::All));
        assert_eq!(Kind::parse("guest"), Some(Kind::Guests));
        assert_eq!(Kind::parse("registered"), Some(Kind::Registered));
        assert_eq!(Kind::parse("admins"), None);
        assert_eq!(Order::parse(""), Some(Order::Newest));
        assert_eq!(Order::parse("active"), Some(Order::Active));
        assert_eq!(Order::parse("a.id; DROP TABLE account"), None);
    }

    #[test]
    fn every_order_ends_on_the_id() {
        for order in [
            Order::Newest,
            Order::Oldest,
            Order::Name,
            Order::Games,
            Order::Active,
        ] {
            assert!(order.sql().ends_with("a.id DESC") || order.sql().ends_with("a.id ASC"));
        }
    }
}
