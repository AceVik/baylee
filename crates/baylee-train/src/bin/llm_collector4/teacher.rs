//! The teacher accounts' way in since the cutover of 30.10.2026
//! (`docs/central-login.md` §"Cutover"). No gateway registers anybody or
//! signs anybody in with a password any more, and a guest may not ask for a
//! chair ticket, so the collector writes each slot's teacher account and its
//! session into its own gateway's database, as the gateway's own e2e harness
//! does (`crates/baylee-gateway/tests/common/mod.rs`, `login`).
//!
//! Only into its own: [`OwnDatabase::prove`] has the gateway behind the
//! collector's admin token make a one-use closed-beta key, finds that key's
//! row in `--database-url`, has the gateway revoke it and finds the
//! revocation there too. A database that does not show both is another
//! gateway's, or none's, and is refused before anything is written to it:
//! the gateways running beside the collector's have their own databases and
//! their own admin tokens.
//!
//! The account is what registration made before the cutover: an `llmt…`
//! username by the protocol's rule, a display name that says what plays its
//! chair ([`Sits`]), no address, no
//! closed-beta key, English, and the tag the database hands out, so the
//! official export sees and anonymises it as it did. Its password hash is
//! random bytes in the hash's shape: there is no password, and no route
//! that would take one.
use std::io::Read as _;

use anyhow::{Context as _, anyhow, ensure};
use baylee_db::entity::{account, session_token};
use baylee_db::sea_orm::{
    ActiveValue::{NotSet, Set},
    ConnectionTrait as _, DatabaseConnection, DbBackend, EntityTrait as _, Statement,
    TransactionTrait as _,
};
use sha2::{Digest as _, Sha256};

use crate::admin_cleanup::AdminCleanup;

/// How long a teacher's session lasts unused: an account session's
/// (`baylee-gateway` `auth::TOKEN_TTL`), which the gateway slides while the
/// slot uses it.
const SESSION_TTL: time::Duration = time::Duration::hours(12);

/// A database proved to be the collector's own gateway's. Made only by
/// [`OwnDatabase::prove`], so nothing writes a teacher anywhere else.
pub struct OwnDatabase {
    db: DatabaseConnection,
}

/// What a slot's account seats, and so what it is called: a chair an
/// account holds is called by the account, and a seat bridge plays only a
/// chair called what plays it (`baylee_seat::Disclosure`). A language model
/// sits as `LLM-teacher-<slot>`; the house of a pairing's `house:<level>`
/// as `HOUSE-bench-<slot>`, since a chair ticket seats only a language
/// model, so the house always sits in an account's own chair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sits {
    Model,
    House,
}

impl Sits {
    /// The account's display name in slot `slot_id`.
    pub fn display_name(self, slot_id: usize) -> String {
        match self {
            Self::Model => format!("{}teacher-{slot_id}", baylee_seat::Disclosure::Llm.prefix()),
            Self::House => format!("{}bench-{slot_id}", baylee_seat::Disclosure::House.prefix()),
        }
    }
}

/// A teacher account written into an [`OwnDatabase`], and its session's
/// bearer.
pub struct Teacher {
    pub account: uuid::Uuid,
    pub token: String,
}

impl OwnDatabase {
    /// `db`, once the gateway behind `admin` showed its own write there.
    ///
    /// # Errors
    /// The gateway refusing the probe key or its revocation, and a database
    /// that does not show the key made and then revoked: nothing is written
    /// to that database.
    pub async fn prove(admin: &AdminCleanup, db: DatabaseConnection) -> anyhow::Result<Self> {
        let probe = admin.make_probe_key().await?;
        let made = key_revoked(&db, probe).await;
        // Revoked whatever was seen: the key admits nobody after the proof.
        let revoked = admin.revoke_probe_key(probe).await;
        let after = key_revoked(&db, probe).await;
        revoked?;
        ensure!(
            matches!((made, after), (Ok(Some(false)), Ok(Some(true)))),
            "--database-url is not the database of the gateway behind --admin-url; \
             nothing was written to it"
        );
        Ok(Self { db })
    }

