//! The set of GEMM designs, and building them with mlir-aie's IRON
//! `whole_array` design (bf16 in, f32 out, 4 columns x 4 rows of npu1).
//!
//! Each design is one xclbin plus one instruction stream for one shape,
//! written to `<out>/<name>/{final.xclbin, insts.bin, design.json}`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

/// The mlir-aie release the designs are built with.
pub const MLIR_AIE_VERSION: &str = "1.4.5";
/// The array the designs fill: npu1 has 4 columns of 4 compute rows.
pub const AIE_COLS: usize = 4;
pub const AIE_ROWS: usize = 4;

/// One GEMM `C[M,N] = A[M,K] * B[K,N]` and how it is tiled.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DesignSpec {
    pub name: String,
    /// What the design is for: `gemm` (a net shape) or `dispatch` (the
    /// smallest GEMM, for the fixed per-call cost).
    pub role: String,
    /// The shape the caller wants, `[M, K, N]`.
    pub logical: [usize; 3],
    /// The shape the NPU computes, padded up to the tiling's multiples.
    pub padded: [usize; 3],
    /// The per-core tile `[m, k, n]`.
    pub tile: [usize; 3],
    pub n_aie_cols: usize,
    pub dtype_in: String,
    pub dtype_out: String,
    pub mlir_aie: String,
}

/// The `(K, N)` weight shapes of one r1 encoder layer.
pub const LAYER_SHAPES: [(&str, usize, usize); 4] = [
    ("ffn_up", 640, 2560),
    ("ffn_down", 2560, 640),
    ("qkv", 640, 1920),
    ("attn_out", 640, 640),
];

/// Rows per call: batch 1, 4, 16, 64 at about 32 tokens per row (r1's
/// decision rows: p50 25, mean 27, p99 60 tokens), and 8192 as an upper
/// bound.
pub const M_VALUES: [usize; 5] = [32, 128, 512, 2048, 8192];

/// The tile for `m_rows` rows: the largest m of 64, 32, 16 whose four row
/// tiles divide M, with k = 64 and n = 32; below 64 rows, m = 16 and M is
/// padded up to 64.
///
/// On AIE2 the bf16->f32 kernel needs m and n in steps of 16 and k in steps
/// of 8. 64 x 64 x 32 keeps a core's double-buffered A (2 x 8 KiB), B (2 x
/// 4 KiB) and f32 C (2 x 8 KiB) at 40 KiB of its 64 KiB; n = 64 would also
/// fit with k = 32 but needs N in steps of 256, which 640 and 1920 are not.
fn tile_for(m_rows: usize) -> [usize; 3] {
    let m = [64, 32, 16]
        .into_iter()
        .find(|m| m_rows.is_multiple_of(m * AIE_ROWS))
        .unwrap_or(16);
    [m, 64, 32]
}

fn round_up(x: usize, step: usize) -> usize {
    x.div_ceil(step) * step
}

fn spec(role: &str, name: String, logical: [usize; 3], tile: [usize; 3]) -> DesignSpec {
    let [m, k, n] = tile;
    let padded = [
        round_up(logical[0], m * AIE_ROWS),
        round_up(logical[1], k),
        round_up(logical[2], n * AIE_COLS),
    ];
    DesignSpec {
        name,
        role: role.to_owned(),
        logical,
        padded,
        tile,
        n_aie_cols: AIE_COLS,
        dtype_in: "bf16".to_owned(),
        dtype_out: "f32".to_owned(),
        mlir_aie: MLIR_AIE_VERSION.to_owned(),
    }
}

/// Every design the benchmark runs: the tiny dispatch probe, then each layer
/// shape at each M, then FFN up with bf16 instead of f32 output (the
/// `variant` role), which halves the C traffic of the widest shape.
pub fn design_set() -> Vec<DesignSpec> {
    let mut out = vec![spec("dispatch", "dispatch_64x64x64".to_owned(), [64, 64, 64], [16, 64, 16])];
    for (label, k, n) in LAYER_SHAPES {
        for m in M_VALUES {
            out.push(spec("gemm", format!("{label}_{m}x{k}x{n}"), [m, k, n], tile_for(m)));
        }
    }
    let (label, k, n) = LAYER_SHAPES[0];
    for m in [512, 2048] {
        let mut v = spec("variant", format!("{label}_bf16out_{m}x{k}x{n}"), [m, k, n], tile_for(m));
        "bf16".clone_into(&mut v.dtype_out);
        out.push(v);
    }
    out
}

/// Bytes per element of a design's output.
pub fn out_elem_bytes(spec: &DesignSpec) -> usize {
    if spec.dtype_out == "bf16" { 2 } else { 4 }
}

/// Where the IRON toolchain lives on the build host.
pub struct Toolchain {
    /// The venv with the pinned `mlir_aie` and `llvm-aie` wheels.
    pub venv: PathBuf,
    /// A checkout of mlir-aie at the wheel's tag, for `whole_array.py`.
    pub mlir_aie_src: PathBuf,
    /// A prefix with `usr/bin/xclbinutil` and the libraries it needs, when
    /// XRT is not installed on the build host.
    pub xrt_root: Option<PathBuf>,
}

impl Toolchain {
    fn site_packages(&self) -> Result<PathBuf, String> {
        let lib = self.venv.join("lib");
        fs::read_dir(&lib)
            .map_err(|e| format!("{}: {e}", lib.display()))?
            .flatten()
            .map(|e| e.path().join("site-packages"))
            .find(|p| p.join("mlir_aie").is_dir())
            .ok_or_else(|| format!("no site-packages with mlir_aie under {}", lib.display()))
    }

