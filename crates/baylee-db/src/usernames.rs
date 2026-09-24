//! Usernames for the accounts that were made without one (#269).
//!
//! An account used to sign in with its address. Every account made before
//! usernames existed is given one here, from the part of its address before
//! the `@`, so that its owner can sign in with that from now on; the gateway
//! tells them which name they got the first time they sign in with the
//! address.
//!
//! The name is built with the rule itself ([`baylee_protocol::names`]), not
//! with a copy of it in SQL: a copy is a second rule, and the day the two
//! disagreed a backfilled name would be one its owner could not type. And it
//! is built in a loop that always ends on a free name. A migration that
//! stopped on a taken one would stop the gateway from starting.

use baylee_protocol::names::{self, USERNAME_MAX, USERNAME_MIN, Username};
use sea_orm::{ConnectionTrait, DbErr, Statement};
use std::collections::BTreeSet;

/// The name an account with this address and tag is given, when `taken`
/// says which keys are already someone's.
///
/// The address's local part, made to keep the rule ([`names::nearest`]),
/// with case kept as the owner typed it. The account's tag, in the hex a
/// handle shows it in, is added after a `-` when that part is shorter than
/// [`USERNAME_MIN`] or taken: the tag is the account's own, so the name
/// stays recognisable as theirs. Taken even so, a count follows the tag.
/// The part before the tag is cut so that the tag always fits.
#[must_use]
pub fn backfilled(address: &str, tag: i32, taken: impl Fn(&str) -> bool) -> Username {
    let local = address.split('@').next().unwrap_or_default();
    let base = names::nearest(local);
    let hex = tag_hex(tag);
    let mut count = 0_u32;
    loop {
        let candidate = match count {
            0 if base.len() >= USERNAME_MIN => base.clone(),
            0 => {
                count += 1;
                continue;
            }
            1 => joined(&base, &hex),
            n => joined(&base, &format!("{hex}-{n}")),
        };
        count += 1;
        // Always a name from the first suffix on: letters and digits either
        // side of one `-`. Checked all the same, because the rule is the
        // authority and this function only means to keep it.
        if let Ok(name) = names::username(&candidate)
            && !taken(&name.key)
        {
            return name;
        }
    }
}

/// `base` and `suffix` with a `-` between them, `base` cut so the whole fits
/// in [`USERNAME_MAX`]; only `suffix` when nothing of `base` is left.
fn joined(base: &str, suffix: &str) -> String {
    let room = USERNAME_MAX.saturating_sub(suffix.len() + 1);
    // `nearest` again, for the separator a cut may leave at the end; `base`
    // is ASCII, so the cut falls between characters.
    let cut = names::nearest(&base[..base.len().min(room)]);
    if cut.is_empty() {
        suffix.to_string()
    } else {
        format!("{cut}-{suffix}")
    }
}

/// A tag as a handle shows it: lowercase hex, at least four digits
/// (`baylee-gateway`'s `handle::tag_text`, which this crate cannot link).
fn tag_hex(tag: i32) -> String {
    format!("{:04x}", tag.unsigned_abs())
}

