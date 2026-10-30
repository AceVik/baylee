//! Read-only development audit of immutable source episodes, never conversion.
use super::*;
use clap::Parser;
use flate2::read::GzDecoder;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read as _, Seek as _, SeekFrom, Write as _},
    path::{Path, PathBuf},
};

#[derive(Parser)]
struct Args {
    /// Source run directories; immutable run.json/games.jsonl/indexed gzip records.
    #[arg(long, value_delimiter = ',', required = true)]
    runs: Vec<PathBuf>,
    /// Fixed case denominator; no replacements or retries are collected.
    #[arg(long, value_delimiter = ',', default_value = "64,65,96,97")]
    cases: Vec<u64>,
    /// Existing immutable source-file SHA-256 map.
    #[arg(long, requires = "reference_report")]
    input_seals: Option<PathBuf>,
    /// Existing Gate A report, binding case records, roles and original sidecars.
    #[arg(long, requires = "input_seals")]
    reference_report: Option<PathBuf>,
    /// New audit artifact; existing artifacts are never overwritten.
    #[arg(long)]
    out: PathBuf,
}

fn references(directory: &Path, row: &Value, seals: &Value, report: &Value) -> anyhow::Result<()> {
    let shard = row["shard"].as_str().context("source shard")?;
    ensure!(
        Path::new(shard).components().count() == 1,
        "source shard must be a filename"
    );
    for path in [
        directory.join("run.json"),
        directory.join("games.jsonl"),
        directory.join("records").join(shard),
    ] {
        let key = path.to_str().context("non-UTF8 source path")?;
        let expected = seals[key]
            .as_str()
            .context("source absent from reference seals")?;
        ensure!(
            baylee_train_ops::files::sha256(&path)? == expected,
            "reference file seal mismatch: {key}"
        );
    }
    let reference = report["rows"]
        .as_array()
        .context("reference report rows")?
        .iter()
        .find(|item| item["deal"] == row["i"])
        .context("case absent from reference report")?;
    ensure!(
        reference["source_record_sha256"] == row["record_sha256"]
            && reference["source_rollout_sidecar_sha256"]
                == crate::shadow::sha(&serde_json::to_vec(&row["rollout_sidecar"])?),
        "case record/sidecar differs from reference report"
    );
    ensure!(
        reference["learner_seat"] == row["focal_seat"]
            && reference["fallback_events"] == row["fallback_events"],
        "case roles or recovery boundary differ from reference report"
    );
    if !reference["first_recovery_boundary"].is_null() {
        let boundary = &reference["first_recovery_boundary"];
        let bytes = member(directory, row)?;
        let offset = usize::try_from(
            boundary["record_byte_offset"]
                .as_u64()
                .context("reference prefix offset")?,
        )?;
        ensure!(
            crate::shadow::sha(
                bytes
                    .get(..offset)
                    .context("reference prefix outside record")?
            ) == boundary["raw_prefix_sha256"],
            "reference prefix byte seal mismatch"
        );
    }
    Ok(())
}

fn member(directory: &Path, row: &Value) -> anyhow::Result<Vec<u8>> {
    let shard = row["shard"].as_str().context("source shard")?;
    ensure!(
        Path::new(shard).components().count() == 1,
        "source shard must be a filename"
    );
    let mut file = File::open(directory.join("records").join(shard))?;
    let offset = row["offset"].as_u64().context("source offset")?;
    let len = row["len"].as_u64().context("source length")?;
    ensure!(
        len <= 128 * 1024 * 1024
            && offset
                .checked_add(len)
                .is_some_and(|end| file.metadata().is_ok_and(|m| end <= m.len())),
        "source member outside bounds"
    );
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::new();
    GzDecoder::new(file.take(len))
        .take(512 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 512 * 1024 * 1024 && bytes.ends_with(b"\n"),
        "source record too large or truncated"
    );
    ensure!(
        row["record_sha256"] == crate::shadow::sha(&bytes),
        "source record digest mismatch"
    );
    Ok(bytes)
}

