//! Terms of use an admin published (`docs/protocol.md` §"Terms of use
//! (WG-1)"): hand-written statements, as in [`crate::card_blocks`]. A version
//! never changes once stored, and the newest one is in force; the gateway
//! checks the texts and computes the digest before it stores one.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement, TransactionTrait};
use time::OffsetDateTime;

/// One language of a version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermsText {
    /// Its primary language tag.
    pub lang: String,
    /// The date it names as its last change, if any.
    pub updated: Option<String>,
    /// The Markdown.
    pub markdown: String,
}

/// A published version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Published {
    /// What players accept.
    pub version: String,
    /// The digest of its texts, as the gateway computed it.
    pub digest: Vec<u8>,
    /// When it was published.
    pub published_at: OffsetDateTime,
    /// By whom (the audit actor).
    pub published_by: String,
    /// Its languages, in tag order; empty in [`versions`].
    pub texts: Vec<TermsText>,
}

fn read(row: &sea_orm::QueryResult) -> Result<Published, DbErr> {
    Ok(Published {
        version: row.try_get("", "version")?,
        digest: row.try_get("", "digest")?,
        published_at: row.try_get("", "published_at")?,
        published_by: row.try_get("", "published_by")?,
        texts: Vec::new(),
    })
}

/// The version in force, with its texts, or none.
///
/// # Errors
///
/// When a query fails.
pub async fn current(db: &impl ConnectionTrait) -> Result<Option<Published>, DbErr> {
    let Some(row) = db
        .query_one_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT version, digest, published_at, published_by FROM gateway_terms \
             ORDER BY published_at DESC, version DESC LIMIT 1",
        ))
        .await?
    else {
        return Ok(None);
    };
    let mut published = read(&row)?;
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT lang, updated, markdown FROM gateway_terms_text WHERE version = $1 ORDER BY lang",
            [published.version.clone().into()],
        ))
        .await?;
    published.texts = rows
        .iter()
        .map(|row| {
            Ok(TermsText {
                lang: row.try_get("", "lang")?,
                updated: row.try_get("", "updated")?,
                markdown: row.try_get("", "markdown")?,
            })
        })
        .collect::<Result<_, DbErr>>()?;
    Ok(Some(published))
}

/// Every published version, newest first, without texts.
///
/// # Errors
///
/// When the query fails.
pub async fn versions(db: &impl ConnectionTrait) -> Result<Vec<Published>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT version, digest, published_at, published_by FROM gateway_terms \
             ORDER BY published_at DESC, version DESC",
        ))
        .await?;
    rows.iter().map(read).collect()
}

/// Stores a new version and its texts in one transaction; `false` when the
/// version already exists, which is never overwritten.
///
/// # Errors
///
/// When a statement fails, a bound is broken (the tables check them), or
/// the commit fails.
pub async fn publish(
    db: &DatabaseConnection,
    version: &str,
    digest: &[u8; 32],
    texts: &[TermsText],
    by: &str,
) -> Result<bool, DbErr> {
    let tx = db.begin().await?;
    let inserted = tx
        .execute_raw(Statement::from_sql_and_values(
            tx.get_database_backend(),
            "INSERT INTO gateway_terms (version, digest, published_by) VALUES ($1, $2, $3) \
             ON CONFLICT DO NOTHING",
            [version.into(), digest.to_vec().into(), by.into()],
        ))
        .await?
        .rows_affected();
    if inserted == 0 {
        tx.rollback().await?;
        return Ok(false);
    }
    for text in texts {
        tx.execute_raw(Statement::from_sql_and_values(
            tx.get_database_backend(),
            "INSERT INTO gateway_terms_text (version, lang, updated, markdown) VALUES ($1, $2, $3, $4)",
            [
                version.into(),
                text.lang.clone().into(),
                text.updated.clone().into(),
                text.markdown.clone().into(),
            ],
        ))
        .await?;
    }
    tx.commit().await?;
    Ok(true)
}
