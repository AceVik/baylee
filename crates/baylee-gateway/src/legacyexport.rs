//! `baylee-gateway legacy export`: the accounts that still sign in here with
//! a password, written once for Baylee Central's legacy store at the cutover
//! (`docs/central-login.md` §"Cutover"). Central signs them in from then on
//! and moves them; this gateway keeps their game data and learns the link
//! at their first admission.
//!
//! The document goes to standard output only, never to a terminal: it holds
//! every remaining password hash. Piped into Central's
//! `central-service --legacy-import-stdin`, it is checked there against the
//! count and digest printed here on standard error. It opens the database as
//! it stands: the export reads the schema the stopped gateway left.
//!
//! Central takes a password hash as its salt and output only, under the one
//! set of parameters this gateway hashes with: it parses no PHC string, and
//! refuses nothing it would have to guess at.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{IsTerminal, Write};

const USAGE: &str = "usage: baylee-gateway legacy export | central-service --legacy-import-stdin";

/// The format Central's import reads.
pub(crate) const FORMAT: &str = "baylee-legacy-accounts/1";

/// The only hash Central takes: the parameters this gateway hashes with
/// (`auth::hash_password`, argon2's defaults).
const PINNED: &str = "$argon2id$v=19$m=19456,t=2,p=1$";

/// The export document.
#[derive(Serialize)]
pub(crate) struct Export {
    pub(crate) format: &'static str,
    pub(crate) accounts: Vec<Row>,
    pub(crate) count: u64,
    pub(crate) digest: [u8; 32],
}

/// One account, as Central's store keeps it.
#[derive(Clone, Serialize)]
pub(crate) struct Row {
    pub(crate) legacy_id: String,
    pub(crate) username: String,
    pub(crate) display_name: Option<String>,
    pub(crate) tag: Option<u32>,
    pub(crate) password: Password,
    pub(crate) terms_version: Option<String>,
    pub(crate) must_change_password: bool,
    pub(crate) ban: Option<Ban>,
    pub(crate) snapshot: Vec<u8>,
}

/// An Argon2id hash under the pinned parameters ([`PINNED`]).
#[derive(Clone, Serialize)]
pub(crate) struct Password {
    pub(crate) salt: [u8; 16],
    pub(crate) hash: [u8; 32],
}

/// A ban in force at the export.
#[derive(Clone, Serialize)]
pub(crate) struct Ban {
    /// Until when, Unix seconds; `None` for good.
    pub(crate) until: Option<i64>,
}

/// `baylee-gateway legacy …`: the exit status.
pub(crate) async fn cli(args: &[String]) -> i32 {
    match args {
        [command] if command == "export" => {}
        [help] if help == "help" || help == "--help" => {
            println!("{USAGE}");
            return 0;
        }
        _ => {
            eprintln!("baylee-gateway legacy: {USAGE}");
            return 2;
        }
    }
    if std::io::stdout().is_terminal() {
        eprintln!(
            "baylee-gateway legacy: the export holds password hashes; pipe it, never print it"
        );
        return 2;
    }
    let Some(url) = std::env::var("DATABASE_URL").ok().filter(|u| !u.is_empty()) else {
        eprintln!("baylee-gateway legacy: DATABASE_URL is not set");
        return 1;
    };
    let db = match baylee_db::open(&url, 1).await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("baylee-gateway legacy: {e:#}");
            return 1;
        }
    };
    let accounts = match baylee_db::central::legacy_accounts(&db).await {
        Ok(accounts) => accounts,
        Err(e) => {
            eprintln!("baylee-gateway legacy: the accounts: {e}");
            return 1;
        }
    };
    let export = match export(accounts) {
        Ok(export) => export,
        Err(why) => {
            eprintln!("baylee-gateway legacy: {why}");
            return 1;
        }
    };
    let mut out = std::io::stdout().lock();
    if serde_json::to_writer(&mut out, &export).is_err() || out.flush().is_err() {
        eprintln!("baylee-gateway legacy: standard output refused the export");
        return 1;
    }
    let must_change = export
        .accounts
        .iter()
        .filter(|row| row.must_change_password)
        .count();
    let banned = export
        .accounts
        .iter()
        .filter(|row| row.ban.is_some())
        .count();
    eprintln!(
        "legacy export: {} accounts ({must_change} must choose a new password, {banned} banned), digest {}",
        export.count,
        crate::auth::hex_lower(&export.digest)
    );
    0
}