#[allow(
    clippy::too_many_lines,
    reason = "Ordered immutable source and first-boundary audit"
)]
fn case(directory: &Path, run: &Value, row: &Value) -> anyhow::Result<Value> {
    ensure!(
        row["source_run_sha256"] == crate::shadow::sha(&fs::read(directory.join("run.json"))?),
        "source run digest mismatch"
    );
    let encoder = u32::try_from(
        run["model_identity"]["encoder_version"]
            .as_u64()
            .context("source encoder")?,
    )?;
    let learner = usize::try_from(row["focal_seat"].as_u64().context("source learner seat")?)?;
    let roles = row["actor_roles"]
        .as_array()
        .context("source actor roles")?;
    ensure!(
        roles
            .iter()
            .filter(|role| role["role"] == "learner")
            .count()
            == 1
            && roles
                .iter()
                .any(|role| role["role"] == "learner" && role["seat"] == learner),
        "source learner role mismatch"
    );
    let sidecar: Vec<RolloutStepSidecar4> = serde_json::from_value(row["rollout_sidecar"].clone())?;
    let bytes = member(directory, row)?;
    let mut offset = 0;
    let mut seat_inputs = 0;
    let mut boundary = None;
    let mut end_seen = false;
    for raw in bytes.split_inclusive(|&byte| byte == b'\n') {
        let line: Line = serde_json::from_slice(raw)?;
        match line {
            Line::Input {
                n,
                seat,
                by,
                action,
                ..
            } => {
                ensure!(by == Source::Seat, "audit requires controlled neural seats");
                let sample = sidecar.get(seat_inputs).context("source missing sidecar")?;
                if sample.provenance.source == SourceKind4::House {
                    boundary = Some((offset, n, seat, action, seat_inputs));
                    break;
                }
                seat_inputs += 1;
            }
            Line::End { .. } => {
                end_seen = true;
                break;
            }
            _ => {}
        }
        offset += raw.len();
    }
    let source = json!({"directory":directory,"i":row["i"],"record_sha256":row["record_sha256"],
        "learner_seat":learner,"actor_roles":roles,
        "original_outcome":row["outcome"],"original_sidecar_count":sidecar.len(),
        "original_sidecars_sha256":crate::shadow::sha(&serde_json::to_vec(&row["rollout_sidecar"])?)});
    if let Some((offset, input_n, seat, proposal, count)) = boundary {
        let prefix = &bytes[..offset];
        let (session, _, last_n, bindings) =
            super::replay(prefix, &sidecar[..count], None, encoder, 0)?;
        let player = PlayerId::new(seat);
        let hash = super::boundary_view_hash(&session, player, encoder)?;
        let diagnostic = row["fallback_events"]
            .as_array()
            .and_then(|events| {
                events.iter().find(|event| {
                    event["accepted"] == true
                        && event["seat"] == seat
                        && (event["record_input_n"] == input_n
                            || event["record_byte_offset"] == offset)
                })
            })
            .context("first fallback lacks exact original diagnostic binding")?;
        ensure!(
            diagnostic["action"] == serde_json::to_value(&proposal)?,
            "original fallback action mismatch"
        );
        let reason = if diagnostic["reason"] == "outer_repeated_view" {
            "outer_repeated_view"
        } else if diagnostic["diagnostic"]["reason"] == "repeated_view" {
            "internal_repeated_view"
        } else {
            "other_fallback"
        };
        let result = super::stop(
            prefix,
            &sidecar[..count],
            &bindings,
            encoder,
            learner,
            player,
            reason != "other_fallback",
            reason,
            hash,
            Some(&proposal),
            session.snapshot_hash(),
        )?;
        Ok(
            json!({"source":source,"audit":"stopped_prefix_replayed","first_house_input_n":input_n,
            "last_included_input_n":last_n,"included_source_sidecars_sha256":crate::shadow::sha(&serde_json::to_vec(&row["rollout_sidecar"].as_array().context("source sidecars")?[..count])?),
            "original_sample_bindings_available":false,"reconstructed_binding_scope":"engine/action/provenance diagnostics only; no numerical policy LP admission",
            "boundary_diagnostic":result,"natural_house_completed_outcome_used_as_target":false}),
        )
    } else {
        ensure!(seat_inputs == sidecar.len(), "source has unused sidecars");
        let (session, _, last_n, _) = super::replay(&bytes[..offset], &sidecar, None, encoder, 0)?;
        ensure!(
            end_seen && matches!(session.pending(), Pending::GameOver(_)),
            "control lacks a natural engine terminal"
        );
        Ok(
            json!({"source":source,"audit":"natural_control_replayed","inputs":last_n,
            "all_canonical_snapshot_hashes_match":true,"training_data_created":false}),
        )
    }
}

