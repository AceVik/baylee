use super::super::cards::{Card, fold};
use super::{Importer, LICENSE, Options, Source, bump};
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt::Write, io::Read};

fn identity(cards: &BTreeMap<u64, u64>) -> String {
    let text = format!(
        "[{}]",
        cards
            .iter()
            .map(|(card, count)| format!("[{card}, {count}]"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    files::hex(&Sha256::digest(text.as_bytes()))
}
pub(super) fn read(
    importer: &mut Importer,
    source: &Source,
    input: impl Read,
    options: &Options,
    report: &mut Value,
) -> Result<()> {
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(input);
    let header = reader
        .headers()?
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if i == 0 {
                s.trim_start_matches('\u{feff}').to_owned()
            } else {
                s.to_owned()
            }
        })
        .collect::<Vec<_>>();
    let columns = header
        .iter()
        .enumerate()
        .filter_map(|(i, name)| {
            name.strip_prefix("deck_")
                .map(|name| (i, importer.lookup.get(&fold(name)).cloned()))
        })
        .collect::<Vec<_>>();
    ensure!(!columns.is_empty(), "No deck_ columns in published schema");
    let known = columns.iter().filter(|(_, card)| card.is_some()).count();
    report["known_card_columns"] = json!(known);
    report["card_columns"] = json!(columns.len());
    if known * 5 < columns.len() * 2 {
        bump(report, "skipped_low_coverage_source");
        return Ok(());
    }
    let draft = header.iter().position(|name| name == "draft_id");
    for (number, row) in reader.records().enumerate() {
        if report["counts"]["rows"].as_u64().unwrap_or(0) >= options.rows_per_source
            || report["counts"]["accepted"].as_u64().unwrap_or(0) >= options.decks_per_source
        {
            break;
        }
        let row = row?;
        bump(report, "rows");
        if let Some((cards, names, held)) = deck(&row, &columns, report)? {
            admit(
                importer,
                source,
                (cards, names, held),
                draft
                    .map(|i| row.get(i).context("missing draft_id"))
                    .transpose()?,
                number + 2,
                report,
            )?;
        }
    }
    Ok(())
}
type Deck = (BTreeMap<u64, u64>, BTreeMap<u64, String>, bool);
fn deck(
    row: &csv::StringRecord,
    columns: &[(usize, Option<Card>)],
    report: &mut Value,
) -> Result<Option<Deck>> {
    let mut counts: BTreeMap<u64, i64> = BTreeMap::new();
    let mut names = BTreeMap::new();
    let mut held = false;
    for (column, card) in columns {
        let text = row.get(*column).context("missing deck column")?.trim();
        let count = if text.is_empty() {
            0
        } else {
            text.parse::<i64>()?
        };
        if count == 0 {
            continue;
        }
        let Some(card) = card else {
            bump(report, "unsupported_card");
            return Ok(None);
        };
        let current = counts.entry(card.id).or_default();
        *current = current.checked_add(count).context("card count overflow")?;
        names.insert(card.id, card.name.clone());
        held |= card.held;
    }
    let size = counts
        .values()
        .try_fold(0i64, |total, n| total.checked_add(*n))
        .context("deck size overflow")?;
    if !(40..=100).contains(&size) || counts.values().any(|n| *n < 1) {
        bump(report, "invalid_size");
        return Ok(None);
    }
    let counts = counts
        .into_iter()
        .map(|(id, n)| Ok((id, u64::try_from(n)?)))
        .collect::<Result<_>>()?;
    Ok(Some((counts, names, held)))
}
fn admit(
    importer: &mut Importer,
    source: &Source,
    deck: Deck,
    family: Option<&str>,
    number: usize,
    report: &mut Value,
) -> Result<()> {
    let (cards, names, held) = deck;
    let identity = identity(&cards);
    if !importer.seen.insert(identity.clone()) {
        bump(report, "duplicate");
        return Ok(());
    }
    if importer.similarity.maximum(&cards) >= 0.85 {
        bump(report, "near_duplicate");
        return Ok(());
    }
    let family = family.unwrap_or(&identity);
    let family_hash = files::hex(&Sha256::digest(
        format!("{}:{family}", source.expansion).as_bytes(),
    ));
    let bucket = u32::from_str_radix(&family_hash[..8], 16)? % 10;
    let split = if held || bucket == 0 {
        "test"
    } else if bucket == 1 {
        "val"
    } else {
        "train"
    };
    let path = importer.root.join("decks").join(format!("{identity}.txt"));
    let mut text = format!(
        "# baylee deck export v1\n# name: {} {} {}\n# format: freeform\n# source: 17Lands, {}, CSV row {number}\n# licence: CC BY 4.0, {LICENSE}\n# changes: none (card names normalized)\n",
        source.expansion,
        source.mode,
        &identity[..12],
        source.url
    );
    for (card, count) in &cards {
        writeln!(text, "{count} {}", names[card])?;
    }
    std::fs::write(&path, text)?;
    importer.accepted.push(json!({"id":format!("17lands/{}/{}",source.expansion,&identity[..20]),"name":format!("{} {} {}",source.expansion,source.mode,&identity[..12]),"path":path,"format":"limited","split":split,"holdout_tier":if held{"unseen_cards".to_owned()}else{format!("{split}_limited")},"contains_held_out_cards":held,"is_derived":false,"source_key":"17lands_public_bulk","source_ref":source.url,"source_row":number,"source_date":importer.fetched,"license":"CC-BY-4.0","license_url":LICENSE,"deck_hash":identity,"family_hash":family_hash,"main_count":cards.values().sum::<u64>(),"distinct_cards":cards.len(),"working_pool_hash":importer.pool}));
    importer.similarity.add(&cards);
    bump(report, "accepted");
    Ok(())
}
