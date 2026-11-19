//! `hw_bench` — times one exported graph on one execution provider and checks
//! its answers against the FP32 CPU net, on rows `hw_capture4` recorded from
//! real games (`baylee_driver::rowdump`).
//!
//! Three modes:
//!
//! - `time`: forwards per second and call latency for every batch size and
//!   seat count asked; each seat is a thread with its own session (a worker
//!   slot), rows are shuffled with a fixed seed and stacked like
//!   `netplay4::run_rows` (padded to the call's longest axes, or to shape
//!   buckets with `--buckets`). Temperatures and the APU's power are sampled
//!   from `hwmon` while it runs.
//! - `ref`: the outputs of every row, one at a time, written as a row dump
//!   (run it with `--ep cpu` on the FP32 graph).
//! - `parity`: the outputs compared with such a reference: top-1 agreement
//!   per decision kind, largest logit, value and memory differences.
//!   `--chained` feeds each seat's memory from this provider's own previous
//!   observation, so its errors compound as they would in play.
//!
//! ```text
//! hw_bench --mode time --ep webgpu --model fp16 --rows rows.bin \
//!     --batches 1,4,16,64 --seats 1,2,4 --out time.json
//! ```

#![allow(
    clippy::cast_precision_loss,
    reason = "counts and timings become statistics; none nears 2^52"
)]

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail, ensure};
use baylee_driver::rowdump::{DumpRow, DumpTensor, RowReader, RowWriter, TensorData};
use baylee_driver::{AcceleratorKind, GraphPrecision, HardwareConfig};
use ort::session::{Session, SessionInputValue};
use ort::value::{DynValue, Tensor};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Time,
    Ref,
    Parity,
    Stats,
    Io,
    Check,
    Plan,
}

/// Which rows a run takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Decisions,
    Observations,
}

#[derive(Debug)]
struct Args {
    mode: Mode,
    model: PathBuf,
    rows: Vec<PathBuf>,
    ep: AcceleratorKind,
    intra: usize,
    kind: Kind,
    batches: Vec<usize>,
    seats: Vec<usize>,
    seconds: f64,
    warmup: usize,
    buckets: Buckets,
    reference: Option<PathBuf>,
    out: Option<PathBuf>,
    limit: usize,
    chained: bool,
    parity_batch: usize,
    label: String,
    seed: u64,
    /// ONNX Runtime profile file prefix: which provider ran each node.
    profile: Option<String>,
    /// Free dimensions fixed for the session (`--dims batch=1,entities=32`).
    dims: Vec<(String, i64)>,
    /// Session configuration entries (`--config key=value,...`).
    configs: Vec<(String, String)>,
    /// Where ONNX Runtime writes the graph it optimized (a path prefix).
    save_optimized: Option<String>,
    /// Stack rows of like size (`--sort`) instead of shuffled ones.
    sort: bool,
    /// Unix time (seconds) at which timing starts, after warm-up.
    start_at: Option<f64>,
    /// Parity over decision rows only (`--decisions-only`).
    decisions_only: bool,
    /// `--mode plan`: bucket sets to compare, `E,M,C;E,M,C|...`.
    plans: String,
    /// `--mode plan`: cost model, call ms, row ms, token ms.
    cost: (f64, f64, f64),
}

fn list(raw: &str) -> anyhow::Result<Vec<usize>> {
    raw.split(',')
        .map(|s| s.trim().parse::<usize>().context("a list of numbers"))
        .collect()
}

impl Args {
    #[allow(clippy::too_many_lines, reason = "one flat table of flags")]
    fn parse() -> anyhow::Result<Self> {
        let mut a = Self {
            mode: Mode::Time,
            model: PathBuf::new(),
            rows: Vec::new(),
            ep: AcceleratorKind::Cpu,
            intra: 0,
            kind: Kind::Decisions,
            batches: vec![1, 4, 16, 64],
            seats: vec![1],
            seconds: 15.0,
            warmup: 3,
            buckets: Buckets::None,
            reference: None,
            out: None,
            limit: usize::MAX,
            chained: false,
            parity_batch: 1,
            label: String::new(),
            seed: 0x5eed_1234,
            profile: None,
            dims: Vec::new(),
            configs: Vec::new(),
            save_optimized: None,
            sort: false,
            start_at: None,
            decisions_only: false,
            plans: String::new(),
            cost: (4.5, 0.32, 0.0174),
        };
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            if flag == "--chained" {
                a.chained = true;
                continue;
            }
            if flag == "--sort" {
                a.sort = true;
                continue;
            }
            if flag == "--decisions-only" {
                a.decisions_only = true;
                continue;
            }
            let value = it.next().with_context(|| format!("{flag} takes a value"))?;
            match flag.as_str() {
                "--mode" => {
                    a.mode = match value.as_str() {
                        "time" => Mode::Time,
                        "ref" => Mode::Ref,
                        "parity" => Mode::Parity,
                        "stats" => Mode::Stats,
                        "io" => Mode::Io,
                        "check" => Mode::Check,
                        "plan" => Mode::Plan,
                        other => bail!("unknown mode {other}"),
                    }
                }
                "--model" => a.model = value.into(),
                "--rows" => a.rows.push(value.into()),
                "--ep" => a.ep = AcceleratorKind::from_str(&value)?,
                "--intra" => a.intra = value.parse()?,
                "--kind" => {
                    a.kind = match value.as_str() {
                        "decisions" => Kind::Decisions,
                        "observations" => Kind::Observations,
                        other => bail!("unknown kind {other}"),
                    }
                }
                "--batches" => a.batches = list(&value)?,
                "--seats" => a.seats = list(&value)?,
                "--seconds" => a.seconds = value.parse()?,
                "--warmup" => a.warmup = value.parse()?,
                "--buckets" => a.buckets = Buckets::parse(&value)?,
                "--ref" => a.reference = Some(value.into()),
                "--out" => a.out = Some(value.into()),
                "--limit" => a.limit = value.parse()?,
                "--parity-batch" => a.parity_batch = value.parse()?,
                "--label" => a.label = value,
                "--seed" => a.seed = value.parse()?,
                "--start-at" => a.start_at = Some(value.parse()?),
                "--plans" => a.plans = value,
                "--cost" => {
                    let v: Vec<f64> = value.split(',').map(str::parse).collect::<Result<_, _>>()?;
                    ensure!(v.len() == 3, "--cost call_ms,row_ms,token_ms");
                    a.cost = (v[0], v[1], v[2]);
                }
                "--profile" => a.profile = Some(value),
                "--dims" => {
                    a.dims = pairs(&value)?
                        .into_iter()
                        .map(|(k, v)| Ok((k, v.parse()?)))
                        .collect::<anyhow::Result<_>>()?;
                }
                "--config" => a.configs = pairs(&value)?,
                "--save-optimized" => a.save_optimized = Some(value),
                other => bail!("unknown flag {other}"),
            }
        }
        let fixed = fixed_extents(&a.dims);
        if !fixed.is_empty() && matches!(a.buckets, Buckets::None) {
            a.buckets = Buckets::Steps(fixed.into_iter().map(|(k, n)| (k, vec![n])).collect());
        }
        ensure!(!a.rows.is_empty(), "--rows names at least one row dump");
        ensure!(
            a.model.join("model.onnx.json").is_file(),
            "--model names an exported model directory"
        );
        Ok(a)
    }
}

