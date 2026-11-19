//! BF16 GEMM microbenchmark for the XDNA (npu1, Phoenix) NPU, with an f32
//! CPU baseline on the same shapes.
//!
//! ```text
//! npu-gemm-bench list
//! npu-gemm-bench build-designs --venv V --mlir-aie-src S [--xrt-root R] --out DIR [--jobs N]
//! npu-gemm-bench run --designs DIR --out results.json [--warmup 10] [--iters 100]
//!     [--cpu-warmup 3] [--cpu-iters 50] [--cpu-threads 1,8] [--only TEXT]
//!     [--skip-npu] [--skip-layers] [--skip-cpu] [--soak-secs 5]
//! npu-gemm-bench soak-layers --m 2048 --seconds 60 --out X.json [--designs DIR] [--no-check]
//! ```
//!
//! `build-designs` drives mlir-aie's IRON `whole_array.py` (bf16 in, f32 out,
//! 4 x 4 cores) once per shape; it runs on any `x86_64` Linux host with the
//! pinned wheels in a venv. `run` needs the NPU: per design it times
//! sync-in + run + sync-out, checks the output against an f32 CPU GEMM and an
//! f64 one, then times the f32 CPU GEMM at each thread count, and writes one
//! JSON file plus a Markdown table on stdout. `soak-layers` keeps the NPU
//! busy with one encoder layer's four GEMMs at one M, back to back, for a
//! fixed time, and records the layer rate and the package power meanwhile.

mod cpu;
mod data;
mod designs;
mod npu;
mod power;
mod stats;
mod xrt;

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use crate::designs::{DesignSpec, Toolchain};
use crate::npu::{LayerResult, NpuResult};
use crate::power::{PowerStats, Sampler};
use crate::stats::{Summary, gflops};

fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn opt_num<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> Result<T, String> {
    match opt(args, name) {
        Some(v) => v.parse().map_err(|_| format!("{name}: not a number: {v}")),
        None => Ok(default),
    }
}

