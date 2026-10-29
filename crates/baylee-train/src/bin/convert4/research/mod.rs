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
    #[arg(long, value_delimiter = ',', required = true)]
    runs: Vec<PathBuf>,
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

fn convert(args: &Args) -> anyhow::Result<()> {
    ensure!(!args.out.exists(), "research output already exists");
    let mut runs = Vec::new();
    let mut source_roots = BTreeSet::new();
    for path in &args.runs {
        let run = source::Run::load(path)?;
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
    // Leave incomplete artifacts on failure, never an ordinary meta.json that
    // an existing cache/target loader could mistake for natural-outcome data.
    fs::create_dir(&args.out)?;
    fs::create_dir(args.out.join("episodes"))?;
    let catalog = catalog(&args.out)?;
    let mut ledger = BufWriter::new(File::create(args.out.join("attempts.jsonl"))?);
    let mut episodes = Vec::new();
    let mut attempts = 0_u64;
    let mut unknowns = 0_u64;
    let mut source_keys = BTreeSet::new();
    for run in &runs {
        for game in &run.games {
            let source_key = (
                &run.source_sha,
                game.row["game"].as_str().context("research source key")?,
            );
            ensure!(
                source_keys.insert(source_key),
                "duplicate research attempt identity"
            );
            attempts += 1;
            let id = u32::try_from(attempts)?;
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
                unknowns += 1;
                let converted = stream_research4(&record, id, &game.sidecar, &proof, |_| {
                    Err("unknown attempt emitted a target row".into())
                })
                .map_err(anyhow::Error::msg)?;
                ensure!(
                    converted.excluded_unknown && converted.frames == 0,
                    "unknown attempt acquired targets"
                );
            } else {
                let (name, converted) =
                    write_episode(&args.out, id, &record, &game.sidecar, &proof)?;
                receipt["episode_file"] = json!(name);
                receipt["frames"] = json!(converted.frames);
                receipt["actor_decisions"] = json!(converted.actor_decisions);
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
    manifest["content_fingerprint"] = json!(sha(&serde_json::to_vec(&manifest)?));
    files::atomic_json(&args.out.join("research.manifest.json"), &manifest)?;
    for run in &runs {
        run.recheck()?;
    }
    files::atomic_json(
        &args.out.join("research.complete.json"),
        &json!({"schema":"failure_terminal_research_conversion_receipt_v1",
        "manifest_sha256":files::sha256(&args.out.join("research.manifest.json"))?,
        "content_fingerprint":manifest["content_fingerprint"],"admitted":false}),
    )?;
    Ok(())
}

pub(super) fn run() -> anyhow::Result<()> {
    convert(&Args::parse_from(std::env::args_os().skip(1)))
}

#[cfg(test)]
mod tests;