/// The variable axes of the graph's inputs (after the batch axis).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Axis {
    Fixed,
    Entities,
    Edges,
    Cands,
    Pairs,
    EntityRules,
    CandRules,
}

const VARIABLE_AXES: [Axis; 6] = [
    Axis::Entities,
    Axis::Edges,
    Axis::Cands,
    Axis::Pairs,
    Axis::EntityRules,
    Axis::CandRules,
];

/// What each axis of an input is, after the batch axis.
fn roles(name: &str) -> &'static [Axis] {
    use Axis::{CandRules, Cands, Edges, Entities, EntityRules, Fixed, Pairs};
    match name {
        "entity_feat" | "entity_dsl" => &[Entities, Fixed],
        "entity_mask" => &[Entities],
        "edge_index" => &[Edges, Fixed],
        "edge_type" | "edge_mask" => &[Edges],
        "cand_feat" | "cand_dsl" => &[Cands, Fixed],
        "cand_entity_idx" | "cand_seat_idx" | "cand_mask" => &[Cands],
        "pair_index" => &[Pairs, Fixed],
        "pair_mask" => &[Pairs],
        "entity_rules" => &[Entities, EntityRules, Fixed],
        "cand_rules" => &[Cands, CandRules, Fixed],
        _ => &[],
    }
}

/// The padding an input carries, as `netplay4::run_rows` pads.
fn fill_i64(name: &str) -> i64 {
    if name == "cand_entity_idx" || name == "cand_seat_idx" {
        -1
    } else {
        0
    }
}

/// How a call's variable axes are rounded up, so a compiling provider sees
/// few shapes.
#[derive(Clone, Debug)]
enum Buckets {
    /// Every call at its own longest axes.
    None,
    /// The next power of two.
    Pow2,
    /// The smallest listed size that fits, per axis; larger values round
    /// up to a multiple of the last.
    Steps(BTreeMap<Axis, Vec<usize>>),
}

impl Buckets {
    /// `none`, `pow2`, or `e=32,64,128;m=...;c=...;p=...;er=...;cr=...`.
    fn parse(raw: &str) -> anyhow::Result<Self> {
        match raw {
            "none" => return Ok(Self::None),
            "pow2" => return Ok(Self::Pow2),
            _ => {}
        }
        let mut steps = BTreeMap::new();
        for part in raw.split(';') {
            let (axis, sizes) = part.split_once('=').context("axis=sizes")?;
            let axis = match axis.trim() {
                "e" => Axis::Entities,
                "m" => Axis::Edges,
                "c" => Axis::Cands,
                "p" => Axis::Pairs,
                "er" => Axis::EntityRules,
                "cr" => Axis::CandRules,
                other => bail!("unknown axis {other}"),
            };
            let mut sizes = list(sizes)?;
            sizes.sort_unstable();
            ensure!(sizes.first().is_some_and(|&s| s > 0), "bucket sizes > 0");
            steps.insert(axis, sizes);
        }
        Ok(Self::Steps(steps))
    }

    fn round(&self, axis: Axis, n: usize) -> usize {
        match self {
            Self::None => n,
            Self::Pow2 => n.next_power_of_two(),
            Self::Steps(steps) => match steps.get(&axis) {
                None => n,
                Some(sizes) => sizes.iter().copied().find(|&s| s >= n).unwrap_or_else(|| {
                    let last = *sizes.last().unwrap_or(&1);
                    n.div_ceil(last) * last
                }),
            },
        }
    }

    fn describe(&self) -> String {
        match self {
            Self::None => "none".to_owned(),
            Self::Pow2 => "pow2".to_owned(),
            Self::Steps(steps) => format!("{steps:?}"),
        }
    }
}

/// Copies `src` (`sdims`) into the leading corner of `dst` (`ddims`).
fn pad_copy<T: Copy>(src: &[T], sdims: &[usize], dst: &mut [T], ddims: &[usize]) {
    match sdims.len() {
        0 => dst[0] = src[0],
        1 => dst[..sdims[0]].copy_from_slice(&src[..sdims[0]]),
        _ => {
            let s: usize = sdims[1..].iter().product();
            let d: usize = ddims[1..].iter().product();
            for i in 0..sdims[0] {
                pad_copy(
                    &src[i * s..(i + 1) * s],
                    &sdims[1..],
                    &mut dst[i * d..(i + 1) * d],
                    &ddims[1..],
                );
            }
        }
    }
}

/// A row's own extent on `axis`.
fn own(row: &DumpRow, axis: Axis) -> usize {
    for t in &row.tensors {
        if let Some(j) = roles(&t.name).iter().position(|&r| r == axis) {
            return t.dims[j + 1];
        }
    }
    0
}

/// One call's inputs, ready to run.
struct Batch {
    inputs: Vec<(String, DynValue)>,
    /// The call's extents per variable axis, in [`VARIABLE_AXES`] order.
    shape: Vec<usize>,
    /// Entity plus candidate tokens the rows really have.
    real_tokens: usize,
    /// Entity plus candidate tokens the call computes.
    padded_tokens: usize,
}

/// Stacks `rows` into one call, padded to their longest axes rounded by
/// `buckets`.
fn assemble(rows: &[&DumpRow], buckets: &Buckets) -> anyhow::Result<Batch> {
    let first = rows.first().context("a call has rows")?;
    let mut target = HashMap::new();
    for axis in VARIABLE_AXES {
        let longest = rows.iter().map(|r| own(r, axis)).max().unwrap_or(0);
        if longest > 0 {
            target.insert(axis, buckets.round(axis, longest));
        }
    }
    let b = rows.len();
    let mut inputs = Vec::with_capacity(first.tensors.len());
    for (ti, t) in first.tensors.iter().enumerate() {
        let dims = call_dims(t, b, &target);
        let sources = row_tensors(rows, ti, t, &dims)?;
        let value = match &t.data {
            TensorData::F32(_) => {
                let out = stacked(&sources, &dims, 0.0_f32, &t.name, |d| match d {
                    TensorData::F32(v) => Some(v),
                    _ => None,
                })?;
                Tensor::from_array((dims, out))?.into_dyn()
            }
            TensorData::I64(_) => {
                let out = stacked(&sources, &dims, fill_i64(&t.name), &t.name, |d| match d {
                    TensorData::I64(v) => Some(v),
                    _ => None,
                })?;
                Tensor::from_array((dims, out))?.into_dyn()
            }
            TensorData::Bool(_) => {
                let out = stacked(&sources, &dims, false, &t.name, |d| match d {
                    TensorData::Bool(v) => Some(v),
                    _ => None,
                })?;
                Tensor::from_array((dims, out))?.into_dyn()
            }
        };
        inputs.push((t.name.clone(), value));
    }
    let real_tokens = rows
        .iter()
        .map(|r| own(r, Axis::Entities) + own(r, Axis::Cands))
        .sum();
    let padded_tokens = b
        * (target.get(&Axis::Entities).copied().unwrap_or(0)
            + target.get(&Axis::Cands).copied().unwrap_or(0));
    let shape = VARIABLE_AXES
        .iter()
        .map(|a| target.get(a).copied().unwrap_or(0))
        .collect();
    Ok(Batch {
        inputs,
        shape,
        real_tokens,
        padded_tokens,
    })
}

