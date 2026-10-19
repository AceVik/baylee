//! 1024-bit (`[u64; 16]`) bitmap operations with portable scalar reference paths
//! and architecture-specific SIMD fast paths (`aarch64` NEON, `x86_64` SSE2,
//! `wasm32` `simd128`).
//!
//! Integer/bitwise operations are strictly deterministic across all targets.
//! Keeping the first word's short-circuit on `intersects` and `is_empty` avoids
//! loading all 128 bytes when common creature types decide the answer on word 0.

#[inline]
#[allow(dead_code)]
pub(crate) const fn intersects_scalar(a: &[u64; 16], b: &[u64; 16]) -> bool {
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

#[inline]
#[allow(dead_code)]
pub(crate) const fn is_empty_scalar(a: &[u64; 16]) -> bool {
    if a[0] != 0 {
        return false;
    }
    let mut any = 0;
    let mut i = 1;
    while i < 16 {
        any |= a[i];
        i += 1;
    }
    any == 0
}

#[inline]
#[allow(dead_code)]
pub(crate) const fn contains_all_scalar(a: &[u64; 16], b: &[u64; 16]) -> bool {
    let mut missing = 0;
    let mut i = 0;
    while i < 16 {
        missing |= !a[i] & b[i];
        i += 1;
    }
    missing == 0
}

#[inline]
#[allow(dead_code)]
pub(crate) const fn union_with_scalar(dst: &mut [u64; 16], src: &[u64; 16]) {
    let mut i = 0;
    while i < 16 {
        dst[i] |= src[i];
        i += 1;
    }
}

#[inline]
#[allow(clippy::too_many_lines)]
pub(crate) fn intersects(a: &[u64; 16], b: &[u64; 16]) -> bool {
    if a[0] & b[0] != 0 {
        return true;
    }
    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::aarch64::{
            vandq_u64, vld1q_u64, vmaxvq_u32, vorrq_u64, vreinterpretq_u32_u64,
        };
        // SAFETY: `a` and `b` are valid `[u64; 16]` arrays (128 bytes each).
        // Every `add(idx)` below stays within `0..=14`, reading 2 `u64`s (16 bytes)
        // in bounds via `vld1q_u64` which only requires 8-byte `u64` alignment.
        unsafe {
            let ap = a.as_ptr();
            let bp = b.as_ptr();
            let v0 = vandq_u64(vld1q_u64(ap), vld1q_u64(bp));
            let v1 = vandq_u64(vld1q_u64(ap.add(2)), vld1q_u64(bp.add(2)));
            let v2 = vandq_u64(vld1q_u64(ap.add(4)), vld1q_u64(bp.add(4)));
            let v3 = vandq_u64(vld1q_u64(ap.add(6)), vld1q_u64(bp.add(6)));
            let v4 = vandq_u64(vld1q_u64(ap.add(8)), vld1q_u64(bp.add(8)));
            let v5 = vandq_u64(vld1q_u64(ap.add(10)), vld1q_u64(bp.add(10)));
            let v6 = vandq_u64(vld1q_u64(ap.add(12)), vld1q_u64(bp.add(12)));
            let v7 = vandq_u64(vld1q_u64(ap.add(14)), vld1q_u64(bp.add(14)));
            let p0 = vorrq_u64(vorrq_u64(v0, v1), vorrq_u64(v2, v3));
            let p1 = vorrq_u64(vorrq_u64(v4, v5), vorrq_u64(v6, v7));
            let all = vorrq_u64(p0, p1);
            vmaxvq_u32(vreinterpretq_u32_u64(all)) != 0
        }
    }
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    #[allow(clippy::cast_ptr_alignment)] // loadu/storeu explicitly accept unaligned addresses.
    {
        use core::arch::x86_64::{
            __m128i, _mm_and_si128, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8,
            _mm_or_si128, _mm_setzero_si128,
        };
        // SAFETY: `a` and `b` are valid 128-byte `[u64; 16]` arrays.
        // `_mm_loadu_si128` performs unaligned 16-byte loads at offsets `0..=14` `u64`s.
        unsafe {
            let ap = a.as_ptr();
            let bp = b.as_ptr();
            let v0 = _mm_and_si128(
                _mm_loadu_si128(ap.cast::<__m128i>()),
                _mm_loadu_si128(bp.cast::<__m128i>()),
            );
            let v1 = _mm_and_si128(
                _mm_loadu_si128(ap.add(2).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(2).cast::<__m128i>()),
            );
            let v2 = _mm_and_si128(
                _mm_loadu_si128(ap.add(4).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(4).cast::<__m128i>()),
            );
            let v3 = _mm_and_si128(
                _mm_loadu_si128(ap.add(6).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(6).cast::<__m128i>()),
            );
            let v4 = _mm_and_si128(
                _mm_loadu_si128(ap.add(8).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(8).cast::<__m128i>()),
            );
            let v5 = _mm_and_si128(
                _mm_loadu_si128(ap.add(10).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(10).cast::<__m128i>()),
            );
            let v6 = _mm_and_si128(
                _mm_loadu_si128(ap.add(12).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(12).cast::<__m128i>()),
            );
            let v7 = _mm_and_si128(
                _mm_loadu_si128(ap.add(14).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(14).cast::<__m128i>()),
            );
            let p0 = _mm_or_si128(_mm_or_si128(v0, v1), _mm_or_si128(v2, v3));
            let p1 = _mm_or_si128(_mm_or_si128(v4, v5), _mm_or_si128(v6, v7));
            let all = _mm_or_si128(p0, p1);
            _mm_movemask_epi8(_mm_cmpeq_epi8(all, _mm_setzero_si128())) != 0xFFFF
        }
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        use core::arch::wasm32::{v128, v128_and, v128_any_true, v128_load, v128_or};
        // SAFETY: `a` and `b` are valid 128-byte `[u64; 16]` arrays.
        // `v128_load` reads 16 bytes in bounds at offsets `0..=14` `u64`s.
        unsafe {
            let ap = a.as_ptr();
            let bp = b.as_ptr();
            let v0 = v128_and(v128_load(ap.cast::<v128>()), v128_load(bp.cast::<v128>()));
            let v1 = v128_and(
                v128_load(ap.add(2).cast::<v128>()),
                v128_load(bp.add(2).cast::<v128>()),
            );
            let v2 = v128_and(
                v128_load(ap.add(4).cast::<v128>()),
                v128_load(bp.add(4).cast::<v128>()),
            );
            let v3 = v128_and(
                v128_load(ap.add(6).cast::<v128>()),
                v128_load(bp.add(6).cast::<v128>()),
            );
            let v4 = v128_and(
                v128_load(ap.add(8).cast::<v128>()),
                v128_load(bp.add(8).cast::<v128>()),
            );
            let v5 = v128_and(
                v128_load(ap.add(10).cast::<v128>()),
                v128_load(bp.add(10).cast::<v128>()),
            );
            let v6 = v128_and(
                v128_load(ap.add(12).cast::<v128>()),
                v128_load(bp.add(12).cast::<v128>()),
            );
            let v7 = v128_and(
                v128_load(ap.add(14).cast::<v128>()),
                v128_load(bp.add(14).cast::<v128>()),
            );
            let p0 = v128_or(v128_or(v0, v1), v128_or(v2, v3));
            let p1 = v128_or(v128_or(v4, v5), v128_or(v6, v7));
            v128_any_true(v128_or(p0, p1))
        }
    }
    #[cfg(not(any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", target_feature = "sse2"),
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    {
        intersects_scalar(a, b)
    }
}

