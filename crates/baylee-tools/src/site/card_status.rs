//! Landing-page overview and lazy per-set JSON, generated only on request.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::{Value, json};

use super::{bulk, catalog};

#[derive(clap::Args)]
pub(super) struct Args {
    #[arg(long, default_value = "HEAD")]
    rev: String,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long)]
    sets_cache: Option<PathBuf>,
    #[arg(long)]
    no_fetch: bool,
    #[arg(long)]
    bulk: Option<PathBuf>,
    #[arg(long)]
    fetch_bulk: bool,
    #[arg(long)]
    card_cache: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub(super) struct Card {
    pub name: String,
    pub oracle_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub img: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
}

type Sets = BTreeMap<String, Vec<Card>>;

pub(super) fn cards(
    catalog: &catalog::Catalog,
    printings: &bulk::Printings,
    previous: &BTreeMap<String, String>,
    facts: &bulk::Facts,
) -> Result<(Vec<String>, Sets)> {
    let mut order = Vec::new();
    let mut sets: BTreeMap<String, Vec<Card>> = BTreeMap::new();
    for row in &catalog.rows {
        if !sets.contains_key(&row.set) {
            order.push(row.set.clone());
        }
        let (status, note, original) = if let Some(reason) = catalog.never.get(&row.oracle) {
            ("never", Some(reason.clone()), None)
        } else if let Some(card) = catalog.pool.get(&row.oracle) {
            (
                card.status.as_str(),
                card.note.clone(),
                card.printing.as_deref(),
            )
        } else {
            ("absent", None, None)
        };
        let chosen = if let Some(slots) = printings.get(&row.oracle) {
            slots
                .get(&row.set)
                .map(String::as_str)
                .or(original)
                .or_else(|| slots.get("*").map(String::as_str))
        } else {
            original.or_else(|| previous.get(&row.oracle).map(String::as_str))
        };
        if let Some(id) = chosen {
            ensure!(bulk::valid_uuid(id), "{}: invalid Scryfall id", row.name);
        }
        let img = bulk::pictured(status, &row.set, chosen, facts);
        let printing = img.and_then(|id| facts.get(id));
        sets.entry(row.set.clone()).or_default().push(Card {
            name: row.name.clone(),
            oracle_id: row.oracle.clone(),
            status: status.into(),
            note,
            img: img.map(str::to_owned),
            uri: printing.and_then(|p| p.uri.clone()),
            artist: printing.and_then(|p| p.artist.clone()),
        });
    }
    let in_pool = sets
        .values()
        .flatten()
        .filter(|c| {
            ["implemented", "generated", "partial", "unimplemented"].contains(&c.status.as_str())
        })
        .count();
    ensure!(
        in_pool == catalog.pool.len(),
        "pool {} != catalog pool statuses {in_pool}",
        catalog.pool.len()
    );
    Ok((order, sets))
}

fn previous(
    out: &Path,
    need_ids: bool,
) -> Result<(BTreeMap<String, Value>, BTreeMap<String, String>)> {
    let path = out.join("sets.json");
    if !path.exists() {
        return Ok((BTreeMap::new(), BTreeMap::new()));
    }
    let doc: Value = serde_json::from_reader(BufReader::new(File::open(path)?))?;
    let mut sets = BTreeMap::new();
    let mut ids = BTreeMap::new();
    for s in doc["sets"]
        .as_array()
        .context("previous overview has no sets array")?
    {
        let code = bulk::string(s, "code")
            .filter(|s| bulk::valid_code(s))
            .context("previous overview has invalid set code")?;
        sets.insert(code.to_owned(), s.clone());
        let path = out.join("sets").join(format!("set-{code}.json"));
        if need_ids && path.exists() {
            let doc: Value = serde_json::from_reader(BufReader::new(File::open(path)?))?;
            for card in doc["cards"]
                .as_array()
                .context("previous set has no cards array")?
            {
                if let Some(img) = bulk::string(card, "img") {
                    let oid = bulk::string(card, "oracle_id")
                        .context("previous card has no oracle id")?;
                    ids.insert(oid.into(), img.into());
                }
            }
        }
    }
    Ok((sets, ids))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut temp =
        tempfile::NamedTempFile::new_in(path.parent().context("JSON path has no parent")?)?;
    serde_json::to_writer(&mut temp, value)?;
    writeln!(temp)?;
    temp.persist(path)?;
    Ok(())
}

