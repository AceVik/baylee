//! The CPU side: an f32 GEMM baseline at a fixed thread count, and the f64
//! reference the NPU and the f32 baseline are both checked against.

use gemm::Parallelism;

/// `C = A * B` for row-major `A` (m x k) and `B` (k x n), in `T` (f32 or
/// f64), on the current rayon pool with `threads` workers (1 runs inline).
fn gemm_row_major<T: Copy + Default + 'static>(
    a: &[T],
    b: &[T],
    c: &mut [T],
    (m, k, n): (usize, usize, usize),
    threads: usize,
    one: T,
) {
    assert_eq!(a.len(), m * k, "A shape");
    assert_eq!(b.len(), k * n, "B shape");
    assert_eq!(c.len(), m * n, "C shape");
    let par = if threads <= 1 { Parallelism::None } else { Parallelism::Rayon(threads) };
    // SAFETY: the three slices hold exactly the m x k, k x n and m x n
    // row-major matrices the strides describe (row stride = columns, column
    // stride = 1); `c` does not alias `a` or `b` (it is a unique borrow).
    // With read_dst = false, gemm overwrites C without reading it.
    unsafe {
        gemm::gemm(
            m,
            n,
            k,
            c.as_mut_ptr(),
            1,
            n as isize,
            false,
            a.as_ptr(),
            1,
            k as isize,
            b.as_ptr(),
            1,
            n as isize,
            T::default(),
            one,
            false,
            false,
            false,
            par,
        );
    }
}

/// `C = A * B` in f32 with `threads` workers.
pub fn gemm_f32(a: &[f32], b: &[f32], c: &mut [f32], shape: (usize, usize, usize), threads: usize) {
    gemm_row_major(a, b, c, shape, threads, 1.0);
}

/// The exact-input reference: the bf16 inputs widened to f64, multiplied
/// and summed in f64 on every core.
pub fn reference_f64(a: &[f32], b: &[f32], shape: (usize, usize, usize)) -> Vec<f64> {
    let a: Vec<f64> = a.iter().map(|&x| f64::from(x)).collect();
    let b: Vec<f64> = b.iter().map(|&x| f64::from(x)).collect();
    let mut c = vec![0.0_f64; shape.0 * shape.2];
    gemm_row_major(&a, &b, &mut c, shape, rayon::current_num_threads(), 1.0);
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive(a: &[f32], b: &[f32], (m, k, n): (usize, usize, usize)) -> Vec<f64> {
        let mut c = vec![0.0; m * n];
        for i in 0..m {
            for j in 0..n {
                c[i * n + j] = (0..k).map(|p| f64::from(a[i * k + p]) * f64::from(b[p * n + j])).sum();
            }
        }
        c
    }

    #[test]
    fn gemm_matches_naive_row_major() {
        let shape = (5, 7, 3);
        let a: Vec<f32> = (0..35).map(|i| (i as f32) * 0.25 - 3.0).collect();
        let b: Vec<f32> = (0..21).map(|i| 1.5 - (i as f32) * 0.125).collect();
        let want = naive(&a, &b, shape);
        let mut c = vec![0.0; 15];
        gemm_f32(&a, &b, &mut c, shape, 1);
        for (g, w) in c.iter().zip(&want) {
            assert!((f64::from(*g) - w).abs() < 1e-4, "{g} vs {w}");
        }
        let r = reference_f64(&a, &b, shape);
        for (g, w) in r.iter().zip(&want) {
            assert!((g - w).abs() < 1e-12);
        }
    }
}
