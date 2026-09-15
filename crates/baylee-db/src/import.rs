//! The one-way door out of `gateway-store.json`.
//!
//! A gateway that already has accounts in a file must not lose them because
//! the storage underneath changed. On the first start against an empty
//! database, the file is read, written into the tables and then moved aside
//! by the caller — never deleted, because the safest thing to do with the only
//! copy of somebody's account is to keep it.
//!
//! # Why the old shape is declared here and not imported
//!
//! The legacy structs below are a *frozen* copy of what the file format was,
//! not a reference to what the gateway's types are now. Those two used to be
//! the same declaration, and that is the trap: the day a field is renamed in
//! the live code, an importer that shares the declaration silently stops
//! reading the files it exists to read, and the only symptom is an empty
//! database that came up clean.
//!
//! It is also why this crate does not depend on `baylee-gateway`. The
//! dependency would point the wrong way round — the thing being migrated
//! *from* cannot be the authority on what it looked like.

use crate::entity::prelude::*;
use crate::entity::{account, client_settings, confirmation, deck, session_token, standing_answer};
use anyhow::{Context, Result};
use sea_orm::{
    ActiveValue::{NotSet, Set},
    ConnectionTrait, DatabaseConnection, EntityTrait, PaginatorTrait, TransactionTrait,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use time::OffsetDateTime;
use uuid::Uuid;

/// What one import did, for the log line that says so.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Imported {
    /// Accounts written.
    pub accounts: usize,
    /// Decks written.
    pub decks: usize,
    /// Live sessions carried over.
    pub tokens: usize,
    /// Unspent confirmation links carried over.
    pub confirmations: usize,
    /// Standing answers written.
    pub answers: usize,
    /// Settings documents written.
    pub settings: usize,
    /// Rows dropped because the account they hang off was not in the file.
    ///
    /// Counted rather than ignored: a non-zero number here says the file was
    /// already inconsistent, which is worth seeing once even though the
    /// import is the right place for it to stop mattering.
    pub orphans: usize,
}

impl Imported {
    /// Whether anything at all was written.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self == Self {
            orphans: self.orphans,
            ..Self::default()
        }
    }
}

/// The whole legacy file. Every collection defaulted, because the file grew
/// its fields one release at a time and an older one is still a valid file.
#[derive(Debug, Default, Deserialize)]
pub struct Legacy {
    /// Accounts by id.
    #[serde(default)]
    pub accounts: HashMap<String, LegacyAccount>,
    /// Session tokens by hash.
    #[serde(default)]
    pub tokens: HashMap<String, LegacyToken>,
    /// Decks by id.
    #[serde(default)]
    pub decks: HashMap<String, LegacyDeck>,
    /// Standing answers by account id.
    #[serde(default)]
    pub automation: HashMap<String, Vec<LegacyAnswer>>,
    /// Client preferences by account id.
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
    /// Confirmation links by token hash.
    #[serde(default)]
    pub confirmations: HashMap<String, LegacyConfirmation>,
}

/// A legacy account row.
#[derive(Debug, Deserialize)]
pub struct LegacyAccount {
    /// Account id, a `UUIDv7` as a string.
    pub id: String,
    /// Login e-mail.
    pub email: String,
    /// Display name.
    pub display_name: String,
    /// Argon2id PHC hash.
    pub password_hash: String,
    /// Unix seconds.
    pub created_at: u64,
    /// Unix seconds, if confirmed.
    #[serde(default)]
    pub confirmed_at: Option<u64>,
    /// Registration language.
    #[serde(default)]
    pub lang: String,
}

/// A legacy session token.
#[derive(Debug, Deserialize)]
pub struct LegacyToken {
    /// SHA-256 of the bearer token.
    pub token_hash: String,
    /// Owning account id.
    pub account_id: String,
    /// Unix seconds.
    pub expires_at: u64,
}

/// A legacy confirmation link.
#[derive(Debug, Deserialize)]
pub struct LegacyConfirmation {
    /// SHA-256 of the token in the link.
    pub token_hash: String,
    /// The account it confirms.
    pub account_id: String,
    /// Unix seconds.
    pub expires_at: u64,
}

