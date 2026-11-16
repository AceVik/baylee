//! Build a small cosmetic lookup offline from cached public Scryfall metadata.
//! Fetch is explicit, serial, rate-limited and fixed-origin. No image is copied.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

const LIMIT: u64 = 2 * 1024 * 1024;
const UA: &str = "Baylee-Tower-Palette/1 (https://github.com/AceVik/baylee)";

#[derive(clap::Args)]
pub(super) struct Args {
    /// JSON emitted by baylee-tower-content --catalog.
    #[arg(long)]
    catalog: PathBuf,
    /// Public metadata cache; no account or credentials are used.
    #[arg(long)]
    cache: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Permit requests to api.scryfall.com for absent cached metadata.
    #[arg(long)]
    fetch: bool,
}

#[derive(Deserialize)]
struct Content {
    catalog: Catalog,
}
#[derive(Deserialize)]
struct Catalog {
    cards: Vec<Want>,
}
#[derive(Deserialize)]
struct Want {
    run: Facts,
}
#[derive(Deserialize)]
struct Facts {
    presentation: Presentation,
}
#[derive(Deserialize)]
struct Presentation {
    printing: Uuid,
}

#[derive(Clone, Deserialize, Serialize)]
struct Card {
    id: Uuid,
    oracle_id: Uuid,
    set: String,
    collector_number: String,
    lang: String,
    image_status: String,
}

#[derive(Serialize)]
struct Pair {
    en: Uuid,
    de: Uuid,
    oracle: Uuid,
    set: String,
    number: String,
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= LIMIT, "metadata exceeds two MiB");
    Ok(bytes)
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".tower-print-{}.tmp", Uuid::now_v7()));
    let result = (|| -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

/// Encode one path segment; metadata can never introduce another URL or path.
fn segment(text: &str) -> String {
    use std::fmt::Write as _;
    let mut encoded = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn cached(
    args: &Args,
    agent: &ureq::Agent,
    id: Uuid,
    language: &str,
    route: &str,
) -> Result<Option<Card>> {
    let path = args.cache.join(format!("{id}-{language}.json"));
    if path.is_file() {
        return Ok(serde_json::from_slice(&read(&path)?)?);
    }
    ensure!(
        args.fetch,
        "uncached printing {id}/{language}; use --fetch to contact Scryfall"
    );
    for attempt in 0..3 {
        std::thread::sleep(Duration::from_millis(150));
        let mut response = agent
            .get(format!("https://api.scryfall.com/cards/{route}"))
            .header("User-Agent", UA)
            .header("Accept", "application/json")
            .call()
            .map_err(|_| anyhow::anyhow!("Scryfall transport failed for {id}/{language}"))?;
        let status = response.status().as_u16();
        if status == 429 && attempt < 2 {
            let delay = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|text| text.parse::<u64>().ok())
                .unwrap_or(10);
            ensure!(delay <= 60, "rate-limited: retry later");
            std::thread::sleep(Duration::from_secs(delay));
            continue;
        }
        let card = if status == 404 {
            None
        } else {
            ensure!(
                status == 200,
                "Scryfall status {status} for {id}/{language}"
            );
            let mut body = Vec::new();
            response
                .body_mut()
                .as_reader()
                .take(LIMIT + 1)
                .read_to_end(&mut body)?;
            ensure!(
                body.len() as u64 <= LIMIT,
                "Scryfall metadata exceeds two MiB"
            );
            Some(serde_json::from_slice(&body).context("invalid Scryfall card metadata")?)
        };
        write(&path, &serde_json::to_vec(&card)?)?;
        return Ok(card);
    }
    anyhow::bail!("Scryfall retry limit reached")
}

fn exact(en: &Card, de: &Card) -> bool {
    en.lang == "en"
        && de.lang == "de"
        && en.oracle_id == de.oracle_id
        && en.set == de.set
        && en.collector_number == de.collector_number
        && !en.id.is_nil()
        && !de.id.is_nil()
        && en.id != de.id
        && matches!(de.image_status.as_str(), "highres_scan" | "lowres")
}

pub(super) fn run(args: &Args) -> Result<()> {
    let content: Content = serde_json::from_slice(&read(&args.catalog)?)?;
    ensure!(
        !content.catalog.cards.is_empty() && content.catalog.cards.len() <= 2048,
        "invalid palette size"
    );
    let prints: BTreeSet<_> = content
        .catalog
        .cards
        .into_iter()
        .map(|card| card.run.presentation.printing)
        .collect();
    ensure!(prints.iter().all(|id| !id.is_nil()), "nil source printing");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(20)))
        .http_status_as_error(false)
        .build()
        .into();
    let mut pairs = Vec::new();
    for (i, id) in prints.iter().enumerate() {
        let en =
            cached(args, &agent, *id, "en", &id.to_string())?.context("source print missing")?;
        ensure!(
            en.id == *id && en.lang == "en",
            "source printing identity/language changed"
        );
        ensure!(
            !en.set.is_empty()
                && en.set.len() <= 12
                && !en.collector_number.is_empty()
                && en.collector_number.len() <= 32,
            "invalid edition metadata"
        );
        let route = format!("{}/{}/de", segment(&en.set), segment(&en.collector_number));
        if let Some(de) = cached(args, &agent, *id, "de", &route)?
            && exact(&en, &de)
        {
            pairs.push(Pair {
                en: *id,
                de: de.id,
                oracle: en.oracle_id,
                set: en.set,
                number: en.collector_number,
            });
        }
        if (i + 1).is_multiple_of(25) {
            eprintln!("{} / {} printing pairs checked", i + 1, prints.len());
        }
    }
    let output = serde_json::json!({"schema":1,"source_count":prints.len(),"pairs":pairs});
    write(&args.output, &serde_json::to_vec_pretty(&output)?)?;
    eprintln!(
        "{} exact German editions, {} English fallbacks",
        pairs.len(),
        prints.len() - pairs.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn localized_print_requires_same_rules_edition_number_and_real_image() {
        let en = Card {
            id: Uuid::from_u128(1),
            oracle_id: Uuid::from_u128(3),
            set: "abc".into(),
            collector_number: "12".into(),
            lang: "en".into(),
            image_status: "highres_scan".into(),
        };
        let mut de = Card {
            id: Uuid::from_u128(2),
            lang: "de".into(),
            ..en.clone()
        };
        assert!(exact(&en, &de));
        de.oracle_id = Uuid::from_u128(4);
        assert!(!exact(&en, &de));
        de.oracle_id = en.oracle_id;
        de.collector_number = "12a".into();
        assert!(!exact(&en, &de));
        de.collector_number.clone_from(&en.collector_number);
        de.image_status = "placeholder".into();
        assert!(!exact(&en, &de));
    }
    #[test]
    fn remote_metadata_cannot_escape_a_path_segment() {
        assert_eq!(segment("12★/?#%"), "12%E2%98%85%2F%3F%23%25");
    }
}
