//! Integer-only bitmap reduction. Keeping the first word's short circuit is
//! important for common creature types; reducing the rest lets LLVM use NEON,
//! SSE2 or Wasm SIMD without a branch for each empty word.

#[inline]
pub(crate) const fn intersects(a: &[u64; 16], b: &[u64; 16]) -> bool {
    if a[0] & b[0] != 0 {
        return true;
    }
    let mut any = 0;
    let mut i = 1;
    while i < 16 {
        any |= a[i] & b[i];
        i += 1;
    }
    any != 0
}

#[cfg(test)]
mod tests {
    use super::intersects;

    #[test]
    fn reduction_matches_scalar_for_sparse_dense_and_disjoint_bitmaps() {
        let mut seed = 7221_u64;
        for case in 0..100_000 {
            let mut a = [0; 16];
            let mut b = [0; 16];
            for i in 0..16 {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                a[i] = seed;
                b[i] = if case % 2 == 0 {
                    !seed
                } else {
                    seed.rotate_left(13)
                };
            }
            if case % 4 == 0 {
                b[case % 16] = a[case % 16];
            }
            assert_eq!(intersects(&a, &b), (0..16).any(|i| a[i] & b[i] != 0));
        }
        for bit in 0..1024 {
            let mut a = [0; 16];
            let mut b = [0; 16];
            a[bit / 64] = 1 << (bit % 64);
            assert!(!intersects(&a, &b));
            b[bit / 64] = a[bit / 64];
            assert!(intersects(&a, &b));
            assert!(intersects(&b, &a));
        }
    }
}