/// The call's dimensions for input `t`: `batch` rows, then each axis at the
/// input's fixed extent or the call's target for its variable axis.
fn call_dims(t: &DumpTensor, batch: usize, target: &HashMap<Axis, usize>) -> Vec<usize> {
    let r = roles(&t.name);
    std::iter::once(batch)
        .chain(t.dims.iter().enumerate().skip(1).map(|(j, &own)| {
            match r.get(j - 1).copied().unwrap_or(Axis::Fixed) {
                Axis::Fixed => own,
                role => target[&role],
            }
        }))
        .collect()
}

/// Input `ti` of every row, checked to be `t`'s input with fixed axes equal
/// to the call's and variable axes no longer than the call's.
fn row_tensors<'a>(
    rows: &[&'a DumpRow],
    ti: usize,
    t: &DumpTensor,
    dims: &[usize],
) -> anyhow::Result<Vec<&'a DumpTensor>> {
    let r = roles(&t.name);
    rows.iter()
        .map(|row| {
            let s = &row.tensors[ti];
            ensure!(s.name == t.name, "rows list their inputs in one order");
            for (j, (&have, &want)) in s.dims.iter().zip(dims).enumerate().skip(1) {
                let role = r.get(j - 1).copied().unwrap_or(Axis::Fixed);
                ensure!(
                    if role == Axis::Fixed {
                        have == want
                    } else {
                        have <= want
                    },
                    "{}: axis {j} is {have}, the call has {want}",
                    t.name
                );
            }
            Ok(s)
        })
        .collect()
}

/// The rows' values of one input stacked into a call of `dims`, each row
/// padded with `fill`; `values` picks the input's elements of type `T`.
fn stacked<T: Copy>(
    sources: &[&DumpTensor],
    dims: &[usize],
    fill: T,
    name: &str,
    values: impl Fn(&TensorData) -> Option<&Vec<T>>,
) -> anyhow::Result<Vec<T>> {
    let per: usize = dims[1..].iter().product();
    let mut out = vec![fill; sources.len() * per];
    for (i, s) in sources.iter().enumerate() {
        let Some(v) = values(&s.data) else {
            bail!("{name}: types")
        };
        pad_copy(
            v,
            &s.dims[1..],
            &mut out[i * per..(i + 1) * per],
            &dims[1..],
        );
    }
    Ok(out)
}

/// Runs one call; outputs per row at the row's own candidate and pair
/// counts (embeddings left out).
fn run(
    session: &mut Session,
    batch: &Batch,
    rows: &[&DumpRow],
) -> anyhow::Result<Vec<Vec<DumpTensor>>> {
    let inputs: Vec<(&str, SessionInputValue<'_>)> = batch
        .inputs
        .iter()
        .map(|(n, v)| (n.as_str(), SessionInputValue::from(v)))
        .collect();
    let outputs = session.run(inputs)?;
    let b = rows.len();
    let mut per_row: Vec<Vec<DumpTensor>> = vec![Vec::new(); b];
    for (name, value) in &outputs {
        if name.ends_with("_emb") {
            continue;
        }
        let Ok((shape, data)) = value.try_extract_tensor::<f32>() else {
            continue;
        };
        let dims: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
        ensure!(dims.first() == Some(&b), "{name}: batch axis {dims:?}");
        let per = data.len() / b;
        let axis = if name.starts_with("cand") && dims.len() == 2 {
            Some(Axis::Cands)
        } else if name.starts_with("pair") && dims.len() == 2 {
            Some(Axis::Pairs)
        } else {
            None
        };
        for (i, row) in rows.iter().enumerate() {
            let chunk = &data[i * per..(i + 1) * per];
            let (dims_i, v) = if let Some(axis) = axis {
                let n = own(row, axis);
                (vec![1, n], chunk[..n].to_vec())
            } else {
                let mut d = dims.clone();
                d[0] = 1;
                (d, chunk.to_vec())
            };
            per_row[i].push(DumpTensor::new(name, dims_i, TensorData::F32(v))?);
        }
    }
    Ok(per_row)
}

fn load_rows(paths: &[PathBuf]) -> anyhow::Result<Vec<DumpRow>> {
    let mut rows = Vec::new();
    for path in paths {
        let file = File::open(path).with_context(|| path.display().to_string())?;
        let mut reader = RowReader::new(BufReader::with_capacity(1 << 20, file))?;
        rows.extend(reader.read_all()?);
    }
    Ok(rows)
}

fn session(args: &Args, graph: u8, slots: usize) -> anyhow::Result<Session> {
    let hw = HardwareConfig {
        accelerator: args.ep,
        precision: GraphPrecision::Fp32,
        intra_threads: args.intra,
        graceful_cpu_fallback: false,
        ..HardwareConfig::default()
    };
    let path = args.model.join(format!("model.k{graph}.onnx"));
    let profile = args.profile.clone();
    let dims = args.dims.clone();
    let configs = args.configs.clone();
    let optimized = args.save_optimized.clone();
    #[allow(
        clippy::result_large_err,
        reason = "ort's builder error carries the builder back by design"
    )]
    baylee_driver::ort_env::build_session_with(&path, &hw, slots, |mut b| {
        if let Some(prefix) = profile {
            b = b.with_profiling(format!("{prefix}-k{graph}"))?;
        }
        for (name, size) in dims {
            b = b.with_dimension_override(name, size)?;
        }
        for (key, value) in configs {
            b = b.with_config_entry(key, value)?;
        }
        if let Some(path) = optimized {
            b = b.with_optimized_model_path(format!("{path}-k{graph}.onnx"))?;
        }
        Ok(b)
    })
}

/// The extents `--dims` fixes, by axis (the graph's own symbols).
fn fixed_extents(dims: &[(String, i64)]) -> BTreeMap<Axis, usize> {
    dims.iter()
        .filter_map(|(name, n)| {
            let axis = match name.as_str() {
                "E" => Axis::Entities,
                "M" => Axis::Edges,
                "C" => Axis::Cands,
                "P" => Axis::Pairs,
                "R" => Axis::EntityRules,
                "Q" => Axis::CandRules,
                _ => return None,
            };
            Some((axis, usize::try_from(*n).ok()?))
        })
        .collect()
}

/// `name=value,name=value` pairs.
fn pairs(raw: &str) -> anyhow::Result<Vec<(String, String)>> {
    raw.split(',')
        .map(|p| {
            let (k, v) = p.split_once('=').context("name=value")?;
            Ok((k.trim().to_owned(), v.trim().to_owned()))
        })
        .collect()
}

/// The graph's inputs and outputs: names, element types, shapes and the
/// names of their free dimensions (what `--dims` can fix).
fn print_io(args: &Args, graph: u8) -> anyhow::Result<()> {
    let s = session(args, graph, 1)?;
    for i in s.inputs() {
        println!("input  {} {:?}", i.name(), i.dtype());
    }
    for o in s.outputs() {
        println!("output {} {:?}", o.name(), o.dtype());
    }
    Ok(())
}

