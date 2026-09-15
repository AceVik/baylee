//! What the gateway remembers, and how it asks.
//!
//! Accounts, sessions, decks, confirmation links, standing answers and client
//! preferences, in PostgreSQL through [`baylee_db`]. This module is the only
//! place in the gateway that knows there is a database: every route calls a
//! function here and gets a plain struct back.
//!
//! # The types are the wire, not the schema
//!
//! `Account::id` is a `String` and `Account::created_at` is unix seconds,
//! while the columns behind them are a `uuid` and a `timestamptz`. That is
//! deliberate and is the seam this module exists to hold. Those two shapes
//! are what the client parses — `DeckSummary`, `GameSummary` and the lobby's
//! JSON all read them — so a change of storage that changed them would be a
//! protocol change wearing a migration's clothes. The conversion happens
//! here, at the edge, in four small functions nobody else calls.
//!
//! # What stopped being possible
//!
//! Registering used to read "is this address taken", decide, and then write,
//! with nothing in between to stop two requests doing it at once. The
//! uniqueness now lives in the index, so the second writer is refused by the
//! database rather than by a check it had already passed.

use crate::auth;
use anyhow::Result;
use baylee_db::entity::account::Entity as Accounts;
use baylee_db::entity::client_settings::Entity as Settings;
use baylee_db::entity::confirmation::Entity as Confirmations;
use baylee_db::entity::deck::Entity as Decks;
use baylee_db::entity::session_token::Entity as Sessions;
use baylee_db::entity::standing_answer::Entity as Answers;
use baylee_db::entity::{
    account, client_settings, confirmation, deck, session_token, standing_answer,
};
use sea_orm::{
    ActiveValue::{NotSet, Set},
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    sea_query::{Expr, ExprTrait, Func, OnConflict},
};
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

/// What a registration supplies, which is everything the database does not.
///
/// Separate from [`Account`] so a caller *cannot* supply a tag or an id.
/// The tag is an identity column, and a value written into it by hand would
/// not move the sequence and would collide with a later registration; the
/// id is `DEFAULT uuidv7()`, and a `UUIDv7` minted by a gateway is ordered
/// by *that gateway's* clock, which is the one thing several of them behind
/// one database cannot agree on.
#[derive(Clone, Debug)]
pub struct NewAccount {
    /// Login e-mail.
    pub email: String,
    /// Display name shown in the lobby. Not unique.
    pub display_name: String,
    /// Argon2id PHC password hash.
    pub password_hash: String,
    /// Created at (unix seconds).
    pub created_at: u64,
    /// When the address was confirmed, if it has been.
    pub confirmed_at: Option<u64>,
    /// The language the account registered in, for the mail it is sent.
    pub lang: String,
}

/// A registered account. The username is the e-mail address; the
/// display name is shown to other players.
#[derive(Clone, Debug)]
pub struct Account {
    /// Account id (`UUIDv7`).
    pub id: String,
    /// Login e-mail (lowercased, unique).
    pub email: String,
    /// Display name shown in the lobby. Not unique — [`Account::tag`] is.
    pub display_name: String,
    /// The discriminator, handed out by the database. See [`crate::handle`].
    pub tag: i32,
    /// Argon2id PHC password hash.
    pub password_hash: String,
    /// When the address was confirmed, if it has been.
    ///
    /// `None` on a gateway that sends mail means the account cannot log in
    /// yet. On a gateway with no SMTP configured it means nothing at all —
    /// see [`crate::mail::Mailer::required`] — which is why this is an
    /// `Option` and not a bool: "never asked" and "asked and not answered"
    /// are the same field, and only the mailer decides which one matters.
    pub confirmed_at: Option<u64>,
    /// The language the account registered in, for the mail it is sent.
    pub lang: String,
}

/// An outstanding "confirm your address" link.
///
/// Only the hash is kept, for the same reason a session token's is: a link
/// that reached a mailbox is a working login, and a stored one would be a
/// row that grants an account to whoever can read the table.
#[derive(Clone, Debug)]
pub struct Confirmation {
    /// SHA-256 of the token in the link, as bytes.
    pub token_hash: Vec<u8>,
    /// The account it confirms.
    pub account_id: String,
    /// Expiry (unix seconds). A link that never expired would be a password
    /// that never expired, sitting in a mailbox.
    pub expires_at: u64,
}