/// The document for `accounts`, or every reason one of them cannot move.
/// The whole export is refused for one such account: the operator decides
/// about it, the export never leaves it behind unsaid.
pub(crate) fn export(accounts: Vec<baylee_db::central::LegacyAccount>) -> Result<Export, String> {
    let mut refused = Vec::new();
    let mut rows = Vec::with_capacity(accounts.len());
    for account in accounts {
        let Some(username) = account.username_key else {
            refused.push(format!("{}: no username", account.id));
            continue;
        };
        let Some(snapshot) = account.snapshot else {
            refused.push(format!(
                "{}: snapshot larger than 192 KiB or 2,048 rows",
                account.id
            ));
            continue;
        };
        let Some(password) = pinned(&account.password_hash) else {
            refused.push(format!(
                "{}: password hash with other parameters",
                account.id
            ));
            continue;
        };
        let Some(tag) = u32::try_from(account.tag).ok().filter(|tag| *tag > 0) else {
            refused.push(format!("{}: no tag", account.id));
            continue;
        };
        // Central keeps a name only when it keeps the rule (`routes::central`
        // sent the same at a move).
        let display_name =
            crate::auth::valid_display_name(&account.display_name).then_some(account.display_name);
        rows.push(Row {
            legacy_id: account.id,
            username,
            display_name,
            tag: Some(tag),
            password,
            terms_version: account.terms_version,
            must_change_password: account.must_change_password,
            ban: account.ban.map(|ban| Ban { until: ban.until }),
            snapshot: snapshot.into_bytes(),
        });
    }
    if !refused.is_empty() {
        return Err(format!(
            "{} accounts cannot move, so nothing was exported: {}",
            refused.len(),
            refused.join("; ")
        ));
    }
    rows.sort_by(|a, b| a.legacy_id.as_bytes().cmp(b.legacy_id.as_bytes()));
    let count = u64::try_from(rows.len()).map_err(|_| "too many accounts".to_string())?;
    let digest = digest(&rows);
    Ok(Export {
        format: FORMAT,
        accounts: rows,
        count,
        digest,
    })
}

/// The salt and output of `phc` when it is written under exactly the pinned
/// parameters, with a 16-byte salt and a 32-byte output.
fn pinned(phc: &str) -> Option<Password> {
    if !phc.starts_with(PINNED) {
        return None;
    }
    let parsed = argon2::password_hash::phc::PasswordHash::new(phc).ok()?;
    Some(Password {
        salt: parsed.salt?.as_ref().try_into().ok()?,
        hash: parsed.hash?.as_bytes().try_into().ok()?,
    })
}

