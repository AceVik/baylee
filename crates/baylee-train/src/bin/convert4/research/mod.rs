//! Explicit conversion-only research publication; no ordinary dataset/cache ABI.
use anyhow::{Context as _, ensure};
use baylee_train::{
    carddsl::{
        DSL_VERSION, card_ability_table, card_back_summary_table, card_summary_table,
        token_summary_table,
    },
    convert4::{
        RolloutStepSidecar4,
        research::{FRAME_CONTRACT, StreamedResearch4, stream_research4},
    },
    failure_terminal::{VerifiedEpisode, contract, sha},
};
use baylee_train_ops::files;
use clap::Parser;
use flate2::{Compression, write::GzEncoder};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufWriter, Write as _},
    path::{Path, PathBuf},
};

mod source;

#[derive(Parser)]
struct Args {
    /// Complete original research run directories; no ordinary/diagnostic runs.
    #[arg(
        long,
        value_delimiter = ',',
        required_unless_present = "attempt_ledger_runs",
        conflicts_with = "attempt_ledger_runs"
    )]
    runs: Vec<PathBuf>,
    /// Research runs with an existing verified final operational attempt ledger.
    #[arg(
        long,
        value_delimiter = ',',
        required_unless_present = "runs",
        conflicts_with = "runs"
    )]
    attempt_ledger_runs: Vec<PathBuf>,
    /// New separate research artifact directory; must not exist.
    #[arg(long)]
    out: PathBuf,
}

fn catalog(root: &Path) -> anyhow::Result<Value> {
    let mut cards: Vec<_> = baylee_cards::all().map(|card| card.index.get()).collect();
    cards.sort_unstable();
    cards.dedup();
    let structure = baylee_train::cardstructure::catalog(&cards)?;
    let structure_sha = baylee_train::cardstructure::digest(&structure)?;
    files::atomic_json(
        &root.join("card_structure.json"),
        &serde_json::to_value(&structure)?,
    )?;
    let tables = [
        ("card_dsl.bin", card_summary_table(&cards)),
        ("card_dsl_back.bin", card_back_summary_table(&cards)),
        ("token_dsl.bin", token_summary_table()),
        ("card_abilities.bin", card_ability_table(&cards)),
    ];
    for (name, values) in tables {
        super::write_f32_bin(&root.join(name), &values)?;
    }
    Ok(
        json!({"scope":"complete_compiled_pool","cards":cards,"structure_sha256":structure_sha,
        "dsl_version":DSL_VERSION,"tokens":baylee_cards::tokens::ALL.len(),
        "source_catalog_runtime_equivalence":"requires_separate_numerical_model_audit"}),
    )
}

fn output_seals(root: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let mut seals = BTreeMap::new();
    for name in [
        "attempts.jsonl",
        "card_structure.json",
        "card_dsl.bin",
        "card_dsl_back.bin",
        "token_dsl.bin",
        "card_abilities.bin",
    ] {
        seals.insert(name.into(), files::sha256(&root.join(name))?);
    }
    for entry in fs::read_dir(root.join("episodes"))? {
        let entry = entry?;
        ensure!(
            entry.file_type()?.is_file(),
            "unexpected research output directory member"
        );
        let name = format!(
            "episodes/{}",
            entry
                .file_name()
                .to_str()
                .context("research episode name")?
        );
        ensure!(
            !name.ends_with(".partial"),
            "unfinished research episode output"
        );
        seals.insert(name, files::sha256(&entry.path())?);
    }
    Ok(seals)
}

fn write_episode(
    root: &Path,
    id: u32,
    record: &[u8],
    sidecar: &[RolloutStepSidecar4],
    proof: &VerifiedEpisode,
) -> anyhow::Result<(String, StreamedResearch4)> {
    let name = format!("episode-{id:06}.research.jsonl.gz");
    let temporary = root.join("episodes").join(format!("{name}.partial"));
    let mut writer = GzEncoder::new(
        BufWriter::new(File::create(&temporary)?),
        Compression::fast(),
    );
    let converted = stream_research4(record, id, sidecar, proof, |frame| {
        serde_json::to_writer(&mut writer, &frame).map_err(|error| error.to_string())?;
        writer.write_all(b"\n").map_err(|error| error.to_string())
    })
    .map_err(anyhow::Error::msg)?;
    let mut file = writer.finish()?;
    file.flush()?;
    file.get_ref().sync_all()?;
    fs::rename(&temporary, root.join("episodes").join(&name))?;
    Ok((format!("episodes/{name}"), converted))
}

