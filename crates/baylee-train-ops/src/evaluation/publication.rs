//! Verified development preview selection. This never changes model weights.
// Counts come from resident evaluation arrays, below f64's exact integer range.
#![allow(clippy::cast_precision_loss)]
use super::{atomic_json, compare, read_json, read_verified};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
const PROTOCOL: &[&str] = &[
    "against",
    "baseline",
    "seats",
    "shape",
    "teams",
    "deck_split",
    "held_out_cards",
    "first_seed",
    "deal_offset",
    "deals",
    "policy_seed",
    "temperature",
    "thinking_steps",
];
/// Hash a file in bounded memory.
///
/// # Errors
/// Returns errors opening or reading the file.
pub fn digest(path: &Path) -> Result<String> {
    crate::files::sha256(path)
}
fn hex(bytes: &[u8]) -> String {
    super::hex(bytes)
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .with_context(|| format!("missing string {key}"))
}
fn model(value: &Value) -> Result<PathBuf> {
    absolute(Path::new(string(value, "model")?))
}
fn absolute(path: &Path) -> Result<PathBuf> {
    if let Ok(path) = path.canonicalize() {
        return Ok(path);
    }
    let path = std::path::absolute(path)?;
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            component => result.push(component.as_os_str()),
        }
    }
    Ok(result)
}
fn inputs(job: &Value) -> Result<&serde_json::Map<String, Value>> {
    job["inputs"].as_object().context("missing job inputs")
}
/// The digest a completed job recorded for the file at `path`, matched by
/// canonical path. The guard records an input as the job declared it
/// (relative to the job's `cwd`, or through a symlink such as macOS's `/var`),
/// while the model directory checked here is canonical; two recorded
/// spellings of one file with different digests are refused.
fn recorded<'a>(job: &'a Value, path: &Path) -> Result<Option<&'a Value>> {
    let wanted = path.canonicalize()?;
    let cwd = job["cwd"].as_str().map(Path::new);
    let mut found: Option<&Value> = None;
    for (declared, digest) in inputs(job)? {
        let declared = Path::new(declared);
        let declared = match cwd {
            Some(cwd) if declared.is_relative() => cwd.join(declared),
            _ => declared.to_owned(),
        };
        if declared.canonicalize().is_ok_and(|p| p == wanted) {
            ensure!(
                found.is_none_or(|seen| seen == digest),
                "A job recorded one input with two digests"
            );
            found = Some(digest);
        }
    }
    Ok(found)
}
fn direct(name: &str) -> bool {
    Path::new(name).file_name().is_some_and(|n| n == name)
}
/// Verify checkpoint, metadata, every declared graph and its loaded default alias.
///
/// # Errors
/// Rejects missing inputs, changed bytes, malformed metadata or graph aliases.
pub fn verified_model_inputs(value: &Value, job: &Value) -> Result<BTreeMap<String, String>> {
    let root = model(value)?;
    let mut recorded = BTreeMap::new();
    for (path, checksum) in inputs(job)? {
        let p = Path::new(path);
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if p.canonicalize()
            .ok()
            .and_then(|p| p.parent().map(Path::to_owned))
            .as_ref()
            == Some(&root)
            && (["net.pt", "model.onnx.json"].contains(&name)
                || p.extension().is_some_and(|e| e == "onnx"))
        {
            let checksum = checksum.as_str().context("invalid model digest")?;
            ensure!(
                digest(&root.join(name))? == checksum,
                "Evaluated model input changed: {name}"
            );
            recorded.insert(name.to_owned(), checksum.to_owned());
        }
    }
    ensure!(
        recorded.contains_key("net.pt") && recorded.contains_key("model.onnx.json"),
        "Evaluation omitted checkpoint or metadata input hashes"
    );
    let metadata = read_json(&root.join("model.onnx.json"))?;
    let graphs = metadata["graphs"]
        .as_object()
        .filter(|g| !g.is_empty())
        .context("Evaluation metadata has invalid graph filenames")?;
    for name in graphs.values() {
        let name = name.as_str().context("invalid graph filename")?;
        ensure!(
            direct(name),
            "Evaluation metadata has invalid graph filenames"
        );
        ensure!(
            recorded.contains_key(name),
            "Evaluation omitted declared graph input hashes"
        );
    }
    let k = value["thinking_steps"].as_u64().unwrap_or(1).to_string();
    let graph = graphs
        .get(&k)
        .and_then(Value::as_str)
        .context("missing thinking graph")?;
    ensure!(
        digest(&root.join("model.onnx"))? == recorded[graph],
        "Default graph differs from the evaluated thinking graph"
    );
    Ok(recorded)
}
fn checked(root: &Path, label: &str, shape: &str) -> Result<(Value, Value, Value, Value)> {
    let path = root.join(format!("evaluation/{label}-{shape}.json"));
    let job = root.join(format!("jobs/eval-{label}-{shape}/job.json"));
    let (v, p, h) = read_verified(&path, &job)?;
    ensure!(
        v["refused"].as_u64().unwrap_or(0) == 0 && v["net_answer_caps"].as_u64().unwrap_or(0) == 0,
        "Refusals or capped games require review before live selection"
    );
    let evidence = json!({"path":path,"sha256":h,"job":job,"quality":{"stalls":v["stalls"],"house_fallbacks":v["house_fallbacks"],"fallback_rate":v["fallback_rate"]}});
    Ok((v, p, evidence, read_json(&job)?))
}
fn label(progress: &Value, selected: &Path) -> Result<String> {
    for row in progress["rounds"].as_array().into_iter().flatten() {
        if model(row)? == selected {
            return Ok(format!(
                "round-{:02}",
                row["round"].as_u64().context("missing round")?
            ));
        }
    }
    Ok("initial".to_owned())
}
/// Construct a preview manifest from verified paired development evaluations.
///
/// # Errors
/// Rejects incomplete or incompatible evaluations and changed model artifacts.
#[allow(clippy::too_many_lines)] // Model evidence and seals form one transaction.
pub fn manifest(root: &Path) -> Result<Value> {
    let root = root.canonicalize()?;
    let progress = read_json(&root.join("progress.json"))?;
    let selected = Path::new(string(&progress, "best_development_candidate")?).canonicalize()?;
    let label = label(&progress, &selected)?;
    let mut benchmark = BTreeMap::new();
    let mut evidence = BTreeMap::new();
    let mut scores = Vec::new();
    let mut jobs = Vec::new();
    for shape in ["duels", "commander"] {
        let (v, p, source, j) = checked(&root, &label, shape)?;
        let (r, rp, _, rj) = checked(&root, "initial", shape)?;
        verified_model_inputs(&v, &j)?;
        verified_model_inputs(&r, &rj)?;
        compare(&v, &r, &p, &rp, 128, false)?;
        ensure!(
            model(&v)? == selected,
            "Selected checkpoint differs from evaluation"
        );
        let rows = v["deal_details"].as_array().context("missing deals")?;
        scores.push(
            rows.iter()
                .map(|r| super::number(r, "net_utility"))
                .collect::<Result<Vec<_>>>()?
                .iter()
                .sum::<f64>()
                / rows.len() as f64,
        );
        let mut sorted = rows.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|r| super::case_key(r, true).unwrap_or_default());
        let deals = sorted
            .iter()
            .map(|r| {
                json!([
                    r["table"],
                    r["seed"],
                    r["decks"],
                    r["focal_seat"],
                    r["base_utility"],
                    r["base_capped"]
                ])
            })
            .collect::<Vec<_>>();
        let protocol: serde_json::Map<_, _> = PROTOCOL
            .iter()
            .map(|k| ((*k).to_owned(), v[*k].clone()))
            .collect();
        benchmark.insert(
            shape,
            json!({"provenance":p,"protocol":protocol,"deals":deals}),
        );
        evidence.insert(shape, source);
        jobs.push(j);
    }
    let score = scores.iter().sum::<f64>() / scores.len() as f64;
    ensure!(
        score.is_finite()
            && (score - super::number(&progress, "best_development_score")?).abs() <= 1e-12,
        "Controller rank disagrees with measured utility"
    );
    let metadata = read_json(&selected.join("model.onnx.json"))?;
    ensure!(
        metadata
            .get("critic_semantics")
            .is_none_or(|marker| { *marker == json!({"kind":"mtg_side_outcome_utility_v1"}) })
            && metadata.get("research_ppo_contract").is_none()
            && metadata.get("research_ppo_progress").is_none(),
        "Live selection requires MTG outcome critic semantics; research or unknown utility stays offline"
    );
    let parity = super::number(&metadata, "parity_max_abs_diff")?;
    ensure!(
        (0.0..=0.001).contains(&parity) && metadata["checked_frames"].as_u64().unwrap_or(0) > 0,
        "No successful numerical export parity check"
    );
    let graphs = metadata["graphs"].as_object().context("missing graphs")?;
    let mut names = BTreeSet::from([
        "net.pt".to_owned(),
        "model.onnx.json".into(),
        "model.onnx".into(),
        "card_structure.json".into(),
        "model.k0.onnx".into(),
        "model.k1.onnx".into(),
    ]);
    for name in graphs.values() {
        names.insert(name.as_str().context("invalid graph filename")?.to_owned());
    }
    let mut hashes = BTreeMap::new();
    for name in names {
        ensure!(
            direct(&name),
            "Model files must be direct children of the frozen directory"
        );
        hashes.insert(name.clone(), digest(&selected.join(name))?);
    }
    let default = graphs["1"].as_str().context("missing K1 graph")?;
    ensure!(
        hashes["model.onnx"] == hashes[default],
        "Default graph is not the evaluated K=1 graph"
    );
    for job in jobs {
        for (name, hash) in &hashes {
            if ["model.onnx", "card_structure.json"].contains(&name.as_str()) {
                continue;
            }
            ensure!(
                recorded(&job, &selected.join(name))?.and_then(Value::as_str)
                    == Some(hash.as_str()),
                "Checkpoint changed since its completed comparison: {name}"
            );
        }
    }
    // Preserve the Python benchmark digest representation (sorted keys and
    // default comma/colon spacing), so existing preview manifests remain usable.
    let benchmark = serde_json::to_value(benchmark)?;
    let encoded = python_json(&benchmark)?;
    Ok(
        json!({"schema_version":1,"model":selected,"label":label.replace("round-","v4-R"),"score":score,"benchmark_id":hex(&Sha256::digest(encoded.as_bytes())),"sha256":hashes,"evidence":evidence,"source_selection":root.join("progress.json"),"scope":"development leader for local playtests; not a strength promotion"}),
    )
}
fn python_json(value: &Value) -> Result<String> {
    Ok(match value {
        Value::Array(a) => format!(
            "[{}]",
            a.iter()
                .map(python_json)
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        ),
        Value::Object(m) => {
            let sorted: BTreeMap<_, _> = m.iter().collect();
            format!(
                "{{{}}}",
                sorted
                    .iter()
                    .map(|(k, v)| Ok(format!("{}: {}", python_string(k)?, python_json(v)?)))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ")
            )
        }
        Value::String(s) => python_string(s)?,
        _ => serde_json::to_string(value)?,
    })
}
fn python_string(value: &str) -> Result<String> {
    use std::fmt::Write;
    let json = serde_json::to_string(value)?;
    let mut escaped = String::with_capacity(json.len());
    for c in json.chars() {
        if c.is_ascii() {
            escaped.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                write!(escaped, "\\u{unit:04x}")?;
            }
        }
    }
    Ok(escaped)
}
/// Assess completed frozen confirmation evidence; incomplete evidence stays pending.
///
/// # Errors
/// Rejects changed selection, graph hashes or confirmation identity and protocol.
pub fn confirmation_assessment(root: &Path, selected: &Path) -> Result<Value> {
    let root = root.canonicalize()?;
    let selected = absolute(selected)?;
    let selection = root.join("confirmation-selection.json");
    if !selection.exists() {
        return Ok(json!({"status":"pending","veto":false}));
    }
    let selection = read_json(&selection)?;
    if model(&selection)? != selected {
        return Ok(json!({"status":"not_this_candidate","veto":false}));
    }
    for shape in ["duels", "commander"] {
        for label in ["selected", "initial"] {
            let path = root.join(format!("jobs/eval-confirmation-{label}-{shape}/job.json"));
            if !path.exists() || read_json(&path)?["status"] != "complete" {
                return Ok(json!({"status":"pending","veto":false}));
            }
        }
    }
    ensure!(
        selection["selected_before_confirmation"] == true,
        "Confirmation candidate was not fixed before confirmation"
    );
    ensure!(
        digest(&selected.join("net.pt"))? == selection["model_sha256"],
        "Preselected confirmation checkpoint changed"
    );
    let progress = read_json(&root.join("progress.json"))?;
    let development_label = label(&progress, &selected)?;
    let mut differences = Vec::new();
    let mut evidence = BTreeMap::new();
    for shape in ["duels", "commander"] {
        let (v, p, candidate, j) = checked(&root, "confirmation-selected", shape)?;
        let (r, rp, baseline, rj) = checked(&root, "confirmation-initial", shape)?;
        let (initial, _, _, ij) = checked(&root, "initial", shape)?;
        let (development, _, _, dj) = checked(&root, &development_label, shape)?;
        ensure!(
            model(&r)? == model(&initial)?,
            "Confirmation reference differs from the original initial model"
        );
        ensure!(
            model(&development)? == selected,
            "Confirmation candidate differs from its development model"
        );
        for (v, j) in [(&v, &j), (&r, &rj), (&initial, &ij), (&development, &dj)] {
            verified_model_inputs(v, j)?;
        }
        let checkpoint = selected.join("net.pt");
        ensure!(
            model(&v)? == selected && recorded(&j, &checkpoint)? == selection.get("model_sha256"),
            "Confirmation evaluated a different candidate checkpoint"
        );
        let paired = compare(&v, &r, &p, &rp, 128, false)?;
        let delta = super::number(&paired["paired_uncertainty"], "paired_utility_difference")?;
        differences.push(delta);
        evidence.insert(shape,json!({"candidate":candidate,"reference":baseline,"paired_difference":delta,"outcomes":paired["outcomes"]}));
    }
    let mean = differences.iter().sum::<f64>() / differences.len() as f64;
    Ok(
        json!({"status":if mean<=0.0 {"nonpositive_confirmation"} else {"positive_point_estimate"},"veto":mean<=0.0,"mean_paired_utility_difference":mean,"selected_model_sha256":selection["model_sha256"],"evidence":evidence,
        "rule":"Conservative operational veto when completed equal-format confirmation mean is nonpositive; no significance threshold or established-strength claim.",
        "scope":"Independent seeds on development deck families; experimental preview only. No automatic rollback or interruption of a running game."}),
    )
}
/// Select a verified development preview, preserving pins vetoed by confirmation.
///
/// # Errors
/// Returns validation, integrity or atomic-publication failures.
pub fn publish(root: &Path, output: &Path) -> Result<bool> {
    let progress = read_json(&root.join("progress.json"))?;
    let selected = Path::new(string(&progress, "best_development_candidate")?).canonicalize()?;
    let current = if output.exists() {
        Some(read_json(output)?)
    } else {
        None
    };
    let confirmation = confirmation_assessment(root, &selected)?;
    let same = current
        .as_ref()
        .is_some_and(|v| model(v).ok().as_ref() == Some(&selected));
    let current_confirmation = if same {
        confirmation.clone()
    } else if let Some(c) = &current {
        confirmation_assessment(root, &model(c)?)?
    } else {
        Value::Null
    };
    let mut report = json!({"checked_unix":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64(),"candidate":selected,"current_model":current.as_ref().map(|c|c["model"].clone()),"confirmation":confirmation,"current_confirmation":current_confirmation,"review_required":confirmation["veto"]==true || current_confirmation["veto"]==true,"updated":false,"selection_unchanged":true,"scope":"experimental local preview; not strength promotion"});
    let reason;
    let mut updated = false;
    if confirmation["veto"] == true {
        reason = if same {
            "confirmation_veto_existing_selection_requires_review"
        } else {
            "confirmation_veto_candidate_not_published"
        };
    } else if same {
        reason = if report["review_required"] == true {
            "unchanged_current_selection_requires_review"
        } else {
            "unchanged"
        };
    } else {
        let mut candidate = manifest(root)?;
        if let Some(c) = &current {
            ensure!(
                c["benchmark_id"] == candidate["benchmark_id"],
                "Different benchmark; requires a comparable baseline before switching"
            );
        }
        if current.as_ref().is_some_and(|c| {
            c["score"].as_f64().unwrap_or(f64::INFINITY)
                >= candidate["score"].as_f64().unwrap_or(f64::NEG_INFINITY)
        }) {
            reason = "not_a_higher_development_score";
        } else {
            candidate["confirmation"] = confirmation;
            atomic_json(output, &candidate)?;
            report["updated"] = json!(true);
            report["selection_unchanged"] = json!(false);
            report["review_required"] = json!(false);
            updated = true;
            reason = "published_experimental_development_preview";
        }
    }
    report["decision"] = json!(reason);
    let name = output
        .file_name()
        .context("invalid output filename")?
        .to_string_lossy();
    atomic_json(
        &output.with_file_name(format!("{name}.confirmation-status.json")),
        &report,
    )?;
    Ok(updated)
}
