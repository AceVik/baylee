use super::indices;
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::Path;
const MIB: u64 = 1 << 20;
fn fraction(count: usize, total: usize) -> Result<Value> {
    if total == 0 {
        return Ok(Value::Null);
    }
    Ok(json!(
        f64::from(u32::try_from(count)?) / f64::from(u32::try_from(total)?)
    ))
}
pub(super) fn finish(value: &Value) -> Result<Value> {
    let plan = &value["plan"];
    let metadata = &value["metadata"];
    let out = Path::new(plan["out"].as_str().context("report output")?);
    ensure!(!out.exists(), "preserving existing batch audit report");
    for (path, digest) in plan["input_seals"]
        .as_object()
        .context("frozen audit source seals")?
    {
        ensure!(
            files::sha256(Path::new(path))? == digest.as_str().context("source SHA256")?,
            "batch audit input changed"
        );
    }
    let length = metadata["epoch_batches"]
        .as_u64()
        .context("epoch batches")?;
    let samples = plan["samples"].as_u64().context("sample count")?;
    let records: Vec<Value> = value["records"]
        .as_array()
        .context("batch measurements")?
        .iter()
        .map(super::measurement::normalize)
        .collect::<Result<_>>()?;
    let observed: Vec<u64> = records
        .iter()
        .map(|r| r["batch_index"].as_u64().context("batch index"))
        .collect::<Result<_>>()?;
    ensure!(
        observed == indices(length, samples)?,
        "measured indices differ from native deterministic sampling plan"
    );
    let measured: Vec<_> = records.iter().filter(|r| r["skipped"] == false).collect();
    let mut ordered: Vec<(u64, u64)> = measured
        .iter()
        .map(|r| {
            Ok((
                r["raw_batch_bytes"].as_u64().context("raw batch bytes")?,
                r["batch_index"].as_u64().context("index")?,
            ))
        })
        .collect::<Result<_>>()?;
    ordered.sort_by_key(|&(size, _)| size);
    let pick = |numerator: usize, denominator: usize| -> Value {
        if ordered.is_empty() {
            Value::Null
        } else {
            json!(ordered[(ordered.len() - 1) * numerator / denominator].1)
        }
    };
    let sizes: Vec<u64> = ordered.iter().map(|&(size, _)| size).collect();
    let mut report = json!({"schema_version":2,"sampling_version":"integer_even_v1","tool_sha256":plan["tool_sha256"],"tensor_worker_sha256":plan["worker_sha256"],"job":plan["job"],"job_sha256":plan["job_sha256"],"runtime":plan["runtime"],"frozen_code_sha256":plan["frozen_code_sha256"],"seed":plan["seed"],"epoch":0,"batch_size":plan["batch_size"],"unroll":plan["unroll"],"epoch_batches":length,"samples":records.len(),"measured":measured.len(),"skipped":records.len()-measured.len(),"budget_mib":plan["budget_mib"],"elapsed_seconds":value["elapsed_seconds"],"fractions":{"le128MiB":fraction(sizes.iter().filter(|s|**s<=128*MIB).count(),sizes.len())?,"gt128MiB":fraction(sizes.iter().filter(|s|**s>128*MIB).count(),sizes.len())?,"gt256MiB":fraction(sizes.iter().filter(|s|**s>256*MIB).count(),sizes.len())?},"benchmark_windows":{"first":observed[0],"median":pick(1,2),"high_p95":pick(95,100),"argmax":pick(1,1)},"batches":records,"limitations":["Deterministic evenly-spaced sample; sampled maximum is NOT global maximum.","Shape analysis only; no model forward, training or GPU; K0 observations retained.","Mid-game windows with H0 are NOT a chronologically correct quality evaluation.","Budget bounds estimated tensor allocations, not fixed index/catalog/Python RSS; dummy masks count as allocated entries.","No full payload hash/CRC scan; frozen loader cache behavior remains its own contract; unknown formats stay unknown."]});
    for key in [
        "dataset",
        "data_fingerprint",
        "namespace",
        "metadata_inputs",
        "sampler_version",
    ] {
        report[key] = metadata
            .get(key)
            .context("missing tensor dataset identity")?
            .clone();
    }
    files::atomic_json(out, &report)?;
    Ok(json!({"complete":true,"out":out,"samples":records.len()}))
}