fn recorded_receipt(
    root: &Path,
    id: u32,
    run: &source::Run,
    game: &source::Game,
) -> anyhow::Result<Value> {
    let record = game.record()?;
    let proof = game.verify(&record, run.encoder)?;
    let unknown = proof.returns().training_utility.is_none();
    let mut receipt = json!({"episode_id":id,"source_run_sha256":run.source_sha,"source_game":game.row["game"],
        "source_index":game.row["i"],"source_record_sha256":game.row["record_sha256"],
        "source_sidecars_sha256":sha(&serde_json::to_vec(&game.row["rollout_sidecar"])?),
        "source_episode_sha256":sha(&serde_json::to_vec(&game.row["research_episode"])?),
        "source_row_sha256":sha(&serde_json::to_vec(&game.row)?),"actor_roles":game.row["actor_roles"],
        "last_included_input_n":proof.last_input_n(),"returns":proof.returns(),
        "target_disposition":if unknown {"excluded_unknown_no_target_rows"}else{"verified_research_return"},
        "episode_file":null,"frames":0,"actor_decisions":0});
    if unknown {
        let converted = stream_research4(&record, id, &game.sidecar, &proof, |_| {
            Err("unknown attempt emitted a target row".into())
        })
        .map_err(anyhow::Error::msg)?;
        ensure!(
            converted.excluded_unknown && converted.frames == 0,
            "unknown attempt acquired targets"
        );
    } else {
        let (name, converted) = write_episode(root, id, &record, &game.sidecar, &proof)?;
        receipt["episode_file"] = json!(name);
        receipt["frames"] = json!(converted.frames);
        receipt["actor_decisions"] = json!(converted.actor_decisions);
    }
    Ok(receipt)
}

fn unrecorded_receipt(run: &source::Run, accounting: &Value) -> anyhow::Result<Value> {
    let started = accounting["source_attempt_status"] == "operational_unknown";
    ensure!(
        started || accounting["source_attempt_status"] == "not_started",
        "unsupported recordless research attempt status"
    );
    Ok(
        json!({"episode_id":null,"source_run_sha256":run.source_sha,"source_game":null,
        "source_record_sha256":null,"source_sidecars_sha256":null,"source_episode_sha256":null,
        "unknown_reason":if started {accounting["source_attempt_ledger_entry"]["unknown_reason"].clone()}else{Value::Null},
        "returns":if started {json!({"termination_kind":"unknown","training_utility":null,"mtg_outcome":null,"bootstrap":false})}else{Value::Null},
        "target_disposition":if started {"excluded_operational_unknown_no_target_rows"}else{"not_started_no_target_rows"},
        "episode_file":null,"frames":0,"actor_decisions":0}),
    )
}

fn load_runs(args: &Args) -> anyhow::Result<Vec<source::Run>> {
    ensure!(
        args.runs.is_empty() != args.attempt_ledger_runs.is_empty(),
        "select exactly one research denominator adapter; mixed/empty sources are unsupported"
    );
    let finalized = !args.attempt_ledger_runs.is_empty();
    let mut runs = Vec::new();
    let mut source_roots = BTreeSet::new();
    for path in if finalized {
        &args.attempt_ledger_runs
    } else {
        &args.runs
    } {
        let run = if finalized {
            source::Run::load_finalized(path)?
        } else {
            source::Run::load(path)?
        };
        ensure!(
            source_roots.insert(run.root.clone()),
            "duplicate research source run"
        );
        if let Some(first) = runs.first() {
            let first: &source::Run = first;
            ensure!(
                run.metadata["model_identity"] == first.metadata["model_identity"]
                    && run.encoder == first.encoder,
                "research sources must use the exact same frozen learner and encoder"
            );
        }
        runs.push(run);
    }
    ensure!(!runs.is_empty(), "empty research source list");
    Ok(runs)
}