/// The graph the rows of `kind` mostly ran.
fn graph_of(rows: &[DumpRow], kind: Kind) -> u8 {
    if kind == Kind::Observations {
        return 0;
    }
    let mut counts = BTreeMap::new();
    for r in rows.iter().filter(|r| r.meta.k > 0) {
        *counts.entry(r.meta.graph).or_insert(0_usize) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|&(_, n)| n)
        .map_or(2, |(g, _)| g)
}

/// A fixed-seed shuffle (splitmix64), so every provider sees the same calls.
fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut state = seed;
    let mut next = || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };
    for i in (1..items.len()).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

/// Readings from `hwmon` and `/proc/stat` while a measurement runs.
#[derive(Default)]
struct Sensors {
    samples: usize,
    cpu_temp_max: f64,
    gpu_temp_max: f64,
    apu_power_sum: f64,
    apu_power_max: f64,
    gpu_busy_sum: f64,
    cpu_busy_sum: f64,
}

fn read_num(path: &std::path::Path) -> Option<f64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn hwmon_dirs() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    if let Ok(dir) = std::fs::read_dir("/sys/class/hwmon") {
        for e in dir.flatten() {
            let p = e.path();
            if let Ok(name) = std::fs::read_to_string(p.join("name")) {
                out.push((name.trim().to_owned(), p));
            }
        }
    }
    out
}

fn cpu_jiffies() -> Option<(u64, u64)> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    let line = stat.lines().next()?;
    let v: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|x| x.parse().ok())
        .collect();
    let idle = v.get(3)? + v.get(4).copied().unwrap_or(0);
    Some((v.iter().sum(), idle))
}

fn gpu_busy_path() -> Option<PathBuf> {
    let dir = std::fs::read_dir("/sys/class/drm").ok()?;
    dir.flatten()
        .map(|e| e.path().join("device/gpu_busy_percent"))
        .find(|p| p.is_file())
}

fn sample_sensors(stop: &Arc<Mutex<bool>>) -> Sensors {
    let hw = hwmon_dirs();
    let gpu_busy = gpu_busy_path();
    let mut s = Sensors::default();
    let mut last = cpu_jiffies();
    loop {
        std::thread::sleep(Duration::from_millis(500));
        if *stop
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            break;
        }
        s.samples += 1;
        for (name, p) in &hw {
            match name.as_str() {
                "k10temp" => {
                    if let Some(t) = read_num(&p.join("temp1_input")) {
                        s.cpu_temp_max = s.cpu_temp_max.max(t / 1000.0);
                    }
                }
                "amdgpu" => {
                    if let Some(t) = read_num(&p.join("temp1_input")) {
                        s.gpu_temp_max = s.gpu_temp_max.max(t / 1000.0);
                    }
                    let w = read_num(&p.join("power1_input"))
                        .or_else(|| read_num(&p.join("power1_average")))
                        .map_or(0.0, |uw| uw / 1e6);
                    s.apu_power_sum += w;
                    s.apu_power_max = s.apu_power_max.max(w);
                }
                _ => {}
            }
        }
        if let Some(p) = &gpu_busy {
            s.gpu_busy_sum += read_num(p).unwrap_or(0.0);
        }
        let now = cpu_jiffies();
        if let (Some((t0, i0)), Some((t1, i1))) = (last, now)
            && t1 > t0
        {
            s.cpu_busy_sum += 100.0 * (1.0 - (i1 - i0) as f64 / (t1 - t0) as f64);
        }
        last = now;
    }
    s
}

fn sensors_json(s: &Sensors) -> Value {
    let n = s.samples.max(1) as f64;
    json!({
        "samples": s.samples,
        "cpu_temp_max_c": s.cpu_temp_max,
        "gpu_temp_max_c": s.gpu_temp_max,
        "apu_power_mean_w": s.apu_power_sum / n,
        "apu_power_max_w": s.apu_power_max,
        "gpu_busy_mean_pct": s.gpu_busy_sum / n,
        "cpu_busy_mean_pct": s.cpu_busy_sum / n,
    })
}

/// Unix time now, in seconds.
fn unix_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

/// `--start-at`: with every seat warmed, the clock starts at an agreed
/// instant, so processes on other devices measure the same window.
/// Returns the instant it starts.
fn wait_until(at: Option<f64>) -> f64 {
    if let Some(at) = at {
        let now = unix_now();
        if at > now {
            std::thread::sleep(Duration::from_secs_f64(at - now));
        } else {
            eprintln!(
                "--start-at passed {:.1} s before the seats were ready",
                now - at
            );
        }
    }
    unix_now()
}

/// One seat's share of a timing run.
struct SeatResult {
    latencies_ms: Vec<f64>,
    rows: usize,
    seconds: f64,
    real_tokens: usize,
    padded_tokens: usize,
    /// Distinct call shapes this seat ran (what a compiling provider builds).
    shapes: usize,
}

/// One seat's calls: every `seats`-th batch of the rows, at most 48 of them,
/// spread evenly over the rows (which matters when they are sorted by size).
fn seat_batches<'a>(
    rows: &'a [DumpRow],
    seat: usize,
    seats: usize,
    batch: usize,
    buckets: &Buckets,
) -> anyhow::Result<Vec<(Batch, Vec<&'a DumpRow>)>> {
    let calls = rows.len() / batch;
    let mine: Vec<usize> = (seat..calls).step_by(seats).collect();
    let step = mine.len().div_ceil(48).max(1);
    let mut batches = Vec::new();
    for &call in mine.iter().step_by(step) {
        let chunk: Vec<&DumpRow> = rows[call * batch..(call + 1) * batch].iter().collect();
        batches.push((assemble(&chunk, buckets)?, chunk));
    }
    ensure!(!batches.is_empty(), "too few rows for batch {batch}");
    Ok(batches)
}