#[inline]
pub(crate) fn is_empty(a: &[u64; 16]) -> bool {
    if a[0] != 0 {
        return false;
    }
    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::aarch64::{vld1q_u64, vmaxvq_u32, vorrq_u64, vreinterpretq_u32_u64};
        // SAFETY: `a` is a valid 128-byte `[u64; 16]` array; all 16-byte loads are in bounds.
        unsafe {
            let ap = a.as_ptr();
            let p0 = vorrq_u64(
                vorrq_u64(vld1q_u64(ap), vld1q_u64(ap.add(2))),
                vorrq_u64(vld1q_u64(ap.add(4)), vld1q_u64(ap.add(6))),
            );
            let p1 = vorrq_u64(
                vorrq_u64(vld1q_u64(ap.add(8)), vld1q_u64(ap.add(10))),
                vorrq_u64(vld1q_u64(ap.add(12)), vld1q_u64(ap.add(14))),
            );
            vmaxvq_u32(vreinterpretq_u32_u64(vorrq_u64(p0, p1))) == 0
        }
    }
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    #[allow(clippy::cast_ptr_alignment)] // loadu/storeu explicitly accept unaligned addresses.
    {
        use core::arch::x86_64::{
            __m128i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_or_si128,
            _mm_setzero_si128,
        };
        // SAFETY: `a` is a valid 128-byte `[u64; 16]` array; unaligned loads are in bounds.
        unsafe {
            let ap = a.as_ptr();
            let p0 = _mm_or_si128(
                _mm_or_si128(
                    _mm_loadu_si128(ap.cast::<__m128i>()),
                    _mm_loadu_si128(ap.add(2).cast::<__m128i>()),
                ),
                _mm_or_si128(
                    _mm_loadu_si128(ap.add(4).cast::<__m128i>()),
                    _mm_loadu_si128(ap.add(6).cast::<__m128i>()),
                ),
            );
            let p1 = _mm_or_si128(
                _mm_or_si128(
                    _mm_loadu_si128(ap.add(8).cast::<__m128i>()),
                    _mm_loadu_si128(ap.add(10).cast::<__m128i>()),
                ),
                _mm_or_si128(
                    _mm_loadu_si128(ap.add(12).cast::<__m128i>()),
                    _mm_loadu_si128(ap.add(14).cast::<__m128i>()),
                ),
            );
            let all = _mm_or_si128(p0, p1);
            _mm_movemask_epi8(_mm_cmpeq_epi8(all, _mm_setzero_si128())) == 0xFFFF
        }
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        use core::arch::wasm32::{v128, v128_any_true, v128_load, v128_or};
        // SAFETY: `a` is a valid 128-byte `[u64; 16]` array; 16-byte loads are in bounds.
        unsafe {
            let ap = a.as_ptr();
            let p0 = v128_or(
                v128_or(
                    v128_load(ap.cast::<v128>()),
                    v128_load(ap.add(2).cast::<v128>()),
                ),
                v128_or(
                    v128_load(ap.add(4).cast::<v128>()),
                    v128_load(ap.add(6).cast::<v128>()),
                ),
            );
            let p1 = v128_or(
                v128_or(
                    v128_load(ap.add(8).cast::<v128>()),
                    v128_load(ap.add(10).cast::<v128>()),
                ),
                v128_or(
                    v128_load(ap.add(12).cast::<v128>()),
                    v128_load(ap.add(14).cast::<v128>()),
                ),
            );
            !v128_any_true(v128_or(p0, p1))
        }
    }
    #[cfg(not(any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", target_feature = "sse2"),
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    {
        is_empty_scalar(a)
    }
}

