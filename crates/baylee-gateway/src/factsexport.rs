//! `baylee-gateway pool-facts --out <file>`: this gateway's legality facts
//! as a file, for an AI runner on another machine (`docs/llm-seat.md`
//! §"Deck building").
//!
//! A ranked host admits a deck only when its catalog has a verdict for every
//! card, so an AI checks its decks against the host's own table. Where the
//! host lets no guest in to read `GET /pool/facts` and `GET /cards/blocked`,
//! its operator writes the same two answers here, on the host, and copies
//! the file to the runner: what `baylee_seat::deckbuild::facts::load` reads,
//! `{"gateway", "fetched", "pool", "blocked"}`. The table is the route's own
//! ([`crate::poolfacts::PoolFactsCache::get`], the same bytes), and the
//! blocks are the route's own reading ([`crate::routes::blocked_now`]) of
//! the blocks in force in the database.
//!
//! It reads the gateway's database with the gateway's own settings
//! (`DATABASE_URL`, `BAYLEE_PUBLIC_URL` for the origin the file names), as
//! the database stands: no migration, no listener, no session, and no
//! route changes. The file is written through a temporary file beside it
//! and moved into place, mode 0644: it names no one and holds no secret.

use baylee_catalog::{Catalog, PoolFactsBody};
use std::path::{Path, PathBuf};

/// What `baylee-gateway pool-facts help` prints.
pub(crate) const USAGE: &str = "usage: baylee-gateway pool-facts --out <file>

Writes this gateway's legality facts (its catalog's table, as GET /pool/facts
serves it, and the cards blocked now, as GET /cards/blocked does) to <file>,
for an AI runner's `--facts` file. Reads DATABASE_URL and BAYLEE_PUBLIC_URL;
opens no port.";

/// `baylee-gateway pool-facts …`: the exit status.
pub(crate) async fn cli(args: &[String]) -> i32 {
    let out = match args {
        [flag, file] if flag == "--out" => PathBuf::from(file),
        [help] if help == "help" || help == "--help" => {
            println!("{USAGE}");
            return 0;
        }
        _ => {
            eprintln!("baylee-gateway pool-facts: {USAGE}");
            return 2;
        }
    };
    let origin = match origin(std::env::var("BAYLEE_PUBLIC_URL").ok().as_deref()) {
        Ok(origin) => origin,
        Err(why) => {
            eprintln!("baylee-gateway pool-facts: {why}");
            return 1;
        }
    };
    let Some(url) = std::env::var("DATABASE_URL").ok().filter(|u| !u.is_empty()) else {
        eprintln!("baylee-gateway pool-facts: DATABASE_URL is not set");
        return 1;
    };
    // As it stands: an operator's read, never a migration of the database
    // the running gateway serves from.
    let db = match baylee_db::open(&url, 1).await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("baylee-gateway pool-facts: {e:#}");
            return 1;
        }
    };
    match write(&db, &origin, &out).await {
        Ok(said) => {
            println!("{said}");
            0
        }
        Err(why) => {
            eprintln!("baylee-gateway pool-facts: {why}");
            1
        }
    }
}

/// The origin `BAYLEE_PUBLIC_URL` names (`https://host`, the scheme, host
/// and port of it alone).
///
/// # Errors
/// When it is unset, or no `http`/`https` address.
fn origin(public_url: Option<&str>) -> Result<String, String> {
    let named = public_url.map(str::trim).filter(|u| !u.is_empty()).ok_or(
        "BAYLEE_PUBLIC_URL is not set: the file names the gateway it describes by its public \
         address",
    )?;
    let refused = || "BAYLEE_PUBLIC_URL is no http:// or https:// address".to_string();
    let (scheme, rest) = named.split_once("://").ok_or_else(refused)?;
    let scheme = scheme.to_ascii_lowercase();
    if !matches!(scheme.as_str(), "https" | "http") {
        return Err(refused());
    }
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let plain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']');
    if host.is_empty() || !host.chars().all(plain) || host.starts_with(':') {
        return Err(refused());
    }
    Ok(format!("{scheme}://{}", host.to_ascii_lowercase()))
}

/// Reads the table and the blocks and writes the file: what it wrote, in a
/// sentence.
async fn write(
    db: &sea_orm::DatabaseConnection,
    origin: &str,
    out: &Path,
) -> Result<String, String> {
    let catalog = Catalog::from_connection(db.clone());
    let held = crate::poolfacts::PoolFactsCache::default()
        .get(&catalog)
        .await
        .map_err(|e| format!("the card catalog: {e:#}"))?;
    let pool: PoolFactsBody =
        serde_json::from_slice(held.body()).map_err(|e| format!("the pool's table: {e}"))?;
    if pool.cards.is_empty() {
        return Err(
            "the card catalog holds no table (no ingest yet), and no deck could pass against an \
             empty one"
                .into(),
        );
    }
    let blocks = crate::blocks::Blocks::default();
    blocks
        .reload(db)
        .await
        .map_err(|e| format!("the card blocks: {e}"))?;
    let now = time::OffsetDateTime::now_utc();
    let blocked = crate::routes::blocked_now(&blocks, now).blocked;
    let facts = serde_json::json!({
        "gateway": origin,
        "fetched": crate::blocks::iso(now),
        "pool": pool,
        "blocked": blocked,
    });
    let bytes = serde_json::to_vec(&facts).map_err(|e| format!("the file: {e}"))?;
    write_whole(out, &bytes).map_err(|e| format!("writing {}: {e}", out.display()))?;
    Ok(format!(
        "the legality facts of {origin} (version {}, {} cards, {} blocked) are in {}",
        pool.version,
        pool.cards.len(),
        blocked.len(),
        out.display()
    ))
}

/// `bytes` to `path`, whole or not at all, mode 0644.
fn write_whole(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("pool-facts");
    let temporary = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o644))?;
    }
    std::fs::rename(&temporary, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temporary);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_names_the_gateway_by_its_public_origin() {
        assert_eq!(
            origin(Some("https://baylee.acevik.de/")),
            Ok("https://baylee.acevik.de".into())
        );
        assert_eq!(
            origin(Some(" http://127.0.0.1:28766/path?q ")),
            Ok("http://127.0.0.1:28766".into())
        );
        assert!(
            origin(None)
                .unwrap_err()
                .contains("BAYLEE_PUBLIC_URL is not set")
        );
        assert!(origin(Some("")).is_err());
        assert!(origin(Some("ftp://host")).is_err());
        assert!(origin(Some("not an address")).is_err());
        assert!(origin(Some("https://user@host")).is_err(), "no user in it");
        assert!(origin(Some("https://")).is_err());
        assert_eq!(
            origin(Some("HTTPS://Baylee.Test")),
            Ok("https://baylee.test".into())
        );
    }

    #[test]
    fn the_file_is_written_whole_and_readable_by_all() {
        let dir = std::env::temp_dir().join(format!("baylee-pool-facts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("facts.json");
        write_whole(&path, b"{}").unwrap();
        write_whole(&path, b"{\"second\":1}").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"second\":1}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o644);
        }
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        assert_eq!(names, ["facts.json"], "no temporary file left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
