//! Timing one design on the NPU and checking its output.

use std::path::Path;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::cpu;
use crate::data::{self, Bf16Matrix, ErrorStats, SplitMix};
use crate::designs::DesignSpec;
use crate::stats::{Summary, gflops};
use crate::xrt::{Npu, Sync};

/// What one design measured.
#[derive(Debug, Serialize)]
pub struct NpuResult {
    pub name: String,
    pub role: String,
    pub logical: [usize; 3],
    pub padded: [usize; 3],
    pub tile: [usize; 3],
    pub instr_words: usize,
    /// Registering the xclbin and creating the hardware context, once.
    pub open_ms: f64,
    /// Sync A and B to the device, run, sync C back: the per-call cost.
    pub e2e: Summary,
    /// `run.start()` + `run.wait()` alone.
    pub run: Summary,
    /// Syncing A and B to the device.
    pub sync_in: Summary,
    /// Syncing C back from the device.
    pub sync_out: Summary,
    /// Copying A (bf16) from a Rust vector into the BO; not part of `e2e`.
    pub copy_a: Summary,
    /// GFLOP/s at the e2e and run p50, counting the logical shape's FLOPs.
    pub gflops_e2e_p50: f64,
    pub gflops_run_p50: f64,
    /// NPU output against the f32 CPU GEMM of the same bf16 inputs.
    pub err_vs_cpu_f32: ErrorStats,
    /// NPU output against the f64 GEMM of the same bf16 inputs.
    pub err_vs_f64: ErrorStats,
    /// The f32 CPU GEMM against the f64 one, for scale.
    pub cpu_f32_err_vs_f64: ErrorStats,
}

/// Copies a row-major `rows x cols` matrix into a zeroed `prows x pcols` one.
fn pad(src: &[u16], (rows, cols): (usize, usize), (prows, pcols): (usize, usize)) -> Vec<u16> {
    if (rows, cols) == (prows, pcols) {
        return src.to_vec();
    }
    let mut out = vec![0_u16; prows * pcols];
    for r in 0..rows {
        out[r * pcols..r * pcols + cols].copy_from_slice(&src[r * cols..(r + 1) * cols]);
    }
    out
}

/// The logical `rows x cols` corner of a row-major `_ x pcols` matrix.
fn crop(src: &[f32], (rows, cols): (usize, usize), pcols: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        out.extend_from_slice(&src[r * pcols..r * pcols + cols]);
    }
    out
}

fn read_instructions(path: &Path) -> Result<Vec<u32>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() % 4 != 0 {
        return Err(format!("{}: length {} is not a multiple of 4", path.display(), bytes.len()));
    }
    Ok(bytes.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect())
}

/// The seeded inputs for a logical shape: A in [-1, 1), B in [-1, 1) /
/// sqrt(K) so C stays near unit scale, both rounded to bf16.
pub fn inputs(m: usize, k: usize, n: usize) -> (Bf16Matrix, Bf16Matrix) {
    let mut rng = SplitMix::new(((m as u64) << 40) ^ ((k as u64) << 20) ^ n as u64);
    let a = Bf16Matrix::random(m * k, 1.0, &mut rng);
    let b = Bf16Matrix::random(k * n, 1.0 / (k as f32).sqrt(), &mut rng);
    (a, b)
}

