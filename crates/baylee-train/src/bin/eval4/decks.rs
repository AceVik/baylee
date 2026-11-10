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

fn shape_matches(shape_filter: &str, entry_fmt: &str) -> bool {
    if shape_filter.is_empty() || shape_filter == "all" {
        return true;
    }
    shape_filter
        .split(',')
        .map(str::trim)
        .any(|part| part == "all" || part == entry_fmt)
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
        if !shape_matches(shape_str, entry_fmt) {
            continue;
        }
        if !holdout_tier.is_empty() {
            if entry_tier != holdout_tier {
                continue;
            }
        } else if split == "test" || split == "all" {
            let entry_held = entry["contains_held_out_cards"].as_bool().unwrap_or(false);
            if held_out_cards && entry_tier != "unseen_cards" {
                continue;
            }
            if !held_out_cards && (entry_tier == "unseen_cards" || entry_held) {
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
                skipped.push(
                    json!({"id": id, "path": rel_path, "reason": format!("unreadable: {error}")}),
                );
                continue;
            }
        };
        let key = entry["id"].as_str().unwrap_or(rel_path).to_owned();
        match HouseDeck::parse_any(&key, &text) {
            Ok(deck) => out.push(deck),
            Err(error) => {
                skipped.push(
                    json!({"id": id, "path": rel_path, "reason": format!("unparsable: {error}")}),
                );
            }
        }
    }
    (out, skipped)
}

fn shuffle_slice<T>(items: &mut [T], rng: &mut baylee_train::deckgen::Rng) {
    let len = items.len();
    for i in (1..len).rev() {
        let j = (rng.next() as usize) % (i + 1);
        items.swap(i, j);
    }
}