    fn command(&self, spec: &DesignSpec, dir: &Path) -> Result<Command, String> {
        let site = self.site_packages()?;
        let script = self
            .mlir_aie_src
            .join("programming_examples/basic/matrix_multiplication/whole_array/whole_array.py");
        let mut path = vec![self.venv.join("bin"), site.join("mlir_aie/bin")];
        let mut ld = Vec::new();
        if let Some(root) = &self.xrt_root {
            path.push(root.join("usr/bin"));
            ld.push(root.join("usr/lib/x86_64-linux-gnu"));
        }
        if let Some(sys) = std::env::var_os("PATH") {
            path.extend(std::env::split_paths(&sys));
        }
        let [m_dim, k_dim, n_dim] = spec.padded;
        let [m, k, n] = spec.tile;
        let mut cmd = Command::new(self.venv.join("bin/python3"));
        cmd.current_dir(dir)
            .arg(script)
            .args(["--dev", "npu", "--n-aie-cols", &spec.n_aie_cols.to_string()])
            .args(["-M", &m_dim.to_string(), "-K", &k_dim.to_string(), "-N", &n_dim.to_string()])
            .args(["-m", &m.to_string(), "-k", &k.to_string(), "-n", &n.to_string()])
            .args(["--dtype_in", &spec.dtype_in, "--dtype_out", &spec.dtype_out])
            .arg("--xclbin-path=final.xclbin")
            .arg("--insts-path=insts.bin")
            .env("PATH", std::env::join_paths(path).map_err(|e| e.to_string())?)
            .env("PYTHONPATH", site.join("mlir_aie/python"))
            .env("MLIR_AIE_INSTALL_DIR", site.join("mlir_aie"))
            .env("PEANO_INSTALL_DIR", site.join("llvm-aie"));
        if !ld.is_empty() {
            cmd.env("LD_LIBRARY_PATH", std::env::join_paths(ld).map_err(|e| e.to_string())?);
        }
        Ok(cmd)
    }
}

/// Builds every design in `specs` into `out`, `jobs` at a time. A design
/// whose xclbin and instructions already exist is kept.
pub fn build_all(tc: &Toolchain, specs: &[DesignSpec], out: &Path, jobs: usize) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let next = AtomicUsize::new(0);
    let failures = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..jobs.max(1) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(spec) = specs.get(i) else { break };
                    if let Err(e) = build_one(tc, spec, out) {
                        eprintln!("FAILED {}: {e}", spec.name);
                        failures.lock().expect("lock").push(spec.name.clone());
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("lock");
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("{} design(s) failed: {}", failures.len(), failures.join(", ")))
    }
}

fn build_one(tc: &Toolchain, spec: &DesignSpec, out: &Path) -> Result<(), String> {
    let dir = out.join(&spec.name);
    if dir.join("final.xclbin").is_file() && dir.join("insts.bin").is_file() && dir.join("design.json").is_file() {
        println!("kept    {}", spec.name);
        return Ok(());
    }
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let started = std::time::Instant::now();
    let output = tc
        .command(spec, &dir)?
        .output()
        .map_err(|e| format!("start python: {e}"))?;
    let mut log = output.stdout;
    log.extend_from_slice(&output.stderr);
    fs::write(dir.join("build.log"), &log).map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("{} (see {})", output.status, dir.join("build.log").display()));
    }
    for f in ["final.xclbin", "insts.bin"] {
        if !dir.join(f).is_file() {
            return Err(format!("{f} missing after a successful build"));
        }
    }
    let json = serde_json::to_string_pretty(spec).map_err(|e| e.to_string())?;
    fs::write(dir.join("design.json"), json).map_err(|e| e.to_string())?;
    println!("built   {} in {:.1} s", spec.name, started.elapsed().as_secs_f64());
    Ok(())
}

/// The designs found under `dir`, in the order of [`design_set`] where they
/// belong to it.
pub fn load_all(dir: &Path) -> Result<Vec<(DesignSpec, PathBuf)>, String> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten() {
        let d = entry.path();
        let Ok(text) = fs::read_to_string(d.join("design.json")) else { continue };
        let spec: DesignSpec =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", d.join("design.json").display()))?;
        found.push((spec, d));
    }
    let order: Vec<String> = design_set().into_iter().map(|s| s.name).collect();
    found.sort_by_key(|(s, _)| order.iter().position(|n| *n == s.name).unwrap_or(usize::MAX));
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_design_meets_the_whole_array_constraints() {
        for s in design_set() {
            let [mm, kk, nn] = s.padded;
            let [m, k, n] = s.tile;
            assert_eq!(mm % (m * AIE_ROWS), 0, "{}", s.name);
            assert_eq!(kk % k, 0, "{}", s.name);
            assert_eq!(nn % (n * AIE_COLS), 0, "{}", s.name);
            // AIE2 bf16 -> f32 micro-kernel: m, n in steps of 16, k of 8.
            assert_eq!(m % 16, 0);
            assert_eq!(n % 16, 0);
            assert_eq!(k % 8, 0);
        }
    }

    #[test]
    fn only_m_32_is_padded() {
        for s in design_set() {
            if s.logical[0] == 32 {
                assert_eq!(s.padded, [64, s.logical[1], s.logical[2]], "{}", s.name);
                assert_eq!(s.tile, [16, 64, 32]);
            } else {
                assert_eq!(s.logical, s.padded, "{}", s.name);
            }
        }
        assert_eq!(design_set().len(), 1 + LAYER_SHAPES.len() * M_VALUES.len() + 2);
    }
}
