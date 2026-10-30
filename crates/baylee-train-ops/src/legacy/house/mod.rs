//! Read archived flat columns one decision at a time; no training tensors or merged copy.
mod columns;
#[cfg(test)]
mod tests;
use crate::files;
use anyhow::{Context, Result, ensure};
use columns::{Columns, i64};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
#[derive(Default)]
struct Count {
    offered: u64,
    taken: u64,
}
fn column(meta: &Value, name: &str) -> Result<usize> {
    meta["meta_cols"]
        .as_array()
        .context("legacy metadata columns")?
        .iter()
        .position(|v| v == name)
        .with_context(|| format!("missing legacy column {name}"))
}
fn scan(
    columns: &mut Columns,
    priority: i32,
    indices: [usize; 3],
    counts: &mut BTreeMap<(u32, u16), Count>,
) -> Result<u64> {
    let mut ent_start = i64(&mut columns.ent_offset)?;
    let mut opt_start = i64(&mut columns.opt_offset)?;
    ensure!(
        ent_start == 0 && opt_start == 0,
        "legacy offsets must start at zero"
    );
    let mut decisions = 0;
    for _ in 0..columns.rows {
        let ent_end = i64(&mut columns.ent_offset)?;
        let opt_end = i64(&mut columns.opt_offset)?;
        ensure!(
            ent_start <= ent_end
                && ent_end <= columns.entities
                && opt_start <= opt_end
                && opt_end <= columns.options,
            "legacy offset outside column"
        );
        let info = columns.metadata()?;
        if info[indices[0]] == priority && info[indices[1]] > 1 {
            decisions += 1;
            let chosen = u64::try_from(info[indices[2]])?;
            ensure!(
                chosen < opt_end - opt_start,
                "legacy chosen action outside represented options"
            );
            let cards = columns.cards(ent_start, ent_end)?;
            for (index, [head, entity, verb]) in
                columns.options(opt_start, opt_end)?.into_iter().enumerate()
            {
                if !matches!(head, 1 | 2) {
                    continue;
                }
                let Ok(entity) = usize::try_from(entity) else {
                    continue;
                };
                let Some(card) = cards.get(entity) else {
                    continue;
                };
                let verb = u16::try_from(verb)?;
                let usage = if head == 1 {
                    verb
                } else {
                    100_u16.checked_add(verb).context("ability slot overflow")?
                };
                let count = counts.entry((*card, usage)).or_default();
                count.offered = count
                    .offered
                    .checked_add(1)
                    .context("offered count overflow")?;
                count.taken += u64::from(u64::try_from(index)? == chosen);
            }
        }
        ent_start = ent_end;
        opt_start = opt_end;
    }
    ensure!(
        ent_start == columns.entities && opt_start == columns.options,
        "legacy final offsets disagree with metadata"
    );
    Ok(decisions)
}
struct Inventory {
    names: BTreeMap<u32, String>,
    mana: BTreeSet<(u32, u16)>,
}
fn inventory(path: Option<&Path>) -> Result<Inventory> {
    let mut names = BTreeMap::new();
    let mut mana = BTreeSet::new();
    if let Some(path) = path.filter(|path| path.exists()) {
        for row in files::read_json(path)?
            .as_array()
            .context("pool inventory array")?
        {
            let card = u32::try_from(row["card"].as_u64().context("inventory card")?)?;
            names.insert(
                card,
                row["name"]
                    .as_str()
                    .context("inventory card name")?
                    .to_owned(),
            );
            for ability in row["abilities"].as_array().context("inventory abilities")? {
                let position = ability["position"].as_u64().context("ability position")?;
                if ability["kind"] == "mana" && position < 48 {
                    mana.insert((card, u16::try_from(position)?));
                }
            }
        }
    }
    Ok(Inventory { names, mana })
}
/// Count offered/chosen card uses in archived encoder2/3 priority decisions.
/// # Errors
/// Rejects malformed column sizes, offsets, labels and unsupported encoder versions.
pub fn analyze(root: &Path, names: Option<&Path>) -> Result<Value> {
    let meta = files::read_json(&root.join("dataset.json"))?;
    ensure!(
        matches!(meta["encoder_version"].as_u64(), Some(2 | 3)),
        "house usage requires encoder2 or3 options"
    );
    let priority = i32::try_from(
        meta["pending_kinds"]
            .as_array()
            .context("pending kinds")?
            .iter()
            .position(|v| v == "priority")
            .context("priority kind")?,
    )?;
    let indices = [
        column(&meta, "pending_kind")?,
        column(&meta, "n_opts")?,
        column(&meta, "chosen")?,
    ];
    let width = meta["meta_cols"]
        .as_array()
        .context("metadata columns")?
        .len();
    let mut counts = BTreeMap::new();
    let mut decisions = 0_u64;
    for shard in meta["shards"].as_array().context("legacy shards")? {
        if shard["samples"] == 0 {
            continue;
        }
        decisions = decisions
            .checked_add(scan(
                &mut Columns::open(root, shard, width)?,
                priority,
                indices,
                &mut counts,
            )?)
            .context("priority count overflow")?;
    }
    let Inventory { names, mana } = inventory(names)?;
    let first_non_card = meta["ids"]["ledger_rows"]
        .as_u64()
        .context("ledger rows")?
        .checked_add(1)
        .context("card row overflow")?;
    let verbs = ["land", "cast", "suspend", "mana", "pick"];
    let mut rows = Vec::new();
    for ((card, usage), count) in counts {
        if card == 0 || u64::from(card) >= first_non_card || usage == 3 {
            continue;
        }
        let label = if usage < 100 {
            verbs
                .get(usize::from(usage))
                .context("unknown archived card verb")?
                .to_string()
        } else {
            let slot = usage - 100;
            if slot >= 48 || mana.contains(&(card - 1, slot)) {
                continue;
            }
            format!("ability {slot}")
        };
        let rate =
            f64::from(u32::try_from(count.taken)?) / f64::from(u32::try_from(count.offered)?);
        rows.push(json!({"card_id":card,"card_index":card-1,"name":names.get(&(card-1)).map_or("?",String::as_str),"use":label,"offered":count.offered,"taken":count.taken,"rate":rate}));
    }
    rows.sort_by(|a, b| {
        a["rate"]
            .as_f64()
            .unwrap_or(0.)
            .total_cmp(&b["rate"].as_f64().unwrap_or(0.))
            .then_with(|| b["offered"].as_u64().cmp(&a["offered"].as_u64()))
    });
    Ok(
        json!({"data":root,"priority_decisions":decisions,"rows":rows,"interpretation":"offered-but-unused is a diagnostic signal, not a correctness label"}),
    )
}