/// SHA-256 over the count and each row, in the order given: the digest
/// Central's import computes again from what it stored
/// (`docs/central-login.md` §"Cutover").
pub(crate) fn digest(rows: &[Row]) -> [u8; 32] {
    fn field(hash: &mut Sha256, bytes: &[u8]) {
        let length = u32::try_from(bytes.len()).expect("a field is under 4 GiB");
        hash.update(length.to_be_bytes());
        hash.update(bytes);
    }
    fn optional(hash: &mut Sha256, bytes: Option<&[u8]>) {
        match bytes {
            None => hash.update([0]),
            Some(bytes) => {
                hash.update([1]);
                field(hash, bytes);
            }
        }
    }
    let mut hash = Sha256::new();
    hash.update((rows.len() as u64).to_be_bytes());
    for row in rows {
        field(&mut hash, row.legacy_id.as_bytes());
        field(&mut hash, row.username.as_bytes());
        optional(&mut hash, row.display_name.as_deref().map(str::as_bytes));
        optional(
            &mut hash,
            row.tag
                .map(|tag| tag.to_string())
                .as_deref()
                .map(str::as_bytes),
        );
        field(&mut hash, &row.password.salt);
        field(&mut hash, &row.password.hash);
        optional(&mut hash, row.terms_version.as_deref().map(str::as_bytes));
        hash.update([u8::from(row.must_change_password)]);
        let ban = row.ban.as_ref().map(|ban| {
            ban.until
                .map_or_else(|| "forever".to_owned(), |until| until.to_string())
        });
        optional(&mut hash, ban.as_deref().map(str::as_bytes));
        field(&mut hash, &row.snapshot);
    }
    hash.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_db::central::LegacyAccount;

    fn account(id: &str) -> LegacyAccount {
        LegacyAccount {
            id: id.into(),
            username_key: Some(format!("user-{id}")),
            display_name: "Alice".into(),
            tag: 7,
            password_hash: crate::auth::hash_password("a-very-fine-password"),
            terms_version: Some("2026-10-01".into()),
            must_change_password: false,
            ban: None,
            snapshot: Some(r#"{"decks": [], "preferences": {}}"#.into()),
        }
    }

    #[test]
    fn the_gateways_own_hashes_are_the_pinned_ones() {
        // The pin is what Central refuses everything else by; a change of
        // the gateway's defaults must fail here, not at the cutover.
        let phc = crate::auth::hash_password("anything");
        let password = pinned(&phc).expect("the gateway's own hash");
        let parsed = argon2::password_hash::phc::PasswordHash::new(&phc).expect("a PHC string");
        assert_eq!(
            password.salt.as_slice(),
            parsed.salt.expect("salted").as_ref()
        );
        assert_eq!(
            password.hash.as_slice(),
            parsed.hash.expect("hashed").as_bytes()
        );
        assert!(pinned("$argon2id$v=19$m=65536,t=3,p=1$c2FsdHNhbHRzYWx0c2E$aGFzaA").is_none());
        assert!(pinned("$argon2i$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0c2E$aGFzaA").is_none());
        // The pinned parameters with a short salt or output.
        assert!(pinned("$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA").is_none());
    }

    #[test]
    fn rows_are_sorted_and_the_digest_covers_every_field() {
        let export = export(vec![account("b"), account("a")]).expect("both move");
        assert_eq!(export.count, 2);
        assert_eq!(export.accounts[0].legacy_id, "a");
        let before = digest(&export.accounts);
        assert_eq!(export.digest, before);
        let changes: [fn(&mut Row); 10] = [
            |row| row.legacy_id.push('x'),
            |row| row.username.push('x'),
            |row| row.display_name = None,
            |row| row.tag = Some(8),
            |row| row.password.salt[0] ^= 1,
            |row| row.password.hash[31] ^= 1,
            |row| row.terms_version = None,
            |row| row.must_change_password = true,
            |row| row.ban = Some(Ban { until: None }),
            |row| row.snapshot.push(b' '),
        ];
        for change in changes {
            let mut rows = export.accounts.clone();
            change(&mut rows[1]);
            assert_ne!(digest(&rows), before);
        }
        let mut swapped = export.accounts.clone();
        swapped.swap(0, 1);
        assert_ne!(digest(&swapped), before, "the order is part of it");
    }

    #[test]
    fn one_account_that_cannot_move_refuses_the_whole_export() {
        let mut nameless = account("a");
        nameless.username_key = None;
        let mut large = account("b");
        large.snapshot = None;
        let mut foreign = account("c");
        foreign.password_hash = "$argon2id$v=19$m=65536,t=3,p=1$c2FsdA$aGFzaA".into();
        let why = export(vec![nameless, large, foreign, account("d")])
            .err()
            .expect("refused");
        assert!(why.starts_with("3 accounts cannot move"), "{why}");
        for id in ["a: no username", "b: snapshot", "c: password hash"] {
            assert!(why.contains(id), "{why}");
        }
    }

    #[test]
    fn a_name_the_rule_refuses_stays_behind_and_the_tag_comes() {
        let mut old = account("a");
        old.display_name = "x".into();
        let export = export(vec![old]).expect("it moves");
        assert_eq!(export.accounts[0].display_name, None);
        assert_eq!(export.accounts[0].tag, Some(7));
    }
}