/// Runs `batches` round-robin on the clock for `seconds` (at least three
/// calls) and counts what they did.
fn seat_clock(
    session: &mut Session,
    batches: &[(Batch, Vec<&DumpRow>)],
    seconds: f64,
) -> anyhow::Result<SeatResult> {
    let shapes = batches
        .iter()
        .map(|(b, _)| b.shape.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let start = Instant::now();
    let mut out = SeatResult {
        latencies_ms: Vec::new(),
        rows: 0,
        seconds: 0.0,
        real_tokens: 0,
        padded_tokens: 0,
        shapes,
    };
    let mut i = 0;
    while out.latencies_ms.len() < 3 || start.elapsed().as_secs_f64() < seconds {
        let (b, r) = &batches[i % batches.len()];
        let t = Instant::now();
        run(session, b, r)?;
        out.latencies_ms.push(t.elapsed().as_secs_f64() * 1e3);
        out.rows += r.len();
        out.real_tokens += b.real_tokens;
        out.padded_tokens += b.padded_tokens;
        i += 1;
    }
    out.seconds = start.elapsed().as_secs_f64();
    Ok(out)
}

fn time_one(
    args: &Args,
    rows: &Arc<Vec<DumpRow>>,
    graph: u8,
    batch: usize,
    seats: usize,
) -> anyhow::Result<Value> {
    let barrier = Arc::new(Barrier::new(seats + 1));
    let go = Arc::new(Barrier::new(seats + 1));
    let load = Instant::now();
    let mut handles = Vec::new();
    for seat in 0..seats {
        let rows = Arc::clone(rows);
        let barrier = Arc::clone(&barrier);
        let go = Arc::clone(&go);
        let buckets = args.buckets.clone();
        let mut session = session(args, graph, seats)?;
        let (warmup, seconds) = (args.warmup, args.seconds);
        handles.push(std::thread::spawn(move || -> anyhow::Result<SeatResult> {
            let batches = seat_batches(&rows, seat, seats, batch, &buckets)?;
            // `--warmup 0` runs every call once first: a compiling provider
            // builds each shape before the clock starts.
            let warm = if warmup == 0 { batches.len() } else { warmup };
            for i in 0..warm {
                let (b, r) = &batches[i % batches.len()];
                run(&mut session, b, r)?;
            }
            barrier.wait();
            go.wait();
            seat_clock(&mut session, &batches, seconds)
        }));
    }
    let stop = Arc::new(Mutex::new(false));
    let sensor_stop = Arc::clone(&stop);
    barrier.wait();
    let load_ms = load.elapsed().as_millis();
    let window_start = wait_until(args.start_at);
    go.wait();
    let sensors = std::thread::spawn(move || sample_sensors(&sensor_stop));
    let mut results = Vec::new();
    for h in handles {
        results.push(h.join().map_err(|_| anyhow::anyhow!("a seat panicked"))??);
    }
    *stop
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
    let sensors = sensors
        .join()
        .map_err(|_| anyhow::anyhow!("sensors panicked"))?;
    let wall = results.iter().map(|r| r.seconds).fold(0.0, f64::max);
    let total_rows: usize = results.iter().map(|r| r.rows).sum();
    let mut lat: Vec<f64> = results
        .iter()
        .flat_map(|r| r.latencies_ms.iter().copied())
        .collect();
    lat.sort_by(f64::total_cmp);
    let real: usize = results.iter().map(|r| r.real_tokens).sum();
    let padded: usize = results.iter().map(|r| r.padded_tokens).sum();
    let report = json!({
        "batch": batch,
        "seats": seats,
        "calls": lat.len(),
        "rows": total_rows,
        "seconds": wall,
        "rows_per_s": total_rows as f64 / wall,
        "calls_per_s": lat.len() as f64 / wall,
        "latency_ms": {
            "p50": percentile(&lat, 0.5),
            "p95": percentile(&lat, 0.95),
            "mean": lat.iter().sum::<f64>() / lat.len().max(1) as f64,
            "max": lat.last().copied().unwrap_or(f64::NAN),
        },
        "distinct_shapes_per_seat": results.iter().map(|r| r.shapes).collect::<Vec<_>>(),
        "per_seat_rows_per_s": results.iter().map(|r| r.rows as f64 / r.seconds).collect::<Vec<_>>(),
        "tokens": {"real": real, "computed": padded, "padding_overhead": padded as f64 / real.max(1) as f64 - 1.0},
        "session_load_ms_incl_warmup": load_ms,
        "window_unix_start": window_start,
        "sensors": sensors_json(&sensors),
    });
    println!("{report}");
    Ok(report)
}

/// Agreement of one provider with the reference, per decision kind.
#[derive(Default)]
struct KindStats {
    rows: usize,
    top1_checked: usize,
    top1_agree: usize,
    /// Reference top-1 margin (logits) at each disagreement.
    disagree_margins: Vec<f32>,
    subset_checked: usize,
    subset_agree: usize,
    pair_checked: usize,
    pair_agree: usize,
    max_cand_diff: f32,
    sum_cand_diff: f64,
    cand_n: usize,
    max_value_diff: f32,
    max_h_diff: f32,
}

fn argmax_masked(v: &[f32], mask: &[bool]) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;
    let mut second = f32::NEG_INFINITY;
    for (i, (&x, &m)) in v.iter().zip(mask).enumerate() {
        if !m {
            continue;
        }
        match best {
            Some((_, b)) if x <= b => second = second.max(x),
            Some((_, b)) => {
                second = b;
                best = Some((i, x));
            }
            None => best = Some((i, x)),
        }
    }
    best.map(|(i, b)| (i, b - second))
}

fn bools(row: &DumpRow, name: &str) -> Vec<bool> {
    match row.tensor(name).map(|t| &t.data) {
        Some(TensorData::Bool(v)) => v.clone(),
        _ => Vec::new(),
    }
}

fn max_diff(a: &[f32], b: &[f32], mask: Option<&[bool]>) -> (f32, f64, usize) {
    let mut max = 0.0_f32;
    let mut sum = 0.0_f64;
    let mut n = 0;
    for (i, (&x, &y)) in a.iter().zip(b).enumerate() {
        if mask.is_some_and(|m| !m.get(i).copied().unwrap_or(false)) {
            continue;
        }
        if x.abs() >= 1e4 || y.abs() >= 1e4 || !x.is_finite() || !y.is_finite() {
            continue;
        }
        let d = (x - y).abs();
        max = max.max(d);
        sum += f64::from(d);
        n += 1;
    }
    (max, sum, n)
}

fn compare(stats: &mut KindStats, input: &DumpRow, got: &[DumpTensor], want: &DumpRow) {
    stats.rows += 1;
    let get = |name: &str| {
        got.iter()
            .find(|t| t.name == name)
            .and_then(DumpTensor::f32s)
    };
    let want_t = |name: &str| want.tensor(name).and_then(DumpTensor::f32s);
    if let (Some(g), Some(w)) = (get("h_out"), want_t("h_out")) {
        stats.max_h_diff = stats.max_h_diff.max(max_diff(g, w, None).0);
    }
    if let (Some(g), Some(w)) = (get("value"), want_t("value")) {
        stats.max_value_diff = stats.max_value_diff.max(max_diff(g, w, None).0);
    }
    if input.meta.k == 0 {
        return;
    }
    let cmask = bools(input, "cand_mask");
    if let (Some(g), Some(w)) = (get("cand_logits"), want_t("cand_logits")) {
        let (m, s, n) = max_diff(g, w, Some(&cmask));
        stats.max_cand_diff = stats.max_cand_diff.max(m);
        stats.sum_cand_diff += s;
        stats.cand_n += n;
        if cmask.iter().filter(|&&m| m).count() >= 2
            && let (Some((gi, _)), Some((wi, margin))) =
                (argmax_masked(g, &cmask), argmax_masked(w, &cmask))
        {
            stats.top1_checked += 1;
            if gi == wi {
                stats.top1_agree += 1;
            } else {
                stats.disagree_margins.push(margin);
            }
        }
        if input.meta.head == 1 {
            stats.subset_checked += 1;
            let same = g
                .iter()
                .zip(w)
                .zip(&cmask)
                .filter(|(_, m)| **m)
                .all(|((x, y), _)| (*x > 0.0) == (*y > 0.0));
            stats.subset_agree += usize::from(same);
        }
    }
    let pmask = bools(input, "pair_mask");
    if input.meta.head == 2
        && pmask.iter().filter(|&&m| m).count() >= 2
        && let (Some(g), Some(w)) = (get("pair_logits"), want_t("pair_logits"))
        && let (Some((gi, _)), Some((wi, _))) = (argmax_masked(g, &pmask), argmax_masked(w, &pmask))
    {
        stats.pair_checked += 1;
        stats.pair_agree += usize::from(gi == wi);
    }
}