// All output is derived from one validated revision; keep generation in one pipeline.
#[allow(clippy::too_many_lines)]
pub(super) fn run(root: &Path, args: Args) -> Result<()> {
    let (rev, files) = catalog::tree(root, &args.rev)?;
    let catalog = catalog::parse(&files)?;
    let from_root = |path: PathBuf| {
        if path.is_absolute() {
            path
        } else {
            root.join(path)
        }
    };
    let out = args
        .out
        .map_or_else(|| root.join("web/site/data"), from_root);
    let cache = args.sets_cache.map_or_else(
        || root.join("target/site-cache/scryfall-sets.json"),
        from_root,
    );
    let (previous, ids) = previous(&out, args.bulk.is_none())?;
    let input_bulk = args.bulk.map(from_root);
    let bulk = if input_bulk.is_none() && args.fetch_bulk {
        Some(bulk::fetch_bulk(cache.parent().unwrap_or(Path::new(".")))?)
    } else {
        input_bulk
    };
    let meta = bulk::sets(&cache, !args.no_fetch)?;
    let mut facts = bulk::printing_facts(
        &args
            .card_cache
            .map_or_else(|| root.join("data/scryfall-cache"), from_root),
    );
    let wanted: BTreeSet<_> = catalog.rows.iter().map(|r| r.oracle.clone()).collect();
    let printings = bulk.as_ref().map_or_else(
        || Ok(BTreeMap::new()),
        |p| bulk::bulk_printings(p, &wanted, Some(&mut facts)),
    )?;
    let (order, by_set) = cards(&catalog, &printings, &ids, &facts)?;
    let mut totals: BTreeMap<&str, usize> = catalog::STATUSES.into_iter().map(|s| (s, 0)).collect();
    let mut overview = Vec::new();
    for (position, code) in order.iter().enumerate() {
        let cards = &by_set[code];
        let mut entry = json!({ "code": code, "position": position,
            "name": meta.get(code).and_then(|v| bulk::string(v, "name")).or_else(|| previous.get(code).and_then(|v| bulk::string(v, "name"))).map_or_else(|| code.to_uppercase(), str::to_owned),
            "released": meta.get(code).and_then(|v| bulk::string(v, "released_at")).or_else(|| previous.get(code).and_then(|v| bulk::string(v, "released"))), "total": cards.len() });
        for status in catalog::STATUSES {
            let count = cards.iter().filter(|c| c.status == status).count();
            entry[status] = json!(count);
            *totals.get_mut(status).unwrap() += count;
        }
        overview.push(entry);
    }
    fs::create_dir_all(out.join("sets"))?;
    for code in &order {
        write_json(
            &out.join("sets").join(format!("set-{code}.json")),
            &json!({"code": code, "cards": by_set[code]}),
        )?;
    }
    // Only obsolete generated set files belong to this tool; leave unrelated files alone.
    for entry in fs::read_dir(out.join("sets"))? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(code) = name
            .strip_prefix("set-")
            .and_then(|n| n.strip_suffix(".json"))
            && bulk::valid_code(code)
            && !by_set.contains_key(code)
            && entry.file_type()?.is_file()
        {
            fs::remove_file(entry.path())?;
        }
    }
    let now = time::OffsetDateTime::now_utc();
    let generated = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    let mut head = serde_json::to_string(
        &json!({"rev": rev, "generated_at": generated, "corpus": catalog.rows.len(), "totals": totals}),
    )?;
    head.pop();
    let mut temp = tempfile::NamedTempFile::new_in(&out)?;
    writeln!(temp, "{head},\"sets\":[")?;
    for (i, set) in overview.iter().enumerate() {
        if i != 0 {
            writeln!(temp, ",")?;
        }
        serde_json::to_writer(&mut temp, set)?;
    }
    writeln!(temp, "\n]}}")?;
    temp.persist(out.join("sets.json"))?;
    println!(
        "rev {}: {} cards in {} sets, pool {} card files",
        &rev[..12],
        catalog.rows.len(),
        order.len(),
        catalog.pool.len()
    );
    println!("{}", serde_json::to_string(&totals)?);
    Ok(())
}

/// Whether the committed card status (`web/site/data/sets.json`) still
/// describes `revision`: it was measured at an ancestor of it, and nothing
/// the status is measured from ([`catalog::INPUTS`]) changed since. A release
/// tag must pass this, so the site's numbers are those of the client the
/// same release ships (`docs/releasing.md`).
///
/// # Errors
/// When the status is missing, unreadable, measured elsewhere, or stale.
pub(super) fn fresh(root: &Path, revision: &str) -> Result<()> {
    let sets = root.join("web/site/data/sets.json");
    let doc: Value = serde_json::from_str(
        &fs::read_to_string(&sets).context("web/site/data/sets.json cannot be read")?,
    )
    .context("web/site/data/sets.json is not JSON")?;
    let measured = doc
        .get("rev")
        .and_then(Value::as_str)
        .context("web/site/data/sets.json names no `rev`")?;
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
    };
    let head = git(&[
        "rev-parse",
        "--verify",
        "--end-of-options",
        &format!("{revision}^{{commit}}"),
    ])?;
    ensure!(head.status.success(), "cannot resolve {revision}");
    let head = String::from_utf8(head.stdout)?.trim().to_owned();
    let ancestor = git(&["merge-base", "--is-ancestor", measured, &head])?;
    ensure!(
        ancestor.status.success(),
        "the card status was measured at {measured}, which is not an ancestor of {head}; \
         run `cargo tools site card-status` and commit web/site/data"
    );
    let mut diff = vec!["diff", "--name-only", measured, head.as_str(), "--"];
    diff.extend(catalog::INPUTS);
    let changed = git(&diff)?;
    ensure!(
        changed.status.success(),
        "cannot compare {measured} with {head}"
    );
    let changed = String::from_utf8(changed.stdout)?;
    let files: Vec<&str> = changed.lines().collect();
    ensure!(
        files.is_empty(),
        "the card status was measured at {measured}, and {} card file(s) changed since \
         (first: {}); run `cargo tools site card-status` and commit web/site/data",
        files.len(),
        files.first().copied().unwrap_or_default()
    );
    println!("card status measured at {measured}: the pool of {head} is the same");
    Ok(())
}