/// A legacy deck.
#[derive(Debug, Deserialize)]
pub struct LegacyDeck {
    /// Deck id.
    pub id: String,
    /// Owning account id.
    pub account_id: String,
    /// Deck name.
    pub name: String,
    /// Main deck lines.
    pub cards: Vec<String>,
    /// Sideboard lines.
    #[serde(default)]
    pub sideboard: Vec<String>,
    /// Commander name.
    #[serde(default)]
    pub commander: Option<String>,
    /// Sleeve image id.
    #[serde(default)]
    pub sleeve: Option<String>,
    /// Playmat image id.
    #[serde(default)]
    pub playmat: Option<String>,
    /// Unix seconds.
    pub updated_at: u64,
}

/// A legacy standing answer.
#[derive(Debug, Deserialize)]
pub struct LegacyAnswer {
    /// Registry index of the card.
    pub card: u32,
    /// Index into that card's ability list.
    pub ability: u32,
    /// The remembered answer.
    pub yes: bool,
}

/// Parse a store file's text.
///
/// # Errors
///
/// If the text is not the object the file format is.
pub fn read_legacy(text: &str) -> Result<Legacy> {
    serde_json::from_str(text).context("the store file is not a store")
}

/// Unix seconds as the database wants them.
///
/// A clamp rather than an error: a timestamp so far out of range that `time`
/// refuses it is a corrupted field, and losing one account over one bad
/// `expires_at` would be the wrong trade — the session it describes is
/// expired either way.
fn at(seconds: u64) -> OffsetDateTime {
    i64::try_from(seconds)
        .ok()
        .and_then(|s| OffsetDateTime::from_unix_timestamp(s).ok())
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

/// Read a legacy id, or mint a fresh one.
///
/// Every id the gateway ever wrote was a `UUIDv7`, so this parses in
/// practice. It mints instead of failing because a hand-edited development
/// store with `"id": "dev"` in it should import as an account with a new id,
/// not stop the gateway from starting.
fn id_of(raw: &str, minted: &mut HashMap<String, Uuid>) -> Uuid {
    if let Ok(id) = Uuid::parse_str(raw) {
        return id;
    }
    *minted.entry(raw.to_owned()).or_insert_with(Uuid::now_v7)
}

/// Every row a file turns into, and what it dropped on the way.
///
/// Separated from the writing on purpose: deciding which rows a file becomes
/// is where the judgement is — a remapped id, an orphan, a clamped
/// timestamp — and none of it needs a database to be wrong in. [`plan`] is
/// therefore pure, and the tests for all of it run in CI with no server.
pub struct Plan {
    /// Accounts, which must be written first.
    pub accounts: Vec<account::ActiveModel>,
    /// Decks.
    pub decks: Vec<deck::ActiveModel>,
    /// Live sessions.
    pub tokens: Vec<session_token::ActiveModel>,
    /// Unspent confirmation links.
    pub confirmations: Vec<confirmation::ActiveModel>,
    /// Standing answers.
    pub answers: Vec<standing_answer::ActiveModel>,
    /// Settings documents.
    pub settings: Vec<client_settings::ActiveModel>,
    /// Rows whose account was not in the file.
    pub orphans: usize,
}

impl Plan {
    /// What this plan would write.
    #[must_use]
    pub fn tally(&self) -> Imported {
        Imported {
            accounts: self.accounts.len(),
            decks: self.decks.len(),
            tokens: self.tokens.len(),
            confirmations: self.confirmations.len(),
            answers: self.answers.len(),
            settings: self.settings.len(),
            orphans: self.orphans,
        }
    }
}

/// Turn a parsed file into the rows it becomes.
///
/// `now` is a parameter rather than a clock read: the only field it fills is
/// a settings document's `updated_at`, which the file never carried, and a
/// function that read the clock could not be tested for what it does with
/// one.
#[must_use]
pub fn plan(legacy: &Legacy, now: OffsetDateTime) -> Plan {
    let mut minted = HashMap::new();
    let mut ids: HashMap<&str, Uuid> = HashMap::new();

    let accounts: Vec<account::ActiveModel> = legacy
        .accounts
        .values()
        .map(|a| {
            let id = id_of(&a.id, &mut minted);
            ids.insert(a.id.as_str(), id);
            account::ActiveModel {
                id: Set(id),
                email: Set(a.email.clone()),
                display_name: Set(a.display_name.clone()),
                // The tag is the database's. An imported account is a new
                // one as far as the sequence is concerned, and it gets its
                // number in whatever order the file hands them over — the
                // old store had nothing to carry in.
                tag: NotSet,
                password_hash: Set(a.password_hash.clone()),
                created_at: Set(at(a.created_at)),
                confirmed_at: Set(a.confirmed_at.map(at)),
                lang: Set(a.lang.clone()),
            }
        })
        .collect();

    // Everything below hangs off an account, so each of them asks the same
    // question first and answers a miss the same way.
    let mut owners = Owners {
        ids,
        orphans: 0,
        minted,
    };

    Plan {
        accounts,
        decks: owners.decks(legacy),
        tokens: owners.tokens(legacy),
        confirmations: owners.confirmations(legacy),
        answers: owners.answers(legacy),
        settings: owners.settings(legacy, now),
        orphans: owners.orphans,
    }
}

/// Which account each child row belongs to, and how many found none.
///
/// A struct rather than five closures over one counter: a closure that
/// borrows `orphans` mutably cannot be called from inside an iterator that
/// also borrows it, which is the shape all five of these want.
struct Owners<'a> {
    ids: HashMap<&'a str, Uuid>,
    orphans: usize,
    minted: HashMap<String, Uuid>,
}