/// The house templates (`data/decks/*.txt`) `manifest` lists in another split
/// than `split`. A template is the same deck as its manifest entry under
/// another key (`victory` for `house/victory`), so the split that keeps the
/// entry out of a pool keeps the template out too.
fn templates_of_other_splits(
    manifest: &Value,
    root: &std::path::Path,
    split: &str,
) -> BTreeSet<PathBuf> {
    if split.is_empty() || split == "all" {
        return BTreeSet::new();
    }
    manifest["decks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["split"].as_str() != Some(split))
        .filter_map(|entry| entry["path"].as_str())
        .filter_map(|path| fs::canonicalize(root.join(path)).ok())
        .collect()
}

/// Expands training deck variance across the manifest decks of `split` in
/// every shape (`constructed`, `commander`, `highlander`, `starter_limited`),
/// the working house deck templates (`data/decks/*.txt`) the manifest does not
/// hold in another split, and procedural `deckgen` decks across all 6
/// archetypes and 3 formats (`Constructed`, `Highlander`, `Commander`), then
/// groups and shuffles them into `seats`-sized format-homogeneous table pods
/// seeded by `first_seed`.
///
/// `split` is the run's `--deck-split`, which an on-policy collection must
/// name `train`. Until 10.11.2026 this read every split: a collection seated
/// the val and test decks without held-out cards too.
///
/// Without a manifest, the repository's own (`data/decks/manifest.v4.json`)
/// still says which templates are held out of `split`.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn load_expanded_training_decks(
    manifest_path: Option<&PathBuf>,
    split: &str,
    working: &Working,
    held: &BTreeSet<baylee_core::ids::CardIndex>,
    pool: &deckgen::Pool,
    seats: usize,
    first_seed: u64,
    seed_base: u64,
    requested_generated: usize,
) -> anyhow::Result<(Vec<HouseDeck>, usize, Vec<Value>)> {
    let mut raw_decks = Vec::new();
    let mut skipped = Vec::new();
    let mut seen_keys = BTreeSet::new();
    let mut held_templates = BTreeSet::new();

    let repo_manifest = repo_root().join("data/decks/manifest.v4.json");
    let template_manifest = manifest_path.unwrap_or(&repo_manifest);
    if let Ok(raw) = fs::read(template_manifest) {
        let val: Value = serde_json::from_slice(&raw)
            .with_context(|| format!("reading manifest {}", template_manifest.display()))?;
        held_templates = templates_of_other_splits(&val, &repo_root(), split);
    }

    if let Some(mpath) = manifest_path {
        let raw =
            fs::read(mpath).with_context(|| format!("reading manifest {}", mpath.display()))?;
        let val: Value = serde_json::from_slice(&raw)?;
        let (loaded, m_skipped) =
            select_manifest_decks(&val, &repo_root(), split, "all", "", false);
        skipped = m_skipped;
        for deck in loaded {
            let usable = if deck.failing(working).is_empty() {
                Some(deck)
            } else {
                deck.working_only(working).ok().map(|(cut, _)| cut)
            };
            if let Some(d) = usable
                && d.failing(working).is_empty()
                && seen_keys.insert(d.key.clone())
            {
                raw_decks.push(d);
            }
        }
    }

    if let Ok(entries) = fs::read_dir(baylee_train::housedeck::dir()) {
        let mut paths: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
            .collect();
        paths.sort();
        for path in paths {
            if fs::canonicalize(&path).is_ok_and(|path| held_templates.contains(&path)) {
                continue;
            }
            let Ok(house) = HouseDeck::load(&path) else {
                continue;
            };
            let usable = if house.failing(working).is_empty() {
                Some(house)
            } else {
                house.working_only(working).ok().map(|(cut, _)| cut)
            };
            if let Some(deck) = usable
                && deck.failing(working).is_empty()
                && !deck.cards().any(|c| held.contains(&c))
                && seen_keys.insert(deck.key.clone())
            {
                raw_decks.push(deck);
            }
        }
    }

    let manifest_and_house_count = raw_decks.len();
    let gen_target = requested_generated.max(64);
    let shapes = [Shape::Constructed, Shape::Highlander, Shape::Commander];
    let mut generated = 0;
    let mut attempt = 0_u64;
    let gen_seed_base = seed_base
        .wrapping_mul(1_000_003)
        .wrapping_add(first_seed.wrapping_mul(6_364_136_223_846_793_005));
    while generated < gen_target && attempt < 4_000 {
        let shape = shapes[(generated + (attempt as usize)) % shapes.len()];
        let arch = Archetype::ALL[(generated + (attempt as usize)) % Archetype::ALL.len()];
        let s = gen_seed_base.wrapping_add(attempt);
        attempt += 1;
        let Ok(g) = deckgen::generate(pool, shape, arch, s) else {
            continue;
        };
        if g.main.iter().chain(&g.leaders).any(|c| held.contains(c)) {
            continue;
        }
        let shape_tag = match shape {
            Shape::Constructed => "con",
            Shape::Highlander => "hl",
            Shape::Commander => "cmd",
        };
        let key = format!("gen-{shape_tag}-{generated:03}");
        if seen_keys.insert(key.clone()) {
            raw_decks.push(HouseDeck {
                key,
                deck: g.loaded(),
            });
            generated += 1;
        }
    }

    let mut rng = baylee_train::deckgen::Rng::new(first_seed ^ 0xdec0_de42_9911_7733);
    let (mut commander_decks, mut constructed_decks): (Vec<HouseDeck>, Vec<HouseDeck>) = raw_decks
        .into_iter()
        .partition(|d| !d.deck.commanders.is_empty());
    shuffle_slice(&mut constructed_decks, &mut rng);
    shuffle_slice(&mut commander_decks, &mut rng);

    let mut pods: Vec<Vec<HouseDeck>> = Vec::new();
    for chunk in constructed_decks.chunks_exact(seats) {
        pods.push(chunk.to_vec());
    }
    for chunk in commander_decks.chunks_exact(seats) {
        pods.push(chunk.to_vec());
    }
    shuffle_slice(&mut pods, &mut rng);

    let mut final_decks: Vec<HouseDeck> = pods.into_iter().flatten().collect();
    let con_rem = constructed_decks.len() % seats;
    if con_rem > 0 {
        final_decks.extend_from_slice(&constructed_decks[constructed_decks.len() - con_rem..]);
    }
    let cmd_rem = commander_decks.len() % seats;
    if cmd_rem > 0 {
        final_decks.extend_from_slice(&commander_decks[commander_decks.len() - cmd_rem..]);
    }

    let final_manifest_count = manifest_and_house_count.min(final_decks.len());
    Ok((final_decks, final_manifest_count, skipped))
}