/// A stored session token (only the SHA-256 hash is kept).
#[derive(Clone, Debug)]
pub struct StoredToken {
    /// SHA-256 of the bearer token, as bytes.
    pub token_hash: Vec<u8>,
    /// Owning account id.
    pub account_id: String,
    /// Expiry (unix seconds, sliding).
    pub expires_at: u64,
}

/// What saving a new deck supplies: everything but the id, which the
/// database mints.
#[derive(Clone, Debug)]
pub struct NewDeck {
    /// Whose deck it is.
    pub account_id: String,
    /// What the player called it.
    pub name: String,
    /// The main deck, as `"N Card Name"` rows.
    pub cards: Vec<String>,
    /// The sideboard, in the same spelling.
    pub sideboard: Vec<String>,
    /// The commander, if the deck has one.
    pub commander: Option<String>,
    /// Image id of the sleeve.
    pub sleeve: Option<String>,
    /// Image id of the playmat.
    pub playmat: Option<String>,
    /// Last written (unix seconds).
    pub updated_at: u64,
}

/// A player's deck (card names; resolved against the registry at use).
#[derive(Clone, Debug)]
pub struct Deck {
    /// Deck id (`UUIDv7`).
    pub id: String,
    /// Owning account id.
    pub account_id: String,
    /// Deck name.
    pub name: String,
    /// Card lines (`"N Card Name"`).
    pub cards: Vec<String>,
    /// Sideboard lines, in the same form. Cards outside the game a seat may
    /// reach; never shuffled into the library.
    pub sideboard: Vec<String>,
    /// Commander card name, if any.
    pub commander: Option<String>,
    /// Image id of the sleeve this deck's cards show face-down. `None` means
    /// the client draws its own generated back.
    pub sleeve: Option<String>,
    /// Image id of the playmat this deck's seat plays on.
    pub playmat: Option<String>,
    /// Last update (unix seconds).
    pub updated_at: u64,
}

/// A remembered answer to one optional ability, replayed into every game
/// the account sits down to.
///
/// The engine addresses standing answers by `AbilityRef { card, index }`,
/// a handle that says nothing about a particular game — which is exactly
/// what makes it storable here. "Always gain the life from Ondu Cleric's
/// rally trigger" is a preference about a *card*, so it belongs to the
/// account and not to the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StandingAnswer {
    /// Registry index of the card the ability is printed on.
    pub card: u32,
    /// Index into that card's ability list (`AbilityRef::index`); the
    /// reserved values above `AbilityRef::FIRST_RESERVED` name the
    /// abilities that are not listed on the card.
    pub ability: u32,
    /// What to answer without asking.
    pub yes: bool,
}

// ---------------------------------------------------------- the two edges