impl<'a> Owners<'a> {
    /// The account a row names, counting the miss when there is none.
    ///
    /// A miss is dropped rather than fatal. The old format enforced no
    /// reference at all, so a file *can* hold a deck whose owner is gone;
    /// writing it would fail the foreign key and take the whole import with
    /// it, which would lose every account over one stale row.
    fn owner(&mut self, raw: &str) -> Option<Uuid> {
        let found = self.ids.get(raw).copied();
        if found.is_none() {
            self.orphans += 1;
        }
        found
    }

    fn decks(&mut self, legacy: &'a Legacy) -> Vec<deck::ActiveModel> {
        legacy
            .decks
            .values()
            .filter_map(|d| {
                let account_id = self.owner(&d.account_id)?;
                Some(deck::ActiveModel {
                    id: Set(id_of(&d.id, &mut self.minted)),
                    account_id: Set(account_id),
                    name: Set(d.name.clone()),
                    cards: Set(d.cards.clone()),
                    sideboard: Set(d.sideboard.clone()),
                    commander: Set(d.commander.clone()),
                    sleeve: Set(d.sleeve.clone()),
                    playmat: Set(d.playmat.clone()),
                    updated_at: Set(at(d.updated_at)),
                })
            })
            .collect()
    }

    fn tokens(&mut self, legacy: &'a Legacy) -> Vec<session_token::ActiveModel> {
        legacy
            .tokens
            .values()
            .filter_map(|t| {
                Some(session_token::ActiveModel {
                    account_id: Set(self.owner(&t.account_id)?),
                    token_hash: Set(t.token_hash.clone()),
                    expires_at: Set(at(t.expires_at)),
                })
            })
            .collect()
    }

    fn confirmations(&mut self, legacy: &'a Legacy) -> Vec<confirmation::ActiveModel> {
        legacy
            .confirmations
            .values()
            .filter_map(|c| {
                Some(confirmation::ActiveModel {
                    account_id: Set(self.owner(&c.account_id)?),
                    token_hash: Set(c.token_hash.clone()),
                    expires_at: Set(at(c.expires_at)),
                })
            })
            .collect()
    }

    fn answers(&mut self, legacy: &'a Legacy) -> Vec<standing_answer::ActiveModel> {
        let mut rows = Vec::new();
        for (raw, answers) in &legacy.automation {
            let Some(account_id) = self.owner(raw) else {
                // One miss, not one per answer: the account is the row that
                // is gone, and counting its answers separately would report
                // a file as forty times more broken than it is.
                continue;
            };
            rows.extend(answers.iter().map(|r| standing_answer::ActiveModel {
                account_id: Set(account_id),
                card: Set(i64::from(r.card)),
                ability: Set(i64::from(r.ability)),
                yes: Set(r.yes),
            }));
        }
        rows
    }