fn required(args: &[String], name: &str) -> Result<PathBuf, String> {
    opt(args, name).map(PathBuf::from).ok_or_else(|| format!("missing {name}"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("list") => {
            for s in designs::design_set() {
                println!("{:<26} logical {:?} padded {:?} tile {:?}", s.name, s.logical, s.padded, s.tile);
            }
            Ok(())
        }
        Some("build-designs") => build_designs(&args[1..]),
        Some("run") => run(&args[1..]),
        Some("soak-layers") => soak_layers(&args[1..]),
        _ => Err("usage: npu-gemm-bench list | build-designs ... | run ... | soak-layers ... (see the crate docs)".to_owned()),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn build_designs(args: &[String]) -> Result<(), String> {
    let tc = Toolchain {
        venv: required(args, "--venv")?,
        mlir_aie_src: required(args, "--mlir-aie-src")?,
        xrt_root: opt(args, "--xrt-root").map(PathBuf::from),
    };
    let out = required(args, "--out")?;
    let jobs = opt_num(args, "--jobs", 4)?;
    let mut specs = designs::design_set();
    if let Some(only) = opt(args, "--only") {
        specs.retain(|s| s.name.contains(&only));
    }
    designs::build_all(&tc, &specs, &out, jobs)
}

/// The CPU baseline at one thread count on one shape.
#[derive(Debug, Serialize)]
struct CpuResult {
    name: String,
    shape: [usize; 3],
    threads: usize,
    time: Summary,
    gflops_p50: f64,
    power: Option<PowerStats>,
}

/// Package power with the machine idle, under the NPU, and under the CPU.
#[derive(Debug, Serialize)]
struct Soak {
    design: String,
    idle: Option<PowerStats>,
    npu_calls: usize,
    npu_gflops: f64,
    npu_power: Option<PowerStats>,
    cpu_threads: usize,
    cpu_calls: usize,
    cpu_gflops: f64,
    cpu_power: Option<PowerStats>,
}

#[derive(Debug, Serialize)]
struct Meta {
    host: String,
    kernel: String,
    cpu: String,
    npu_fw: String,
    npu_name: String,
    unix_time: u64,
    mlir_aie: &'static str,
    warmup: usize,
    iters: usize,
    cpu_warmup: usize,
    cpu_iters: usize,
    rayon_default_threads: usize,
    power_sensor: Option<String>,
    notes: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct Results {
    meta: Meta,
    npu: Vec<NpuResult>,
    layers: Vec<LayerResult>,
    cpu: Vec<CpuResult>,
    soak: Option<Soak>,
}

fn read_trim(path: &str) -> String {
    std::fs::read_to_string(path).map(|s| s.trim().to_owned()).unwrap_or_default()
}

fn unix_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

/// A sustained run of one layer's GEMMs.
#[derive(Debug, Serialize)]
struct LayerSoak {
    host: String,
    m: usize,
    designs: Vec<String>,
    seconds_requested: f64,
    /// Wall-clock bounds of the timed window, seconds since the Unix epoch.
    unix_start: f64,
    unix_end: f64,
    seconds: f64,
    layers: usize,
    layers_per_s: f64,
    /// Logical FLOPs of all layers run, per second of the window.
    gflops: f64,
    /// Per layer: sync A in, run, sync C out for each of the four GEMMs.
    layer: Summary,
    /// The worst GEMM against the f64 reference before the window, unless
    /// `--no-check`.
    max_abs: Option<f64>,
    max_rel: Option<f64>,
    power_sensor: Option<String>,
    power: Option<PowerStats>,
    /// `[ms since the window opened, watts]`, every 20 ms.
    ppt_samples: Vec<[f64; 2]>,
}

fn soak_layers(args: &[String]) -> Result<(), String> {
    let dir = opt(args, "--designs").map_or_else(|| PathBuf::from("designs"), PathBuf::from);
    let m: usize = opt_num(args, "--m", 2048)?;
    let secs: f64 = opt_num(args, "--seconds", 60.0)?;
    let out = required(args, "--out")?;
    let check = !flag(args, "--no-check");
    let group: Vec<(DesignSpec, PathBuf)> = designs::load_all(&dir)?
        .into_iter()
        .filter(|(s, _)| s.role == "gemm" && s.logical[0] == m)
        .collect();
    if group.len() != designs::LAYER_SHAPES.len() {
        return Err(format!(
            "M={m}: found {} of the {} layer designs in {}; M is one of {:?}",
            group.len(),
            designs::LAYER_SHAPES.len(),
            dir.display(),
            designs::M_VALUES
        ));
    }
    let sensor = power::ppt_sensor();
    let soak = npu::with_layer(&group, check, |layer_once, info| {
        for _ in 0..5 {
            layer_once(None)?;
        }
        eprintln!("soak-layers M={m} for {secs} s ...");
        let unix_start = unix_now();
        let sampler = sensor.clone().map(Sampler::start);
        let start = Instant::now();
        let mut times = Vec::new();
        while start.elapsed().as_secs_f64() < secs {
            let t = Instant::now();
            layer_once(None)?;
            times.push(t.elapsed());
        }
        let seconds = start.elapsed().as_secs_f64();
        let (power, ppt_samples) = sampler.map_or((None, Vec::new()), Sampler::stop_series);
        Ok(LayerSoak {
            host: read_trim("/etc/hostname"),
            m,
            designs: group.iter().map(|(s, _)| s.name.clone()).collect(),
            seconds_requested: secs,
            unix_start,
            unix_end: unix_now(),
            seconds,
            layers: times.len(),
            layers_per_s: times.len() as f64 / seconds,
            gflops: info.flop * times.len() as f64 / seconds / 1e9,
            layer: Summary::of(&times),
            max_abs: info.check.map(|c| c.0),
            max_rel: info.check.map(|c| c.1),
            power_sensor: sensor.as_ref().map(|p| p.display().to_string()),
            power,
            ppt_samples,
        })
    })?;
    let json = serde_json::to_string_pretty(&soak).map_err(|e| e.to_string())?;
    std::fs::write(&out, json).map_err(|e| format!("{}: {e}", out.display()))?;
    println!(
        "soak-layers M={} {:.1} s: {} layers, {:.1} layers/s, {:.0} GFLOP/s, layer p50 {:.3} ms p95 {:.3} ms, PPT {}",
        soak.m,
        soak.seconds,
        soak.layers,
        soak.layers_per_s,
        soak.gflops,
        soak.layer.p50_ms,
        soak.layer.p95_ms,
        soak.power
            .map_or_else(|| "-".to_owned(), |p| format!("mean {:.1} W max {:.1} W ({} samples)", p.mean_w, p.max_w, p.samples))
    );
    eprintln!("wrote {}", out.display());
    Ok(())
}

fn cpu_model() -> String {
    read_trim("/proc/cpuinfo")
        .lines()
        .find_map(|l| l.strip_prefix("model name"))
        .map(|l| l.trim_start_matches([' ', '\t', ':']).to_owned())
        .unwrap_or_default()
}

fn run(args: &[String]) -> Result<(), String> {
    let out = required(args, "--out")?;
    let warmup = opt_num(args, "--warmup", 10)?;
    let iters = opt_num(args, "--iters", 100)?;
    let cpu_warmup = opt_num(args, "--cpu-warmup", 3)?;
    let cpu_iters = opt_num(args, "--cpu-iters", 50)?;
    let soak_secs: f64 = opt_num(args, "--soak-secs", 5.0)?;
    let cpu_threads: Vec<usize> = opt(args, "--cpu-threads")
        .unwrap_or_else(|| "1,8".to_owned())
        .split(',')
        .map(|t| t.trim().parse().map_err(|_| format!("--cpu-threads: {t}")))
        .collect::<Result<_, _>>()?;
    let only = opt(args, "--only");
    let sensor = power::ppt_sensor();

    let mut specs: Vec<(DesignSpec, Option<PathBuf>)> = if flag(args, "--skip-npu") {
        designs::design_set().into_iter().map(|s| (s, None)).collect()
    } else {
        designs::load_all(&required(args, "--designs")?)?
            .into_iter()
            .map(|(s, d)| (s, Some(d)))
            .collect()
    };
    if let Some(only) = &only {
        specs.retain(|(s, _)| s.name.contains(only.as_str()));
    }
    if specs.is_empty() {
        return Err("no designs to run".to_owned());
    }

    let mut npu_results = Vec::new();
    for (spec, dir) in &specs {
        let Some(dir) = dir else { continue };
        eprintln!("npu  {} ...", spec.name);
        let r = npu::bench(spec, dir, warmup, iters)?;
        eprintln!(
            "     e2e p50 {:.3} ms p95 {:.3} ms, run p50 {:.3} ms, {:.1} GFLOP/s, max_abs {:.2e} max_rel {:.2e}",
            r.e2e.p50_ms, r.e2e.p95_ms, r.run.p50_ms, r.gflops_e2e_p50, r.err_vs_cpu_f32.max_abs, r.err_vs_cpu_f32.max_rel
        );
        npu_results.push(r);
    }

    let mut layers = Vec::new();
    if !flag(args, "--skip-layers") {
        layers = layer_benches(&specs, warmup, iters)?;
    }

    let mut cpu_results = Vec::new();
    if !flag(args, "--skip-cpu") {
        for &threads in &cpu_threads {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .map_err(|e| e.to_string())?;
            for (spec, _) in specs.iter().filter(|(s, _)| s.role == "gemm") {
                eprintln!("cpu  {} x{threads} ...", spec.name);
                let r = pool.install(|| cpu_bench(spec, threads, cpu_warmup, cpu_iters, sensor.as_ref()));
                eprintln!("     p50 {:.3} ms, {:.1} GFLOP/s", r.time.p50_ms, r.gflops_p50);
                cpu_results.push(r);
            }
        }
    }

    let soak = if soak_secs > 0.0 && !flag(args, "--skip-npu") && !flag(args, "--skip-cpu") {
        soak_power(&specs, soak_secs, &cpu_threads, sensor.as_ref())?
    } else {
        None
    };

    let results = Results {
        meta: Meta {
            host: read_trim("/etc/hostname"),
            kernel: read_trim("/proc/sys/kernel/osrelease"),
            cpu: cpu_model(),
            npu_fw: read_trim("/sys/class/accel/accel0/device/fw_version"),
            npu_name: read_trim("/sys/class/accel/accel0/device/vbnv"),
            unix_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            mlir_aie: designs::MLIR_AIE_VERSION,
            warmup,
            iters,
            cpu_warmup,
            cpu_iters,
            rayon_default_threads: rayon::current_num_threads(),
            power_sensor: sensor.as_ref().map(|p| p.display().to_string()),
            notes: vec![
                "e2e = sync A and B to device + run (start+wait) + sync C from device; copy_a (host memcpy of A into the BO) is separate",
                "GFLOP/s count the logical shape's 2*M*K*N; padded designs compute more",
                "err_vs_*: inputs are bf16; references use the same bf16 values; max_rel only over |ref| >= 1% of max|ref|",
                "CPU baseline: gemm 0.19.0 f32, row-major, on a rayon pool of the given size (1 = inline)",
                "power: amdgpu hwmon PPT (socket power, all of CPU+GPU+NPU+SoC), sampled every 20 ms",
            ],
        },
        npu: npu_results,
        layers,
        cpu: cpu_results,
        soak,
    };
    let json = serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?;
    std::fs::write(&out, json).map_err(|e| format!("{}: {e}", out.display()))?;
    print_table(&results);
    eprintln!("wrote {}", out.display());
    Ok(())
}

/// One layer per M: its four GEMMs back to back in one hardware context.
fn layer_benches(specs: &[(DesignSpec, Option<PathBuf>)], warmup: usize, iters: usize) -> Result<Vec<LayerResult>, String> {
    let mut out = Vec::new();
    for m in designs::M_VALUES {
        let group: Vec<(DesignSpec, PathBuf)> = specs
            .iter()
            .filter(|(s, _)| s.role == "gemm" && s.logical[0] == m)
            .filter_map(|(s, d)| d.clone().map(|d| (s.clone(), d)))
            .collect();
        if group.len() < 2 {
            continue;
        }
        eprintln!("layer M={m} ({} GEMMs in one context) ...", group.len());
        let r = npu::bench_layer(&group, warmup, iters)?;
        eprintln!("     layer p50 {:.3} ms p95 {:.3} ms, {:.1} GFLOP/s", r.layer.p50_ms, r.layer.p95_ms, r.gflops_p50);
        out.push(r);
    }
    Ok(out)
}

fn cpu_bench(spec: &DesignSpec, threads: usize, warmup: usize, iters: usize, sensor: Option<&PathBuf>) -> CpuResult {
    let [m, k, n] = spec.logical;
    let (a, b) = npu::inputs(m, k, n);
    let mut c = vec![0.0_f32; m * n];
    for _ in 0..warmup {
        cpu::gemm_f32(&a.wide, &b.wide, &mut c, (m, k, n), threads);
    }
    let sampler = sensor.cloned().map(Sampler::start);
    let mut times = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t = Instant::now();
        cpu::gemm_f32(&a.wide, &b.wide, &mut c, (m, k, n), threads);
        times.push(t.elapsed());
    }
    let power = sampler.and_then(Sampler::stop);
    let time = Summary::of(&times);
    CpuResult {
        name: spec.name.clone(),
        shape: spec.logical,
        threads,
        gflops_p50: gflops(m, k, n, time.p50_ms),
        time,
        power,
    }
}

/// Idle power, then the largest design on the NPU and the same GEMM on the
/// most CPU threads, each for `secs` seconds.
fn soak_power(
    specs: &[(DesignSpec, Option<PathBuf>)],
    secs: f64,
    cpu_threads: &[usize],
    sensor: Option<&PathBuf>,
) -> Result<Option<Soak>, String> {
    let Some((spec, Some(dir))) = specs
        .iter()
        .filter(|(s, d)| s.role == "gemm" && d.is_some())
        .max_by_key(|(s, _)| s.logical.iter().product::<usize>())
    else {
        return Ok(None);
    };
    let [m, k, n] = spec.logical;
    let flop = 2.0 * (m * k * n) as f64;
    eprintln!("soak idle / npu / cpu on {} for {secs} s each ...", spec.name);

    let sampler = sensor.cloned().map(Sampler::start);
    std::thread::sleep(std::time::Duration::from_secs_f64(secs.min(3.0)));
    let idle = sampler.and_then(Sampler::stop);

    let sampler = sensor.cloned().map(Sampler::start);
    let (npu_calls, npu_time) = npu::soak(spec, Path::new(dir), secs)?;
    let npu_power = sampler.and_then(Sampler::stop);

    let threads = cpu_threads.iter().copied().max().unwrap_or(8);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|e| e.to_string())?;
    let (a, b) = npu::inputs(m, k, n);
    let mut c = vec![0.0_f32; m * n];
    let sampler = sensor.cloned().map(Sampler::start);
    let start = Instant::now();
    let mut cpu_calls = 0;
    pool.install(|| {
        while start.elapsed().as_secs_f64() < secs {
            cpu::gemm_f32(&a.wide, &b.wide, &mut c, (m, k, n), threads);
            cpu_calls += 1;
        }
    });
    let cpu_time = start.elapsed();
    let cpu_power = sampler.and_then(Sampler::stop);

    Ok(Some(Soak {
        design: spec.name.clone(),
        idle,
        npu_calls,
        npu_gflops: flop * npu_calls as f64 / npu_time.as_secs_f64() / 1e9,
        npu_power,
        cpu_threads: threads,
        cpu_calls,
        cpu_gflops: flop * cpu_calls as f64 / cpu_time.as_secs_f64() / 1e9,
        cpu_power,
    }))
}

fn print_table(r: &Results) {
    println!("| design | M (padded) | K | N | tile m,k,n | NPU e2e p50 ms | p95 ms | run p50 ms | NPU GFLOP/s | max abs err | max rel err | CPU 1t p50 ms | CPU 8t p50 ms |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    let cpu_ms = |name: &str, threads: usize| {
        r.cpu
            .iter()
            .find(|c| c.name == name && c.threads == threads)
            .map_or_else(|| "-".to_owned(), |c| format!("{:.3}", c.time.p50_ms))
    };
    for n in &r.npu {
        let [m, k, nn] = n.logical;
        let m_txt = if n.padded[0] == m { m.to_string() } else { format!("{m} ({})", n.padded[0]) };
        println!(
            "| {} | {} | {} | {} | {},{},{} | {:.3} | {:.3} | {:.3} | {:.1} | {:.2e} | {:.2e} | {} | {} |",
            n.name,
            m_txt,
            k,
            nn,
            n.tile[0],
            n.tile[1],
            n.tile[2],
            n.e2e.p50_ms,
            n.e2e.p95_ms,
            n.run.p50_ms,
            n.gflops_e2e_p50,
            n.err_vs_cpu_f32.max_abs,
            n.err_vs_cpu_f32.max_rel,
            cpu_ms(&n.name, 1),
            cpu_ms(&n.name, 8),
        );
    }
    if r.npu.is_empty() {
        for c in &r.cpu {
            println!("cpu {} x{}: p50 {:.3} ms, {:.1} GFLOP/s", c.name, c.threads, c.time.p50_ms, c.gflops_p50);
        }
    }
    if !r.layers.is_empty() {
        println!("\n| layer M | GEMMs | NPU layer p50 ms | p95 ms | NPU GFLOP/s | CPU 1t sum p50 ms | CPU 8t sum p50 ms | max rel err |");
        println!("|---|---|---|---|---|---|---|---|");
        for l in &r.layers {
            let cpu_sum = |threads: usize| {
                let times: Vec<f64> = l
                    .designs
                    .iter()
                    .filter_map(|d| r.cpu.iter().find(|c| &c.name == d && c.threads == threads))
                    .map(|c| c.time.p50_ms)
                    .collect();
                if times.len() == l.designs.len() {
                    format!("{:.3}", times.iter().sum::<f64>())
                } else {
                    "-".to_owned()
                }
            };
            println!(
                "| {} | {} | {:.3} | {:.3} | {:.1} | {} | {} | {:.2e} |",
                l.m,
                l.designs.len(),
                l.layer.p50_ms,
                l.layer.p95_ms,
                l.gflops_p50,
                cpu_sum(1),
                cpu_sum(8),
                l.max_rel
            );
        }
    }
    if let Some(s) = &r.soak {
        let w =|p: &Option<PowerStats>| p.map_or_else(|| "-".to_owned(), |p| format!("{:.1} W", p.mean_w));
        println!(
            "\nsoak {}: idle {}; NPU {:.0} GFLOP/s at {}; CPU x{} {:.0} GFLOP/s at {}",
            s.design,
            w(&s.idle),
            s.npu_gflops,
            w(&s.npu_power),
            s.cpu_threads,
            s.cpu_gflops,
            w(&s.cpu_power)
        );
    }
}