#[inline]
#[allow(clippy::too_many_lines)]
pub(crate) fn contains_all(a: &[u64; 16], b: &[u64; 16]) -> bool {
    if !a[0] & b[0] != 0 {
        return false;
    }
    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::aarch64::{
            vbicq_u64, vld1q_u64, vmaxvq_u32, vorrq_u64, vreinterpretq_u32_u64,
        };
        // SAFETY: `a` and `b` are valid 128-byte `[u64; 16]` arrays; all loads are in bounds.
        unsafe {
            let ap = a.as_ptr();
            let bp = b.as_ptr();
            let v0 = vbicq_u64(vld1q_u64(bp), vld1q_u64(ap));
            let v1 = vbicq_u64(vld1q_u64(bp.add(2)), vld1q_u64(ap.add(2)));
            let v2 = vbicq_u64(vld1q_u64(bp.add(4)), vld1q_u64(ap.add(4)));
            let v3 = vbicq_u64(vld1q_u64(bp.add(6)), vld1q_u64(ap.add(6)));
            let v4 = vbicq_u64(vld1q_u64(bp.add(8)), vld1q_u64(ap.add(8)));
            let v5 = vbicq_u64(vld1q_u64(bp.add(10)), vld1q_u64(ap.add(10)));
            let v6 = vbicq_u64(vld1q_u64(bp.add(12)), vld1q_u64(ap.add(12)));
            let v7 = vbicq_u64(vld1q_u64(bp.add(14)), vld1q_u64(ap.add(14)));
            let p0 = vorrq_u64(vorrq_u64(v0, v1), vorrq_u64(v2, v3));
            let p1 = vorrq_u64(vorrq_u64(v4, v5), vorrq_u64(v6, v7));
            vmaxvq_u32(vreinterpretq_u32_u64(vorrq_u64(p0, p1))) == 0
        }
    }
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    #[allow(clippy::cast_ptr_alignment)] // loadu/storeu explicitly accept unaligned addresses.
    {
        use core::arch::x86_64::{
            __m128i, _mm_andnot_si128, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8,
            _mm_or_si128, _mm_setzero_si128,
        };
        // SAFETY: `a` and `b` are valid 128-byte `[u64; 16]` arrays; unaligned loads are in bounds.
        unsafe {
            let ap = a.as_ptr();
            let bp = b.as_ptr();
            let v0 = _mm_andnot_si128(
                _mm_loadu_si128(ap.cast::<__m128i>()),
                _mm_loadu_si128(bp.cast::<__m128i>()),
            );
            let v1 = _mm_andnot_si128(
                _mm_loadu_si128(ap.add(2).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(2).cast::<__m128i>()),
            );
            let v2 = _mm_andnot_si128(
                _mm_loadu_si128(ap.add(4).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(4).cast::<__m128i>()),
            );
            let v3 = _mm_andnot_si128(
                _mm_loadu_si128(ap.add(6).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(6).cast::<__m128i>()),
            );
            let v4 = _mm_andnot_si128(
                _mm_loadu_si128(ap.add(8).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(8).cast::<__m128i>()),
            );
            let v5 = _mm_andnot_si128(
                _mm_loadu_si128(ap.add(10).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(10).cast::<__m128i>()),
            );
            let v6 = _mm_andnot_si128(
                _mm_loadu_si128(ap.add(12).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(12).cast::<__m128i>()),
            );
            let v7 = _mm_andnot_si128(
                _mm_loadu_si128(ap.add(14).cast::<__m128i>()),
                _mm_loadu_si128(bp.add(14).cast::<__m128i>()),
            );
            let p0 = _mm_or_si128(_mm_or_si128(v0, v1), _mm_or_si128(v2, v3));
            let p1 = _mm_or_si128(_mm_or_si128(v4, v5), _mm_or_si128(v6, v7));
            let all = _mm_or_si128(p0, p1);
            _mm_movemask_epi8(_mm_cmpeq_epi8(all, _mm_setzero_si128())) == 0xFFFF
        }
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        use core::arch::wasm32::{v128, v128_andnot, v128_any_true, v128_load, v128_or};
        // SAFETY: `a` and `b` are valid 128-byte `[u64; 16]` arrays; 16-byte loads are in bounds.
        unsafe {
            let ap = a.as_ptr();
            let bp = b.as_ptr();
            let v0 = v128_andnot(v128_load(bp.cast::<v128>()), v128_load(ap.cast::<v128>()));
            let v1 = v128_andnot(
                v128_load(bp.add(2).cast::<v128>()),
                v128_load(ap.add(2).cast::<v128>()),
            );
            let v2 = v128_andnot(
                v128_load(bp.add(4).cast::<v128>()),
                v128_load(ap.add(4).cast::<v128>()),
            );
            let v3 = v128_andnot(
                v128_load(bp.add(6).cast::<v128>()),
                v128_load(ap.add(6).cast::<v128>()),
            );
            let v4 = v128_andnot(
                v128_load(bp.add(8).cast::<v128>()),
                v128_load(ap.add(8).cast::<v128>()),
            );
            let v5 = v128_andnot(
                v128_load(bp.add(10).cast::<v128>()),
                v128_load(ap.add(10).cast::<v128>()),
            );
            let v6 = v128_andnot(
                v128_load(bp.add(12).cast::<v128>()),
                v128_load(ap.add(12).cast::<v128>()),
            );
            let v7 = v128_andnot(
                v128_load(bp.add(14).cast::<v128>()),
                v128_load(ap.add(14).cast::<v128>()),
            );
            let p0 = v128_or(v128_or(v0, v1), v128_or(v2, v3));
            let p1 = v128_or(v128_or(v4, v5), v128_or(v6, v7));
            !v128_any_true(v128_or(p0, p1))
        }
    }
    #[cfg(not(any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", target_feature = "sse2"),
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    {
        contains_all_scalar(a, b)
    }
}