    /// Writes slot `slot_id`'s account that seats `sits` and a session for it.
    ///
    /// # Errors
    /// Randomness or the database failing; nothing is left half-written.
    pub async fn teacher(&self, slot_id: usize, sits: Sits) -> anyhow::Result<Teacher> {
        let name = baylee_protocol::names::username(&account_username(uuid::Uuid::now_v7()))
            .map_err(|fault| anyhow!("Teacher username refused by the rule: {fault:?}"))?;
        let mut secret = [0_u8; 32];
        urandom(&mut secret)?;
        let token = baylee_train_ops::files::hex(&secret);
        let now = time::OffsetDateTime::now_utc()
            .replace_nanosecond(0)
            .context("Whole-second clock")?;
        let txn = self.db.begin().await.context("Teacher account write")?;
        let made = account::Entity::insert(account::ActiveModel {
            id: NotSet,
            email: Set(None),
            username: Set(Some(name.shown)),
            username_key: Set(Some(name.key)),
            display_name: Set(sits.display_name(slot_id)),
            tag: NotSet,
            password_hash: Set(Some(inert_hash()?)),
            created_at: Set(now),
            confirmed_at: Set(None),
            lang: Set("en".into()),
            guest: Set(false),
            invite_id: Set(None),
            terms_version: Set(None),
            terms_accepted_at: Set(None),
            must_change_password: Set(false),
            banned_at: Set(None),
            banned_until: Set(None),
            ban_reason: Set(None),
            banned_by: Set(None),
        })
        .exec_with_returning(&txn)
        .await
        .context("Teacher account write")?;
        session_token::Entity::insert(session_token::ActiveModel {
            token_hash: Set(Sha256::digest(token.as_bytes()).to_vec()),
            account_id: Set(made.id),
            expires_at: Set(now + SESSION_TTL),
        })
        .exec(&txn)
        .await
        .context("Teacher session write")?;
        txn.commit().await.context("Teacher account write")?;
        Ok(Teacher {
            account: made.id,
            token,
        })
    }

    /// Deletes a teacher written here that no gateway has used, the way the
    /// gateway deletes an account (`baylee_db::accounts::delete`). A teacher
    /// that played goes through the gateway's `DELETE /account` instead.
    ///
    /// # Errors
    /// The database failing.
    pub async fn forget(&self, account: uuid::Uuid) -> anyhow::Result<()> {
        baylee_db::accounts::delete(&self.db, account)
            .await
            .context("Teacher account deletion")?;
        Ok(())
    }
}

/// Whether the closed-beta key `id` is revoked in `db`; `None` where `db`
/// has no such key.
async fn key_revoked(db: &DatabaseConnection, id: uuid::Uuid) -> anyhow::Result<Option<bool>> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT revoked_at IS NOT NULL AS revoked FROM invite WHERE id = $1",
            [id.into()],
        ))
        .await?;
    Ok(row
        .map(|row| row.try_get::<bool>("", "revoked"))
        .transpose()?)
}

pub(crate) fn account_username(id: uuid::Uuid) -> String {
    let suffix = id.simple().to_string();
    format!("llmt{}", &suffix[12..])
}

fn urandom(bytes: &mut [u8]) -> anyhow::Result<()> {
    std::fs::File::open("/dev/urandom")
        .and_then(|mut random| random.read_exact(bytes))
        .context("Reading /dev/urandom")
}

/// An Argon2id PHC string over a random salt and output, the parameters the
/// gateway hashed with: the shape the column holds, for which no password
/// exists.
fn inert_hash() -> anyhow::Result<String> {
    let mut bytes = [0_u8; 48];
    urandom(&mut bytes)?;
    Ok(format!(
        "$argon2id$v=19$m=19456,t=2,p=1${}${}",
        b64(&bytes[..16]),
        b64(&bytes[16..])
    ))
}

/// Standard base64 without padding, as PHC strings write it.
fn b64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0_usize, |n, (i, b)| n | usize::from(*b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(char::from(ALPHABET[(n >> (18 - 6 * i)) & 63]));
        }
    }
    out
}

#[cfg(test)]
mod tests;
