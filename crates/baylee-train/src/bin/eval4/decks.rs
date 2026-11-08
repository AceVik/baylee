/// The manifest's decks this run selects, and every selected entry it could
/// not use. A deck whose file is missing or does not parse on this build
/// changes which decks a run plays; it is reported, never dropped in silence.
fn load_manifest_decks(
    manifest_path: &PathBuf,
    split: &str,
    shape_str: &str,
    holdout_tier: &str,
    held_out_cards: bool,
) -> anyhow::Result<(Vec<HouseDeck>, Vec<Value>)> {
    let raw = fs::read(manifest_path)
        .with_context(|| format!("reading manifest {}", manifest_path.display()))?;
    let val: Value = serde_json::from_slice(&raw)?;
    let (decks, skipped) = select_manifest_decks(
        &val,
        &repo_root(),
        split,
        shape_str,
        holdout_tier,
        held_out_cards,
    );
    if !skipped.is_empty() {
        eprintln!(
            "[eval4] {} selected manifest decks could not be used (manifest_skipped_decks)",
            skipped.len()
        );
    }
    Ok((decks, skipped))
}

fn select_manifest_decks(
    val: &Value,
    root: &std::path::Path,
    split: &str,
    shape_str: &str,
    holdout_tier: &str,
    held_out_cards: bool,
) -> (Vec<HouseDeck>, Vec<Value>) {
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    for entry in val["decks"].as_array().into_iter().flatten() {
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
        let id = entry["id"].clone();
        let Some(rel_path) = entry["path"].as_str() else {
            skipped.push(json!({"id": id, "reason": "no path"}));
            continue;
        };
        let text = match fs::read_to_string(root.join(rel_path)) {
            Ok(text) => text,
            Err(error) => {
                skipped.push(json!({"id": id, "path": rel_path, "reason": format!("unreadable: {error}")}));
                continue;
            }
        };
        let key = entry["id"].as_str().unwrap_or(rel_path).to_owned();
        match HouseDeck::parse_any(&key, &text) {
            Ok(deck) => out.push(deck),
            Err(error) => {
                skipped.push(json!({"id": id, "path": rel_path, "reason": format!("unparsable: {error}")}));
            }
        }
    }
    (out, skipped)
}

#[cfg(test)]
mod manifest_deck_tests {
    use super::*;

    /// A selected deck that is missing is reported; one the split filters
    /// out is not a skip.
    #[test]
    fn a_selected_deck_that_cannot_be_read_is_reported() {
        let root = std::env::temp_dir().join(format!("eval4-manifest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let manifest = json!({"decks": [
            {"id": "gone", "path": "missing.txt", "split": "test", "format": "constructed",
             "holdout_tier": "eval_benchmark"},
            {"id": "other-split", "path": "missing-too.txt", "split": "train",
             "format": "constructed", "holdout_tier": "train_active"},
        ]});
        let (decks, skipped) =
            select_manifest_decks(&manifest, &root, "test", "constructed", "", false);
        assert!(decks.is_empty());
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0]["id"], "gone");
        assert!(skipped[0]["reason"].as_str().unwrap().starts_with("unreadable"));
        fs::remove_dir_all(root).unwrap();
    }
}
