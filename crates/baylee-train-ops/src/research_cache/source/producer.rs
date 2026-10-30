use super::*;

pub(super) struct Proof {
    pub ledger: Value,
    pub planned: Value,
    pub game: Option<Value>,
    pub learner: u64,
}

#[cfg(target_os = "linux")]
pub(super) fn load(
    manifest: &Value,
    seals: &mut BTreeMap<PathBuf, String>,
) -> Result<BTreeMap<(String, u64), Proof>> {
    let mut proofs = BTreeMap::new();
    let mut roots = BTreeSet::new();
    for source in manifest["source_runs"]
        .as_array()
        .context("research source runs")?
    {
        let root = Path::new(
            source["directory"]
                .as_str()
                .context("research producer directory")?,
        );
        ensure!(
            root.is_absolute() && root.canonicalize()? == root && roots.insert(root.to_owned()),
            "duplicate or relocated research producer"
        );
        let ledger = crate::research_attempts::verify_finalized(root)?;
        ensure!(
            source["finalized_attempt_ledger"] == ledger,
            "producer ledger changed"
        );
        let mut expected: BTreeMap<PathBuf, String> =
            serde_json::from_value(ledger["source_files"].clone())?;
        for name in ["attempts.final.json", "attempts.complete.json"] {
            let path = bound(root, name)?;
            expected.insert(path.clone(), files::sha256(&path)?);
        }
        ensure!(
            source["source_files"] == serde_json::to_value(&expected)?
                && source["source_run_sha256"] == ledger["source_run_sha256"]
                && source["games_index_sha256"] == expected[&root.join("games.jsonl")],
            "external producer source seals differ"
        );
        let run = read_json(&root.join("run.json"))?;
        validate_run(manifest, source, &run)?;
        let plan = read_json(&root.join("attempts.plan.json"))?;
        let planned: BTreeMap<_, _> = plan["attempts"]
            .as_array()
            .context("producer planned attempts")?
            .iter()
            .map(|row| Ok((uint(&row["deal"])?, row.clone())))
            .collect::<Result<_>>()?;
        for row in ledger["attempts"]
            .as_array()
            .context("producer ledger attempts")?
        {
            let deal = uint(&row["deal"])?;
            let planned = planned.get(&deal).context("missing producer plan")?.clone();
            let learner = uint(&planned["focal_seat"])?;
            ensure!(
                learner == deal % 2
                    && planned["deck_keys"]
                        .as_array()
                        .is_some_and(|keys| keys.len() == 2),
                "producer planned role/decks mismatch"
            );
            let game = if row["status"] == "recorded" {
                let game = read_json(Path::new(
                    row["game_receipt"]
                        .as_str()
                        .context("producer game receipt")?,
                ))?;
                validate_roles(&game, &run, learner)?;
                Some(game)
            } else {
                None
            };
            let key = (
                source["source_run_sha256"]
                    .as_str()
                    .context("producer run SHA")?
                    .to_owned(),
                deal,
            );
            ensure!(
                proofs
                    .insert(
                        key,
                        Proof {
                            ledger: row.clone(),
                            planned,
                            game,
                            learner
                        }
                    )
                    .is_none(),
                "duplicate original planned attempt"
            );
        }
        seals.extend(expected);
    }
    ensure!(!proofs.is_empty(), "empty original planned denominator");
    Ok(proofs)
}

#[cfg(not(target_os = "linux"))]
pub(super) fn load(
    _: &Value,
    _: &mut BTreeMap<PathBuf, String>,
) -> Result<BTreeMap<(String, u64), Proof>> {
    anyhow::bail!("finalized research cache sources require Linux");
}

pub(super) fn recheck(manifest: &Value) -> Result<()> {
    load(manifest, &mut BTreeMap::new())?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_run(manifest: &Value, source: &Value, run: &Value) -> Result<()> {
    let c = &run["collection_contract"];
    ensure!(
        c == &source["collection_contract"]
            && c["version"] == 2
            && c["purpose"] == "failure_terminal_research_v1"
            && c["research_target_contract"] == contract()
            && c["precision"] == "float32"
            && c["seat_rng_rule"] == "independent_policy_seed_xor_physical_seat_v1"
            && run["training_data"] == false
            && run["seats"] == 2
            && run["policy_mode"] == "raw"
            && run["policy_contract"] == "raw_neural_sampler2_v1"
            && run["sampling_version"] == 2
            && run["policy_provenance_version"] == 2
            && run["thinking_steps"] == 2
            && run["temperature"]
                .as_f64()
                .is_some_and(|t| (t - 0.7).abs() < 1e-6)
            && run["model_identity"] == manifest["learner_model_identity"]
            && c["learner_model_identity"] == run["model_identity"]
            && run["model_identity"]["encoder_version"] == manifest["encoder_version"]
            && source["evaluator_revision"] == run["evaluator_revision"]
            && source["evaluator_dirty"] == run["evaluator_dirty"]
            && run["held_out_cards"] == false
            && run["holdout_tier"] == ""
            && run["shadow_teacher"].is_null()
            && (run["decks_manifest"].is_null() || run["deck_split"] == "train")
            && matches!(
                c["opponent_role"].as_str(),
                Some("historical" | "self_play")
            )
            && c["opponent_model_identity"]["encoder_version"] == manifest["encoder_version"]
            && (c["opponent_role"] != "self_play"
                || c["opponent_model_identity"] == c["learner_model_identity"]),
        "external producer research contract mismatch"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_roles(game: &Value, run: &Value, learner: u64) -> Result<()> {
    ensure!(
        game["focal_seat"] == learner,
        "recorded physical learner seat changed"
    );
    let roles = game["actor_roles"]
        .as_array()
        .context("original actor roles")?;
    let c = &run["collection_contract"];
    let mut seats = BTreeSet::new();
    for role in roles {
        let seat = uint(&role["seat"])?;
        let (name, identity) = if seat == learner {
            ("learner", &c["learner_model_identity"])
        } else {
            (
                c["opponent_role"].as_str().context("opponent role")?,
                &c["opponent_model_identity"],
            )
        };
        ensure!(
            seat < 2
                && seats.insert(seat)
                && role["role"] == name
                && role["checkpoint_sha256"] == identity["source_checkpoint_sha256"]
                && role["thinking_steps"] == 2
                && role["temperature"]
                    .as_f64()
                    .is_some_and(|t| (t - 0.7).abs() < 1e-6),
            "original actor role/model mismatch"
        );
    }
    ensure!(
        seats.len() == 2 && roles.len() == 2,
        "incomplete original actor roles"
    );
    Ok(())
}