pub(crate) fn run() -> anyhow::Result<()> {
    let args = Args::parse_from(std::env::args_os().skip(1));
    let wanted: BTreeSet<_> = args.cases.iter().copied().collect();
    ensure!(
        wanted.len() == args.cases.len() && !wanted.is_empty(),
        "duplicate or empty audit denominator"
    );
    ensure!(!args.out.exists(), "audit output already exists");
    let reference = args
        .input_seals
        .as_ref()
        .zip(args.reference_report.as_ref())
        .map(|(seals, report)| {
            let seal_bytes = fs::read(seals)?;
            let report_bytes = fs::read(report)?;
            Ok::<_, anyhow::Error>((
                serde_json::from_slice::<Value>(&seal_bytes)?,
                serde_json::from_slice::<Value>(&report_bytes)?,
                json!({"input_seals":seals,"input_seals_sha256":crate::shadow::sha(&seal_bytes),
                "report":report,"report_sha256":crate::shadow::sha(&report_bytes)}),
            ))
        })
        .transpose()?;
    let mut found = std::collections::BTreeMap::new();
    let mut inputs = Vec::new();
    for directory in &args.runs {
        let run_bytes = fs::read(directory.join("run.json"))?;
        let run: Value = serde_json::from_slice(&run_bytes)?;
        let index = fs::read(directory.join("games.jsonl"))?;
        inputs.push(json!({"directory":directory,"run_sha256":crate::shadow::sha(&run_bytes),"games_sha256":crate::shadow::sha(&index)}));
        for line in index
            .split(|&byte| byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let row: Value = serde_json::from_slice(line)?;
            let index = row["i"].as_u64().context("source case index")?;
            if wanted.contains(&index) {
                ensure!(!found.contains_key(&index), "duplicate source case {index}");
                let audited = || {
                    if let Some((seals, report, _)) = &reference {
                        references(directory, &row, seals, report)?;
                    }
                    case(directory, &run, &row)
                };
                let report = match audited() {
                    Ok(value) => value,
                    Err(error) => json!({"i":index,"audit":"failed","error":format!("{error:#}")}),
                };
                found.insert(index, report);
            }
        }
    }
    for index in &wanted {
        found
            .entry(*index)
            .or_insert_with(|| json!({"i":index,"audit":"missing"}));
    }
    let all_replayed = found
        .values()
        .all(|row| row["audit"] != "failed" && row["audit"] != "missing");
    let output = json!({"schema":"failure_terminal_gate_audit_v1","admissible_training_data":false,
        "purpose":"fixed known-trigger development diagnostics; original artifacts remain immutable",
        "expected_cases":args.cases,"all_cases_replayed":all_replayed,"inputs":inputs,
        "reference_verification":reference.as_ref().map(|(_,_,identity)|identity),
        "engine_revision":baylee_build::COMMIT,"engine_dirty":baylee_build::DIRTY,
        "cases":found.into_values().collect::<Vec<_>>()});
    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.out)?;
    file.write_all(&serde_json::to_vec_pretty(&output)?)?;
    file.sync_all()?;
    ensure!(
        all_replayed,
        "one or more fixed audit cases failed; inspect {}",
        args.out.display()
    );
    Ok(())
}
