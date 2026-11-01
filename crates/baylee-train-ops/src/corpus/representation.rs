//! Exact public structural-summary alias counts, without training-data changes.
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
fn floats(path: &Path) -> Result<Vec<u8>> {
    let bytes = std::fs::read(path)?;
    ensure!(bytes.len() % 4 == 0, "Malformed float32 representation");
    ensure!(
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .all(|b| f32::from_le_bytes(*b).is_finite()),
        "Nonfinite structural representation"
    );
    Ok(bytes)
}
/// Measure byte-exact front-summary aliases and summary-plus-ability aliases.
///
/// # Errors
/// Rejects missing files, incompatible matrix shapes or nonfinite features.
pub fn audit(data: &Path, catalog: &Path) -> Result<Value> {
    let meta = files::read_json(&data.join("meta.json"))?;
    let catalog = files::read_json(catalog)?;
    let mut names = BTreeMap::new();
    for row in catalog["cards"]
        .as_array()
        .context("missing catalog cards")?
    {
        names.insert(
            row["id"].as_u64().context("invalid catalog id")?,
            row["name"]
                .as_str()
                .context("invalid card name")?
                .to_owned(),
        );
    }
    let cards = meta["cards"].as_array().context("missing dataset cards")?;
    ensure!(!cards.is_empty(), "empty structural representation");
    let summary = floats(&data.join("card_dsl.bin"))?;
    let abilities = floats(&data.join("card_abilities.bin"))?;
    ensure!(
        summary.len()
            == cards
                .len()
                .checked_mul(160 * 4)
                .context("summary shape overflow")?
            && abilities.len() % cards.len() == 0,
        "Malformed structural representation shape"
    );
    let width = abilities.len() / cards.len();
    ensure!(width % 4 == 0, "Malformed ability representation shape");
    let mut summaries: BTreeMap<Vec<u8>, Vec<Value>> = BTreeMap::new();
    let mut combined: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    for (index, card) in cards.iter().enumerate() {
        let card = card.as_u64().context("invalid dataset card id")?;
        let Some(name) = names.get(&card) else {
            continue;
        };
        let row = &summary[index * 640..(index + 1) * 640];
        summaries
            .entry(row.to_vec())
            .or_default()
            .push(json!({"id":card,"name":name}));
        let mut key = row.to_vec();
        key.extend_from_slice(&abilities[index * width..(index + 1) * width]);
        *combined.entry(key).or_default() += 1;
    }
    let working: usize = summaries.values().map(Vec::len).sum();
    let unique = summaries.len();
    let mut aliases: Vec<_> = summaries.into_values().filter(|g| g.len() > 1).collect();
    aliases.sort_by(|a, b| {
        b.len()
            .cmp(&a.len())
            .then_with(|| a[0]["id"].as_u64().cmp(&b[0]["id"].as_u64()))
    });
    Ok(
        json!({"encoder_version":meta["encoder_version"],"dsl_version":meta["dsl_version"],"card_dsl_sha256":files::sha256(&data.join("card_dsl.bin"))?,"working_cards":working,"unique_front_summaries":unique,"collision_groups":aliases.len(),"cards_in_collision_groups":aliases.iter().map(Vec::len).sum::<usize>(),"summary_plus_ability_collision_groups":combined.values().filter(|n|**n>1).count(),"summary_plus_ability_colliding_cards":combined.values().filter(|n|**n>1).sum::<usize>(),"scope":"Front face only. Combined ability rows are diagnostic and are not all actor inputs.","interpretation":"Check rule differences; exact aliases include legitimate functional reprints.","groups":aliases}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_aliases_preserve_signed_zero_and_ability_distinctions() -> Result<()> {
        let root = tempfile::tempdir()?;
        let root = root.path();
        files::atomic_json(
            &root.join("meta.json"),
            &json!({"cards":[3,2,7,9],"encoder_version":7,"dsl_version":1}),
        )?;
        files::atomic_json(
            &root.join("catalog.json"),
            &json!({"cards":[{"id":2,"name":"two"},{"id":3,"name":"three"},{"id":7,"name":"seven"}]}),
        )?;
        let mut raw = vec![0u8; 4 * 640];
        raw[2 * 640..2 * 640 + 4].copy_from_slice(&(-0f32).to_le_bytes());
        std::fs::write(root.join("card_dsl.bin"), raw)?;
        let abilities = [0f32, 1.0, 0.0, 0.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        std::fs::write(root.join("card_abilities.bin"), abilities)?;
        let report = audit(root, &root.join("catalog.json"))?;
        assert_eq!(report["working_cards"], 3);
        assert_eq!(report["collision_groups"], 1);
        assert_eq!(report["groups"][0][0]["id"], 3);
        assert_eq!(report["summary_plus_ability_collision_groups"], 0);
        std::fs::write(root.join("card_abilities.bin"), f32::NAN.to_le_bytes())?;
        assert!(audit(root, &root.join("catalog.json")).is_err());
        Ok(())
    }
}
