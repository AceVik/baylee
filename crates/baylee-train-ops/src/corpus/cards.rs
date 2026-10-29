//! Shared authoritative card-name lookup for external deck importers.
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Clone)]
pub(super) struct Card {
    pub id: u64,
    pub name: String,
    pub held: bool,
}
pub(super) fn fold(value: &str) -> String {
    icu_casemap::CaseMapper::new()
        .fold_string(value)
        .into_owned()
}
pub(super) fn registry_cards(registry: &Value) -> Result<BTreeMap<String, Card>> {
    let mut lookup = BTreeMap::new();
    for row in registry["cards"]
        .as_array()
        .context("missing card registry")?
    {
        let card = Card {
            id: row["id"].as_u64().context("invalid card id")?,
            name: row["name"]
                .as_str()
                .context("missing card name")?
                .to_owned(),
            held: row["held_out"]
                .as_bool()
                .context("missing held-out marker")?,
        };
        let faces = row["faces"]
            .as_array()
            .context("missing card faces")?
            .iter()
            .map(|f| f.as_str().context("invalid face name"))
            .collect::<Result<Vec<_>>>()?;
        lookup.insert(fold(&card.name), card.clone());
        for face in &faces {
            lookup.insert(fold(face), card.clone());
        }
        lookup.insert(fold(&faces.join(" // ")), card);
    }
    Ok(lookup)
}