/// Unix seconds as a timestamp.
fn at(seconds: u64) -> OffsetDateTime {
    i64::try_from(seconds)
        .ok()
        .and_then(|s| OffsetDateTime::from_unix_timestamp(s).ok())
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

/// A timestamp as unix seconds. Negative is before 1970 and is clamped
/// rather than wrapped: there is no field here a pre-epoch value would be
/// meaningful in, and `u64::MAX` seconds from now is a worse answer than
/// zero.
fn secs(at: OffsetDateTime) -> u64 {
    u64::try_from(at.unix_timestamp()).unwrap_or(0)
}

/// A stored id as the gateway hands it around.
fn id(uuid: Uuid) -> String {
    uuid.to_string()
}

/// A handed-around id as the database wants it.
///
/// Answers `None` for anything that is not a `UUID`, which is how a route
/// that was given a deck id out of a URL refuses it: a malformed id is a
/// deck that does not exist, not a query that fails.
fn uuid(raw: &str) -> Option<Uuid> {
    Uuid::parse_str(raw).ok()
}

impl From<account::Model> for Account {
    fn from(row: account::Model) -> Self {
        Self {
            id: id(row.id),
            email: row.email,
            display_name: row.display_name,
            tag: row.tag,
            password_hash: row.password_hash,
            confirmed_at: row.confirmed_at.map(secs),
            lang: row.lang,
        }
    }
}

impl From<deck::Model> for Deck {
    fn from(row: deck::Model) -> Self {
        Self {
            id: id(row.id),
            account_id: id(row.account_id),
            name: row.name,
            cards: row.cards,
            sideboard: row.sideboard,
            commander: row.commander,
            sleeve: row.sleeve,
            playmat: row.playmat,
            updated_at: secs(row.updated_at),
        }
    }
}

impl From<confirmation::Model> for Confirmation {
    fn from(row: confirmation::Model) -> Self {
        Self {
            token_hash: row.token_hash,
            account_id: id(row.account_id),
            expires_at: secs(row.expires_at),
        }
    }
}

// -------------------------------------------------------------- accounts

/// One account by id, or `None`.
///
/// # Errors
///
/// If the database refuses.
pub async fn account(db: &DatabaseConnection, account_id: &str) -> Result<Option<Account>> {
    let Some(id) = uuid(account_id) else {
        return Ok(None);
    };
    Ok(Accounts::find_by_id(id).one(db).await?.map(Into::into))
}

/// One account by login e-mail, case-insensitively.
///
/// The comparison is `lower(email) = lower($1)`, which is the expression the
/// unique index is built on — so this is an index lookup and not the scan an
/// `ILIKE` would have been.
///
/// # Errors
///
/// If the database refuses.
pub async fn account_by_email(db: &DatabaseConnection, email: &str) -> Result<Option<Account>> {
    Ok(Accounts::find()
        .filter(Expr::expr(Func::lower(Expr::col(account::Column::Email))).eq(email.to_lowercase()))
        .one(db)
        .await?
        .map(Into::into))
}

/// Handles for a set of account ids, for the lobby's rosters.
///
/// `Alice#af03`, not `Alice`: a display name is no longer unique, so a
/// roster of bare names would put two people called Alice at one table with
/// nothing between them. This is the only place the two halves are joined —
/// every roster and every `GameSetup` goes through here, and nothing below
/// the gateway ever learns that a tag exists.
///
/// One query rather than one per seat: a full table at eight chairs was eight
/// round trips to name eight people.
///
/// # Errors
///
/// If the database refuses.
pub async fn display_names(
    db: &DatabaseConnection,
    wanted: impl IntoIterator<Item = String>,
) -> Result<HashMap<String, String>> {
    let ids: Vec<Uuid> = wanted.into_iter().filter_map(|w| uuid(&w)).collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(Accounts::find()
        .filter(account::Column::Id.is_in(ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| {
            (
                id(row.id),
                crate::handle::handle(&row.display_name, row.tag),
            )
        })
        .collect())
}

/// The account a tag names, for a player looking somebody up.
///
/// # Errors
///
/// If the database refuses.
pub async fn account_by_tag(db: &DatabaseConnection, tag: i32) -> Result<Option<Account>> {
    Ok(Accounts::find()
        .filter(account::Column::Tag.eq(tag))
        .one(db)
        .await?
        .map(Into::into))
}

/// Write a new account, answering `None` when the address is already taken.
///
/// The refusal comes from the unique index rather than from a check, which is
/// what closes the window the file-backed version had: two registrations of
/// one address could both read "free" and both write. A *display name* is no
/// longer among the things that can be taken.
///
/// What comes back is the row the database made, not the one that went in,
/// because the tag is the database's to hand out: [`account::Column::Tag`]
/// is an identity column and is `NotSet` on the way in. The caller needs it
/// — the confirmation mail names the account it is about.
///
/// # Errors
///
/// If the database refuses for any reason other than that clash.
pub async fn create_account(db: &DatabaseConnection, new: NewAccount) -> Result<Option<Account>> {
    let row = account::ActiveModel {
        id: NotSet,
        email: Set(new.email),
        display_name: Set(new.display_name),
        tag: NotSet,
        password_hash: Set(new.password_hash),
        created_at: Set(at(new.created_at)),
        confirmed_at: Set(new.confirmed_at.map(at)),
        lang: Set(new.lang),
    };
    match Accounts::insert(row).exec_with_returning(db).await {
        Ok(made) => Ok(Some(made.into())),
        Err(e) if is_taken(&e) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Whether a write failed because something unique already exists.
fn is_taken(e: &sea_orm::DbErr) -> bool {
    matches!(e, sea_orm::DbErr::RecordNotInserted) || e.to_string().contains("duplicate key value")
}

/// Mark an address confirmed. Silent when the account is gone.
///
/// # Errors
///
/// If the database refuses.
pub async fn confirm_account(db: &DatabaseConnection, account_id: &str, now: u64) -> Result<()> {
    let Some(id) = uuid(account_id) else {
        return Ok(());
    };
    Accounts::update_many()
        .col_expr(account::Column::ConfirmedAt, Expr::value(at(now)))
        .filter(account::Column::Id.eq(id))
        .filter(account::Column::ConfirmedAt.is_null())
        .exec(db)
        .await?;
    Ok(())
}

// ------------------------------------------------------------- sessions

/// Store a session token.
///
/// # Errors
///
/// If the database refuses.
pub async fn put_token(db: &DatabaseConnection, token: StoredToken) -> Result<()> {
    let Some(account_id) = uuid(&token.account_id) else {
        return Ok(());
    };
    Sessions::insert(session_token::ActiveModel {
        token_hash: Set(token.token_hash),
        account_id: Set(account_id),
        expires_at: Set(at(token.expires_at)),
    })
    .exec(db)
    .await?;
    Ok(())
}

/// Resolve a bearer token to its account id, sliding the expiry.
///
/// # Why the renewal is not on every request
///
/// The file-backed version bumped `expires_at` each time a token was seen,
/// which cost nothing because the whole store was already in memory. Against
/// a database it is an `UPDATE` per authenticated request — every deck list,
/// every lobby poll, every settings read — to move a deadline that is twelve
/// hours away.
///
/// So it renews only once the token is past its half-life. A session is still
/// kept alive by use, which is the whole point of a sliding expiry, and the
/// write happens at most once per six hours per session instead of once per
/// request. The cost is stated rather than hidden: a token that is used
/// constantly and then abandoned lapses up to six hours earlier than it
/// would have, which is the direction an error should go.
///
/// # Errors
///
/// If the database refuses.
pub async fn resolve_token(
    db: &DatabaseConnection,
    token: &str,
    now: u64,
) -> Result<Option<String>> {
    let hash = auth::token_digest(token);
    let Some(row) = Sessions::find_by_id(hash.clone()).one(db).await? else {
        return Ok(None);
    };
    let expires = secs(row.expires_at);
    if expires < now {
        Sessions::delete_by_id(hash).exec(db).await?;
        return Ok(None);
    }

    let ttl = auth::TOKEN_TTL.as_secs();
    if expires.saturating_sub(now) < ttl / 2 {
        Sessions::update_many()
            .col_expr(session_token::Column::ExpiresAt, Expr::value(at(now + ttl)))
            .filter(session_token::Column::TokenHash.eq(hash))
            .exec(db)
            .await?;
    }
    Ok(Some(id(row.account_id)))
}

/// Sign one session out.
///
/// # Errors
///
/// If the database refuses.
pub async fn drop_token(db: &DatabaseConnection, token: &str) -> Result<()> {
    Sessions::delete_by_id(auth::token_digest(token))
        .exec(db)
        .await?;
    Ok(())
}

/// Remove every lapsed session, answering how many went.
///
/// One `DELETE ... WHERE expires_at <= $1` on the index the migration made
/// for it, instead of the whole map walked in memory.
///
/// # Errors
///
/// If the database refuses.
pub async fn sweep_tokens(db: &DatabaseConnection, now: u64) -> Result<u64> {
    Ok(Sessions::delete_many()
        .filter(session_token::Column::ExpiresAt.lte(at(now)))
        .exec(db)
        .await?
        .rows_affected)
}

// -------------------------------------------------------- confirmations

/// Drop every confirmation link for one account, and every link that has
/// expired.
///
/// Both halves matter: a fresh link has to invalidate the last one that was
/// mailed, or a resend would leave two working links behind; and nothing else
/// ever walks this table, so expiry has to be swept somewhere.
///
/// # Errors
///
/// If the database refuses.
pub async fn clear_confirmations(
    db: &DatabaseConnection,
    account_id: &str,
    now: u64,
) -> Result<()> {
    if let Some(id) = uuid(account_id) {
        Confirmations::delete_many()
            .filter(confirmation::Column::AccountId.eq(id))
            .exec(db)
            .await?;
    }
    Confirmations::delete_many()
        .filter(confirmation::Column::ExpiresAt.lte(at(now)))
        .exec(db)
        .await?;
    Ok(())
}

/// Store a confirmation link.
///
/// # Errors
///
/// If the database refuses.
pub async fn put_confirmation(db: &DatabaseConnection, link: Confirmation) -> Result<()> {
    let Some(account_id) = uuid(&link.account_id) else {
        return Ok(());
    };
    Confirmations::insert(confirmation::ActiveModel {
        token_hash: Set(link.token_hash),
        account_id: Set(account_id),
        expires_at: Set(at(link.expires_at)),
    })
    .exec(db)
    .await?;
    Ok(())
}

/// Spend a confirmation link: read it and remove it in one step, so a link
/// followed twice works once.
///
/// # Errors
///
/// If the database refuses.
pub async fn take_confirmation(
    db: &DatabaseConnection,
    token_hash: &[u8],
) -> Result<Option<Confirmation>> {
    let found = Confirmations::find_by_id(token_hash.to_owned())
        .one(db)
        .await?;
    let Some(found) = found else {
        return Ok(None);
    };
    let removed = Confirmations::delete_by_id(token_hash.to_owned())
        .exec(db)
        .await?;
    // Somebody else spent it between the read and the delete. Answering
    // `None` is what makes "a link works once" true rather than nearly true.
    if removed.rows_affected == 0 {
        return Ok(None);
    }
    Ok(Some(found.into()))
}

// ----------------------------------------------------------------- decks

/// One account's decks, newest first.
///
/// # Errors
///
/// If the database refuses.
pub async fn decks_of(db: &DatabaseConnection, account_id: &str) -> Result<Vec<Deck>> {
    let Some(id) = uuid(account_id) else {
        return Ok(Vec::new());
    };
    Ok(Decks::find()
        .filter(deck::Column::AccountId.eq(id))
        .order_by_desc(deck::Column::UpdatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// One deck by id.
///
/// # Errors
///
/// If the database refuses.
pub async fn deck(db: &DatabaseConnection, deck_id: &str) -> Result<Option<Deck>> {
    let Some(id) = uuid(deck_id) else {
        return Ok(None);
    };
    Ok(Decks::find_by_id(id).one(db).await?.map(Into::into))
}

/// Several decks by id, in one query.
///
/// Used where a table's seats each name one: a rematch reads every played
/// deck at once rather than once per chair.
///
/// # Errors
///
/// If the database refuses.
pub async fn decks_by_id(
    db: &DatabaseConnection,
    wanted: impl IntoIterator<Item = String>,
) -> Result<HashMap<String, Deck>> {
    let ids: Vec<Uuid> = wanted.into_iter().filter_map(|w| uuid(&w)).collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(Decks::find()
        .filter(deck::Column::Id.is_in(ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (id(row.id), row.into()))
        .collect())
}

/// Save a new deck and answer with the id the database gave it.
///
/// Separate from [`put_deck`] because the two are different statements once
/// the id is the database's: this one inserts and reads the key back, and
/// `put_deck` is the upsert that an edit of an existing deck goes through.
/// One function doing both would have to decide which it was by looking at
/// whether an id was set, which is exactly the "is it there yet" reasoning
/// the id moved to the database to be rid of.
///
/// # Errors
///
/// If the database refuses.
pub async fn create_deck(db: &DatabaseConnection, new: NewDeck) -> Result<Option<String>> {
    let Some(account_id) = uuid(&new.account_id) else {
        return Ok(None);
    };
    let made = Decks::insert(deck::ActiveModel {
        id: NotSet,
        account_id: Set(account_id),
        name: Set(new.name),
        cards: Set(new.cards),
        sideboard: Set(new.sideboard),
        commander: Set(new.commander),
        sleeve: Set(new.sleeve),
        playmat: Set(new.playmat),
        updated_at: Set(at(new.updated_at)),
    })
    .exec_with_returning(db)
    .await?;
    Ok(Some(id(made.id)))
}

/// Write a deck, replacing one of the same id.
///
/// # Errors
///
/// If the database refuses.
pub async fn put_deck(db: &DatabaseConnection, saved: Deck) -> Result<()> {
    let (Some(id), Some(account_id)) = (uuid(&saved.id), uuid(&saved.account_id)) else {
        return Ok(());
    };
    Decks::insert(deck::ActiveModel {
        id: Set(id),
        account_id: Set(account_id),
        name: Set(saved.name),
        cards: Set(saved.cards),
        sideboard: Set(saved.sideboard),
        commander: Set(saved.commander),
        sleeve: Set(saved.sleeve),
        playmat: Set(saved.playmat),
        updated_at: Set(at(saved.updated_at)),
    })
    .on_conflict(
        OnConflict::column(deck::Column::Id)
            .update_columns([
                deck::Column::Name,
                deck::Column::Cards,
                deck::Column::Sideboard,
                deck::Column::Commander,
                deck::Column::Sleeve,
                deck::Column::Playmat,
                deck::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}

/// Delete one of an account's decks, answering whether it was theirs to
/// delete.
///
/// The ownership is part of the `DELETE` rather than a read before it: a
/// check and then a write is two statements another request can slip
/// between, and this way the row is only ever removed by the account that
/// owns it.
///
/// # Errors
///
/// If the database refuses.
pub async fn delete_deck(db: &DatabaseConnection, deck_id: &str, account_id: &str) -> Result<bool> {
    let (Some(id), Some(owner)) = (uuid(deck_id), uuid(account_id)) else {
        return Ok(false);
    };
    Ok(Decks::delete_many()
        .filter(deck::Column::Id.eq(id))
        .filter(deck::Column::AccountId.eq(owner))
        .exec(db)
        .await?
        .rows_affected
        > 0)
}

// ------------------------------------------------- automation, settings

/// One account's standing answers.
///
/// # Errors
///
/// If the database refuses.
pub async fn automation_of(
    db: &DatabaseConnection,
    account_id: &str,
) -> Result<Vec<StandingAnswer>> {
    let Some(id) = uuid(account_id) else {
        return Ok(Vec::new());
    };
    Ok(Answers::find()
        .filter(standing_answer::Column::AccountId.eq(id))
        .all(db)
        .await?
        .into_iter()
        .map(|row| StandingAnswer {
            card: u32::try_from(row.card).unwrap_or(0),
            ability: u32::try_from(row.ability).unwrap_or(0),
            yes: row.yes,
        })
        .collect())
}

/// Replace one account's standing answers with this list.
///
/// A replacement and not a merge, because that is what the route means: the
/// client sends the whole list it believes in, and an answer it left out is
/// one the player has stopped standing by.
///
/// # Errors
///
/// If the database refuses.
pub async fn put_automation(
    db: &DatabaseConnection,
    account_id: &str,
    answers: Vec<StandingAnswer>,
) -> Result<()> {
    let Some(id) = uuid(account_id) else {
        return Ok(());
    };
    Answers::delete_many()
        .filter(standing_answer::Column::AccountId.eq(id))
        .exec(db)
        .await?;
    if answers.is_empty() {
        return Ok(());
    }
    Answers::insert_many(answers.into_iter().map(|a| standing_answer::ActiveModel {
        account_id: Set(id),
        card: Set(i64::from(a.card)),
        ability: Set(i64::from(a.ability)),
        yes: Set(a.yes),
    }))
    // The client is allowed to send the same ability twice; the last one it
    // named is the one it means.
    .on_conflict(
        OnConflict::columns([
            standing_answer::Column::AccountId,
            standing_answer::Column::Card,
            standing_answer::Column::Ability,
        ])
        .update_column(standing_answer::Column::Yes)
        .to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}

/// One account's client preferences, if it has written any.
///
/// # Errors
///
/// If the database refuses.
pub async fn settings_of(
    db: &DatabaseConnection,
    account_id: &str,
) -> Result<Option<serde_json::Value>> {
    let Some(id) = uuid(account_id) else {
        return Ok(None);
    };
    Ok(Settings::find_by_id(id).one(db).await?.map(|row| row.doc))
}

/// Write one account's client preferences.
///
/// # Errors
///
/// If the database refuses.
pub async fn put_settings(
    db: &DatabaseConnection,
    account_id: &str,
    doc: serde_json::Value,
) -> Result<()> {
    let Some(id) = uuid(account_id) else {
        return Ok(());
    };
    Settings::insert(client_settings::ActiveModel {
        account_id: Set(id),
        doc: Set(doc),
        updated_at: Set(OffsetDateTime::now_utc()),
    })
    .on_conflict(
        OnConflict::column(client_settings::Column::AccountId)
            .update_columns([
                client_settings::Column::Doc,
                client_settings::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}