#[inline]
pub(crate) fn union_with(dst: &mut [u64; 16], src: &[u64; 16]) {
    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::aarch64::{vld1q_u64, vorrq_u64, vst1q_u64};
        // SAFETY: `dst` and `src` are disjoint valid 128-byte `[u64; 16]` arrays.
        // Every load/store of 2 `u64`s at offsets `0..=14` is in bounds.
        unsafe {
            let dp = dst.as_mut_ptr();
            let sp = src.as_ptr();
            let mut i = 0;
            while i < 16 {
                let d = vld1q_u64(dp.add(i));
                let s = vld1q_u64(sp.add(i));
                vst1q_u64(dp.add(i), vorrq_u64(d, s));
                i += 2;
            }
        }
    }
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    #[allow(clippy::cast_ptr_alignment)] // loadu/storeu explicitly accept unaligned addresses.
    {
        use core::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_or_si128, _mm_storeu_si128};
        // SAFETY: `dst` and `src` are disjoint valid 128-byte `[u64; 16]` arrays.
        unsafe {
            let dp = dst.as_mut_ptr();
            let sp = src.as_ptr();
            let mut i = 0;
            while i < 16 {
                let d = _mm_loadu_si128(dp.add(i).cast::<__m128i>());
                let s = _mm_loadu_si128(sp.add(i).cast::<__m128i>());
                _mm_storeu_si128(dp.add(i).cast::<__m128i>(), _mm_or_si128(d, s));
                i += 2;
            }
        }
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        use core::arch::wasm32::{v128, v128_load, v128_or, v128_store};
        // SAFETY: `dst` and `src` are disjoint valid 128-byte `[u64; 16]` arrays.
        unsafe {
            let dp = dst.as_mut_ptr();
            let sp = src.as_ptr();
            let mut i = 0;
            while i < 16 {
                let d = v128_load(dp.add(i).cast::<v128>());
                let s = v128_load(sp.add(i).cast::<v128>());
                v128_store(dp.add(i).cast::<v128>(), v128_or(d, s));
                i += 2;
            }
        }
    }
    #[cfg(not(any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", target_feature = "sse2"),
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    {
        union_with_scalar(dst, src);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        contains_all, contains_all_scalar, intersects, intersects_scalar, is_empty,
        is_empty_scalar, union_with, union_with_scalar,
    };

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
            if case % 7 == 0 {
                b = a;
                b[case % 16] &= b[(case + 3) % 16];
            }
            if case % 11 == 0 {
                a = [0; 16];
            }
            assert_eq!(intersects(&a, &b), intersects_scalar(&a, &b));
            assert_eq!(is_empty(&a), is_empty_scalar(&a));
            assert_eq!(contains_all(&a, &b), contains_all_scalar(&a, &b));

            let mut u_fast = a;
            let mut u_ref = a;
            union_with(&mut u_fast, &b);
            union_with_scalar(&mut u_ref, &b);
            assert_eq!(u_fast, u_ref);
        }
        for bit in 0..1024 {
            let mut a = [0; 16];
            let mut b = [0; 16];
            assert!(is_empty(&a));
            a[bit / 64] = 1 << (bit % 64);
            assert!(!is_empty(&a));
            assert_eq!(is_empty(&a), is_empty_scalar(&a));
            assert!(!intersects(&a, &b));
            assert!(!contains_all(&b, &a));
            assert!(contains_all(&a, &b));
            b[bit / 64] = a[bit / 64];
            assert!(intersects(&a, &b));
            assert!(intersects(&b, &a));
            assert!(contains_all(&a, &b));
        }
    }
}