fn stats_json(s: &KindStats) -> Value {
    let mut margins = s.disagree_margins.clone();
    margins.sort_by(f32::total_cmp);
    json!({
        "rows": s.rows,
        "top1_checked": s.top1_checked,
        "top1_agree": s.top1_agree,
        "top1_agreement": if s.top1_checked > 0 { s.top1_agree as f64 / s.top1_checked as f64 } else { f64::NAN },
        "disagreement_ref_margin_p50": margins.get(margins.len() / 2),
        "disagreement_ref_margin_max": margins.last(),
        "subset_checked": s.subset_checked,
        "subset_agree": s.subset_agree,
        "pair_checked": s.pair_checked,
        "pair_agree": s.pair_agree,
        "max_cand_logit_diff": s.max_cand_diff,
        "mean_cand_logit_diff": if s.cand_n > 0 { s.sum_cand_diff / s.cand_n as f64 } else { 0.0 },
        "max_value_diff": s.max_value_diff,
        "max_h_out_diff": s.max_h_diff,
    })
}

/// The rows a reference or parity run takes, in capture order: the first
/// `limit` decision rows of `graph` and every observation before them.
fn parity_rows(rows: Vec<DumpRow>, graph: u8, limit: usize) -> Vec<DumpRow> {
    let mut out = Vec::new();
    let mut decisions = 0;
    for r in rows {
        if r.meta.k == 0 {
            out.push(r);
        } else if r.meta.graph == graph {
            if decisions >= limit {
                break;
            }
            decisions += 1;
            out.push(r);
        }
    }
    out
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse()?;
    let started = Instant::now();
    let mut rows = load_rows(&args.rows)?;
    let graph = graph_of(&rows, args.kind);
    eprintln!(
        "{} rows loaded in {:.1} s; graph k{graph}",
        rows.len(),
        started.elapsed().as_secs_f64()
    );
    let header = json!({
        "tool": "hw_bench",
        "label": args.label,
        "mode": format!("{:?}", args.mode),
        "ep": args.ep.as_str(),
        "model": args.model,
        "graph": graph,
        "intra": args.intra,
        "buckets": args.buckets.describe(),
        "sorted_by_size": args.sort,
        "dims": args.dims,
        "rows_files": args.rows,
    });
    match args.mode {
        Mode::Time => {
            rows.retain(|r| match args.kind {
                Kind::Decisions => r.meta.k > 0 && r.meta.graph == graph,
                Kind::Observations => r.meta.k == 0,
            });
            // Fixed free dimensions: only rows that fit, all padded to them.
            let fixed = fixed_extents(&args.dims);
            if !fixed.is_empty() {
                let before = rows.len();
                rows.retain(|r| fixed.iter().all(|(&axis, &n)| own(r, axis) <= n));
                eprintln!("{} of {before} rows fit {:?}", rows.len(), args.dims);
            }
            shuffle(&mut rows, args.seed);
            if args.sort {
                // Calls of like-sized rows, as a batching server that groups
                // by size would make them.
                rows.sort_by_key(|r| own(r, Axis::Entities) + own(r, Axis::Cands));
            }
            let rows = Arc::new(rows);
            let mut configs = Vec::new();
            for &seats in &args.seats {
                for &batch in &args.batches {
                    configs.push(time_one(&args, &rows, graph, batch, seats)?);
                }
            }
            let report = json!({"header": header, "rows": rows.len(), "configs": configs});
            if let Some(out) = &args.out {
                std::fs::write(out, serde_json::to_vec_pretty(&report)?)?;
            }
        }
        Mode::Ref => {
            let rows = parity_rows(rows, graph, args.limit);
            let out = args
                .out
                .as_ref()
                .context("--out names the reference file")?;
            let mut writer = RowWriter::new(BufWriter::new(File::create(out)?), &header)?;
            let mut sessions = BTreeMap::new();
            for r in &rows {
                let g = r.meta.graph;
                let s = match sessions.entry(g) {
                    std::collections::btree_map::Entry::Occupied(o) => o.into_mut(),
                    std::collections::btree_map::Entry::Vacant(v) => {
                        v.insert(session(&args, g, 1)?)
                    }
                };
                let batch = assemble(&[r], &Buckets::None)?;
                let outs = run(s, &batch, &[r])?.pop().context("one row")?;
                writer.write(&DumpRow {
                    meta: r.meta.clone(),
                    tensors: outs,
                })?;
            }
            writer.finish()?;
            eprintln!(
                "{} reference rows in {:.1} s",
                rows.len(),
                started.elapsed().as_secs_f64()
            );
        }
        Mode::Parity => parity(&args, rows, graph, &header)?,
        Mode::Stats => stats(&args, &rows, &header)?,
        Mode::Io => print_io(&args, graph)?,
        Mode::Check => check_indices(&rows, &header)?,
        Mode::Plan => plan_buckets(&args, &rows, &header)?,
    }
    Ok(())
}

/// A parity run's rows and the reference rows beside them.
fn parity_pairs(
    args: &Args,
    rows: Vec<DumpRow>,
    graph: u8,
) -> anyhow::Result<(Vec<DumpRow>, Vec<DumpRow>)> {
    let rows = parity_rows(rows, graph, args.limit);
    // A reference of every row serves any `--limit`: the run's rows are a
    // prefix of it (each row's metadata is checked against it later).
    let mut reference = load_rows(std::slice::from_ref(
        args.reference
            .as_ref()
            .context("--ref names the reference")?,
    ))?;
    ensure!(
        reference.len() >= rows.len(),
        "the reference has {} rows, the run {}",
        reference.len(),
        rows.len()
    );
    reference.truncate(rows.len());
    // `--decisions-only`: without `--chained` a decision needs no
    // observation run before it (its memory is the recorded one), so
    // decisions can fill calls of any batch size.
    if args.decisions_only {
        ensure!(
            !args.chained,
            "--decisions-only leaves out the observations --chained needs"
        );
        Ok(rows
            .into_iter()
            .zip(reference)
            .filter(|(r, _)| r.meta.k > 0)
            .unzip())
    } else {
        Ok((rows, reference))
    }
}