/// Runs one design: one checked call, `warmup` untimed calls, then `iters`
/// timed ones.
pub fn bench(spec: &DesignSpec, dir: &Path, warmup: usize, iters: usize) -> Result<NpuResult, String> {
    let [m, k, n] = spec.logical;
    let [pm, pk, pn] = spec.padded;
    let (a, b) = inputs(m, k, n);
    let a_pad = pad(&a.bits, (m, k), (pm, pk));
    let b_pad = pad(&b.bits, (k, n), (pk, pn));
    let instr = read_instructions(&dir.join("insts.bin"))?;

    let opened = Instant::now();
    let npu = Npu::open(&dir.join("final.xclbin"), "MLIR_AIE")?;
    let open_ms = opened.elapsed().as_secs_f64() * 1e3;
    let bo_instr = npu.instr_bo(instr.len() * 4)?;
    bo_instr.write_u32(&instr);
    bo_instr.sync(Sync::ToDevice)?;
    let bo_a = npu.data_bo(pm * pk * 2, 3)?;
    let bo_b = npu.data_bo(pk * pn * 2, 4)?;
    let bo_c = npu.data_bo(pm * pn * 4, 5)?;
    bo_a.write_u16(&a_pad);
    bo_b.write_u16(&b_pad);
    // All-ones bytes are NaN as f32: a C the NPU did not write shows up.
    bo_c.fill(0xFF);
    bo_c.sync(Sync::ToDevice)?;
    let run = npu.run(&bo_instr, instr.len() as u32, &bo_a, &bo_b, &bo_c)?;

    // The checked call.
    bo_a.sync(Sync::ToDevice)?;
    bo_b.sync(Sync::ToDevice)?;
    run.exec()?;
    bo_c.sync(Sync::FromDevice)?;
    let got = crop(&bo_c.read_f32(pm * pn), (m, n), pn);

    for _ in 0..warmup {
        bo_a.sync(Sync::ToDevice)?;
        bo_b.sync(Sync::ToDevice)?;
        run.exec()?;
        bo_c.sync(Sync::FromDevice)?;
    }
    let mut t_copy = Vec::with_capacity(iters);
    let mut t_in = Vec::with_capacity(iters);
    let mut t_run = Vec::with_capacity(iters);
    let mut t_out = Vec::with_capacity(iters);
    let mut t_e2e = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        bo_a.write_u16(&a_pad);
        let t1 = Instant::now();
        bo_a.sync(Sync::ToDevice)?;
        bo_b.sync(Sync::ToDevice)?;
        let t2 = Instant::now();
        run.exec()?;
        let t3 = Instant::now();
        bo_c.sync(Sync::FromDevice)?;
        let t4 = Instant::now();
        t_copy.push(t1 - t0);
        t_in.push(t2 - t1);
        t_run.push(t3 - t2);
        t_out.push(t4 - t3);
        t_e2e.push(t4 - t1);
    }
    // The output of the last timed call must still be right.
    let last = crop(&bo_c.read_f32(pm * pn), (m, n), pn);
    drop(run);

    let reference = cpu::reference_f64(&a.wide, &b.wide, (m, k, n));
    let mut cpu_c = vec![0.0_f32; m * n];
    cpu::gemm_f32(&a.wide, &b.wide, &mut cpu_c, (m, k, n), rayon::current_num_threads());
    let cpu_wide: Vec<f64> = cpu_c.iter().map(|&x| f64::from(x)).collect();
    let err_vs_f64 = data::compare(&got, &reference);
    let err_last = data::compare(&last, &reference);
    if err_last.max_abs > err_vs_f64.max_abs * 1.0001 + 1e-6 || err_last.non_finite > 0 {
        return Err(format!(
            "{}: the last timed call differs from the checked one (max_abs {} vs {})",
            spec.name, err_last.max_abs, err_vs_f64.max_abs
        ));
    }

    let e2e = Summary::of(&t_e2e);
    let run = Summary::of(&t_run);
    Ok(NpuResult {
        name: spec.name.clone(),
        role: spec.role.clone(),
        logical: spec.logical,
        padded: spec.padded,
        tile: spec.tile,
        instr_words: instr.len(),
        open_ms,
        gflops_e2e_p50: gflops(m, k, n, e2e.p50_ms),
        gflops_run_p50: gflops(m, k, n, run.p50_ms),
        e2e,
        run,
        sync_in: Summary::of(&t_in),
        sync_out: Summary::of(&t_out),
        copy_a: Summary::of(&t_copy),
        err_vs_cpu_f32: data::compare(&got, &cpu_wide),
        err_vs_f64,
        cpu_f32_err_vs_f64: data::compare(&cpu_c, &reference),
    })
}

/// Runs `spec` back to back for `secs` seconds and returns how many calls
/// finished; the power sampler runs around it.
pub fn soak(spec: &DesignSpec, dir: &Path, secs: f64) -> Result<(usize, Duration), String> {
    let [pm, pk, pn] = spec.padded;
    let [m, k, n] = spec.logical;
    let (a, b) = inputs(m, k, n);
    let instr = read_instructions(&dir.join("insts.bin"))?;
    let npu = Npu::open(&dir.join("final.xclbin"), "MLIR_AIE")?;
    let bo_instr = npu.instr_bo(instr.len() * 4)?;
    bo_instr.write_u32(&instr);
    bo_instr.sync(Sync::ToDevice)?;
    let bo_a = npu.data_bo(pm * pk * 2, 3)?;
    let bo_b = npu.data_bo(pk * pn * 2, 4)?;
    let bo_c = npu.data_bo(pm * pn * 4, 5)?;
    bo_a.write_u16(&pad(&a.bits, (m, k), (pm, pk)));
    bo_b.write_u16(&pad(&b.bits, (k, n), (pk, pn)));
    let run = npu.run(&bo_instr, instr.len() as u32, &bo_a, &bo_b, &bo_c)?;
    let start = Instant::now();
    let mut calls = 0;
    while start.elapsed().as_secs_f64() < secs {
        bo_a.sync(Sync::ToDevice)?;
        bo_b.sync(Sync::ToDevice)?;
        run.exec()?;
        bo_c.sync(Sync::FromDevice)?;
        calls += 1;
    }
    Ok((calls, start.elapsed()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_and_crop_round_trip() {
        let src: Vec<u16> = (0..6).collect();
        let p = pad(&src, (2, 3), (4, 4));
        assert_eq!(p, vec![0, 1, 2, 0, 3, 4, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let wide: Vec<f32> = p.iter().map(|&x| f32::from(x)).collect();
        assert_eq!(crop(&wide, (2, 3), 4), vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
    }
}
