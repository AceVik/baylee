//! Standalone measured experiment: scalar, portable reduction, hybrid and
//! handwritten AArch64 ASM / SSE2 / Wasm SIMD. Production uses the portable
//! hybrid: explicit SIMD lost the first-word fast path and did not beat LLVM.
//! Compile with rustc -O --edition 2024; see docs/perf-client.md.
#![allow(unsafe_code)] // Isolated benchmark; invariants documented at each kernel.

#[cfg(not(target_arch = "wasm32"))]
use std::hint::black_box;
type Bits = [u64; 16];
#[inline(never)]
fn reference(a: &Bits, b: &Bits) -> bool {
    (0..16).any(|i| a[i] & b[i] != 0)
}
#[inline(never)]
fn reduction(a: &Bits, b: &Bits) -> bool {
    let mut acc = 0;
    for i in 0..16 {
        acc |= a[i] & b[i];
    }
    acc != 0
}
#[path = "../../crates/baylee-core/src/types/bitmap.rs"]
mod portable;
#[inline(never)]
fn hybrid(a: &Bits, b: &Bits) -> bool {
    portable::intersects(a, b)
}
#[cfg(target_arch = "aarch64")]
#[inline(never)]
fn specialized(a: &Bits, b: &Bits) -> bool {
    let result: u32;
    // SAFETY: a and b point to 128 initialized readable bytes. AArch64 NEON
    // supports unaligned loads; all temporary registers are declared. No
    // stores, stack manipulation, floating-point operations or overreads.
    unsafe {
        core::arch::asm!(
        "ldp q0, q1, [{a}]", "ldp q2, q3, [{a}, #32]",
        "ldp q4, q5, [{b}]", "ldp q6, q7, [{b}, #32]",
        "and v0.16b, v0.16b, v4.16b", "and v1.16b, v1.16b, v5.16b",
        "and v2.16b, v2.16b, v6.16b", "and v3.16b, v3.16b, v7.16b",
        "orr v0.16b, v0.16b, v1.16b", "orr v2.16b, v2.16b, v3.16b",
        "orr v16.16b, v0.16b, v2.16b",
        "ldp q0, q1, [{a}, #64]", "ldp q2, q3, [{a}, #96]",
        "ldp q4, q5, [{b}, #64]", "ldp q6, q7, [{b}, #96]",
        "and v0.16b, v0.16b, v4.16b", "and v1.16b, v1.16b, v5.16b",
        "and v2.16b, v2.16b, v6.16b", "and v3.16b, v3.16b, v7.16b",
        "orr v0.16b, v0.16b, v1.16b", "orr v2.16b, v2.16b, v3.16b",
        "orr v0.16b, v0.16b, v2.16b", "orr v16.16b, v16.16b, v0.16b",
        "umaxv s0, v16.4s", "fmov {result:w}, s0",
        a=in(reg) a.as_ptr(), b=in(reg) b.as_ptr(), result=out(reg) result,
        out("v0") _,out("v1") _,out("v2") _,out("v3") _,
        out("v4") _,out("v5") _,out("v6") _,out("v7") _,out("v16") _,
        options(nostack,readonly));
    }
    result != 0
}
#[cfg(target_arch = "x86_64")]
#[inline(never)]
fn specialized(a: &Bits, b: &Bits) -> bool {
    use core::arch::x86_64::*;
    // SAFETY: SSE2 is mandatory on x86_64. Each unaligned load is within
    // the corresponding 128-byte initialized array; integers only.
    unsafe {
        let mut acc = _mm_setzero_si128();
        for i in (0..16).step_by(2) {
            acc = _mm_or_si128(
                acc,
                _mm_and_si128(
                    _mm_loadu_si128(a.as_ptr().add(i).cast()),
                    _mm_loadu_si128(b.as_ptr().add(i).cast()),
                ),
            );
        }
        _mm_movemask_epi8(_mm_cmpeq_epi8(acc, _mm_setzero_si128())) != 65535
    }
}
#[cfg(target_arch = "wasm32")]
#[inline(never)]
fn specialized(a: &Bits, b: &Bits) -> bool {
    use core::arch::wasm32::*;
    // SAFETY: built with simd128; each unaligned vector load remains
    // inside the initialized 128-byte array, with no relaxed SIMD.
    unsafe {
        let mut acc = u64x2_splat(0);
        for i in (0..16).step_by(2) {
            acc = v128_or(
                acc,
                v128_and(
                    v128_load(a.as_ptr().add(i).cast()),
                    v128_load(b.as_ptr().add(i).cast()),
                ),
            );
        }
        v128_any_true(acc)
    }
}
// SAFETY: this standalone executable/library exports exactly one symbol named
// verify; it does not link a second implementation under that name.
#[unsafe(no_mangle)]
pub extern "C" fn verify() -> u32 {
    let mut seed = 7221u64;
    for case in 0..100_000 {
        let mut a = [0; 16];
        let mut b = [0; 16];
        for i in 0..16 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            a[i] = seed;
            b[i] = if case % 2 == 0 {
                !seed
            } else {
                seed.rotate_left(13)
            };
        }
        if case % 4 == 0 {
            b[case as usize % 16] = a[case as usize % 16];
        }
        if reference(&a, &b) != hybrid(&a, &b)
            || reference(&a, &b) != reduction(&a, &b)
            || reference(&a, &b) != specialized(&a, &b)
        {
            return case + 1;
        }
    }
    for bit in 0..1024 {
        let mut a = [0; 16];
        let mut b = [0; 16];
        a[bit / 64] = 1 << (bit % 64);
        if specialized(&a, &b) || hybrid(&a, &b) {
            return 100_001;
        }
        b[bit / 64] = a[bit / 64];
        if !specialized(&a, &b) || !reduction(&a, &b) || !hybrid(&a, &b) {
            return 100_002;
        }
    }
    0
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    assert_eq!(verify(), 0);
    for workload in 0..3 {
        let mut data = Vec::new();
        for n in 0..1024 {
            let mut a = [0; 16];
            let mut b = [0; 16];
            a[n % 16] = 1 << (n % 64);
            if workload == 1 {
                b = a;
            } else if workload == 2 {
                a[0] = 1;
                b[0] = 1;
            }
            data.push((a, b));
        }
        for (name, f) in [
            ("reference", reference as fn(&Bits, &Bits) -> bool),
            ("reduction", reduction),
            ("hybrid", hybrid),
            ("specialized", specialized),
        ] {
            let mut samples = Vec::new();
            for _ in 0..7 {
                let start = std::time::Instant::now();
                for _ in 0..5000 {
                    for (a, b) in &data {
                        black_box(f(black_box(a), black_box(b)));
                    }
                }
                samples.push(start.elapsed().as_nanos() as f64 / (5000.0 * 1024.0));
            }
            samples.sort_by(f64::total_cmp);
            println!("workload={workload} {name} median_ns={:.3}", samples[3]);
        }
    }
}