fn parity(args: &Args, rows: Vec<DumpRow>, graph: u8, header: &Value) -> anyhow::Result<()> {
    let (rows, reference) = parity_pairs(args, rows, graph)?;
    let mut sessions = BTreeMap::new();
    let mut by_kind: BTreeMap<String, KindStats> = BTreeMap::new();
    let mut all_decisions = KindStats::default();
    let mut observations = KindStats::default();
    // `--chained`: each seat's memory from this provider's own observations.
    let mut memory: HashMap<(u32, u8), Vec<f32>> = HashMap::new();
    // A fixed batch (`--dims B=n`) takes calls of exactly n rows; a short
    // call is filled with copies of its last row, whose answers are dropped.
    let fixed_b = args
        .dims
        .iter()
        .find(|(name, _)| name == "B")
        .and_then(|&(_, b)| usize::try_from(b).ok());
    let pb = fixed_b.unwrap_or(args.parity_batch).max(1);
    let mut i = 0;
    // With `--dims`, rows larger than the fixed extents are left out.
    let fixed = fixed_extents(&args.dims);
    let fits = |r: &DumpRow| fixed.iter().all(|(&axis, &n)| own(r, axis) <= n);
    let mut skipped = 0_usize;
    while i < rows.len() {
        if !fits(&rows[i]) {
            skipped += 1;
            i += 1;
            continue;
        }
        // A call holds consecutive rows of one graph; chained runs go one
        // row at a time so each sees the memory before it.
        let g = rows[i].meta.graph;
        let mut end = i + 1;
        while !args.chained
            && end < rows.len()
            && end - i < pb
            && rows[end].meta.graph == g
            && fits(&rows[end])
        {
            end += 1;
        }
        let mut owned: Vec<DumpRow> = Vec::new();
        for r in &rows[i..end] {
            let mut r = r.clone();
            if args.chained
                && let Some(h) = memory.get(&(r.meta.game, r.meta.seat))
                && let Some(t) = r.tensors.iter_mut().find(|t| t.name == "h_in")
            {
                t.data = TensorData::F32(h.clone());
            }
            owned.push(r);
        }
        let real = owned.len();
        if let Some(b) = fixed_b {
            while owned.len() < b {
                owned.push(owned[real - 1].clone());
            }
        }
        let call: Vec<&DumpRow> = owned.iter().collect();
        let s = match sessions.entry(g) {
            std::collections::btree_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::btree_map::Entry::Vacant(v) => v.insert(session(args, g, 1)?),
        };
        let batch = assemble(&call, &args.buckets)?;
        let outs = run(s, &batch, &call)?;
        for (j, out) in outs.iter().enumerate().take(real) {
            let input = call[j];
            let want = &reference[i + j];
            ensure!(want.meta == input.meta, "the reference is of other rows");
            if input.meta.k == 0 {
                compare(&mut observations, input, out, want);
                if let Some(h) = out
                    .iter()
                    .find(|t| t.name == "h_out")
                    .and_then(DumpTensor::f32s)
                {
                    memory.insert((input.meta.game, input.meta.seat), h.to_vec());
                }
            } else {
                compare(&mut all_decisions, input, out, want);
                compare(
                    by_kind.entry(input.meta.label.clone()).or_default(),
                    input,
                    out,
                    want,
                );
            }
        }
        i = end;
    }
    let report = json!({
        "header": header,
        "chained": args.chained,
        "parity_batch": pb,
        "decisions_only": args.decisions_only,
        "skipped_rows": skipped,
        "decisions": stats_json(&all_decisions),
        "observations": stats_json(&observations),
        "by_kind": by_kind.iter().map(|(k, s)| (k.clone(), stats_json(s))).collect::<serde_json::Map<_, _>>(),
    });
    println!("{}", serde_json::to_string_pretty(&report["decisions"])?);
    if let Some(out) = &args.out {
        std::fs::write(out, serde_json::to_vec_pretty(&report)?)?;
    }
    Ok(())
}

/// Percentiles of each variable axis over the rows, per kind of row: what
/// shape buckets must cover.
fn stats(args: &Args, rows: &[DumpRow], header: &Value) -> anyhow::Result<()> {
    let mut out = serde_json::Map::new();
    for (kind, keep) in [("decisions", true), ("observations", false)] {
        let picked: Vec<&DumpRow> = rows.iter().filter(|r| (r.meta.k > 0) == keep).collect();
        let mut axes = serde_json::Map::new();
        let mut tokens: Vec<f64> = picked
            .iter()
            .map(|r| (own(r, Axis::Entities) + own(r, Axis::Cands)) as f64)
            .collect();
        tokens.sort_by(f64::total_cmp);
        for axis in VARIABLE_AXES {
            let mut v: Vec<f64> = picked.iter().map(|r| own(r, axis) as f64).collect();
            v.sort_by(f64::total_cmp);
            axes.insert(
                format!("{axis:?}"),
                json!({
                    "p50": percentile(&v, 0.5), "p90": percentile(&v, 0.9),
                    "p99": percentile(&v, 0.99), "max": v.last(),
                    "mean": v.iter().sum::<f64>() / v.len().max(1) as f64,
                }),
            );
        }
        axes.insert(
            "tokens_entities_plus_cands".to_owned(),
            json!({
                "p50": percentile(&tokens, 0.5), "p90": percentile(&tokens, 0.9),
                "p99": percentile(&tokens, 0.99), "max": tokens.last(),
                "mean": tokens.iter().sum::<f64>() / tokens.len().max(1) as f64,
            }),
        );
        let mut labels: BTreeMap<&str, usize> = BTreeMap::new();
        for r in &picked {
            *labels.entry(r.meta.label.as_str()).or_default() += 1;
        }
        out.insert(
            kind.to_owned(),
            json!({"rows": picked.len(), "axes": axes, "labels": labels}),
        );
    }
    let report = json!({"header": header, "stats": out});
    println!("{}", serde_json::to_string_pretty(&report)?);
    if let Some(path) = &args.out {
        std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    }
    Ok(())
}