fn convert(args: &Args) -> anyhow::Result<()> {
    ensure!(!args.out.exists(), "research output already exists");
    let runs = load_runs(args)?;
    let finalized = !args.attempt_ledger_runs.is_empty();
    // Leave incomplete artifacts on failure, never an ordinary meta.json that
    // an existing cache/target loader could mistake for natural-outcome data.
    fs::create_dir(&args.out)?;
    fs::create_dir(args.out.join("episodes"))?;
    let catalog = catalog(&args.out)?;
    let mut ledger = BufWriter::new(File::create(args.out.join("attempts.jsonl"))?);
    let mut episodes = Vec::new();
    let mut attempts = 0_u64;
    let mut unknowns = 0_u64;
    let (mut assigned, mut recorded, mut operational_unknown, mut not_started) =
        (0_u64, 0_u64, 0_u64, 0_u64);
    let mut source_keys = BTreeSet::new();
    let mut attempt_keys = BTreeSet::new();
    for run in &runs {
        for attempt in run.attempts()? {
            attempts += 1;
            let id = u32::try_from(attempts)?;
            let mut receipt = if let Some(game) = attempt.game {
                ensure!(
                    source_keys.insert((
                        &run.source_sha,
                        game.row["game"].as_str().context("research source key")?
                    )),
                    "duplicate research attempt identity"
                );
                recorded_receipt(&args.out, id, run, game)?
            } else {
                unrecorded_receipt(run, &attempt.accounting)?
            };
            if finalized {
                ensure!(
                    attempt_keys.insert((
                        &run.source_sha,
                        attempt.accounting["source_index"]
                            .as_u64()
                            .context("planned source index")?
                    )),
                    "duplicate planned research attempt identity"
                );
                receipt.as_object_mut().context("research receipt")?.extend(
                    attempt
                        .accounting
                        .as_object()
                        .context("source accounting")?
                        .clone(),
                );
                receipt["attempt_id"] = json!(id);
                assigned += u64::from(receipt["assigned"] == true);
                recorded += u64::from(receipt["source_attempt_status"] == "recorded");
                operational_unknown +=
                    u64::from(receipt["source_attempt_status"] == "operational_unknown");
                not_started += u64::from(receipt["source_attempt_status"] == "not_started");
            }
            unknowns += u64::from(receipt["returns"]["termination_kind"] == "unknown");
            if !receipt["episode_file"].is_null() {
                episodes.push(receipt.clone());
            }
            serde_json::to_writer(&mut ledger, &receipt)?;
            ledger.write_all(b"\n")?;
        }
    }
    ledger.flush()?;
    ledger.get_ref().sync_all()?;
    for run in &runs {
        run.recheck()?;
    }
    let seals = output_seals(&args.out)?;
    let mut manifest = json!({"schema":"failure_terminal_research_conversion_v1","status":"conversion_only_unadmitted",
        "attempt_denominator_contract":"complete_index_v2","operational_abort_ledger_supported":false,
        "target_contract":contract(),"frame_contract":FRAME_CONTRACT,"preprocessing_version":1,
        "policy_provenance_version":2,"sampling_version":2,"encoder_version":runs[0].encoder,
        "learner_model_identity":runs[0].metadata["model_identity"],"ppo_eligible":false,
        "ordinary_target_cache_compatible":false,"catalog":catalog,
        "source_runs":runs.iter().map(source::Run::identity).collect::<Vec<_>>(),
        "planned_attempts":attempts,"recorded_indexed_attempts":attempts,"unknown_attempts":unknowns,
        "target_episodes":episodes,"output_files":seals,"converter_revision":baylee_build::COMMIT,"converter_dirty":baylee_build::DIRTY});
    if finalized {
        manifest["schema"] = json!("failure_terminal_research_conversion_v2");
        manifest["attempt_denominator_contract"] = json!("failure_terminal_attempt_ledger_v1");
        manifest["operational_abort_ledger_supported"] = json!(true);
        manifest
            .as_object_mut()
            .context("research manifest")?
            .remove("recorded_indexed_attempts");
        manifest["attempt_counts"] = json!({"planned":attempts,"assigned":assigned,"recorded":recorded,
            "operational_unknown":operational_unknown,"not_started":not_started,
            "recorded_unknown":unknowns-operational_unknown,"target_episodes":episodes.len()});
    }
    publish_manifest(&args.out, manifest, &runs)
}

fn publish_manifest(root: &Path, mut manifest: Value, runs: &[source::Run]) -> anyhow::Result<()> {
    manifest["content_fingerprint"] = json!(sha(&serde_json::to_vec(&manifest)?));
    files::atomic_json(&root.join("research.manifest.json"), &manifest)?;
    for run in runs {
        run.recheck()?;
    }
    files::atomic_json(
        &root.join("research.complete.json"),
        &json!({"schema":"failure_terminal_research_conversion_receipt_v1",
        "manifest_sha256":files::sha256(&root.join("research.manifest.json"))?,
        "content_fingerprint":manifest["content_fingerprint"],"admitted":false}),
    )?;
    Ok(())
}

pub(super) fn run() -> anyhow::Result<()> {
    convert(&Args::parse_from(std::env::args_os().skip(1)))
}

#[cfg(test)]
mod tests;