/// Gives every account that has an address and no username one, oldest tag
/// first, and says how many it named.
///
/// Oldest first, so that of two addresses with the same local part the
/// account that came first keeps the plain name. Takes any connection, so a
/// migration or an import runs it inside its own transaction.
///
/// # Errors
///
/// When a read or a write fails.
pub async fn name_the_unnamed(db: &impl ConnectionTrait) -> Result<u64, DbErr> {
    let backend = db.get_database_backend();
    let mut taken: BTreeSet<String> = db
        .query_all_raw(Statement::from_string(
            backend,
            "SELECT username_key FROM account WHERE username_key IS NOT NULL",
        ))
        .await?
        .iter()
        .map(|row| row.try_get::<String>("", "username_key"))
        .collect::<Result<_, _>>()?;
    let unnamed = db
        .query_all_raw(Statement::from_string(
            backend,
            "SELECT id, email, tag FROM account \
             WHERE username IS NULL AND email IS NOT NULL ORDER BY tag",
        ))
        .await?;
    let mut named = 0;
    for row in unnamed {
        let id: uuid::Uuid = row.try_get("", "id")?;
        let email: String = row.try_get("", "email")?;
        let tag: i32 = row.try_get("", "tag")?;
        let name = backfilled(&email, tag, |key| taken.contains(key));
        db.execute_raw(Statement::from_sql_and_values(
            backend,
            "UPDATE account SET username = $1, username_key = $2 WHERE id = $3",
            [
                name.shown.clone().into(),
                name.key.clone().into(),
                id.into(),
            ],
        ))
        .await?;
        taken.insert(name.key);
        named += 1;
    }
    Ok(named)
}

#[cfg(test)]
mod tests {
    use super::backfilled;
    use std::collections::BTreeSet;

    fn shown(address: &str, tag: i32, taken: &[&str]) -> String {
        let taken: BTreeSet<&str> = taken.iter().copied().collect();
        backfilled(address, tag, |key| taken.contains(key)).shown
    }

    #[test]
    fn the_local_part_is_the_name_when_it_keeps_the_rule_and_is_free() {
        assert_eq!(shown("alice@example.com", 3, &[]), "alice");
        assert_eq!(shown("Alice.Smith@example.com", 3, &[]), "Alice.Smith");
        assert_eq!(shown("alice+baylee@example.com", 3, &[]), "alice-baylee");
    }

    #[test]
    fn a_short_part_is_given_the_tag_and_an_empty_one_is_the_tag() {
        assert_eq!(shown("al@example.com", 0xaf03, &[]), "al-af03");
        assert_eq!(shown("x@example.com", 7, &[]), "x-0007");
        assert_eq!(shown("+++@example.com", 7, &[]), "0007");
        assert_eq!(shown("日本@example.com", 0x1_0000, &[]), "10000");
    }

    #[test]
    fn a_taken_name_is_given_the_tag_and_a_taken_tag_a_count() {
        assert_eq!(shown("Alice@example.com", 7, &["alice"]), "Alice-0007");
        assert_eq!(
            shown("alice@example.com", 7, &["alice", "alice-0007"]),
            "alice-0007-2"
        );
        assert_eq!(
            shown(
                "alice@example.com",
                7,
                &["alice", "alice-0007", "alice-0007-2"]
            ),
            "alice-0007-3"
        );
    }

    /// The tag always fits: the part before it is cut, and a separator the
    /// cut leaves at its end goes.
    #[test]
    fn a_long_part_is_cut_so_the_tag_fits() {
        let long = format!("{}.{}@example.com", "a".repeat(18), "b".repeat(10));
        let name = shown(&long, 7, &[&format!("{}.{}", "a".repeat(18), "bbbbb")]);
        assert_eq!(name, format!("{}-0007", "a".repeat(18)));
        assert!(name.len() <= 24);
    }

    /// Whatever the address, the name is one its owner can type back.
    #[test]
    fn every_backfilled_name_keeps_the_rule() {
        let taken = ["alice", "alice-0001", "0001", "a-0001"];
        for address in [
            "alice@x",
            "a@x",
            "@x",
            "...@x",
            "a..b@x",
            "Ａｌｉｃｅ@x",
            "\u{202E}evil@x",
            "no-at-sign",
            &"z".repeat(64),
        ] {
            let taken: BTreeSet<&str> = taken.iter().copied().collect();
            let name = backfilled(address, 1, |key| taken.contains(key));
            assert!(
                baylee_protocol::names::username(&name.shown).is_ok(),
                "{address:?} came out as {:?}",
                name.shown
            );
            assert!(!taken.contains(name.key.as_str()), "{address:?}");
        }
    }
}