/// Index ranges of every row's index inputs, against the row's own extents:
/// a provider's gather or scatter may fault on what the CPU tolerates.
/// Counts, per input, entries below or above the valid range, split by
/// whether the entry is live (its mask set) or padding.
/// One index input to check: name, values, whether each is live, the
/// valid range (low inclusive, high exclusive).
type IndexCheck = (&'static str, Vec<i64>, Vec<bool>, i64, i64);

#[allow(
    clippy::too_many_lines,
    reason = "one flat table of the graph's index inputs and their ranges"
)]
fn check_indices(rows: &[DumpRow], header: &Value) -> anyhow::Result<()> {
    #[derive(Default)]
    struct Range {
        min: i64,
        max: i64,
        live_low: usize,
        live_high: usize,
        pad_low: usize,
        pad_high: usize,
        rows_bad: usize,
    }
    let ints = |row: &DumpRow, name: &str| -> Vec<i64> {
        match row.tensor(name).map(|t| &t.data) {
            Some(TensorData::I64(v)) => v.clone(),
            _ => Vec::new(),
        }
    };
    let mut ranges: BTreeMap<&str, Range> = BTreeMap::new();
    let mut bad_examples: Vec<Value> = Vec::new();
    for row in rows {
        let e = own(row, Axis::Entities) as i64;
        let edge_mask = bools(row, "edge_mask");
        let cand_mask = bools(row, "cand_mask");
        let pair_mask = bools(row, "pair_mask");
        let entity_mask = bools(row, "entity_mask");
        // (input, values, per-entry live flag, valid low, valid high exclusive)
        let width = |name: &str| {
            row.tensor(name)
                .map_or(1, |t| t.dims.iter().skip(2).product::<usize>().max(1))
        };
        let live_of = |mask: &[bool], per: usize, n: usize| -> Vec<bool> {
            (0..n)
                .map(|i| mask.get(i / per).copied().unwrap_or(false))
                .collect()
        };
        let checks: Vec<IndexCheck> = vec![
            {
                let v = ints(row, "edge_index");
                let live = live_of(&edge_mask, 2, v.len());
                ("edge_index", v, live, 0, e + 9)
            },
            {
                let v = ints(row, "cand_entity_idx");
                let live = live_of(&cand_mask, 1, v.len());
                ("cand_entity_idx", v, live, -1, e)
            },
            {
                let v = ints(row, "cand_seat_idx");
                let live = live_of(&cand_mask, 1, v.len());
                ("cand_seat_idx", v, live, -1, 8)
            },
            {
                let v = ints(row, "pair_index");
                let live = live_of(&pair_mask, 2, v.len());
                ("pair_index", v, live, 0, e)
            },
            {
                let v = ints(row, "edge_type");
                let live = live_of(&edge_mask, 1, v.len());
                ("edge_type", v, live, 0, i64::MAX)
            },
            {
                let v = ints(row, "entity_rules");
                let live = live_of(&entity_mask, width("entity_rules"), v.len());
                ("entity_rules", v, live, 0, i64::MAX)
            },
            {
                let v = ints(row, "cand_rules");
                let live = live_of(&cand_mask, width("cand_rules"), v.len());
                ("cand_rules", v, live, 0, i64::MAX)
            },
            {
                // Rule references are (node, role) pairs; roles index an
                // 8-row embedding, nodes the catalog table.
                let v: Vec<i64> = ints(row, "entity_rules")
                    .into_iter()
                    .chain(ints(row, "cand_rules"))
                    .skip(1)
                    .step_by(2)
                    .collect();
                let live = vec![true; v.len()];
                ("rule_roles", v, live, 0, 8)
            },
            {
                let v: Vec<i64> = ints(row, "entity_rules")
                    .into_iter()
                    .chain(ints(row, "cand_rules"))
                    .step_by(2)
                    .collect();
                let live = vec![true; v.len()];
                ("rule_nodes", v, live, 0, 164_959)
            },
        ];
        for (name, values, live, low, high) in checks {
            let r = ranges.entry(name).or_insert_with(|| Range {
                min: i64::MAX,
                max: i64::MIN,
                ..Range::default()
            });
            let mut bad = false;
            for (&x, &l) in values.iter().zip(&live) {
                r.min = r.min.min(x);
                r.max = r.max.max(x);
                let (lo, hi) = (x < low, x >= high);
                match (l, lo, hi) {
                    (true, true, _) => r.live_low += 1,
                    (true, _, true) => r.live_high += 1,
                    (false, true, _) => r.pad_low += 1,
                    (false, _, true) => r.pad_high += 1,
                    _ => {}
                }
                bad |= l && (lo || hi);
            }
            if bad {
                r.rows_bad += 1;
                if bad_examples.len() < 10 {
                    bad_examples.push(
                        json!({"input": name, "meta": format!("{:?}", row.meta), "entities": e}),
                    );
                }
            }
        }
    }
    let report = json!({
        "header": header,
        "rows": rows.len(),
        "ranges": ranges.iter().map(|(k, r)| (k.to_string(), json!({
            "min": r.min, "max": r.max,
            "live_below_range": r.live_low, "live_above_range": r.live_high,
            "pad_below_range": r.pad_low, "pad_above_range": r.pad_high,
            "rows_with_live_out_of_range": r.rows_bad,
        }))).collect::<serde_json::Map<_, _>>(),
        "examples": bad_examples,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Shape-bucket plans against the rows' real extents (`--mode plan`):
/// for each candidate set of buckets (`--plans "E,M,C;E,M,C|E,M,C"`), which
/// share of decision and observation rows each bucket takes (the smallest
/// that fits), the tokens a call computes against the tokens the rows
/// have, and a modelled cost per row at each batch size: a fixed cost per
/// call plus a cost per computed token (`--cost call_ms,row_ms,token_ms`,
/// fitted to measured `MIGraphX` calls).
#[allow(
    clippy::many_single_char_names,
    reason = "e, m and c are the graph's own axis names"
)]
fn plan_buckets(args: &Args, rows: &[DumpRow], header: &Value) -> anyhow::Result<()> {
    let (call_ms, row_ms, token_ms) = args.cost;
    let mut plans = Vec::new();
    for plan in args.plans.split('|') {
        let mut buckets: Vec<(usize, usize, usize)> = plan
            .split(';')
            .map(|b| {
                let v = list(b)?;
                ensure!(v.len() == 3, "a bucket is E,M,C");
                Ok((v[0], v[1], v[2]))
            })
            .collect::<anyhow::Result<_>>()?;
        buckets.sort_by_key(|&(e, m, c)| e + m + c);
        let mut kinds = serde_json::Map::new();
        for (kind, decisions) in [("decisions", true), ("observations", false)] {
            let mut counts = vec![0_usize; buckets.len()];
            let (mut real, mut computed, mut unfit) = (0_usize, 0_usize, 0_usize);
            let mut n = 0_usize;
            for r in rows.iter().filter(|r| (r.meta.k > 0) == decisions) {
                let (e, m) = (own(r, Axis::Entities), own(r, Axis::Edges));
                let c = if decisions { own(r, Axis::Cands) } else { 1 };
                n += 1;
                match buckets
                    .iter()
                    .position(|&(be, bm, bc)| e <= be && m <= bm && c <= bc)
                {
                    Some(i) => {
                        counts[i] += 1;
                        real += e + c;
                        let (be, _, bc) = buckets[i];
                        computed += be + if decisions { bc } else { 1 };
                    }
                    None => unfit += 1,
                }
            }
            let fit = (n - unfit).max(1) as f64;
            let mean_tokens = computed as f64 / fit;
            let per_row: serde_json::Map<String, Value> = [1_usize, 4, 16, 64]
                .iter()
                .map(|&b| {
                    let ms = call_ms / b as f64 + row_ms + token_ms * mean_tokens;
                    (
                        format!("b{b}"),
                        json!({"ms_per_row": ms, "rows_per_s": 1000.0 / ms}),
                    )
                })
                .collect();
            kinds.insert(
                kind.to_owned(),
                json!({
                    "rows": n,
                    "unfit": unfit,
                    "share_per_bucket": counts.iter().map(|&c| c as f64 / n.max(1) as f64).collect::<Vec<_>>(),
                    "mean_real_tokens": real as f64 / fit,
                    "mean_computed_tokens": mean_tokens,
                    "padding_overhead": computed as f64 / real.max(1) as f64 - 1.0,
                    "model": per_row,
                }),
            );
        }
        plans.push(json!({"buckets": buckets, "kinds": kinds}));
    }
    let report = json!({"header": header, "cost": [call_ms, row_ms, token_ms], "plans": plans});
    println!("{}", serde_json::to_string_pretty(&report)?);
    if let Some(path) = &args.out {
        std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_keeps_each_row_in_its_corner() {
        let src = [1, 2, 3, 4, 5, 6];
        let mut dst = [0; 12];
        pad_copy(&src, &[2, 3], &mut dst, &[3, 4]);
        assert_eq!(dst, [1, 2, 3, 0, 4, 5, 6, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn buckets_round_up() {
        let b = Buckets::parse("e=32,64,128;c=8,16").unwrap();
        assert_eq!(b.round(Axis::Entities, 33), 64);
        assert_eq!(b.round(Axis::Entities, 129), 256);
        assert_eq!(b.round(Axis::Cands, 3), 8);
        assert_eq!(b.round(Axis::Edges, 77), 77);
        assert_eq!(Buckets::Pow2.round(Axis::Edges, 77), 128);
    }

    #[test]
    fn argmax_reports_the_margin_over_the_runner_up() {
        let (i, m) = argmax_masked(&[0.5, 2.0, 1.5, 9.0], &[true, true, true, false]).unwrap();
        assert_eq!(i, 1);
        assert!((m - 0.5).abs() < 1e-6);
    }
}