#[cfg(test)]
mod manifest_deck_tests {
    use super::*;
    use std::collections::BTreeMap;

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
        assert!(
            skipped[0]["reason"]
                .as_str()
                .unwrap()
                .starts_with("unreadable")
        );
        fs::remove_dir_all(root).unwrap();
    }

    /// The manifest's ids by split.
    fn splits(manifest: &Value) -> BTreeMap<String, String> {
        manifest["decks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                (
                    entry["id"].as_str().unwrap().to_owned(),
                    entry["split"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    }

    /// The house templates the manifest lists outside the train split, by
    /// the key a template is loaded under (`victory` for `data/decks/victory.txt`).
    fn held_template_keys(manifest: &Value) -> BTreeSet<String> {
        manifest["decks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["split"] != "train")
            .filter_map(|entry| entry["path"].as_str())
            .filter(|path| path.starts_with("data/decks/") && path.matches('/').count() == 2)
            .map(|path| {
                path["data/decks/".len()..]
                    .trim_end_matches(".txt")
                    .to_owned()
            })
            .collect()
    }

    /// Every deck an on-policy collection can seat is of the train split:
    /// no val or test deck of the manifest, and no house template the
    /// manifest holds out of train (`victory` among them).
    fn assert_train_only(decks: &[HouseDeck], manifest: &Value) {
        let splits = splits(manifest);
        let held_templates = held_template_keys(manifest);
        assert!(!held_templates.is_empty());
        for deck in decks {
            if let Some(split) = splits.get(&deck.key) {
                assert_eq!(
                    split, "train",
                    "{} of the {split} split was seated",
                    deck.key
                );
            }
            assert!(
                !held_templates.contains(&deck.key),
                "the house template {} the manifest holds out was seated",
                deck.key
            );
        }
    }

    /// A train collection, by either path a run takes to its decks, never
    /// seats a val or test deck: the expanded pool of an on-policy
    /// collection or of `--shape all`, and the plain selection of every other
    /// run that records (`--shadow-teacher`, `--tactician-explore`).
    #[test]
    fn a_train_collection_never_seats_a_test_or_val_deck() {
        let working = Working::scan(&repo_root()).unwrap();
        let held = deckgen::held_out(&working, 20);
        let pool = deckgen::pool_for_working(&working, &held);
        let manifest_path = repo_root().join("data/decks/manifest.v4.json");
        let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        for seats in [2, 4] {
            let (decks, _, _) = load_expanded_training_decks(
                Some(&manifest_path),
                "train",
                &working,
                &held,
                &pool,
                seats,
                1_130_301,
                7_777,
                64,
            )
            .unwrap();
            assert_train_only(&decks, &manifest);
        }
        // Without a manifest, the repository's still keeps its held-out templates out.
        let (decks, _, _) = load_expanded_training_decks(
            None, "train", &working, &held, &pool, 2, 1_130_301, 7_777, 64,
        )
        .unwrap();
        assert_train_only(&decks, &manifest);
        for shape in ["all", "constructed", "commander"] {
            let (decks, _) =
                select_manifest_decks(&manifest, &repo_root(), "train", shape, "", false);
            assert!(!decks.is_empty(), "{shape}");
            assert_train_only(&decks, &manifest);
        }
    }

    /// A split carved out of train (`holdout2`) is selected by its name and
    /// is in no train pool, the plain one or the expanded one.
    #[test]
    fn a_carved_out_split_is_selected_by_name_and_never_as_train() {
        let manifest = json!({"decks": [
            {"id": "eval/azorius-control", "path": "data/decks/eval/azorius-control.txt",
             "split": "holdout2", "format": "constructed"},
            {"id": "eval/red-burn", "path": "data/decks/eval/red-burn.txt",
             "split": "train", "format": "constructed"},
        ]});
        let root = repo_root();
        let keys =
            |decks: Vec<HouseDeck>| decks.into_iter().map(|deck| deck.key).collect::<Vec<_>>();
        let (holdout, _) =
            select_manifest_decks(&manifest, &root, "holdout2", "constructed", "", false);
        assert_eq!(keys(holdout), ["eval/azorius-control"]);
        let (train, _) = select_manifest_decks(&manifest, &root, "train", "all", "", false);
        assert_eq!(keys(train), ["eval/red-burn"]);
        let path = std::env::temp_dir().join(format!("holdout2-{}.json", std::process::id()));
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let working = Working::scan(&root).unwrap();
        let held = deckgen::held_out(&working, 20);
        let pool = deckgen::pool_for_working(&working, &held);
        let (expanded, _, _) = load_expanded_training_decks(
            Some(&path),
            "train",
            &working,
            &held,
            &pool,
            2,
            7,
            7_777,
            64,
        )
        .unwrap();
        assert!(
            expanded
                .iter()
                .all(|deck| deck.key != "eval/azorius-control")
        );
        assert!(expanded.iter().any(|deck| deck.key == "eval/red-burn"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_template_is_held_out_where_its_manifest_entry_is() {
        let manifest = json!({"decks": [
            {"id": "house/victory", "path": "data/decks/victory.txt", "split": "test"},
            {"id": "house/breya", "path": "data/decks/breya.txt", "split": "train"},
        ]});
        let root = repo_root();
        let victory = fs::canonicalize(root.join("data/decks/victory.txt")).unwrap();
        let held = templates_of_other_splits(&manifest, &root, "train");
        assert_eq!(held, BTreeSet::from([victory.clone()]));
        assert!(templates_of_other_splits(&manifest, &root, "all").is_empty());
        assert!(!templates_of_other_splits(&manifest, &root, "test").contains(&victory));
    }

    #[test]
    fn expanded_training_decks_include_all_formats_house_templates_and_procedural_pods() {
        let working = Working::scan(&repo_root()).unwrap();
        let held = deckgen::held_out(&working, 20);
        let pool = deckgen::pool_for_working(&working, &held);
        let manifest_path = repo_root().join("data/decks/manifest.v4.json");
        let (decks, manifest_count, _skipped) = load_expanded_training_decks(
            Some(&manifest_path),
            "train",
            &working,
            &held,
            &pool,
            4,
            1_006_000,
            7_777,
            64,
        )
        .unwrap();
        assert!(
            manifest_count >= 900,
            "expected >= 900 train manifest and house decks, got {manifest_count}"
        );
        for format in ["commander", "highlander", "starter_limited"] {
            let manifest: Value =
                serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
            let of_format: BTreeSet<&str> = manifest["decks"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|entry| entry["split"] == "train" && entry["format"] == format)
                .filter_map(|entry| entry["id"].as_str())
                .collect();
            assert!(
                decks
                    .iter()
                    .any(|deck| of_format.contains(deck.key.as_str())),
                "no {format} deck of the train split"
            );
        }
        assert!(
            decks.len() >= manifest_count + 64,
            "expected procedural decks to be added, got total {}",
            decks.len()
        );
        // Verify first 100 4-seat pods are format-homogeneous (all Commander or all non-Commander)
        for pod in decks.as_chunks::<4>().0.iter().take(100) {
            let is_cmd = !pod[0].deck.commanders.is_empty();
            assert!(
                pod.iter().all(|d| d.deck.commanders.is_empty() != is_cmd),
                "mixed Commander and Constructed decks in the same 4-seat pod"
            );
        }
    }
}
