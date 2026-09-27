//! The web UI's admins (#311): a name and an Argon2id hash each, made,
//! listed and removed only by the binary's `admin` subcommand
//! (`baylee-feedback admin add <name>`). No route creates one.

use std::sync::LazyLock;

use anyhow::{Result, bail};
use argon2::password_hash::phc::PasswordHash;
use argon2::{Argon2, PasswordHasher as _, PasswordVerifier as _};
use sea_orm::{ConnectionTrait as _, DatabaseConnection, Statement};
use uuid::Uuid;

/// The shortest password `admin add` takes, in characters.
pub const MIN_PASSWORD_CHARS: usize = 12;

/// The longest password anyone may try, in bytes: Argon2 over a megabyte
/// is work an unauthenticated caller should not be able to order.
pub const MAX_PASSWORD_BYTES: usize = 1024;

/// The longest admin name, in characters.
pub const MAX_NAME_CHARS: usize = 64;

/// Whether `name` may name an admin: 1–64 ASCII letters, digits, `.`, `-`,
/// `_`. Compared as given, so `Viktor` and `viktor` are two names.
#[must_use]
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// `password` as a PHC string: Argon2id, the library's default parameters,
/// a random salt.
///
/// # Panics
///
/// Only when the OS random source is unavailable.
#[must_use]
pub fn hash_password(password: &str) -> String {
    let hash: PasswordHash = Argon2::default()
        .hash_password(password.as_bytes())
        .expect("argon2 hashing works");
    hash.to_string()
}

/// A hash made the same way as a real one, so an unknown name costs the
/// same Argon2 run as a known one and the answer's timing says nothing
/// about which names exist.
static DUMMY: LazyLock<String> = LazyLock::new(|| hash_password("no such admin, no such password"));

/// Whether `password` matches `stored`; `None` (no such admin) is checked
/// against [`DUMMY`] and is always `false`.
#[must_use]
pub fn verify_password(stored: Option<&str>, password: &str) -> bool {
    if password.len() > MAX_PASSWORD_BYTES {
        return false;
    }
    let known = stored.is_some();
    let phc = stored.unwrap_or(&DUMMY);
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    let fits = Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok();
    fits && known
}

/// A line read as a password: the line ending goes, everything else,
/// spaces included, is the password.
#[must_use]
pub fn password_line(line: &str) -> &str {
    line.strip_suffix('\n')
        .map_or(line, |l| l.strip_suffix('\r').unwrap_or(l))
}

/// Makes admin `name`.
///
/// # Errors
///
/// When the name is not [`valid_name`], the password is shorter than
/// [`MIN_PASSWORD_CHARS`] or longer than [`MAX_PASSWORD_BYTES`], an admin
/// of that name exists, or the database fails.
pub async fn add(db: &DatabaseConnection, name: &str, password: &str) -> Result<()> {
    if !valid_name(name) {
        bail!("`{name}` is not an admin name: 1-64 letters, digits, `.`, `-`, `_`");
    }
    if password.chars().count() < MIN_PASSWORD_CHARS {
        bail!("a password needs at least {MIN_PASSWORD_CHARS} characters");
    }
    if password.len() > MAX_PASSWORD_BYTES {
        bail!("a password is at most {MAX_PASSWORD_BYTES} bytes");
    }
    let owned = password.to_owned();
    let hash = tokio::task::spawn_blocking(move || hash_password(&owned)).await?;
    let done = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO feedback_admin (id, name, password_hash) VALUES ($1, $2, $3) \
             ON CONFLICT (name) DO NOTHING",
            [Uuid::now_v7().into(), name.into(), hash.into()],
        ))
        .await?;
    if done.rows_affected() == 0 {
        bail!("an admin named `{name}` already exists");
    }
    Ok(())
}

/// Removes admin `name` and, with it, every session they hold. Whether
/// there was one.
///
/// # Errors
///
/// When the database fails.
pub async fn remove(db: &DatabaseConnection, name: &str) -> Result<bool> {
    let done = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM feedback_admin WHERE name = $1",
            [name.into()],
        ))
        .await?;
    Ok(done.rows_affected() > 0)
}

/// One admin as `admin list` shows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// The name they sign in with.
    pub name: String,
    /// When they were made, `YYYY-MM-DD HH:MM` UTC.
    pub created: String,
    /// How many sessions they hold, expired ones not yet swept included.
    pub sessions: i64,
}

/// Every admin, by name.
///
/// # Errors
///
/// When the database fails.
pub async fn list(db: &DatabaseConnection) -> Result<Vec<Listed>> {
    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT a.name, \
                 to_char(a.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI') AS created, \
                 (SELECT count(*) FROM feedback_session s WHERE s.admin_id = a.id) AS sessions \
             FROM feedback_admin a ORDER BY a.name",
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(Listed {
                name: row.try_get("", "name")?,
                created: row.try_get("", "created")?,
                sessions: row.try_get("", "sessions")?,
            })
        })
        .collect()
}

/// The stored hash and id of admin `name`, if there is one.
pub(crate) async fn credentials(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<(Uuid, String)>, sea_orm::DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT id, password_hash FROM feedback_admin WHERE name = $1",
            [name.into()],
        ))
        .await?;
    row.map(|row| Ok((row.try_get("", "id")?, row.try_get("", "password_hash")?)))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_letters_digits_and_three_marks() {
        for fine in ["viktor", "a", "ops-1", "a.b_c", &"x".repeat(64)] {
            assert!(valid_name(fine), "{fine}");
        }
        for bad in ["", "a b", "ä", "a/b", "a\nb", &"x".repeat(65)] {
            assert!(!valid_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn a_password_verifies_against_its_own_hash_only() {
        let hash = hash_password("correct horse battery");
        assert!(hash.starts_with("$argon2id$"), "{hash}");
        assert!(verify_password(Some(&hash), "correct horse battery"));
        assert!(!verify_password(Some(&hash), "correct horse batter"));
        assert!(!verify_password(
            Some("not a hash"),
            "correct horse battery"
        ));
    }

    #[test]
    fn no_password_opens_an_unknown_name_not_even_the_dummys() {
        assert!(!verify_password(None, "no such admin, no such password"));
        assert!(!verify_password(None, ""));
    }

    #[test]
    fn the_dummy_is_made_with_the_same_parameters_as_a_real_hash() {
        let params = |phc: &str| phc.split('$').nth(3).map(str::to_owned);
        assert_eq!(params(&DUMMY), params(&hash_password("x")));
    }
}
