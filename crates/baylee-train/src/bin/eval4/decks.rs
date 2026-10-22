fn load_manifest_decks(
    manifest_path: &PathBuf,
    split: &str,
    shape_str: &str,
    holdout_tier: &str,
    held_out_cards: bool,
) -> anyhow::Result<Vec<HouseDeck>> {
    let raw = fs::read(manifest_path)
        .with_context(|| format!("reading manifest {}", manifest_path.display()))?;
    let val: Value = serde_json::from_slice(&raw)?;
    let root = repo_root();
    let mut out = Vec::new();
    if let Some(entries) = val["decks"].as_array() {
        for entry in entries {
            let entry_split = entry["split"].as_str().unwrap_or("");
            let entry_fmt = entry["format"].as_str().unwrap_or("");
            let entry_tier = entry["holdout_tier"].as_str().unwrap_or("");
            if !split.is_empty() && split != "all" && entry_split != split {
                continue;
            }
            if !shape_str.is_empty() && entry_fmt != shape_str {
                continue;
            }
            if !holdout_tier.is_empty() {
                if entry_tier != holdout_tier {
                    continue;
                }
            } else if split == "test" {
                if held_out_cards && entry_tier != "unseen_cards" {
                    continue;
                }
                if !held_out_cards && entry_tier == "unseen_cards" {
                    continue;
                }
            }
            let Some(rel_path) = entry["path"].as_str() else {
                continue;
            };
            let full_path = root.join(rel_path);
            let Ok(text) = fs::read_to_string(&full_path) else {
                continue;
            };
            let key = entry["id"].as_str().unwrap_or(rel_path).to_owned();
            let Ok(hd) = HouseDeck::parse_any(&key, &text) else {
                continue;
            };
            out.push(hd);
        }
    }
    Ok(out)
}