/// Publish the archived usage report atomically, including its descriptive caveat.
/// # Errors
/// Returns dataset, serialization and filesystem failures.
pub fn report(root: &Path, out: &Path, names: Option<&Path>, minimum: u64) -> Result<Value> {
    use std::{fmt::Write as _, io::Write as _};
    let report = analyze(root, names)?;
    std::fs::create_dir_all(out)?;
    files::atomic_json(&out.join("usage.json"), &report)?;
    let mut text = format!(
        "# House usage — {}\n\n{} priority decisions with a choice. Offered at least {minimum} times, rarest taken first.\n\nOffered-but-unused is a diagnostic signal, not a correctness label.\n\n| card | use | offered | taken | rate |\n|---|---|---|---|---|\n",
        root.file_name().unwrap_or_default().to_string_lossy(),
        report["priority_decisions"]
    );
    for row in report["rows"]
        .as_array()
        .context("usage rows")?
        .iter()
        .filter(|r| r["offered"].as_u64().is_some_and(|n| n >= minimum))
        .take(60)
    {
        let name = row["name"]
            .as_str()
            .context("card name")?
            .replace('|', "\\|")
            .replace('\n', " ");
        writeln!(
            text,
            "| {name} ({}) | {} | {} | {} | {:.4} |",
            row["card_index"],
            row["use"].as_str().context("card usage")?,
            row["offered"],
            row["taken"],
            row["rate"].as_f64().context("usage rate")?
        )?;
    }
    let mut temporary = tempfile::NamedTempFile::new_in(out)?;
    temporary.write_all(text.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(out.join("usage.md"))?;
    Ok(report)
}
