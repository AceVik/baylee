//! Explicit, selective restoration of reviewed cards to canonical stubs.

use crate::{BTreeSet, Path, card_files, catalog, fs, layout, ledger, scryfall, stubgen};
use anyhow::Context;

#[cfg(test)]
mod tests;

/// Plan every selected card before writing any, without fetching or regenerating tables.
pub(crate) fn run(
    root: &Path,
    cache: &Path,
    names: &[String],
    names_file: Option<&Path>,
    reason: &str,
    check: bool,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        !reason.trim().is_empty(),
        "hold-back reason must not be blank"
    );
    anyhow::ensure!(
        !reason.contains(['\n', '\r']),
        "hold-back reason must be one line"
    );
    let mut names: BTreeSet<String> = names.iter().cloned().collect();
    if let Some(path) = names_file {
        names.extend(
            fs::read_to_string(root.join(path))?
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_owned),
        );
    }
    anyhow::ensure!(!names.is_empty(), "select at least one card to hold back");
    let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS)?;
    let cats = compiled_catalogs();
    let cycles = layout::LandCycles::parse(&fs::read_to_string(root.join("data/land-cycles.tsv"))?);
    let cards = root.join("crates/baylee-cards/src/cards");
    let files = card_files(&cards)?;
    let cache = root.join(cache);
    let mut selected = BTreeSet::new();
    let mut plan = Vec::new();
    for name in names {
        let row = ledger.entry_named(&name)?;
        if !selected.insert(row.index) {
            continue;
        }
        let slug = crate::front_face_slug(&row.name);
        let path = files
            .get(&slug)
            .with_context(|| format!("no card file for {name}"))?;
        let source = fs::read_to_string(path)?;
        let printing = reviewed_printing(&cache, row, &source)?;
        let (_, output) =
            stubgen::render_unimplemented(&printing, row, &ledger, &cats, &cycles, reason)?;
        plan.push((path.clone(), output));
    }
    let sources: Vec<_> = plan.iter().map(|(_, text)| text.clone()).collect();
    let formatted = crate::files::format_rust_many(&root.join("target"), &sources)?;
    let mut changed = Vec::new();
    for ((path, _), text) in plan.iter().zip(formatted) {
        crate::write_verbatim(check, path, &text, &mut changed)?;
    }
    if check {
        anyhow::ensure!(
            changed.is_empty(),
            "{} selected cards need restoration",
            changed.len()
        );
    }
    println!(
        "held back {} selected cards; regenerate compiled tables separately with codegen --tables",
        plan.len()
    );
    Ok(())
}

/// Keep the reviewed card's printing and frozen identity, never today's default.
fn reviewed_printing(
    cache: &Path,
    row: &ledger::LedgerEntry,
    source: &str,
) -> anyhow::Result<scryfall::ScryfallCard> {
    anyhow::ensure!(
        !source.contains(stubgen::OWNED_MARKER),
        "{} is a finished machine-owned card; fix its reader before restoring it",
        row.name,
    );
    let string_field = |field| {
        crate::knob(source, field)
            .and_then(|value| crate::quoted_value(value, "\""))
            .with_context(|| format!("{} has no {field}", row.name))
    };
    anyhow::ensure!(
        string_field("oracle_id")? == row.oracle_id,
        "{} source identity disagrees with ledger",
        row.name
    );
    let id = string_field("scryfall_id")?;
    // Only conventional printing ids may select a cache path.
    anyhow::ensure!(
        id.len() == 36 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'),
        "invalid printing id for {}",
        row.name
    );
    let pinned = scryfall::printing_cache_path(cache, &id);
    let payload = if pinned.is_file() {
        fs::read_to_string(&pinned)?
    } else {
        fs::read_to_string(cache.join(format!("{}.json", stubgen::slug(&row.name)))).with_context(
            || {
                format!(
                    "missing cached printing {id} for {}; run scryfall-cache",
                    row.name
                )
            },
        )?
    };
    let printing: scryfall::ScryfallCard = serde_json::from_str(&payload)?;
    anyhow::ensure!(
        printing.id == id,
        "cached default for {} is another printing; fetch its pinned printing first",
        row.name
    );
    anyhow::ensure!(
        printing.oracle_id.as_deref() == Some(&row.oracle_id),
        "cached oracle identity disagrees with ledger for {}",
        row.name
    );
    Ok(printing)
}

/// Existing subtype assignments for selective operations, without refreshing catalogs.
pub(crate) fn compiled_catalogs() -> catalog::SubtypeCatalogs {
    use baylee_core::{generated::subtypes, ids::SubtypeId};
    let mut cats = catalog::SubtypeCatalogs::default();
    for raw in 0..subtypes::COUNT {
        let id = SubtypeId::new(raw);
        if let (Some(kind), Some(name)) = (subtypes::kind(id), subtypes::name(id)) {
            for (candidate, list) in cats.ordered_mut() {
                if candidate == kind {
                    list.push(name.to_owned());
                }
            }
        }
    }
    cats.normalize();
    cats
}