    fn settings(
        &mut self,
        legacy: &'a Legacy,
        now: OffsetDateTime,
    ) -> Vec<client_settings::ActiveModel> {
        legacy
            .settings
            .iter()
            .filter_map(|(raw, doc)| {
                Some(client_settings::ActiveModel {
                    account_id: Set(self.owner(raw)?),
                    doc: Set(doc.clone()),
                    updated_at: Set(now),
                })
            })
            .collect()
    }
}

/// Write a legacy store into an empty database, all of it or none of it.
///
/// Answers `None` when the database already holds accounts — importing twice
/// would either fail on the unique index or, worse, half-succeed.
///
/// The whole import is one transaction, and that is the point rather than a
/// tidiness: this is the only code in the workspace that touches somebody's
/// real accounts, and a failure partway through would commit the accounts,
/// leave the file in place (correct) and then find `count > 0` on the next
/// start — so the decks would never be imported and nothing would say so.
/// The count is asked *inside* the transaction for the same reason.
///
/// # Errors
///
/// If the database refuses a write.
pub async fn import_legacy(db: &DatabaseConnection, legacy: &Legacy) -> Result<Option<Imported>> {
    let txn = db.begin().await.context("opening the import")?;

    if Account::find()
        .count(&txn)
        .await
        .context("counting accounts")?
        > 0
    {
        return Ok(None);
    }

    let plan = plan(legacy, OffsetDateTime::now_utc());
    let tally = plan.tally();

    // Accounts first and on their own: every other table is a foreign key
    // into this one, so a child written before its parent fails rather than
    // dangles.
    insert_all::<Account, _>(&txn, plan.accounts).await?;
    insert_all::<Deck, _>(&txn, plan.decks).await?;
    insert_all::<SessionToken, _>(&txn, plan.tokens).await?;
    insert_all::<Confirmation, _>(&txn, plan.confirmations).await?;
    insert_all::<StandingAnswer, _>(&txn, plan.answers).await?;
    insert_all::<ClientSettings, _>(&txn, plan.settings).await?;

    txn.commit().await.context("committing the import")?;

    Ok(Some(tally))
}

/// Insert in batches, because one statement per row over a network is the
/// slow way and one statement for ten thousand rows exceeds what a parameter
/// list may hold.
///
/// Takes any connection rather than the pool, so the caller decides whether
/// these rows are part of a transaction.
async fn insert_all<E, A>(db: &impl ConnectionTrait, rows: Vec<A>) -> Result<()>
where
    E: EntityTrait,
    A: sea_orm::ActiveModelTrait<Entity = E> + Send,
{
    /// Six columns at 500 rows is 3000 parameters, comfortably under the
    /// 65535 a Postgres statement allows.
    const BATCH: usize = 500;

    for chunk in rows.into_iter().collect::<Vec<_>>().chunks(BATCH) {
        E::insert_many(chunk.to_vec())
            .exec_without_returning(db)
            .await
            .with_context(|| format!("importing into {}", E::default().table_name()))?;
    }
    Ok(())
}

/// Read a store file and import it, answering `None` when there is nothing to
/// do — no file, or a database that already has accounts.
///
/// # Errors
///
/// If the file exists but cannot be read or parsed, or the database refuses.
pub async fn import_file(db: &DatabaseConnection, path: &Path) -> Result<Option<Imported>> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(None);
    };
    let legacy =
        read_legacy(&text).with_context(|| format!("reading the store at {}", path.display()))?;
    import_legacy(db, &legacy).await
}

/// Where a store file goes once it has been imported.
///
/// Kept, not deleted: it is the only copy of somebody's account until the
/// database has been backed up once.
#[must_use]
pub fn imported_name(path: &Path) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".imported");
    name.into()
}

#[cfg(test)]
mod tests;
