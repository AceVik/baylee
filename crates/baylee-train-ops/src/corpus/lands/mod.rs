//! Public 17Lands bulk deck import, retaining no player metadata or gameplay labels.
mod rows;
#[cfg(test)]
mod tests;
use super::{
    cards::{Card, registry_cards},
    mtgo::similarity::SimilarityIndex,
};
use crate::files;
use anyhow::{Context, Result};
use flate2::read::MultiGzDecoder;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    path::PathBuf,
    time::Duration,
};
const LICENSE: &str = "https://creativecommons.org/licenses/by/4.0/";
const SOURCE_PREFIX: &str = "https://17lands-public.s3.amazonaws.com/analysis_data/game_data/";
#[derive(Clone)]
struct Source {
    expansion: String,
    mode: String,
    url: String,
}
/// Limits and optional pinned local inputs for a public bulk deck import.
pub struct Options {
    /// Native card catalog export.
    pub cards: PathBuf,
    /// New external corpus directory.
    pub out: PathBuf,
    /// Maximum inspected rows from each selected source.
    pub rows_per_source: u64,
    /// Maximum accepted decks from each selected source.
    pub decks_per_source: u64,
    /// Optional comma-separated expansion names.
    pub sets: String,
    /// Read an already captured public catalog instead of fetching it.
    pub catalog: Option<PathBuf>,
    /// Read named gzip bulk files from a local mirror instead of making requests.
    pub bulk_dir: Option<PathBuf>,
}
fn request(url: &str) -> Result<ureq::http::Response<ureq::Body>> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(45)))
        .build()
        .new_agent();
    Ok(agent
        .get(url)
        .header(
            "User-Agent",
            "baylee-training/4 (non-commercial deck research)",
        )
        .header("Accept-Encoding", "identity")
        .call()?)
}
fn fetch_catalog() -> Result<Value> {
    let base = "https://17lands.cdn.prismic.io/api/v2";
    let api: Value = request(base)?.body_mut().read_json()?;
    let reference = api["refs"]
        .as_array()
        .context("missing public catalog revisions")?
        .iter()
        .find(|r| r["isMasterRef"] == true)
        .and_then(|r| r["ref"].as_str())
        .context("missing master public catalog revision")?;
    let query = "[[at(document.type,\"public-data\")]]";
    let mut url = format!("{base}/documents/search?");
    form(&mut url, "ref", reference);
    url.push('&');
    form(&mut url, "q", query);
    Ok(request(&url)?.body_mut().read_json()?)
}
fn form(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push('=');
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(char::from(b));
        } else if b == b' ' {
            out.push('+');
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{b:02X}");
        }
    }
}
fn catalog_sources(catalog: &Value) -> Result<Vec<Source>> {
    let mut sources = BTreeMap::new();
    for entry in catalog["results"][0]["data"]["datasets"]
        .as_array()
        .context("missing published datasets")?
    {
        let expansion = entry["expansion"][0]["text"]
            .as_str()
            .context("missing expansion")?;
        let mode = entry["format"][0]["text"]
            .as_str()
            .context("missing format")?;
        let rank = match mode {
            "TradDraft" => 0,
            "TradSealed" => 1,
            "Sealed" => 2,
            _ => 3,
        };
        if let Some(paragraphs) = entry["game_data"].as_array() {
            for paragraph in paragraphs {
                if let Some(spans) = paragraph["spans"].as_array() {
                    for span in spans {
                        if let Some(url) = span["data"]["url"].as_str().filter(|url| {
                            url.starts_with(SOURCE_PREFIX) && url.ends_with(".csv.gz")
                        }) && sources
                            .get(expansion)
                            .is_none_or(|(prior, _): &(u8, Source)| rank < *prior)
                        {
                            sources.insert(
                                expansion.to_owned(),
                                (
                                    rank,
                                    Source {
                                        expansion: expansion.to_owned(),
                                        mode: mode.to_owned(),
                                        url: url.to_owned(),
                                    },
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(sources.into_values().map(|(_, source)| source).collect())
}
struct Importer {
    root: PathBuf,
    pool: Value,
    lookup: BTreeMap<String, Card>,
    accepted: Vec<Value>,
    similarity: SimilarityIndex,
    seen: BTreeSet<String>,
    fetched: String,
}
fn timestamp() -> String {
    let now = time::OffsetDateTime::now_utc();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}+00:00",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.microsecond()
    )
}
/// Import only complete supported decks from catalog-published bulk sources.
///
/// # Errors
/// Rejects occupied output directories, invalid catalogs and failed local publication.
/// Individual source failures are retained in import-report.json with admitted prefixes.
pub fn import(options: &Options) -> Result<Value> {
    if let Some(parent) = options.out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(&options.out)?;
    let root = options.out.canonicalize()?;
    std::fs::create_dir(root.join("decks"))?;
    let registry = files::read_json(&options.cards)?;
    let lookup = registry_cards(&registry)?;
    let catalog = if let Some(path) = &options.catalog {
        files::read_json(path)?
    } else {
        fetch_catalog()?
    };
    files::atomic_json(&root.join("source-catalog.json"), &catalog)?;
    let selected = if options.sets.is_empty() {
        None
    } else {
        Some(options.sets.split(',').collect::<BTreeSet<_>>())
    };
    let mut importer = Importer {
        root,
        pool: registry["working_pool_hash"].clone(),
        lookup,
        accepted: Vec::new(),
        similarity: SimilarityIndex::default(),
        seen: BTreeSet::new(),
        fetched: timestamp(),
    };
    let mut reports = Vec::new();
    for source in catalog_sources(&catalog)? {
        if selected
            .as_ref()
            .is_some_and(|sets| !sets.contains(source.expansion.as_str()))
        {
            continue;
        }
        let mut report = json!({"expansion":source.expansion,"mode":source.mode,"url":source.url,"retrieved_at":importer.fetched,"counts":{}});
        let result = (|| -> Result<()> {
            if let Some(root) = &options.bulk_dir {
                let name = source
                    .url
                    .rsplit('/')
                    .next()
                    .context("invalid bulk filename")?;
                let file = File::open(root.join(name))?;
                rows::read(
                    &mut importer,
                    &source,
                    MultiGzDecoder::new(file),
                    options,
                    &mut report,
                )
            } else {
                let mut response = request(&source.url)?;
                report["etag"] = response
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map_or(Value::Null, |v| json!(v));
                rows::read(
                    &mut importer,
                    &source,
                    MultiGzDecoder::new(response.body_mut().as_reader()),
                    options,
                    &mut report,
                )
            }
        })();
        if let Err(error) = result {
            report["error"] = json!(format!("NativeImportError: {error}"));
        }
        reports.push(report);
        files::atomic_json(
            &importer.root.join("manifest.json"),
            &json!({"decks":importer.accepted}),
        )?;
        files::atomic_json(&importer.root.join("import-report.json"), &reports)?;
    }
    let held = importer
        .accepted
        .iter()
        .filter(|d| d["contains_held_out_cards"] == true)
        .filter_map(|d| d["family_hash"].as_str())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for deck in &mut importer.accepted {
        if deck["family_hash"]
            .as_str()
            .is_some_and(|f| held.contains(f))
        {
            deck["split"] = json!("test");
            deck["holdout_tier"] = json!("unseen_cards_family");
        }
    }
    let manifest = json!({"decks":importer.accepted});
    files::atomic_json(&importer.root.join("manifest.json"), &manifest)?;
    Ok(manifest)
}
fn bump(report: &mut Value, key: &str) {
    let n = report["counts"][key].as_u64().unwrap_or(0);
    report["counts"][key] = json!(n + 1);
}
